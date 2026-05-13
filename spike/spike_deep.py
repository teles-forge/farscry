
"""
farscry - Deep Research Classifier Spikes
Approach 1: SigLIP zero-shot (visual, Apache 2.0)
Approach 2: Sentence-embedding cosine similarity (text, MIT) <- novel
Approach 3: Screen-type-specific rules router
Approach 4: Text-feature logistic regression (supervised upper bound)

Run from spike/ directory:  uv run spike_deep.py
"""

import sys, os, re, time, statistics
sys.path.insert(0, os.path.dirname(__file__))
from spike_approaches import GENERATORS, GEN_NAMES, Element, CLASSES
from collections import Counter, defaultdict

import numpy as np
from PIL import Image, ImageDraw, ImageFont

DEVICE_NAME = None


def build_ood_with_crops(img_size: int = 96):
    """Returns list of (Element, crop_PIL) from all 20 OOD screenshots."""
    from spike_approaches import (
        _elements_terminal, _elements_config_form,
        _elements_vscode, _elements_github_issue, _elements_chat
    )

    gen_fns = [_elements_terminal, _elements_config_form,
               _elements_vscode, _elements_github_issue, _elements_chat]

    from domain_validation import (
        gen_terminal, gen_config_form, gen_vscode_panel,
        gen_github_issue, gen_chat_panel, MIN_CROP_PX
    )
    image_gens = [gen_terminal, gen_config_form, gen_vscode_panel,
                  gen_github_issue, gen_chat_panel]

    result = []
    for img_gen, el_gen, gname in zip(image_gens, gen_fns, GEN_NAMES):
        for variant in range(4):
            img, raw_elements = img_gen(variant)
            W, H = img.size
            elements = el_gen(variant)

            raw_valid = [
                (b, c) for b, c in raw_elements
                if (b[2]-b[0]) >= MIN_CROP_PX and (b[3]-b[1]) >= MIN_CROP_PX
            ]
            el_valid = [
                e for e in elements
                if e.width >= MIN_CROP_PX and e.height >= MIN_CROP_PX
            ]

            for el, (bounds, _) in zip(el_valid, raw_valid):
                x1, y1, x2, y2 = bounds
                x1 = max(0, x1); y1 = max(0, y1)
                x2 = min(W, x2); y2 = min(H, y2)
                if (x2-x1) < MIN_CROP_PX or (y2-y1) < MIN_CROP_PX:
                    continue
                crop = img.crop((x1, y1, x2, y2)).resize(
                    (img_size, img_size), Image.LANCZOS)
                result.append((el, crop))
    return result


def score(correct, total, pred_dist=None):
    ov = sum(correct.values()) / max(sum(total.values()), 1)
    return ov

def pprint(name, correct, total, elapsed, pred_dist=None):
    ov = score(correct, total)
    flag = "" if ov >= 0.75 else (" " if ov >= 0.60 else "No")
    print(f"\n{'─'*56}")
    print(f"  {name}  ->  {ov*100:.1f}%   [{elapsed:.1f}s]")
    print(f"{'─'*56}")
    for cls in ["button", "input", "label", "heading"]:
        t = total.get(cls, 0); c = correct.get(cls, 0)
        acc = c/t if t else 0
        f2 = "" if acc>=0.6 else (" " if acc>=0.3 else "No")
        print(f"  {f2} {cls:10s}: {c:3d}/{t:3d}  ({acc*100:.0f}%)")
    if pred_dist:
        print(f"  preds: {pred_dist}")
    verdict = " GO" if ov>=0.75 else ("  CONDITIONAL" if ov>=0.60 else "No NO-GO")
    print(f"  Verdict: {verdict}")
    return ov


SIGLIP_PROMPTS = {
    "button":  "This is a photo of a clickable button in a user interface",
    "input":   "This is a photo of an input text field or search box in a form",
    "label":   "This is a photo of a text label, caption, or description",
    "heading": "This is a photo of a heading, title, or section name",
    "unknown": "This is a photo of an unknown user interface element",
}

