# Changelog

All notable changes to this project will be documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning follows [Semantic Versioning](https://semver.org/).








## [0.1.8] - 2026-04-19

### Bug Fixes

- feat: add __dir__ for IPython tab completion [fix]

## [0.1.7] - 2026-04-19

### Bug Fixes

- feat: add support for reading file by Rust [fix]

## [0.1.6] - 2026-04-18

### Bug Fixes

- Merge pull request #3 from andree0/f/py-object [fix]

feat: add to_object method [fix]

## [0.1.5] - 2026-04-18

### Bug Fixes

- fix: build wheels [fix]

## [0.1.4] - 2026-04-18

### Bug Fixes

- Merge branch 'main' of github.com:andree0/fast-xml-flattener [fix]

## [0.1.3] - 2026-04-18

### Bug Fixes

- Merge branch 'main' of github.com:andree0/fast-xml-flattener [fix]

## [0.1.2] - 2026-04-18

### Bug Fixes

- fix: release workflow [fix]

## [0.1.0] - 2026-04-18

### New Features

- Initial release: `to_json`, `to_flatten_json`, `to_dict`, `to_flatten_dict`, `to_csv`, `to_parquet`
- Single-pass Rust parser using `quick-xml` 0.39 with `Event::GeneralRef` support
- GIL-free string/CSV/Parquet outputs via `py.detach()`
- Direct `PyDict` construction for dict variants — no JSON round-trip
- xmltodict-compatible semantics (`@attr`, `#text`, auto-list for repeated tags)
