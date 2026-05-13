"""
farscry - Image interpreter for automation workflows.

Thin Python wrapper around the farscry native binary.
All heavy lifting (OCR, CoreML, ONNX Runtime) happens in the binary.

Usage::

    from farscry import extract, diff, extract_batch

    vasp   = extract('screenshot.png')
    vasp   = extract(image_bytes)
    delta  = diff('before.png', 'after.png')
    results = extract_batch(['a.png', 'b.png'])
"""

from __future__ import annotations

import os
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import List, Optional, Union

__version__ = "0.1.0"
__all__ = ["extract", "diff", "extract_batch", "FarscryError"]


class FarscryError(RuntimeError):
    """Raised when the farscry binary exits with a non-zero status."""

    def __init__(self, message: str, exit_code: int = 1) -> None:
        super().__init__(message)
        self.exit_code = exit_code


def _binary() -> Path:
    """Return the path to the farscry binary bundled with this package."""
    pkg_bin = Path(__file__).parent / "bin"
    for name in ("farscry", "farscry.exe"):
        candidate = pkg_bin / name
        if candidate.exists():
            return candidate

    import shutil
    found = shutil.which("farscry")
    if found:
        return Path(found)

    raise FarscryError(
        "farscry binary not found.\n"
        "Re-install with: pip install --force-reinstall farscry\n"
        "Or install via: brew install teles-forge/farscry/farscry"
    )


def _run(*args: str, input_data: Optional[bytes] = None) -> str:
    """Run the farscry binary with *args; return stdout as str."""
    cmd = [str(_binary()), *args]
    try:
        result = subprocess.run(
            cmd,
            input=input_data,
            capture_output=True,
            check=False,
        )
    except FileNotFoundError as exc:
        raise FarscryError(f"farscry binary not executable: {exc}") from exc

    if result.returncode != 0:
        stderr = result.stderr.decode("utf-8", errors="replace").strip()
        raise FarscryError(
            f"farscry exited with code {result.returncode}: {stderr}",
            exit_code=result.returncode,
        )

    return result.stdout.decode("utf-8", errors="replace")


def extract(
    image: Union[str, "os.PathLike[str]", bytes],
    *,
    lang: str = "eng",
    json: bool = False,
    affordances: bool = False,
    context: bool = False,
) -> str:
    """
    Extract VASP context from an image.

    Parameters
    ----------
    image:
        File path (str / Path) **or** raw image bytes (PNG/JPG/WebP).
    lang:
        BCP-47 language code(s), e.g. ``"eng"`` or ``"eng+por"``.
    json:
        Return JSON instead of VASP text format.
    affordances:
        Include the affordances section in the output.
    context:
        Return only the ``agent_context`` line.

    Returns
    -------
    str
        VASP text (or JSON) output.
    """
    flags: List[str] = ["extract"]
    if lang != "eng":
        flags += ["--lang", lang]
    if json:
        flags.append("--json")
    if affordances:
        flags.append("--affordances")
    if context:
        flags.append("--context")

    if isinstance(image, (str, os.PathLike)):
        flags.append(str(image))
        return _run(*flags)
    else:
        with tempfile.NamedTemporaryFile(suffix=".png", delete=False) as tmp:
            tmp.write(image)
            tmp_path = tmp.name
        try:
            flags.append(tmp_path)
            return _run(*flags)
        finally:
            os.unlink(tmp_path)


def diff(
    before: Union[str, "os.PathLike[str]"],
    after: Union[str, "os.PathLike[str]"],
    *,
    json: bool = False,
) -> str:
    """
    Compute the VASP semantic delta between two screenshots.

    Parameters
    ----------
    before, after:
        Paths to the two images.
    json:
        Return JSON delta instead of VASP diff text.

    Returns
    -------
    str
        VASP diff output (or JSON).
    """
    flags: List[str] = ["diff", str(before), str(after)]
    if json:
        flags.append("--json")
    return _run(*flags)


def extract_batch(
    images: List[Union[str, "os.PathLike[str]"]],
    *,
    lang: str = "eng",
    json: bool = False,
) -> List[str]:
    """
    Extract VASP context from multiple images in parallel.

    Parameters
    ----------
    images:
        List of image file paths.
    lang:
        BCP-47 language code(s).
    json:
        Return JSON per image.

    Returns
    -------
    list[str]
        One VASP string per image, in the same order.
    """
    if not images:
        return []

    flags: List[str] = ["extract", *[str(p) for p in images]]
    if lang != "eng":
        flags += ["--lang", lang]
    if json:
        flags.append("--json")

    output = _run(*flags)

    parts = output.split("---\n")
    return [p.strip() for p in parts if p.strip()]
