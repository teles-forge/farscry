# We added visual state tracking to OSWorld. Here's what benchmarks can't see.

---

## Title

We added visual state tracking to OSWorld. Here's what benchmarks can't see.

Alternative (with real corpus):
Computer-use agents fail silently. [X]% of failed sessions have actions that returned OK but changed nothing on screen.

---

## Body

Claude Computer Use succeeds on ~15% of OSWorld tasks.
We wanted to understand the other 85%.

So we built farscry — a Rust CLI that gives every visual state a stable address
(StateId), records sessions in a deduplicated binary format (VASF), and lets you
diff two sessions the way you diff code.

Then we ran it on [N] agent sessions.

What we found:

- [X]% of failures are silent failures — action returned OK, screen didn't
  change, agent continued with a broken world model
- [Y]% involve visual loops — same StateId 3+ consecutive times
- [Z]% of failures happen at the same visual states

The primitive that makes this measurable: StateId.
A perceptual hash. Cross-session comparable.
StateId before == StateId after = action had zero effect.

---

**1. Claude CUA at 15%. What happens in the other 85%?**

OSWorld (arXiv:2404.07972) is the canonical benchmark for computer-use agents.
GPT-4V baseline: 11.7% task success. Claude Computer Use: ~14.9%. State of the
art as of this writing.

Benchmarks measure task completion. They don't measure what happens when the
agent fails. They can't — there's no standard format for recording what the agent
saw at each step.

We built one.

---

**2. What we measured**

We introduce two new metrics computable from VASF sessions:

**AER — Action Effect Rate**

The fraction of agent actions that produce a detectable visual change.

```
AER = (steps with StateId change) / (total steps)
```

A low AER means the agent is taking actions that don't change the screen.
In OSWorld parlance: the agent thinks it succeeded. The task state disagrees.

**VLR — Visual Loop Rate**

The fraction of sessions where the same StateId appears 3 or more times.

```
VLR = (sessions with repeated StateId ≥ 3) / (total sessions)
```

A high VLR means the agent is cycling: same screen, different action, same
outcome, repeat.

Both metrics require a StateId: a stable, cross-session visual fingerprint.
VASF provides it. Existing benchmarks don't.

---

**3. Results: [X]% silent failures, [Y]% visual loops**

*[Numbers will be filled in once the corpus is measured. Until then: these are
the metrics, not the values. The methodology below is fully reproducible.]*

```bash
farscry analyze sessions/*.vasf
```

```
Analyzed: [F] failed sessions, [S] successful sessions

FAILURE PATTERN ANALYSIS
──────────────────────────────────────────────────
Top states preceding failures:

  1. StateId phash:____  →  [N] failures ([X]%)
     screen_type: Config
     agent_context: "Save button disabled"
     avg_steps_before_failure: 2.3

SILENT FAILURE DETECTION
──────────────────────────────────────────────────
  [N] sessions ([X]%) contain silent failures
  Action returned OK. StateId unchanged. Agent continued.

VISUAL LOOPS
──────────────────────────────────────────────────
  [N] sessions ([Y]%) contain visual loops
  Same StateId 3+ consecutive times.
  Avg tokens burned in loops: [Z]/session
```

*[measured when corpus is ready]*

---

**4. What existing benchmarks don't measure**

**OSWorld**: task success/failure at the end. No per-action verification. No
visual state history. You know the agent failed. You don't know where or why.

**Agent Reliability paper** (arXiv:2403.xxxxx): measures reliability via
repeated runs. No visual state metrics. Can't distinguish silent failure from
correct no-op.

**AgentSight**: Linux-only session recording. No visual state fingerprinting.
No cross-session state comparison. No deduplicated binary format.

farscry adds to all of these: a stable visual address (StateId) for every screen
the agent sees, recorded in a compact binary format (VASF), analyzable with
`farscry analyze`.

---

**5. Two new metrics**

We propose AER and VLR as standard additions to computer-use benchmarks.

They require only:
1. A way to fingerprint visual states (StateId = perceptual hash)
2. A session format that records state transitions (VASF)
3. An analysis pass over sessions (farscry analyze)

All three are open source and available today.

---

**6. The dataset and the tool**

Dataset: [N] sessions (VASF format) — browser-use + OSWorld tasks + Claude CUA
via API. Public on HuggingFace: [link — coming before arXiv submission]

Tool:

```bash
npm install -g farscry

# record a session
farscry serve --mcp --record session.vasf

# analyze across sessions
farscry analyze sessions/*.vasf --json
```

Spec: vasp-protocol.github.io/spec
GitHub: github.com/teles-forge/farscry
Paper: [arXiv link — coming]

---

**7. Call to action**

If you run computer-use agents:

1. Record sessions with `farscry serve --mcp --record`
2. Run `farscry analyze sessions/*.vasf`
3. Tell us your AER and VLR

If you're a researcher: the preprint is at [arXiv link]. The dataset is at
[HuggingFace link]. Everything is reproducible with the commands above.

```bash
npm install -g farscry
```

---

## Comment responses (pre-drafted)

**"What's the difference between a silent failure and a correct no-op?"**

A no-op is when the agent correctly determines no action is needed. A silent
failure is when the agent takes an action (click, type, submit) that returns
success, but produces no visual change. StateId before == StateId after.
farscry can't distinguish intent, but it can flag cases where an action claimed
to succeed but changed nothing on screen. Those are the failures worth auditing.

**"Why not just use OSWorld's existing evaluation?"**

OSWorld evaluates task success at the end. It doesn't give you per-step visual
state, so you can't compute AER or VLR. farscry doesn't replace OSWorld — it
adds a layer of observability on top. You can run farscry alongside any existing
benchmark.

**"How is VASF different from screen recordings?"**

A screen recording is a video. VASF is a structured binary: each frame is a
perceptual hash + compressed VASP text (typed UI elements with coordinates). The
format deduplicates consecutive identical states, so a 300-step session might
store 40 unique visual states. Each state has a stable cross-session ID (StateId)
you can query across thousands of sessions in milliseconds.

**"What's StateId exactly?"**

A perceptual hash (phash) of the screenshot. Two screenshots with the same
visual content get the same StateId. Two screenshots with different content get
different StateIds — even if the agent thinks the state is the same. It's the
primitive that makes cross-session analysis possible.

**"Why Rust?"**

Zero runtime dependencies. Single ~8MB binary. Ships via npm, pip, Homebrew, and
curl. The binary is the distribution unit. CoreML and ONNX Runtime bindings for
38ms warm latency on M4 Pro.

**"What does [X]% silent failure rate mean in practice?"**

It means [X]% of failed sessions had at least one step where: the agent took an
action, the action returned a success response, and the screen was identical
before and after (same StateId). The agent then made its next decision based on
a world model that was already broken.
