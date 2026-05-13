
"""
Correct comparison: run ORT recognition on BOTH bboxes (ORT's and CoreML's).
This tells us whether CoreML recognition on the CoreML-detected crop matches ORT.
"""

import numpy as np
import coremltools as ct
import onnxruntime as ort
from PIL import Image

onnx_path  = "../spike/models/en_pp-ocrv5_mobile_rec.onnx"
mlpkg_path = "models/en_pp-ocrv5_mobile_rec_test.mlpackage"
dict_path  = "../spike/models/ppocrv5_en_dict.txt"
img_path   = "../spike/test.png"

sess      = ort.InferenceSession(onnx_path, providers=["CPUExecutionProvider"])
cpu_model = ct.models.MLModel(mlpkg_path, compute_units=ct.ComputeUnit.CPU_ONLY)
all_model = ct.models.MLModel(mlpkg_path, compute_units=ct.ComputeUnit.ALL)

with open(dict_path) as f:
    chars = [l.rstrip("\n") for l in f]

def ctc_greedy(logits, blank_idx=0):
    ids = np.argmax(logits[0], axis=-1)
    result, prev = [], None
    for idx in ids:
        if int(idx) != blank_idx and int(idx) != prev:
            char_idx = int(idx) - 1
            if 0 <= char_idx < len(chars):
                result.append(chars[char_idx])
        prev = int(idx)
    return "".join(result)

def make_rec_input(img_rgb, bbox_pts):
    """Perspective-style crop + (pixel/255-0.5)/0.5 norm, BGR, zero-padded to 320."""
    xs = [p[0] for p in bbox_pts]
    ys = [p[1] for p in bbox_pts]
    x1, y1 = int(min(xs)), int(min(ys))
    x2, y2 = int(max(xs)), int(max(ys))
    crop = img_rgb.crop((x1, y1, x2, y2))
    ow, oh = crop.size
    ratio = ow / max(oh, 1)
    new_w = min(int(48 * ratio), 320)
    crop_r = crop.resize((new_w, 48), Image.BILINEAR)
    arr = np.array(crop_r).astype(np.float32) / 255.0
    arr_bgr = arr[:, :, ::-1]
    norm = (arr_bgr - 0.5) / 0.5
    tensor = np.zeros((1, 3, 48, 320), dtype=np.float32)
    tensor[0, :, :, :new_w] = norm.transpose(2, 0, 1)
    return tensor

img = Image.open(img_path).convert("RGB")

ort_bbox = [(21.0, 8.0), (216.0, 11.0), (215.0, 43.0), (20.0, 41.0)]

cml_bbox = [(15.0, 94.0), (221.0, 97.0), (220.0, 128.0), (15.0, 125.0)]

print(f"Image size: {img.size}")
print()

for name, bbox in [("ORT region  (y≈8-43)",  ort_bbox),
                   ("CoreML region (y≈94-128)", cml_bbox)]:
    tensor_in = make_rec_input(img, bbox)
    ort_out  = sess.run(None, {"x": tensor_in})[0]
    cpu_raw  = cpu_model.predict({"x": tensor_in})
    all_raw  = all_model.predict({"x": tensor_in})
    cpu_key  = list(cpu_raw.keys())[0]
    all_key  = list(all_raw.keys())[0]

    ort_text = ctc_greedy(ort_out)
    cpu_text = ctc_greedy(cpu_raw[cpu_key])
    all_text = ctc_greedy(all_raw[all_key])

    diff_cpu = np.abs(ort_out - cpu_raw[cpu_key]).max()
    diff_all = np.abs(ort_out - all_raw[all_key]).max()

    print(f"{'─'*60}")
    print(f"Region: {name}")
    print(f"  ORT  decoded: '{ort_text}'")
    print(f"  CPU  decoded: '{cpu_text}'")
    print(f"  ALL  decoded: '{all_text}'")
    print(f"  max diff ORT vs CPU: {diff_cpu:.2e}")
    print(f"  max diff ORT vs ALL: {diff_all:.2e}")
    print(f"  ORT==CPU: {'YES OK' if ort_text == cpu_text else 'NO FAIL'}")
    print(f"  ORT==ALL: {'YES OK' if ort_text == all_text else 'NO FAIL'}")
