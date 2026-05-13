Story 11 - Site update (farscry.dev final copy + GIF)

**Status:** Ready
**Blocked by:** Story 9 (farscry binary working and installable)
**Estimated hours:** 6h

What to build

Update farscry.dev (Astro Starlight, already scaffolded) with final launch copy and the demo GIF.

Homepage copy

**Headline:** "vision APIs describe. farscry gives coordinates."
**Subheadline:** "Agents that act, not guess."

**Hero code block:**
```
farscry screenshot.png  ->  button "Save"  at (400,300)  enabled:true
                        ->  input  "Email" at (200,180)  value:""
                        ->  error  "Card declined" at (20,350)
```

**Benchmark strip (3 numbers):**
- `8.9x` fewer tokens than cloud vision on 1080p screenshots
- `21ms` Apple Silicon - local, offline, no API key
- `100%` accuracy parity with cloud vision (N=40 benchmark)

**Three modes section:**
- Mode 1: Describe - any image -> typed VASP context
- Mode 2: Diff - semantic delta between two states
- Mode 3: Clipboard - Cmd+Shift+4 -> farscry -> agent

**Install section (4 methods):**
```bash
npm
npm install -g farscry

pip
pip install farscry

Homebrew
brew install teles-forge/farscry/farscry

curl
curl -fsSL https://farscry.dev/install | sh
```

**VASP protocol section:**
Brief explanation: like MCP for tools, VASP for visual state. Link to vasp-protocol.github.io/spec.

**Demo GIF section:**
Embed the demo GIF (recorded in this story).

Demo GIF - required

Record this exact workflow (screencapture or similar):
1. Real error visible on screen (terminal or VS Code - real error, not synthetic)
2. Cmd+Shift+4 -> screenshot saved to clipboard
3. Terminal: `farscry --from-clipboard | claude "fix this"` (Enter)
4. VASP output appears line by line on stdout
5. MCP client gives the specific fix
6. Total screen recording: < 15 seconds
7. No voiceover. No text overlay. Terminal speaks.

Export as GIF or MP4. Optimise: < 3MB.

Docs pages

Update:
- Getting started: `npm install farscry && farscry setup`
- Three modes: extract, diff, clipboard with examples
- VASP format: full output format documentation
- MCP integration: MCP client + MCP client config snippets
- Benchmark: link to `spike/benchmark/` in repo

Acceptance criteria

- [ ] farscry.dev live with updated homepage copy
- [ ] Benchmark numbers visible on homepage: 8.9x, 21ms, 100%
- [ ] Three modes documented with working code examples
- [ ] Install section: all 4 methods shown
- [ ] Demo GIF recorded (< 15 seconds, real error, no narration)
- [ ] Demo GIF embedded on homepage
- [ ] GIF file size < 3MB
- [ ] VASP protocol section with link to vasp-protocol.github.io
- [ ] Getting started page: user can go from install to first VASP output in < 5 minutes

Dependencies

Story 9 (farscry binary) - demo GIF requires working binary.
