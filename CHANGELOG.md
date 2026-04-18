# Changelog

All notable changes to this project will be documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning follows [Semantic Versioning](https://semver.org/).

## [0.1.0] - 2026-04-18

### New Features

- Initial release: `to_json`, `to_flatten_json`, `to_dict`, `to_flatten_dict`, `to_csv`, `to_parquet`
- Single-pass Rust parser using `quick-xml` 0.39 with `Event::GeneralRef` support
- GIL-free string/CSV/Parquet outputs via `py.detach()`
- Direct `PyDict` construction for dict variants — no JSON round-trip
- xmltodict-compatible semantics (`@attr`, `#text`, auto-list for repeated tags)
