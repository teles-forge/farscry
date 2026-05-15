# farscry

The observability layer for computer-use agents.

[![Version](https://img.shields.io/crates/v/farscry)](https://crates.io/crates/farscry)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue)](LICENSE)
[![npm](https://img.shields.io/npm/v/farscry)](https://www.npmjs.com/package/farscry)

## Overview

farscry records what agents see, detects when they fail silently, and lets you analyze session recordings without sending a single pixel to any external service. It converts screenshots to structured VASP text for agent context, and stores sessions as deduplicated `.vasf` files. The full pipeline runs locally — no cloud, no data egress.

## Quick Start

```bash
farscry setup --hook
farscry extract screen.png
farscry serve --mcp
```

## What It Records

Sessions are stored as `.vasf` files (Visual Agent Session File). Each file contains only the frames that are perceptually unique — farscry computes a 63-bit DCT perceptual hash for every frame and discards any frame within Hamming distance 10 of the previous stored state. On a real Retina session (3600×2338), 89% of frames were identical: `farscry pack` stored 1 frame in 160.

Each stored frame carries:
- A `state_id` (hex hash of the pHash)
- A VASP snapshot: typed UI elements with pixel coordinates
- A timestamp and terminal PID

## CLI Reference

| Command | Description | Example |
|---------|-------------|---------|
| `extract` | Convert a screenshot to VASP structured text | `farscry extract screen.png` |
| `diff` | Semantic delta between two screenshots | `farscry diff before.png after.png` |
| `annotate` | Render bounding boxes onto a screenshot | `farscry annotate screen.png -o out.png` |
| `pack` | Pack a directory of screenshots into a `.vasf` file | `farscry pack shots/ -o session.vasf` |
| `timeline` | Print the sequence of unique states in a session | `farscry timeline session.vasf` |
| `info` | Print metadata for a `.vasf` file | `farscry info session.vasf` |
| `serve` | Run the MCP server | `farscry serve --mcp` |
| `hook` | Install or remove the terminal recording hook | `farscry hook --remove` |
| `session` | List and inspect recorded sessions | `farscry session --list` |
| `record` | Start a recording session manually | `farscry record --daemon --global --pid $$ --silent` |
| `daemon` | Manage the global recording daemon | `farscry daemon unregister $$` |

## MCP Integration

Add to your agent's MCP configuration:

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

The agent sends a screenshot path; farscry returns structured VASP text. No pixels leave the machine.

## Install

```bash
cargo install farscry
```

```bash
brew install farscry
```

```bash
npm install -g farscry
```

```bash
pip install farscry
```

```bash
curl -fsSL https://farscry.dev/install | sh
```

## Performance

| Metric | Platform | Value |
|--------|----------|-------|
| Token reduction — OCR + structured output | ScreenSpot-Pro, N=223 | **15.5×** |
| Token reduction — session deduplication | Retina 3600×2338, real session | **160×** |
| Warm daemon response time | macOS M4 Pro, CoreML | **38 ms** |
| Daemon RSS — all terminals, one process | macOS | **22 MB** |
| Daemon VmRSS | Linux, Docker + Xvfb | **11 MB** |

## Roadmap

### v0.5.0

- `farscry augment`: inject silent failure warnings directly into agent context via MCP — zero code changes to the agent
- `farscry watch session.vasf --detect`: real-time silent failure and visual loop detection
- Semantic export: webhook, Slack, JSONL log on session failure — never sends pixels, always structured text
- `farscry watch-dir <path>`: file-system watch for agent screenshot directories (FSEvents/inotify)
- `farscry diff --json`: structured diff output for tooling integration

### v0.6.0

- VASP adapters: native Playwright and OpenAI Vision support (currently stubs)
- `farscry install-lang`: multilingual OCR models via CDN — Portuguese, Chinese, Japanese, Russian, Korean, Arabic
- Per-window capture works when minimized or behind other windows (SCContentFilter)
- `farscry serve` screen-lock awareness: maintains last StateId when display sleeps

## License

Apache 2.0
