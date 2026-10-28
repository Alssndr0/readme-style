# quickparse

A tiny, dependency-free parser for config files that never surprises you.

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

> **Note:** quickparse needs Python 3.10 or newer.

## Usage

```python
import quickparse

config = quickparse.load("settings.ini")
print(config["server"]["port"])
```

Warning: `quickparse.loads` does not expand environment variables by default.

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
