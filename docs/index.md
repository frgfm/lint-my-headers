---
description: Check copyright and license headers across nine languages. Refresh stale years with small, reviewable changes.
---

# Lint My Headers

Check copyright and license headers across nine languages. Refresh stale years
with small, reviewable changes.

For maintainers who already keep headers in their source files. Declare the
owner, license, and languages in one policy. Run a read-only check, review the
findings, and update eligible stale years.

## What it does

- **Nine languages:** Python, JavaScript, TypeScript, Rust, Go, Swift, Bash, C,
  and C++.
- **Read-only checks:** `lmh check` reports what needs attention without writing
  source files.
- **Limited repairs:** `lmh fix` updates one recognized stale year for your
  configured owner, or inserts a missing header with a declared creation year.

!!! info "Version 0.8.0"

    Missing-header insertion, additional header layouts, and the wheel hook join
    all nine supported languages. Select `languages` in your policy; the default
    is Python. See the
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

- [Compare header tools](comparison.md): compare features, check latency,
  throughput, memory, disk use, and installation steps.
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

Existing-header repairs preserve the creation year, body bytes, and file mode.
Ambiguous layouts, unsafe links, and concurrently changed targets are refused.
Missing headers without a creation year, wrong owners, malformed notices, and
future years require manual review.

Ownership and licensing always come from your declared policy. Lint My Headers
does not establish legal, SPDX, or REUSE compliance. Version 0.8.0 can
[insert a missing header with a declared creation year](getting-started.md#insert-a-missing-header).
Both `lmh` and `lint-my-headers` expose the same commands, with no language
toolchain or Node.js runtime needed by the installed native CLI.

The project is licensed under
[Apache-2.0](https://github.com/frgfm/lint-my-headers/blob/main/LICENSE).
[Report an issue](https://github.com/frgfm/lint-my-headers/issues) or read the
[contributor guide](https://github.com/frgfm/lint-my-headers/blob/main/CONTRIBUTING.md).
