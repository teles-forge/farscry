# farscry v0.3.0 - Roadmap

**Status:** In Progress
**Target:** Q4 2026
**Author:** Darlysson Teles

---

## Strategic context

v0.2.0 fulfilled the promises made in v0.1.0: multi-language, annotate,
Windows clipboard, VASP adapters.

v0.3.0 is the version that changes the product category.

v0.1.0 was: "local OCR that gives agents typed coordinates."
v0.3.0 is: "the observability layer for computer-use agents."

The shift: from a tool you call manually to infrastructure that runs
automatically alongside any agent session.

The model is Datadog APM: one-line install, zero configuration, imperceptible
overhead, full session data available when something fails.

---

## The problem this version solves

Computer-use agents have no observability layer.

When an agent fails at step 47, you have:
- LLM call logs (what the agent said, not what it saw)
- Maybe a folder of screenshots with no structure
- No way to diff a passing session against a failing one
- No record of which UI states the agent actually observed

Every existing observability tool (LangSmith, Langfuse, AgentOps) tracks
reasoning and LLM calls. None tracks visual observation. This is the gap.

---

## Feature 1 — farscry hook (CRITICAL PATH)

### farscry setup --hook

One-time setup. Adds `eval "$(farscry hook init)"` to the shell rc file.
Creates `~/.farscry/sessions/` directory.

After setup: every new terminal session auto-starts `farscry record --daemon`
in the background. The agent does not know farscry is running.

```bash
farscry setup --hook

farscry hook installed in ~/.zshrc
Sessions saved to: ~/.farscry/sessions/
Overhead: <1% CPU  ~18KB/min disk  ~20MB RAM
Open a new terminal to start recording.
```

### farscry hook init

Outputs shell functions to stdout. Eval'd by the shell on startup.
Handles: session start, session stop on EXIT trap, PID tracking.

### farscry hook remove

Removes the eval line. Restores rc file from backup.

**Files:**
- `crates/farscry/src/commands/hook.rs` (new)
- `crates/farscry/src/main.rs` — add Hook subcommand

**Acceptance criteria:**
- `farscry setup --hook` runs without error on macOS, Linux, Windows
- New terminal session starts daemon in background
- `ps aux | grep farscry` shows the daemon process
- Closing terminal stops recording, .vasf is valid

---

## Feature 2 — farscry record --daemon (CRITICAL PATH)

OS-level screen capture. Three-thread architecture:

**Thread 1 — Capture (1fps timer)**
- macOS: `core-graphics` crate, `CGDisplayCreateImage`
- Linux + Windows: `scrap` crate
- Writes to ring buffer (capacity 3). Drops silently if full. Never blocks.

**Thread 2 — pHash (reads ring buffer)**
- Uses existing `StateHasher` from `farscry-core`
- Hamming distance <= 10: writes timeline entry only (12 bytes)
- Hamming distance > 10: sends frame to Thread 3

**Thread 3 — OCR (reads channel)**
- Runs existing `Pipeline::process`
- Writes to .vasf: state_table + state_data + timeline

**New VasfWriter (append mode):**
- Header written once at start (frame_count = 0)
- STATE_TABLE and STATE_DATA grown incrementally
- TIMELINE appended 12 bytes at a time
- `finalize()`: rewrites header with final counts
- Crash-safe: file readable at any point mid-session

**Flags:** `--daemon`, `--fps N` (default 1), `--output PATH`,
`--silent`, `--threshold N` (default 10)

**Files:**
- `crates/farscry/src/commands/record.rs` (new)
- `crates/farscry-core/src/vasf.rs` — add VasfWriter struct

**Dependencies (platform-gated):**

```toml
[target.'cfg(target_os = "macos")'.dependencies]
core-graphics = "0.23"

[target.'cfg(not(target_os = "macos"))'.dependencies]
scrap = "0.5"
```

**Acceptance criteria:**
- 3 minutes of any agent session produces a .vasf with frames
- CPU overhead < 1% on Apple Silicon at 1fps
- Kill -9 scenario: .vasf readable up to last complete write
- `farscry timeline` on the output shows frames in order

---

## Feature 3 — farscry session list / latest

