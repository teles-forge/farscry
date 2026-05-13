
"""
farscry benchmark runner - 20 screenshots x 2 runs = 40 data points.

RUN A: cloud model reads raw image via its Read tool (cloud vision systems)
RUN B: farscry extracts VASP text, cloud model reads structured output

Evaluation is keyword-based against ground_truth.json.
All full responses are saved for manual review.

Token counting:
  RUN A: image tokens estimated from image dimensions
         (cloud model image encoding: 170 tokens per 512x512 tile, rounded up)
  RUN B: farscry output characters / 4 (standard token approximation)
  Both: response text tokens estimated similarly
"""

import json
import subprocess
import time
import csv
import re
from pathlib import Path

BENCH  = Path(__file__).parent
SS     = BENCH / "screenshots"
OUT    = BENCH / "results"
OUT.mkdir(exist_ok=True)

PIPE_BIN = "pipe"

EVAL_PROMPT = (
    "Answer these two questions briefly:\n"
    "1. What is the specific error/element identified?\n"
    "2. What exact action or fix is needed?\n\n"
    "Be specific with file names, line numbers, field names, or coordinates when visible."
)


def image_tokens(path: Path) -> int:
    """
    cloud model image token cost:
    - Image is tiled into 512x512 chunks (each ~170 tokens)
    - Plus 85 base tokens per image
    - Approximate: (ceil(w/512) * ceil(h/512)) * 170 + 85
    """
    try:
        from PIL import Image
        img = Image.open(path)
        w, h = img.size
        tiles_w = max(1, -(-w // 512))
        tiles_h = max(1, -(-h // 512))
        return tiles_w * tiles_h * 170 + 85
    except Exception:
        return 800

def text_tokens(s: str) -> int:
    """Approximate: 1 token ≈ 4 chars (English text)."""
    return max(1, len(s) // 4)


def run_cmd(cmd: list[str], timeout: int = 90) -> tuple[str, float, int]:
    """Returns (stdout, elapsed_seconds, returncode)."""
    t0 = time.perf_counter()
    try:
        r = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
        elapsed = time.perf_counter() - t0
        return r.stdout.strip(), elapsed, r.returncode
    except subprocess.TimeoutExpired:
        return "TIMEOUT", timeout, 1
    except Exception as e:
        return f"ERROR: {e}", 0.0, 1


def run_a(img_path: Path, question: str) -> dict:
    """RUN A: cloud model reads the image directly via its built-in Read tool."""
    prompt = (
        f"Read the image at {img_path}.\n\n"
        f"{question}\n\n"
        f"{EVAL_PROMPT}"
    )
    stdout, elapsed, rc = run_cmd([
        "claude", "--print",
        "--dangerously-skip-permissions",
        prompt,
    ])
    return {
        "response":      stdout,
        "elapsed_s":     round(elapsed, 2),
        "rc":            rc,
        "input_tokens":  image_tokens(img_path) + text_tokens(prompt),
        "output_tokens": text_tokens(stdout),
    }


def run_b(img_path: Path, question: str) -> dict:
    """RUN B: farscry extracts VASP text, cloud model reads structured output via stdin."""
    t0 = time.perf_counter()
    try:
        farscry_result = subprocess.run(
            [PIPE_BIN, str(img_path)],
            capture_output=True, text=True, timeout=30,
        )
        vasp_text = farscry_result.stdout.strip()
    except Exception as e:
        vasp_text = f"FARSCRY_ERROR: {e}"
    farscry_ms = (time.perf_counter() - t0) * 1000

    if not vasp_text or vasp_text.startswith("FARSCRY_ERROR"):
        return {
            "response": f"farscry failed: {vasp_text}",
            "elapsed_s": 0.0, "rc": 1,
            "input_tokens": 0, "output_tokens": 0,
            "farscry_output": vasp_text, "farscry_ms": farscry_ms,
        }

    claude_prompt = f"{question}\n\n{EVAL_PROMPT}"
    t0 = time.perf_counter()
    try:
        r = subprocess.run(
            ["claude", "--print", "--dangerously-skip-permissions", claude_prompt],
            input=vasp_text,
            capture_output=True, text=True, timeout=60,
        )
        claude_out = r.stdout.strip()
        claude_rc  = r.returncode
    except Exception as e:
        claude_out = f"CLAUDE_ERROR: {e}"
        claude_rc  = 1
    claude_elapsed = time.perf_counter() - t0

    return {
        "response":       claude_out,
        "elapsed_s":      round(farscry_ms/1000 + claude_elapsed, 2),
        "rc":             claude_rc,
        "input_tokens":   text_tokens(vasp_text) + text_tokens(claude_prompt),
        "output_tokens":  text_tokens(claude_out),
        "farscry_output": vasp_text,
        "farscry_ms":     round(farscry_ms, 1),
    }


def score_response(response: str, gt: dict) -> bool:
    """
    Returns True if response contains key terms from ground truth.
    Looks for file names, line numbers, element names, or key phrases.
    Case-insensitive. Requires at least 2 key tokens to match.
    """
    response_lower = response.lower()

    combined = f"{gt['correct_element']} {gt['correct_action']}"
    tokens = re.findall(r'[a-zA-Z0-9:._/-]+', combined)
    key_tokens = [
        t.lower() for t in tokens
        if len(t) > 2 or (t.isdigit() and int(t) < 1000)
    ]

    if not key_tokens:
        return False

    matches = sum(1 for t in key_tokens if t in response_lower)
    threshold = max(2, int(len(key_tokens) * 0.4))
    return matches >= threshold


def run_benchmark():
    gt_path = BENCH / "ground_truth.json"
    gt = json.loads(gt_path.read_text())

    screenshots = sorted(SS.glob("*.png"))
    print(f"Found {len(screenshots)} screenshots. Running {len(screenshots)*2} evaluations.\n")

    rows = []
    full_log = []

    for img in screenshots:
        name = img.name
        if name not in gt:
            print(f"  SKIP {name} - no ground truth")
            continue

        truth = gt[name]
        question = truth["question"]
        print(f"  {name}  [{truth['category']}]  advantage={truth['advantage']}")

        print(f"    RUN A (vision)  ...", end=" ", flush=True)
        a = run_a(img, question)
        a_correct = score_response(a["response"], truth)
        print(f"{'OK' if a_correct else 'FAIL'}  {a['elapsed_s']:.1f}s  ~{a['input_tokens']}->{a['output_tokens']} tok")

        print(f"    RUN B (farscry) ...", end=" ", flush=True)
        b = run_b(img, question)
        b_correct = score_response(b["response"], truth)
        print(f"{'OK' if b_correct else 'FAIL'}  {b['elapsed_s']:.1f}s  ~{b['input_tokens']}->{b['output_tokens']} tok  (farscry {b.get('farscry_ms',0):.0f}ms)")

        time.sleep(4)

        token_reduction = round(a["input_tokens"] / max(b["input_tokens"], 1), 1)

        rows.append({
            "screenshot":         name,
            "category":           truth["category"],
            "advantage":          truth["advantage"],
            "run_a_correct":      "yes" if a_correct else "no",
            "run_b_correct":      "yes" if b_correct else "no",
            "run_a_input_tokens": a["input_tokens"],
            "run_b_input_tokens": b["input_tokens"],
            "token_reduction_x":  token_reduction,
            "run_a_elapsed_s":    a["elapsed_s"],
            "run_b_elapsed_s":    b["elapsed_s"],
            "farscry_ms":         b.get("farscry_ms", 0),
            "winner":             ("farscry" if b_correct and not a_correct
                                   else "vision" if a_correct and not b_correct
                                   else "tie" if a_correct and b_correct
                                   else "both_fail"),
        })

        full_log.append({
            "screenshot": name,
            "ground_truth": truth,
            "run_a": a,
            "run_b": b,
            "run_a_correct": a_correct,
            "run_b_correct": b_correct,
        })

        print()

    csv_path = OUT / "results.csv"
    with open(csv_path, "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=rows[0].keys())
        w.writeheader()
        w.writerows(rows)

    log_path = OUT / "full_log.json"
    log_path.write_text(json.dumps(full_log, indent=2))

    n = len(rows)
    a_acc   = sum(1 for r in rows if r["run_a_correct"] == "yes") / n
    b_acc   = sum(1 for r in rows if r["run_b_correct"] == "yes") / n
    farscry_wins = sum(1 for r in rows if r["winner"] == "farscry")
    vision_wins  = sum(1 for r in rows if r["winner"] == "vision")
    ties         = sum(1 for r in rows if r["winner"] == "tie")
    both_fail    = sum(1 for r in rows if r["winner"] == "both_fail")

    avg_token_reduction = sum(r["token_reduction_x"] for r in rows) / n
    avg_farscry_ms = sum(r["farscry_ms"] for r in rows) / n

    cats = {}
    for r in rows:
        c = r["category"]
        if c not in cats:
            cats[c] = {"n": 0, "a_correct": 0, "b_correct": 0}
        cats[c]["n"] += 1
        if r["run_a_correct"] == "yes": cats[c]["a_correct"] += 1
        if r["run_b_correct"] == "yes": cats[c]["b_correct"] += 1

    summary = {
        "n_screenshots": n,
        "run_a_accuracy": round(a_acc, 3),
        "run_b_accuracy": round(b_acc, 3),
        "accuracy_delta_pp": round((b_acc - a_acc) * 100, 1),
        "farscry_wins": farscry_wins,
        "vision_wins":  vision_wins,
        "ties":         ties,
        "both_fail":    both_fail,
        "avg_token_reduction_x": round(avg_token_reduction, 1),
        "avg_farscry_ocr_ms": round(avg_farscry_ms, 1),
        "by_category": {
            c: {
                "n": v["n"],
                "run_a_acc": round(v["a_correct"]/v["n"], 2),
                "run_b_acc": round(v["b_correct"]/v["n"], 2),
            }
            for c, v in cats.items()
        },
        "note_on_methodology": (
            "Evaluation is keyword-based against pre-registered ground truth. "
            "A response is marked correct if >=40% of key tokens from the ground truth "
            "correct_element + correct_action appear in the response (minimum 2 tokens). "
            "Full responses are in full_log.json for manual review. "
            "Token counts for RUN A include image encoding estimate "
            "(170 tokens/512px tile + 85 base). "
            "This is NOT an OSWorld-style task completion benchmark - it measures "
            "element identification accuracy from screenshots, not end-to-end agent task success."
        ),
    }

    summary_path = OUT / "summary.json"
    summary_path.write_text(json.dumps(summary, indent=2))

    print("=" * 60)
    print(f"RESULTS - {n} screenshots")
    print(f"  RUN A (cloud vision systems):  {a_acc*100:.0f}% correct ({sum(1 for r in rows if r['run_a_correct']=='yes')}/{n})")
    print(f"  RUN B (farscry VASP):   {b_acc*100:.0f}% correct ({sum(1 for r in rows if r['run_b_correct']=='yes')}/{n})")
    print(f"  Accuracy delta:         {(b_acc-a_acc)*100:+.1f}pp")
    print(f"  farscry wins:           {farscry_wins}")
    print(f"  Vision wins:            {vision_wins}")
    print(f"  Both correct (tie):     {ties}")
    print(f"  Both wrong:             {both_fail}")
    print(f"  Avg token reduction:    {avg_token_reduction:.1f}x")
    print(f"  Avg farscry OCR time:   {avg_farscry_ms:.0f}ms")
    print()
    print(f"Results saved to {OUT}/")
    print(f"  results.csv   - one row per screenshot")
    print(f"  summary.json  - aggregated stats")
    print(f"  full_log.json - all raw responses (manual review)")

    return summary


if __name__ == "__main__":
    run_benchmark()
