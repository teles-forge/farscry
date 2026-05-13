farscry

> Image interpreter for automation workflows - local, offline, 8x fewer tokens

```bash
npm install -g farscry
farscry screenshot.png
```

```
=== farscry visual context ===
source: screenshot.png
screen_type: ui
state_id: phash:7f1f7e7f7f7f7f7e
confidence: high
lang: eng
agent_context: "Payment Settings • Help • Logout"
---
[[top-right] ]  label   "Help"
[[top-right] ]  label   "Logout"
[[top-left]  ]  label   "Payment Settings"
[[top-left]  ]  input   "Max Value"  value:"1500"
[[bottom-left]]  button  "Save Changes"
```

What it does

`farscry` converts screenshots into typed, coordinate-rich text that automation workflows can act on directly - without vision APIs, without API keys, and with 8x fewer tokens than sending raw images.

It speaks [VASP (Visual Application State Protocol)](https://vasp-protocol.github.io/spec) - a structured format designed for agent->UI interaction.

Install

```bash
npm (this package)
npm install -g farscry

pip
pip install farscry

Homebrew
brew install teles-forge/farscry/farscry

curl
curl -fsSL https://farscry.dev/install | sh
```

Usage

```bash
Extract VASP from image
farscry screenshot.png

JSON output
farscry screenshot.png --json

Diff two screenshots
farscry diff before.png after.png

Read from clipboard (macOS)
farscry --from-clipboard

MCP server mode
farscry serve --mcp

Setup guide
farscry setup
```

MCP integration

Add to `.claude/mcp.json`:
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

Platforms

| Platform | Architecture | Status |
|----------|-------------|--------|
| macOS | Apple Silicon (arm64) |  Native CoreML, 21ms |
| macOS | Intel (x64) |  |
| Linux | x86_64 |  |
| Windows | x86_64 |  |

License

Apache-2.0. See [NOTICES](https://github.com/teles-forge/farscry/blob/main/NOTICES.md) for ONNX Runtime attribution.
