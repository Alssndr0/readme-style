"""Badges inferred from project facts and README links, rendered with shields.io."""

from __future__ import annotations

import re
from dataclasses import dataclass
from urllib.parse import quote, urlencode, urlparse

from .document import Document
from .project import Package, Project, Workflow

GROUPS: dict[str, tuple[str, ...]] = {
    "package": ("pypi", "npm", "crates", "go"),
    "downloads": ("pypi-downloads", "npm-downloads", "crates-downloads"),
    "runtime": ("python", "node", "rust", "go-version"),
    "ci": ("ci",),
    "coverage": ("codecov",),
    "docs": ("docs", "docsrs"),
    "license": ("license",),
    "tooling": ("ruff", "black", "uv", "pre-commit"),
    "research": ("arxiv", "paper", "project-page", "hf-models", "hf-datasets", "hf-spaces", "colab"),
    "community": ("discord",),
}
BADGE_IDS = tuple(dict.fromkeys(i for ids in GROUPS.values() for i in ids))


@dataclass(frozen=True)
class Badge:
    id: str
    alt: str
    image: str
    link: str | None
    reason: str

    def markdown(self) -> str:
        image = f"![{_escape_alt(self.alt)}]({_dest(self.image)})"
        return f"[{image}]({_dest(self.link)})" if self.link else image


def expand(entries: tuple[str, ...] | list[str]) -> list[str]:
    ids: list[str] = []
    for entry in entries:
        for badge_id in GROUPS.get(entry, (entry,)):
            if badge_id not in ids:
                ids.append(badge_id)
    return ids


def unknown_entries(entries: tuple[str, ...] | list[str]) -> list[str]:
    return [e for e in entries if e not in GROUPS and e not in BADGE_IDS]


def select_badges(
    project: Project, doc: Document, entries: tuple[str, ...], style: str, limit: int, exclude: tuple[str, ...] = ()
) -> list[Badge]:
    available = candidates(project, doc, style)
    skip = set(expand(exclude))
    chosen = [available[i] for i in expand(entries) if i in available and i not in skip]
    return chosen[:limit]


# shields.io


def _part(text: str) -> str:
    return quote(text.replace("-", "--").replace("_", "__").replace(" ", "_"), safe="")


def shields(path: str, style: str, **params: str) -> str:
    query = urlencode({**{k: v for k, v in params.items() if v}, "style": style})
    return f"https://img.shields.io/{path}?{query}"


def static(label: str, message: str, color: str, style: str, **params: str) -> str:
    path = f"badge/{_part(label)}-{_part(message)}-{color}" if label else f"badge/{_part(message)}-{color}"
    return shields(path, style, **params)


def _endpoint(url: str, style: str) -> str:
    return f"https://img.shields.io/endpoint?{urlencode({'url': url, 'style': style})}"


def _escape_alt(text: str) -> str:
    return text.replace("\\", "\\\\").replace("[", "\\[").replace("]", "\\]")


def _dest(url: str | None) -> str:
    url = url or ""
    return f"<{url}>" if re.search(r"[\s()<>]", url) else url


# evidence that a package is published, read from the README itself


def _name_pattern(name: str) -> str:
    return r"[-_.]".join(re.escape(part) for part in re.split(r"[-_.]+", name))


def install_evidence(package: Package, text: str) -> str | None:
    n = _name_pattern(package.name)
    flags = r"(?:-\S+\s+)*"
    patterns = {
        "pypi": rf"(?:(?:python3?\s+-m\s+)?pip3?|uv\s+pip|pipx|uv\s+tool)\s+install\s+{flags}[\"']?{n}(?![\w-])"
        rf"|(?:uv|poetry|pdm|rye)\s+add\s+{flags}{n}(?![\w-])|uvx\s+{n}(?![\w-])",
        "npm": rf"(?:npm\s+(?:install|i|add)|yarn\s+(?:global\s+)?add|pnpm\s+(?:add|install|i)|bun\s+(?:add|install|i))"
        rf"\s+{flags}{n}(?![\w/-])|(?:npx|bunx|pnpx)\s+{n}(?![\w/-])",
        "crates": rf"cargo\s+(?:add|install)\s+{flags}{n}(?![\w-])",
        "go": re.escape(package.name),
    }
    match = re.search(patterns[package.registry], text, re.I)
    if not match:
        return None
    if package.registry == "go":
        return f"the README mentions {package.name}"
    return f"the README shows `{match.group(0).strip()}`"


# runtime versions


