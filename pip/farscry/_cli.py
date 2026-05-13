"""
farscry._cli - Entry point for `farscry` console script.

Delegates directly to the native binary so the CLI experience is identical
whether you installed via npm, pip, brew, or curl.
"""
from __future__ import annotations

import os
import sys

from farscry import _binary


def main() -> None:
    binary = _binary()
    os.execv(str(binary), [str(binary)] + sys.argv[1:])


if __name__ == "__main__":
    main()
