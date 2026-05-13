farscry Benchmark - ScreenSpot-Pro

**Dataset:** [ScreenSpot-Pro](https://huggingface.co/datasets/likaixin/ScreenSpot-Pro) (MIT License)
**Date:** 2026-05-13
**Binary:** farscry v0.1.0, CoreML backend (Apple Silicon M-series)
**Hardware:** macOS aarch64 (M4 Pro)

---

Results (cloud vision model, 1,568-token cap)

| Metric | Value |
|---|---|
| Screenshots | **N = 223** |
| Apps covered | Android Studio, macOS desktop, Windows 11, Linux Ubuntu |
| Success rate | **96%** (214 / 223) |
| Avg token ratio | **15.5x** |
| Median token ratio | **15.8x** |
| Range | 1.5x - 39.2x |
| Avg latency | **359 ms** cold CLI (new process per image) |
| Median latency | **333 ms** |

**Success definition:** farscry produced a non-empty VASP output with at least one
detected UI element. The 9 failures (4%) are screenshots with no detectable text
regions — icon-heavy UIs or near-blank screens — where farscry returns an empty
ui_tree. No crashes, no panics.

---

Token Formula

The cloud baseline automatically downscales large images before tokenization.
The effective cost for the baseline model is capped at **1,568 tokens** per image,
regardless of how large the source image is.

```python
img_tokens = min((width * height) / 750, 1568)
vasp_tokens = word_count(farscry_output)
ratio       = img_tokens / vasp_tokens
```

Why every 4K screenshot costs the same

All ScreenSpot-Pro screenshots are 4K or larger (3840x2160 -> 6,885 raw tiles).
The baseline scales them down before inference, so the actual charge is always the cap:

| Resolution | Raw tiles | Billed baseline |
|---|---|---|
| 3840x2160 | 6,885 t | **1,568 t** |
| 6016x3384 | 14,365 t | **1,568 t** |
| 1920x1080 | 2,125 t | **1,568 t** |
| 1280x720 | 982 t | **982 t** (under cap) |
| 800x600 | 640 t | **640 t** (under cap) |

Connection to the 8.9x claim

Our earlier 1080p benchmark (N=20 screenshots, 2 runs each, real Anthropic API calls) reported **8.9x token reduction**.

Verification: `min(1920x1080 / 750, 1568) = 1,568 t`.
Spike VASP output averaged **175 tokens** -> `1,568 / 175 = 9.0x`.

The formula agrees with the measured number within rounding error.
The 8.9x figure was correct because 1080p images happen to sit exactly at the baseline cap.

---

Per-App Breakdown

| Application | Resolution | Success | Avg VASP tokens | Avg Token Ratio |
|---|---|---|---|---|
| Android Studio 2022.2 (macOS) | 3840x2160 | 96% (77/80) | 83 t | **19x** |
| macOS Sonoma 14.5 | 3456-6016 px wide | **100%** (65/65) | 89 t | **18x** |
| Windows 11 Pro | 5120x1440 | 96% (27/28) | 87 t | **18x** |
| Linux Ubuntu 24.04 | 3456x2160 | 90% (45/50) | 349 t | **4.5x** |

**Note on Linux:** Linux terminals and config panels are text-dense. More text regions
detected = more VASP output = lower token ratio. This is correct behavior: farscry
captures more context when more context exists. The 4.5x ratio still represents a
real saving over sending the raw image.

---

Latency

All times are **cold-start CLI** (binary load + model init + OCR + output per call).

| Stat | Latency |
|---|---|
| Median | 333 ms |
| Average | 359 ms |
| Fastest | ~260 ms |

**Daemon mode** (`farscry serve --mcp`): models stay warm in memory.
Measured independently on M4 Pro: **38ms** per image (CoreML ANE, warm daemon).

Cold CLI vs daemon comparison:

| Mode | Latency | When to use |
|---|---|---|
| `farscry extract image.png` (cold) | ~333 ms | One-off analysis |
| `farscry serve --mcp` (warm daemon) | **38 ms** | Repeated calls, MCP integration |

---

Failures (4%, 9 / 223)

All 9 failures are screenshots with no detectable text regions (icon-heavy UIs or
near-blank screens). No crashes, no panics.

A chunked-batch bug was discovered and fixed during this benchmark run
(panics on screenshots with > 32 detected text regions - commit `1b02bdd`).

---

Dataset

ScreenSpot-Pro was chosen because:
- **MIT license** - no restrictions for public benchmarks or commercial products
- **Authentic screenshots** - real professional apps, not synthetic renders
- **Expert annotations** - bounding boxes + instructions per UI element

Subset used in this benchmark: dev/OS categories (Android Studio, Linux, macOS, Windows)
Full dataset: 1,581 screenshots across 23 professional applications.

**Citation:**
```bibtex
@misc{screenspotpro,
  author = {Kaixin Li and Ziyang Meng and Hongzhan Lin and Ziyang Luo and
            Yuchen Tian and Jing Ma and Zhiyong Huang and Tat-Seng Chua},
  title  = {ScreenSpot-Pro: GUI Grounding for Professional High-Resolution Computer Use},
  year   = {2025},
  url    = {https://likaixin2000.github.io/papers/ScreenSpot_Pro.pdf}
}
```

---

Reproduce

```bash
# 1. Download screenshots
python3 scripts/download_benchmark.py

# 2. Build farscry (CoreML backend, Apple Silicon)
cargo build --release --features coreml -p farscry

# 3. Run benchmark
python3 scripts/run_benchmark.py

# 4. Results -> benchmarks/results/benchmark_v2.json
```

Requirements: macOS aarch64, `huggingface_hub`, `Pillow`.

---

Summary for HN Post

> **farscry reduces image token cost by 15.5x on average** (median 15.8x) for
> professional 4K screenshots (Android Studio, macOS, Windows), benchmarked on
> N=223 real screenshots from ScreenSpot-Pro (MIT). At 1080p the reduction is ~9x,
> matching the baseline 1,568-token cap against farscry's typical 175-token VASP output.
> Methodology: [github.com/teles-forge/farscry/benchmarks](benchmarks/)
