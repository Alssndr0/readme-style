import pytest

from readme_style.slug import Slugger, slugify


@pytest.mark.parametrize(
    ("text", "slug"),
    [
        ("Hello World", "hello-world"),
        ("API Reference (v2)", "api-reference-v2"),
        ("C++ & Rust", "c--rust"),
        ("🚀 Quick Start", "-quick-start"),
        ("What's new?", "whats-new"),
        ("snake_case_name", "snake_case_name"),
        ("1.2.3 Release", "123-release"),
        ("Ünïcödé", "ünïcödé"),
        ("sampling / inference", "sampling--inference"),
    ],
)
def test_slugify_matches_github(text, slug):
    assert slugify(text) == slug


def test_repeated_headings_get_suffixes():
    slugger = Slugger()
    assert [slugger.slug(t) for t in ["Usage", "Usage", "Usage-1", "Usage"]] == [
        "usage",
        "usage-1",
        "usage-1-1",
        "usage-2",
    ]
