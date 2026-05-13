
"""
Test recognition with the CORRECT normalization: (pixel/255 - 0.5) / 0.5
Expected: ORT, CPU CoreML, ALL CoreML all decode 'Payment Settings'
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

def ctc_greedy(logits):
    ids = np.argmax(logits[0], axis=-1)
    blank = logits.shape[-1] - 1
    result, prev = [], None
    for idx in ids:
        if idx != blank and idx != prev:
            if idx < len(chars):
                result.append(chars[idx])
        prev = idx
    return "".join(result)

def make_rec_input(img_rgb, bbox_pts):
    """Extract crop, resize to 48h, normalize (pixel/255 - 0.5) / 0.5, BGR order."""
    xs = [p[0] for p in bbox_pts]
    ys = [p[1] for p in bbox_pts]
    x1, y1, x2, y2 = int(min(xs)), int(min(ys)), int(max(xs)), int(max(ys))
    crop = img_rgb.crop((x1, y1, x2, y2))
    ow, oh = crop.size
    ratio = ow / oh
    new_w = min(int(48 * ratio), 320)
    crop_r = crop.resize((new_w, 48), Image.BILINEAR)
    padded = Image.new("RGB", (320, 48), (255, 255, 255))
    padded.paste(crop_r, (0, 0))

    arr = np.array(padded).astype(np.float32) / 255.0
    arr_bgr = arr[:, :, ::-1]
    norm = (arr_bgr - 0.5) / 0.5
    tensor = norm.transpose(2, 0, 1)[np.newaxis]
    return tensor

img = Image.open(img_path).convert("RGB")
bbox_pts = [(15.0, 94.0), (221.0, 97.0), (220.0, 128.0), (15.0, 125.0)]
tensor_in = make_rec_input(img, bbox_pts)

print(f"Input shape: {tensor_in.shape}")
print(f"First 5 vals channel-0: {tensor_in[0,0,0,:5]}")
print(f"Expected for white padding: {(1.0 - 0.5) / 0.5:.4f}")

ort_out = sess.run(None, {"x": tensor_in})[0]
cpu_raw = cpu_model.predict({"x": tensor_in})
all_raw = all_model.predict({"x": tensor_in})
cpu_key = list(cpu_raw.keys())[0]
all_key = list(all_raw.keys())[0]

ort_text = ctc_greedy(ort_out)
cpu_text = ctc_greedy(cpu_raw[cpu_key])
all_text = ctc_greedy(all_raw[all_key])

print(f"\n{'─'*50}")
print(f"ORT  decoded: '{ort_text}'")
print(f"CPU  decoded: '{cpu_text}'")
print(f"ALL  decoded: '{all_text}'")
print(f"Expected:      'Payment Settings'")
print(f"{'─'*50}")
print(f"ORT  correct: {'YES OK' if 'Payment' in ort_text else 'NO FAIL'}")
print(f"CPU  correct: {'YES OK' if 'Payment' in cpu_text else 'NO FAIL'}")
print(f"ALL  correct: {'YES OK' if 'Payment' in all_text else 'NO FAIL'}")
print(f"\nMax diff ORT vs CPU: {np.abs(ort_out - cpu_raw[cpu_key]).max():.8f}")
print(f"Max diff ORT vs ALL: {np.abs(ort_out - all_raw[all_key]).max():.8f}")
