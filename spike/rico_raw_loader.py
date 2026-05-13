
"""
RICO raw semantic_annotations JSON parser.

Parses the recursive view hierarchy JSONs from:
  rico_dataset_v0.1/semantic_annotations/<id>.json

Each JSON file has a root element with `componentLabel` and recursive `children`.
This loader:
  - Recursively flattens the full tree
  - Extracts every element that has a non-null componentLabel
  - Maps to farscry 5-class taxonomy
  - Returns (crop_PIL, class_str) pairs

Usage:
  loader = RicoRawLoader("path/to/semantic_annotations/", "path/to/screenshots/")
  crops = loader.load(max_screens=1000)
"""

import json
import random
import statistics
from pathlib import Path
from typing import Iterator

from PIL import Image


RICO_CLASSES = [
    "Text",
    "Image",
    "Icon",
    "Text Button",
    "List Item",
    "Input",
    "Background Image",
    "Card",
    "Web View",
    "Radio Button",
    "Drawer",
    "Checkbox",
    "Advertisement",
    "Modal",
    "Pager Indicator",
    "Slider",
    "On/Off Switch",
    "Button Bar",
    "Toolbar",
    "Number Stepper",
    "Multi-Tab",
    "Date Picker",
    "Map View",
    "Video",
    "Bottom Navigation",
]

RICO_TO_FARSCRY_RAW = {
    "Text":         "text_raw",
    "Text Button":  "button",
    "Input":        "input",
    "Radio Button": "button",
    "Checkbox":     "button",
    "On/Off Switch":"button",
}

MIN_CROP_PX = 10


def _flatten_node(node: dict, acc: list[tuple[list[int], str, float]]) -> None:
    """
    Recursively collect (bounds, farscry_raw_class, height) from node and children.
    `farscry_raw_class` is "text_raw" for Text, "button"/"input" for others, "unknown" for rest.
    """
    if not isinstance(node, dict):
        return

    label_str = node.get("componentLabel")
    if label_str is not None:
        farscry = RICO_TO_FARSCRY_RAW.get(label_str, "unknown")
        bounds = node.get("bounds", [])
        if len(bounds) == 4:
            x1, y1, x2, y2 = bounds
            h = y2 - y1
            acc.append(([x1, y1, x2, y2], farscry, float(h)))

    for child in node.get("children", []) or []:
        _flatten_node(child, acc)


def _apply_heading_heuristic(
    raw: list[tuple[list[int], str, float]]
) -> list[tuple[list[int], str]]:
    """
    Split "text_raw" elements into "label" or "heading" by relative height.
    Elements with height > 1.5 x median(text heights) -> heading; rest -> label.
    """
    text_heights = [h for _, cls, h in raw if cls == "text_raw"]
    if text_heights:
        median_h = statistics.median(text_heights)
        threshold = median_h * 1.5
    else:
        threshold = float("inf")

    result = []
    for bounds, cls, h in raw:
        if cls == "text_raw":
            cls = "heading" if h >= threshold else "label"
        result.append((bounds, cls))
    return result


def _find_screenshot(screens_dir: Path, stem: str) -> Path | None:
    """Find screenshot with given numeric stem. Tries .jpg, .png."""
    for ext in (".jpg", ".png", ".jpeg"):
        p = screens_dir / f"{stem}{ext}"
        if p.exists():
            return p
    return None


