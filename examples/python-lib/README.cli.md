<div align="center"><!-- readme-style:center -->

<!-- readme-style:logo theme=cli -->
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/logo-dark.svg">
  <img src="assets/logo.svg" alt="quickparse logo" width="100">
</picture>
<!-- /readme-style:logo -->

# quickparse

A tiny, dependency-free parser for config files that never surprises you.

<!-- readme-style:header theme=cli -->
[![PyPI version](https://img.shields.io/pypi/v/quickparse?logo=pypi&logoColor=white&labelColor=1f2328&color=3d444d&style=flat-square)](https://pypi.org/project/quickparse/) [![CI](https://img.shields.io/github/actions/workflow/status/acme/quickparse/ci.yml?label=CI&logo=github&labelColor=1f2328&color=3d444d&style=flat-square)](https://github.com/acme/quickparse/actions/workflows/ci.yml) ![Python 3.10+](https://img.shields.io/badge/python-3.10%2B-3d444d?logo=python&logoColor=white&labelColor=1f2328&style=flat-square) [![License: MIT](https://img.shields.io/badge/license-MIT-3d444d?labelColor=1f2328&style=flat-square)](LICENSE)

<kbd>[Features](#features)</kbd> <kbd>[Installation](#installation)</kbd> <kbd>[Usage](#usage)</kbd> <kbd>[Configuration](#configuration)</kbd> <kbd>[Changelog](#changelog)</kbd> <kbd>[Docs](https://quickparse.readthedocs.io)</kbd>
<!-- /readme-style:header -->

</div><!-- /readme-style:center -->

quickparse reads INI, TOML-ish and `.env` files into plain dictionaries, keeps comments
around for round trips, and tells you exactly which line broke.

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

## Contributing

Pull requests are welcome. Please run `pytest` before sending one.

## License

MIT