def python_version(project: Project) -> str | None:
    spec = project.python_requires
    if spec:
        if m := re.search(r"(?:>=|~=|\^)\s*(\d+\.\d+)", spec):
            return f"{m[1]}+"
        if m := re.search(r"==\s*(\d+\.\d+)", spec):
            return m[1]
    if project.python_classifiers:
        lowest = min(project.python_classifiers, key=lambda v: tuple(int(x) for x in v.split(".")))
        return f"{lowest}+"
    return None


def node_version(spec: str | None) -> str | None:
    if not spec or not (m := re.search(r"\d+", spec)):
        return None
    return f"{m[0]}+" if re.search(r"[>^~x*]", spec) else m[0]


_CI_SKIP = re.compile(
    r"release|publish|deploy|pages|stale|label|codeql|dependabot|greet|welcome|lock|triage|bump|backport"
    r"|\bcla\b|semantic|pr-title|issue|docs?\b|notify|authori[sz]|comment|remind|cleanup|cache|\bbots?\b|mirror",
    re.I,
)
_CI_RANK = [r"ci|tests?|build|main|checks?|pipeline", r"test", r"\bci\b", r"build|check|lint"]


def pick_workflow(workflows: list[Workflow]) -> Workflow | None:
    pool = [
        w
        for w in workflows
        if w.triggers & {"push", "pull_request", "pull_request_target", "merge_group"}
        and not _CI_SKIP.search(f"{w.file} {w.name}")
    ]

    def rank(w: Workflow) -> int:
        stem = w.file.rsplit(".", 1)[0]
        if re.fullmatch(_CI_RANK[0], stem, re.I) or re.fullmatch(_CI_RANK[0], w.name, re.I):
            return 0
        label = f"{w.name} {stem}"
        return next((i for i, p in enumerate(_CI_RANK[1:], 1) if re.search(p, label, re.I)), len(_CI_RANK))

    ranked = [w for w in pool if rank(w) < len(_CI_RANK)]
    return min(ranked, key=rank) if ranked else None


# research and community links found in the README

_ARXIV = re.compile(r"arxiv\.org/(?:abs|pdf)/(\d{4}\.\d{4,5})", re.I)
_ARXIV_CITE = re.compile(r"(?:arXiv:|eprint\s*=\s*[{\"])\s*(\d{4}\.\d{4,5})", re.I)
_PAPER_HOSTS = {
    "openreview.net": "OpenReview",
    "aclanthology.org": "ACL Anthology",
    "proceedings.neurips.cc": "NeurIPS",
    "openaccess.thecvf.com": "CVF",
    "proceedings.mlr.press": "PMLR",
}
_PAPER_TEXT = re.compile(r"paper|arxiv|pdf|preprint", re.I)
_MODEL_TEXT = re.compile(r"model|checkpoint|weight|hugging|🤗", re.I)
_DEMO_TEXT = re.compile(r"demo|space|try|playground", re.I)
_DATA_TEXT = re.compile(r"data", re.I)
_PROJECT_PAGE = re.compile(r"\b(project page|project website|website|homepage)\b", re.I)
_HF_NON_MODEL = {"docs", "blog", "papers", "learn", "join", "pricing", "tasks", "collections", "models", "settings"}


def _hf_badge(kind: str, href: str, style: str) -> Badge:
    message, badge_id = {"spaces": ("Demo", "hf-spaces"), "datasets": ("Dataset", "hf-datasets")}.get(
        kind, ("Models", "hf-models")
    )
    return Badge(
        badge_id,
        f"Hugging Face {message.lower()}",
        static("Hugging Face", message, "FFD21E", style, logo="huggingface", logoColor="000"),
        href,
        f"the README links a Hugging Face {message.lower()} page",
    )


