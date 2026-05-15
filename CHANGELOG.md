# Changelog

## [0.4.0] — unreleased

## [0.3.0] — 2026-05-15

### Added
- `farscry hook`: zero-friction sidecar terminal recording via shell hook (`farscry setup --hook`)
  - Automatically records every terminal session with <1% CPU overhead
  - 18KB/min disk, ~22MB RAM (single daemon for all terminals)
- `farscry record --daemon`: background screen capture daemon with pHash deduplication
- `farscry session --list` / `--latest`: list and inspect recorded sessions
- `farscry daemon`: global single-daemon architecture — N terminals = 1 process
- `farscry hook --init` / `--remove`: manage shell hook lifecycle
- Window-specific capture via `CGWindowListCreateImage` — works when terminal is minimized or behind other windows

### Fixed
- Screen capture now targets the specific terminal window, not the entire display
- Graceful daemon shutdown — VASF header finalized on SIGTERM/SIGINT
- Screen Recording permission dialog shown correctly on first run

## [0.2.0] — 2026-05-15

### Added
- `farscry pack`: compress screenshot directories to `.vasf` with pHash deduplication — 160x token reduction measured on Retina display
- `farscry timeline`: replay a `.vasf` session as a state-change timeline
- `farscry info`: session statistics — unique states, dedup percentage, token reduction ratio
- `farscry annotate`: bounding box visualization overlay on screenshots
- `farscry serve --mcp --record <path>`: session recording via MCP server with automatic deduplication
- Smart paste: `farscry setup` now auto-configures Cmd+V in all detected terminals (iTerm2, Warp, Kitty, Alacritty)
- `farscry setup --undo-smart-paste`: restore all terminal configs from backup

### Changed
- `farscry serve` now supports optional `--record` flag for automatic VASF recording

## [0.1.0] — 2026-04-XX

### Added
- `farscry extract`: screenshot → structured VASP text (OCR + element classification), 15.5x token reduction
- `farscry diff`: semantic diff between two screenshots
- `farscry serve --mcp`: MCP server for agent integration
- `farscry paste`: smart paste with OCR-to-clipboard pipeline
- `farscry setup`: interactive CLI setup wizard
- `farscry install-lang`: install additional OCR language models
- VASP (Visual Agent State Protocol) structured output format
- CoreML (Apple Neural Engine) and ONNX Runtime backends
