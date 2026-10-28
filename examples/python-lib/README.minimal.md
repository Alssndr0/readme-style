# quickparse

<!-- readme-style:header theme=minimal -->
[![PyPI version](https://img.shields.io/pypi/v/quickparse?logo=pypi&logoColor=white&style=flat-square)](https://pypi.org/project/quickparse/) [![CI](https://img.shields.io/github/actions/workflow/status/acme/quickparse/ci.yml?label=CI&logo=github&style=flat-square)](https://github.com/acme/quickparse/actions/workflows/ci.yml) [![License: MIT](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
<!-- /readme-style:header -->

A tiny, dependency-free parser for config files that never surprises you.

quickparse reads INI, TOML-ish and `.env` files into plain dictionaries, keeps comments
around for round trips, and tells you exactly which line broke.

<!-- readme-style:toc theme=minimal -->
**Contents**

- [Features](#features)
- [Installation](#installation)
- [Usage](#usage)
  - [Command line](#command-line)
- [Configuration](#configuration)
- [Changelog](#changelog)
- [Contributing](#contributing)
- [License](#license)
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
