"""Offline facts about the project around a README: metadata, CI, license, tools, logos.

Nothing here touches the network. Every fact records where it came from, so
``readme-style --explain`` can show why a badge was or was not added.
"""

from __future__ import annotations

import json
import os
import posixpath
import re
import subprocess
import tomllib
from dataclasses import dataclass, field
from pathlib import Path


@dataclass
class Package:
    registry: str  # pypi, npm, crates or go
    name: str
    publish_evidence: str | None = None  # why we believe it is published, if we do


@dataclass
class Workflow:
    file: str
    name: str
    triggers: set[str]
    text: str


@dataclass
class Logo:
    light: str  # path relative to the README, shown in light mode (or always)
    dark: str | None = None


@dataclass
class Project:
    readme_dir: Path
    root: Path
    name: str | None = None
    description: str | None = None
    license: str | None = None
    license_file: str | None = None
    repo: str | None = None  # "owner/name" on GitHub
    repo_source: str | None = None
    packages: list[Package] = field(default_factory=list)
    python_requires: str | None = None
    python_classifiers: list[str] = field(default_factory=list)
    node_engine: str | None = None
    rust_version: str | None = None
    go_version: str | None = None
    workflows: list[Workflow] = field(default_factory=list)
    tools: dict[str, str] = field(default_factory=dict)  # tool -> evidence
    urls: dict[str, str] = field(default_factory=dict)  # lowercased label -> url
    logo: Logo | None = None

    @property
    def docs_url(self) -> str | None:
        """The documentation URL, unless it is just the repository page."""
        url = self.urls.get("documentation") or self.urls.get("docs")
        if url and self.repo and parse_github(url) == self.repo:
            return None
        return url

    def rel(self, path: Path) -> str:
        return Path(os.path.relpath(path, self.readme_dir)).as_posix()


def find_root(start: Path) -> Path:
    """The nearest directory that looks like a repository root (has .git or .github)."""
    start = start.resolve()
    for candidate in (start, *start.parents):
        if (candidate / ".git").exists() or (candidate / ".github").is_dir():
            return candidate
    return start


def _search(project: Project, *names: str) -> Path | None:
    """First existing file among `names`, looking in the README folder, then up to the root."""
    here = project.readme_dir
    while True:
        for name in names:
            if (here / name).is_file():
                return here / name
        if here == project.root or here.parent == here:
            return None
        here = here.parent


def _read_toml(path: Path) -> dict:
    try:
        return tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError):
        return {}


def load_project(readme: Path, *, repo: str | None = None, logo: str | None = None) -> Project:
    readme_dir = readme.resolve().parent
    project = Project(readme_dir=readme_dir, root=find_root(readme_dir))
    _python(project)
    _node(project)
    _rust(project)
    _go(project)
    if isinstance(project.description, str):
        project.description = " ".join(project.description.split()) or None
    else:
        project.description = None
    _license(project)
    _workflows(project)
    _tools(project)
    _repo(project, repo)
    _logo(project, logo)
    return project


# ecosystems


def _python(project: Project) -> None:
    path = _search(project, "pyproject.toml")
    if not path:
        return
    data = _read_toml(path)
    meta = data.get("project") or data.get("tool", {}).get("poetry") or {}
    name = meta.get("name")
    if not name:
        return
    project.name = project.name or name
    project.description = project.description or meta.get("description")
    project.packages.append(Package("pypi", name))
    project.python_requires = meta.get("requires-python") or (
        meta.get("dependencies", {}).get("python") if isinstance(meta.get("dependencies"), dict) else None
    )
    project.python_classifiers = [
        c.rsplit("::", 1)[1].strip()
        for c in meta.get("classifiers", [])
        if re.fullmatch(r"Programming Language :: Python :: 3\.\d+", c)
    ]
    lic = meta.get("license")
    if isinstance(lic, str):
        project.license = project.license or _spdx(lic)
    elif isinstance(lic, dict) and lic.get("text"):
        project.license = project.license or _spdx(lic["text"])
    for classifier in meta.get("classifiers", []):
        if classifier.startswith("License :: OSI Approved :: ") and not project.license:
            project.license = _CLASSIFIER_LICENSES.get(classifier.rsplit("::", 1)[1].strip())
    for label, url in (meta.get("urls") or {}).items():
        project.urls.setdefault(label.lower(), url)
    if meta.get("homepage"):
        project.urls.setdefault("homepage", meta["homepage"])


