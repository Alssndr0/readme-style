from readme_style.document import Document


def test_regions_and_wrappers_are_found():
    doc = Document(
        '<div align="center"><!-- readme-style:center -->\n\n'
        "# Title\n\n"
        "<!-- readme-style:header theme=startup -->\nbadges\n<!-- /readme-style:header -->\n\n"
        "</div><!-- /readme-style:center -->\n"
    )
    assert [(r.name, r.start, r.end, r.attrs) for r in doc.regions] == [("header", 4, 6, {"theme": "startup"})]
    assert doc.wrapper_lines == [0, 8]
    assert doc.title is not None and doc.title.text == "Title"


def test_markers_inside_code_blocks_are_ignored():
    doc = Document(
        "# T\n\n```md\n<!-- readme-style:toc -->\nx\n<!-- /readme-style:toc -->\n<a><!-- readme-style:center -->\n```\n"
    )
    assert doc.regions == []
    assert doc.wrapper_lines == []


def test_unclosed_marker_is_reported_and_ignored():
    doc = Document("# T\n\n<!-- readme-style:toc -->\n\ntext\n")
    assert doc.regions == []
    assert any("never closed" in w for w in doc.warnings)


def test_title_tagline_and_sections():
    doc = Document("# Name\n\nShort pitch.\n\nLonger intro.\n\n## Install\n\n### Pip\n\n## Usage\n")
    assert doc.title.text == "Name"
    assert doc.tagline is not None and doc.lines[doc.tagline.start] == "Short pitch."
    assert [h.text for h in doc.sections] == ["Install", "Usage"]
    assert doc.intro_end == 6


def test_first_heading_counts_as_title_when_nothing_outranks_it():
    doc = Document("tool\n----\n\nintro\n\n### One\n\n### Two\n")
    assert doc.title is not None and doc.title.text == "tool"
    assert [h.text for h in doc.sections] == ["One", "Two"]


def test_html_title():
    doc = Document('<p align="center"><img src="logo.png"></p>\n<h1 align="center">Name</h1>\n\ntext\n')
    assert doc.title is not None and doc.title.heading is None
    assert doc.has_hero_image


def test_user_badges_and_toc_detection():
    doc = Document(
        "# T\n\n[![CI](https://github.com/o/r/actions/workflows/ci.yml/badge.svg)](x)\n\n"
        "- [A](#a)\n- [B](#b)\n- [C](#c)\n\n## A\n## B\n## C\n"
    )
    assert doc.has_user_badges
    assert not doc.has_hero_image
    assert doc.has_user_toc


def test_links_carry_their_line():
    doc = Document("# T\n\nSee [the paper](https://arxiv.org/abs/2401.00001).\n\n## Usage\n\n[x](https://y)\n")
    assert [(link.href, link.text, link.line) for link in doc.links] == [
        ("https://arxiv.org/abs/2401.00001", "the paper", 2),
        ("https://y", "x", 6),
    ]
