
"""
Compare mlpackage (Python can load) vs mlmodelc (Rust loads).
Uses the SAME mlpackage that was compiled to the mlmodelc.
"""
import numpy as np
import coremltools as ct
import onnxruntime as ort
from PIL import Image

sess = ort.InferenceSession("../spike/models/en_pp-ocrv5_mobile_rec.onnx", providers=["CPUExecutionProvider"])
pkg_model = ct.models.MLModel("models/en_pp-ocrv5_mobile_rec.mlpackage", compute_units=ct.ComputeUnit.ALL)
with open("../spike/models/ppocrv5_en_dict.txt") as f:
    chars = [l.rstrip("\n") for l in f]
img = Image.open("../spike/test.png").convert("RGB")

def ctc(logits, blank=0):
    ids = np.argmax(logits[0], axis=-1)
    result, prev = [], None
    for idx in ids:
        if int(idx) != blank and int(idx) != prev:
            ci = int(idx) - 1
            if 0 <= ci < len(chars):
                result.append(chars[ci])
        prev = int(idx)
    return "".join(result)

cml_bbox = [(16.0, 2.0), (220.0, 5.0), (220.0, 35.0), (15.0, 32.0)]
xs = [p[0] for p in cml_bbox]; ys = [p[1] for p in cml_bbox]
crop = img.crop((int(min(xs)), int(min(ys)), int(max(xs)), int(max(ys))))
ow, oh = crop.size
new_w = min(int(48 * ow / max(oh, 1)), 320)
crop_r = crop.resize((new_w, 48), Image.BILINEAR)
arr = np.array(crop_r).astype(np.float32) / 255.0
arr_bgr = arr[:,:,::-1]
norm = (arr_bgr - 0.5) / 0.5
t = np.zeros((1, 3, 48, 320), dtype=np.float32)
t[0, :, :, :new_w] = norm.transpose(2, 0, 1)

print(f"Input shape: {t.shape}, first 5 vals: {t[0,0,0,:5]}")

ort_out = sess.run(None, {"x": t})[0]
pkg_raw = pkg_model.predict({"x": t})
pkg_key = list(pkg_raw.keys())[0]

print(f"ORT  decoded: '{ctc(ort_out)}'")
print(f"PKG  decoded: '{ctc(pkg_raw[pkg_key])}'")
print(f"\nORT first 5 logits: {ort_out[0,0,:5]}")
print(f"PKG first 5 logits: {pkg_raw[pkg_key][0,0,:5]}")
print(f"Max diff: {np.abs(ort_out - pkg_raw[pkg_key]).max():.2e}")
