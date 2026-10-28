from __future__ import annotations

from pathlib import Path

import pytest

from readme_style import get_theme, style_text
from readme_style.project import Project, load_project


def write_tree(root: Path, files: dict[str, str]) -> None:
    (root / ".github").mkdir(exist_ok=True)  # marks the repository root
    for name, content in files.items():
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")


@pytest.fixture
def tree(tmp_path: Path):
    def make(files: dict[str, str]) -> Path:
        write_tree(tmp_path, files)
        return tmp_path / "README.md"

    return make


def bare_project(tmp_path: Path, **facts) -> Project:
    return Project(readme_dir=tmp_path, root=tmp_path, **facts)


def run(text: str, theme: str = "minimal", project: Project | None = None, tmp_path: Path | None = None) -> str:
    project = project or bare_project(tmp_path or Path("/nonexistent"))
    return style_text(text, get_theme(theme), project).output


@pytest.fixture
def styled(tree):
    """Style a README inside a throwaway project made of `files`."""

    def make(readme: str, theme: str = "minimal", files: dict[str, str] | None = None) -> str:
        path = tree({"README.md": readme, **(files or {})})
        return style_text(readme, get_theme(theme), load_project(path)).output

    return make
