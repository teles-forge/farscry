
"""
Test whether torch.jit.trace on SVTR generalises to batch > 1.

Chain:
  ONNX -> onnx2torch (eager) -> jit.trace(batch=1) -> run(batch=N) -> compare with ORT

If max diff < 0.01 for batch=4 and batch=22, we can re-convert with
ct.RangeDim and get a single batched CoreML call for all regions at once.
"""
import numpy as np
import onnx
import onnx2torch
import torch
import onnxruntime as ort

ONNX = "../spike/models/en_pp-ocrv5_mobile_rec.onnx"

print("=== Loading ONNX -> onnx2torch ===")
onnx_model  = onnx.load(ONNX)
torch_model = onnx2torch.convert(onnx_model).eval()

print("=== Tracing with batch=1 ===")
traced = torch.jit.trace(torch_model, torch.zeros(1, 3, 48, 320))

sess = ort.InferenceSession(ONNX, providers=["CPUExecutionProvider"])

np.random.seed(0)
for N in [1, 2, 4, 8, 16, 22]:
    inp_np = np.random.randn(N, 3, 48, 320).astype(np.float32)
    inp_t  = torch.from_numpy(inp_np)

    ort_outs = []
    for i in range(N):
        out = sess.run(None, {"x": inp_np[i:i+1]})[0]
        ort_outs.append(out)
    ort_stack = np.concatenate(ort_outs, axis=0)

    try:
        with torch.no_grad():
            tr_out = traced(inp_t).numpy()
        diff = np.abs(ort_stack - tr_out).max()
        print(f"batch={N:2d}  diff={diff:.2e}  {'OK OK' if diff < 0.01 else 'FAIL FAIL'}")
    except Exception as e:
        print(f"batch={N:2d}  EXCEPTION: {e}")
