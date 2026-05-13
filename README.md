# farscry

> Converts screenshots into structured context that AI agents can act on.
> No vision API. No cloud. One binary.

*farscry (n.) a magical artifact that reveals what is hidden at a distance.*

**Problem**: Devin CLI, Claude Code, and Cursor struggle with images.
Give them a screenshot, they guess. Give them farscry output, they understand.

## Benchmark

| Tool | Time | Cost/image | Offline | Coordinates |
|---|---|---|---|---|
| **farscry (warm)** | **38ms** | **$0** | **✅** | **✅** |
| **farscry (cold)** | **~350ms** | **$0** | **✅** | **✅** |
| Tesseract 5.5.2 (4K) | ~2,500ms | $0 | ✅ | ❌ |
| Cloud Vision | ~2-5s | $0.0047 | ❌ | ❌ |

N=223 screenshots (ScreenSpot-Pro, MIT). Warm daemon measured independently on M4 Pro (CoreML).

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

## Smart paste

Configure Cmd+V to auto-detect images in terminal:

```bash
farscry setup
# -> detects your agents (claude, devin, codex, aider)
# -> configures ffix alias for your preferred agent
# -> asks: configure smart Cmd+V? (y/N)
# -> creates ~/.farscry/smart-paste.sh
# -> shows key binding instructions for your terminal
```

After setup: screenshot -> Cmd+V -> agent understands. No command to type.

Supported terminals:
- macOS: iTerm2, Warp (Terminal.app: use `fp` alias instead)
- Linux: Kitty, Gnome Terminal
- Windows: Windows Terminal

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

### Setup (recommended)
```bash
farscry setup
```
Detects claude, devin, codex, aider. Shows the alias to add and MCP config to paste. Saves your preferred agent to `~/.farscry/config.toml`.

### Zero-friction workflow
```bash
echo "alias fp='farscry paste'" >> ~/.zshrc && source ~/.zshrc

# Every time after: screenshot → fp → done
fp
fp "explain this error"
fp --agent devin
```

### Claude Code
```bash
farscry extract screen.png | claude -p "fix this"
farscry extract --from-clipboard | claude -p "fix this"
```

### Devin
```bash
devin -p "$(farscry extract screen.png): fix this"
devin -p "$(farscry extract --from-clipboard): fix this"
```

### Codex
```bash
farscry extract screen.png | codex exec "fix this:"
farscry extract --from-clipboard | codex exec "fix this:"
```

### MCP (all agents)
```bash
farscry serve --mcp
```
Supports multiple images via `image_paths` parameter.

### Supported image formats
PNG, JPEG, GIF, WEBP, TIFF. From clipboard, file, or stdin.
From clipboard: Cmd+Shift+4, Shottr, or Cmd+C on an image file in Finder.

## License

Apache 2.0
