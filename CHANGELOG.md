# Changelog

All notable changes are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

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