def _node(project: Project) -> None:
    path = _search(project, "package.json")
    if not path:
        return
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return
    name = data.get("name")
    if not name:
        return
    project.name = project.name or name
    project.description = project.description or data.get("description")
    if not data.get("private"):
        project.packages.append(Package("npm", name))
    engines = data.get("engines") or {}
    project.node_engine = engines.get("node")
    if isinstance(data.get("license"), str):
        project.license = project.license or _spdx(data["license"])
    if data.get("homepage"):
        project.urls.setdefault("homepage", data["homepage"])
    repository = data.get("repository")
    url = repository.get("url") if isinstance(repository, dict) else repository
    if isinstance(url, str):
        project.urls.setdefault("repository", url)


def _rust(project: Project) -> None:
    path = _search(project, "Cargo.toml")
    if not path:
        return
    package = _read_toml(path).get("package") or {}
    name = package.get("name")
    if not name:
        return
    project.name = project.name or name
    if isinstance(package.get("description"), str):
        project.description = project.description or package["description"]
    if package.get("publish") is not False:
        project.packages.append(Package("crates", name))
    if isinstance(package.get("rust-version"), str):
        project.rust_version = package["rust-version"]
    if isinstance(package.get("license"), str):
        project.license = project.license or _spdx(package["license"])
    for key in ("homepage", "documentation", "repository"):
        if isinstance(package.get(key), str):
            project.urls.setdefault(key, package[key])


def _go(project: Project) -> None:
    path = _search(project, "go.mod")
    if not path:
        return
    text = path.read_text(encoding="utf-8", errors="replace")
    module = re.search(r"^module\s+(\S+)", text, re.M)
    if not module:
        return
    project.packages.append(Package("go", module[1]))
    project.name = project.name or module[1].rsplit("/", 1)[-1]
    version = re.search(r"^go\s+(\d+\.\d+)", text, re.M)
    project.go_version = version[1] if version else None


# license

_CLASSIFIER_LICENSES = {
    "MIT License": "MIT",
    "Apache Software License": "Apache-2.0",
    "BSD License": "BSD",
    "ISC License (ISCL)": "ISC",
    "GNU General Public License v3 (GPLv3)": "GPL-3.0",
    "GNU General Public License v2 (GPLv2)": "GPL-2.0",
    "GNU Lesser General Public License v3 (LGPLv3)": "LGPL-3.0",
    "GNU Affero General Public License v3": "AGPL-3.0",
    "Mozilla Public License 2.0 (MPL 2.0)": "MPL-2.0",
    "The Unlicense (Unlicense)": "Unlicense",
}

_LICENSE_TEXTS = [
    (r"GNU AFFERO GENERAL PUBLIC LICENSE", "AGPL-3.0"),
    (r"GNU LESSER GENERAL PUBLIC LICENSE\s+Version 2\.1", "LGPL-2.1"),
    (r"GNU LESSER GENERAL PUBLIC LICENSE", "LGPL-3.0"),
    (r"GNU GENERAL PUBLIC LICENSE\s+Version 2", "GPL-2.0"),
    (r"GNU GENERAL PUBLIC LICENSE", "GPL-3.0"),
    (r"Apache License,?\s+Version 2\.0", "Apache-2.0"),
    (r"Mozilla Public License,? (?:Version )?2\.0", "MPL-2.0"),
    (r"Boost Software License", "BSL-1.0"),
    (r"This is free and unencumbered software released into the public domain", "Unlicense"),
    (r"Creative Commons Attribution 4\.0", "CC-BY-4.0"),
    (r"CC0 1\.0 Universal", "CC0-1.0"),
    (r"Permission to use, copy, modify, and/or distribute this software for any", "ISC"),
    (r"Permission is hereby granted, free of charge", "MIT"),
]


def _spdx(value: str) -> str | None:
    value = value.strip()
    if not value or len(value) > 60 or "\n" in value:
        return None
    return value


def detect_license_text(text: str) -> str | None:
    for pattern, spdx in _LICENSE_TEXTS:
        if re.search(pattern, text, re.I):
            return spdx
    if re.search(r"Redistribution and use in source and binary forms", text, re.I):
        return "BSD-3-Clause" if re.search(r"Neither the name", text, re.I) else "BSD-2-Clause"
    return None


