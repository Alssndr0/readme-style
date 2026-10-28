"""The gallery in examples/ doubles as snapshot tests. Regenerate with scripts/build_examples.py."""

from pathlib import Path

import pytest

from readme_style import THEMES, get_theme, style_file, style_text
from readme_style.engine import read_text

EXAMPLES = Path(__file__).resolve().parent.parent / "examples"
CASES = [(d, t) for d in sorted(EXAMPLES.iterdir()) if (d / "README.md").is_file() for t in THEMES]


@pytest.mark.parametrize(("directory", "theme"), CASES, ids=[f"{d.name}-{t}" for d, t in CASES])
def test_example_snapshot(directory: Path, theme: str):
    expected = read_text(directory / f"README.{theme}.md")
    result = style_file(directory / "README.md", theme=theme)
    assert result.output == expected, "run scripts/build_examples.py and review the diff"
    assert style_text(expected, get_theme(theme), result.project).output == expected
