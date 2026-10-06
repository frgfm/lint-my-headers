---
description: Configure explicit ownership and license policies, select languages and paths, and understand supported header layouts.
---

# Configuration

One explicit policy for your source files. Choose the languages, paths, and notice that belong to your project.

## Configuration files

Discovery searches upward from the invocation directory and selects the nearest
applicable policy. Within each directory, priority is:

| Priority | File | Policy location |
| --- | --- | --- |
| 1 | `.lmh.toml` | Top-level keys. |
| 2 | `pyproject.toml` | `[tool.lint-my-headers]`. |
| 3 | `Cargo.toml` | `[package.metadata.lint-my-headers]` or `[workspace.metadata.lint-my-headers]`. Package settings take precedence. |
| 4 | `package.json` | A `"lint-my-headers"` object. |

Manifests without LMH settings are skipped. Invalid configurations fail;
configurations are never merged. To select an exact file:

```shell
lmh check --config Cargo.toml
```

Other explicit TOML filenames use the `[tool.lint-my-headers]` section.
Configured paths are relative to the policy file. Positional CLI paths are
relative to the invocation directory. CLI policy options override file settings.

### Rust example

```toml
[package.metadata.lint-my-headers]
owner = "Example Organization"
starting-year = 2024
license = "Apache-2.0"
languages = ["rust"]
paths = ["src", "tests"]
ignore-folders = ["target"]
```

For a virtual workspace, use `[workspace.metadata.lint-my-headers]`. Go modules,
SwiftPM/Xcode projects, and shell or C/C++ projects can use `.lmh.toml`;
`go.mod`, `go.work`, and `Package.swift` do not supply header policy.

## Options

| Key / CLI option | Meaning and default |
| --- | --- |
| `owner` / `--owner` | Required exact, single-line copyright owner. |
| `starting-year` / `--starting-year` | Required earliest accepted file creation year. |
| `creation-year` / `--creation-year` | Unreleased source only. Optional declared first year for inserting missing headers; default unset. Applies to all selected missing headers, not existing ones. |
| `license` / `--license` | SPDX identifier selecting a prose notice; requires a local `LICENSE`. |
| `license-notice` / `--license-notice` | Custom notice file; configure exactly one license source. |
| `paths` / positional paths | Selected files or directories; default `.`. |
| `languages` / `--languages` | Non-empty language allowlist; default `["python"]`. |
| `ignore-files` / `--ignore-files` | Exact excluded basenames; default `["__init__.py"]`. |
| `ignore-folders` / `--ignore-folders` | Excluded subtrees; default `[".github"]`. |

Configuration lists are arrays. CLI language and ignore lists are comma-separated
and replace the corresponding configured list:

```shell
lmh check --languages typescript,rust --ignore-folders node_modules,target
```

Exclusions apply to explicit inputs too. Limit paths or exclude dependency/build
directories such as `node_modules`, `target`, `vendor`, and `.build`; excluded
subtrees are not traversed. The tool does not infer exclusions from `.gitignore`.

## Supported languages

Only enabled, supported extensions are checked, including for explicit files.
Unknown language names and empty language lists fail.

| Selector | Extensions | Header marker |
| --- | --- | --- |
| `python` | `.py` | `#` |
| `javascript` | `.js`, `.jsx`, `.mjs`, `.cjs` | `//` |
| `typescript` | `.ts`, `.tsx`, `.mts`, `.cts`, including declarations | `//` |
| `rust` | `.rs` | `//` |
| `go` | `.go`, including test and platform files | `//` |
| `swift` | `.swift`, including `Package.swift` | `//` |
| `bash` (alias `shell`) | `.sh`, `.bash` | `#` |
| `c` | `.c`, `.h` | `//` |
| `cpp` (alias `c++`) | `.cc`, `.cpp`, `.cxx`, `.c++`, `.C`, `.hh`, `.hpp`, `.hxx`, `.h++`, `.H`, `.ipp`, `.tpp`, `.inl`, `.h` | `//` |

Shared `.h` headers are checked with either C selector, using C first and falling
back to C++ when needed. Other extensions and extensionless shebang scripts are
skipped. Files are checked regardless of build tags or platform suffixes.

## Header layouts