def _license(project: Project) -> None:
    names = [
        "LICENSE",
        "LICENSE.md",
        "LICENSE.txt",
        "LICENCE",
        "LICENCE.md",
        "COPYING",
        "LICENSE-MIT",
        "LICENSE-APACHE",
    ]
    path = _search(project, *names)
    if not path:
        return
    project.license_file = project.rel(path)
    if not project.license or project.license == "BSD":
        text = path.read_text(encoding="utf-8", errors="replace")
        project.license = detect_license_text(text) or project.license


# CI and tooling

_ON = re.compile(r"""^(?:on|"on"|'on'|true)\s*:(.*)$""", re.M)


def parse_triggers(text: str) -> set[str]:
    match = _ON.search(text)
    if not match:
        return set()
    inline = match[1].split("#", 1)[0].strip()
    if inline:
        return {t.strip(" '\"") for t in inline.strip("[]").split(",") if t.strip()}
    triggers: set[str] = set()
    indent: int | None = None
    for line in text[match.end() :].splitlines()[1:]:
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        width = len(line) - len(line.lstrip())
        if width == 0:
            break
        if indent is None:
            indent = width
        if width == indent:
            key = re.match(r"\s*([\w-]+)\s*:", line)
            if key:
                triggers.add(key[1])
            elif line.strip().startswith("- "):
                triggers.add(line.strip()[2:].strip(" '\""))
    return triggers


def _workflows(project: Project) -> None:
    folder = project.root / ".github" / "workflows"
    if not folder.is_dir():
        return
    for path in sorted(folder.iterdir()):
        if path.suffix not in (".yml", ".yaml") or not path.is_file():
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        name = re.search(r"""^name:\s*["']?(.+?)["']?\s*$""", text, re.M)
        project.workflows.append(Workflow(path.name, name[1] if name else path.stem, parse_triggers(text), text))


_PUBLISH = {
    "pypi": re.compile(
        r"pypa/gh-action-pypi-publish|twine\s+upload|uv\s+publish|poetry\s+publish|flit\s+publish"
        r"|hatch\s+publish|maturin\s+(?:publish|upload)|PyO3/maturin-action",
        re.I,
    ),
    "npm": re.compile(r"\b(?:npm|pnpm|yarn)\s+publish\b|changesets/action|JS-DevTools/npm-publish", re.I),
    "crates": re.compile(r"cargo\s+publish|release-plz|katyo/publish-crates", re.I),
}


def _tools(project: Project) -> None:
    pyproject = _read_toml(p) if (p := _search(project, "pyproject.toml")) else {}
    tool = pyproject.get("tool", {})
    precommit_path = project.root / ".pre-commit-config.yaml"
    precommit = precommit_path.read_text(encoding="utf-8", errors="replace") if precommit_path.is_file() else ""
    if precommit:
        project.tools["pre-commit"] = ".pre-commit-config.yaml"
    if "ruff" in tool:
        project.tools["ruff"] = "[tool.ruff] in pyproject.toml"
    elif path := _search(project, "ruff.toml", ".ruff.toml"):
        project.tools["ruff"] = path.name
    elif "astral-sh/ruff-pre-commit" in precommit:
        project.tools["ruff"] = "ruff hook in .pre-commit-config.yaml"
    if "black" in tool:
        project.tools["black"] = "[tool.black] in pyproject.toml"
    elif "psf/black" in precommit:
        project.tools["black"] = "black hook in .pre-commit-config.yaml"
    if path := _search(project, "uv.lock"):
        project.tools["uv"] = path.name
    if path := _search(project, "codecov.yml", ".codecov.yml"):
        project.tools["codecov"] = path.name
    if path := _search(project, ".readthedocs.yaml", ".readthedocs.yml", "readthedocs.yml"):
        project.tools["readthedocs"] = path.name
    for workflow in project.workflows:
        if "codecov" not in project.tools and "codecov/codecov-action" in workflow.text:
            project.tools["codecov"] = f"codecov action in {workflow.file}"
        for package in project.packages:
            pattern = _PUBLISH.get(package.registry)
            if pattern and not package.publish_evidence and (m := pattern.search(workflow.text)):
                package.publish_evidence = f"{workflow.file} runs {m.group(0)}"


