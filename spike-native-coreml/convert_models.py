
"""
Step 1 of native CoreML spike.

Converts PP-OCRv5 ONNX models to .mlpackage (MLProgram format),
then measures raw CoreML inference time - BEFORE writing any Rust.

If this script shows <50ms combined, native CoreML in Rust is worth implementing.
If it shows >181ms, save the effort.

Usage:
    uv run spike-native-coreml/convert_models.py
"""

import time
import os
import sys
import numpy as np
import torch
import coremltools as ct
from transformers import AutoModelForTextRecognition, AutoImageProcessor

MODELS_DIR = os.path.join(os.path.dirname(__file__), "../spike/models")
OUTPUT_DIR = os.path.join(os.path.dirname(__file__), "models")
RUNS = 6

DET_ONNX = os.path.join(MODELS_DIR, "pp-ocrv5_mobile_det.onnx")
REC_HF_MODEL = "PaddlePaddle/PP-OCRv5_mobile_rec_safetensors"
DET_MLB  = os.path.join(OUTPUT_DIR, "pp-ocrv5_mobile_det.mlpackage")
REC_MLB  = os.path.join(OUTPUT_DIR, "en_pp-ocrv5_mobile_rec.mlpackage")

DET_SHAPE = (1, 3, 960, 960)
REC_SHAPE = (1, 3, 48, 320)


def convert_onnx(onnx_path: str, mlpkg_path: str, input_shape: tuple, name: str) -> ct.models.MLModel:
    print(f"\n[convert_onnx] {name}")
    print(f"  onnx  : {onnx_path}")
    print(f"  output: {mlpkg_path}")
    print(f"  shape : {input_shape}")

    if os.path.exists(mlpkg_path):
        print("  (cached - skipping conversion)")
        t = time.perf_counter()
        model = ct.models.MLModel(mlpkg_path, compute_units=ct.ComputeUnit.ALL)
        load_ms = (time.perf_counter() - t) * 1000
        print(f"  model load: {load_ms:.1f}ms")
        return model

    import onnx
    import onnx2torch

    t = time.perf_counter()
    onnx_model = onnx.load(onnx_path)
    torch_model = onnx2torch.convert(onnx_model).eval()

    example_input = torch.zeros(*input_shape, dtype=torch.float32)
    traced = torch.jit.trace(torch_model, example_input)

    inputs = [ct.TensorType(name="x", shape=input_shape, dtype=np.float32)]
    model = ct.convert(
        traced,
        inputs=inputs,
        convert_to="mlprogram",
        minimum_deployment_target=ct.target.macOS13,
        compute_precision=ct.precision.FLOAT32,
    )
    convert_ms = (time.perf_counter() - t) * 1000
    print(f"  conversion: {convert_ms:.0f}ms")

    model.save(mlpkg_path)
    print(f"  saved to {mlpkg_path}")

    t = time.perf_counter()
    model = ct.models.MLModel(mlpkg_path, compute_units=ct.ComputeUnit.ALL)
    load_ms = (time.perf_counter() - t) * 1000
    print(f"  model load (ComputeUnit.ALL): {load_ms:.1f}ms")

    try:
        print(f"  output names: {list(model.output_description.keys())}")
    except AttributeError:
        print(f"  output names: {list(model.output_description)}")

    return model


def convert_pytorch(hf_model_name: str, mlpkg_path: str, input_shape: tuple, name: str) -> ct.models.MLModel:
    print(f"\n[convert_pytorch] {name}")
    print(f"  hf_model : {hf_model_name}")
    print(f"  output: {mlpkg_path}")
    print(f"  shape : {input_shape}")

    if os.path.exists(mlpkg_path):
        print("  (cached - skipping conversion)")
        t = time.perf_counter()
        model = ct.models.MLModel(mlpkg_path, compute_units=ct.ComputeUnit.ALL)
        load_ms = (time.perf_counter() - t) * 1000
        print(f"  model load: {load_ms:.1f}ms")
        return model

    t = time.perf_counter()
    print(f"  Loading model from Hugging Face...")
    torch_model = AutoModelForTextRecognition.from_pretrained(hf_model_name)
    torch_model.eval()

    image_processor = AutoImageProcessor.from_pretrained(hf_model_name)
    print(f"  Image processor: {image_processor}")

    example_input = torch.zeros(*input_shape, dtype=torch.float32)
    traced = torch.jit.trace(torch_model, example_input, strict=False)

    inputs = [ct.TensorType(name="x", shape=input_shape, dtype=np.float32)]
    model = ct.convert(
        traced,
        inputs=inputs,
        convert_to="mlprogram",
        minimum_deployment_target=ct.target.macOS13,
        compute_precision=ct.precision.FLOAT32,
    )
    convert_ms = (time.perf_counter() - t) * 1000
    print(f"  conversion: {convert_ms:.0f}ms")

    model.save(mlpkg_path)
    print(f"  saved to {mlpkg_path}")

    t = time.perf_counter()
    model = ct.models.MLModel(mlpkg_path, compute_units=ct.ComputeUnit.ALL)
    load_ms = (time.perf_counter() - t) * 1000
    print(f"  model load (ComputeUnit.ALL): {load_ms:.1f}ms")

    try:
        print(f"  output names: {list(model.output_description.keys())}")
    except AttributeError:
        print(f"  output names: {list(model.output_description)}")

    return model


