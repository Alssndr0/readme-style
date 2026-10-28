<div align="center"><!-- readme-style:center -->

<!-- readme-style:logo theme=ml-research -->
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/logo-dark.svg">
  <img src="assets/logo-light.svg" alt="tidy-cli logo" width="120">
</picture>
<!-- /readme-style:logo -->

tidy-cli
========

Format and lint the Markdown tables in your docs from the terminal.

<!-- readme-style:header theme=ml-research -->
![Node.js 18+](https://img.shields.io/badge/node-18%2B-339933?logo=nodedotjs&logoColor=white&style=flat) [![License: MIT](https://img.shields.io/badge/license-MIT-blue?style=flat)](LICENSE) [![Discord](https://img.shields.io/badge/Discord-join-5865F2?logo=discord&logoColor=white&style=flat)](https://discord.gg/acme-tidy)

**[Install](#install)** · **[Usage](#usage)** · **[Options](#options)** · **[FAQ](#faq)** · **[Contributing](#contributing)** · **[License](#license)**
<!-- /readme-style:header -->

</div><!-- /readme-style:center -->

`tidy` aligns columns, normalizes separators and flags rows with the wrong number of
cells, so tables stay readable in plain text and render correctly on GitHub.

<!-- readme-style:toc theme=ml-research -->
<details>
<summary><b>Table of Contents</b></summary>

- [Install](#install)
- [Usage](#usage)
- [Options](#options)
- [FAQ](#faq)
- [Contributing](#contributing)
- [License](#license)

</details>
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
