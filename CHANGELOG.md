# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- EXCEPT blocks for negative existential queries (reject matches where condition is true)
- OPTIONAL blocks for optional variable binding (extend matches if possible)
- Cross-product semantics for multiple OPTIONAL blocks
- Validation for unique variable names across extension blocks

### Fixed
- Bug where negated labeled edges didn't register the source variable

## [0.1.0] - 2025-12-23

Initial release.

[Unreleased]: https://github.com/rmalouf/treesearch/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/rmalouf/treesearch/releases/tag/v0.1.0
