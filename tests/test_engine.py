from pathlib import Path

import pytest

from readme_style import THEMES, get_theme, style_file, style_text
from readme_style.engine import recorded_theme
from readme_style.project import Logo

from .conftest import bare_project

README = (
    """# quickparse

A tiny parser for config files.

## Features

- Fast

## Install

```bash
pip install quickparse
```

> **Note:** needs Python 3.10.

## Usage

Use it.

## Changelog

"""
    + "\n".join(f"- change {i}" for i in range(15))
    + """

## License

MIT
"""
)


@pytest.fixture
def project(tmp_path):
    return bare_project(
        tmp_path,
        name="quickparse",
        description="A tiny parser.",
        license="MIT",
        license_file="LICENSE",
        repo="acme/quickparse",
        logo=Logo("assets/logo.svg", "assets/logo-dark.svg"),
    )


@pytest.mark.parametrize("theme", THEMES)
def test_every_theme_is_idempotent(theme, project):
    once = style_text(README, get_theme(theme), project).output
    assert style_text(once, get_theme(theme), project).output == once


@pytest.mark.parametrize("first", THEMES)
@pytest.mark.parametrize("second", THEMES)
def test_switching_themes_equals_styling_from_scratch(first, second, project):
    direct = style_text(README, get_theme(second), project).output
    via = style_text(style_text(README, get_theme(first), project).output, get_theme(second), project).output
    assert via == direct


def test_crlf_and_bom_are_preserved(project):
    source = "\ufeff" + README.replace("\n", "\r\n")
    out = style_text(source, get_theme("startup"), project).output
    assert out.startswith("\ufeff")
    assert "\r\n" in out and "\n" not in out.replace("\r\n", "")


def test_recorded_theme_is_read_back(project):
    out = style_text(README, get_theme("startup"), project).output
    assert recorded_theme(out) == "startup"


def test_style_file_uses_config_then_markers_then_default(tree):
    path = tree({"README.md": README, ".readme-style.toml": 'theme = "ml-research"\ntoc = "none"\n'})
    result = style_file(path)
    assert result.theme.name == "ml-research" and result.theme.toc == "none"
    assert result.theme_source.endswith(".readme-style.toml")
    Path(path.parent / ".readme-style.toml").unlink()
    path.write_text(result.output)
    assert style_file(path).theme_source == "markers in README.md"
    assert style_file(path, theme="minimal").theme.name == "minimal"


def test_a_readme_without_newline_at_end(project):
    out = style_text("# T\n\ntext", get_theme("minimal"), project).output
    assert not out.endswith("\n")