def _research(project: Project, doc: Document, style: str) -> list[Badge]:
    """Paper, model and demo badges, only for links that are clearly about this project.

    A research README cites other papers and models too, so a link counts when it sits in
    the intro, when its text says what it is, or (for Hugging Face) when its organization
    matches the GitHub owner.
    """
    badges: dict[str, Badge] = {}
    owner = (project.repo or "").split("/")[0].lower()

    def add(badge: Badge) -> None:
        badges.setdefault(badge.id, badge)

    cited = _ARXIV_CITE.search(doc.text)
    if cited:
        add(
            Badge(
                "arxiv",
                f"arXiv {cited[1]}",
                static("arXiv", cited[1], "B31B1B", style, logo="arxiv", logoColor="white"),
                f"https://arxiv.org/abs/{cited[1]}",
                "the README's citation names an arXiv preprint",
            )
        )

    for link in doc.links:
        in_intro = link.line < doc.intro_end
        parsed = urlparse(link.href)
        host = parsed.netloc.lower().removeprefix("www.")
        path = parsed.path.strip("/")
        if m := _ARXIV.search(link.href):
            if in_intro or _PAPER_TEXT.search(link.text):
                add(
                    Badge(
                        "arxiv",
                        f"arXiv {m[1]}",
                        static("arXiv", m[1], "B31B1B", style, logo="arxiv", logoColor="white"),
                        f"https://arxiv.org/abs/{m[1]}",
                        "the README links its arXiv paper",
                    )
                )
        elif host in _PAPER_HOSTS:
            if in_intro or _PAPER_TEXT.search(link.text):
                add(
                    Badge(
                        "paper",
                        "Paper",
                        static("Paper", _PAPER_HOSTS[host], "8C1C13", style),
                        link.href,
                        f"the README links its paper on {host}",
                    )
                )
        elif host == "huggingface.co" and "/" in path:
            kind, rest = path.split("/", 1)
            org = rest.split("/")[0] if kind in ("spaces", "datasets") else kind
            text_ok = {"spaces": _DEMO_TEXT, "datasets": _DATA_TEXT}.get(kind, _MODEL_TEXT).search(link.text)
            if kind in _HF_NON_MODEL or not (in_intro or text_ok or (owner and org.lower() == owner)):
                continue
            add(_hf_badge(kind, link.href, style))
        elif host == "colab.research.google.com":
            add(
                Badge(
                    "colab",
                    "Open in Colab",
                    static("Open in", "Colab", "F9AB00", style, logo="googlecolab", logoColor="white"),
                    link.href,
                    "the README links a Colab notebook",
                )
            )
        elif parsed.scheme in ("http", "https") and _PROJECT_PAGE.search(link.text):
            add(
                Badge(
                    "project-page",
                    "Project page",
                    static("", "Project Page", "2F80ED", style),
                    link.href,
                    f"the README links '{link.text}'",
                )
            )
    homepage = project.urls.get("homepage", "")
    if urlparse(homepage).netloc.endswith("github.io"):
        add(
            Badge(
                "project-page",
                "Project page",
                static("", "Project Page", "2F80ED", style),
                homepage,
                "the project homepage is a GitHub Pages site",
            )
        )
    return list(badges.values())


def _community(doc: Document, style: str) -> list[Badge]:
    for link in doc.links:
        if re.search(r"(?:discord\.gg|discord\.com/invite)/\w+", link.href, re.I):
            return [
                Badge(
                    "discord",
                    "Discord",
                    static("Discord", "join", "5865F2", style, logo="discord", logoColor="white"),
                    link.href,
                    "the README links a Discord invite",
                )
            ]
    return []


# all candidates