def bench(model: ct.models.MLModel, dummy_input: np.ndarray, name: str):
    print(f"\n[bench] {name} - {RUNS} runs")
    inp = {"x": dummy_input}
    times = []
    for i in range(1, RUNS + 1):
        t = time.perf_counter()
        _ = model.predict(inp)
        ms = (time.perf_counter() - t) * 1000
        times.append(ms)
        tag = " <- cold" if i == 1 else (" <- steady" if i == RUNS else "")
        print(f"  run {i:02d}: {ms:.1f}ms{tag}")

    steady = times[-1]
    min_t  = min(times)
    print(f"  ── steady: {steady:.1f}ms  min: {min_t:.1f}ms")
    print(f"  ── 181ms goal: {'PASS OK' if steady < 181 else 'FAIL FAIL'}")
    print(f"  ── 100ms goal: {'PASS OK' if steady < 100 else 'FAIL FAIL'}")
    print(f"  ── 30ms goal : {'PASS OK' if steady < 30  else 'FAIL FAIL'}")
    return steady


def main():
    print("=== farscry - native CoreML spike (Python timing) ===")
    print(f"coremltools: {ct.__version__}")
    print(f"output dir : {OUTPUT_DIR}")

    os.makedirs(OUTPUT_DIR, exist_ok=True)

    det_model = convert_onnx(DET_ONNX, DET_MLB, DET_SHAPE, "PP-OCRv5 detection (DBNet++)")
    rec_model = convert_pytorch(REC_HF_MODEL, REC_MLB, REC_SHAPE, "PP-OCRv5 recognition (SVTR-LCNet)")

    det_input = np.zeros(DET_SHAPE, dtype=np.float32)
    rec_input = np.zeros(REC_SHAPE, dtype=np.float32)

    det_steady = bench(det_model, det_input, "DET (960x960)")
    rec_steady = bench(rec_model, rec_input, "REC (48x320)")

    print(f"\n[bench] COMBINED pipeline (det + 10x rec) - {RUNS} runs")
    combined_times = []
    for i in range(1, RUNS + 1):
        t = time.perf_counter()
        _ = det_model.predict({"x": det_input})
        for _ in range(10):
            _ = rec_model.predict({"x": rec_input})
        ms = (time.perf_counter() - t) * 1000
        combined_times.append(ms)
        tag = " <- cold" if i == 1 else (" <- steady" if i == RUNS else "")
        print(f"  run {i:02d}: {ms:.1f}ms{tag}")

    combined_steady = combined_times[-1]

    print("\n========================================")
    print("SPIKE SUMMARY (native CoreML - Python)")
    print("========================================")
    print(f"  det  steady-state    : {det_steady:.1f}ms")
    print(f"  rec  steady-state    : {rec_steady:.1f}ms")
    print(f"  combined (det+10 rec): {combined_steady:.1f}ms")
    print(f"  ── 181ms goal: {'PASS OK' if combined_steady < 181 else 'FAIL FAIL'}")
    print(f"  ── 100ms goal: {'PASS OK' if combined_steady < 100 else 'FAIL FAIL'}")
    print(f"  ── 30ms goal : {'PASS OK' if combined_steady < 30  else 'FAIL FAIL'}")
    print("========================================")

    if combined_steady < 100:
        print("\n-> VERDICT: GO - native CoreML is fast enough. Proceed with Rust objc2-core-ml spike.")
    elif combined_steady < 181:
        print("\n-> VERDICT: MARGINAL - hits 181ms but not 100ms. Evaluate if ORT CoreML EP is already sufficient.")
    else:
        print("\n-> VERDICT: NO-GO - native CoreML does not meet 181ms. Likely op coverage issue or ANE not engaging.")


if __name__ == "__main__":
    main()