class RicoRawLoader:
    """
    Loads crops from raw RICO semantic_annotations JSON files.

    Args:
        annotations_dir: path to the `semantic_annotations/` folder from the zip.
        screenshots_dir: path to the screenshots folder (same zip or separate).
                         If None, tries to find a `combined/` folder inside
                         the same parent as annotations_dir.
        img_size:        output crop size (square).
        seed:            random seed for shuffling.
    """

    def __init__(
        self,
        annotations_dir: str | Path,
        screenshots_dir: str | Path | None = None,
        img_size: int = 96,
        seed: int = 42,
    ):
        self.ann_dir = Path(annotations_dir)
        self.img_size = img_size
        self.seed = seed

        if screenshots_dir is not None:
            self.screens_dir = Path(screenshots_dir)
        else:
            parent = self.ann_dir.parent
            for candidate in ["combined", "screenshots", "images", "imgs"]:
                d = parent / candidate
                if d.exists():
                    self.screens_dir = d
                    break
            else:
                self.screens_dir = self.ann_dir

        json_files = sorted(self.ann_dir.glob("*.json"))
        self._json_files = json_files
        print(f"RicoRawLoader: found {len(json_files)} JSON files in {self.ann_dir}")
        print(f"  screenshots dir: {self.screens_dir}")

    def _load_one(
        self, json_path: Path
    ) -> list[tuple[Image.Image, str]]:
        """Load crops from a single annotation JSON. Returns [] on error."""
        stem = json_path.stem
        img_path = _find_screenshot(self.screens_dir, stem)
        if img_path is None:
            return []
        try:
            img = Image.open(img_path).convert("RGB")
        except Exception:
            return []

        W, H = img.size
        try:
            with open(json_path, "r", encoding="utf-8") as f:
                data = json.load(f)
        except Exception:
            return []

        root = data if "bounds" in data else data.get("root", data)
        raw: list[tuple[list[int], str, float]] = []
        _flatten_node(root, raw)
        if not raw:
            return []

        elements = _apply_heading_heuristic(raw)

        crops = []
        for bounds, cls in elements:
            x1, y1, x2, y2 = bounds
            x1, y1 = max(0, x1), max(0, y1)
            x2, y2 = min(W, x2), min(H, y2)
            if (x2 - x1) < MIN_CROP_PX or (y2 - y1) < MIN_CROP_PX:
                continue
            crop = img.crop((x1, y1, x2, y2)).resize(
                (self.img_size, self.img_size), Image.LANCZOS
            )
            crops.append((crop, cls))
        return crops

    def load(
        self,
        max_screens: int = 1000,
        split: str = "train",
        train_ratio: float = 0.85,
    ) -> list[tuple[Image.Image, str]]:
        """
        Load up to max_screens and return list of (crop, class_str).

        split: "train" | "test" | "all"
        """
        rng = random.Random(self.seed)
        all_files = list(self._json_files)
        rng.shuffle(all_files)

        n_train = int(len(all_files) * train_ratio)
        if split == "train":
            files = all_files[:n_train]
        elif split == "test":
            files = all_files[n_train:]
        else:
            files = all_files

        files = files[:max_screens]

        from tqdm import tqdm
        crops: list[tuple[Image.Image, str]] = []
        for jf in tqdm(files, desc=f"  {split} crops ({split})", leave=False):
            crops.extend(self._load_one(jf))
        return crops

    def iter_files(
        self, max_screens: int | None = None
    ) -> Iterator[tuple[Path, list[tuple[Image.Image, str]]]]:
        """Iterate (json_path, crops) lazily for memory-efficient processing."""
        files = list(self._json_files)
        if max_screens is not None:
            files = files[:max_screens]
        for jf in files:
            yield jf, self._load_one(jf)


def diagnose(annotations_dir: str, screenshots_dir: str | None = None,
             n: int = 20) -> None:
    """Print class distribution for first n screens."""
    from collections import Counter
    loader = RicoRawLoader(annotations_dir, screenshots_dir)
    counts: Counter = Counter()
    processed = 0
    for jf, crops in loader.iter_files(n):
        for _, cls in crops:
            counts[cls] += 1
        processed += 1
    print(f"\nDiagnosis over {processed} screens:")
    print(f"  Total crops: {sum(counts.values())}")
    for cls, c in counts.most_common():
        pct = c / sum(counts.values()) * 100
        print(f"  {cls:10s}: {c:5d} ({pct:.1f}%)")


if __name__ == "__main__":
    import sys
    if len(sys.argv) < 2:
        print("Usage: uv run rico_raw_loader.py <annotations_dir> [screenshots_dir]")
        sys.exit(1)
    ann = sys.argv[1]
    scr = sys.argv[2] if len(sys.argv) > 2 else None
    diagnose(ann, scr, n=50)