def run_siglip(pairs):
    import torch
    from transformers import AutoProcessor, AutoModel

    t0 = time.time()
    print("  Loading google/siglip-base-patch16-224 ...")

    device = torch.device("mps") if torch.backends.mps.is_available() else torch.device("cpu")
    proc  = AutoProcessor.from_pretrained("google/siglip-base-patch16-224")
    model = AutoModel.from_pretrained("google/siglip-base-patch16-224").to(device)
    model.eval()
    load_time = time.time() - t0
    print(f"  Loaded in {load_time:.1f}s on {device}")

    labels_text  = list(SIGLIP_PROMPTS.values())
    labels_class = list(SIGLIP_PROMPTS.keys())

    correct, total, pred_dist = defaultdict(int), defaultdict(int), Counter()
    crop_times = []

    BATCH = 32
    all_crops   = [c for _, c in pairs]
    all_labels  = [e.true_label for e, _ in pairs]

    for b_start in range(0, len(all_crops), BATCH):
        batch_crops  = all_crops[b_start:b_start+BATCH]
        batch_labels = all_labels[b_start:b_start+BATCH]
        tc = time.time()
        inputs = proc(
            text=labels_text,
            images=batch_crops,
            return_tensors="pt",
            padding="max_length",
        ).to(device)
        with torch.no_grad():
            out = model(**inputs)
            logits = out.logits_per_image
            preds  = logits.argmax(dim=1).cpu().tolist()
        crop_times.append((time.time() - tc) / len(batch_crops))

        for pred_idx, true in zip(preds, batch_labels):
            pred = labels_class[pred_idx]
            pred_dist[pred] += 1
            total[true] += 1
            if pred == true:
                correct[true] += 1

    elapsed = time.time() - t0
    avg_ms  = statistics.mean(crop_times) * 1000
    print(f"  Avg inference: {avg_ms:.1f}ms/crop")
    return dict(correct), dict(total), elapsed, dict(pred_dist)


PROTOTYPES = {
    "button": [
        "Save", "Cancel", "Submit", "OK", "Delete", "Confirm", "Apply",
        "Back", "Next", "Continue", "Done", "Retry", "Run", "Build",
        "Deploy", "Comment", "Close issue", "Merge", "Create", "Update",
    ],
    "input": [
        "Enter your name", "Enter your email", "Type here...",
        "Search...", "Enter value", "sk-xxxxxxxxxxxxxxxx",
        "https://api.example.com", "30", "user@example.com",
        "Enter your password", "Add a comment...",
    ],
    "label": [
        "API Key:", "Endpoint:", "First Name:", "Email Address:",
        "Timeout (s):", "Max Retries:", "Username:", "Password:",
        "Description:", "Status:", "opened 2 hours ago by user123",
        "Traceback (most recent call last):",
        "a NullPointerException is thrown.",
        "checking logs now",
    ],
    "heading": [
        "Configuration", "User Registration", "Payment Portal",
        "Dashboard", "Settings", "Bug Report", "## Describe the bug",
        "utils.py", "alice", "bob",
        "Bug: NullPointerException in process()",
    ],
}

def run_text_embedding(pairs):
    from sentence_transformers import SentenceTransformer
    import torch

    t0 = time.time()
    print("  Loading all-MiniLM-L6-v2 ...")

    model = SentenceTransformer("sentence-transformers/all-MiniLM-L6-v2")
    load_time = time.time() - t0
    print(f"  Loaded in {load_time:.1f}s")

    print("  Computing prototype embeddings...")
    class_embs = {}
    for cls, texts in PROTOTYPES.items():
        embs = model.encode(texts, convert_to_tensor=True, normalize_embeddings=True)
        class_embs[cls] = embs.mean(dim=0)

    t_inf = time.time()
    texts_all = [el.text.strip() for el, _ in pairs]
    labels_all = [el.true_label for el, _ in pairs]

    query_embs = model.encode(texts_all, convert_to_tensor=True,
                               normalize_embeddings=True, batch_size=64)

    correct, total, pred_dist = defaultdict(int), defaultdict(int), Counter()

    for i, (emb, true) in enumerate(zip(query_embs, labels_all)):
        best_cls, best_sim = "unknown", -1.0
        for cls, cent in class_embs.items():
            sim = float(torch.dot(emb, cent))
            if sim > best_sim:
                best_sim, best_cls = sim, cls
        pred_dist[best_cls] += 1
        total[true] += 1
        if best_cls == true:
            correct[true] += 1

    elapsed = time.time() - t0
    per_crop_ms = (time.time() - t_inf) / max(len(pairs), 1) * 1000
    print(f"  Avg inference: {per_crop_ms:.2f}ms/element (text encode only)")
    return dict(correct), dict(total), elapsed, dict(pred_dist)


