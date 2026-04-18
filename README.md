# fast-xml-flattener

[![PyPI](https://img.shields.io/pypi/v/fast-xml-flattener)](https://pypi.org/project/fast-xml-flattener/)
[![Python](https://img.shields.io/pypi/pyversions/fast-xml-flattener)](https://pypi.org/project/fast-xml-flattener/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/built%20with-Rust-orange?logo=rust)](https://www.rust-lang.org/)
[![CI](https://github.com/andree0/fast-xml-flattener/actions/workflows/ci.yml/badge.svg)](https://github.com/andree0/fast-xml-flattener/actions)

**Flatten nested XML into CSV, JSON, Parquet, or Python dicts — in milliseconds, not seconds.**

`fast-xml-flattener` is a Rust-powered Python library that converts XML documents into flat, analysis-ready representations. It uses a zero-copy streaming parser and builds output structures in a single tree walk, with no intermediate `serde_json::Value` or DOM allocation. The result: throughput that leaves pure-Python parsers far behind.

---

## Why fast-xml-flattener?

| Library | XML → flat dict (10 MB) | XML → CSV (10 MB) | Notes |
|---|---|---|---|
| **fast-xml-flattener** | **~80 ms** | **~90 ms** | Rust, single-pass, zero DOM |
| `xmltodict` + manual flatten | ~950 ms | ~1 200 ms | Pure Python, full DOM |
| `lxml` + XPath flatten | ~420 ms | ~530 ms | C binding, but two-pass |

*Benchmarked on Apple M2 Pro / Linux x86-64, CPython 3.13, 10 MB synthetic XML with 50 k records.*

The speed advantage grows with document size: the Rust parser processes data at memory-bandwidth speed while holding the GIL only for dict-returning functions (`to_dict`, `to_flatten_dict`). All other outputs (`to_json`, `to_csv`, `to_parquet`) release the GIL entirely, so they compose well with multi-threaded Python workloads.

---

## Features

- Flatten nested XML into **JSON**, **flatten-JSON**, native Python **dict**, **flatten-dict**, **CSV**, or **Parquet**
- **Single-pass** streaming parser — no DOM, no intermediate `Value` allocation
- **GIL-free** for string/CSV/Parquet outputs — safe to use from thread pools
- **xmltodict-compatible** semantics: `@attr`, `#text`, auto-list for repeated tags
- Namespace stripping, CDATA, entity references, comments — all handled correctly
- Supports Python 3.9+

## Output Formats

| Function | Returns | Description |
|---|---|---|
| `to_json(xml)` | `str` | 1:1 JSON preserving XML structure (`@attr`, `#text`) |
| `to_flatten_json(xml, separator=".")` | `str` | Flat JSON with dot-notation keys (`user.address.city`) |
| `to_dict(xml)` | `dict` | 1:1 nested Python dict — built directly in Rust, no JSON round-trip |
| `to_flatten_dict(xml, separator=".")` | `dict` | Flat Python dict with dot-notation keys |
| `to_csv(xml, include_attrs=True)` | `str` | Tabular CSV, one row per XML record |
| `to_parquet(xml, path, include_attrs=True)` | `None` | Columnar Parquet file for big-data workflows |

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
```

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
uv run pytest            # Python integration tests (52 cases)
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
