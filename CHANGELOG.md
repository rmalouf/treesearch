# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.1] - 2026-08-11

### Fixed
- Wheels are now built with an explicit abi3 floor of Python 3.12. 
- `pip install treesearch-ud[viz]` now installs spaCy. 

### Changed
- `load()` raises `TypeError` instead of `ValueError` for a bad `source` type

### Development
- Update GitHub Actions workflows
- `RELEASING.md` documents the workflows and release process
- Lint cleanly under ruff 0.16
- Upgraded locked dependencies (notably textual 7.5 to 8.2)

## [0.2.0] - 2026-01-21

### Added
- `Treebank.filter(pattern)` method for efficient existence checks (stops after first match per tree)
- Tree visualization with displaCy via `render()` and `to_displacy()` functions
- Optional `viz` extras for spaCy dependency
- Regex support for constraint matching with automatic anchoring

### Performance
- Improved multi-threaded Python performance by releasing GIL during expensive operations
- Python bindings now compatible with free-threaded Python 3.13+ (PEP 703)

## [0.1.0] - 2025-12-29

Initial release.

[0.2.1]: https://github.com/rmalouf/treesearch/releases/tag/v0.2.1
[0.2.0]: https://github.com/rmalouf/treesearch/releases/tag/v0.2.0
[0.1.0]: https://github.com/rmalouf/treesearch/releases/tag/v0.1.0
