
"""
Find and fix all Reshape nodes that hardcode batch=1 in the recognition ONNX model.
The SVTR attention mechanism does: Reshape([1, 8, 40, 15]) which must become
Reshape([-1, 8, 40, 15]) to support dynamic batch.

Strategy:
1. Identify all constant initializers used as Reshape "shape" inputs
2. For any that start with "1" followed by non-batch spatial dims, replace with "-1"
3. Save fixed model and verify with ORT
"""
import numpy as np
import onnx
import onnxruntime as ort
from onnx import TensorProto, numpy_helper

SRC  = "../spike/models/en_pp-ocrv5_mobile_rec.onnx"
DST  = "../spike/models/en_pp-ocrv5_mobile_rec_batchdyn.onnx"

model = onnx.load(SRC)
graph = model.graph

init_by_name = {init.name: init for init in graph.initializer}

fixes = 0
for node in graph.node:
    if node.op_type != "Reshape":
        continue
    if len(node.input) < 2:
        continue
    shape_name = node.input[1]
    if shape_name not in init_by_name:
        continue
    init = init_by_name[shape_name]
    shape_arr = numpy_helper.to_array(init)
    if shape_arr.ndim != 1 or shape_arr[0] != 1:
        continue

    old_shape = list(shape_arr)
    new_shape = [-1] + list(shape_arr[1:])
    print(f"Reshape fix: {old_shape} -> {new_shape}  (node output: {node.output[0]})")

    new_arr = np.array(new_shape, dtype=np.int64)
    new_init = numpy_helper.from_array(new_arr, name=shape_name + "_batchdyn")
    graph.initializer.append(new_init)
    node.input[1] = new_init.name
    fixes += 1

print(f"\nFixed {fixes} Reshape node(s)")

onnx.save(model, DST)
print(f"Saved to {DST}")

sess_orig = ort.InferenceSession(SRC, providers=["CPUExecutionProvider"])
sess_new  = ort.InferenceSession(DST, providers=["CPUExecutionProvider"])

np.random.seed(42)
for N in [1, 2, 4, 8, 22]:
    inp = np.random.randn(N, 3, 48, 320).astype(np.float32)
    try:
        out_new  = sess_new.run(None, {"x": inp})[0]
        outs_orig = np.concatenate([
            sess_orig.run(None, {"x": inp[i:i+1]})[0] for i in range(N)
        ], axis=0)
        diff = np.abs(out_new - outs_orig).max()
        print(f"batch={N:2d}  ORT diff new vs orig: {diff:.2e}  {'OK OK' if diff < 0.01 else 'FAIL FAIL'}")
    except Exception as e:
        print(f"batch={N:2d}  EXCEPTION: {e}")
