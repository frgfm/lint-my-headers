# Lint My Headers v0.8.0

Version 0.8.0 adds missing-header insertion with a declared creation year,
broader header recognition, and a wheel-only repository hook for pre-commit
and prek. Recognized stale-year repairs continue to change only the end year.

## Install and upgrade

```shell
uv tool install --force lint-my-headers==0.8.0
```

Or use `python -m pip install --upgrade lint-my-headers==0.8.0`.
PyPI installation requires Python 3.11+. Native wheels include both CLI names;
source builds require Rust 1.93 and a C compiler.

## Changes since 0.7.0

- Insert a missing header only when the configured owner and notice are explicit
  and `--creation-year` or `creation-year` declares the selected files' actual
  first copyright year. `starting-year` remains a validation floor. Without a
  creation year, missing headers remain unresolved.
- Recognize common prose copyright forms, ordinary block headers, and
  single-owner SPDX copyright/license pairs. Existing-header repairs preserve
  every byte except the eligible end year.
- Add the `lmh-wheel` repository hook, pinned to the release's native PyPI wheel.
  It requires uv and disables source builds. The `lmh` source hook remains available.
- Publish feature and runtime comparisons with six alternative header tools,
  including raw samples and reproduction commands. Performance and installed-size
  measurements remain labeled as the recorded 0.7.0 snapshot; they have not been
  rerun for 0.8.0.
- Keep benchmark reports out of package distributions and retain byte-preserving
  repair, link, ambiguity, encoding, and concurrent-change guards.

## Migration and integration

Existing policies continue to work. Missing-header insertion is opt-in:

```shell
lmh check --creation-year 2024 src/new_file.py
lmh fix --creation-year 2024 src/new_file.py
lmh check src/new_file.py
git diff
```

Use the actual declared creation year for the selected files, with separate runs
for different years. Existing legal markers, unsafe targets, and ambiguous
content require manual review. LMH never infers owners, licenses, or years and
does not establish legal or REUSE compliance.

For pre-commit or prek, install uv and select the wheel hook:

```yaml
repos:
  - repo: https://github.com/frgfm/lint-my-headers
    rev: v0.8.0
    hooks:
      - id: lmh-wheel
```

For the GitHub Action, use `frgfm/lint-my-headers@v0.8.0`. Prefer the immutable
release commit SHA for production integrations. Both CLI names, exit codes,
JSON schema version 1, diagnostic codes, and Action inputs/outputs are preserved.

---

# Lint My Headers v0.7.0

Version 0.7.0 brings the multilingual CLI to the published native PyPI wheels.
Check and safely refresh existing copyright and license headers in Python,
JavaScript, TypeScript, Rust, Go, Swift, Bash/shell, C, and C++ using one
explicit policy.

## Install and upgrade

```shell
uv tool install lint-my-headers==0.7.0
# Existing uv tool installation:
uv tool install --force lint-my-headers==0.7.0
```

For pip, run `python -m pip install --upgrade lint-my-headers==0.7.0`.
PyPI tooling and the optional launcher require Python 3.11+. Wheels contain
native executables; source builds require Rust 1.93 and a C compiler.

## Changes since 0.6.0

- Support JavaScript, TypeScript, Rust, Go, Swift, Bash/shell, C, and C++ in
  addition to Python, including language-specific leading-header handling.
- Discover policy in `.lmh.toml`, `pyproject.toml`, Cargo package/workspace
  metadata, or `package.json`. Search upward for the nearest policy without
  merging configurations.
- Accelerate large-tree checks and repairs with bounded parallel work and
  reuse of header-analysis state. The performance guide retains measured
  results, fixtures, and reproduction instructions.
- Reject blank custom license notices and require complete notice lines,
  including a final line without a trailing newline.
- Provide the documentation site at `https://docs.fgfm.dev/lint-my-headers/`
  through package metadata and maintained installation/integration examples.

## Migration and integration

The default language remains Python. Existing 0.6.0 Python policies continue
to work. Add only the languages used by your project:

```toml
[tool.lint-my-headers]
owner = "Example Organization"
starting-year = 2024
license = "Apache-2.0"
languages = ["python", "typescript", "rust"]
paths = ["src", "tests"]
ignore-folders = ["node_modules", "target"]
```

Use your established owner, earliest accepted creation year, and license,
and keep the matching `LICENSE` at the project root. Blank custom notice files
are rejected; source headers with incomplete notice lines are reported.

For the GitHub Action, update the ref to `frgfm/lint-my-headers@v0.7.0`.
The Action installs its exact PyPI version with source builds disabled.
Prefer the immutable release commit SHA, recorded in the GitHub release.
The README and integration guide provide a wheel-only pre-commit/prek recipe
for multilingual checks and a tagged source-hook alternative.

Both CLI names, `check`/`fix`, exit codes 0/1/2, JSON schema version 1,
diagnostic meanings, and Action inputs/outputs are preserved. `check` remains
read-only. `fix` changes only one recognized stale year for the configured
owner, preserves other bytes and mode, and refuses ambiguous or unsafe
targets. Missing headers require manual insertion; this release does not
claim legal, SPDX, or REUSE compliance.

---

# Lint My Headers v0.6.0

`validate-python-headers` is now **Lint My Headers**. This is an intentional clean break before the first PyPI publication.

## Required migration

GitHub does not redirect Action calls after a repository rename. Every old reference will fail with `repository not found` and must be changed:

```diff
- uses: frgfm/validate-python-headers@v0.5.1
+ uses: frgfm/lint-my-headers@v0.6.0
```

For security-sensitive workflows, use the immutable v0.6.0 release commit SHA instead of the tag.

| Before | v0.6.0 |
| --- | --- |
| repository and Action `frgfm/validate-python-headers` | `frgfm/lint-my-headers` |
| command `vph` | `lmh` |
| command `validate-python-headers` | `lint-my-headers` |
| configuration `[tool.validate-python-headers]` | `[tool.lint-my-headers]` |
| Python module `validate_headers` | `lint_my_headers` |
| diagnostics without stable codes | `LMH001`–`LMH008`, `LMH900` |

No legacy aliases or redirect PyPI package are provided. The historical PyPI name remains unclaimed by design.

## Highlights

- Native Rust executables, platform-specific PyPI wheels, and a Cargo-buildable source distribution.
- Stable JSON schema version 1 for coding agents and CI.
- One deterministic diagnostic per unresolved file.
- Python-aware BOM, shebang, and PEP 263 encoding handling.
- Conservative stale-year repair with symlink, reparse-point, hard-link, and concurrent-change protection.
- SPDX License List Data v3.28.0 with compatibility for every notice accepted from the previous v3.17 snapshot.
- Wheel-only pre-commit/prek recipe and composite Action with `issues` and `changed` outputs; explicit source mode remains available for unreleased code.
- Renamed upstream agent skill and eval assets retained; historical model evaluations have not been rerun.

The optional Python `main` forwards to the installed executable and returns its exit code. `python -m lint_my_headers` replaces the Python process on Unix and uses a child process on Windows. Internal Python parsing APIs are removed. Unsupported Python-specific encodings return `LMH007`; supported codecs and source-build requirements are documented in the README.

v0.6.0 supports Python source files. The broader name does not promise additional source languages in this release.

## Owner-controlled release gates

- [ ] Recheck the PyPI, repository, and Marketplace names.
- [ ] Delist historical Marketplace releases without deleting tags.
- [ ] Merge the implementation while the old repository path still exists.
- [ ] Rename the repository to `lint-my-headers` and update its description/topics.
- [ ] Configure the protected GitHub `pypi` environment and matching pending PyPI Trusted Publisher.
- [ ] Publish and verify `v0.6.0rc1` on live PyPI and through the remote Action.
- [ ] Publish final `v0.6.0` only from a passing RC.
- [ ] Update user-controlled downstream repositories to the immutable final SHA.

If PyPI accepts only part of an upload, rerun the failed `publish` and `verify` jobs against the retained `release-artifacts` artifact. Do not rerun `build`; the workflow refuses a same-named remote file whose SHA-256 differs.
