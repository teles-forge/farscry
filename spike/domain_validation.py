
"""
farscry - Domain Validation Experiment
Validates RICO -> dev-tool-screenshot domain gap for MobileNetV3-Small classifier.

DATA SOURCE: raw RICO semantic_annotations/ (150MB zip, extracted locally)
  Download: https://storage.googleapis.com/crowdstf-rico-uiuc-4540/rico_dataset_v0.1/semantic_annotations.zip

Run from spike/ directory:
  uv run domain_validation.py

Decisions already closed (see architecture doc Q7):
  Architecture : MobileNetV3-Small INT8 (~2.5MB)
  Classes      : button / input / label / heading / unknown
  Dataset      : RICO (shunk031/Rico on HuggingFace)
"""

import os, sys, time, random, math
from collections import Counter, defaultdict
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFont

import torch
import torch.nn as nn
import torch.optim as optim
from torch.utils.data import Dataset, DataLoader, WeightedRandomSampler
import torchvision.transforms as T
import timm
import sys, os
sys.path.insert(0, os.path.dirname(__file__))
from rico_raw_loader import RicoRawLoader
from tqdm import tqdm


SEED         = 42
IMG_SIZE     = 96
BATCH_SIZE   = 64
EPOCHS       = 5
LR           = 1e-3
TRAIN_LIMIT  = 1000
MIN_CROP_PX  = 10

CLASSES      = ["button", "input", "label", "heading", "unknown"]
N_CLASSES    = len(CLASSES)
CLASS_IDX    = {c: i for i, c in enumerate(CLASSES)}

RICO_CLASSES = [
    "Text","Image","Icon","Text Button","List Item","Input",
    "Background Image","Card","Web View","Radio Button","Drawer",
    "Checkbox","Advertisement","Modal","Pager Indicator","Slider",
    "On/Off Switch","Button Bar","Toolbar","Number Stepper",
    "Multi-Tab","Date Picker","Map View","Video","Bottom Navigation",
]
RICO_TO_FARSCRY = {
    0:  "text_raw",
    3:  "button",
    5:  "input",
    9:  "button",
    11: "button",
    16: "button",
}

random.seed(SEED)
np.random.seed(SEED)
torch.manual_seed(SEED)


def get_device() -> torch.device:
    if torch.backends.mps.is_available():
        return torch.device("mps")
    if torch.cuda.is_available():
        return torch.device("cuda")
    return torch.device("cpu")

DEVICE = get_device()
print(f"Device: {DEVICE}")


def extract_crops(example: dict) -> list[tuple[Image.Image, str]]:
    """
    Returns list of (crop_img, farscry_class_str) for one RICO record.
    Applies heading heuristic: Text elements above 1.5x median height -> heading.
    """
    img = example["screenshot"].convert("RGB")
    W, H = img.size

    raw: list[tuple[list[int], str]] = []
    text_heights: list[float] = []

    for group in example["children"]:
        for bounds, label_id in zip(group["bounds"], group["component_label"]):
            x1, y1, x2, y2 = bounds
            x1, y1 = max(0, x1), max(0, y1)
            x2, y2 = min(W, x2), min(H, y2)
            if (x2 - x1) < MIN_CROP_PX or (y2 - y1) < MIN_CROP_PX:
                continue
            farscry = RICO_TO_FARSCRY.get(label_id)
            if farscry is None:
                farscry = "unknown"
            raw.append(([x1, y1, x2, y2], farscry))
            if farscry == "text_raw":
                text_heights.append(y2 - y1)

    median_h = float(np.median(text_heights)) if text_heights else 0.0
    threshold = median_h * 1.5

    result: list[tuple[Image.Image, str]] = []
    for bounds, farscry in raw:
        x1, y1, x2, y2 = bounds
        if farscry == "text_raw":
            farscry = "heading" if (y2 - y1) >= threshold else "label"
        crop = img.crop((x1, y1, x2, y2)).resize((IMG_SIZE, IMG_SIZE), Image.LANCZOS)
        result.append((crop, farscry))

    return result


TRAIN_TF = T.Compose([
    T.RandomHorizontalFlip(),
    T.ColorJitter(brightness=0.3, contrast=0.3, saturation=0.2),
    T.ToTensor(),
    T.Normalize([0.485, 0.456, 0.406], [0.229, 0.224, 0.225]),
])
EVAL_TF = T.Compose([
    T.ToTensor(),
    T.Normalize([0.485, 0.456, 0.406], [0.229, 0.224, 0.225]),
])


