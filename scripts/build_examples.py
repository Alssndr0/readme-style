"""Regenerate examples/<name>/README.<theme>.md from each example's README.md.

    python scripts/build_examples.py

tests/test_examples.py fails when these files are out of date.
"""

from __future__ import annotations

import sys
from pathlib import Path

from readme_style import THEMES, style_file
from readme_style.engine import write_text

EXAMPLES = Path(__file__).resolve().parent.parent / "examples"


def example_dirs() -> list[Path]:
    return sorted(p for p in EXAMPLES.iterdir() if (p / "README.md").is_file())


def main() -> int:
    for directory in example_dirs():
        for theme in THEMES:
            target = directory / f"README.{theme}.md"
            write_text(target, style_file(directory / "README.md", theme=theme).output)
            print(target.relative_to(EXAMPLES.parent))
    return 0


if __name__ == "__main__":
    sys.exit(main())
