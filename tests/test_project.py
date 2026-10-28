import json

from readme_style.badges import candidates, install_evidence, pick_workflow, python_version
from readme_style.document import Document
from readme_style.project import Package, Workflow, detect_license_text, load_project, parse_github, parse_triggers

PYPROJECT = """
[project]
name = "quickparse"
description = "A tiny parser."
requires-python = ">=3.10"
license = "MIT"

[project.urls]
Documentation = "https://quickparse.readthedocs.io"
Repository = "https://github.com/acme/quickparse"

[tool.ruff]
line-length = 100
"""


def test_python_project(tree):
    readme = tree(
        {
            "README.md": "# quickparse\n",
            "pyproject.toml": PYPROJECT,
            "uv.lock": "",
            ".github/workflows/release.yml": "on:\n  release:\njobs:\n  x:\n    steps:\n      - uses: pypa/gh-action-pypi-publish@release/v1\n",
        }
    )
    project = load_project(readme)
    assert (project.name, project.description, project.license) == ("quickparse", "A tiny parser.", "MIT")
    assert project.repo == "acme/quickparse"
    assert project.packages[0].publish_evidence == "release.yml runs pypa/gh-action-pypi-publish"
    assert set(project.tools) == {"ruff", "uv"}
    assert python_version(project) == "3.10+"


def test_node_project_and_private_packages(tree):
    data = {"name": "tidy-cli", "description": "Tidy.", "license": "ISC", "engines": {"node": ">=18"}}
    project = load_project(tree({"README.md": "# x\n", "package.json": json.dumps(data)}))
    assert project.packages == [Package("npm", "tidy-cli")]
    assert project.node_engine == ">=18"
    private = load_project(tree({"README.md": "# x\n", "package.json": json.dumps({**data, "private": True})}))
    assert private.packages == []


def test_license_detection_from_text(tree):
    assert detect_license_text("Apache License\n Version 2.0, January 2004") == "Apache-2.0"
    assert (
        detect_license_text("Redistribution and use in source and binary forms ... Neither the name") == "BSD-3-Clause"
    )
    project = load_project(
        tree({"README.md": "# x\n", "LICENSE": "MIT License\n\nPermission is hereby granted, free of charge"})
    )
    assert (project.license, project.license_file) == ("MIT", "LICENSE")


def test_trigger_parsing():
    assert parse_triggers("on: push\n") == {"push"}
    assert parse_triggers("on: [push, pull_request]\n") == {"push", "pull_request"}
    assert parse_triggers("name: x\non:\n  push:\n    branches: [main]\n  pull_request:\njobs:\n") == {
        "push",
        "pull_request",
    }
    assert parse_triggers('"on":\n  - push\n  - workflow_dispatch\n') == {"push", "workflow_dispatch"}


def test_ci_workflow_choice():
    flows = [
        Workflow("notify-ci.yml", "Notify CI authorization", {"pull_request_target"}, ""),
        Workflow("lint.yml", "Lint", {"push"}, ""),
        Workflow("tests.yml", "Tests", {"push", "pull_request"}, ""),
        Workflow("release.yml", "Release", {"push"}, ""),
    ]
    assert pick_workflow(flows).file == "tests.yml"
    assert pick_workflow([Workflow("buf.yml", "Buf", {"pull_request"}, "")]) is None


def test_github_urls():
    assert parse_github("git@github.com:acme/tool.git") == "acme/tool"
    assert parse_github("https://github.com/acme/tool") == "acme/tool"
    assert parse_github("https://gitlab.com/acme/tool") is None


def test_install_commands_count_as_publish_evidence():
    package = Package("pypi", "quick_parse")
    assert install_evidence(package, "```\npip install -U quick-parse[cli]\n```")
    assert not install_evidence(package, "pip install quick-parser")
    assert install_evidence(Package("npm", "@acme/tidy"), "npm i -g @acme/tidy")


def test_logo_detection_prefers_light_and_dark_pairs(tree):
    readme = tree(
        {
            "README.md": "# x\n",
            "assets/logo.svg": "<svg/>",
            "assets/logo-dark.svg": "<svg/>",
            "assets/logo.png": "png",
            "docs/logo-white.png": "png",
        }
    )
    logo = load_project(readme).logo
    assert (logo.light, logo.dark) == ("assets/logo.svg", "assets/logo-dark.svg")


def test_research_badges_only_for_the_projects_own_links(tmp_path):
    from .conftest import bare_project

    project = bare_project(tmp_path, repo="lab/model")
    doc = Document(
        "# Model\n\n[Paper](https://arxiv.org/abs/2401.00001) · [Weights](https://huggingface.co/lab/model-7b)\n\n"
        "## Related work\n\nBuilds on [BERT](https://arxiv.org/abs/1810.04805) and "
        "[their model](https://huggingface.co/google/bert).\n"
    )
    found = candidates(project, doc, "flat")
    assert found["arxiv"].link == "https://arxiv.org/abs/2401.00001"
    assert found["hf-models"].link == "https://huggingface.co/lab/model-7b"
