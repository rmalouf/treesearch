# Installation

Treesearch requires Python 3.12+ and is currently installed from source using maturin.

## Prerequisites

### Python 3.12+

Ensure you have Python 3.12 or later installed:

```bash
python --version
```

### Rust Toolchain

Since treesearch is built with Rust, you'll need the Rust toolchain installed:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Or visit [rustup.rs](https://rustup.rs/) for installation instructions.

## Installation from Source

### 1. Clone the Repository

```bash
git clone https://github.com/rmalouf/treesearch.git
cd treesearch
```

### 2. Install with maturin

Using uv (recommended):

```bash
# Install uv if you don't have it
curl -LsSf https://astral.sh/uv/install.sh | sh

# Install in development mode
uv pip install -e .
```

Or using pip and maturin directly:

```bash
# Install maturin
pip install maturin

# Build and install in development mode
maturin develop --release
```

### 3. Verify Installation

```python
import treesearch
print(treesearch.__doc__)
```

## Installing Documentation Dependencies

To build and view this documentation locally:

```bash
uv pip install --group docs
```

Then serve the documentation:

```bash
mkdocs serve
```

Visit `http://127.0.0.1:8000` in your browser.

## Development Setup

For development work:

```bash
# Install dev dependencies
uv pip install --group dev

# Run tests
pytest

# Check Rust code
cargo check

# Run Rust tests
cargo test
```

## Next Steps

- [Quick Start Tutorial](quickstart.md) - Get started with your first search
- [Query Language](../guide/query-language.md) - Learn the query syntax
