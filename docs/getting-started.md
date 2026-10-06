---
description: Install Lint My Headers, declare a header policy, and run your first read-only check and safe year refresh.
---

# Getting started

Install the CLI, declare your policy, and run your first read-only check.

## Install

Install the published multilingual release or build the CLI from source.

=== "Published release"

    With [uv](https://docs.astral.sh/uv/), install the native wheel:

    ```shell
    uv tool install lint-my-headers==0.7.0
    lmh --version
    ```

    PyPI tooling requires Python 3.11+. Wheels contain native `lmh` and
    `lint-my-headers` executables. Building a source distribution also requires
    Rust and a C compiler. Version 0.7.0 supports all nine languages and every
    [configuration container](configuration.md#configuration-files).

=== "Source checkout"

    Use the pinned Rust 1.93 toolchain and a native linker:

    ```shell
    git clone https://github.com/frgfm/lint-my-headers.git
    cd lint-my-headers
    cargo install --path . --locked
    lmh --version
    ```

    Ensure Cargo's binary directory is on your `PATH`. Python is optional when
    using the native CLI directly.

## Declare the policy

Use your project's established ownership and license. These example values are
placeholders; the tool never infers them. Set paths to directories that exist and
keep the corresponding `LICENSE` at the project root.

=== "pyproject.toml"

    Add this section to `pyproject.toml`:

    ```toml
    [tool.lint-my-headers]
    owner = "Example Organization"
    starting-year = 2024
    license = "Apache-2.0"
    paths = ["src", "tests"]
    ignore-files = ["version.py"]
    ignore-folders = ["src/generated"]
    ```

=== ".lmh.toml"

    Create `.lmh.toml` in your project root:

    ```toml
    owner = "Example Organization"
    starting-year = 2024
    license = "Apache-2.0"
    languages = ["python", "javascript", "typescript", "rust", "go", "swift", "bash", "c", "cpp"]
    paths = ["src", "tests"]
    ignore-files = ["version.py"]
    ignore-folders = ["src/generated"]
    ```

Select only the languages used by your project. The default is Python. Both
configuration examples work with the published release and source checkout.

## Check existing headers

Run from the project directory:

```shell
lmh check
lmh check src/package tests/test_api.py
lmh check --help
```

The first command uses configured paths; positional paths replace that list.
Checks are read-only. A valid Python header in 2026 looks like this:

```python
# Copyright (C) 2024-2026, Example Organization.

# This program is licensed under the Apache License 2.0.
# See LICENSE or go to <https://www.apache.org/licenses/LICENSE-2.0> for full license details.
```

Bash uses the same `#` marker. Other supported languages use `//`, with the same
text and blank lines. See [header layouts](configuration.md#header-layouts) for
language-specific details. Missing headers require manual insertion and review.
The [additional layouts](configuration.md#additional-layouts-unreleased) support
common copyright lines, ordinary blocks, and SPDX pairs in the unreleased source
checkout; they are not available in published 0.7.0.

## Refresh a stale year

For a recognized header ending in 2025, `lmh check` reports `LMH004`. When the
diagnostic is marked fixable and source changes are authorized:

```shell
lmh fix
lmh check
git diff
```

In 2026, a creation year of `2024` with an end year of `2025` becomes
`2024-2026`. The tool retains the creation year and every other byte. It leaves
ineligible findings for manual review.

| Exit code | Meaning |
| --- | --- |
| `0` | No unresolved findings; any eligible repairs completed. |
| `1` | Unresolved findings remain. |
| `2` | Invocation, configuration, or I/O failure. |

For CI, use [Integrations](integrations.md). For programmatic output, see
[Diagnostics & agents](diagnostics.md).
