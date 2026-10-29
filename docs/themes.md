# Themes

A theme says which pieces to add and how to lay them out. GitHub strips custom CSS, so the visual
range comes from layout (centering through `align`), badge style and colors, navigation, folded
sections and callouts. Each theme is a TOML file in
[`crates/readme-style-core/themes`](../crates/readme-style-core/themes) that lists only what differs
from [`base.toml`](../crates/readme-style-core/themes/base.toml).

`readme-style --list-themes` lists the built-in themes and those your configuration defines.

## Built-in themes

### minimal

Pure Markdown, left aligned: badges under the title (flat-square), a contents list once there are
four sections, GitHub alerts. No HTML, so it reads well anywhere Markdown is rendered.

### startup

A centered hero: the logo (with its dark-mode variant), the title, the tagline (the package
description when the README has none), large `for-the-badge` badges, a bold nav bar of the main
sections and a divider. Long changelog, news and FAQ sections fold.

### ml-research

A paper-style header: arXiv, project page, Hugging Face (models, datasets, demos) and Colab badges
for links that are about this project, then runtime and license; a nav bar and a folded table of
contents. Long news and changelog sections fold.

### cli

For command-line tools: a centered header, badges in one monochrome palette, and navigation as
`<kbd>` key caps. Long changelog and FAQ sections fold.

### docs

For long READMEs: a three-level table of contents, back-to-top links after long sections, a small
logo, and folded release notes.

See every theme applied to the same projects in [examples](../examples/).

## Overriding a theme

Any setting can be overridden from the configuration, with the same tables as the theme files:

```toml
theme = "startup"

[badges]
style = "flat"
exclude = ["downloads"]

[nav]
max = 4
```

## Defining a theme

A configuration can define themes of its own. They extend `base` unless they say otherwise, and
cannot reuse a built-in name:

```toml
theme = "brand"

[themes.brand]
extends = "startup"
description = "Startup, in our colors"

[themes.brand.badges]
style = "flat-square"
label-color = "1f2328"
color = "ff6600"
```

The README records the theme in its markers, so `readme-style --check` keeps using it without
being told, as long as the configuration still defines it.

## Adding a built-in theme

1. Add `crates/readme-style-core/themes/NAME.toml` with a `description` and the settings that
   differ from `base.toml`.
2. Register it in `BUILTIN` in `crates/readme-style-core/src/theme.rs`.
3. Run `cargo xtask examples` and review the new `examples/*/README.NAME.md` files.
4. Run `cargo xtask corpus fetch` once and `cargo xtask corpus run`: it must report no refusals,
   no instability and no theme-switch drift.
5. Describe it in this file and in the README's theme table.
