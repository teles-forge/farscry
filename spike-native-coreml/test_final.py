
"""
Final correctness test after fixing BOTH bugs:
  Bug 1: normalization was (pixel/255-0.5)/0.5  <- this was actually correct
  Bug 2: CTC blank index = 0 (NOT last index)

oar-ocr-core source confirms blank_index = 0 in CTCLabelDecode::new().
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

print(f"Dict size: {len(chars)}, first 5: {chars[:5]}")
print(f"Model output classes: 438 (= 1 blank + {len(chars)} chars + 1 space)")

def ctc_greedy(logits, blank_idx=0):
    """
    CTC greedy decode with blank at index 0 (PaddleOCR convention).
    chars[i] maps to class index i+1 (blank occupies index 0).
    """
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
    """
    Extract crop, resize to height=48 (aspect-preserving), pad to 320,
    normalize (pixel/255 - 0.5) / 0.5 in BGR order, return [1,3,48,320].
    """
    xs = [p[0] for p in bbox_pts]
    ys = [p[1] for p in bbox_pts]
    x1, y1 = int(min(xs)), int(min(ys))
    x2, y2 = int(max(xs)), int(max(ys))
    crop = img_rgb.crop((x1, y1, x2, y2))

    ow, oh = crop.size
    ratio  = ow / max(oh, 1)
    new_w  = min(int(48 * ratio), 320)
    crop_r = crop.resize((new_w, 48), Image.BILINEAR)

    padded = Image.new("RGB", (320, 48), (255, 255, 255))
    padded.paste(crop_r, (0, 0))

    arr     = np.array(padded).astype(np.float32) / 255.0
    arr_bgr = arr[:, :, ::-1]
    norm    = (arr_bgr - 0.5) / 0.5
    tensor  = norm.transpose(2, 0, 1)[np.newaxis]
    return tensor

img       = Image.open(img_path).convert("RGB")
bbox_pts  = [(15.0, 94.0), (221.0, 97.0), (220.0, 128.0), (15.0, 125.0)]
tensor_in = make_rec_input(img, bbox_pts)

print(f"\nInput shape: {tensor_in.shape}")
print(f"Pixel at [0,0,0,0] after norm: {tensor_in[0,0,0,0]:.4f}  (white=1.0, black=-1.0)")

ort_out = sess.run(None, {"x": tensor_in})[0]
cpu_raw = cpu_model.predict({"x": tensor_in})
all_raw = all_model.predict({"x": tensor_in})
cpu_key = list(cpu_raw.keys())[0]
all_key = list(all_raw.keys())[0]

ort_text = ctc_greedy(ort_out)
cpu_text = ctc_greedy(cpu_raw[cpu_key])
all_text = ctc_greedy(all_raw[all_key])

print(f"\n{'═'*55}")
print(f"  ORT  decoded: '{ort_text}'")
print(f"  CPU  decoded: '{cpu_text}'")
print(f"  ALL  decoded: '{all_text}'")
print(f"  Expected:      'Payment Settings'")
print(f"{'═'*55}")
print(f"  ORT  correct: {'YES OK' if 'Payment' in ort_text else 'NO FAIL'}")
print(f"  CPU  correct: {'YES OK' if 'Payment' in cpu_text else 'NO FAIL'}")
print(f"  ALL  correct: {'YES OK' if 'Payment' in all_text else 'NO FAIL'}")
print(f"\n  Max diff ORT vs CPU: {np.abs(ort_out - cpu_raw[cpu_key]).max():.2e}")
print(f"  Max diff ORT vs ALL: {np.abs(ort_out - all_raw[all_key]).max():.2e}")
print(f"\nVERDICT: The CoreML model conversion is {'CORRECT OK' if 'Payment' in all_text else 'STILL BROKEN FAIL'}")
