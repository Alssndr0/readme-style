tidy-cli
========

<!-- readme-style:header theme=minimal -->
[![npm version](https://img.shields.io/npm/v/tidy-cli?logo=npm&style=flat-square)](https://www.npmjs.com/package/tidy-cli) [![CI](https://img.shields.io/github/actions/workflow/status/acme/tidy-cli/ci.yml?label=CI&logo=github&style=flat-square)](https://github.com/acme/tidy-cli/actions/workflows/ci.yml) [![License: MIT](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
<!-- /readme-style:header -->

Format and lint the Markdown tables in your docs from the terminal.

`tidy` aligns columns, normalizes separators and flags rows with the wrong number of
cells, so tables stay readable in plain text and render correctly on GitHub.

<!-- readme-style:toc theme=minimal -->
**Contents**

- [Install](#install)
- [Usage](#usage)
- [Options](#options)
- [FAQ](#faq)
- [Contributing](#contributing)
- [License](#license)
<!-- /readme-style:toc -->

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
