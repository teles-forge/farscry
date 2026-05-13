
import numpy as np
import coremltools as ct
import onnxruntime as ort
from PIL import Image

sess = ort.InferenceSession("../spike/models/en_pp-ocrv5_mobile_rec.onnx", providers=["CPUExecutionProvider"])
cpu_model = ct.models.MLModel("models/en_pp-ocrv5_mobile_rec_test.mlpackage", compute_units=ct.ComputeUnit.CPU_ONLY)
all_model = ct.models.MLModel("models/en_pp-ocrv5_mobile_rec_test.mlpackage", compute_units=ct.ComputeUnit.ALL)
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

def make_input(bbox_pts):
    xs = [p[0] for p in bbox_pts]; ys = [p[1] for p in bbox_pts]
    crop = img.crop((int(min(xs)), int(min(ys)), int(max(xs)), int(max(ys))))
    ow, oh = crop.size
    new_w = min(int(48 * ow / max(oh, 1)), 320)
    crop_r = crop.resize((new_w, 48), Image.BILINEAR)
    arr = np.array(crop_r).astype(np.float32) / 255.0
    arr_bgr = arr[:,:,::-1]
    norm = (arr_bgr - 0.5) / 0.5
    t = np.zeros((1, 3, 48, 320), dtype=np.float32)
    t[0, :, :, :new_w] = norm.transpose(2, 0, 1)
    return t

for name, bbox in [
    ("ORT  (y≈8-43)", [(21.0,8.0),(216.0,11.0),(215.0,43.0),(20.0,41.0)]),
    ("CML  (y≈2-35)", [(16.0,2.0),(220.0,5.0),(220.0,35.0),(15.0,32.0)]),
]:
    t = make_input(bbox)
    ort_out = sess.run(None, {"x": t})[0]
    cpu_out = cpu_model.predict({"x": t})
    all_out = all_model.predict({"x": t})
    k = list(cpu_out.keys())[0]
    print(f"Region {name}")
    print(f"  ORT: '{ctc(ort_out)}'")
    print(f"  CPU: '{ctc(cpu_out[k])}'")
    print(f"  ALL: '{ctc(all_out[k])}'")
