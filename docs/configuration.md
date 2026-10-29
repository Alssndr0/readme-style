# Configuration

Configuration is optional. readme-style looks for it from the README's folder up to the repository
root (the nearest folder with `.git` or `.github`), and the first match wins:

| File | Where the settings go |
|------|-----------------------|
| `.readme-style.toml` | the top level |
| `pyproject.toml` | `[tool.readme-style]` |
| `Cargo.toml` | `[package.metadata.readme-style]` or `[workspace.metadata.readme-style]` |
| `package.json` | a `"readme-style"` object |

## Which theme is used

1. `--theme` on the command line.
2. `theme` in the configuration.
3. The theme recorded in the README's own markers by an earlier run.
4. `minimal`.

## Top-level keys

| Key | Type | Meaning |
|-----|------|---------|
| `theme` | string | The theme to apply: a built-in one or one defined under `themes`. |
| `repo` | `"owner/name"` | The GitHub repository, when there is no GitHub `origin` remote and no repository URL in the metadata. |
| `themes` | table of tables | Themes of your own; see [themes.md](themes.md). |
| `ai` | table | The model `readme-style suggest` asks; see below. Styling ignores it. |

Every other key is a theme table, merged over the selected theme. Unknown keys are errors, reported
with the file they are in.

## Theme settings

The defaults are those of `base.toml`; each built-in theme changes some of them.

### `[layout]`

| Key | Default | Values |
|-----|---------|--------|
| `align` | `"left"` | `left`, `center`: centering wraps the logo, title, tagline and header. |
| `header-position` | `"after-title"` | `after-title`, or `after-tagline` (also after badge rows already there). |
| `divider` | `false` | A horizontal rule under the header. |
| `tagline-from-metadata` | `false` | Show the package description under the title when the README has no tagline and does not already say it. |

### `[logo]`

| Key | Default | Meaning |
|-----|---------|---------|
| `show` | `false` | Put the project's logo above the title (unless the README already opens with an image). |
| `width` | `120` | Width in pixels. |
| `dark-variants` | `true` | Give a logo the README already shows a dark-mode variant through `<picture>`. |
| `path` | detected | The logo file, relative to the README. Its dark variant is found next to it (`logo-dark.svg`, `logo_white.png` and so on). |

Without `path`, readme-style looks for files whose name contains `logo` in `.`, `assets`, `images`,
`img`, `media`, `static`, `.github`, `docs` and similar folders, and pairs light and dark variants
by name: `dark`, `white`, `inverted` and `on-dark` mean the dark-mode file.

### `[badges]`

| Key | Default | Meaning |
|-----|---------|---------|
| `include` | `["package", "ci", "license"]` | Groups or badges to show, in order. |
| `exclude` | `[]` | Groups or badges to leave out. |
| `style` | `"flat-square"` | `flat`, `flat-square`, `for-the-badge`, `plastic`, `social`. |
| `max` | `6` | At most this many badges. |
| `label-color` | none | Hex color for every badge's label. |
| `color` | none | Hex color for every badge's message. |

Groups and the badges in them:

| Group | Badges | Needs |
|-------|--------|-------|
| `package` | `pypi`, `npm`, `crates`, `go` | Evidence the package is published: a publish step in a workflow, or an install command in the README. |
| `downloads` | `pypi-downloads`, `npm-downloads`, `crates-downloads` | The same. |
| `runtime` | `python`, `node`, `rust`, `go-version` | `requires-python` or classifiers, `engines.node`, `rust-version`, the `go` directive. |
| `ci` | `ci` | A GitHub repository and a workflow that runs on pushes or pull requests and is named like tests, CI, build or lint. |
| `coverage` | `codecov` | A Codecov configuration or the Codecov action. |
| `docs` | `docs`, `docsrs` | A Read the Docs configuration or a documentation URL; docs.rs for published crates. |
| `license` | `license` | A license in the metadata or a recognized license file. |
| `tooling` | `ruff`, `black`, `uv`, `pre-commit` | Their configuration or lock files. |
| `research` | `arxiv`, `paper`, `project-page`, `hf-models`, `hf-datasets`, `hf-spaces`, `colab` | Links in the README that are about this project: in the intro, described by their own text, or (Hugging Face) under the GitHub owner's name. |
| `community` | `discord` | A Discord invite in the README. |

A README that already shows badges before its first section keeps them and gets none added.

### `[nav]`

| Key | Default | Meaning |
|-----|---------|---------|
| `show` | `false` | A row of links to the main sections (skipped when the intro already has one). |
| `max` | `6` | At most this many links; features, install, usage, docs, results, citation, contributing and license come first. |
| `style` | `"bold"` | `bold`, `plain`, or `kbd` (each link in a key cap). |
| `separator` | `" · "` | Between links. |

### `[toc]`

| Key | Default | Meaning |
|-----|---------|---------|
| `style` | `"list"` | `none`, `list`, or `details` (folded). |
| `depth` | `2` | Heading levels listed, from 1 to 6. |
| `min-sections` | `4` | Only once the README has this many sections. |
| `title` | `"Contents"` | Shown above the list. |

A README with its own table of contents (a heading named like one, `[TOC]`, or a list of links to
its sections) gets none added. Subsections of changelog, news and FAQ sections are not listed.

### `[callouts]`

| Key | Default | Meaning |
|-----|---------|---------|
| `enabled` | `true` | `> **Note:** ...`, `**Warning:** ...`, `Tip: ...` (note, tip, important, warning, caution) become GitHub alerts. |

### `[collapse]`

| Key | Default | Meaning |
|-----|---------|---------|
| `sections` | `[]` | Regular expressions, matched case-insensitively against whole section titles (leading emoji and punctuation ignored). |
| `min-lines` | `12` | Only sections with at least this many non-blank lines fold. |
| `label` | `"Click to expand"` | The summary line; `{title}` is replaced by the section title. |

### `[back-to-top]`

| Key | Default | Meaning |
|-----|---------|---------|
| `enabled` | `false` | A right-aligned link back to the title after long sections. |
| `min-lines` | `25` | Only after sections with at least this many non-blank lines. |
| `label` | `"Back to top"` | The link text. |

## `[ai]`: the model for `readme-style suggest`

Only `readme-style suggest` reads this table; styling never uses a model. Flags (`--provider`,
`--model`, `--base-url`) win over it, and it wins over the environment.

| Key | Default | Meaning |
|-----|---------|---------|
| `provider` | detected | `anthropic`, or `openai` for any server with the OpenAI chat-completions API. Without it: `openai` when a base URL is set, then `anthropic` when `ANTHROPIC_API_KEY` or `ANTHROPIC_AUTH_TOKEN` is set, then `openai` when `OPENAI_BASE_URL` or `OPENAI_API_KEY` is. |
| `model` | `claude-opus-5-5` for Anthropic; the server's only model otherwise | The model name. |
| `base-url` | `https://api.anthropic.com`, or `OPENAI_BASE_URL`, or `https://api.openai.com/v1` | Where to send requests; `ANTHROPIC_BASE_URL` also works for Anthropic. |
| `api-key-env` | `ANTHROPIC_API_KEY` or `OPENAI_API_KEY` | The environment variable holding the key. Keys are never read from the configuration file. |

```toml
[ai]                                   # a local vLLM server
provider = "openai"
base-url = "http://127.0.0.1:8000/v1"
```
