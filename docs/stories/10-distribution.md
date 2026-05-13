Story 10 - Distribution

**Status:** Ready
**Blocked by:** Story 9 (farscry binary)
**Estimated hours:** 16h

What to build

Complete distribution pipeline: GitHub Actions CI/CD, npm wrapper, pip wrapper, Homebrew tap, curl installer. All must work before launch.

GitHub Actions - CI (`.github/workflows/ci.yml`)

Triggers: push to main, pull_request.

Jobs:
1. `audit` - `cargo audit`
2. `fmt` - `cargo fmt --check`
3. `clippy` - `cargo clippy -- -D warnings`
4. `test` - `cargo test --workspace`

Platforms for test: ubuntu-latest, macos-latest, windows-latest.

GitHub Actions - Release (`.github/workflows/release.yml`)

Trigger: `push` to tags `v*.*.*`.

4 native runners (no cross-compilation - native builds for binary correctness):
- `ubuntu-latest` -> `farscry-x86_64-unknown-linux-gnu`
- `macos-latest` (arm64) -> `farscry-aarch64-apple-darwin`
- `macos-13` (x86_64) -> `farscry-x86_64-apple-darwin`
- `windows-latest` -> `farscry-x86_64-pc-windows-msvc`

Steps per platform:
1. `cargo build --release`
2. SHA256 checksum: `sha256sum farscry > farscry-{platform}.sha256`
3. Upload to GitHub Releases: binary + sha256 file
4. (after all platforms) trigger npm publish + pip publish

npm wrapper (`npm/`)

**`package.json`:**
```json
{
  "name": "farscry",
  "version": "0.1.0",
  "description": "Image interpreter for automation workflows - local, offline, 8x fewer tokens",
  "scripts": {
    "postinstall": "node postinstall.js"
  },
  "bin": { "farscry": "bin/farscry" }
}
```

**`postinstall.js`:**
1. Detect `process.platform` + `process.arch` -> map to release binary name
2. Download from `https://github.com/teles-forge/farscry/releases/download/v{VERSION}/farscry-{platform}`
3. Download `.sha256` file from same URL
4. Compute SHA256 of downloaded binary
5. **FAIL LOUDLY if checksums do not match** - do not silently continue
6. Save to `bin/farscry` (or `bin/farscry.exe` on Windows)
7. `chmod +x bin/farscry`

**Platform mapping:**
```
darwin + arm64  -> farscry-aarch64-apple-darwin
darwin + x64    -> farscry-x86_64-apple-darwin
linux + x64     -> farscry-x86_64-unknown-linux-gnu
win32 + x64     -> farscry-x86_64-pc-windows-msvc.exe
```

pip wrapper (`pip/`)

Python package using `hatch` build backend. Same download pattern as npm:
1. `python/farscry/__init__.py` - `extract()`, `diff()`, `extract_batch()` functions
2. `hatch_build.py` - downloads correct binary for current platform during `pip install`
3. SHA256 verification before saving
4. `farscry` entry point script

**Python API:**
```python
from farscry import extract, diff, extract_batch

vasp = extract('screenshot.png')
vasp = extract(image_bytes)          # bytes input
delta = diff('before.png', 'after.png')
results = extract_batch(['a.png', 'b.png'])
vasp = extract('screenshot.png', lang='eng+por', affordances=True)
```

Homebrew tap (`teles-forge/homebrew-farscry`)

Formula at `Formula/farscry.rb`:
```ruby
class Farscry < Formula
  desc "Image interpreter for automation workflows - local, offline, 8x fewer tokens"
  homepage "https://farscry.dev"
  version "0.1.0"
  license "Apache-2.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/teles-forge/farscry/releases/download/v0.1.0/farscry-aarch64-apple-darwin"
      sha256 "{SHA256}"
    else
      url "https://github.com/teles-forge/farscry/releases/download/v0.1.0/farscry-x86_64-apple-darwin"
      sha256 "{SHA256}"
    end
  end

  def install
    bin.install "farscry-#{arch}-apple-darwin" => "farscry"
  end
end
```

curl installer (`farscry.dev/install`)

Cloudflare Worker serving:
```bash
curl -fsSL https://farscry.dev/install | sh
```

Script:
1. Detect OS + arch
2. Download binary from latest GitHub Release
3. SHA256 verify
4. Install to `/usr/local/bin/farscry` (or `~/.local/bin/farscry` if no write perm)
5. Print: "farscry installed. Run: farscry setup"

NOTICES.md

Required by ONNX Runtime (MIT) license for all distributions:
```markdown
NOTICES

ONNX Runtime
Copyright (c) Microsoft Corporation
Licensed under MIT License
https://github.com/microsoft/onnxruntime/blob/main/LICENSE
```

Acceptance criteria

- [ ] GitHub Actions CI: all 4 jobs pass on push to main
- [ ] GitHub Actions Release: triggered by `v*` tag, 4 binaries uploaded with SHA256
- [ ] `npm install farscry` works on Mac M1/M2/M3
- [ ] `npm install farscry` works on Mac Intel
- [ ] `npm install farscry` works on Linux x86_64
- [ ] `npm install farscry` works on Windows x86_64
- [ ] npm postinstall: tamper test - corrupt binary byte -> SHA256 mismatch -> install FAILS with error
- [ ] `pip install farscry` works on all 4 platforms
- [ ] pip: `from farscry import extract; extract('screen.png')` returns VASP string
- [ ] `brew install teles-forge/farscry/farscry` works on macOS
- [ ] `curl -fsSL https://farscry.dev/install | sh` installs binary
- [ ] NOTICES.md present in repo root
- [ ] `cargo audit` passes in CI
- [ ] `farscry` npm package name registered on npmjs.com (squatting prevention)
- [ ] `farscry` pypi package name registered on pypi.org

Dependencies

Story 9 (farscry binary - must be named `farscry`, not `pipe`).
