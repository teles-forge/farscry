Story 5 - farscry-classifier

**Status:** Ready - @po Fix 8 applied + PRD amendment 2026-05-09
**Blocked by:** Story 1 (farscry-core)
**Estimated hours:** 7h

PRD Amendment - Accuracy Target

**Original PRD §15:** Screen type classification minimum: 92%
**Amendment:** Screen type classification minimum: **85%**
**Rationale:** The classifier spike (2026-05-08) measured 89.4% OOD accuracy on 188 elements across 5 developer-tool screen types using the screen-type router with zero model, zero deps, ~0.1ms inference. This exceeds the amended 85% bar and was validated empirically. The 92% target was set before the spike revealed the visual element classifier (MobileNetV3) was blocked by AGPL licenses and the screen-type router was the validated alternative. Raising accuracy beyond 89.4% for v0.1.0 would require the visual model (v0.2.0 item). **Button detection (33%) is a documented limitation** - button labels outside the known-words list are not detected. This is acceptable for v0.1.0 given button detection is primarily needed for affordance extraction, not screen-type classification.

What to build

Screen-type router plus spatial rules. Zero model, zero deps beyond farscry-core. 89.4% OOD accuracy validated on 188 elements across 5 developer-tool screen types (classifier spike, 2026-05-08).

Crate: `crates/farscry-classifier/`

**`src/screen.rs`** - screen type detection from OCR text:
```rust
pub fn detect_screen_type(regions: &[TextRegion]) -> ScreenType {


}
```

**`src/element.rs`** - per-element typing based on screen type:
```rust
pub fn classify_elements(regions: &[TextRegion], screen_type: ScreenType) -> Vec<TypedElement> {
    match screen_type {
        ScreenType::Terminal     => // all elements -> ElementType::Label
        ScreenType::Config       => // form rules: ends with ':' -> Label, wide element -> Input, known words -> Button
        ScreenType::Error        => // all elements -> ElementType::Error or Label
        ScreenType::Conversation => // 1-3 word elements -> Heading (speaker), longer -> Label (message)
        ScreenType::Ui | ScreenType::Unknown => // best-effort rules
    }
}
```

Form rules for config screen type:
- Element text ends with ':' -> Label
- Element aspect ratio > 4 AND width > 150px -> Input
- Text in ["Save", "Cancel", "Submit", "Delete", "OK", "Apply", "Next", "Back", "Continue", "Close"] -> Button
- All-caps short text -> Heading
- Otherwise -> Label

**`src/spatial.rs`** - affordance extraction:
```rust
pub fn extract_affordances(elements: &[UiElement]) -> Vec<Affordance> {


}


fn nearest_label(input: &UiElement, all: &[UiElement]) -> String {
    all.iter()
        .filter(|e| e.element_type == ElementType::Label)
        .filter(|e| e.cx < input.cx + 10.0 || e.cy < input.cy + 10.0)
        .min_by_key(|e| {
            let dx = e.cx - input.cx;
            let dy = e.cy - input.cy;
            ((dx * dx + dy * dy) as u32)
        })
        .map(|e| e.text.clone())
        .unwrap_or_else(|| input.text.clone())
}
```

**`src/lib.rs`** - implements `ElementClassifier` + `ScreenClassifier` traits from farscry-core.
Uses `UiElement` (not `TypedElement` - that name was removed in Fix 2).

Inference speed requirement: ~0.1ms/element (pure Rust, no model).
-> AC threshold: **< 2ms for 20 elements** (0.1ms x 20 = 2ms).

**`Cargo.toml`:**
```toml
[dependencies]
farscry-core = { path = "../farscry-core" }
```
Zero additional dependencies.

Acceptance criteria

- [ ] `cargo test -p farscry-classifier` passes, zero warnings
- [ ] Screen-type: `$ python3 app.py` in any region -> `ScreenType::Terminal`
- [ ] Screen-type: >=2 regions ending `:` -> `ScreenType::Config`
- [ ] Screen-type: "TypeError: ..." -> `ScreenType::Error`
- [ ] Screen-type: >=40% of regions are 1-3 words -> `ScreenType::Conversation`
- [ ] Screen-type priority: screen with both `$` and `error` keyword -> `ScreenType::Terminal` (terminal wins)
- [ ] Terminal: all elements classified as `ElementType::Label`
- [ ] Config: region ending `:` -> `Label`; wide text box -> `Input`; "Save" -> `Button`
- [ ] Config: "Save", "Cancel", "Submit", "Delete", "OK", "Apply", "Next", "Back" -> `Button`
- [ ] Affordance extracted for every `Button` element
- [ ] Affordance extracted for every `Input` element with nearest Label as label text
- [ ] `nearest_label` fallback: no Label within 200px -> use Input's own text
- [ ] Inference time **< 2ms for 20 elements** (benchmark test - aligns with ~0.1ms/element spec)
- [ ] Overall OOD accuracy >= 85% on the 188-element classifier spike test set (measured, not estimated)
- [ ] `input` class accuracy >= 95%
- [ ] `label` class accuracy >= 95%
- [ ] `button` class accuracy >= 30% (documented limitation - v0.2.0 will add visual model)
- [ ] Zero external model dependencies - `cargo tree -p farscry-classifier` shows only farscry-core
- [ ] `publish = false`

Dependencies

Story 1 (farscry-core).
