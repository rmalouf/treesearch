# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] - 2026-09-23

### Added
- `treesearch-tui` workspace crate: terminal browser for refining queries (`cargo run -p treesearch-tui -- GLOB [QUERY]`)
- `Treebank::search_with` takes a shared `Progress` for progress counters and cancellation
- `python` cargo feature (on by default) so the core crate can build without pyo3

### Changed
- **Breaking:** Variables used in edge and precedence constraints must be declared
- **Breaking:** `EXCEPT` and `OPTIONAL` blocks can no longer redeclare `MATCH` variables (they may still use them in edge and precedence constraints); node constraints belong in `MATCH`
- New variables in `EXCEPT` and `OPTIONAL` blocks never bind to a word already bound by `MATCH`. Previously they avoided only the `MATCH` variables mentioned in the block, so adding a vacuous `S [];` could change results
- Pattern variables keep their declaration order (`BasePattern::with_constraints` takes a `Vec`)

### Fixed
- Regexes with top-level alternation are now fully anchored: `/h|u/` no longer matches `"help"`, and `-/obj|iobj/->` no longer matches `obj:lvc`
- Free-threaded wheels (cp313t, cp314t) are published again for linux x86_64 and macOS arm64

### Documentation
- Rewrote the query language reference to match the implementation (alternation and grouping, `_`, negative edges, `!=` on absent features, escaping, variable scoping)
- Fixed reversed `aux` edges in the progressive and modal examples, and the description of `!->` in `API.md`

### Development
- Simplified GitHub Actions workflows. Docs are built on every push and deployed to Pages manually

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

[Unreleased]: https://github.com/rmalouf/treesearch/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/rmalouf/treesearch/releases/tag/v0.3.0
[0.2.1]: https://github.com/rmalouf/treesearch/releases/tag/v0.2.1
[0.2.0]: https://github.com/rmalouf/treesearch/releases/tag/v0.2.0
[0.1.0]: https://github.com/rmalouf/treesearch/releases/tag/v0.1.0
