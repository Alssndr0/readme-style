from pathlib import Path

import pytest

from readme_style.project import Logo
from readme_style.themes import get_theme

from .conftest import bare_project, run


@pytest.mark.parametrize(
    ("source", "expected"),
    [
        ("> **Note:** text", "> [!NOTE]\n> text"),
        ("> **Warning**: careful", "> [!WARNING]\n> careful"),
        ("> Tip: use it", "> [!TIP]\n> use it"),
        ("> ⚠️ **Caution:** hot", "> [!CAUTION]\n> hot"),
        ("> :warning: **Important:** read", "> [!IMPORTANT]\n> read"),
        ("**Note:** a paragraph\nover two lines", "> [!NOTE]\n> a paragraph\n> over two lines"),
        ("> **Note**\n> on the next line", "> [!NOTE]\n> on the next line"),
    ],
)
def test_callouts(source, expected):
    assert run(source + "\n") == expected + "\n"


@pytest.mark.parametrize(
    "source",
    [
        "> [!NOTE]\n> already an alert",
        "> **Note that** this is prose",
        "Note that this is prose.",
        "> **Note:**",
        "- **Note:** inside a list",
        "> > **Note:** nested",
    ],
)
def test_callouts_leave_other_text_alone(source):
    assert run(source + "\n") == source + "\n"


def test_long_changelog_is_collapsed_and_heading_kept():
    body = "\n".join(f"- change {i}" for i in range(12))
    out = run(f"# T\n\n## Changelog\n\n{body}\n\n## License\n\nMIT\n", theme="startup")
    assert (
        "## Changelog\n\n<details><summary>Click to expand</summary><!-- readme-style:collapse -->\n\n- change 0" in out
    )
    assert "- change 11\n\n</details><!-- /readme-style:collapse -->\n\n## License" in out


def test_short_changelog_is_not_collapsed():
    out = run("# T\n\n## Changelog\n\n- one\n- two\n", theme="startup")
    assert "<details>" not in out


def test_toc_lists_sections_with_anchors(tmp_path):
    readme = "# T\n\nIntro.\n\n## Install\n\n### From source\n\n## Usage\n\n## FAQ\n\n### Why?\n\n## License\n"
    out = run(readme, tmp_path=tmp_path)
    assert (
        "<!-- readme-style:toc theme=minimal -->\n**Contents**\n\n- [Install](#install)\n  - [From source](#from-source)\n"
        "- [Usage](#usage)\n- [FAQ](#faq)\n- [License](#license)\n<!-- /readme-style:toc -->\n\n## Install"
    ) in out


def test_toc_needs_enough_sections_and_respects_a_user_toc():
    assert "readme-style:toc" not in run("# T\n\n## A\n\n## B\n")
    readme = "# T\n\n## Table of Contents\n\n- [A](#a)\n\n## A\n\n## B\n\n## C\n\n## D\n"
    assert "readme-style:toc" not in run(readme)


def test_header_badges_go_after_the_title_in_minimal(tmp_path):
    project = bare_project(tmp_path, license="MIT", license_file="LICENSE")
    out = run("# T\n\nPitch.\n", project=project)
    assert out.startswith("# T\n\n<!-- readme-style:header theme=minimal -->\n[![License: MIT]")
    assert out.endswith("<!-- /readme-style:header -->\n\nPitch.\n")


def test_existing_badges_suppress_inferred_ones(tmp_path):
    project = bare_project(tmp_path, license="MIT")
    readme = "# T\n\n[![x](https://img.shields.io/badge/a-b-c)](y)\n\nPitch.\n"
    assert run(readme, project=project) == readme


def test_startup_centers_logo_title_tagline_and_header(tmp_path):
    project = bare_project(tmp_path, license="MIT", logo=Logo("logo.svg", "logo-dark.svg"), name="t")
    out = run("# T\n\nPitch.\n\n## Install\n\n## Usage\n", theme="startup", project=project)
    expected_head = (
        '<div align="center"><!-- readme-style:center -->\n\n'
        "<!-- readme-style:logo theme=startup -->\n<picture>\n"
        '  <source media="(prefers-color-scheme: dark)" srcset="logo-dark.svg">\n'
        '  <img src="logo.svg" alt="t logo" width="120">\n</picture>\n<!-- /readme-style:logo -->\n\n'
        "# T\n\nPitch.\n\n<!-- readme-style:header theme=startup -->\n"
    )
    assert out.startswith(expected_head)
    assert "**[Install](#install)** · **[Usage](#usage)**\n\n---\n<!-- /readme-style:header -->\n\n</div>" in out


def test_metadata_description_becomes_tagline_only_when_missing(tmp_path):
    project = bare_project(tmp_path, description="Fast things.")
    assert "Fast things." in run("# T\n\n## A\n\n## B\n", theme="startup", project=project)
    out = run("# T\n\nFast things, truly.\n\n## A\n", theme="startup", project=project)
    assert out.count("Fast things") == 1


def test_existing_logo_gets_a_dark_variant(tmp_path):
    project = bare_project(tmp_path, logo=Logo("assets/logo.png", "assets/logo-dark.png"))
    out = run("[![Logo](./assets/logo.png)](https://site)\n\n# T\n", project=project)
    assert out.startswith(
        '<a href="https://site">\n<picture>\n  <source media="(prefers-color-scheme: dark)" srcset="./assets/logo-dark.png">\n'
        '  <img src="./assets/logo.png" alt="Logo">\n</picture>\n</a>\n'
    )
    html = '<p align="center"><img src="assets/logo.png" width="200"></p>\n\n# T\n'
    assert '<picture><source media="(prefers-color-scheme: dark)" srcset="assets/logo-dark.png"><img' in run(
        html, project=project
    )


def test_nav_prefers_well_known_sections(tmp_path):
    names = ["Intro", "Features", "Background", "Installation", "Design", "Usage", "Internals", "Ideas", "License"]
    readme = "# T\n\n" + "".join(f"## {n}\n\n" for n in names)
    out = run(readme, theme="startup", project=bare_project(tmp_path))
    nav = next(line for line in out.splitlines() if line.startswith("**["))
    assert nav == (
        "**[Intro](#intro)** · **[Features](#features)** · **[Background](#background)** · "
        "**[Installation](#installation)** · **[Usage](#usage)** · **[License](#license)**"
    )


def test_theme_overrides_validate():
    theme = get_theme("minimal").with_overrides({"toc": "details", "badge-style": "flat"})
    assert theme.toc == "details" and theme.badge_style == "flat"
    with pytest.raises(ValueError):
        get_theme("minimal").with_overrides({"toc": "sidebar"})
    with pytest.raises(ValueError):
        get_theme("minimal").with_overrides({"colour": "red"})
    with pytest.raises(ValueError):
        get_theme("minimal").with_overrides({"nav": "yes"})


def test_no_title_readme_keeps_its_hero_first(tmp_path):
    project = bare_project(tmp_path, license="MIT")
    readme = '<p align="center"><img src="logo.png"></p>\n<h3 align="center">Fast.</h3>\n\nIntro text.\n\n## A\n'
    out = run(readme, theme="startup", project=project)
    assert out.index("Fast.") < out.index("readme-style:header")
    assert out.index("readme-style:header") < out.index("Intro text.")


def test_paths_are_posix(tmp_path: Path):
    project = bare_project(tmp_path)
    assert project.rel(tmp_path / "a" / "b.svg") == "a/b.svg"
