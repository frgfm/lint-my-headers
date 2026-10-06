<p align="center">
  <a href="https://docs.fgfm.dev/lint-my-headers/">
    <img src="https://raw.githubusercontent.com/frgfm/lint-my-headers/74325965b1920514787d2a92b43789235a75ec12/docs/assets/images/logo.svg" alt="Lint My Headers" width="80" height="80">
  </a>
</p>

<h1 align="center">Lint My Headers</h1>

<p align="center">
  <strong>Fast copyright and license header checks, powered by Rust.</strong><br>
  One policy for your codebase. Clear findings. Small, reviewable repairs.
</p>

<p align="center">
  <a href="https://pypi.org/project/lint-my-headers/"><img src="https://img.shields.io/pypi/v/lint-my-headers?color=ba5b3b" alt="PyPI version"></a>
  <a href="https://github.com/frgfm/lint-my-headers/actions/workflows/tests.yml"><img src="https://github.com/frgfm/lint-my-headers/actions/workflows/tests.yml/badge.svg?branch=main" alt="Tests"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-ba5b3b" alt="Apache-2.0 license"></a>
</p>

<p align="center">
  <a href="https://docs.fgfm.dev/lint-my-headers/"><strong>Documentation</strong></a> ·
  <a href="#quick-start"><strong>Quick start</strong></a> ·
  <a href="#integrations"><strong>Integrations</strong></a> ·
  <a href="https://github.com/frgfm/lint-my-headers/releases"><strong>Releases</strong></a>
</p>

Keep copyright and license notices consistent across your codebase, and turn an annual
refresh into a small diff. Run `lmh` locally, in CI, or from a coding agent using the
same explicit policy.

## Highlights

- **Native Rust execution.** A compiled CLI with no Python, Node.js, or language toolchain
  required to run the installed binary.
- **Nine languages, one policy.** Python, JavaScript, TypeScript, Rust, Go, Swift, Bash,
  C, and C++.
- **Leading-header checks.** Recognizes the opening comment region and requires notices
  before code, keeping header-like text in program bodies out of repairs.
- **Precise, reviewable fixes.** Refreshes recognized stale years while preserving the
  creation year, owner, all other bytes, and file permissions.
- **Fits your workflow.** First-party pre-commit/prek hook, GitHub Action, and stable JSON
  diagnostics for scripts and coding agents.

## Performance

On **10,000 mixed-language files** (1 KiB/file), the release CLI measured:

| Task | LMH | HawkEye 7.2.0 |
| --- | ---: | ---: |
| Routine check | 36 ms | 76 ms |
| Check stale headers | 62 ms | 76 ms |
| Repair stale years | 147 ms | 125 ms |

Five timing trials after warmup on the same shared Linux runner. Both tools receive
the same CPU budget; LMH uses up to four available workers for large trees.

