# Show HN: farscry – I was tired of agents guessing where to click

---

## Title

Show HN: farscry – I was tired of agents guessing where to click

vision APIs describe. farscry gives coordinates.

---

## Body

```
farscry screenshot.png
```

```
=== farscry visual context ===
screen_type: config
---
[middle-right]  button  "Save Changes"  enabled:true  at (400,300)
[middle-center] input   value="1500"    editable:true at (200,120)
[bottom]        error   "Value must be <= 10000"       at (20,350)

affordances:
  click -> "Save Changes" at (400,300)
  type  -> "Max Value"    at (200,120)  current:"1500"
```

---

Vision APIs return prose. Agents guess where to click.
They fail 58% of the time (OSWorld, arXiv:2404.07972, GPT-4V baseline, Table 2).

farscry gives exact typed coordinates from any image.
Local. Free. Deterministic. No API key. No GPU.

---

**Why I built this:**

Devin and Claude Code struggle with images.
Give them a screenshot, they describe it in prose.
Give them farscry output, they know exactly what to click.

I wanted to pipe screenshots directly into my agent
without sending them to a cloud API every time.

---

**Four modes:**

1. **Extract** — any image -> typed coordinates

```bash
farscry error.png | claude "fix this"
farscry figma.png | claude "build this component"
```

2. **Diff** — what changed between two states

```bash
farscry diff before.png after.png
# appeared: error "Card declined"
# changed: button "Submit" -> "Processing..." disabled
# ~175 tokens vs 3,136 re-sending both images
```

3. **Clipboard alias** — typed command, one word

```bash
ffix  # after: farscry setup
```

`farscry setup` adds `ffix` to your shell.
`ffix` = `farscry extract --from-clipboard | claude -p "fix this"`
Use when: you want a typed command.

4. **Smart paste** — Cmd+V auto-detects images

After `farscry setup`, Cmd+V in your terminal checks the clipboard.
Image? Runs farscry. Text? Normal paste.

Screenshot -> Cmd+V -> done. No command to type.
Use when: you want zero typing.

---

**Benchmarks** (N=223 real screenshots, ScreenSpot-Pro MIT, reproducible):

| Tool | Time | Tokens/image | Coordinates | Cost |
|---|---|---|---|---|
| farscry daemon | 38ms | ~175 | Yes | $0 |
| Tesseract 4K | ~2,500ms | raw text | No | $0 |
| Cloud Vision | ~2-5s (network-dependent) | ~1,568 | No | $0.0047 |

65x faster than Tesseract on 4K screens.
9x fewer tokens than Cloud Vision on 1080p.
100% accuracy parity with Cloud Vision (N=20 screenshots, 2 runs each — small sample, manual verification).

Run it yourself: github.com/teles-forge/farscry/tree/main/benchmarks

---

**VASP (Visual Agent State Protocol)** — the open standard behind farscry.

Like MCP standardized tool connectivity,
VASP standardizes visual context for agents.

Any tool can output VASP. Any agent can consume it.
spec: vasp-protocol.github.io/spec

---

**What it doesn't do:**

- Icon-only buttons (no text label): missed
- Charts, graphs, diagrams: no structured output
- `--from-clipboard` on Linux: requires xclip installed
- Windows: binary ships, clipboard not yet implemented
- Not a visual grounding model — farscry is fast and local OCR, not ML-based semantic understanding
- Element classification accuracy on complex UIs: 89.4% OOD, not 100%

---

**Install:**

```bash
npm install -g farscry
# or: pip install farscry
# or: brew install teles-forge/farscry/farscry

farscry setup  # auto-configures Claude Code, Cursor, Windsurf
# setup asks: configure smart Cmd+V? (y/N)
# if yes: shows your terminal's key binding instructions
# result: Cmd+V auto-detects images in terminal
```

---

GitHub: github.com/teles-forge/farscry
Site: farscry.dev
VASP spec: vasp-protocol.github.io/spec
Benchmark methodology: github.com/teles-forge/farscry/benchmarks

Built with Rust. Apache 2.0.

---

## Comment responses (pre-drafted)

**"Why not just use Tesseract?"**

Tesseract returns raw text. farscry returns typed UI elements with coordinates and states (enabled/disabled, current values). Tesseract at 4K takes ~2,500ms. farscry warm daemon is 38ms. They solve different problems — Tesseract does OCR, farscry does UI understanding.

**"How does latency compare to cloud vision?"**

Cloud Vision typically takes 2-5s per image and costs ~$0.0047/image. farscry warm daemon is 38ms at $0. The 38ms is measured on M4 Pro with CoreML. x86 with ORT backend is ~300ms warm.

**"What's VASP?"**

VASP (Visual Agent State Protocol) is an open format for how agents receive visual context — typed elements with coordinates, not prose. Same positioning as MCP for tool connectivity, but for visual state. farscry is the reference implementation. Spec at vasp-protocol.github.io/spec.

**"Does it work on Linux/Windows?"**

Yes. Four pre-built binaries: macOS arm64 (CoreML, 38ms warm), Linux x64, Windows x64 (ORT backend, ~300ms warm). The npm and pip packages auto-download the correct binary on install.

**"What about accuracy on complex UIs?"**

96% success rate across N=223 real professional screenshots from ScreenSpot-Pro (Android Studio, macOS, Windows 11, Linux). The 4% failures are icon-heavy screens with no detectable text. Full breakdown in benchmarks/README.md.

**"Why Rust?"**

Zero runtime dependencies. Single ~8MB binary. Ships via npm, pip, Homebrew, and curl without dragging in Python or a runtime. CoreML and ONNX Runtime bindings exist for Rust. The binary is the distribution unit.

**"What's the diff token count based on?"**

Measured. A typical 1080p screenshot renders to ~1,568 tokens via Claude's image encoding formula (512 base + tiles). VASP text output averages ~175 tokens across N=223. farscry diff produces ~100 tokens for a partial-change verification. Numbers in benchmarks/README.md.

**"Where does the 58% failure rate come from?"**

OSWorld benchmark (arXiv:2404.07972), GPT-4V baseline, Table 2. farscry doesn't claim to fix this directly — it gives agents exact coordinates instead of prose descriptions, which addresses the root cause of most coordinate errors.
