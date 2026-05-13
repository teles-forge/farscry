
"""
Test ANE vs CPU on the same model + same input.
Also test with real image crop (same as what Rust will process).

Hypothesis: garbled text = ANE changes results for this model.
If CPU is correct but ANE is not -> force CPU for recognition in Rust.
"""

import os
import numpy as np
import coremltools as ct
import onnxruntime as ort
from PIL import Image

import onnx
import onnx2torch
import torch

onnx_path    = "../spike/models/en_pp-ocrv5_mobile_rec.onnx"
mlpkg_path   = "models/en_pp-ocrv5_mobile_rec_test.mlpackage"

print("=== Converting ONNX -> PyTorch -> CoreML (fresh) ===")
onnx_model  = onnx.load(onnx_path)
torch_model = onnx2torch.convert(onnx_model).eval()
traced = torch.jit.trace(torch_model, torch.zeros(1, 3, 48, 320))
inputs = [ct.TensorType(name="x", shape=(1, 3, 48, 320), dtype=np.float32)]
cml = ct.convert(
    traced, inputs=inputs, convert_to="mlprogram",
    minimum_deployment_target=ct.target.macOS13,
    compute_precision=ct.precision.FLOAT32,
)
cml.save(mlpkg_path)
print(f"Saved to {mlpkg_path}")

print("\n=== A: Random input - ORT vs CoreML CPU ===")

np.random.seed(42)
rand_in = np.random.randn(1, 3, 48, 320).astype(np.float32)

sess = ort.InferenceSession(onnx_path, providers=["CPUExecutionProvider"])
ort_out = sess.run(None, {"x": rand_in})[0]

cpu_model = ct.models.MLModel(mlpkg_path, compute_units=ct.ComputeUnit.CPU_ONLY)
cpu_raw = cpu_model.predict({"x": rand_in})
cpu_key = list(cpu_raw.keys())[0]
cpu_out = cpu_raw[cpu_key]

diff_cpu = np.abs(ort_out - cpu_out).max()
print(f"ORT  t=0 argmax: {np.argmax(ort_out[0,0,:])}  logits[:5]: {ort_out[0,0,:5]}")
print(f"CPU  t=0 argmax: {np.argmax(cpu_out[0,0,:])}  logits[:5]: {cpu_out[0,0,:5]}")
print(f"Max diff ORT vs CPU:  {diff_cpu:.8f}  -> {'OK OK' if diff_cpu < 0.01 else 'BUG FAIL'}")

print("\n=== B: Random input - ORT vs CoreML ALL (ANE included) ===")
all_model = ct.models.MLModel(mlpkg_path, compute_units=ct.ComputeUnit.ALL)
all_raw = all_model.predict({"x": rand_in})
all_key = list(all_raw.keys())[0]
all_out = all_raw[all_key]

diff_all = np.abs(ort_out - all_out).max()
print(f"ORT  t=0 argmax: {np.argmax(ort_out[0,0,:])}  logits[:5]: {ort_out[0,0,:5]}")
print(f"ALL  t=0 argmax: {np.argmax(all_out[0,0,:])}  logits[:5]: {all_out[0,0,:5]}")
print(f"Max diff ORT vs ALL:  {diff_all:.8f}  -> {'OK OK' if diff_all < 0.01 else 'BUG FAIL <- ANE corrupts recognition'}")

print("\n=== C: Real image crop - ORT vs CoreML CPU ===")

img_path = "../spike/test.png"
if not os.path.exists(img_path):
    print("spike/test.png not found - skipping real-image test")
else:
    img = Image.open(img_path).convert("RGB")
    crop = img.crop((15, 94, 221, 128))
    ow, oh = crop.size
    ratio = ow / oh
    new_w = min(int(48 * ratio), 320)
    crop_r = crop.resize((new_w, 48), Image.BILINEAR)
    padded = Image.new("RGB", (320, 48), (255, 255, 255))
    padded.paste(crop_r, (0, 0))

    arr = np.array(padded).astype(np.float32) / 255.0

    MEAN = np.array([0.485, 0.456, 0.406], dtype=np.float32)
    STD  = np.array([0.229, 0.224, 0.225], dtype=np.float32)
    arr_bgr = arr[:, :, ::-1]
    norm = (arr_bgr - MEAN) / STD
    tensor_in = norm.transpose(2, 0, 1)[np.newaxis]

    ort_crop = sess.run(None, {"x": tensor_in})[0]
    cpu_crop = cpu_model.predict({"x": tensor_in})
    all_crop = all_model.predict({"x": tensor_in})
    cpu_crop_np = cpu_crop[cpu_key]
    all_crop_np = all_crop[all_key]

    def ctc_greedy(logits, chars):
        """Minimal CTC greedy decode (blank = last index)."""
        ids = np.argmax(logits[0], axis=-1)
        blank = logits.shape[-1] - 1
        result, prev = [], None
        for idx in ids:
            if idx != blank and idx != prev:
                if idx < len(chars):
                    result.append(chars[idx])
            prev = idx
        return "".join(result)

    dict_path = "../spike/models/ppocrv5_en_dict.txt"
    if os.path.exists(dict_path):
        with open(dict_path) as f:
            chars = [l.rstrip("\n") for l in f]
    else:
        chars = [str(i) for i in range(ort_crop.shape[-1]-1)]

    print(f"Input tensor shape: {tensor_in.shape}")
    print(f"Input first 5 vals: {tensor_in[0,0,0,:5]}")
    ort_text  = ctc_greedy(ort_crop,  chars)
    cpu_text  = ctc_greedy(cpu_crop_np, chars)
    all_text  = ctc_greedy(all_crop_np, chars)

    print(f"\nORT  decoded: '{ort_text}'")
    print(f"CPU  decoded: '{cpu_text}'")
    print(f"ALL  decoded: '{all_text}'")
    print(f"\nExpected: 'Payment Settings'")
    print(f"ORT correct:  {'YES OK' if 'Payment' in ort_text else 'NO'}")
    print(f"CPU correct:  {'YES OK' if 'Payment' in cpu_text else 'NO'}")
    print(f"ALL correct:  {'YES OK' if 'Payment' in all_text else 'NO'}")

    diff_real_cpu = np.abs(ort_crop - cpu_crop_np).max()
    diff_real_all = np.abs(ort_crop - all_crop_np).max()
    print(f"\nMax diff (real crop) ORT vs CPU: {diff_real_cpu:.6f}")
    print(f"Max diff (real crop) ORT vs ALL: {diff_real_all:.6f}")
