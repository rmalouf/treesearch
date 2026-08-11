# Releasing

Maintainer notes for CI and the PyPI release process.

## Workflows

Three workflows live in `.github/workflows/`:

| Workflow | Triggers | What it does |
|---|---|---|
| `test.yml` | push to `main`, PRs, manual | `cargo fmt`, ruff, Rust tests, Python tests on 3.12/3.13/3.14 × Linux/macOS |
| `docs.yml` | push to `main` or PR touching `docs/**`, `mkdocs.yml`, `pyproject.toml`; manual | Builds the mkdocs site with `--strict`; deploys to GitHub Pages on `main` only |
| `pypi.yml` | push of a `v*` tag; manual | Builds wheels + sdist, smoke-tests them, publishes to PyPI |

## Cutting a release

A release is triggered by **pushing a tag that starts with `v`**. Nothing else
publishes — not a push to `main`, not a PR, not a manual run.

```bash
# 1. Bump the version in Cargo.toml (pyproject.toml reads it dynamically)
#    and commit it. The tag must match this version exactly.
vim Cargo.toml            # version = "0.2.1"
cargo check               # refresh Cargo.lock
git commit -am "Release 0.2.1"
git push

# 2. Tag and push
git tag v0.2.1
git push origin v0.2.1
```

The `pypi.yml` workflow then runs, in order:

1. **version-check** — fails if the tag doesn't match `Cargo.toml` (`v0.2.1` requires `version = "0.2.1"`)
2. **builds** — one wheel each for linux x86_64/aarch64, macOS x86_64/aarch64, Windows x64, plus an sdist
3. **smoke-test** — installs the built Linux wheel on 3.12, 3.13, and 3.14 and runs the test suite
4. **release** — attests provenance, publishes to PyPI, creates a GitHub Release

If any step fails, nothing is published. `skip-existing: true` means re-running a
tag whose version is already on PyPI won't error, so a partial failure is safe to
retry.

Note that tags are not branch-scoped: a `v*` tag pushed from any branch will
trigger a release.

### Dry run

Trigger `pypi.yml` manually (Actions → Publish to PyPI → Run workflow) from a
branch. It builds every wheel and runs the smoke test, but skips `version-check`
and `release`, so nothing is published.

## One-time setup

These are configured outside the repo and are required before the first release:

- **PyPI Trusted Publishing** — on PyPI, add a publisher for project
  `treesearch-ud`: owner `rmalouf`, repo `treesearch`, workflow `pypi.yml`,
  environment `pypi`. For the very first release the project doesn't exist yet,
  so add it as a *pending* publisher. Then create the `pypi` environment under
  repo Settings → Environments. No API token is stored anywhere; the workflow
  authenticates over OIDC.
- **GitHub Pages** — Settings → Pages → Source must be set to **GitHub Actions**,
  or `docs.yml` fails at the deploy step.

## Wheels are abi3

`Cargo.toml` sets `pyo3 = { features = ["abi3-py312"] }`, so each platform builds
a *single* `cp312-abi3` wheel that works on Python 3.12 and every later version.
That's why the build matrix has one entry per platform rather than one per
Python version.

The `abi3-py312` part matters. With a bare `abi3` feature and no minimum version,
PyO3 uses whatever interpreter it finds at build time as the ABI floor — building
on 3.14 then yields a `cp314-abi3` wheel that Python 3.12 and 3.13 users cannot
install, silently contradicting `requires-python = ">=3.12"`. The `smoke-test`
job exists to catch exactly this: it installs the built wheel on all three
supported versions.

To check the tag of a locally built wheel:

```bash
uv run maturin build --profile dev --out dist
ls dist/    # expect treesearch_ud-<version>-cp312-abi3-<platform>.whl
```

## Known gaps

- Ruff in CI covers `python/` and `tests/` only; the scripts in `examples/` have
  outstanding lint and format errors.
- There is no `cargo clippy` gate — the lib currently emits ~12 clippy warnings.
- No musllinux (Alpine) or Windows ARM64 wheels are built.
