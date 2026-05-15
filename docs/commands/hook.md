# farscry hook

Records every terminal session automatically.

## Setup

```bash
farscry setup --hook
```

Opens a new terminal. Recording starts automatically.

## What it records

Each terminal session creates a `.vasf` file in `~/.farscry/sessions/`.
pHash deduplication keeps only unique screen states — identical frames are not stored.

## Session files

```bash
farscry session --list
farscry session --latest
```

## Remove

```bash
farscry hook --remove
```

## Overhead

- CPU: <1%
- Disk: ~18 KB/min
- RAM: 22 MB (single daemon, all terminals)
