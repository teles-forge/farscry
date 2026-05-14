# farscry v0.2.0 - Roadmap and Spike Planning

**Status:** Planning
**Target:** Q3 2026
**Author:** Darlysson Teles

---

## Strategic context

v0.1.0 proved the core: local OCR pipeline, typed VASP output, 38ms daemon,
benchmark with methodology. The HN post establishes credibility.

v0.2.0 has three goals:
1. Fulfill the promise already made (multi-language, broken install-lang command)
2. Add the one feature that makes the project go viral on its own (annotate)
3. Begin the protocol adoption path (VASP adapters)

---

## Feature 1 — Multi-language OCR

**Why:**
`farscry install-lang por` currently throws an error: "Multi-language support
arrives in v0.2." This was publicly committed. Anyone who tries it sees a
broken command. Fix this before anything else.

**How:**

PP-OCRv5 publishes per-language ONNX recognition models on HuggingFace:
- `PP-OCRv5_server_rec` for base English
- `PP-OCRv5_mobile_rec_*` for language-specific models

Steps:
1. Update `install_lang()` in `crates/farscry/src/main.rs` to actually download
   the model instead of returning an error.
2. The model URL pattern: `https://huggingface.co/PaddlePaddle/PP-OCRv5/resolve/main/rec_{lang}.onnx`
3. Download to `~/.farscry/models/lang/{lang}.onnx`
4. SHA256 verify before use (follow the same pattern as `verify.rs`)
5. Update `farscry-ocr-ort/src/engine.rs` to accept `lang` parameter and
   load the correct recognition model.

**Files to touch:**
- `crates/farscry/src/main.rs` - install_lang() function
- `crates/farscry-ocr-ort/src/engine.rs` - lang model loading
- `crates/farscry-ocr-ort/src/verify.rs` - SHA256 hashes per language
- `crates/farscry-ocr-coreml/src/engine.rs` - CoreML lang support

**Dependencies:**
- `reqwest` (already in Cargo.toml for model download)
- Language model SHA256 hashes (must be measured, not guessed)

**Acceptance criteria:**
- `farscry install-lang por` downloads and verifies the Portuguese model
- `farscry extract screenshot.png --lang por` runs OCR in Portuguese
- `farscry extract screenshot.png --lang eng+por` runs mixed-language OCR

---

## Feature 2 — `farscry annotate` (highest strategic priority)

**Why:**
This is the feature with the highest viral coefficient. A command that takes
a screenshot and returns the same image with bounding boxes drawn, element
labels, and confidence scores visible.

Every person who uses `farscry annotate` will:
- Share the output image
- Show it to their team
- Post it on Twitter/X
- Use it to debug their agent

The image will have farscry's visual style. It is self-marketing that requires
no effort after implementation.

It also proves accuracy in a way that text output cannot. When someone sees
`[middle-right] button "Save Changes" enabled:true` in text, they trust it
less than when they see a screenshot with a green bounding box drawn exactly
around the Save Changes button.

**How:**

```rust
// New subcommand in Commands enum:
Annotate {
    paths: Vec<PathBuf>,
    #[arg(short = 'o', long)]
    output: Option<PathBuf>,
    #[arg(long, default_value = "rgba(99,102,241,0.8)")]
    color: String,
}
```

Implementation:
1. Run the full OCR pipeline (same as extract)
2. Use the `imageproc` crate to draw rectangles over each detected element
3. Use the `rusttype` or `ab_glyph` crate to draw text labels
4. Each element gets a colored bounding box + label with element type and text
5. Affordances get a different color (e.g., blue for clickable, green for typeable)
6. Write the annotated image to stdout or -o FILE

Visual style to follow:
- Box color: purple/indigo (matches the VASP brand `--accent-high: #818cf8`)
- Label background: dark semi-transparent
- Font: monospace, small
- Show: element type, truncated text, enabled state

**Files to touch:**
- `crates/farscry/src/main.rs` - add Annotate subcommand
- `crates/farscry-formatter/src/lib.rs` - add format_annotated() or keep in main
- `Cargo.toml` - add `imageproc`, `ab_glyph`

**Dependencies:**
```toml
imageproc = "0.25"
ab_glyph = "0.2"
```

**Acceptance criteria:**
- `farscry annotate screenshot.png -o annotated.png` produces annotated image
- Bounding boxes match the positions returned by `farscry extract`
- Works with both CoreML and ORT backends
- Output image is visually clean and shareable

---

## Feature 3 — Windows clipboard

**Why:**
The binary ships for Windows but `farscry extract --from-clipboard` is not
implemented on Windows. Platform completeness. Credibility.

**How:**

The Windows clipboard for images uses `GetClipboardData` with `CF_DIB` or
`CF_BITMAP`. The safest approach on Windows is a PowerShell one-liner:

