# farscry v0.3.0 — Roadmap

**Status:** Planning
**Target:** Q3 2026
**Author:** Darlysson Teles

---

## Strategic context

v0.2.0 proved the tooling stack: multi-language OCR, annotate, session recording,
VASF format, VASP adapters. v0.3.0 is about the paper.

The goal: submit a preprint to arXiv before the HN post. The preprint requires a
real corpus, real numbers for AER and VLR, and a public dataset.

Everything in v0.3.0 serves that goal.

---

## Priority 1 — Corpus generation (300–1000 real sessions)

**Why first:** The paper has no numbers without the corpus.

**Sources:**

- browser-use + OSWorld tasks
  Record with: `farscry serve --mcp --record sessions/$(date +%s).vasf`

- Claude CUA via Anthropic API
  Same recording method. Target: 100+ sessions per task category.

- farscry setup --hook during own agent use
  Passive recording during real work sessions.

**Target:**
- Minimum: 300 sessions (100 failed, 200 successful) before preprint
- Preferred: 1000 sessions for statistical significance

**Deliverable:** `/corpus/sessions/*.vasf` committed to a private repo,
mirrored to HuggingFace before paper submission.

---

## Priority 2 — Measure AER and VLR with real numbers

**Why:** The paper's central contribution is these two metrics. They need real
values from a real corpus.

**How:**

```bash
farscry analyze corpus/sessions/*.vasf > results/analysis.json
farscry analyze corpus/sessions/*.vasf --json | jq '.silent_failure_pct'
```

**Definitions to lock before measurement:**

AER (Action Effect Rate):
```
AER = unique_states / total_input
```
Interpretation: fraction of agent calls that produced a new visual state.
A low AER means most agent actions had no visual effect.

VLR (Visual Loop Rate):
```
VLR = sessions_with_visual_loop / total_sessions
```
Where visual_loop = any StateId appearing 3+ times in a session's frame sequence.

**Deliverable:** `results/analysis.json` with real [X], [Y], [Z] numbers.
Replace all placeholder values in docs/hn-post.md and README.md.

---

## Priority 3 — Dataset public on HuggingFace

**Why:** Reviewers and the HN audience need to reproduce the numbers.

**What to publish:**

```
farscry-sessions-v1/
  sessions/          # all .vasf files
  metadata.jsonl     # per-session: task_id, agent, success, terminal_state_id
  README.md          # exact commands to reproduce analysis
  scripts/
    download.sh
    analyze.sh       # runs farscry analyze and outputs results
```

**README.md in dataset must include:**

```bash
# Install
npm install -g farscry

# Download dataset
huggingface-cli download teles-forge/farscry-sessions-v1 --local-dir sessions/

# Reproduce paper numbers
farscry analyze sessions/*.vasf --json
```

**Deliverable:** Public HuggingFace dataset before arXiv submission.

---

## Priority 4 — arXiv preprint submitted

**Target categories:** cs.AI + cs.HC

**Paper structure:**

1. Abstract: AER and VLR as new metrics for computer-use benchmarks
2. Introduction: OSWorld 15% success rate. What happens in the other 85%?
3. Related work: OSWorld, AgentSight, Agent Reliability. What they miss.
4. Method: StateId, VASF format, AER and VLR definitions
5. Experiments: corpus generation, analysis results
6. Results: [X]% AER, [Y]% VLR, top failure states
7. Discussion: implications for benchmark design
8. Conclusion + dataset release

**Deliverable:** Preprint submitted to arXiv before HN post goes live.

---

## Priority 5 — HN post + Anthropic outreach simultaneous

**Timing:** arXiv preprint accepted → HN post → Anthropic DM same day.

**HN post:** Use docs/hn-post.md (alternative title with real numbers).
Replace all [X], [Y], [Z] with real values from Priority 2.

**Anthropic outreach:**
- Subject: "We measured silent failures in Claude CUA sessions"
- Body: paper link + dataset link + offer to share raw data
- Contact: safety@anthropic.com + direct DMs to CUA team researchers

**Deliverable:** Coordinated launch. Paper live before post. Data downloadable
before post. Reproducibility commands verified by at least one person other
than the author.

---

## Non-goals for v0.3.0

- Accuracy improvements to OCR pipeline (that's v0.2.x)
- New output formats
- Windows clipboard improvements
- Performance work

All of those wait until after the paper is submitted.
