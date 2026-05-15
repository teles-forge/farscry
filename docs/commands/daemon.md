# farscry daemon

Single global recording daemon — one process for all terminals.

## Architecture

One daemon per machine handles all registered terminals.
N terminals → 22 MB RSS total (not N × 22 MB).

## Usage

The daemon starts automatically when you run `farscry setup --hook`.

For explicit control:

```bash
farscry record --daemon --global --pid $$ --silent
farscry daemon unregister $$
```

## IPC

The daemon uses a Unix socket at `~/.farscry/daemon.sock`.
Terminals register via `REGISTER <pid>`, unregister via `UNREGISTER <pid>`.

## macOS

Uses ScreenCaptureKit SCStream (macOS 12.3+).
32×32 output requested from GPU — no full-resolution copy to heap.
Steady-state RSS: 22 MB.

## Linux

Uses scrap (XCB/X11 shared memory).
Requires `DISPLAY` environment variable.
Works in Docker with Xvfb. Steady-state VmRSS: 11 MB.
