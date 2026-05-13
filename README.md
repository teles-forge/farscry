# farscry

> Converts screenshots into structured context that AI agents can act on.
> No vision API. No cloud. One binary.

*farscry (n.) a magical artifact that reveals what is hidden at a distance.*

**Problem**: Devin CLI, Claude Code, and Cursor struggle with images.
Give them a screenshot, they guess. Give them farscry output, they understand.

## Benchmark

| Tool | Time | Cost/image | Works offline |
|------|------|------------|---------------|
| **farscry (warm)** | **38ms** | **$0** | **✅** |
| **farscry (cold)** | **~350ms** | **$0** | **✅** |
| Tesseract 5.5.2 (4K) | ~2,500ms | $0 | ✅ |
| Cloud Vision | ~2-5s | $0.0047 | ❌ |

N=223 screenshots (ScreenSpot-Pro, MIT). Warm daemon measured independently on M4 Pro (CoreML).

*farscry wins on speed, cost, and agent-readiness.*

## Install

```bash
# npm (global)
npm install -g farscry

# pip
pip install farscry

# curl (any platform)
curl -fsSL https://farscry.dev/install | sh
```

## Quick start

```bash
# Describe any image, returns coordinates
farscry screen.png

# Diff before/after an action
farscry diff before.png after.png

# Pipe from clipboard (Cmd+Shift+4 on macOS)
farscry --from-clipboard | your-agent "fix this"

# Run as MCP server
farscry serve --mcp
```

## How it works

```
[image] → [binarize] → [layout detect] → [OCR per region] → [classify] → [VASP output]
```

Detects: error messages · UI fields + values · terminal output · conversations · config screens

## Output (VASP format)

~175 tokens average. Typed elements with exact pixel coordinates, not descriptions.

```
=== farscry visual context ===
screen_type: config
---

[mid-right]  button  "Save Changes"  enabled:true
[mid-center] input   value="1500"    editable:true
[bottom]     error   "Value must be ≤ 10000"

affordances:
  click → "Save Changes"  at (400,300)
  type  → "Max Value"     at (200,120)  current:"1500"
```

## Agent integrations

### Claude Code / Cursor / Windsurf / Zed (MCP)

```bash
farscry setup
```

Auto-detects your agent and shows the config snippet to paste.

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

### Any terminal agent (pipe)

```bash
farscry --from-clipboard | claude "fix this"
farscry --from-clipboard | devin "fix this"
farscry screen.png | codex "fix this"
```

### Devin Web (API preprocessing)

```bash
vasp=$(farscry screen.png)
```

Include `$vasp` in your Devin prompt before sending.

## License

Apache 2.0
