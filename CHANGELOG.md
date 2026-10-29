# Changelog

All notable changes are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- `readme-style suggest tagline` and `readme-style suggest diagram` ask a model (Anthropic, or any
  OpenAI-compatible server such as a local vLLM) for a tagline or a Mermaid architecture diagram and
  print the suggestions. Configured with flags, the environment, or an `[ai]` table (provider and
  model only: a repository cannot choose where your key is sent).
- A before-and-after gallery in `examples/`: four projects, every theme, what changed and the diff.
- A browser playground (`cargo xtask playground`): the engine compiled to WebAssembly, with a
  sanitized, sandboxed preview, the diff, badge evidence, the examples, and project folders read in
  the page.

### Changed

- When a nav bar has more sections than room, it prefers where to start and how to use the project
  (install, usage, configuration, API), then benchmarks, results and FAQ, then the project's own
  sections, and last contributing, license, sponsors and similar closing sections.

### Fixed

- Line endings are kept line by line: a lone carriage return no longer crashes styling or slips past
  the guard, and one CRLF line in an LF file stays as it was instead of converting the whole file.
- YAML front matter (Hugging Face model cards, GitHub's metadata table) stays first and untouched.
- Text after a callout lead-in keeps its meaning (`**Note:** # of retries` no longer becomes a
  heading); callouts that cannot be converted safely are left as they are.
- Table of contents and nav labels are built from the parsed heading, so code spans such as
  `` `Vec<T>` `` survive.
- `--write` follows a symlinked README instead of replacing the link, and keeps hard links;
  `--stdout` writes the README byte for byte.
- A dark logo variant works for file names with non-ASCII characters; custom theme names are
  checked; an unrelated broken `package.json` or manifest no longer stops styling; license files
  are found by any capitalization, with the same result on every operating system.
- A lone carriage return followed by an inserted blank line no longer reads as one CRLF (which
  merged paragraphs); new lines take the file's most common ending, lone CR included, and every
  result is checked to read back as the styled text.
- `--write` creates its temporary file under a fresh name, so a file planted at the old fixed name
  is never followed; it no longer writes through a symlinked README that points outside the
  repository; and a hard-linked README is left intact when the disk is full or a size limit is hit.
- `suggest --base-url` ignores the `[ai]` table, so a repository cannot choose which of your keys
  is sent to the server you named.
- Callouts: text after the lead-in that GitHub would read as a heading underline (`--`) or a table
  row (`:--`) is escaped, and so is text that looks like a link or footnote definition instead of
  being refused; a line break after the text is kept, and the guard now compares line breaks.
- Front matter with keys such as `C++:` is recognized; `---`, a `# Heading`, `---` is not front
  matter.
- Headings made only of an image or HTML get no empty table of contents or nav entry.
- A README is read and written only when it lies in the repository its path is in, after
  following links to the file or to a folder above it, and never inside `.git`: a link in a
  cloned repository can no longer make `--check` print, `suggest` send, or `--write` rewrite a
  file elsewhere. A read-only README is not replaced, and a write keeps the file's owner and group.
- `> **Warning:**` followed by a list or a heading in the same quote becomes an alert holding that
  block, instead of being refused. A definition-like opening keeps its reference link
  (`[x]\:`), and a one-column table row with a trailing pipe is escaped.
- Section titles with emoji are recognized for the nav bar, the table of contents and folding,
  including emoji that end with a variation selector (`❤️`), count as letters (`ℹ️`), are keycaps
  (`1️⃣`) or are written as shortcodes (`:rocket:`).
- A lead-in alone in a quote becomes an alert holding the next block only when that block always
  shows (a list, heading, code, quote, table or rule): GitHub shows `[!NOTE]` as text when an alert
  holds only a comment or invisible HTML.
- A link is followed only if it stays in the repository it sits in, so a folder link to another
  repository is refused too, and `..` from inside a nested repository works. A README in a folder
  that cannot be written is rewritten in place.
- An HTML block that leaves a `<details>`, `<div>` or table open, or holds a folded section, a list
  or a table, is no longer taken for part of the hero or the tagline, so the header is never placed
  inside it; a `<summary>` titled like a table of contents counts as the README's own.

## [0.2.0] - 2026-10-29

### Changed

- Rewritten in Rust: one static binary, parsed with comrak (GitHub's parser), a few milliseconds
  per README.
- Themes are TOML files with inheritance; any setting can be overridden from configuration, and
  configurations can define themes of their own.
- Configuration is also read from `[package.metadata.readme-style]` in `Cargo.toml` and the
  `"readme-style"` key of `package.json`.

### Added

- The `cli` theme (monochrome badges, key-cap navigation) and the `docs` theme (three-level table
  of contents, back-to-top links).
- `--format json` for machine-readable reports, with the evidence for every badge.
- Diagnostics for broken markers, with line numbers; refusals name the line that would change.
- Distribution through crates.io, PyPI and npm, plus release binaries for the GitHub Action.
- A README with its own nav row gets no second one; badges under a table of contents count as the
  README's own.

### Fixed

- Many edge cases found by property testing: raw HTML that swallows markers, indented code, lists
  and footnotes next to regions, regions left by another theme, and footnote definitions the parser
  moves.

## [0.1.0] - 2026-10-28

The Python prototype: the guard, markers, badges, table of contents, nav bar, callouts, folded
sections, logos and the `minimal`, `startup` and `ml-research` themes. Never published.

[Unreleased]: https://github.com/Alssndr0/readme-style/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/Alssndr0/readme-style/releases/tag/v0.2.0
[0.1.0]: https://github.com/Alssndr0/readme-style/commit/7968121