BUTTON_KW = {
    'ok','cancel','save','submit','send','close','delete','confirm','apply',
    'back','next','continue','done','retry','run','build','deploy','push',
    'pull','create','edit','copy','paste','undo','redo','refresh','reload',
    'open','new','add','remove','reset','clear','update','upgrade','install',
    'sign in','log in','logout','sign out','comment','reply','like','follow',
    'share','fork','merge','close issue','resolve','reopen','start','stop',
    'restart','pause','resume','abort','export','import','download','upload',
    'view','preview','search','filter','sort','yes','no','agree','disagree',
    'accept','decline','connect','disconnect','enable','disable','register',
}

def detect_screen_type(elements) -> str:
    """Heuristically detect screen type from element text patterns."""
    texts = [e.text.lower() for e in elements]
    all_text = " ".join(texts)

    if any(t.startswith("$") or t.startswith("#") or
           "traceback" in t or "valueerror" in t or "typeerror" in t
           for t in texts):
        return "terminal"

    colon_labels = sum(1 for t in texts if t.strip().endswith(':'))
    if colon_labels >= 2:
        return "config"

    if any("error" in t or "exception" in t for t in texts):
        return "error"

    short_texts = sum(1 for t in texts if len(t.split()) <= 3)
    if short_texts >= len(texts) * 0.4:
        return "conversation"

    return "ui"

def classify_by_screen_type(el: Element, screen_type: str) -> str:
    text = el.text.strip()
    ar   = el.aspect_ratio

    if screen_type == "terminal":
        return "label"

    if screen_type == "config":
        if text.endswith(':') and text.count(':') == 1: return "label"
        tl = text.lower()
        if tl in BUTTON_KW: return "button"
        if ar > 5.0 and len(text) < 30: return "input"
        return "unknown"

    if screen_type == "error":
        return "label"

    if screen_type == "conversation":
        words = len(text.split())
        if words <= 2: return "heading"
        return "label"

    if text.endswith(':') and text.count(':') == 1: return "label"
    tl = text.lower()
    if tl in BUTTON_KW: return "button"
    if ar > 5.0 and len(text) < 30: return "input"
    return "unknown"

def run_screen_type_router(pairs):
    t0 = time.time()
    correct, total, pred_dist = defaultdict(int), defaultdict(int), Counter()


    from spike_approaches import GENERATORS, GEN_NAMES
    from spike_approaches import (
        _elements_terminal, _elements_config_form,
        _elements_vscode, _elements_github_issue, _elements_chat
    )
    gen_fns = [_elements_terminal, _elements_config_form,
               _elements_vscode, _elements_github_issue, _elements_chat]

    screen_results = {}
    for gen_fn, gname in zip(gen_fns, GEN_NAMES):
        for variant in range(4):
            elements = gen_fn(variant)
            stype = detect_screen_type(elements)
            screen_results[f"{gname}_{variant}"] = stype
            for el in elements:
                pred = classify_by_screen_type(el, stype)
                pred_dist[pred] += 1
                total[el.true_label] += 1
                if pred == el.true_label:
                    correct[el.true_label] += 1

    print("  Screen type detection:")
    for gen_fn, gname in zip(gen_fns, GEN_NAMES):
        types = [screen_results.get(f"{gname}_{v}", "?") for v in range(4)]
        print(f"    {gname:15s}: {types}")

    elapsed = time.time() - t0
    return dict(correct), dict(total), elapsed, dict(pred_dist)


CODE_PAT = re.compile(
    r'(^\s*[$#%>]\s)|(Traceback)|(File ")|(raise )|(def |import )|'
    r'(\.py")|(TypeError|ValueError|Exception)',
)
PHONE_LIKE = re.compile(r'\d{4}[\s-]\d{4}')
EMAIL_LIKE = re.compile(r'@\w+\.\w+')
URL_LIKE   = re.compile(r'https?://')

