
"""
Compare ONNX vs onnx2torch outputs on same input.
If they match -> bug is in coremltools PyTorch conversion.
If they differ -> onnx2torch is corrupting weights/ops.
"""

import onnx
import onnx2torch
import torch
import numpy as np
import onnxruntime as ort

onnx_path = "../spike/models/en_pp-ocrv5_mobile_rec.onnx"

print("=== Loading ONNX model (ORT baseline) ===")
sess = ort.InferenceSession(onnx_path, providers=["CPUExecutionProvider"])

np.random.seed(42)
test_input = np.random.randn(1, 3, 48, 320).astype(np.float32)

ort_out = sess.run(None, {"x": test_input})[0]
print(f"ORT output shape:            {ort_out.shape}")
print(f"ORT t=0 top-5 logits:        {ort_out[0,0,:5]}")
print(f"ORT t=0 argmax:              {np.argmax(ort_out[0,0,:])}")
print(f"ORT t=0 max value:           {ort_out[0,0,:].max():.6f}")

print("\n=== Converting ONNX -> PyTorch via onnx2torch ===")
onnx_model = onnx.load(onnx_path)
torch_model = onnx2torch.convert(onnx_model).eval()

with torch.no_grad():
    torch_out = torch_model(torch.from_numpy(test_input))

torch_out_np = torch_out.numpy() if isinstance(torch_out, torch.Tensor) else torch_out[0].numpy()
print(f"onnx2torch output shape:     {torch_out_np.shape}")
print(f"onnx2torch t=0 top-5 logits: {torch_out_np[0,0,:5]}")
print(f"onnx2torch t=0 argmax:       {np.argmax(torch_out_np[0,0,:])}")
print(f"onnx2torch t=0 max value:    {torch_out_np[0,0,:].max():.6f}")

print("\n=== Comparison ===")
diff = np.abs(ort_out - torch_out_np).max()
mean_diff = np.abs(ort_out - torch_out_np).mean()
print(f"Max absolute diff:   {diff:.8f}")
print(f"Mean absolute diff:  {mean_diff:.8f}")
print(f"Outputs match:       {'YES OK' if diff < 1e-3 else 'NO FAIL'}")

if diff >= 1e-3:
    print("\n>>> ROOT CAUSE: onnx2torch is corrupting the model <<<")
    print(">>> The bug lives in the ONNX->PyTorch step, NOT in coremltools <<<")

    print("\n=== Identifying diverging layers ===")
    for node in onnx_model.graph.node:
        print(f"  {node.op_type:20s} inputs={list(node.input)[:2]} outputs={list(node.output)[:1]}")

else:
    print("\n>>> onnx2torch output matches ORT - bug is in coremltools conversion <<<")
    print(">>> Need to investigate coremltools PyTorch->CoreML step <<<")