class CropDataset(Dataset):
    def __init__(self, crops: list[tuple[Image.Image, str]], transform):
        self.crops = crops
        self.transform = transform

    def __len__(self): return len(self.crops)

    def __getitem__(self, idx):
        img, cls = self.crops[idx]
        return self.transform(img), CLASS_IDX[cls]


def build_weighted_sampler(crops):
    labels = [CLASS_IDX[c] for _, c in crops]
    cnt = Counter(labels)
    weights = [1.0 / cnt[l] for l in labels]
    return WeightedRandomSampler(weights, len(weights), replacement=True)


def build_model() -> nn.Module:
    model = timm.create_model(
        "mobilenetv3_small_100",
        pretrained=True,
        num_classes=N_CLASSES,
    )
    return model.to(DEVICE)


def train_epoch(model, loader, optimizer, criterion):
    model.train()
    total_loss, correct, total = 0.0, 0, 0
    for imgs, labels in loader:
        imgs, labels = imgs.to(DEVICE), labels.to(DEVICE)
        optimizer.zero_grad()
        out = model(imgs)
        loss = criterion(out, labels)
        loss.backward()
        optimizer.step()
        total_loss += loss.item() * len(labels)
        correct += (out.argmax(1) == labels).sum().item()
        total += len(labels)
    return total_loss / total, correct / total


@torch.no_grad()
def evaluate(model, loader):
    model.eval()
    correct, total = 0, 0
    per_class_correct = defaultdict(int)
    per_class_total   = defaultdict(int)
    for imgs, labels in loader:
        imgs, labels = imgs.to(DEVICE), labels.to(DEVICE)
        preds = model(imgs).argmax(1)
        correct += (preds == labels).sum().item()
        total   += len(labels)
        for p, l in zip(preds.cpu(), labels.cpu()):
            per_class_total[l.item()]   += 1
            per_class_correct[l.item()] += int(p == l)
    acc = correct / total if total else 0.0
    return acc, per_class_correct, per_class_total