![Native check latency](https://github.com/frgfm/lint-my-headers/releases/download/benchmark-5152bf26fb17/check.png)

See the [performance guide](docs/benchmarks.md) for repair time, peak memory,
all nine languages, CSVs, methodology and reproduction. These are synthetic,
warm-cache results; CPU limits and storage affect the comparison.

## Installation

Install the published release with [uv](https://docs.astral.sh/uv/):

```shell
uv tool install lint-my-headers==0.7.0
```

Or install with pip: `python -m pip install lint-my-headers==0.7.0`.

Version **0.7.0** supports all nine languages and policy in `.lmh.toml`,
`pyproject.toml`, `Cargo.toml`, or `package.json`. Select your project's languages
in the policy; the default remains Python.

To build from source, use Rust 1.93 and a C compiler:

```shell
cargo install --git https://github.com/frgfm/lint-my-headers --locked
```

PyPI tooling requires Python 3.11+. Wheels ship native `lmh` and `lint-my-headers`
executables; building a source distribution also requires Rust and a C compiler.
See the [installation guide](docs/getting-started.md#install) for source checkout
instructions and launcher details.

## Quick start

Declare your project's actual owner and license in `pyproject.toml`, keep the matching
`LICENSE` at the project root, and select the source folders that exist in your project:

```toml
[tool.lint-my-headers]
owner = "Example Organization"
starting-year = 2024
license = "Apache-2.0"
paths = ["src", "tests"]
```

Check existing headers:

```shell
lmh check
```

Findings tell you exactly where to look and whether a repair is available:

```text
src/example.py:1:1: LMH004 copyright year ends at 2025; expected 2026 [fixable]
```

Refresh recognized stale years and review the result:

```shell
lmh fix
lmh check
git diff
```

A 2026 refresh produces a diff like this:

```diff
-# Copyright (C) 2024-2025, Example Organization.
+# Copyright (C) 2024-2026, Example Organization.
```

Only the end year changes. `check` never writes source files. Without a declared
creation year, missing headers stay available for manual review. See the
[getting-started guide](docs/getting-started.md) for a complete header example.

## Benchmarks

Run `bash scripts/benchmark.sh` to generate CSVs, SVG/PNG charts and an HTML
report with a workflow-overhead calculator. Add `--compare /path/to/hawkeye` for
a locally measured comparison or `--baseline /path/to/previous/lmh` to show the
improvement. See [requirements and reproduction](docs/benchmarks.md#reproduce).

## Configuration and safety

### Insert missing headers (unreleased)

The source checkout can insert missing headers with your declared owner and
license notice. Supply the file's actual first copyright year:

```shell
lmh check --creation-year 2024 src/new_file.py
lmh fix --creation-year 2024 src/new_file.py
lmh check src/new_file.py
git diff
```

`creation-year` can also be set in project configuration. It applies to every
selected missing header. `starting-year` remains the earliest allowed year and
does not supply this value. Without a creation year, missing headers stay
unresolved. Existing legal text and unsafe targets require review. Insertion
keeps source bytes, encoding, preambles, line endings, and file mode. Published
0.7.0 does not include this feature.

Insertion refuses files containing `copyright`, `©`, `SPDX`, `license`, or `licence`
anywhere, including examples and variable names. Review those files manually.

Keep the policy in the file your project already uses:

| File | Configuration |
| --- | --- |
| `.lmh.toml` | Top-level keys. |
| `pyproject.toml` | `[tool.lint-my-headers]`. |
| `Cargo.toml` | `[package.metadata.lint-my-headers]` or `[workspace.metadata.lint-my-headers]`. |
| `package.json` | A `"lint-my-headers"` object. |

The CLI discovers the nearest policy by searching upward, with priority in
that order; Cargo package metadata takes precedence over workspace metadata. Manifests
without LMH settings are skipped. Invalid policies fail and configurations are never
merged. `--config` selects an exact file; CLI options override file settings.

For multilingual projects, select the languages used by your codebase:

```toml
languages = ["python", "typescript", "rust"]
```

The default is Python. Configured paths are relative to the policy file; explicit CLI
paths are relative to the invocation directory. Unsupported extensions are skipped.
Exclude generated, dependency, and build folders with `ignore-folders`; LMH does not
infer exclusions from `.gitignore`.

Published 0.7.0 repairs require one recognized stale year for the configured owner. Ambiguous layouts,
wrong owners, future years, unsupported encodings, symlinks/reparse points, multiple hard
links, and concurrently changed targets are refused. Ownership and licensing always
come from your policy; published 0.7.0 does not insert missing headers or establish legal, SPDX, or
REUSE compliance.

See the [configuration guide](docs/configuration.md) for all options, language aliases,
supported extensions, notice formats, and byte-preservation rules.

## Additional header layouts (unreleased)

The source checkout also accepts common `Copyright` lines, ordinary `/* ... */`
headers, and SPDX copyright/license pairs. These layouts are not supported by
the published **0.7.0** release. See the
[header layout guide](docs/configuration.md#additional-layouts-unreleased) for
examples and limits. Existing-header repairs still change only one recognized stale year.

```c
/*
 * SPDX-FileCopyrightText: 2024-2026 Example Organization
 * SPDX-License-Identifier: Apache-2.0
 */
```

## Agents and JSON

Use the same CLI from scripts and coding agents:

```shell
lmh check --output-format json
```

JSON schema version **1** includes the checked count, completed changes, and diagnostics
with path, line, column, code, message, and repair eligibility. Parsed JSON-mode commands
write only JSON to stdout; malformed CLI syntax remains a stderr usage error.

| Exit | Meaning |
| --- | --- |
| `0` | No unresolved findings. |
| `1` | Findings need review. |
| `2` | Invocation, configuration, or I/O failure. |

An agent should read the declared policy, run `check`, repair only when source changes
are authorized, then recheck and inspect the diff. A failed repair may follow earlier
completed changes; review `changed` and the working tree before retrying.

Use the [first-party agent skill](.agents/skills/lint-my-headers/SKILL.md) and the
[diagnostics guide](docs/diagnostics.md) for the full contract. Retained agent evaluations
are historical Python-era results, not measurements of the Rust CLI.

## Integrations

### pre-commit and prek

The new `lmh-wheel` repository hook uses [uv](https://docs.astral.sh/uv/) to run
the exact released native wheel. Install uv and put it on `PATH`, then select
your project's `languages` in its policy:

```yaml
repos:
  - repo: https://github.com/frgfm/lint-my-headers
    rev: <reviewed-commit-with-lmh-wheel>
    hooks:
      - id: lmh-wheel
```

This hook is available on `main` and will ship in the next release. Replace the
placeholder with a reviewed commit that contains it; `v0.7.0` has only the source
hook. uv manages Python 3.11+ and caches the installed wheel for later runs.
Source builds are disabled, so an unavailable wheel fails without compiling Rust.

Use the published native wheel in an isolated Python 3.11+ hook environment.
This released recipe also works without uv:

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
          - lint-my-headers==0.7.0
```

Alternatively, use the first-party source hook at the release tag:

```yaml
repos:
  - repo: https://github.com/frgfm/lint-my-headers
    rev: v0.7.0
    hooks:
      - id: lmh
```

The source hook compiles Rust on first installation with the pinned toolchain.
pre-commit's Rust installer does not pass `--locked`. All hooks run read-only checks.

### GitHub Action

Add the published release to your existing workflow:

```yaml
steps:
  - uses: actions/checkout@v7
  - uses: frgfm/lint-my-headers@v0.7.0
```

The Action reads repository policy and uses uv to install the exact PyPI version with
source builds disabled. Prefer immutable release SHAs; the
[v0.7.0 release notes](https://github.com/frgfm/lint-my-headers/releases/tag/v0.7.0)
provide its commit SHA. For an unreleased multilingual revision, pin its reviewed SHA
and set `version: source` to build that Action checkout using `Cargo.lock`.

Inputs override policy. The `issues` and `changed` outputs are compact JSON path arrays;
on exit 2, `issues` is empty and `changed` retains completed writes. An I/O error stops
new repairs; already-running repairs finish before results are returned. See the
[integration guide](docs/integrations.md) and [Action reference](action.yml) for all inputs,
outputs, and installation behavior.

### Annual copyright refresh

Create a review PR each January using the maintained workflow below.

<details>
<summary>Annual workflow template, setup, and recovery</summary>

Schedule a review PR for January 1 at **00:01 Europe/Paris**. The scheduler and job use the same timezone so the repair year is January 1's year even while UTC is still December 31. GitHub schedules run from the default branch and can be delayed or dropped under high load; the scheduled minute is not an execution-time guarantee.

First, declare LMH in your project's quality dependencies, then add a `headers-fix` Make target using the same environment as your header check in CI. For a quality dependency group, append the package to the existing group in `pyproject.toml`:

```toml
[dependency-groups]
quality = ["lint-my-headers==0.7.0"]
```

Run `uv lock` and commit the updated `uv.lock`, then add:

```makefile
headers-fix:
	uv run --locked --group quality lmh fix
```

For a quality extra, add the package to `quality` in `[project.optional-dependencies]` and use `uv run --extra quality lmh fix` instead; retain `--locked` when the repository commits its uv lockfile. The target must install/synchronize its environment because scheduled runners start fresh. A global `uv tool install` does not install LMH in the runner's project environment. The LMH version stays in `pyproject.toml`, without a second pin in the workflow.

Save this consumer template as `.github/workflows/update-copyright-years.yml`. This repository's [own workflow](.github/workflows/update-copyright-years.yml) builds its locked Rust source instead of installing a published package; the publication scripts match:

```yaml
name: update copyright years

on:
  schedule:
    - cron: "1 0 1 1 *"
      timezone: Europe/Paris

permissions:
  contents: read

concurrency:
  group: annual-copyright-years
  cancel-in-progress: false

jobs:
  update:
    runs-on: ubuntu-latest
    permissions:
      contents: write
      pull-requests: write
    env:
      TZ: Europe/Paris
      BASE_BRANCH: ${{ github.event.repository.default_branch }}
      UV_PROJECT_ENVIRONMENT: ${{ runner.temp }}/lmh-annual-env
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          ref: ${{ github.event.repository.default_branch }}
          fetch-depth: 0
      - uses: astral-sh/setup-uv@c18668ad3cf93ea998bef934396af7bb5c839dc7 # v10.2.0
        with:
          version: "0.12.5"
      - name: Create the annual pull request
        shell: bash
        env:
          GH_TOKEN: ${{ github.token }}
        run: |
          set -euo pipefail

          copyright_year="$(date +%Y)"
          branch_name="automation/update-copyright-years-$copyright_year"
          test "$branch_name" != "$BASE_BRANCH"

          pr_number="$(gh pr list --base "$BASE_BRANCH" --head "$branch_name" --state all --json number --jq '.[0].number // empty')"
          if test -n "$pr_number"; then
            echo "Annual pull request #$pr_number already exists."
            exit 0
          fi

          remote_sha="$(git ls-remote --heads origin "$branch_name" | cut -f1)"
          if test -z "$remote_sha"; then
            make headers-fix
            if git diff --quiet -- ':(glob)**/*.py'; then
              echo "Copyright years are already current."
              exit 0
            fi

            git switch -c "$branch_name"
            git config user.name "github-actions[bot]"
            git config user.email "41898282+github-actions[bot]@users.noreply.github.com"
            git add -u -- ':(glob)**/*.py'
            git commit -m "chore: update copyright years for $copyright_year"
            git push --set-upstream origin "$branch_name"
          fi

          gh pr create --base "$BASE_BRANCH" --head "$branch_name" \
            --title "chore: update copyright years for $copyright_year" \
            --body "Annual refresh of recognized Python copyright years using the declared header policy."
```

The external UV environment keeps installed dependency files outside the scan and PR. Repairs follow `[tool.lint-my-headers]` paths/exclusions and refresh recognized stale years for its declared owner in supported Python files. Leave `creation-year` unset for this year-only workflow. Missing, ambiguous, unsafe, or wrong-owner notices then require manual review; a failed repair stops before any branch or PR is published. Existing source bytes and modes are preserved. To cover more Python source files, extend the declared paths while preserving generated/vendor exclusions. This annual workflow stages Python changes only.

Each year gets one `automation/update-copyright-years-YYYY` branch. Existing PRs, including closed PRs, are left untouched; reopen the existing PR if it was closed by mistake. If a push succeeded but PR creation failed, rerunning resumes PR creation from that branch without overwriting it. If repairs make no changes and no annual branch already exists, no PR is created. Only tracked Python changes are committed; generated untracked files are excluded. Pushes never target the default branch or force-update an existing branch.

Repository setup:

- Merge the policy, Make target and workflow into the default branch. The workflow has only a schedule trigger, so it does not run on pushes, PRs or manual dispatch.
- Enable **Settings → Actions → General → Workflow permissions → Allow GitHub Actions to create and approve pull requests**. The job requests only `contents: write` and `pull-requests: write`; it does not approve or merge its PR.
- PR workflows triggered by `GITHUB_TOKEN` require a writer to click **Approve workflows to run** before their CI starts. Use an existing GitHub App installation token if unattended PR checks are required.
- GitHub disables schedules in public repositories after 60 days without activity; verify the schedule remains enabled before the annual run.

See GitHub's [schedule behavior](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#schedule) and [workflow-token triggering rules](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/trigger-a-workflow#triggering-a-workflow-from-a-workflow).

For a failed scheduled run, fix the cause and use **Actions → Re-run failed jobs** within [30 days of the original run](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/re-run-workflows-and-jobs). If no run was created or that window has expired, re-enable a disabled schedule for future runs, then run `make headers-fix` on a fresh branch from the default branch, inspect the diff, and open a manual PR. The workflow has no manual dispatch trigger.

</details>

## Contributing

Lint My Headers is open source under [Apache-2.0](LICENSE). Contributions, bug reports,
and feedback are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for local development
and checks, [AGENTS.md](AGENTS.md) for runtime contracts, and
[RELEASE_NOTES.md](RELEASE_NOTES.md) for release and migration notes.

[Report a bug](https://github.com/frgfm/lint-my-headers/issues) ·
[Read the docs](https://docs.fgfm.dev/lint-my-headers/) ·
[View releases](https://github.com/frgfm/lint-my-headers/releases)
