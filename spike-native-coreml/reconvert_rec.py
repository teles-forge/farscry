
"""
Re-convert the recognition ONNX model to CoreML via onnx2torch.
Saves mlpackage, then compiles to mlmodelc.
"""

import os
import time
import onnx
import onnx2torch
import torch
import numpy as np
import coremltools as ct
import onnxruntime as ort

ONNX_PATH   = "../spike/models/en_pp-ocrv5_mobile_rec.onnx"
MLB_PATH    = "models/en_pp-ocrv5_mobile_rec.mlpackage"
MLMC_PATH   = "models/en_pp-ocrv5_mobile_rec.mlmodelc"
DICT_PATH   = "../spike/models/ppocrv5_en_dict.txt"
IMG_PATH    = "../spike/test.png"

INPUT_SHAPE = (1, 3, 48, 320)

print("=== 1. Converting ONNX -> PyTorch -> CoreML ===")
t0 = time.perf_counter()
onnx_model  = onnx.load(ONNX_PATH)
torch_model = onnx2torch.convert(onnx_model).eval()
traced      = torch.jit.trace(torch_model, torch.zeros(*INPUT_SHAPE))
inputs      = [ct.TensorType(name="x", shape=INPUT_SHAPE, dtype=np.float32)]
cml         = ct.convert(
    traced, inputs=inputs, convert_to="mlprogram",
    minimum_deployment_target=ct.target.macOS13,
    compute_precision=ct.precision.FLOAT32,
)
cml.save(MLB_PATH)
print(f"  Saved {MLB_PATH}  ({(time.perf_counter()-t0)*1000:.0f}ms)")

print("\n=== 2. Verifying mlpackage vs ORT (random input) ===")
sess       = ort.InferenceSession(ONNX_PATH, providers=["CPUExecutionProvider"])
cpu_model  = ct.models.MLModel(MLB_PATH, compute_units=ct.ComputeUnit.CPU_ONLY)
all_model  = ct.models.MLModel(MLB_PATH, compute_units=ct.ComputeUnit.ALL)

np.random.seed(42)
rand_in = np.random.randn(*INPUT_SHAPE).astype(np.float32)

ort_out  = sess.run(None, {"x": rand_in})[0]
cpu_out  = cpu_model.predict({"x": rand_in})
all_out  = all_model.predict({"x": rand_in})
cpu_key  = list(cpu_out.keys())[0]
all_key  = list(all_out.keys())[0]

diff_cpu = np.abs(ort_out - cpu_out[cpu_key]).max()
diff_all = np.abs(ort_out - all_out[all_key]).max()
print(f"  Max diff ORT vs CPU: {diff_cpu:.2e}  {'OK OK' if diff_cpu < 0.01 else 'FAIL FAIL'}")
print(f"  Max diff ORT vs ALL: {diff_all:.2e}  {'OK OK' if diff_all < 0.01 else 'FAIL FAIL'}")

print("\n=== 3. Compiling mlpackage -> mlmodelc ===")
import subprocess, shutil

if os.path.exists(MLMC_PATH):
    shutil.rmtree(MLMC_PATH)

try:
    compiled = ct.models.CompiledMLModel.from_model_path(MLB_PATH)
    print(f"  Compiled model available in-memory (not writing .mlmodelc directly)")
except Exception as e:
    print(f"  CompiledMLModel failed: {e}")

result = subprocess.run(
    ["xcrun", "-sdk", "macosx", "coremlc", "compile", MLB_PATH, "models/"],
    capture_output=True, text=True
)
if result.returncode == 0:
    print(f"  xcrun coremlc succeeded: {result.stdout.strip()}")
    print(f"  mlmodelc at: {MLMC_PATH}")
else:
    print(f"  xcrun coremlc stderr: {result.stderr.strip()}")
    print("  Trying Python API compilation...")
    spec = cml.get_spec()
    ct.utils.compile_model(spec, MLMC_PATH)
    print(f"  Compiled to {MLMC_PATH}")

print(f"\nDone. Models at:")
print(f"  {MLB_PATH}")
print(f"  {MLMC_PATH}")
