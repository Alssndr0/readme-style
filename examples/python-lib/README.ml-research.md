<div align="center"><!-- readme-style:center -->

<!-- readme-style:logo theme=ml-research -->
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/logo-dark.svg">
  <img src="assets/logo.svg" alt="quickparse logo" width="120">
</picture>
<!-- /readme-style:logo -->

# quickparse

A tiny, dependency-free parser for config files that never surprises you.

<!-- readme-style:header theme=ml-research -->
![Python 3.10+](https://img.shields.io/badge/python-3.10%2B-3776AB?logo=python&logoColor=white&style=flat) [![License: MIT](https://img.shields.io/badge/license-MIT-blue?style=flat)](LICENSE)

**[Features](#features)** · **[Installation](#installation)** · **[Usage](#usage)** · **[Configuration](#configuration)** · **[Changelog](#changelog)** · **[Docs](https://quickparse.readthedocs.io)**
<!-- /readme-style:header -->

</div><!-- /readme-style:center -->

quickparse reads INI, TOML-ish and `.env` files into plain dictionaries, keeps comments
around for round trips, and tells you exactly which line broke.

<!-- readme-style:toc theme=ml-research -->
<details>
<summary><b>Table of Contents</b></summary>

- [Features](#features)
- [Installation](#installation)
- [Usage](#usage)
  - [Command line](#command-line)
- [Configuration](#configuration)
- [Changelog](#changelog)
- [Contributing](#contributing)
- [License](#license)

</details>
<!-- /readme-style:toc -->

## Features

- Zero dependencies, pure Python
- Round-trips comments and blank lines
- Error messages point at the exact line and column
- Typed accessors: `get_int`, `get_bool`, `get_list`

## Installation

```bash
pip install quickparse
```

> [!NOTE]
> quickparse needs Python 3.10 or newer.

## Usage

```python
import quickparse

config = quickparse.load("settings.ini")
print(config["server"]["port"])
```

> [!WARNING]
> `quickparse.loads` does not expand environment variables by default.

### Command line

```bash
quickparse check settings.ini
```

## Configuration

| Option | Default | Description |
|--------|---------|-------------|
| `strict` | `false` | Fail on duplicate keys |
| `interpolate` | `false` | Expand `${VAR}` references |

## Changelog

<details><summary>Click to expand</summary><!-- readme-style:collapse -->

### 1.4.0

- Added `get_list` with custom separators
- Faster parsing of large files

### 1.3.0

- `.env` support
- Better error messages for unterminated quotes

### 1.2.0

- Comments survive `dump`
- Dropped Python 3.8

### 1.1.0

- First stable release

</details><!-- /readme-style:collapse -->

## Contributing

Pull requests are welcome. Please run `pytest` before sending one.

## License

MIT
