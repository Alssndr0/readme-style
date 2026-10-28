import pytest

from readme_style.editor import Editor


def test_insert_pads_with_single_blank_lines():
    ed = Editor(["# Title", "text"])
    ed.insert(1, ["block"])
    assert ed.apply() == ["# Title", "", "block", "", "text"]


def test_insert_does_not_double_existing_blank_lines():
    ed = Editor(["# Title", "", "text"])
    ed.insert(2, ["block"])
    assert ed.apply() == ["# Title", "", "block", "", "text"]


def test_delete_collapses_the_seam():
    ed = Editor(["a", "", "x", "", "b"])
    ed.delete(2, 3)
    assert ed.apply() == ["a", "", "b"]


def test_delete_at_start_and_end():
    ed = Editor(["x", "", "a", "", "y"])
    ed.delete(0, 1)
    ed.delete(4, 5)
    assert ed.apply() == ["a"]


def test_inserts_at_the_same_line_merge_in_order():
    ed = Editor(["text"])
    ed.insert(0, ["first"])
    ed.insert(0, ["second"])
    assert ed.apply() == ["first", "", "second", "", "text"]


def test_overlapping_edits_are_rejected():
    ed = Editor(["a", "b", "c"])
    ed.replace(0, 2, ["x"])
    ed.replace(1, 3, ["y"])
    with pytest.raises(ValueError):
        ed.apply()
