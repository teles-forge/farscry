@po Validation Report - farscry v0.1.0 Stories

**Validator:** @po
**Date:** 2026-05-09
**Stories reviewed:** 12
**Verdict summary:** 4 FAIL · 5 CONDITIONAL PASS · 0 PASS
**Story 1 clear to start:** No NO - 4 critical blocking issues

---

Summary Table

| # | Story | Verdict | Critical | High | Blocker |
|---|---|---|---|---|---|
| 1 | farscry-core | **CONDITIONAL PASS** | 4 | 1 | VaspOutput fields, VaspDelta fields, BatchResult, StateId size, LanguageNotInstalled error |
| 2 | farscry-ocr-coreml | **FAIL** | 1 | 2 | oar-ocr cannot achieve 21ms - must pivot to `ort` direct or `objc2-core-ml` |
| 3 | farscry-ocr-ort | **CONDITIONAL PASS** | 0 | 2 | Add production platform perf ACs; language model download details |
| 4 | farscry-ocr selector | **CONDITIONAL PASS** | 0 | 1 | PRD workspace update; Windows AC; explicit feature activation |
| 5 | farscry-classifier | **FAIL** | 1 | 3 | 89.4% < 92% PRD minimum; body-AC inconsistency; AffordanceAction::Select undefined |
| 6 | farscry-diff | **CONDITIONAL PASS** | 0 | 3 | scroll_dy sign wrong; diff accuracy ACs missing; diff_from/diff_to not in VaspDelta |
| 7 | farscry-formatter | **FAIL** | 1 | 4 | Position labels `[mid-left]` vs `[middle-left]` contradiction with PRD §6 |
| 8 | farscry-mcp | **CONDITIONAL PASS** | 0 | 3 | Pin MCP spec version; fix auto-diff circular logic; ACs 10/11 not CI-automatable |
| 9 | farscry binary | **FAIL** | 2 | 7 | Missing -o flag, stdin AC, exit 3 AC, animated GIF, blank/corrupted, --context/--lang/--max-size ACs |
| 10 | Distribution | **FAIL** | 1 | 4 | Missing cargo publish; only 3 CI platforms (need 4); NOTICES.md content AC |
| 11 | Site update | **CONDITIONAL PASS** | 0 | 2 | Missing cargo install method; Story 10 implicit dependency |
| 12 | Launch | **CONDITIONAL PASS** | 0 | 1 | HN review criteria must be testable; add Story 10 as explicit dep |

---

Required Fixes Before Story 1 -> @dev

1. **[1-A] StateId type**: 64-bit pHash needs `[u8; 8]` not `[u8; 8; 32]`. Or document why 32 bytes and update Display.
2. **[1-B] VaspOutput struct**: Define all fields from PRD §6 with Rust types.
3. **[1-C] VaspDelta struct**: Define all fields including `diff_from: StateId`, `diff_to: StateId`, entry enum.
4. **[1-D] BatchResult**: Define the type.
5. **[1-E] FarscryError::LanguageNotInstalled**: Add variant for exit code 3.
6. **[1-G] pHash algorithm**: Clarify - full 32x32 2D-DCT (standard pHash) vs JPEG 8x8 block DCT (non-standard).

---

Cross-Cutting Blockers

| ID | Issue | Stories affected |
|---|---|---|
| X-1 | VaspOutput undefined | 1, 7, 8, 9 |
| X-2 | oar-ocr can't reach 21ms | 2, PRD §5 |
| X-3 | Classifier 89.4% < 92% PRD min | 5, 12 |
| X-4 | --install-lang implementation missing | 3, 9 |
| X-5 | `diff --agent` compact format undefined | 7, 9 |
| X-6 | `-o context.vasp` flag missing | 7, 9 |
| X-7 | StateId size contradiction | 1, 6, 7 |
| X-8 | scroll_dy in VaspDelta: not in VASP spec | 1, 6, 7 |

---

Hours Estimate Corrections

| Story | Story estimate | @po realistic estimate |
|---|---|---|
| 1 farscry-core | 6h | 10-12h |
| 2 farscry-ocr-coreml | 8h | 16-24h (implementation pivot required) |
| 9 farscry binary | 12h | 20-28h |
| 10 Distribution | 16h | 24-32h |
| **Total (all 12)** | **87h** | **120-160h** |

