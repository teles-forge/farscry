# Show HN: farscry — screenshots give AI agents coordinates, not descriptions

**Title:** Show HN: farscry — converts screenshots into typed coordinates for AI agents (offline, 38ms, $0)

---

## Body

I built farscry to solve a problem I kept hitting with Devin and Claude Code: give them a screenshot, they describe it. They don't know where to click.

farscry converts any screenshot into structured output with exact pixel coordinates — locally, in 38ms warm, at $0.

```
$ farscry screen.png
→ button  "Save"        at (400,300)  enabled:true
→ input   "Card number" at (300,200)  empty:true
→ error   "Card declined" at (20,350)
```

**The numbers (measured):**

- 38ms warm pipeline on M4 Pro (CoreML)
- ~350ms cold CLI (new process, model init included)
- ~9x fewer tokens than cloud vision at 1080p
- ~15x fewer tokens at 4K (N=223, ScreenSpot-Pro MIT dataset)
- 96% success rate across Android Studio, macOS, Windows 11, Linux
- 100% accuracy parity with cloud vision (N=20 screenshots, 2 runs each)
- $0.0047 per image with Claude Vision → $0 with farscry
- 10,000 images/day: saves ~$47/day

**Three modes:**

```bash
# 1. Describe — any image → coordinates
farscry screen.png

# 2. Diff — what changed between states
farscry diff before.png after.png
→ button "Submit": disabled → enabled
→ spinner: removed

# 3. Pipe from clipboard
farscry --from-clipboard | your-agent "fix this"
```

**MCP server** — agents call it directly:

```bash
farscry serve --mcp
```

Config snippet for Claude/Cursor/Windsurf:
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

**Agent integrations:**

```bash
farscry setup
```

Auto-detects Claude Code, Cursor, Windsurf, or Zed and shows the config snippet to paste.

```json
{
  "mcpServers": {
    "farscry": { "command": "farscry", "args": ["serve", "--mcp"] }
  }
}
```

Any terminal agent via pipe:

```bash
farscry --from-clipboard | claude "fix this"
farscry --from-clipboard | devin "fix this"
farscry screen.png | codex "fix this"
```

**Install:**

```bash
npm install -g farscry
# or
pip install farscry
# or
curl -fsSL https://farscry.dev/install | sh
```

**The output format is VASP** (Visual Application State Protocol) — an open spec at vasp-protocol.github.io/spec. Same idea as MCP for tool connectivity, but for visual state. farscry is the reference implementation.

**Benchmarks are reproducible:** github.com/teles-forge/farscry/tree/main/benchmarks

Apache 2.0. No account needed. Models download once to ~/.farscry/models/ (~12MB English).

---

## Comment responses (pre-drafted)

**"Why not just use Tesseract?"**

Tesseract returns raw text. farscry returns typed UI elements with coordinates, states (enabled/disabled), and affordances (what you can click or type). Tesseract at 4K takes ~2,500ms. farscry warm is 38ms. They solve different problems.

**"How does latency compare to cloud vision?"**

Cloud vision typically takes 2-5 seconds per image and costs ~$0.0047 per image. farscry warm daemon is 38ms and costs $0. At 10,000 images/day that's ~$47/day saved.

**"What's VASP?"**

VASP (Visual Application State Protocol) is an open format spec for how AI agents receive visual context — typed elements with coordinates, not natural language descriptions. Think of it like MCP but for visual state. Spec is at vasp-protocol.github.io/spec.

**"Does it work on Linux/Windows?"**

Yes. Four pre-built binaries: macOS arm64 (CoreML, 38ms warm), macOS x64, Linux x64, Windows x64 (ORT backend, ~300ms — no CoreML on non-Apple). The npm/pip packages auto-download the right binary.

**"What about accuracy on complex UIs?"**

96% success rate on N=223 real professional screenshots (ScreenSpot-Pro dataset — Android Studio, macOS, Windows, Linux). The 4% failures are icon-heavy screens with no detectable text. Full breakdown in benchmarks/README.md.