def text_features(el: Element) -> list:
    t  = el.text.strip()
    tl = t.lower()
    words = t.split()
    return [
        len(t),
        len(words),
        el.aspect_ratio,
        el.height,
        float(t.endswith(':')),
        float(t.isupper()),
        float(t[0].isupper() if t else 0),
        float(bool(CODE_PAT.search(t))),
        float(bool(URL_LIKE.search(t))),
        float(bool(EMAIL_LIKE.search(t))),
        float(tl in BUTTON_KW),
        float(any(c.isdigit() for c in t)),
        float(':' in t and not t.endswith(':')),
        float(el.aspect_ratio > 5.0),
        float(len(words) == 1),
        float(len(words) <= 3),
    ]

def run_text_features(pairs):
    from sklearn.linear_model import LogisticRegression
    from sklearn.model_selection import cross_val_score
    import numpy as np

    t0 = time.time()

    X = np.array([text_features(el) for el, _ in pairs])
    y = np.array([CLASSES.index(el.true_label) if el.true_label in CLASSES
                  else CLASSES.index("unknown") for el, _ in pairs])

    mask = y < len(CLASSES) - 1
    X, y = X[mask], y[mask]

    clf = LogisticRegression(max_iter=500, C=1.0, random_state=42)
    cv_scores = cross_val_score(clf, X, y, cv=5, scoring='accuracy')
    cv_acc = cv_scores.mean()

    clf.fit(X, y)
    y_pred = clf.predict(X)

    correct, total, pred_dist = defaultdict(int), defaultdict(int), Counter()
    for yi, yp in zip(y, y_pred):
        true_cls = CLASSES[yi]
        pred_cls = CLASSES[yp]
        total[true_cls] += 1
        pred_dist[pred_cls] += 1
        if yi == yp:
            correct[true_cls] += 1

    elapsed = time.time() - t0
    print(f"  5-fold CV accuracy: {cv_acc*100:.1f}%  (upper bound = train+test same data)")
    print(f"  Feature importances: {dict(zip(['len','words','ar','h','colon_end','isupper','cap0','code','url','email','btn_kw','digit','colon_mid','wide','1word','short'], clf.coef_.mean(axis=0).tolist()))}")
    return dict(correct), dict(total), elapsed, dict(pred_dist), cv_acc


def run_hybrid(pairs, proto_model=None, proto_embs=None):
    """
    Rules first (high-precision cases), embedding fallback for ambiguous.
    """
    if proto_model is None:
        return None

    import torch
    t0 = time.time()

    texts = [el.text.strip() for el, _ in pairs]
    query_embs = proto_model.encode(texts, convert_to_tensor=True,
                                     normalize_embeddings=True, batch_size=64)

    correct, total, pred_dist = defaultdict(int), defaultdict(int), Counter()

    for i, (el, _) in enumerate(pairs):
        text = el.text.strip()
        tl   = text.lower()

        if text.endswith(':') and text.count(':') == 1:
            pred = 'label'
        elif tl in BUTTON_KW:
            pred = 'button'
        elif el.aspect_ratio > 5.0 and len(text) < 30 and not CODE_PAT.search(text):
            pred = 'input'
        else:
            emb = query_embs[i]
            best_cls, best_sim = "unknown", -1.0
            for cls, cent in proto_embs.items():
                sim = float(torch.dot(emb, cent))
                if sim > best_sim:
                    best_sim, best_cls = sim, cls
            pred = best_cls

        pred_dist[pred] += 1
        total[el.true_label] += 1
        if pred == el.true_label:
            correct[el.true_label] += 1

    elapsed = time.time() - t0
    return dict(correct), dict(total), elapsed, dict(pred_dist)


