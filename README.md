<div align="center"><!-- readme-style:center -->

<!-- readme-style:logo theme=startup -->
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/logo-dark.svg">
  <img src="assets/logo.svg" alt="readme-style logo" width="120">
</picture>
<!-- /readme-style:logo -->

# readme-style

Keep your README's words, upgrade its presentation.

<!-- readme-style:header theme=startup -->
[![CI](https://img.shields.io/github/actions/workflow/status/Alssndr0/readme-style/ci.yml?label=CI&logo=github&style=for-the-badge)](https://github.com/Alssndr0/readme-style/actions/workflows/ci.yml) [![License: MIT](https://img.shields.io/badge/license-MIT-blue?style=for-the-badge)](LICENSE)

**[Why](#why)** · **[Install](#install)** · **[Quick start](#quick-start)** · **[Themes](#themes)** · **[Contributing](#contributing)** · **[License](#license)**

---
<!-- /readme-style:header -->

</div><!-- /readme-style:center -->

readme-style restyles a README in one command: a centered header, badges inferred from your
project, a nav bar or table of contents, GitHub alerts, collapsible long sections and
light and dark logos. It never rewrites your text. Every result is checked against the original,
and if a single word, code block or link would change, nothing is written.

## Why

Generators such as readme-ai write a README from your code. That helps a new repository, but most
projects already have a README whose words are right and whose presentation is not. Asking a chat
model to "make it prettier" works once, then drifts: it rephrases the install steps, drops a flag,
and gives a different result every time.

readme-style takes the opposite trade-off:

- **Your words stay.** The README is parsed with comrak, the Rust port of GitHub's own Markdown
  parser, and only presentation changes are made. A guard compares what GitHub would render before
  and after, and refuses to write if any content differs.
- **Deterministic and idempotent.** The same input always gives the same output, a second run
  changes nothing, and `--check` works in CI.
- **Offline, no API key.** Badges come from `pyproject.toml`, `package.json`, `Cargo.toml`,
  `go.mod`, your workflows, license, tool configuration and git remote, not from a model.
- **Diff first.** You see the diff before anything is written.
- **Fast.** One static binary; a typical README takes a few milliseconds.

## Install

```bash
cargo install readme-style    # from source, with Rust 1.89 or newer
pip install readme-style      # prebuilt binary in a wheel
npm install -g readme-style   # prebuilt binary, or: npx readme-style
```

> [!NOTE]
> readme-style is not released yet. Until the first release, install it from a clone with
> `cargo install --path .`

## Quick start

```bash
cd your-project
readme-style                    # show the diff, then ask before writing
readme-style --theme startup    # pick a theme
readme-style --write            # apply without asking
readme-style --check            # exit 1 if the README is not styled (for CI)
readme-style --explain          # show what was detected, and why each badge is there
```

## Themes

| Theme | Layout | Badges | Navigation | Extras |
|-------|--------|--------|------------|--------|
| `minimal` | Left aligned, pure Markdown | Package, CI, license | Contents list | Callouts |
| `startup` | Centered hero with logo and tagline | Package, downloads, CI, coverage, docs, license, community | Bold nav bar | Folded changelog and FAQ, divider |
| `ml-research` | Centered, paper style | arXiv, project page, Hugging Face, Colab, runtime, license | Nav bar, folded contents | Folded news |
| `cli` | Centered, terminal look | Package, CI, runtime, license, in one monochrome palette | Key-cap nav | Folded changelog and FAQ |
| `docs` | Left aligned, for long READMEs | Package, CI, coverage, docs, license | Three-level contents | Back-to-top links, folded release notes |

This README is styled with `startup`. Themes are TOML: you can override any setting, or define your own
theme that extends a built-in one. See [docs/themes.md](docs/themes.md).

## Before and after

[examples/](examples/README.md) shows three projects styled with every theme: what changed, the
rendered result and the exact diff. On a research repository, for instance, `ml-research` adds a
centered header with arXiv, project page, Hugging Face, Colab, Python and license badges (inferred
from links the README already had), a folded table of contents and a folded news section, and turns
a "**Note:**" into an alert. Nothing else in the file changes.

## What it changes

| Feature | What happens | Comes from |
|---------|--------------|------------|
| Badges | Package version and downloads, CI status, coverage, docs, license, Python, Node, Rust and Go versions, Ruff, uv, pre-commit, arXiv, Hugging Face, Colab, Discord | Project metadata, workflows, config files, links already in the README |
| Table of contents | A list, or a folded `<details>`, with GitHub anchors | Your headings |
| Nav bar | Links to the main sections, plus your docs site | Your headings and docs URL |
| Callouts | `> **Note:** ...` and `**Warning:** ...` become GitHub alerts | Your text |
| Folded sections | Long changelog, news or FAQ sections fold into `<details>`; the heading and its anchor stay | Your sections |
| Back to top | A link back to the title after long sections | Your sections |
| Logo | Placed above the title, as a `<picture>` with a dark-mode variant when you have one | `logo.svg`, `logo-dark.svg` and similar names in `assets/`, `docs/`, `.github/` and friends |
| Centered layout | A `<div align="center">` around the hero, with Markdown still rendered inside | The theme |

A registry badge needs evidence that the package is published: a release workflow (such as
`pypa/gh-action-pypi-publish` or `npm publish`) or an install command for it in the README. A README
that already has badges keeps its own and gets none added; one that already links to its sections
gets no second nav bar.

## The content guarantee

After styling, readme-style removes everything it generated from both versions, renders them the
way GitHub does, and compares them as a sequence of words, code (exact), link targets, images,
heading levels and block boundaries. They must be identical. A GitHub alert's title counts as the
"Note:" it replaced; nothing else is normalized. If anything differs, the file is left untouched,
the command exits with code 3, and the message names the line and the difference.

It then runs the whole pipeline again on its own output, and refuses to write if that would change
anything, so `--check` never flaps. Both checks run on every invocation, not only in tests.

## How it works

Everything readme-style adds sits between HTML comments, which GitHub does not show, so the README
itself is the only state:

```md
<!-- readme-style:header theme=startup -->
[![PyPI version](https://img.shields.io/pypi/v/yourpkg)](https://pypi.org/project/yourpkg/)
<!-- /readme-style:header -->
```

Content between markers is regenerated on every run, so edit outside them. Layout that wraps your
content uses single generated lines, such as `<div align="center"><!-- readme-style:center -->`,
rebuilt from scratch each time. Switching themes gives the same result as styling the original
with the new theme, and later runs reuse the theme recorded in the markers, so `--check` needs no
configuration. [docs/architecture.md](docs/architecture.md) explains the design.

## Limits

- Markdown only: `README.rst` and other formats are left alone.
- Content between markers belongs to readme-style and is rewritten on every run.
- Switching themes can leave a blank line where a removed region was; blank lines between blocks do
  not change how GitHub renders the README.
- HTML that never closes a tag swallows whatever follows it, on GitHub as in a browser, so no change
  next to it is safe: readme-style refuses, and names the line, until the tag is closed.
- Dynamic badges (versions, CI status) come from shields.io and only work for public repositories and
  published packages; that is why registry badges need evidence of publishing.

## Configuration

Optional. Put it in `.readme-style.toml`, `[tool.readme-style]` in `pyproject.toml`,
`[package.metadata.readme-style]` in `Cargo.toml`, or a `"readme-style"` key in `package.json`:

```toml
theme = "startup"
repo = "owner/name"                  # when there is no GitHub remote

[badges]
exclude = ["downloads"]
style = "flat"                       # flat, flat-square, for-the-badge, plastic, social

[toc]
style = "details"                    # none, list or details

[logo]
path = "docs/brand/logo.svg"

[themes.brand]                       # a theme of your own
extends = "startup"
description = "Startup, in our colors"
badges.color = "ff6600"
```

Every setting is listed in [docs/configuration.md](docs/configuration.md).

## Suggestions from a model

Styling never needs a model. For the two things that do take judgment, `readme-style suggest` asks one
and prints the answer; nothing is written, and once you paste a suggestion it is your text:

```bash
readme-style suggest tagline                     # three one-line pitches, from the README and metadata
readme-style suggest diagram                     # a Mermaid flowchart of the architecture
readme-style suggest tagline --base-url http://127.0.0.1:8000/v1   # a local OpenAI-compatible server
```

It uses Anthropic's API when `ANTHROPIC_API_KEY` is set (Claude Opus 5.5 by default, with
refusal fallbacks), or any OpenAI-compatible server such as vLLM, Ollama or LM Studio; set it with
flags, the environment, or an `[ai]` table in the configuration (which may pick the provider and
model, never the host or the key). This is the only command that uses the network.

## Use it in CI

### GitHub Action

```yaml
- uses: actions/checkout@v4
- uses: Alssndr0/readme-style@main   # pin a release tag once there is one
  with:
    mode: check                      # or write, then commit the result in a later step
```

### pre-commit

```yaml
repos:
  - repo: https://github.com/Alssndr0/readme-style
    rev: main                        # pin a release tag once there is one
    hooks:
      - id: readme-style             # or readme-style-check to only verify
```

### Machine-readable output

`readme-style --check --format json` prints one report per README with its status, theme, badges
(each with the evidence that earned it), the steps that changed something, and any diagnostics.

## Roadmap

- Generated SVG banners per theme, in light and dark variants.
- A browser playground built from the same engine compiled to WebAssembly.
- A gallery of before and after on well-known repositories.

## Contributing

Bug reports and pull requests are welcome. [CONTRIBUTING.md](CONTRIBUTING.md) covers the setup,
the tests (unit, snapshot, property and a corpus of real READMEs) and how to add a theme or a badge.

## License

MIT
