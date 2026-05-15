# farscry Benchmarks

## Token Reduction

| Benchmark | Dataset | Result |
|-----------|---------|--------|
| OCR + structured output | ScreenSpot-Pro (N=223) | 15.5x token reduction |
| Session deduplication | Retina 3600×2338, real session | 160x token reduction |

**ScreenSpot-Pro methodology:** farscry extract on each screenshot vs raw base64 PNG sent to frontier model.

**Session deduplication methodology:** `farscry pack` on a real terminal session. 89% of frames were perceptually identical (Hamming distance ≤ 10). Only unique states stored.

## Daemon Performance

| Platform | Metric | Value |
|----------|--------|-------|
| macOS (M4 Pro, CoreML) | Warm daemon response | 38ms |
| macOS (M4 Pro) | Daemon RSS (N terminals) | 22 MB |
| Linux (Docker, Xvfb) | Daemon VmRSS | 11 MB |

## pHash Accuracy

Perceptual hash (63-bit DCT) properties:
- Stable to 1-pixel displacement
- Sensitive to ~20% visual change
- Hamming threshold 10 separates "identical" from "changed"

## Reproduction

```bash
farscry pack screenshots/ -o session.vasf --hamming-threshold 10
farscry info session.vasf
```