def main():
    import torch
    global DEVICE_NAME
    DEVICE_NAME = "mps" if torch.backends.mps.is_available() else "cpu"

    t_start = time.time()
    print("╔══════════════════════════════════════════════════════╗")
    print("║  farscry - Deep Research Classifier Spikes          ║")
    print("╚══════════════════════════════════════════════════════╝\n")

    print("Building OOD pairs (element + crop)...")
    pairs = build_ood_with_crops(96)
    gt_dist = Counter(el.true_label for el, _ in pairs)
    print(f"  {len(pairs)} element-crop pairs: {dict(gt_dist)}\n")

    results = {}

    print("═"*56)
    print("APPROACH 2 - Text Embedding (sentence-transformers)")
    print("═"*56)
    try:
        c, t, el, pd = run_text_embedding(pairs)
        ov = pprint("Text Embedding cosine similarity", c, t, el, pd)
        results["text_embedding"] = (c, t, ov, el)

        from sentence_transformers import SentenceTransformer
        import torch
        _sent_model = SentenceTransformer("sentence-transformers/all-MiniLM-L6-v2")
        _class_embs = {}
        for cls, texts in PROTOTYPES.items():
            embs = _sent_model.encode(texts, convert_to_tensor=True, normalize_embeddings=True)
            _class_embs[cls] = embs.mean(dim=0)
    except Exception as e:
        print(f"  No Text embedding failed: {e}")
        _sent_model = None
        _class_embs = None

    print("\n" + "═"*56)
    print("APPROACH 3 - Screen-Type Router")
    print("═"*56)
    c, t, el, pd = run_screen_type_router(pairs)
    ov = pprint("Screen-type router", c, t, el, pd)
    results["screen_type_router"] = (c, t, ov, el)

    print("\n" + "═"*56)
    print("APPROACH 4 - Text Feature Logistic Regression")
    print("═"*56)
    try:
        c, t, el, pd, cv_acc = run_text_features(pairs)
        ov = pprint(f"Text features LR (train=test), CV={cv_acc*100:.1f}%", c, t, el, pd)
        results["text_features_lr"] = (c, t, ov, el)
    except Exception as e:
        print(f"  No Text feature LR failed: {e}")

    if _sent_model is not None:
        print("\n" + "═"*56)
        print("APPROACH 5 - Hybrid: Rules + Text Embedding")
        print("═"*56)
        res = run_hybrid(pairs, _sent_model, _class_embs)
        if res:
            c, t, el, pd = res
            ov = pprint("Hybrid: rules-first + embedding fallback", c, t, el, pd)
            results["hybrid"] = (c, t, ov, el)

    print("\n" + "═"*56)
    print("APPROACH 1 - SigLIP Zero-Shot (visual)")
    print("═"*56)
    try:
        c, t, el, pd = run_siglip(pairs)
        ov = pprint("SigLIP zero-shot", c, t, el, pd)
        results["siglip"] = (c, t, ov, el)
    except Exception as e:
        print(f"  No SigLIP failed: {e}")

    print("\n" + "═"*56)
    print("SUMMARY: ALL APPROACHES RANKED")
    print("═"*56)

    ranked = sorted(results.items(), key=lambda x: -x[1][2])
    prev_best = 0.255

    for rank, (name, (c, t, ov, el)) in enumerate(ranked, 1):
        flag  = "" if ov>=0.75 else (" " if ov>=0.60 else "No")
        delta = f"+{(ov-prev_best)*100:.1f}pp vs rules" if rank == 1 else ""
        print(f"  #{rank} {flag} {name:35s}: {ov*100:.1f}%  {delta}")

    best_name, (bc, bt, best_ov, _) = ranked[0]
    print(f"\n  Winner: {best_name}  ({best_ov*100:.1f}%)")
    if best_ov >= 0.75:
        print("   GO for v0.1.0")
    elif best_ov >= 0.60:
        print("    CONDITIONAL - discuss tradeoffs")
    else:
        print("  No NO-GO - no approach reaches threshold")
        print(f"  Best gap to threshold: {(0.75-best_ov)*100:.1f}pp needed")

    print(f"\n  Winner per-class ({best_name}):")
    for cls in ["button", "input", "label", "heading"]:
        n = bt.get(cls, 0); k = bc.get(cls, 0)
        acc = k/n if n else 0
        f2 = "" if acc>=0.6 else (" " if acc>=0.3 else "No")
        print(f"    {f2} {cls:10s}: {k}/{n} ({acc*100:.0f}%)")

    total_time = time.time() - t_start
    print(f"\nTotal time: {total_time:.0f}s")


if __name__ == "__main__":
    main()