def _font(size: int):
    for path in [
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/System/Library/Fonts/Helvetica.ttc",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    ]:
        if Path(path).exists():
            try:
                return ImageFont.truetype(path, size)
            except Exception:
                pass
    return ImageFont.load_default()


def _mono(size: int):
    for path in [
        "/System/Library/Fonts/Supplemental/Courier New.ttf",
        "/System/Library/Fonts/Monaco.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
    ]:
        if Path(path).exists():
            try:
                return ImageFont.truetype(path, size)
            except Exception:
                pass
    return ImageFont.load_default()


def _box(draw, x1, y1, x2, y2, fill, outline=None):
    draw.rectangle([x1, y1, x2, y2], fill=fill, outline=outline)


OOD_W = 900


def gen_terminal(idx: int) -> tuple[Image.Image, list[tuple[list[int], str]]]:
    """Terminal / stacktrace screenshot."""
    lines = [
        ("$ python app.py",          "label"),
        ("Traceback (most recent call last):", "label"),
        ('  File "app.py", line 42, in <module>', "label"),
        ('    result = process(data)',            "label"),
        ('  File "utils.py", line 17, in process',"label"),
        (f'    raise ValueError("bad input #{idx}")', "label"),
        (f'ValueError: bad input #{idx}',         "label"),
    ]
    line_h, pad = 28, 12
    H = pad + len(lines) * line_h + pad
    img = Image.new("RGB", (OOD_W, H), (30, 30, 30))
    draw = ImageDraw.Draw(img)
    f = _mono(18)
    elements = []
    for i, (text, cls) in enumerate(lines):
        y = pad + i * line_h
        color = (255, 80, 80) if "Error" in text or "Traceback" in text else (200, 200, 200)
        draw.text((pad, y), text, fill=color, font=f)
        bb = draw.textbbox((pad, y), text, font=f)
        elements.append(([bb[0], bb[1], bb[2], bb[3]], cls))
    return img, elements


def gen_config_form(idx: int) -> tuple[Image.Image, list[tuple[list[int], str]]]:
    """Config form: labels + inputs + submit button."""
    H, pad = 420, 24
    img = Image.new("RGB", (OOD_W, H), (255, 255, 255))
    draw = ImageDraw.Draw(img)
    fh = _font(26)
    ft = _font(20)
    elements = []

    title = f"Configuration #{idx}"
    draw.text((pad, 20), title, fill=(20, 20, 20), font=fh)
    bb = draw.textbbox((pad, 20), title, font=fh)
    elements.append(([bb[0], bb[1], bb[2], bb[3]], "heading"))

    fields = [("API Key:", "sk-••••••••••••"), ("Endpoint:", "https://api.example.com"),
              ("Timeout (s):", "30"), ("Max Retries:", "3")]
    y = 80
    for label_text, val in fields:
        draw.text((pad, y), label_text, fill=(80, 80, 80), font=ft)
        lbb = draw.textbbox((pad, y), label_text, font=ft)
        elements.append(([lbb[0], lbb[1], lbb[2], lbb[3]], "label"))
        ix1, ix2 = 200, OOD_W - pad
        draw.rectangle([ix1, y - 4, ix2, y + 26], outline=(180, 180, 180), width=1)
        draw.text((ix1 + 6, y), val, fill=(50, 50, 50), font=ft)
        elements.append(([[ix1, y - 4, ix2, y + 26][0], y - 4, ix2, y + 26], "input"))
        y += 48

    bx1, by1, bx2, by2 = pad, y + 10, pad + 120, y + 44
    draw.rectangle([bx1, by1, bx2, by2], fill=(37, 99, 235), outline=(29, 78, 216))
    draw.text((bx1 + 22, by1 + 6), "Save", fill=(255, 255, 255), font=ft)
    elements.append(([bx1, by1, bx2, by2], "button"))

    return img, elements


def gen_vscode_panel(idx: int) -> tuple[Image.Image, list[tuple[list[int], str]]]:
    """VS Code-style panel with error message."""
    code_lines = [
        "def process(data: dict) -> Result:",
        '    if not data.get("key"):',
        '        raise ValueError("missing key")',
        "    return transform(data)",
        "",
        "# --- ERROR OUTPUT ---",
        f'  Line {14 + idx}: TypeError: expected str, got int',
        "  Check argument types before calling transform()",
    ]
    line_h, pad = 26, 10
    H = pad + len(code_lines) * line_h + 60
    img = Image.new("RGB", (OOD_W, H), (30, 30, 30))
    draw = ImageDraw.Draw(img)
    fm = _mono(16)
    fh = _font(18)
    elements = []

    tab_text = f"utils_{idx}.py"
    draw.text((pad, pad), tab_text, fill=(200, 200, 200), font=fh)
    tbb = draw.textbbox((pad, pad), tab_text, font=fh)
    elements.append(([tbb[0], tbb[1], tbb[2], tbb[3]], "heading"))

    for i, line in enumerate(code_lines):
        y = pad + 28 + i * line_h
        is_error = "ERROR" in line or "TypeError" in line or "Check" in line
        color = (255, 100, 100) if is_error else (200, 200, 200)
        if line:
            draw.text((pad + 30, y), line, fill=color, font=fm)
            bb = draw.textbbox((pad + 30, y), line, font=fm)
            cls = "label"
            elements.append(([bb[0], bb[1], bb[2], bb[3]], cls))
    return img, elements


def gen_github_issue(idx: int) -> tuple[Image.Image, list[tuple[list[int], str]]]:
    """GitHub issue-style page."""
    H, pad = 500, 20
    img = Image.new("RGB", (OOD_W, H), (255, 255, 255))
    draw = ImageDraw.Draw(img)
    fh = _font(24)
    fb = _font(18)
    ft = _font(15)
    elements = []

    title = f"Bug: NullPointerException in process() #{idx + 100}"
    draw.text((pad, pad), title, fill=(20, 20, 20), font=fh)
    tbb = draw.textbbox((pad, pad), title, font=fh)
    elements.append(([tbb[0], tbb[1], tbb[2], tbb[3]], "heading"))

    meta = "opened 2 hours ago by user123 · 3 comments"
    draw.text((pad, 58), meta, fill=(100, 100, 100), font=ft)
    mbb = draw.textbbox((pad, 58), meta, font=ft)
    elements.append(([mbb[0], mbb[1], mbb[2], mbb[3]], "label"))

    body_lines = [
        "## Describe the bug",
        f"When calling process() with None value (case #{idx}),",
        "a NullPointerException is thrown instead of a clear error.",
        "",
        "## Steps to Reproduce",
        "1. Call process(None)",
        "2. Observe exception",
        "## Expected behavior",
        "Should raise ValueError with descriptive message.",
    ]
    y = 90
    for line in body_lines:
        if line:
            draw.text((pad, y), line, fill=(40, 40, 40), font=fb)
            bb = draw.textbbox((pad, y), line, font=fb)
            elements.append(([bb[0], bb[1], bb[2], bb[3]], "label"))
        y += 26

    for label_text, color in [("Comment", (37, 99, 235)), ("Close issue", (100, 100, 100))]:
        bx1, by1 = pad, y + 10
        bx2, by2 = bx1 + 130, by1 + 34
        draw.rectangle([bx1, by1, bx2, by2], fill=color)
        draw.text((bx1 + 8, by1 + 7), label_text, fill=(255, 255, 255), font=ft)
        elements.append(([bx1, by1, bx2, by2], "button"))
        pad += 150

    return img, elements


def gen_chat_panel(idx: int) -> tuple[Image.Image, list[tuple[list[int], str]]]:
    """Slack/Teams-style chat."""
    messages = [
        ("alice", f"hey, the build #{idx} broke again"),
        ("bob",   "checking logs now"),
        ("alice", "looks like the docker image is stale"),
        ("bob",   "rebuilding... give me 5 min"),
        ("alice", "thanks! pinging Carlos too"),
    ]
    line_h, pad = 54, 12
    H = pad + len(messages) * line_h + pad
    img = Image.new("RGB", (OOD_W, H), (250, 250, 250))
    draw = ImageDraw.Draw(img)
    fu = _font(17)
    fm = _font(16)
    elements = []
    for i, (user, msg) in enumerate(messages):
        y = pad + i * line_h
        draw.text((pad, y), user, fill=(60, 60, 200), font=fu)
        ubb = draw.textbbox((pad, y), user, font=fu)
        elements.append(([ubb[0], ubb[1], ubb[2], ubb[3]], "heading"))
        draw.text((pad, y + 22), msg, fill=(30, 30, 30), font=fm)
        mbb = draw.textbbox((pad, y + 22), msg, font=fm)
        elements.append(([mbb[0], mbb[1], mbb[2], mbb[3]], "label"))
    return img, elements


OOD_GENERATORS = [gen_terminal, gen_config_form, gen_vscode_panel, gen_github_issue, gen_chat_panel]


def generate_ood_dataset() -> list[tuple[Image.Image, str]]:
    """Returns list of (96x96 crop, farscry_class)."""
    crops = []
    out_dir = Path("diff_test/ood_screenshots")
    out_dir.mkdir(parents=True, exist_ok=True)

    for gen_idx, gen_fn in enumerate(OOD_GENERATORS):
        for variant in range(4):
            img, elements = gen_fn(variant)
            img.save(out_dir / f"type{gen_idx}_var{variant}.png")
            W, H = img.size
            for bounds, cls in elements:
                x1, y1, x2, y2 = bounds
                x1, y1 = max(0, x1), max(0, y1)
                x2, y2 = min(W, x2), min(H, y2)
                if (x2 - x1) < MIN_CROP_PX or (y2 - y1) < MIN_CROP_PX:
                    continue
                crop = img.crop((x1, y1, x2, y2)).resize((IMG_SIZE, IMG_SIZE), Image.LANCZOS)
                crops.append((crop, cls))

    print(f"OOD: 20 screenshots generated -> {out_dir}")
    print(f"OOD: {len(crops)} labeled crops")
    return crops


def main():
    t0 = time.time()

    RICO_DIR = Path("semantic_annotations")
    print(f"\n[1/5] Loading RICO raw data from {RICO_DIR} ...")
    if not RICO_DIR.exists():
        print(f"  ERROR: {RICO_DIR} not found.")
        print("  Download: https://storage.googleapis.com/crowdstf-rico-uiuc-4540/rico_dataset_v0.1/semantic_annotations.zip")
        sys.exit(1)

    loader = RicoRawLoader(
        annotations_dir=RICO_DIR,
        screenshots_dir=RICO_DIR,
        img_size=IMG_SIZE,
        seed=SEED,
    )
    print(f"  {len(loader._json_files)} screens available")

    print(f"\n[2/5] Extracting crops from {TRAIN_LIMIT} RICO train screenshots...")
    train_crops = loader.load(max_screens=TRAIN_LIMIT, split="train")

    print(f"  {len(train_crops)} train crops extracted")
    dist = Counter(c for _, c in train_crops)
    for cls in CLASSES:
        print(f"    {cls:10s}: {dist.get(cls, 0):5d}")

    missing = [c for c in CLASSES if dist.get(c, 0) == 0]
    if missing:
        print(f"  WARNING: classes with 0 samples: {missing}")
        train_crops = [(img, "unknown" if c in missing else c) for img, c in train_crops]

    TEST_LIMIT = 500
    print(f"\n[3/5] Extracting crops from {TEST_LIMIT} RICO test screenshots...")
    test_crops = loader.load(max_screens=TEST_LIMIT, split="test")
    print(f"  {len(test_crops)} test crops extracted")

    print(f"\n[4/5] Training MobileNetV3-Small ({EPOCHS} epochs, batch={BATCH_SIZE}, lr={LR})...")
    sampler  = build_weighted_sampler(train_crops)
    train_ds = CropDataset(train_crops, TRAIN_TF)
    test_ds  = CropDataset(test_crops, EVAL_TF)
    train_dl = DataLoader(train_ds, batch_size=BATCH_SIZE, sampler=sampler, num_workers=0)
    test_dl  = DataLoader(test_ds,  batch_size=BATCH_SIZE, shuffle=False,   num_workers=0)

    model     = build_model()
    cnt       = Counter(c for _, c in train_crops)
    total     = sum(cnt.values())
    weights   = torch.tensor([total / (N_CLASSES * cnt.get(c, 1)) for c in CLASSES],
                              dtype=torch.float32).to(DEVICE)
    criterion = nn.CrossEntropyLoss(weight=weights)
    optimizer = optim.Adam(model.parameters(), lr=LR)
    scheduler = optim.lr_scheduler.CosineAnnealingLR(optimizer, T_max=EPOCHS)

    for epoch in range(1, EPOCHS + 1):
        loss, acc = train_epoch(model, train_dl, optimizer, criterion)
        scheduler.step()
        print(f"  Epoch {epoch}/{EPOCHS}  loss={loss:.4f}  train_acc={acc:.3f}")

    print("\n[5/5] Evaluation...")

    id_acc, id_correct, id_total = evaluate(model, test_dl)
    print(f"\n  IN-DISTRIBUTION (RICO test, {len(test_crops)} crops):")
    print(f"  Overall accuracy: {id_acc:.3f} ({id_acc*100:.1f}%)")
    for i, cls in enumerate(CLASSES):
        n = id_total.get(i, 0)
        c = id_correct.get(i, 0)
        acc_c = c / n if n else 0.0
        print(f"    {cls:10s}: {c:4d}/{n:4d}  ({acc_c*100:.0f}%)")

    ood_crops = generate_ood_dataset()
    if ood_crops:
        ood_ds = CropDataset(ood_crops, EVAL_TF)
        ood_dl = DataLoader(ood_ds, batch_size=BATCH_SIZE, shuffle=False, num_workers=0)
        ood_acc, ood_correct, ood_total = evaluate(model, ood_dl)
        print(f"\n  OUT-OF-DISTRIBUTION (20 dev tool screenshots, {len(ood_crops)} crops):")
        print(f"  Overall accuracy: {ood_acc:.3f} ({ood_acc*100:.1f}%)")
        for i, cls in enumerate(CLASSES):
            n = ood_total.get(i, 0)
            c = ood_correct.get(i, 0)
            if n:
                acc_c = c / n
                print(f"    {cls:10s}: {c:4d}/{n:4d}  ({acc_c*100:.0f}%)")

        drop = id_acc - ood_acc
        drop_pct = drop * 100

        print("\n" + "═" * 60)
        print("DOMAIN VALIDATION REPORT")
        print("═" * 60)
        print(f"  Architecture       : MobileNetV3-Small INT8 (proxy FP32)")
        print(f"  Classes            : {', '.join(CLASSES)}")
        print(f"  Train data         : RICO {TRAIN_LIMIT} screenshots")
        print(f"  In-dist accuracy   : {id_acc*100:.1f}%  (RICO test)")
        print(f"  OOD accuracy       : {ood_acc*100:.1f}%  (dev tools)")
        print(f"  Accuracy drop      : {drop_pct:.1f}%")
        print()

        if drop_pct < 10:
            verdict = " GO - RICO sufficient, no supplementation needed"
        elif drop_pct < 20:
            verdict = "  SUPPLEMENT - add dev tool screenshots to training mix"
        else:
            verdict = "No NO-GO - domain gap too large, need different data strategy"
        print(f"  Verdict            : {verdict}")
        print("═" * 60)

        print("\nPer-class OOD performance:")
        class_drops = []
        for i, cls in enumerate(CLASSES):
            id_n = id_total.get(i, 0)
            ood_n = ood_total.get(i, 0)
            if id_n > 0 and ood_n > 0:
                id_c = id_correct.get(i, 0) / id_n
                ood_c = ood_correct.get(i, 0) / ood_n
                class_drops.append((cls, id_c, ood_c, id_c - ood_c))
        class_drops.sort(key=lambda x: -x[3])
        for cls, id_c, ood_c, d in class_drops:
            flag = "" if d > 0.15 else ""
            print(f"  {flag} {cls:10s}: RICO={id_c*100:.0f}%  OOD={ood_c*100:.0f}%  drop={d*100:.0f}%")

    elapsed = time.time() - t0
    print(f"\nTotal time: {elapsed:.0f}s ({elapsed/60:.1f}min)")


if __name__ == "__main__":
    main()
