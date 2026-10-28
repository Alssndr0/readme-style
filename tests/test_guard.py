import pytest

from readme_style import GuardError, get_theme, style_text
from readme_style.guard import atoms, verify
from readme_style.transforms import PIPELINE

from .conftest import bare_project


def test_identical_content_passes():
    verify("# T\n\nSome *text* with `code`.\n", "# T\n\nSome *text* with `code`.\n")


def test_dropped_word_is_caught():
    with pytest.raises(GuardError, match="drop"):
        verify("# T\n\nkeep these words\n", "# T\n\nkeep words\n")


def test_changed_code_is_caught():
    with pytest.raises(GuardError):
        verify("```\npip install x\n```\n", "```\npip install y\n```\n")


def test_changed_link_is_caught():
    with pytest.raises(GuardError):
        verify("[docs](https://a)\n", "[docs](https://b)\n")


def test_merged_paragraphs_are_caught():
    with pytest.raises(GuardError):
        verify("one\n\ntwo\n", "one\ntwo\n")


def test_heading_level_change_is_caught():
    with pytest.raises(GuardError):
        verify("## Usage\n", "### Usage\n")


def test_text_added_outside_markers_is_caught():
    with pytest.raises(GuardError, match="add"):
        verify("# T\n", "# T\n\nsurprise\n")


def test_alerts_count_as_their_keyword():
    verify("> **Note:** read me\n", "> [!NOTE]\n> read me\n")
    verify("**Warning:** hot\n", "> [!WARNING]\n> hot\n")


def test_managed_content_is_ignored():
    after = "# T\n\n<!-- readme-style:header theme=x -->\nbadges here\n<!-- /readme-style:header -->\n"
    verify("# T\n", after)
    assert atoms(after) == atoms("# T\n")


def test_engine_refuses_a_transform_that_loses_content(monkeypatch, tmp_path):
    def evil(doc, ctx):
        return [line for line in doc.lines if "secret" not in line]

    monkeypatch.setattr("readme_style.engine.PIPELINE", [*PIPELINE, evil])
    with pytest.raises(GuardError):
        style_text("# T\n\nsecret sauce\n", get_theme("minimal"), bare_project(tmp_path))