# repository and logo

_GITHUB = re.compile(r"github\.com[:/]+([\w.-]+)/([\w.-]+?)(?:\.git)?/?$")


def parse_github(url: str) -> str | None:
    match = _GITHUB.search(url.strip())
    return f"{match[1]}/{match[2]}" if match else None


def _repo(project: Project, override: str | None) -> None:
    if override:
        project.repo, project.repo_source = override, "config"
        return
    if (project.root / ".git").exists():
        try:
            url = subprocess.run(
                ["git", "-C", str(project.root), "config", "--get", "remote.origin.url"],
                capture_output=True,
                text=True,
                timeout=5,
            ).stdout
        except (OSError, subprocess.SubprocessError):
            url = ""
        if repo := parse_github(url):
            project.repo, project.repo_source = repo, "git remote origin"
            return
    for label in ("repository", "source", "source code", "code", "homepage"):
        if (url := project.urls.get(label)) and (repo := parse_github(url)):
            project.repo, project.repo_source = repo, f"'{label}' URL in project metadata"
            return


LOGO_DIRS = [
    ".",
    "assets",
    "asset",
    "images",
    "image",
    "img",
    "imgs",
    "media",
    "static",
    "resources",
    "art",
    "branding",
    "brand",
    "logo",
    "logos",
    ".github",
    ".github/assets",
    ".github/images",
    "docs",
    "docs/assets",
    "docs/images",
    "docs/img",
    "docs/static",
    "docs/_static",
    "docs/source/_static",
]
_LOGO_FILE = re.compile(r"^(?P<stem>[\w.-]*logo[\w.-]*)\.(?P<ext>svg|png|webp|jpe?g)$", re.I)
_DARK = {"dark", "white", "inverted", "negative", "night", "ondark", "fordark"}
_LIGHT = {"light", "black", "day", "positive", "onlight", "forlight"}
_EXT_RANK = {"svg": 0, "png": 1, "webp": 2, "jpg": 3, "jpeg": 3}


def _variant(stem: str) -> tuple[str, str]:
    """Split a file stem into (base, variant) where variant is light, dark or default."""
    tokens = re.split(r"[-_.]", stem.lower())
    joined = [t for t in tokens if t]
    for i in range(len(joined) - 1):  # "on-dark", "for-light"
        if joined[i] in ("on", "for") and joined[i + 1] in ("dark", "light"):
            joined[i : i + 2] = [joined[i] + joined[i + 1]]
            break
    variant = "default"
    base = []
    for token in joined:
        if token in _DARK:
            variant = "dark"
        elif token in _LIGHT:
            variant = "light"
        else:
            base.append(token)
    return "-".join(base), variant


def _logo(project: Project, override: str | None) -> None:
    if override:
        path = (project.readme_dir / override).resolve()
        if path.is_file():
            base, _ = _variant(path.stem)
            dark = next(
                (
                    p
                    for p in sorted(path.parent.iterdir())
                    if p != path and _LOGO_FILE.match(p.name) and _variant(p.stem) == (base, "dark")
                ),
                None,
            )
            project.logo = Logo(project.rel(path), project.rel(dark) if dark else None)
        return
    for folder in LOGO_DIRS:
        directory = project.readme_dir / folder
        if not directory.is_dir():
            continue
        groups: dict[str, dict[str, list[Path]]] = {}
        for path in sorted(directory.iterdir()):
            match = _LOGO_FILE.match(path.name)
            if not match or not path.is_file():
                continue
            base, variant = _variant(match["stem"])
            groups.setdefault(base, {}).setdefault(variant, []).append(path)
        ranked = sorted(groups.items(), key=lambda kv: (kv[0] != "logo", len(kv[0]), kv[0]))
        for _, variants in ranked:
            light = _best(variants, "light") or _best(variants, "default")
            dark = _best(variants, "dark")
            if light:
                project.logo = Logo(project.rel(light), project.rel(dark) if dark else None)
                return


def _best(variants: dict[str, list[Path]], kind: str) -> Path | None:
    files = variants.get(kind, [])
    return min(files, key=lambda p: _EXT_RANK[p.suffix[1:].lower()]) if files else None


def normalize_src(src: str) -> str:
    return posixpath.normpath(src.strip().removeprefix("./")) if src else ""
