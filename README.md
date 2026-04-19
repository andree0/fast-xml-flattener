# fast-xml-flattener

[![PyPI](https://img.shields.io/pypi/v/fast-xml-flattener)](https://pypi.org/project/fast-xml-flattener/)
[![Python](https://img.shields.io/pypi/pyversions/fast-xml-flattener)](https://pypi.org/project/fast-xml-flattener/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/built%20with-Rust-orange?logo=rust)](https://www.rust-lang.org/)
[![CI](https://github.com/andree0/fast-xml-flattener/actions/workflows/ci.yml/badge.svg)](https://github.com/andree0/fast-xml-flattener/actions)
[![codecov rust](https://img.shields.io/codecov/c/github/andree0/fast-xml-flattener?flag=rust&label=coverage%20rust&logo=rust)](https://codecov.io/gh/andree0/fast-xml-flattener)
[![codecov python](https://img.shields.io/codecov/c/github/andree0/fast-xml-flattener?flag=python&label=coverage%20python&logo=python)](https://codecov.io/gh/andree0/fast-xml-flattener)
![Development](https://img.shields.io/badge/Development-Architected%20by%20Human%20%7C%20Implemented%20by%20AI-blueviolet)

**Flatten nested XML into CSV, JSON, Parquet, or Python dicts — in milliseconds, not seconds.**

`fast-xml-flattener` is a Rust-powered Python library that converts XML documents into flat, analysis-ready representations. It uses a zero-copy streaming parser and builds output structures in a single tree walk, with no intermediate `serde_json::Value` or DOM allocation. The result: throughput that leaves pure-Python parsers far behind.

---

## Why fast-xml-flattener?

### XML → flat dict (median of 7 runs, CPython 3.13)

| Library | 0.5 MB | 5.4 MB | 27 MB |
|---|---|---|---|
| **fast-xml-flattener** | **11 ms** | **225 ms** | **1 089 ms** |
| `lxml` + manual flatten | 27 ms | 407 ms | 2 108 ms |
| `xmltodict` + manual flatten | 63 ms | 997 ms | 4 952 ms |

### XML → flat JSON string (median of 7 runs)

| Library | 0.5 MB | 5.4 MB | 27 MB |
|---|---|---|---|
| **fast-xml-flattener** | **13 ms** | **164 ms** | **884 ms** |
| `xmltodict` + `json.dumps` | 93 ms | 1 147 ms | 5 374 ms |

*Dell Vostro i7-1260P, 64 GB RAM, Linux, CPython 3.13. Synthetic XML with nested records (id, user, address, order fields). See [`benches/benchmark.py`](benches/benchmark.py).*

**4–7× faster than `xmltodict`, 2–2.5× faster than `lxml`** across all tested sizes. The gap widens with document size because the Rust parser operates at memory-bandwidth speed with zero DOM allocation. The GIL is held only for dict-returning functions (`to_dict`, `to_flatten_dict`); all other outputs release it entirely, making the library safe to use from thread pools.

---

## Features

- Flatten nested XML into **JSON**, **flatten-JSON**, native Python **dict**, **flatten-dict**, **CSV**, or **Parquet**
- **Dot-notation object access** — navigate parsed XML like `obj.user.address.city` with `XmlObject`
- **File streaming** — pass a `Path` or filename string; Rust reads the file in buffered chunks without loading it into Python memory
- **Single-pass** streaming parser — no DOM, no intermediate `Value` allocation
- **GIL-free** for string/CSV/Parquet outputs — safe to use from thread pools
- **xmltodict-compatible** semantics: `@attr`, `#text`, auto-list for repeated tags
- Namespace stripping, CDATA, entity references, comments — all handled correctly
- Supports Python 3.10+

## Input

Every function accepts XML content **or** a file path — no manual `open()` required:

```python
# XML string
fxf.to_dict("<root><a>1</a></root>")

# pathlib.Path — Rust reads the file in buffered chunks
fxf.to_dict(Path("data.xml"))

# plain str path (does not start with '<')
fxf.to_dict("data.xml")
```

| Input type | Behaviour |
|---|---|
| `str` starting with `<` | Parsed as XML content |
| `str` not starting with `<` | Treated as a file path |
| `pathlib.Path` / `os.PathLike` | Always treated as a file path |

File I/O happens entirely in Rust via a buffered reader — the file is never fully loaded into Python memory.

## Output Formats

| Function | Returns | Description |
|---|---|---|
| `to_json(xml)` | `str` | 1:1 JSON preserving XML structure (`@attr`, `#text`) |
| `to_flatten_json(xml, separator=".")` | `str` | Flat JSON with dot-notation keys (`user.address.city`) |
| `to_dict(xml)` | `dict` | 1:1 nested Python dict — built directly in Rust, no JSON round-trip |
| `to_flatten_dict(xml, separator=".")` | `dict` | Flat Python dict with dot-notation keys |
| `to_csv(xml, include_attrs=True)` | `str` | Tabular CSV, one row per XML record |
| `to_parquet(xml, path, include_attrs=True)` | `None` | Columnar Parquet file for big-data workflows |
| `to_object(xml)` | `XmlObject` | Dot-notation Python object with attribute and text access |

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

# 1:1 JSON string — preserves nesting
result = fxf.to_json(xml)
# '{"user": {"id": "1", "name": "Alice", "address": {"city": "Warsaw", "zip": "00-001"}}}'

# Flattened JSON string with dot-notation keys
flat = fxf.to_flatten_json(xml)
# '{"user.id": "1", "user.name": "Alice", "user.address.city": "Warsaw", "user.address.zip": "00-001"}'

# Native Python dict (1:1 nested) — no JSON round-trip
d = fxf.to_dict(xml)
print(d["user"]["name"])             # Alice
print(d["user"]["address"]["city"])  # Warsaw

# Flattened native Python dict
fd = fxf.to_flatten_dict(xml, separator=".")
print(fd["user.address.city"])       # Warsaw

# CSV — one row per <user> element
csv = fxf.to_csv(xml, include_attrs=True)

# Parquet — ready for pandas / Spark / DuckDB
fxf.to_parquet(xml, path="output.parquet", include_attrs=True)

# Dot-notation object access
obj = fxf.to_object(xml)
print(obj.root.user.name)              # Alice
print(obj.root.user.address.city)      # Warsaw

# All functions also accept a file path — Rust streams the file without
# loading it into Python memory
from pathlib import Path

d = fxf.to_dict(Path("data.xml"))
obj = fxf.to_object("data.xml")        # plain str path works too
```

### XmlObject — dot-notation access

`to_object()` parses XML and returns an `XmlObject` that wraps the result of `to_dict()`. XML parsing is done in Rust; the object layer adds minimal Python overhead.

```python
xml = '''
<catalog>
  <book id="1" lang="en">
    <title>Clean Code</title>
    <author>Robert C. Martin</author>
  </book>
  <book id="2" lang="pl">
    <title>Czysty Kod</title>
    <author>Robert C. Martin</author>
  </book>
</catalog>
'''

obj = fxf.to_object(xml)

# Navigate nested structure with dot notation
books = obj.catalog.book          # list of XmlObject (repeated tag)
print(books[0].title)             # Clean Code
print(books[1].title)             # Czysty Kod

# Access XML attributes via _attrs (no @ prefix)
print(books[0]._attrs)            # {"id": "1", "lang": "en"}
print(books[0]._attrs["lang"])    # en

# Access text content via _text (useful when element has both text and attrs)
print(books[0].title._text)       # Clean Code

# Get the underlying raw dict via .raw
print(books[0].raw)               # {"@id": "1", "@lang": "en", "title": "Clean Code", ...}
```

| Property / access | Returns | Description |
|---|---|---|
| `obj.child_tag` | `XmlObject`, `list[XmlObject]`, or `str` | Child element; list when tag repeats; str for pure-text leaves |
| `obj._attrs` | `dict[str, str]` | XML attributes of this element (keys without `@` prefix) |
| `obj._text` | `str \| None` | Text content (`#text`) of this element |
| `obj.raw` | `dict \| str` | Underlying value from `to_dict()` — str for pure-text leaves |

### Loading Parquet with pandas

```python
import pandas as pd

df = pd.read_parquet("output.parquet")
print(df.head())
```

### Using with DuckDB

```python
import duckdb

duckdb.sql("SELECT * FROM 'output.parquet'").show()
```

---

## Development

### Requirements

- Python 3.10+ (3.13 recommended for development)
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
```

### Tests

```bash
uv run pytest            # Python integration tests (95 cases)
cargo test               # Rust unit tests (25 cases)
uv run ruff check .      # linting
cargo clippy --all-targets -- -D warnings  # Rust linting
```

## Releasing

Releases are fully automated. Append one of these tags anywhere in your commit message (or PR title when squash-merging) to trigger a release:

| Tag | Bump | Example |
|---|---|---|
| `[fix]` | patch (`0.1.0 → 0.1.1`) | `fix null value in CSV output [fix]` |
| `[minor]` | minor (`0.1.0 → 0.2.0`) | `add streaming API [minor]` |
| `[major]` | major (`0.1.0 → 1.0.0`) | `redesign public API [major]` |

The release pipeline then:
1. Bumps `version` in `Cargo.toml` and `pyproject.toml`
2. Prepends an entry to `CHANGELOG.md`
3. Commits (`chore: bump version to X.Y.Z`) and creates a `vX.Y.Z` git tag
4. Builds wheels for Linux x86\_64/aarch64, macOS universal2, Windows x86\_64
5. Publishes to PyPI via OIDC trusted publishing (no secrets needed)
6. Creates a GitHub Release with the changelog entry and wheel artifacts

### One-time PyPI setup (trusted publishing)

1. Go to **PyPI → Your projects → fast-xml-flattener → Publishing → Add a publisher**
2. Set: GitHub owner `andree0`, repo `fast-xml-flattener`, workflow `release.yml`, environment `pypi`
3. On GitHub: **Settings → Environments → New environment** named `pypi`

No API tokens or secrets are required — OIDC handles authentication.

## License

MIT

**Note on Development**: Architected by me, implemented with AI. This project explores high-performance Rust-Python integration through modern AI-assisted engineering.
