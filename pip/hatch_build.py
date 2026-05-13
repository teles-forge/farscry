"""
hatch_build.py - Hatchling build hook for farscry pip package.

Downloads the correct pre-built native binary from GitHub Releases during
`pip install farscry`.  Verifies SHA256 before saving.  Fails loudly on
any checksum mismatch.

This file is called by hatchling at wheel-build time (both locally and on
the user's machine during `pip install --wheel`).  For `pip install` from
source (sdist), it runs on the target machine so the binary is fetched for
the correct platform.
"""

from __future__ import annotations

import hashlib
import os
import platform
import shutil
import stat
import sys
import tarfile
import tempfile
import urllib.request
import zipfile
from pathlib import Path
from typing import Any, Dict, Optional

from hatchling.builders.hooks.plugin.interface import BuildHookInterface

VERSION = "0.1.0"
REPO    = "teles-forge/farscry"
BASE_URL = f"https://github.com/{REPO}/releases/download/v{VERSION}"


def _asset_name() -> str:
    """Map current platform to the release asset name."""
    system = platform.system().lower()
    machine = platform.machine().lower()

    arch_map = {
        "x86_64": "x86_64",
        "amd64":  "x86_64",
        "arm64":  "aarch64",
        "aarch64":"aarch64",
    }
    arch = arch_map.get(machine)
    if arch is None:
        raise RuntimeError(
            f"Unsupported architecture: {machine}\n"
            f"farscry supports x86_64 and arm64. "
            f"Open an issue: https://github.com/{REPO}/issues"
        )

    if system == "darwin":
        return f"farscry-{arch}-apple-darwin"
    if system == "linux":
        if arch != "x86_64":
            raise RuntimeError(
                f"farscry only ships linux x86_64 binaries in v{VERSION}. "
                f"arm64 Linux support is planned for v0.2."
            )
        return f"farscry-{arch}-unknown-linux-gnu"
    if system == "windows":
        if arch != "x86_64":
            raise RuntimeError("farscry only ships Windows x86_64 binaries.")
        return "farscry-x86_64-pc-windows-msvc"

    raise RuntimeError(
        f"Unsupported OS: {system}. "
        f"farscry supports macOS, Linux, and Windows. "
        f"Open an issue: https://github.com/{REPO}/issues"
    )


def _download(url: str, dest: Path) -> None:
    """Download `url` to `dest`, following redirects."""
    print(f"[farscry] Downloading {url}", flush=True)
    req = urllib.request.Request(
        url, headers={"User-Agent": f"farscry-pip/{VERSION}"}
    )
    with urllib.request.urlopen(req) as resp, open(dest, "wb") as f:
        shutil.copyfileobj(resp, f)


def _sha256(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def _extract(archive: Path, dest: Path, is_windows: bool) -> None:
    if is_windows:
        with zipfile.ZipFile(archive) as zf:
            zf.extractall(dest)
    else:
        with tarfile.open(archive, "r:gz") as tf:
            tf.extractall(dest)


class FarscryBuildHook(BuildHookInterface):
    """Hatchling hook: download native binary at wheel-build time."""

    PLUGIN_NAME = "farscry"

    def initialize(self, version: str, build_data: Dict[str, Any]) -> None:
        if getattr(self, "target_name", None) == "sdist":
            return

        is_windows = platform.system().lower() == "windows"
        binary_name = "farscry.exe" if is_windows else "farscry"

        try:
            asset_name   = _asset_name()
        except RuntimeError as e:
            print(f"[farscry] WARNING: {e}", file=sys.stderr)
            print("[farscry] Skipping binary download.", file=sys.stderr)
            return

        archive_ext  = "zip" if is_windows else "tar.gz"
        archive_url  = f"{BASE_URL}/{asset_name}.{archive_ext}"
        sha256_url   = f"{BASE_URL}/{asset_name}.sha256"

        bin_dir = Path(__file__).parent / "bin"
        bin_dir.mkdir(exist_ok=True)
        binary_dest = bin_dir / binary_name

        if binary_dest.exists():
            print(f"[farscry] Binary already present at {binary_dest}", flush=True)
            self._register_artifact(build_data, bin_dir, binary_name, is_windows)
            return

        with tempfile.TemporaryDirectory() as tmp:
            tmp_path     = Path(tmp)
            archive_path = tmp_path / f"{asset_name}.{archive_ext}"
            sha256_path  = tmp_path / f"{asset_name}.sha256"

            _download(archive_url, archive_path)
            _download(sha256_url,  sha256_path)

            expected = sha256_path.read_text().strip().split()[0].lower()

            _extract(archive_path, tmp_path, is_windows)
            nested = tmp_path / asset_name
            extracted_binary = nested / binary_name

            actual = _sha256(extracted_binary).lower()
            if actual != expected:
                raise RuntimeError(
                    f"[farscry] SHA256 MISMATCH - aborting.\n"
                    f"  expected : {expected}\n"
                    f"  actual   : {actual}\n"
                    f"This may indicate a corrupted download or a supply-chain issue.\n"
                    f"Please retry. If the problem persists, open an issue at "
                    f"https://github.com/{REPO}/issues"
                )

            print(f"[farscry] SHA256 verified OK", flush=True)

            shutil.copy2(extracted_binary, binary_dest)
            if not is_windows:
                binary_dest.chmod(binary_dest.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)

            for entry in nested.iterdir():
                if entry.name.startswith(("libonnxruntime", "onnxruntime")):
                    ort_dest = bin_dir / entry.name
                    shutil.copy2(entry, ort_dest)
                    if not is_windows:
                        ort_dest.chmod(ort_dest.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP)
                    print(f"[farscry] Bundled ORT: {entry.name}", flush=True)

        print(f"[farscry] Binary installed: {binary_dest}", flush=True)
        self._register_artifact(build_data, bin_dir, binary_name, is_windows)

    def _register_artifact(
        self,
        build_data: Dict[str, Any],
        bin_dir: Path,
        binary_name: str,
        is_windows: bool,
    ) -> None:
        """Tell hatchling to include the binary and ORT libs in the wheel."""
        for entry in bin_dir.iterdir():
            if entry.name == ".gitkeep":
                continue
            build_data.setdefault("artifacts", []).append(str(entry))
            build_data.setdefault("force_include", {})[str(entry)] = (
                f"farscry/bin/{entry.name}"
            )
