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
language-specific details. Published 0.7.0 leaves missing headers for manual review.
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

## Insert a missing header (unreleased)

The source checkout can insert a header when the policy supplies its owner and
notice and you declare the file's first copyright year. The existing
`starting-year` is a validation floor; it is not used as the file's year.

```shell
lmh check --creation-year 2024 src/new_file.py
lmh fix --creation-year 2024 src/new_file.py
lmh check src/new_file.py
git diff
```

The new header uses 2024 through the current year. Declare the actual year for
the selected files; `creation-year = 2024` in project configuration has the same
effect. Without this declaration, `fix` keeps `LMH001` unresolved.
Select files with the same actual creation year and run separately for different years.

Insertion keeps the original bytes, encoding, line endings, preambles, and mode.
Existing legal declarations, ambiguous layouts, and notices that cannot be
encoded in the file remain unfixable. Python files that start with parentheses,
joined or escaped strings, or byte, formatted, or template strings also require
review. Existing headers still use the limited end-year repair. This feature is
not in published 0.7.0.

Insertion refuses any file that contains `copyright`, `©`, `SPDX`, `license`,
`licence`, or `all rights reserved`, regardless of case or position. A leading
comment such as `(c) 2024 Other Owner` also needs review. Examples and variable
names can trigger these guards; `LMH001` explains the refusal. Review those files
and add the header manually.

| Exit code | Meaning |
| --- | --- |
| `0` | No unresolved findings; any eligible repairs completed. |
| `1` | Unresolved findings remain. |
| `2` | Invocation, configuration, or I/O failure. |

For CI, use [Integrations](integrations.md). For programmatic output, see
[Diagnostics & agents](diagnostics.md).
