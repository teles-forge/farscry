# NOTICES

This file contains attribution notices for third-party software components
bundled with or used by farscry.

---

## ONNX Runtime

farscry uses [ONNX Runtime](https://github.com/microsoft/onnxruntime)
for cross-platform OCR inference (detection and recognition models).

**Copyright:** Copyright (c) Microsoft Corporation  
**License:** MIT License  
**Source:** https://github.com/microsoft/onnxruntime  
**License text:** https://github.com/microsoft/onnxruntime/blob/main/LICENSE

```
MIT License

Copyright (c) Microsoft Corporation

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

---

## PP-OCRv5 Models

farscry ships pre-converted CoreML and ONNX versions of
[PP-OCRv5](https://github.com/PaddlePaddle/PaddleOCR) text detection and
recognition models.

**Copyright:** Copyright (c) PaddlePaddle Authors  
**License:** Apache License 2.0  
**Source:** https://github.com/PaddlePaddle/PaddleOCR  
**License text:** https://github.com/PaddlePaddle/PaddleOCR/blob/main/LICENSE

---

## oar-ocr

farscry's cross-platform ORT backend uses
[oar-ocr](https://crates.io/crates/oar-ocr) for OCR preprocessing and
postprocessing.

**License:** MIT OR Apache-2.0 (at your option)  
**Source:** https://crates.io/crates/oar-ocr

---

## Rust crate dependencies

farscry is built with Rust and depends on various open-source crates.
A full list of dependency licenses can be generated with:

```bash
cargo install cargo-license
cargo license
```

All production dependencies use MIT, Apache-2.0, ISC, or BSD-style licenses.

---

*farscry itself is licensed under the Apache License 2.0.*
*See [LICENSE](LICENSE) for details.*