```rust
#[cfg(target_os = "windows")]
fn read_clipboard_png_windows() -> Result<Vec<u8>> {
    use std::process::Command;
    let script = r#"
Add-Type -AssemblyName System.Windows.Forms
$img = [System.Windows.Forms.Clipboard]::GetImage()
if ($img -eq $null) { exit 1 }
$ms = New-Object System.IO.MemoryStream
$img.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
[System.Convert]::ToBase64String($ms.ToArray())
"#;
    let output = Command::new("powershell")
        .args(["-Command", script])
        .output()?;
    if !output.status.success() {
        anyhow::bail!("No image in clipboard");
    }
    let b64 = String::from_utf8(output.stdout)?.trim().to_string();
    Ok(base64::decode(b64)?)
}
```

**Files to touch:**
- `crates/farscry/src/main.rs` - add Windows branch in extract_from_clipboard()

**Dependencies:**
```toml
base64 = "0.22"  # for Windows only
```

---

## Feature 4 — VASP adapters

**Why:**
For VASP to become a protocol, other tools need to output VASP without
implementing farscry. Adapters convert existing outputs to VASP format.

This is how protocol adoption starts: make it easy for other ecosystems to
join without rewriting everything.

**Three adapters to build for v0.2.0:**

### 4a. Claude computer-use -> VASP

Anthropic's computer use outputs a JSON with bounding boxes and element labels.
A simple converter that maps this to VaspOutput.

```
claude_result.content[].type == "tool_result"
  .content[].type == "image" + "text"
```

Build as: `farscry convert --from claude-computer-use --input result.json`

### 4b. Playwright accessibility tree -> VASP

Playwright's `page.accessibility.snapshot()` returns a JSON tree of elements.
Map roles (button, textbox, heading) to ElementType, extract text content,
use boundingBox() for coordinates.

Build as: `farscry convert --from playwright-a11y --input snapshot.json`

### 4c. OpenAI vision response -> VASP

GPT-4V returns structured JSON with bounding boxes when asked to identify UI
elements. A converter that maps this to VASP.

Build as: `farscry convert --from openai-vision --input response.json`

**Files to touch:**
- New crate: `crates/farscry-adapters/` with one module per adapter
- `crates/farscry/src/main.rs` - add Convert subcommand

---

## Spike plan for `farscry annotate` (start here)

This is the recommended first spike because:
- Highest viral coefficient
- Self-contained (does not depend on any other v0.2.0 feature)
- Demonstrable result in one sitting
- Requires no new protocol design, only rendering

**Spike steps:**

1. Add `imageproc` and `ab_glyph` to `crates/farscry-formatter/Cargo.toml`

2. Create `crates/farscry-formatter/src/annotate.rs`:
```rust
pub fn annotate_image(
    img: image::DynamicImage,
    output: &VaspOutput,
) -> image::DynamicImage {
    // draw bounding box per UiElement
    // cx, cy, w, h are already in the struct
    // element type determines color
    // text label drawn above box
}
```

3. Add to `Commands` enum in `main.rs`:
```rust
Annotate {
    paths: Vec<PathBuf>,
    #[arg(short = 'o', long)]
    output: Option<PathBuf>,
}
```

4. In `fn annotate_images()`, run pipeline then call `annotate_image()`

5. Test with one real screenshot. Take a screenshot of a form or settings panel.
   Run: `cargo run -p farscry -- annotate test.png -o out.png`
   Open out.png. Verify boxes align with elements.

6. If boxes align: feature done. Add to README, add to docs.

**Estimated effort:** 1-2 days for a working first version.

---

## Commit and release plan

Each feature gets its own branch and PR (internal, single dev = merge directly):

```
git checkout -b feat/multi-language
git checkout -b feat/annotate
git checkout -b feat/windows-clipboard
git checkout -b feat/vasp-adapters
```

Version bump when all four are complete:
```
# Cargo.toml: version = "0.2.0"
git tag v0.2.0
git push origin v0.2.0
```

The GitHub Actions release workflow already handles the rest.

---

## What to document after each feature

For each completed feature, before merging:
1. Update `docs/` in farscry-site (CLI reference, VASP docs if protocol changes)
2. Update README.md with new commands
3. Update VASP spec if any fields change
4. Run benchmark if performance-relevant

---

## What NOT to build in v0.2.0

- `farscry watch` - save for v0.3.0 when daemon state tracking is designed properly
- SDK native clients - save for v0.3.0 when daemon API is stable
- VASP stream / SSE - save for v1.0.0
- farscry cloud - save for v1.0.0 if there is demand

Scope discipline matters. Ship fewer things that work than more things that break.
