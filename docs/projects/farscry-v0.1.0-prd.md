PRD - farscry v0.1.0

**Product:** farscry - Image Interpreter for automation workflows
**Version:** v0.1.0 (MVP)
**Author:** Darlysson Teles ([@TelesNascimento25](https://github.com/TelesNascimento25))
**PM review date:** 2026-05-09
**Spec source:** `farscry-product-spec-v3.md` (single source of truth)
**Status:** Ready for story creation -> @sm

---

1. One-Line Product Statement

> farscry converts any screenshot into typed, coordinate-rich structured context that automation workflows can act on - local, free, reproducible, in under 200ms.

---

2. Problem Being Solved

The root cause

coding agents (MCP client, MCP client, MCP-compatible tools) are effectively blind when working with images. Two failure modes:

1. **Binary file rejection** - MCP client issue #30925 (50+ upvotes, March 2026): the Read tool returns "Binary files not supported." The agent cannot read a screenshot at all.
2. **Prose without coordinates** - when agents do receive vision output, they get prose: "there's a button somewhere on the right." Prose has no coordinates. Agents guess where to click. They guess wrong 58% of the time (OSWorld benchmark).

Three compounding problems

| Problem | Description |
|---|---|
| No structured visual context | Agents receive images they can't parse or prose they can't act on precisely |
| No visual state tracking | After an action, agents must re-send the full new screenshot to verify - wasting 1,568 tokens every check |
| No standard interchange format | Every agent, every framework handles visual context differently. No standard exists. |

The gap in the competitive landscape

| Tool | Local | Typed schema | Diff | MCP | Fixes #30925 |
|---|---|---|---|---|---|
| cloud vision | No | No | No | No | No |
| OmniParser v2 | No (GPU) | partial | No | No | No |
| RETIX (Apr 2026) |  | No | No | No | No |
| agent-image-diff |  | No (pixel only) | partial | No | No |
| **farscry** | **** | **** | **** | **** | **** |

---

3. Product Vision

**"Vision APIs describe. farscry gives coordinates. Agents that act, not guess."**

```
farscry screenshot.png  ->  button "Save"  at (400,300)  enabled:true
                        ->  input  "Email" at (200,180)  value:""
                        ->  error  "Card declined" at (20,350)
```

farscry is the reference implementation of VASP - Visual Agent State Protocol. Like MCP standardized tool connectivity for agents, VASP standardizes visual state representation.

```
MCP  = how agents connect to tools
VASP = how agents understand visual state
```

---

4. Three Modes - All Ship in v0.1.0

Mode 1 - DESCRIBE
Any image -> typed VASP context agents can act on immediately.

```bash
farscry error.png | claude "fix this"
farscry figma.png | claude "build this component"
farscry dashboard.png | claude "what needs attention?"
```

Mode 2 - DIFF
Semantic delta between two UI states. Agents verify actions without re-sending screenshots.

```bash
farscry diff before.png after.png
appeared:  error "Card declined"
changed:   button "Submit" -> "Processing..." disabled:true
unchanged: [3 form fields]
context_similarity: 0.923
```

**Token impact:** ~100 tokens vs 3,136 tokens to re-send two full 1080p screenshots.

Mode 3 - CLIPBOARD
Zero-friction capture. No file saving.

```bash
farscry --from-clipboard | agent "fix this"
Cmd+Shift+4 -> farscry reads clipboard -> pipe to agent
```

Supported platforms: macOS (v0.1.0), Linux (v0.1.0), Windows (v0.2.0).

---

5. Validated Technical Foundation

All measurements from spike experiments (2026-05-08 - 2026-05-09):

| Metric | Value | Source |
|---|---|---|
| OCR latency (Apple Silicon, CoreML) | **21ms** | spike-coreml-ep, M4 Pro |
| OCR latency (x86 CPU, ORT A+B+C+D+F) | **~120ms** | projected from ARM64 measurements |
| Token reduction (1080p screenshot) | **8.9x fewer tokens** | empirical: 1,568 -> 175 tokens |
| Token reduction (800px) | **3.7x avg** | benchmark N=40 |
| Element identification accuracy | **100% parity** with cloud vision | benchmark N=40 |
| Diff engine false positives | **0** | spike: 3/3 tests |
| Screen-type router accuracy | **89.4% OOD** | classifier spike: 188 elements |
| Pipe workflow | **Confirmed end-to-end** | pipe spike |

---

6. VASP Protocol

Core fields

```
vasp_version:       1.0
schema_version:     1
state_id:           phash:<16-char-hex>        # pHash on input image
screen_type:        error|config|terminal|conversation|ui|unknown
confidence:         high|medium|low|none
lang:               eng|por|rus|chi_sim|...
delta_from:         phash:<prior>|null
context_similarity: 0.0-1.0|null               # diff only
context_changed:    true|false|null            # true when similarity < 0.20
agent_context:      <one-line summary>
```

state_id algorithm (reproducible, cross-platform)

```
state_id = pHash(grayscale(resize(image, 32x32)))
```

Steps (unambiguous - standard pHash, NOT JPEG block-DCT):
1. Resize to 32x32 using nearest-neighbor interpolation
2. Grayscale: luma = 0.299R + 0.587G + 0.114B
3. Compute 32x32 2D DCT-II (full image, not 8x8 block DCT)
4. Extract top-left 8x8 DCT coefficients (64 low-frequency values)
5. Exclude DCT[0][0] (DC component); compute mean of remaining 63
6. For each of 63 values: bit=1 if value > mean, else bit=0
7. Pack 63 bits + 1 padding = 64 bits = `StateId([u8; 8])` (big-endian)
8. Display: `phash:<16-char-lowercase-hex>`

Implementation: use `rustdct` crate (pure Rust, MIT, deterministic). Do NOT use FFTW.

**Why pHash on INPUT (not OCR output):** ONNX Runtime floating-point is non-deterministic between x86 AVX2/AVX-512/ARM NEON. pHash is integer-dominant - stable to 1-5px rendering jitter.

Compact text output (default - ~175 tokens typical)

```
=== farscry visual context ===
source: screenshot.png
screen_type: config
state_id: phash:8f4a2c9d1e3b7f6a
confidence: high
agent_context: "Payment settings - Save available"
---
[top-left]    heading  "Payment Settings"
[mid-left]    label    "Max Value:"
[mid-center]  input    value="1500"        editable:true
[mid-right]   button   "Save Changes"      enabled:true
[bottom]      error    "Value must be <= 10000"

affordances:
  click -> "Save Changes" at (400,300)  enabled:true
  type  -> "Max Value"    at (200,120)  current:"1500"
```

Diff output

```
vasp_version: 1.0
diff_from: phash:8f4a2c...
diff_to:   phash:3d9b1e...
context_similarity: 0.847
context_changed: false

delta:
  appeared:   error_banner "Payment failed - card declined"
  changed:    button "Submit" -> "Processing..." enabled:true -> false
  removed:    spinner at (450,200)
  unchanged:  label "Max Value", input "Email"
```

---

7. Architecture

Cargo workspace - 6 crates + binary

```
farscry/ (virtual manifest)
├── crates/
│   ├── farscry-core/        types + traits + pHash + FarscryError
│   ├── farscry-ocr/         selector: CoreML (macOS) | ORT (all)
│   ├── farscry-classifier/  screen-type router + spatial rules
│   ├── farscry-diff/        bipartite matching + context gate
│   ├── farscry-formatter/   VASP text + JSON output
│   └── farscry-mcp/         UDS MCP server + 2 tools
├── crates/farscry/          binary CLI (orchestration only)
├── npm/                     postinstall wrapper
└── pip/                     hatch build hook wrapper
```

Dependency graph (strict, no cycles)

```
farscry (bin)
├── farscry-core         (no workspace deps)
├── farscry-ocr          -> farscry-core
├── farscry-classifier   -> farscry-core
├── farscry-diff         -> farscry-core
├── farscry-formatter    -> farscry-core
└── farscry-mcp          -> farscry-core ONLY
```

Pipeline

```
[image] -> Validator -> Preprocessor -> OCR Engine -> Classifier
        -> State Hasher -> Formatter -> stdout (VASP)
        stderr: progress, warnings, verbose
```

Key design decisions (validated)

| Decision | Choice | Rationale |
|---|---|---|
| Trait objects | `Arc<dyn Trait + Send + Sync + 'static>` | Rayon batch requires Arc (not Box) |
| MCP transport | Unix Domain Socket (default), TCP 127.0.0.1 (--port) | Security: UDS inaccessible over network |
| Daemon concurrency | `Arc<Mutex<Pipeline>>` | ONNX Runtime sessions are not Sync; serialize inference |
| Async/sync boundary | All inference in `tokio::task::spawn_blocking` | Never hold Mutex across `.await` |
| OCR output type | `OcrOutput { regions: Vec<TextRegion>, width: u32, height: u32 }` | Not "HocrOutput" - farscry uses PP-OCRv5, not Tesseract |
| VaspOutput | 12 fields: vasp_version, schema_version, state_id, screen_type, confidence, lang, delta_from, context_similarity, context_changed, agent_context, ui_tree, affordances | Defined in farscry-core §types |
| VaspDelta | 8 fields: vasp_version, diff_from, diff_to, context_similarity, context_changed, agent_context, entries, tokens_saved | DeltaEntry: Appeared|Removed|Changed{before,after}|Unchanged |
| BatchResult | `struct BatchResult { path: PathBuf, output: Result<VaspOutput, FarscryError> }` | Lazy decode inside rayon workers |
| FarscryError | Includes `LanguageNotInstalled(String)` -> maps to exit code 3 | Without this variant, `--lang` errors silently map to exit 1 |
| UiElement | Replaces `TypedElement` in final API - `UiElement { text, element_type, cx, cy, w, h, enabled, value }` | Cleaner name for the VASP output layer |
| ElementType | Includes `Select` variant | Required by `AffordanceAction::Select` |
| macOS OCR backend | `objc2-core-ml` (native) - NOT `oar-ocr` with coreml feature | oar-ocr CoreML bridge: ~298ms (no improvement). Native CoreML: 21ms. |
| state_id | `StateId([u8; 8])` newtype with Display impl | 64-bit pHash = 8 bytes. NOT `StateId([u8; 32])`, NOT `StateId(String)`. Display: `phash:<16-char-hex>` |
| Error handling | `thiserror` in library crates, `anyhow` in binary | Idiomatic Rust pattern |
| Batch input | `Vec<PathBuf>` (not `Vec<DynamicImage>`) | Lazy decode per rayon worker - prevents peak Nx8MB allocation |

---

8. Performance Targets

| Platform | Minimum | Target | Validated |
|---|---|---|---|
| macOS Apple Silicon (CoreML) | < 50ms | < 30ms | **21ms ** |
| macOS Intel (ORT A+B+C+D+F) | < 200ms | < 150ms | projected |
| Linux x86_64 (ORT A+B+C+D+F) | < 200ms | < 150ms | **~120ms projected** |
| Windows (ORT A+B+C+D+F) | < 200ms | < 150ms | projected |
| Batch 10 images (parallel) | < 2s | < 1s | - |
| Binary size | < 10MB | < 8MB | - |
| Models download (English) | < 15MB | < 13MB | **~12MB ** |
| Memory usage | < 150MB | < 100MB | - |
| Warm daemon startup | < 50ms | < 30ms | - |

x86 ORT optimizations shipping in v0.1.0

All are pure configuration changes - zero model changes, zero accuracy regression:

| ID | Optimization | Speedup contribution |
|---|---|---|
| A | `intra_threads = physical_cores` | thread tuning |
| B | `optimization_level = Level2` | graph optimization |
| C | 640px detection resize limit | det model speedup |
| D | `region_batch_size(32)` | rec model throughput |
| F | English model only (default) | smallest model set |

Deferred to v0.2.0: INT8 quantization (needs calibration dataset), DirectML (Windows GPU CI), oneDNN (no prebuilt .so in ORT releases).

---

9. Security Requirements (Non-Negotiable)

| Requirement | Implementation |
|---|---|
| Model integrity | SHA256 verify before every ONNX Runtime load |
| Tamper detection | Store SHA256 in `~/.farscry/.manifest.json`; re-verify on each run |
| MCP binding | UDS default; TCP only on 127.0.0.1 - never 0.0.0.0 |
| Input validation | Magic bytes check before any processing (PNG/JPG/WEBP/GIF) |
| Size limits | 10MB default, >=50px minimum dimension |
| npm postinstall | SHA256 checksum verify before execute |
| NOTICES.md | ONNX Runtime (MIT) attribution required in all distributions |
| Dependency audit | `cargo audit` in every PR and release |
| Config modification | `farscry setup` shows config snippet; user pastes manually - NEVER auto-modify |
| OmniParser exclusion | Zero code, zero weights, zero derived artifacts from AGPL-tainted sources |

---

10. Error Handling Contract

stdout is ALWAYS clean. VASP/JSON to stdout only. All errors, warnings, progress -> stderr.

```bash
farscry screen.png | agent "fix this"   # always safe to pipe
```

| Situation | Exit code | Behavior |
|---|---|---|
| File not found | 1 | stderr error |
| Not an image (PDF, MP4) | 1 | stderr + suggestion |
| Too large (>10MB) | 1 | stderr error |
| Too small (<50px) | 1 | stderr error |
| Corrupted image | 2 | stderr error |
| OCR failed | 2 | stderr error |
| Language not installed | 3 | stderr + `farscry --install-lang <code>` instruction |
| Low quality image | 0 | VASP with `confidence: low` + stderr warning |
| Blank image | 0 | VASP `screen_type: unknown`, `confidence: none` |
| Animated GIF | 0 | First frame extracted + stderr warning |

---

11. CLI Interface (complete)

```bash
Extract (Mode 1)
farscry screenshot.png
cat screenshot.png | farscry
farscry --from-clipboard

Diff (Mode 2)
farscry diff before.png after.png
farscry diff before.png after.png --agent    # compact delta

Batch
farscry *.png
farscry img1.png img2.png img3.png

Output
farscry screenshot.png --json
farscry screenshot.png -o context.vasp
farscry screenshot.png --affordances
farscry screenshot.png --context             # one-line agent_context only

Flags
farscry screenshot.png --text-only           # VASP only, no image to agent
farscry screenshot.png --lang por
farscry screenshot.png --lang eng+por
farscry screenshot.png --max-size 20mb
farscry screenshot.png -v
farscry screenshot.png --debug

Language
farscry --install-lang por
farscry --install-lang rus
farscry --install-lang chi_sim

Setup - shows config snippet, never auto-modifies
farscry setup

Daemon
farscry serve --mcp
farscry serve --mcp --port 3333

Version
farscry --version
```

---

12. MCP Server

Transport: Unix Domain Socket (default `~/.farscry/mcp.sock`); TCP 127.0.0.1 with `--port`.

Two tools exposed:

**farscry_extract**
```json
{
  "name": "farscry_extract",
  "description": "Converts any screenshot into VASP structured context for automation workflows",
  "parameters": {
    "image_path": { "type": "string" },
    "lang": { "type": "string", "default": "eng" },
    "affordances": { "type": "boolean", "default": true }
  }
}
```

**farscry_diff**
```json
{
  "name": "farscry_diff",
  "description": "Returns semantic delta between two screenshots - appeared, changed, removed",
  "parameters": {
    "before": { "type": "string" },
    "after":  { "type": "string" }
  }
}
```

**Agent configuration snippet (shown by `farscry setup`, pasted manually by user):**
```json
{
  "mcpServers": {
    "farscry": {
      "command": "farscry",
      "args": ["serve", "--mcp"]
    }
  }
}
```

**MCP daemon operation:** In `serve --mcp` mode, the agent calls `farscry_extract()` after each action. farscry stores the `state_id` and automatically diffs against the previous state. Agent receives the delta without managing before/after files. The user never manages before/after - that's the daemon's job.

---

13. Distribution - All Required Before Launch

| Channel | Requirement |
|---|---|
| `cargo install farscry` | Binary named `farscry` in crates.io |
| `npm install farscry` | postinstall downloads binary + SHA256 verify |
| `pip install farscry` | hatch build hook, same download pattern |
| `brew install teles-forge/farscry/farscry` | Homebrew tap |
| `curl -fsSL https://farscry.dev/install \| sh` | Cloudflare Worker |
| GitHub Actions CI | audit + fmt + clippy + test (4 platforms) |
| GitHub Actions release | 4 native runners, SHA256 per binary |
| crates.io | farscry-core + farscry (binary) published |

**Supported platforms:**
- `farscry-aarch64-apple-darwin` - Mac M1/M2/M3/M4
- `farscry-x86_64-apple-darwin` - Mac Intel
- `farscry-x86_64-unknown-linux-gnu` - Linux x86_64
- `farscry-x86_64-pc-windows-msvc` - Windows x86_64

---

14. Launch Criteria - Do Not Launch Until All True

```
□ cargo install farscry works (binary named farscry, not pipe)
□ npm install farscry works on Mac M1/M2/M3
□ npm install farscry works on Mac Intel
□ npm install farscry works on Linux x86_64
□ npm install farscry works on Windows
□ pip install farscry works on all above
□ farscry screen.png produces valid VASP output
□ farscry diff before.png after.png produces correct delta
□ farscry --from-clipboard works on macOS
□ farscry --from-clipboard works on Linux
□ farscry serve --mcp connects to MCP client
□ farscry serve --mcp connects to MCP client
□ Demo GIF recorded (< 15 seconds, real error, no narration)
□ farscry.dev live with final copy + benchmark numbers
□ vasp-protocol.github.io/spec live  (done)
□ Benchmark published in repo (N=40  done)
□ cargo audit passes
□ NOTICES.md exists with ONNX Runtime attribution
□ SHA256 checksums on every release binary
```

---

15. Accuracy Acceptance Criteria

Performance
| Metric | Minimum | Target |
|---|---|---|
| 1080p light-mode | < 300ms | < 200ms |
| 1080p dark-mode | < 350ms | < 220ms |
| Batch 10 images | < 2s | < 1s |

Field extraction accuracy (not character accuracy)
| Screen type | Minimum |
|---|---|
| error | 93% |
| config | 91% |
| terminal | 94% |
| conversation | 89% |
| ui | 87% |
| Screen type classification | 85% *(amended from 92% - screen-type router validated at 89.4% OOD, 2026-05-08)* |
| Dark mode | 88% |
| HiDPI / Retina | 90% |

Diff accuracy
| Scenario | Minimum |
|---|---|
| Appeared correctly identified | 95% |
| Changed correctly identified | 92% |
| Removed correctly identified | 95% |
| False positives | < 3% |

---

16. Out of Scope (v0.1.0)

| Feature | Version |
|---|---|
| WASM build | v0.2.0 |
| INT8 static quantization | v0.2.0 |
| DirectML (Windows GPU) | v0.2.0 |
| oneDNN (Intel CPU) | v0.2.0 |
| Windows --from-clipboard | v0.2.0 |
| farscry serve --http | v0.2.0 |
| Visual element classifier (MobileNetV3) | v0.2.0 |
| Docker image | v0.3.0 |
| Zendesk / Intercom integrations | v0.3.0 |
| VASP adapters (cloud vision -> VASP) | v0.3.0 |
| PDF support | v0.4.0 |
| Animated GIF (all frames) | v0.4.0 |

---

17. Story Dependency Order

Stories must be created and implemented in this order. Each is blocked by its predecessor.

| # | Story | Blocked by |
|---|---|---|
| 1 | farscry-core | - |
| 2 | farscry-ocr-coreml | 1 |
| 3 | farscry-ocr-ort | 1 |
| 4 | farscry-ocr (selector) | 2, 3 |
| 5 | farscry-classifier | 1 |
| 6 | farscry-diff | 1 |
| 7 | farscry-formatter | 1 |
| 8 | farscry-mcp | 1 |
| 9 | farscry binary (CLI + clipboard + setup) | 4, 5, 6, 7, 8 |
| 10 | Distribution (CI/CD + npm + pip + Homebrew) | 9 |
| 11 | Site update (farscry.dev final copy + GIF) | 9 |
| 12 | Launch | 10, 11 + all launch criteria met |

---

18. Benchmark (Published - N=40)

Benchmark already executed and documented in `farscry-technical-report.md`:

- **N=40** evaluations (20 screenshots x 2 runs)
- **farscry VASP accuracy:** 100% (20/20)
- **cloud vision accuracy:** 100% (20/20)
- **Token reduction:** 3.7x avg (800px) - 8.9x on 1080p
- **farscry OCR:** 162-1,687ms (dev build); **21ms CoreML release**

Scripts in `spike/benchmark/` are reproducible by anyone with a MCP client API key.

---