```bash
farscry session list

2026-05-13-2130.vasf   10m 14s   180 frames   23 unique   87% dedup   180KB
2026-05-13-1045.vasf    5m 02s    89 frames   11 unique   88% dedup    94KB

farscry session latest
# equivalent to: farscry timeline on most recent .vasf
```

`session list` reads only headers (fast, no STATE_DATA loaded).

**Files:**
- `crates/farscry/src/commands/session.rs` (new)

---

## Feature 4 — farscry serve --mcp --record (extension of existing MCP)

Adds `--record PATH` flag to the existing MCP server.

Every `farscry_extract` call received via MCP during the session is
accumulated into the .vasf using the same VasfWriter as Feature 2.

The difference from `record --daemon`: frames arrive as file paths from the
agent (not captured from the OS). pHash computed the same way.

For agents that already use farscry MCP: zero code changes. Add `--record`
to the startup command.

```bash
farscry serve --mcp --record ~/.farscry/sessions/current.vasf
```

**Files:**
- `crates/farscry-mcp/src/lib.rs` — add --record flag, VasfWriter integration

---

## Feature 5 — farscry pack (ALREADY IMPLEMENTED — feat/vasf)

Status: implemented, tests passing, 74x measured on real data, 88% dedup.

What remains before v0.3.0 ships:
- Benchmark on real 180-frame session (not 25-frame test)
- Public dataset available for benchmark reproduction
- The measured number (not estimated) published in README and benchmark/

---

## Feature 6 — farscry diff session-a.vasf session-b.vasf

Compares the unique state sequences of two sessions.

Uses existing VaspDelta logic. Wraps it to operate on session pairs:
for each state that appears in both sessions, runs the diff engine.
Reports states that appeared only in one session (regressions).

```bash
farscry diff session-passed.vasf session-failed.vasf

State 12:
  before: Button "Save"  enabled:true
  after:  Button "Save"  enabled:false
  new:    Label "Card number invalid" appeared at (412, 340)

State 19 present in passed, absent in failed.
State 22 absent in passed, new in failed: screen_type=Error
```

**Files:**
- `crates/farscry/src/commands/diff.rs` — extend existing diff command for .vasf input

---

## Feature 7 — VASF spec + VASP stream spec published

`vasp-protocol.github.io/vasf`: complete binary format spec.
`vasp-protocol.github.io/spec/stream`: VaspStream and VaspFrame (VASP v1.1).

Both specs must be published and linkable on the day of the HN post.
Any third party implementing VASF in another language must be able to do so
from the spec alone, without reading the Rust source.

---

## Feature 8 — Benchmark dataset public

Before the HN post:
- A real 10-minute browser-use or claude computer-use session
- Frames available for download (no login, no form, no email)
- Command to reproduce: `farscry pack sample-session/ -o result.vasf`
- The real measured number (replace the estimated 105x with the actual result)

The number posted on HN is the number from this dataset. If it is 74x, post
74x. Never post an estimated number as a measured one.

---

## What does NOT enter v0.3.0

OTLP export and webhook on failure: v0.4.0.
Enterprise config (auto-record, field redaction): v0.4.0.
farscry watch (continuous diff on region): v0.4.0.
SDK native clients: v0.4.0.
farscry cloud: v1.0.0 if there is demand.

Scope discipline is what shipped v0.1.0 and v0.2.0. Same rule applies.

---

## Implementation order

1. VasfWriter (append mode) in farscry-core — all features depend on this
2. farscry record --daemon — the core of the hook story
3. farscry hook (setup, init, remove) — the UX layer on top of record
4. farscry session (list, latest) — the inspection layer
5. farscry serve --mcp --record — MCP extension
6. farscry diff session.vasf — session comparison
7. Benchmark on 180-frame real session
8. VASF + VASP stream specs published
9. Public dataset upload
10. HN post

---

## Branch and commit plan

```
git checkout -b feat/hook-record
# VasfWriter + record --daemon + hook + session

git checkout -b feat/mcp-record
# farscry serve --mcp --record extension

git checkout -b feat/session-diff
# farscry diff session-a.vasf session-b.vasf
```

Version bump when all features ship:
```
version = "0.3.0"
git tag v0.3.0
```

---

## What to document after each feature

1. Update README.md (commands section and roadmap)
2. Update VASP spec if any protocol fields change
3. Update benchmark/README.md with new measurements
4. When farscry-site is created: CLI reference and observability docs
