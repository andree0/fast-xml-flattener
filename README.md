# fast-xml-flattener

A high-performance XML flattening library written in Rust with Python bindings. Converts nested XML structures into flat representations suitable for data analysis and storage.

## Features

- Flatten nested XML into **CSV**, **JSON**, **Parquet**, **flatten-JSON**, or **dotted-dict** (Python dot-access objects)
- Built in Rust for maximum performance
- Simple Python API via PyO3 bindings
- Supports Python 3.9+

## Output Formats

| Format | Description |
|--------|-------------|
| `json` | 1:1 JSON representation preserving XML structure |
| `flatten_json` | Flat JSON with dot-notation keys (`user.address.city`) |
| `csv` | Flat tabular representation, one row per XML record |
| `parquet` | Columnar format for big data workflows |
| `dotted-dict` | Python object with dot-access attributes (`obj.foo.bar`) |

## Installation

```bash
pip install fast-xml-flattener
```

## Quick Start

```python
import fast_xml_flattener as fxf

xml = """
<root>
  <user>
    <id>1</id>
    <name>Alice</name>
    <address>
      <city>Warsaw</city>
      <zip>00-001</zip>
    </address>
  </user>
</root>
"""

# 1:1 JSON — preserves nesting
result = fxf.to_json(xml)
# {"user": {"id": "1", "name": "Alice", "address": {"city": "Warsaw", "zip": "00-001"}}}

# Flattened JSON with dot-notation keys
flat = fxf.to_flatten_json(xml)
# {"user.id": "1", "user.name": "Alice", "user.address.city": "Warsaw", "user.address.zip": "00-001"}

# CSV
csv = fxf.to_csv(xml)

# Parquet
fxf.to_parquet(xml, path="output.parquet")

# dotted-dict — Python dot-access object
obj = fxf.to_dotted_dict(xml)
print(obj.user.name)          # Alice
print(obj.user.address.city)  # Warsaw
```

## Development

### Requirements

- Python 3.9+ (3.13 recommended for development)
- Rust (stable)
- [maturin](https://github.com/PyO3/maturin)

### Setup with pyenv (recommended)

```bash
# Install pyenv: https://github.com/pyenv/pyenv
pyenv install 3.13
pyenv local 3.13

# Create and activate virtual environment
pyenv virtualenv 3.13 xml-flattener
pyenv activate xml-flattener

# Install uv and dev dependencies
pip install uv
uv pip install -e ".[dev]"

# Install pre-commit hooks
pre-commit install
```

### Setup without pyenv

```bash
python -m venv venv
source venv/bin/activate
pip install uv
uv pip install -e ".[dev]"
pre-commit install
```

### Build

```bash
uv run maturin develop   # development build
maturin build --release  # release wheel
```

### Tests

```bash
uv run pytest
uv run ruff check .
uv run mypy .
```

## License

MIT
