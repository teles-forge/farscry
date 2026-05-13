Story 6 - farscry-diff

**Status:** Ready
**Blocked by:** Story 1 (farscry-core)
**Estimated hours:** 8h

What to build

Screenshot diff engine: context gate + scroll correction + bipartite matching. Validated in spike (3/3 tests, 0 false positives, 2026-05-08).

Crate: `crates/farscry-diff/`

**`src/delta.rs`** - full diff algorithm:

```
STEP 0 - Context gate (runs FIRST, before scroll correction)
  before_tokens = whitespace_split(lowercase(before_ocr_text))
  after_tokens  = whitespace_split(lowercase(after_ocr_text))
  shared = |intersection(before_tokens, after_tokens)|
  overlap = shared / max(|before_tokens|, |after_tokens|)

  if overlap < 0.20:
      emit context_similarity: overlap, context_changed: true
      emit agent_context: "context changed - unrelated UIs detected"
      RETURN EARLY (skip all matching)

STEP 1 - Scroll correction (median dy/dx from text-similar pairs)
  rough_matches = pairs where text_similarity >= 0.70
  scroll_dy = median(rough_matches.map(|m| m.after.cy - m.before.cy))
  scroll_dx = median(rough_matches.map(|m| m.after.cx - m.before.cx))

STEP 2 - Greedy bipartite matching
  For all (i, j) pairs:
    score(a, b) = 0.4 x text_sim(a.text, b.text)         [normalized Levenshtein]
               + 0.4 x pos_proximity(a, b, scroll_dy/dx)  [Gaussian σ=80px]
               + 0.2 x type_match(a.type, b.type)         [1.0 same, 0.5 different]

  Sort all candidate pairs by score descending
  Greedy bijection: assign highest-score pairs first, lock both indices
  threshold: score > 0.60 = matched

  Classify each matched pair:
    text_sim >= 0.95 -> Unchanged
    text_sim  < 0.95 -> Changed { from: a.text, to: b.text }

  Unmatched in before -> Removed
  Unmatched in after  -> Appeared

STEP 3 - Emit VaspDelta
  context_similarity: overlap (from Step 0)
  context_changed: false
  entries: [Unchanged/Changed/Appeared/Removed]
  scroll_dy: f32
```

**text_similarity:** normalized Levenshtein - `1.0 - dist / max(len_a, len_b)`. Must handle empty strings (return 1.0 if both empty, 0.0 if one empty).

**pos_proximity:** `exp(-dist² / (2 x σ²))` where `σ = 80.0` pixels, `dist = sqrt((bx-ax-dx)² + (by-ay-dy)²)` (scroll-corrected).

**type_match:** 1.0 if same ElementType, 0.5 if different.

**`src/hash.rs`** - re-exports StateHasher impl (pHash already in farscry-core, just wire it here).

**`src/lib.rs`** - implements DiffEngine trait:
```rust
impl DiffEngine for BipartiteDiffEngine {
    fn diff(&self, before: &VaspOutput, after: &VaspOutput) -> VaspDelta;
}
```

**`Cargo.toml`:**
```toml
[dependencies]
farscry-core = { path = "../farscry-core" }
```

Acceptance criteria

- [ ] `cargo test -p farscry-diff` passes
- [ ] Test: context_similarity < 0.20 -> context_changed:true, no delta emitted
- [ ] Test: scroll scenario - 10 elements shifted 240px -> scroll_dy=-240, 4 appeared, 4 removed, 6 unchanged, 0 false positives
- [ ] Test: field filled - "Enter email" unchanged, "user@example.com" appeared OR email field changed detected (1 change minimum)
- [ ] Test: error appeared - 5 unchanged, 1 appeared, 0 false positives
- [ ] Test: same image twice -> all Unchanged, 0 Changed/Appeared/Removed
- [ ] text_similarity("abc", "abc") == 1.0
- [ ] text_similarity("", "") == 1.0
- [ ] text_similarity("abc", "") == 0.0
- [ ] Greedy matching is bijection (no element assigned twice)
- [ ] O(nxm) complexity, tested with 50 elements each side (no timeout)
- [ ] publish = false

Dependencies

Story 1 (farscry-core).
