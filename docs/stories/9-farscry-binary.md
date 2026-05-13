Story 9 - farscry binary (CLI + clipboard + setup)

**Status:** Ready
**Blocked by:** Stories 4, 5, 6, 7, 8 (all crates)
**Estimated hours:** 12h

What to build

The `farscry` binary CLI that orchestrates all crates. Three modes: extract, diff, daemon. Zero business logic - all logic in crates.

Crate: `crates/farscry/src/main.rs`

**Commands (clap):**

```bash
farscry <image.png>                    # extract (Mode 1)
farscry --from-clipboard               # extract from clipboard
cat image.png | farscry                # extract from stdin
farscry diff before.png after.png      # diff (Mode 2)
farscry diff before.png after.png --agent
farscry *.png                          # batch
farscry serve --mcp                    # daemon (Mode 3)
farscry serve --mcp --port 3333
farscry setup                          # show config snippets
farscry --install-lang por             # download language model
farscry --version
```

**Flags for extract:**
- `--json` - JSON output
- `--affordances` - include affordances section (default: true)
- `--context` - output only agent_context line
- `--text-only` - VASP only, do not forward image to agent
- `--lang eng+por` - language(s)
- `--max-size 20mb` - override 10MB limit
- `-v` / `--verbose` - verbose stderr
- `--debug` - everything to stderr

**stdout contract:** ONLY VASP/JSON goes to stdout. ALL errors, warnings, progress go to stderr. This is enforced in main - any crate that writes to stdout is a bug.

**Exit codes:**
- 0: success
- 1: input error (file not found, wrong format, too large, too small)
- 2: processing error (OCR failed, model error)
- 3: configuration error (language not installed)

**`--from-clipboard` implementation:**
```rust

fn read_clipboard_png() -> Result<Vec<u8>, FarscryError> {

    let script = r#"
set out to "/tmp/farscry_clipboard.png"
set tiff to "/tmp/farscry_clipboard.tiff"
try
    set d to (the clipboard as «class PNGf»)
    set f to open for access POSIX file out with write permission
    set eof of f to 0
    write d to f
    close access f
    return out
on error
    set d to (the clipboard as TIFF picture)
    set f to open for access POSIX file tiff with write permission
    set eof of f to 0
    write d to f
    close access f
    do shell script "sips -s format png " & quoted form of tiff & " --out " & quoted form of out
    return out
end try"#;
    let result = Command::new("osascript").arg("-e").arg(script).output()?;
    if !result.status.success() {
        return Err(FarscryError::InvalidInput { message: "No image in clipboard".into() });
    }
    std::fs::read("/tmp/farscry_clipboard.png").map_err(|e| ...)
}


```

**`farscry setup` implementation (NEVER auto-modifies configs):**
```
farscry setup

OK Models: ~/.farscry/models/ (12.1 MB)
OK farscry v0.1.0 installed

Detected agent environments:
  MCP client  - config snippet below
  MCP client    - config snippet below

To use farscry with MCP client, add to .claude/mcp.json:
{
  "mcpServers": {
    "farscry": {
      "command": "farscry",
      "args": ["serve", "--mcp"]
    }
  }
}

Copy and paste manually. farscry never modifies your configs automatically.
```

**Input validation (before any processing):**
```rust
fn validate_image(path: &Path, max_size: u64) -> Result<(), FarscryError> {


}
```

**`Cargo.toml`:**
```toml
[dependencies]
farscry-core       = { path = "../farscry-core" }
farscry-ocr        = { path = "../farscry-ocr" }
farscry-classifier = { path = "../farscry-classifier" }
farscry-diff       = { path = "../farscry-diff" }
farscry-formatter  = { path = "../farscry-formatter" }
farscry-mcp        = { path = "../farscry-mcp" }
clap = { version = "4", features = ["derive"] }
anyhow = "1"
image = "0.25"
tokio = { version = "1", features = ["full"] }
```

Acceptance criteria

- [ ] `cargo build --release -p farscry` -> binary named `farscry`
- [ ] `farscry --version` outputs version string
- [ ] `farscry screen.png` writes VASP to stdout, nothing else
- [ ] `farscry screen.png 2>/dev/null` - stdout is clean (no progress noise)
- [ ] `farscry nonexistent.png` -> exit code 1, error to stderr only
- [ ] `farscry /etc/passwd` -> exit code 1 (not an image)
- [ ] `farscry screen.png --json` -> valid JSON to stdout
- [ ] `farscry diff a.png b.png` -> delta to stdout
- [ ] `farscry --from-clipboard` works on macOS with image in clipboard
- [ ] `farscry --from-clipboard` works on Linux with xclip
- [ ] `farscry setup` outputs config snippets, does NOT modify any file
- [ ] `farscry *.png` processes all PNGs in parallel, `---` separator between outputs
- [ ] `farscry serve --mcp` starts MCP server (test: connect and call farscry_extract)
- [ ] Exit code 0 on success, 1 input error, 2 processing error, 3 config error
- [ ] `cargo audit` passes
- [ ] `cargo clippy -p farscry` zero warnings
- [ ] Binary name in Cargo.toml: `name = "farscry"` (not "pipe")
- [ ] publish = true

Dependencies

Stories 4 (farscry-ocr), 5 (farscry-classifier), 6 (farscry-diff), 7 (farscry-formatter), 8 (farscry-mcp).