Use the [example header](getting-started.md#check-existing-headers) with the
appropriate comment marker and blank lines. A custom `license-notice` file can
contain plain text; existing Python-commented notice files remain accepted.
Custom notices must contain non-whitespace text after removing Python comment
markers. Blank notices fail with exit 2 before source files are checked or repaired.
Notice lines must match in full, including the final line when the notice file has
no trailing newline.

### Additional layouts (unreleased)

!!! info "Source checkout only"

    These layouts are supported by the unreleased source checkout. Published
    version **0.7.0** uses the prose layout above. Build the reviewed source
    revision to use these additions; the year-only repair rules still apply.

Common prose copyright lines may use `Copyright`, `Copyright (C)`, or
`Copyright (c)`, followed by one four-digit year or year range and the exact
configured owner. A comma before the owner and a final period are optional.
The license notice must still match the policy, with one blank line between it
and the copyright. Commented blank lines such as `//` or ` *` are accepted.

For languages that use `//`, an ordinary `/* ... */` block may contain the
whole header. The closing `*/` may follow the final notice on the same line.
A leading `*` on each content line is optional. Python and Bash
continue to use `#` comments. For example, with `owner = "Example Organization"`
and `license = "Apache-2.0"`:

```c
/*
 * Copyright 2024-2026 Example Organization
 *
 * This program is licensed under the Apache License 2.0.
 * See LICENSE or go to <https://www.apache.org/licenses/LICENSE-2.0> for full license details.
 */
```

An SPDX header may instead contain one `SPDX-FileCopyrightText` field and one
`SPDX-License-Identifier` field. The owner must match exactly, and the identifier
must match the configured `license`. Keep the local `LICENSE` file. The fields
may appear in either order, with optional blank lines between them:

```python
# SPDX-FileCopyrightText: 2024-2026 Example Organization
# SPDX-License-Identifier: Apache-2.0

value = 1
```

The same pair works in `//` comments or an ordinary block. A prose copyright
line with an SPDX license identifier is also accepted. A custom `license-notice`
may declare the exact SPDX field instead; it does not enable expression parsing.
Custom notices are checked in full, including any text after an SPDX tag.
An existing prose header may also contain one SPDX identifier after its notice;
both the full notice and the identifier must match the declared policy.
Do not add prose license text to an SPDX-only pair. Separate later line-comment
notes from the pair with a blank line; an SPDX block contains only the pair and
optional blank lines. A separate SPDX tag or prose license notice in the leading
comments also needs review, even after a blank line or outside the chosen block.

Multiple copyright fields, duplicate license identifiers, nested legal blocks,
documentation comments, and mixed or malformed declarations refuse repair.
Multiple holders, omitted years, sidecar files, and `REUSE.toml` are outside this
support. The `license` setting accepts one identifier; custom notices are matched
as text, without evaluating compound expressions. Use REUSE for its full convention.
LMH checks only the declared header policy and does not establish compliance.

??? note "Python"
    UTF-8 BOM, shebang, and PEP 263 cookie are preserved. Verified encodings:
    UTF-8, ASCII, Latin-1, and Windows-1252. Other codecs fail without repair.

??? note "JavaScript / TypeScript"
    UTF-8 BOM, shebang, CRLF, and body are preserved. A shebang needs a blank
    separator before the header. Bare CR headers and Unicode line separators
    are refused.

??? note "Rust"
    UTF-8 BOM, shebang, and newline style are preserved. Use ordinary `//`
    headers; crate attributes follow the header. Shebangs need a blank separator;
    ambiguous comment-prefixed `#!` forms are refused.

??? note "Go"
    UTF-8 BOM and CRLF are preserved. Leading `//go:build` and `// +build`
    directives need a blank separator; constraints after the header are also accepted.

??? note "Swift"
    UTF-8 BOM, CRLF, and shebang are preserved. A leading
    `// swift-tools-version:` stays first, with a blank separator before the header.

??? note "Bash / shell"
    UTF-8 BOM, CRLF, shebang, and executable permissions are preserved.
    A shebang needs a blank separator; other shell dialects are skipped.

??? note "C / C++"
    UTF-8 BOM, CRLF, and body are preserved. Put include guards and
    `#pragma once` after the header.

Validation stops at the first code line after allowed preambles and leading
comments/blank lines. Later copyright notices and body syntax errors are ignored;
the entire file must still decode successfully. Duplicate notices and
copyright-bearing documentation comments in the leading region refuse repair.
Published 0.7.0 also refuses copyright-bearing ordinary blocks; the unreleased
layouts above support unambiguous ordinary blocks.
Directory discovery skips symlinks and reparse points. Explicit linked
files may be checked, but repairs refuse linked files/parents, multiple hard
links, and concurrently changed targets.

The bundled SPDX snapshot is **v3.28.0**, retaining previously accepted v3.17
names/URLs, including `KiCad-libraries-exception`. Compound SPDX expressions and
new exception semantics are unsupported. Selecting a notice does not establish
legal or SPDX compliance.
