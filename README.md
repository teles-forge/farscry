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
farscry extract screen.png

# Diff before/after an action
farscry diff before.png after.png

# Pipe from clipboard (Cmd+Shift+4 on macOS)
farscry extract --from-clipboard | your-agent "fix this"

# Run as MCP server
farscry serve --mcp
```

## Visual debug — annotate any screenshot

```bash
farscry annotate screenshot.png -o annotated.png
# or from clipboard:
farscry annotate --from-clipboard -o /tmp/out.png && open /tmp/out.png

# Add alias for one-command visual debug:
alias fannot='farscry annotate --from-clipboard -o /tmp/farscry_annotated.png && open /tmp/farscry_annotated.png'
```

Then: Shottr screenshot -> `fannot` -> annotated image opens automatically.

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

[middle-right]  button  "Save Changes"  enabled:true
[middle-center] input   value="1500"    editable:true
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

## Roadmap

**v0.1.0** (released)
- Extract: screenshot to typed VASP output, 38ms warm daemon
- Diff: semantic delta between two screenshots
- MCP server, smart paste, ffix alias
- npm, pip, Homebrew, crates.io
- VASP 1.0-draft open RFC

**v0.2.0** (in planning)
- Multi-language OCR (Portuguese, Spanish, German, Japanese)
- `farscry annotate` - screenshot with bounding boxes drawn over elements
- Windows clipboard support
- VASP adapters: Claude computer-use, Playwright, OpenAI vision

**v0.3.0** (planned)
- `farscry watch` - continuous diff on screen region changes
- Loop detection via state_id history in daemon
- SDK native clients (no subprocess overhead)

Full spike docs: [docs/projects/roadmap-v0.2.0.md](docs/projects/roadmap-v0.2.0.md)

## License

Apache 2.0
