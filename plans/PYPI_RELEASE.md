# PyPI Release Checklist

**Version**: 0.1.0
**Last Updated**: December 2025

---

## Pre-Release Status

### ✅ Already Done
- [x] Package metadata in `pyproject.toml` (name, description, author, license, keywords, classifiers)
- [x] README.md suitable for PyPI landing page
- [x] LICENSE file (MIT)
- [x] Version in Cargo.toml (0.1.0)
- [x] Package builds successfully (`maturin build --release`)
- [x] Version attribute exports correctly (`treesearch.__version__` → "0.1.0")
- [x] CHANGELOG.md created with v0.1.0 entry (dated 2025-12-23)

### 🔧 Need to Do

1. **Test on TestPyPI first**
   ```bash
   # Build wheel
   maturin build --release

   # Publish to TestPyPI
   maturin publish --repository testpypi

   # Test install from TestPyPI
   pip install --index-url https://test.pypi.org/simple/ treesearch

   # Verify it works
   python -c "import treesearch; print(treesearch.__version__)"
   ```

2. **Verify installation works**
   - Test in fresh virtual environment
   - Import package: `import treesearch`
   - Run basic example from README
   - Check version: `treesearch.__version__`

3. **When ready, publish to real PyPI**
   ```bash
   maturin publish
   ```

---

## PyPI Account Setup

Need PyPI credentials for publishing:
- PyPI account: https://pypi.org/account/register/
- TestPyPI account: https://test.pypi.org/account/register/
- API tokens (recommended over password):
  - Generate at: https://pypi.org/manage/account/token/
  - Store in `~/.pypirc` or use with `maturin publish --token`

---

## Publishing Commands

### Test Build Locally
```bash
# Build wheel
maturin build --release

# Check what's in the wheel
unzip -l target/wheels/treesearch-0.1.0-*.whl

# Install locally to test
pip install target/wheels/treesearch-0.1.0-*.whl
```

### Publish to TestPyPI
```bash
# First time: create TestPyPI token at https://test.pypi.org
maturin publish --repository testpypi

# Or with token directly
maturin publish --repository testpypi --token YOUR_TOKEN
```

### Test Installation from TestPyPI
```bash
# Create fresh virtual environment
python -m venv test_env
source test_env/bin/activate

# Install from TestPyPI
pip install --index-url https://test.pypi.org/simple/ treesearch

# Test it works
python -c "import treesearch; print(treesearch.__version__)"

# Run quick example
python << 'EOF'
import treesearch
pattern = treesearch.parse_query('MATCH { V [upos="VERB"]; }')
print("Pattern compiled successfully:", pattern)
EOF
```

### Publish to Real PyPI
```bash
# When everything tests OK
maturin publish

# Or with token
maturin publish --token YOUR_TOKEN
```

---

## Post-Release Tasks

### After Publishing to PyPI

1. **Update README.md**
   - Change installation instructions to show `pip install treesearch`
   - Keep "From Source" section but make it secondary

2. **Create Git tag**
   ```bash
   git tag -a v0.1.0 -m "Release version 0.1.0"
   git push origin v0.1.0
   ```

3. **Create GitHub Release**
   - Go to https://github.com/rmalouf/treesearch/releases
   - Create release from tag v0.1.0
   - Use CHANGELOG content as release notes

4. **Announce**
   - Update project documentation to reference PyPI package
   - Consider announcing on relevant linguistics/NLP forums if appropriate

---

## Optional (Recommended)

### CHANGELOG.md

✅ Already created with v0.1.0 release notes (dated 2025-12-23)

### Badge for README

Add to top of README.md:
```markdown
[![PyPI version](https://badge.fury.io/py/treesearch.svg)](https://pypi.org/project/treesearch/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
```

---

## Troubleshooting

### Common Issues

**Build fails on other platforms**:
- Maturin supports cross-compilation
- Can use GitHub Actions to build wheels for Linux/Windows/macOS
- See: https://www.maturin.rs/github_actions

**Version mismatch**:
- Ensure `Cargo.toml` version matches intended release
- Maturin reads version from `Cargo.toml`

**Missing dependencies**:
- Check that `polars>=1.35.2` is available on PyPI
- Verify all classifiers are valid

**Large wheel size**:
- Check `[profile.release]` settings in Cargo.toml
- Consider `strip = true` for smaller binaries
- Current settings include `debug = true` (for profiling) - could remove

---

## Future Release Process

For subsequent releases (0.2.0, etc.):

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Run all tests: `cargo test && pytest`
4. Build and test locally
5. Publish to TestPyPI
6. Verify TestPyPI installation
7. Publish to PyPI
8. Tag release in git
9. Create GitHub release

---

## References

- Maturin documentation: https://www.maturin.rs/
- PyPI publishing guide: https://packaging.python.org/guides/distributing-packages-using-setuptools/
- TestPyPI: https://test.pypi.org/
- Semantic Versioning: https://semver.org/
