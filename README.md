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

**[Why](#why)** · **[Install](#install)** · **[Quick start](#quick-start)** · **[Themes](#themes)** · **[What it changes](#what-it-changes)** · **[License](#license)**

---
<!-- /readme-style:header -->

</div><!-- /readme-style:center -->

readme-style restyles a README in one command: a centered hero, badges inferred from your
project, a nav bar or table of contents, GitHub alerts, collapsible long sections and
light/dark logos. It never rewrites your text. Every run is checked against the original,
and if a single word, code block or link would change, nothing is written.

## Why

Tools like readme-ai generate a README from your code. That helps for a new repository, but
most projects already have a README whose words are right and whose presentation is not.
Asking a chat model to "make it prettier" works once, then drifts: it rephrases the install
steps, drops a flag, and gives a different result every time.

readme-style takes the opposite trade-off:

- **Your words stay.** The README is parsed into a Markdown AST and only presentation changes
  are applied. A guard compares before and after, and refuses to write if any content differs.
- **Deterministic and idempotent.** The same input always gives the same output, a second run
  changes nothing, and `--check` works in CI.
- **Offline, no API key.** Badges come from `pyproject.toml`, `package.json`, `Cargo.toml`,
  `go.mod`, your workflows, license and git remote, not from a model.
- **Diff first.** You see the diff before anything is written.

## Install

```bash
pip install readme-style
```

> [!NOTE]
> readme-style is not on PyPI yet. Until the first release, install it from a clone with
> `pip install .` (Python 3.11 or newer).

## Quick start

```bash
cd your-project
readme-style                    # show the diff, then ask before writing
readme-style --theme startup    # pick a theme
readme-style --write            # apply without asking
readme-style --check            # exit 1 if the README is not styled (for CI)
readme-style --explain          # show what was detected and why
```

## Themes

| Theme | Layout | Badges | Navigation | Extras |
|-------|--------|--------|------------|--------|
| `minimal` | Left aligned, pure Markdown | Package, CI, license (flat-square) | Contents list | Callouts |
| `startup` | Centered hero with logo and tagline | Package, downloads, CI, coverage, docs, license, community (for-the-badge) | Nav bar | Collapsible changelog and FAQ, divider |
| `ml-research` | Centered, paper style | arXiv, project page, Hugging Face, Colab, Python, license (flat) | Nav bar, collapsed contents | Collapsible news |

The [examples](examples/) folder shows them side by side: each example keeps its original
`README.md` next to one styled file per theme. This README is styled with `startup`.

## What it changes

| Feature | What happens | Comes from |
|---------|--------------|------------|
| Badges | Package version and downloads, CI status, coverage, docs, license, Python, Node, Rust and Go versions, Ruff, uv, pre-commit, arXiv, Hugging Face, Colab, Discord | Project metadata, workflows, config files, links already in the README |
| Table of contents | A list, or a collapsed `<details>`, with GitHub anchors | Your headings |
| Nav bar | Links to the main sections, plus your docs site | Your headings and docs URL |
| Callouts | `> **Note:** ...` and `**Warning:** ...` become GitHub alerts | Your text |
| Collapsible sections | Long changelog, news or FAQ sections fold into `<details>`; the heading and its anchor stay | Your sections |
| Logo | Placed above the title, as a `<picture>` with a dark-mode variant when you have one | `logo.svg`, `logo-dark.svg` and similar names in `assets/`, `docs/`, `.github/` and friends |
| Centered layout | A `<div align="center">` around the hero, with Markdown still rendered inside | The theme |

Registry badges only appear with evidence that the package is published: a release workflow
(such as `pypa/gh-action-pypi-publish` or `npm publish`) or an install command for it in the
README. A README that already has badges keeps its own, and gets none added.

## The content guarantee

After styling, readme-style removes everything it generated from both versions, renders them
to HTML and compares them as a sequence of words, code (exact), link targets, images, heading
levels and block boundaries. They must be identical. The only normalization is that an alert
marker such as `[!NOTE]` counts as the word it replaced. If anything else differs, the file is
left untouched and the command exits with code 3, naming the first difference.

It then runs the whole pipeline again on its own output, and refuses to write if that would
change anything, so `--check` never flaps.

## How it works

Everything readme-style adds sits between HTML comments, so the README itself is the only state:

```md
<!-- readme-style:header theme=startup -->
[![PyPI version](https://img.shields.io/pypi/v/yourpkg)](https://pypi.org/project/yourpkg/)
<!-- /readme-style:header -->
```

Content between markers is regenerated on every run, so edit outside them. Layout pieces that
wrap your content use single generated lines, such as
`<div align="center"><!-- readme-style:center -->`, rebuilt from scratch each time. Switching
themes gives exactly the result of styling the original with the new theme, and later runs
reuse the theme recorded in the markers, so `--check` needs no config.

## Configuration

Optional. Use `[tool.readme-style]` in `pyproject.toml`, or the same keys at the top level of
`.readme-style.toml`:

```toml
[tool.readme-style]
theme = "startup"
repo = "owner/name"                            # when there is no GitHub remote
logo = "docs/brand/logo.svg"                   # or false to hide it
badges = ["package", "ci", "license", "ruff"]  # groups or badge ids
exclude-badges = ["downloads"]
toc = "details"                                # none, list or details
collapse = ["changelog", "faq"]                # section titles to fold (regular expressions)
badge-style = "flat"                           # flat, flat-square, for-the-badge, plastic, social
```

Every theme option can be overridden: `align`, `header-position`, `logo-width`,
`picture-logos`, `tagline-from-metadata`, `max-badges`, `nav`, `nav-max`, `divider`,
`toc-depth`, `toc-min-sections`, `toc-title`, `callouts`, `collapse-min-lines` and
`collapse-label`. Badge groups are `package`, `downloads`, `runtime`, `ci`, `coverage`,
`docs`, `license`, `tooling`, `research` and `community`.

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

## Roadmap

- An optional LLM layer for extras that need judgment, such as a tagline or a Mermaid
  architecture diagram, written once into a frozen region and never touching your text. Any
  OpenAI-compatible endpoint, including a local model.
- Generated SVG banners per theme, in light and dark variants.
- A `cli` theme, and custom themes defined in config.
- `npx readme-style`.
- A gallery of before and after on well-known repositories.

## Development

```bash
python -m venv .venv && .venv/bin/pip install -e '.[dev]'
.venv/bin/pytest
.venv/bin/python scripts/build_examples.py         # refresh examples/ after a change
.venv/bin/python scripts/smoke.py path/to/repos/*/  # every theme over real READMEs
```

## License

MIT
