Story 7 - farscry-formatter

**Status:** Ready
**Blocked by:** Story 1 (farscry-core)
**Estimated hours:** 5h

What to build

VASP compact text output (default) and JSON output (--json flag). Token savings line. Batch separator.

Crate: `crates/farscry-formatter/`

**`src/vasp.rs`** - compact text formatter (default output, ~175 tokens typical):

Output structure:
```
=== farscry visual context ===
source: {image_path}
screen_type: {screen_type}
state_id: phash:{hex}
confidence: {high|medium|low|none}
lang: {lang}
agent_context: "{one-line summary}"
---
[{position_label}]  {element_type}  "{text}"  [{optional: at (cx,cy)}]  [{optional: enabled:true/false}]  [{optional: value="..."}]

affordances:
  {click|type|select} -> "{label}" at ({cx},{cy})  {enabled:true/false}  [{current:"{value}"}]

Token savings: ~{N} tokens saved vs sending raw image to cloud vision
```

**Position labels (3x3 grid):**
- Horizontal: left (cx < W/3), center (W/3 <= cx < 2W/3), right (cx >= 2W/3)
- Vertical: top (cy < H/3), middle (H/3 <= cy < 2H/3), bottom (cy >= 2H/3)
- Format: `[top-left]`, `[top-center]`, `[top-right]`, `[middle-left]`, `[middle]`, `[middle-right]`, `[bottom-left]`, `[bottom-center]`, `[bottom-right]`
- Sorted: top-to-bottom by cy, then left-to-right by cx within same row

**agent_context generation (one-line summary):**
- terminal: "Build failed - {first error line}" or "Script completed successfully"
- error: "{error_type} - {suggested_action}"
- config: "{section} - {N} editable fields, {button} available"
- conversation: "{platform} conversation - {last_speaker}: {last_message_preview}"
- ui/unknown: "Screen captured - {N} elements, {N} interactive"

**Token savings estimate:**
```rust
fn token_savings(image_w: u32, image_h: u32, vasp_text: &str) -> u32 {

    let image_tokens = ((image_w + 511) / 512) * ((image_h + 511) / 512) * 170 + 85;
    let vasp_tokens  = (vasp_text.len() / 4) as u32;
    image_tokens.saturating_sub(vasp_tokens)
}
```

**`src/json.rs`** - serde_json output for --json flag. Full VaspOutput serialized.

**`src/batch.rs`** - batch separator:
```
---
file: img1.png
{vasp output}
---
file: img2.png
{vasp output}
```

**`src/diff_formatter.rs`** - diff output formatter for VaspDelta.

**`src/lib.rs`** - implements VaspFormatter trait.

**`Cargo.toml`:**
```toml
[dependencies]
farscry-core = { path = "../farscry-core" }
serde_json = "1"
```

Acceptance criteria

- [ ] `cargo test -p farscry-formatter` passes
- [ ] Default output: starts with `=== farscry visual context ===`
- [ ] state_id field present: `state_id: phash:...`
- [ ] affordances section present when affordances exist
- [ ] Elements sorted top-to-bottom, then left-to-right
- [ ] Token savings line present and > 0 for 1080p images
- [ ] `--json` flag: valid JSON, parseable by `serde_json::from_str`
- [ ] Batch mode: `---` separator between files with `file:` header
- [ ] Diff output: appeared/changed/removed/unchanged sections
- [ ] Diff output: `context_similarity:` and `context_changed:` fields present
- [ ] stdout ONLY - no progress output from this crate
- [ ] publish = false

Dependencies

Story 1 (farscry-core).
