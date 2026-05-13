Story 12 - Launch

**Status:** Blocked
**Blocked by:** Stories 10 + 11 + all launch criteria from PRD
**Estimated hours:** 4h (launch day coordination)

What to build

This is not a code story - it is the launch coordination checklist and the Show HN post.

Pre-launch checklist (all must be  before submitting)

```
□ cargo install farscry works (binary named farscry, not pipe)
□ npm install farscry works on Mac M1/M2/M3
□ npm install farscry works on Mac Intel
□ npm install farscry works on Linux x86_64
□ npm install farscry works on Windows
□ pip install farscry works on all above
□ farscry screen.png produces valid VASP output
□ farscry diff before.png after.png produces correct delta
□ farscry --from-clipboard works on macOS
□ farscry --from-clipboard works on Linux
□ farscry serve --mcp connects to MCP client
□ farscry serve --mcp connects to MCP client
□ Demo GIF recorded (< 15 seconds, real error, no narration)
□ farscry.dev live with final copy + benchmark numbers
□ vasp-protocol.github.io/spec live (already done )
□ Benchmark published in repo (N=40 already done )
□ cargo audit passes
□ NOTICES.md exists with ONNX Runtime attribution
□ SHA256 checksums on every release binary
□ farscry npm package name registered (squatting prevention)
□ farscry pypi package name registered (squatting prevention)
```

Show HN post

**Title (< 80 chars):**
```
Show HN: farscry - vision APIs describe. farscry gives coordinates.
```

**Body:**
```
MCP client issue #30925 (50+ upvotes): "The Read tool returns
Binary files not supported. There is no ReadImage tool."

farscry is the fix. MCP server mode. Typed coordinates.
8x fewer tokens than raw image sending. Works offline.

farscry screenshot.png  ->  button "Save"  at (400,300)  enabled:true
                        ->  input  "Email" at (200,180)  value:""
                        ->  error  "Card declined" at (20,350)

Three things farscry does that no existing tool does together:
1. screenshot -> typed UI tree with exact coordinates (not prose)
2. farscry diff before.png after.png -> semantic delta (~100 tokens vs 3,136)
3. farscry --from-clipboard | claude "fix this" -> zero friction

Benchmark (N=40, reproducible with your API key):
Same accuracy as cloud vision. 8x fewer tokens on 1080p.
Offline. Deterministic. $0. 21ms Apple Silicon.

Closest competitor: RETIX (April 2026) - local describe CLI,
requires 1.6B-8B local VLM, no MCP, no diff, no typed schema,
does not solve #30925.

VASP spec: https://vasp-protocol.github.io/spec (Apache 2.0)
GitHub: https://github.com/teles-forge/farscry
Install: npm install -g farscry

I'm Darlysson Teles, a solo developer. Happy to answer technical questions.
```

**First author self-comment (post immediately after):**
```
Author here. Technical details:

How it works under the hood:
1. Image -> input validator (magic bytes, 10MB limit)
2. PP-OCRv5 via ONNX Runtime (oar-ocr) - text + bounding boxes
3. Screen-type router - detects terminal/config/error/conversation/ui from OCR text patterns (89.4% OOD accuracy, zero model deps)
4. pHash(grayscale(resize(image, 32x32))) -> state_id for loop detection
5. VASP formatter -> typed output with coordinates

On OmniParser: extracts icon bounding boxes for click-coordinate prediction. Needs GPU, AGPL-tainted, no CLI, no diff. Different problem.

On "just use cloud vision": cloud vision describes. farscry diffs. farscry diff tells you "Submit button changed from disabled to enabled; Email changed from empty to user@example.com." Deterministic. Same inputs -> same output every run.

On the benchmark: N=40 synthetic screenshots, both approaches 100%. What this proves: farscry doesn't lose accuracy while cutting tokens by 3.7-8.9x. It does NOT claim to beat cloud vision on accuracy - they're equivalent.

Reproduce the benchmark: git clone teles-forge/farscry && cd spike/benchmark && uv run run_benchmark.py

VASP is Apache 2.0. farscry is the reference implementation. If you're building agent tooling that processes screenshots, the spec is at vasp-protocol.github.io/spec. Build on it.
```

Launch timing

- Submit to HN on Tuesday or Wednesday, 8-9am PT (peak tech traffic)
- Stay available for 4 hours minimum to answer technical questions
- Simultaneously: post to r/rust (architecture), r/LocalLLaMA (offline/free angle), Twitter/X (demo GIF)
- Week 1: Dev.to article "How farscry eliminates visual grounding from agent loops"

Acceptance criteria

- [ ] All pre-launch checklist items are
- [ ] Show HN post drafted and reviewed
- [ ] Author self-comment drafted and ready to paste
- [ ] Launch timing scheduled (Tuesday/Wednesday 8-9am PT)
- [ ] Social posts drafted (r/rust, r/LocalLLaMA, Twitter)
- [ ] @arromber final adversarial review passed before submitting
- [ ] Team member available to monitor HN thread for 4h post-launch

Dependencies

Stories 10 (distribution) + 11 (site). ALL launch criteria from PRD.
