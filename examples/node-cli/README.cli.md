<div align="center"><!-- readme-style:center -->

<!-- readme-style:logo theme=cli -->
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/logo-dark.svg">
  <img src="assets/logo-light.svg" alt="tidy-cli logo" width="100">
</picture>
<!-- /readme-style:logo -->

tidy-cli
========

Format and lint the Markdown tables in your docs from the terminal.

<!-- readme-style:header theme=cli -->
[![npm version](https://img.shields.io/npm/v/tidy-cli?logo=npm&labelColor=1f2328&color=3d444d&style=flat-square)](https://www.npmjs.com/package/tidy-cli) [![CI](https://img.shields.io/github/actions/workflow/status/acme/tidy-cli/ci.yml?label=CI&logo=github&labelColor=1f2328&color=3d444d&style=flat-square)](https://github.com/acme/tidy-cli/actions/workflows/ci.yml) ![Node.js 18+](https://img.shields.io/badge/node-18%2B-3d444d?logo=nodedotjs&logoColor=white&labelColor=1f2328&style=flat-square) [![License: MIT](https://img.shields.io/badge/license-MIT-3d444d?labelColor=1f2328&style=flat-square)](LICENSE)

<kbd>[Install](#install)</kbd> <kbd>[Usage](#usage)</kbd> <kbd>[Options](#options)</kbd> <kbd>[FAQ](#faq)</kbd> <kbd>[Contributing](#contributing)</kbd> <kbd>[License](#license)</kbd>
<!-- /readme-style:header -->

</div><!-- /readme-style:center -->

`tidy` aligns columns, normalizes separators and flags rows with the wrong number of
cells, so tables stay readable in plain text and render correctly on GitHub.

## Install

```sh
npm install -g tidy-cli
```

## Usage

```sh
tidy README.md
tidy --check "docs/**/*.md"
```

> [!WARNING]
> `tidy` rewrites files in place. Commit first, or use `--check`.

## Options

| Flag | Description |
|------|-------------|
| `--check` | Exit with code 1 instead of writing |
| `--align <left\|center>` | Column alignment for header cells |
| `--max-width <n>` | Wrap cells longer than `n` characters |

## FAQ

### Does it support GitHub tables?

Yes, including escaped pipes and inline code inside cells.

### Can I run it in CI?

Yes. `tidy --check` exits with code 1 when a table would change, and prints a diff.

### Does it touch anything outside tables?

No. Everything outside a table is left byte for byte as it was.

### Which Node versions work?

Node 18 and newer.

## Contributing

Bug reports and pull requests are welcome. Join the chat on [Discord](https://discord.gg/acme-tidy).

## License

MIT