def candidates(project: Project, doc: Document, style: str) -> dict[str, Badge]:
    found: dict[str, Badge] = {}

    def add(badge: Badge) -> None:
        found.setdefault(badge.id, badge)

    for package in project.packages:
        evidence = package.publish_evidence or install_evidence(package, doc.user_text)
        if not evidence:
            continue
        name = package.name
        if package.registry == "pypi":
            add(
                Badge(
                    "pypi",
                    "PyPI version",
                    shields(f"pypi/v/{name}", style, logo="pypi", logoColor="white"),
                    f"https://pypi.org/project/{name}/",
                    f"published on PyPI: {evidence}",
                )
            )
            add(
                Badge(
                    "pypi-downloads",
                    "PyPI downloads",
                    shields(f"pypi/dm/{name}", style),
                    f"https://pypistats.org/packages/{name.lower()}",
                    f"published on PyPI: {evidence}",
                )
            )
        elif package.registry == "npm":
            add(
                Badge(
                    "npm",
                    "npm version",
                    shields(f"npm/v/{name}", style, logo="npm"),
                    f"https://www.npmjs.com/package/{name}",
                    f"published on npm: {evidence}",
                )
            )
            add(
                Badge(
                    "npm-downloads",
                    "npm downloads",
                    shields(f"npm/dm/{name}", style),
                    f"https://www.npmjs.com/package/{name}",
                    f"published on npm: {evidence}",
                )
            )
        elif package.registry == "crates":
            add(
                Badge(
                    "crates",
                    "crates.io version",
                    shields(f"crates/v/{name}", style, logo="rust"),
                    f"https://crates.io/crates/{name}",
                    f"published on crates.io: {evidence}",
                )
            )
            add(
                Badge(
                    "crates-downloads",
                    "crates.io downloads",
                    shields(f"crates/d/{name}", style),
                    f"https://crates.io/crates/{name}",
                    f"published on crates.io: {evidence}",
                )
            )
            add(
                Badge(
                    "docsrs",
                    "docs.rs",
                    shields(f"docsrs/{name}", style, logo="docsdotrs"),
                    f"https://docs.rs/{name}",
                    f"published on crates.io: {evidence}",
                )
            )
        elif package.registry == "go":
            add(
                Badge(
                    "go",
                    "Go reference",
                    static("go", "reference", "007D9C", style, logo="go", logoColor="white"),
                    f"https://pkg.go.dev/{name}",
                    evidence,
                )
            )

    if version := python_version(project):
        add(
            Badge(
                "python",
                f"Python {version}",
                static("python", version, "3776AB", style, logo="python", logoColor="white"),
                None,
                "requires-python or classifiers in pyproject.toml",
            )
        )
    if version := node_version(project.node_engine):
        add(
            Badge(
                "node",
                f"Node.js {version}",
                static("node", version, "339933", style, logo="nodedotjs", logoColor="white"),
                None,
                "engines.node in package.json",
            )
        )
    if project.rust_version:
        add(
            Badge(
                "rust",
                f"Rust {project.rust_version}+",
                static("rust", f"{project.rust_version}+", "CE422B", style, logo="rust"),
                None,
                "rust-version in Cargo.toml",
            )
        )
    if project.go_version:
        add(
            Badge(
                "go-version",
                f"Go {project.go_version}+",
                static("go", f"{project.go_version}+", "00ADD8", style, logo="go", logoColor="white"),
                None,
                "go directive in go.mod",
            )
        )

    repo = project.repo
    if repo and (workflow := pick_workflow(project.workflows)):
        add(
            Badge(
                "ci",
                workflow.name,
                shields(
                    f"github/actions/workflow/status/{repo}/{workflow.file}", style, label=workflow.name, logo="github"
                ),
                f"https://github.com/{repo}/actions/workflows/{workflow.file}",
                f"{workflow.file} runs on {', '.join(sorted(workflow.triggers))}",
            )
        )
    if repo and "codecov" in project.tools:
        add(
            Badge(
                "codecov",
                "Coverage",
                shields(f"codecov/c/github/{repo}", style, logo="codecov"),
                f"https://codecov.io/gh/{repo}",
                project.tools["codecov"],
            )
        )

    docs_url = project.docs_url
    if "readthedocs" in project.tools and project.name:
        host = urlparse(docs_url or "").netloc
        slug = host.split(".")[0] if host.endswith(".readthedocs.io") else project.name.lower()
        add(
            Badge(
                "docs",
                "Documentation",
                shields(f"readthedocs/{slug}", style, logo="readthedocs"),
                docs_url or f"https://{slug}.readthedocs.io",
                project.tools["readthedocs"],
            )
        )
    elif docs_url:
        add(
            Badge(
                "docs",
                "Documentation",
                static("docs", "online", "0A7BBB", style),
                docs_url,
                "documentation URL in project metadata",
            )
        )

    if project.license:
        add(
            Badge(
                "license",
                f"License: {project.license}",
                static("license", project.license, "blue", style),
                project.license_file,
                f"license {project.license} from metadata or {project.license_file}",
            )
        )

    tools = project.tools
    if "ruff" in tools:
        add(
            Badge(
                "ruff",
                "Ruff",
                _endpoint("https://raw.githubusercontent.com/astral-sh/ruff/main/assets/badge/v2.json", style),
                "https://github.com/astral-sh/ruff",
                tools["ruff"],
            )
        )
    if "black" in tools:
        add(
            Badge(
                "black",
                "Code style: black",
                static("code style", "black", "000000", style),
                "https://github.com/psf/black",
                tools["black"],
            )
        )
    if "uv" in tools:
        add(
            Badge(
                "uv",
                "uv",
                _endpoint("https://raw.githubusercontent.com/astral-sh/uv/main/assets/badge/v0.json", style),
                "https://github.com/astral-sh/uv",
                tools["uv"],
            )
        )
    if "pre-commit" in tools:
        add(
            Badge(
                "pre-commit",
                "pre-commit",
                static("pre-commit", "enabled", "brightgreen", style, logo="pre-commit"),
                "https://github.com/pre-commit/pre-commit",
                tools["pre-commit"],
            )
        )

    for badge in _research(project, doc, style) + _community(doc, style):
        add(badge)
    return found
