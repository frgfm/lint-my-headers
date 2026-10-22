---
description: Run Lint My Headers with pre-commit, prek, the GitHub Action, and an annual copyright refresh workflow.
---

# Integrations

Use the same header policy in your editor workflow, pull-request checks, and scheduled review PRs.

## pre-commit and prek

### Repository wheel hook

Install [uv](https://docs.astral.sh/uv/) and put it on `PATH`. Use the short
repository hook to run the exact released native wheel:

```yaml
repos:
  - repo: https://github.com/frgfm/lint-my-headers
    rev: v0.8.0
    hooks:
      - id: lmh-wheel
```

The `lmh-wheel` hook is included in 0.8.0.

Configure `languages` in your policy; the default is Python. uv manages Python
3.11+ and caches the wheel environment for later runs. It ignores installed uv
tools and uv configuration files, and disables source builds. A missing platform
wheel fails; choose the source hook below if you need a Rust build.

### Released wheel recipe without uv

For the published 0.8.0 native wheel, use an isolated Python 3.11+ hook.
Configure `languages` in any supported policy file for multilingual checks;
the default is Python. `--only-binary` prevents an unexpected Rust build.

```yaml
repos:
  - repo: local
    hooks:
      - id: lmh
        name: Lint My Headers
        entry: lmh check
        language: python
        types: [file]
        additional_dependencies:
          - --only-binary=lint-my-headers
          - lint-my-headers==0.8.0
```

### Source hook

The existing first-party `lmh` hook builds Rust on its
first installation using the pinned toolchain:

```yaml
repos:
  - repo: https://github.com/frgfm/lint-my-headers
    rev: v0.8.0
    hooks:
      - id: lmh
```

Prefer an immutable release SHA for production workflows.
pre-commit's Rust installer does not pass `--locked`.

### Run a check

Run either tool against all files:

```shell
pre-commit run --all-files
# Or:
prek run --all-files
```

The hook runs `check`; use a separate, reviewed `lmh fix` invocation for repairs.

## GitHub Action

The Action reads the repository's declared policy. A minimal published-release
workflow is:

```yaml
name: headers
on: [push, pull_request]
permissions:
  contents: read
jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
      - uses: frgfm/lint-my-headers@v0.8.0
```

Prefer immutable release SHAs for production workflows. The
[v0.8.0 release notes](https://github.com/frgfm/lint-my-headers/releases/tag/v0.8.0)
provide its commit SHA. The default installer uses uv and the exact PyPI version
declared by the Action ref, with source builds disabled. Unavailable wheels,
missing releases, and floating versions fail.

For an unreleased revision, pin the Action to its reviewed immutable SHA and set
`version: source`. This explicitly builds that Action checkout using `Cargo.lock`.
Configure `languages` in the repository's policy for multilingual checks.

### Inputs and outputs

| Input | Behavior |
| --- | --- |
| `version` | Exact stable/RC PyPI version, or `source`; empty uses the Action ref's package version. |
| `mode` | `check` (default) or `fix`. |
| `owner`, `starting-year` | Override the configured ownership/year policy. |
| `license`, `license-notice` | Override the selected license source. |
| `folders` | Override configured source paths. |
| `ignore-files`, `ignore-folders` | Override exclusions with comma-separated lists. |
| `respect-gitignore` | Apply `.gitignore` rules during directory scans; `true` or `false`. |

Policy inputs default to `__from_config__`. See
[action.yml](https://github.com/frgfm/lint-my-headers/blob/main/action.yml) for the
complete contract. `mode: fix` writes eligible repairs into the working tree;
it does not commit or open a PR.

| Output | Compact JSON array |
| --- | --- |
| `issues` | Files with unresolved findings. |
| `changed` | Files with completed repairs. |

On exit 2, `issues` stays `[]`, while `changed` retains completed repairs.
An I/O error stops new repairs; already-running repairs finish before results
are returned.

## Annual copyright refresh

The repository's maintained
[annual workflow template](https://github.com/frgfm/lint-my-headers/blob/main/README.md#annual-copyright-refresh)
schedules a review PR for **January 1 at 00:01 Europe/Paris**. It uses an explicit
quality dependency and `headers-fix` target, preserves creation years, and creates
at most one branch/PR per year. Its consumer template stages tracked Python
changes only.

Follow that template's repository setup and recovery instructions. GitHub
schedules can be delayed or dropped, run only from the default branch, and can
be disabled after prolonged inactivity. Automated PRs may require approval before
their checks start. The schedule is a review workflow, not an automatic merge.
