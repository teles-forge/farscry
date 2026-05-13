
"""
Convert recognition model with FIXED batch=32.

Technique:
  - jit.trace with batch=32 (bakes 32 into Reshape nodes)
  - CoreML compiled with shape=(32, 3, 48, 320)
  - At inference: pad N crops to 32, single CoreML call, take [:N] outputs
  - Expected speedup: 22x6ms -> ~15ms

Correctness verified against ORT batch-1 baseline.
"""
import os, time, subprocess, shutil
import numpy as np
import onnx
import onnx2torch
import torch
import coremltools as ct
import onnxruntime as ort

ONNX      = "../spike/models/en_pp-ocrv5_mobile_rec.onnx"
OUT_PKG   = "models/en_pp-ocrv5_mobile_rec_b32.mlpackage"
OUT_MC    = "models/en_pp-ocrv5_mobile_rec_b32.mlmodelc"
BATCH     = 32

print(f"=== Converting recognition model (fixed batch={BATCH}) ===")

print("Loading ONNX -> onnx2torch...")
onnx_model  = onnx.load(ONNX)
torch_model = onnx2torch.convert(onnx_model).eval()

print(f"Tracing with batch={BATCH}...")
example     = torch.zeros(BATCH, 3, 48, 320)
traced      = torch.jit.trace(torch_model, example)

print("Converting to CoreML MLProgram...")
t0 = time.perf_counter()
inputs = [ct.TensorType(name="x", shape=(BATCH, 3, 48, 320), dtype=np.float32)]
cml = ct.convert(
    traced, inputs=inputs, convert_to="mlprogram",
    minimum_deployment_target=ct.target.macOS13,
    compute_precision=ct.precision.FLOAT32,
)
print(f"  Conversion: {(time.perf_counter()-t0)*1000:.0f}ms")

cml.save(OUT_PKG)
print(f"  Saved: {OUT_PKG}")

print("\n=== Correctness check (batch=32 CoreML vs NxORT-batch-1) ===")
sess      = ort.InferenceSession(ONNX, providers=["CPUExecutionProvider"])
cpu_model = ct.models.MLModel(OUT_PKG, compute_units=ct.ComputeUnit.CPU_ONLY)
all_model = ct.models.MLModel(OUT_PKG, compute_units=ct.ComputeUnit.ALL)

for N in [1, 4, 8, 22, 32]:
    np.random.seed(N)
    crops = np.random.randn(N, 3, 48, 320).astype(np.float32)
    pad   = np.zeros((BATCH - N, 3, 48, 320), dtype=np.float32)
    inp   = np.concatenate([crops, pad], axis=0)

    ort_stack = np.concatenate([
        sess.run(None, {"x": crops[i:i+1]})[0] for i in range(N)
    ], axis=0)

    cpu_raw = cpu_model.predict({"x": inp})
    all_raw = all_model.predict({"x": inp})
    key     = list(cpu_raw.keys())[0]

    cpu_out = cpu_raw[key][:N]
    all_out = all_raw[key][:N]

    diff_cpu = np.abs(cpu_out - ort_stack).max()
    diff_all = np.abs(all_out - ort_stack).max()
    print(f"  N={N:2d}  CPU diff={diff_cpu:.2e} {'OK' if diff_cpu<0.01 else 'FAIL'}  "
          f"ALL diff={diff_all:.2e} {'OK' if diff_all<0.01 else 'FAIL'}")

print("\n=== Timing: 22 sequential vs 1 batch=32 call ===")
N  = 22
np.random.seed(99)
crops = np.random.randn(N, 3, 48, 320).astype(np.float32)
pad   = np.zeros((BATCH - N, 3, 48, 320), dtype=np.float32)
inp   = np.concatenate([crops, pad], axis=0)

b1_pkg = "models/en_pp-ocrv5_mobile_rec.mlpackage"
if os.path.exists(b1_pkg):
    b1_model = ct.models.MLModel(b1_pkg, compute_units=ct.ComputeUnit.ALL)
    for _ in range(3):
        for i in range(N):
            b1_model.predict({"x": crops[i:i+1]})
    RUNS = 5
    times = []
    for _ in range(RUNS):
        t = time.perf_counter()
        for i in range(N):
            b1_model.predict({"x": crops[i:i+1]})
        times.append((time.perf_counter()-t)*1000)
    print(f"  Sequential (22xb1): {min(times):.1f}ms min / {sum(times)/len(times):.1f}ms avg")

for _ in range(3):
    all_model.predict({"x": inp})
times = []
for _ in range(RUNS):
    t = time.perf_counter()
    all_model.predict({"x": inp})
    times.append((time.perf_counter()-t)*1000)
print(f"  Batch=32 (1 call):  {min(times):.1f}ms min / {sum(times)/len(times):.1f}ms avg")

print(f"\n=== Compiling {OUT_PKG} -> {OUT_MC} ===")
if os.path.exists(OUT_MC):
    shutil.rmtree(OUT_MC)
result = subprocess.run(
    ["xcrun", "-sdk", "macosx", "coremlc", "compile", OUT_PKG, "models/"],
    capture_output=True, text=True
)
if result.returncode == 0:
    print(f"  Compiled: {OUT_MC}")
else:
    print(f"  xcrun error: {result.stderr}")

print("\n=== Done ===")
print(f"  Model: {OUT_MC}")
print(f"  Input shape: [{BATCH}, 3, 48, 320]  (pad N crops to {BATCH}, use output[:N])")
