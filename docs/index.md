---
description: A Rust CLI for checking source copyright and license headers across nine languages and safely refreshing recognized stale years.
---

# Lint My Headers

Check source copyright and license headers across nine languages. Declare your
policy, find issues, and refresh recognized stale years while preserving
everything else.

## What it does

- **Nine languages:** Python, JavaScript, TypeScript, Rust, Go, Swift, Bash, C,
  and C++.
- **Read-only checks:** `lmh check` reports what needs attention without writing
  source files.
- **Year-only repairs:** `lmh fix` updates one recognized stale year for your
  configured owner.

!!! info "Multilingual release"

    Version 0.7.0 provides all nine languages through native PyPI wheels. Select
    `languages` in your policy; the default is Python. See the
    [installation options](getting-started.md#install).

## One year, one small diff

For a recognized header ending in 2025, `lmh check` reports `LMH004`. With source
changes authorized, run `lmh fix` to refresh the year in 2026:

```diff
-# Copyright (C) 2024-2025, Example Organization.
+# Copyright (C) 2024-2026, Example Organization.
```

The creation year, owner, and every other byte stay intact.

## From policy to pull request

1. **Declare your policy.** Set the owner, earliest creation year, license notice,
   and source paths.
2. **Run a check.** Use `lmh check` locally, in CI, or from a coding agent. Review
   each finding.
3. **Review the diff.** Run `lmh fix` for eligible stale years, recheck, and inspect
   the changes.

## Documentation

- [Compare header tools](comparison.md): choose between policy checks, header
  insertion, template formatting, and REUSE workflows.
- [Getting started](getting-started.md): install the CLI, declare a first policy,
  and check your existing headers.
- [Configuration](configuration.md): configuration files, language selectors, and
  header layouts.
- [Integrations](integrations.md): pre-commit, prek, the GitHub Action, and an
  annual review workflow.
- [Diagnostics & agents](diagnostics.md): findings, JSON output, and coding-agent
  repair boundaries.
- [Performance](benchmarks.md): check overhead, full-codebase repair time, memory,
  and a reproducible HawkEye comparison.

## Repair boundaries

Repairs preserve the creation year, body bytes, and file mode. Ambiguous layouts,
unsafe links, and concurrently changed targets are refused. Missing headers,
wrong owners, malformed notices, and future years require manual review.

Ownership and licensing always come from your declared policy. Lint My Headers
does not insert missing headers or establish legal, SPDX, or REUSE compliance.
Both `lmh` and `lint-my-headers` expose the same commands, with no language
toolchain or Node.js runtime needed by the installed native CLI.

The project is licensed under
[Apache-2.0](https://github.com/frgfm/lint-my-headers/blob/main/LICENSE).
[Report an issue](https://github.com/frgfm/lint-my-headers/issues) or read the
[contributor guide](https://github.com/frgfm/lint-my-headers/blob/main/CONTRIBUTING.md).
