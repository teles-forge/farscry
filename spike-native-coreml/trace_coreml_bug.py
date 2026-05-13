
"""
Trace where exactly coremltools breaks the recognition model.

Chain: ONNX -> onnx2torch -> torch.jit.trace -> coremltools -> CoreML
        ok          ok       ??? here???    ??? here???

We test each step in isolation.
"""

import onnx
import onnx2torch
import torch
import numpy as np
import onnxruntime as ort
import coremltools as ct

onnx_path = "../spike/models/en_pp-ocrv5_mobile_rec.onnx"
mlpkg_path = "../spike-native-coreml/models/en_pp-ocrv5_mobile_rec.mlpackage"

np.random.seed(42)
test_input_np = np.random.randn(1, 3, 48, 320).astype(np.float32)
test_input_t  = torch.from_numpy(test_input_np)

print("=== Step 1: ORT baseline ===")
sess = ort.InferenceSession(onnx_path, providers=["CPUExecutionProvider"])
ort_out = sess.run(None, {"x": test_input_np})[0]
print(f"ORT t=0 logits[:5]:    {ort_out[0,0,:5]}")
print(f"ORT t=0 argmax:        {np.argmax(ort_out[0,0,:])}")

print("\n=== Step 2: onnx2torch -> eager PyTorch ===")
onnx_model = onnx.load(onnx_path)
torch_model = onnx2torch.convert(onnx_model).eval()
with torch.no_grad():
    eager_out = torch_model(test_input_t)
eager_np = eager_out.numpy() if isinstance(eager_out, torch.Tensor) else eager_out[0].numpy()
print(f"Eager t=0 logits[:5]:  {eager_np[0,0,:5]}")
print(f"Eager t=0 argmax:      {np.argmax(eager_np[0,0,:])}")
print(f"Matches ORT:           {'YES OK' if np.abs(eager_np - ort_out).max() < 1e-3 else 'NO FAIL'}")

print("\n=== Step 3: torch.jit.trace ===")
traced = torch.jit.trace(torch_model, test_input_t)
with torch.no_grad():
    traced_out = traced(test_input_t)
traced_np = traced_out.numpy()
print(f"Traced t=0 logits[:5]: {traced_np[0,0,:5]}")
print(f"Traced t=0 argmax:     {np.argmax(traced_np[0,0,:])}")
print(f"Matches ORT:           {'YES OK' if np.abs(traced_np - ort_out).max() < 1e-3 else 'NO FAIL'}")
print(f"Max diff (trace vs eager): {np.abs(traced_np - eager_np).max():.8f}")

print("\n=== Step 4: Existing CoreML .mlpackage ===")
try:
    coreml_model = ct.models.MLModel(mlpkg_path, compute_units=ct.ComputeUnit.CPU_ONLY)
    coreml_out = coreml_model.predict({"x": test_input_np})
    out_key = list(coreml_out.keys())[0]
    coreml_np = coreml_out[out_key]
    print(f"CoreML output key:         {out_key}")
    print(f"CoreML output shape:       {coreml_np.shape}")
    print(f"CoreML t=0 logits[:5]:     {coreml_np[0,0,:5]}")
    print(f"CoreML t=0 argmax:         {np.argmax(coreml_np[0,0,:])}")
    diff = np.abs(coreml_np - ort_out).max()
    print(f"Max diff (CoreML vs ORT):  {diff:.6f}")
    print(f"Matches ORT:               {'YES OK' if diff < 1e-2 else 'NO FAIL <- BUG HERE'}")
except Exception as e:
    print(f"Could not load CoreML model: {e}")

print("\n=== Step 5: Fresh CoreML conversion (CPU_ONLY - bypassing ANE) ===")
inputs = [ct.TensorType(name="x", shape=(1, 3, 48, 320), dtype=np.float32)]
fresh_model = ct.convert(
    traced,
    inputs=inputs,
    convert_to="mlprogram",
    minimum_deployment_target=ct.target.macOS13,
    compute_precision=ct.precision.FLOAT32,
)

fresh_out_all = fresh_model.predict({"x": test_input_np})
out_key = list(fresh_out_all.keys())[0]
fresh_np = fresh_out_all[out_key]
print(f"Fresh CoreML (CPU) t=0 logits[:5]: {fresh_np[0,0,:5]}")
print(f"Fresh CoreML (CPU) t=0 argmax:     {np.argmax(fresh_np[0,0,:])}")
diff_fresh_cpu = np.abs(fresh_np - ort_out).max()
print(f"Max diff (fresh CPU vs ORT):        {diff_fresh_cpu:.6f}")
print(f"Matches ORT:                        {'YES OK' if diff_fresh_cpu < 1e-2 else 'NO FAIL <- bug is in coremltools conversion'}")
