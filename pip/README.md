farscry

> Image interpreter for automation workflows - local, offline, 8x fewer tokens

```bash
pip install farscry
farscry screenshot.png
```

```python
from farscry import extract, diff, extract_batch

Extract VASP context from an image
vasp = extract("screenshot.png")
print(vasp)

Diff two screenshots
delta = diff("before.png", "after.png")

Batch processing (parallel)
results = extract_batch(["a.png", "b.png", "c.png"])
```

What it does

`farscry` converts screenshots into typed, coordinate-rich text that automation workflows can act on - without vision APIs, without API keys, and with 8x fewer tokens than raw image sending.

It speaks [VASP (Visual Application State Protocol)](https://vasp-protocol.github.io/spec).

Install

```bash
pip
pip install farscry

npm
npm install -g farscry

Homebrew
brew install teles-forge/farscry/farscry

curl
curl -fsSL https://farscry.dev/install | sh
```

Python API

```python
from farscry import extract, diff, extract_batch, FarscryError

From file path
vasp = extract("screenshot.png")

From bytes (PNG/JPG/WebP)
with open("screenshot.png", "rb") as f:
    vasp = extract(f.read())

JSON output
vasp_json = extract("screenshot.png", json=True)

Diff
delta = diff("before.png", "after.png")

Batch (parallel via rayon)
results = extract_batch(["a.png", "b.png"])

Error handling
try:
    vasp = extract("nonexistent.png")
except FarscryError as e:
    print(f"Failed (exit {e.exit_code}): {e}")
```

Platforms

| Platform | Architecture | Notes |
|----------|-------------|-------|
| macOS | Apple Silicon (arm64) | Native CoreML, 21ms |
| macOS | Intel (x64) | |
| Linux | x86_64 | |
| Windows | x86_64 | |

License

Apache-2.0. See [NOTICES](https://github.com/teles-forge/farscry/blob/main/NOTICES.md) for ONNX Runtime attribution.
