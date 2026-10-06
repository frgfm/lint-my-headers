---
name: lint-my-headers
description: Configure, run, troubleshoot, and safely integrate lint-my-headers (lmh) in Python, JavaScript, TypeScript, Rust, Go, Swift, Bash/shell, C, and C++ repositories. Use when a user wants to check or fix copyright or license headers, configure LMH policy, install its pre-commit or prek hook, configure its GitHub Action or annual review pull request, or interpret lmh JSON diagnostics. Do not use to choose a license, determine copyright ownership, provide legal advice, or manage unsupported languages.
license: Apache-2.0
compatibility: Requires lmh with JSON schema version 1; multilingual support requires --languages. PyPI installation and the optional Python launcher require Python 3.11+. Setup guidance works without an installed CLI.
---

# Lint My Headers

Use `lmh` as the deterministic implementation. Your job is to establish explicit policy, sequence read-only checks before authorized repairs, and explain the CLI's structured result without inventing legal facts.

## Boundaries

- Do not choose a license or determine who owns copyright. Ask the user for the exact owner, earliest accepted year, and license identifier or custom notice when the repository does not establish them unambiguously.
- Do not infer legal facts from Git history, package authors, neighboring headers, or a majority pattern.
- Do not implement a second parser or edit header text by hand.
- Do not install software, use the network, modify files, commit, push, create a branch or pull request, or change repository settings unless the user separately authorizes that action.
- Treat exit `1` as policy findings, not tool failure. Treat exit `2` as a command, configuration, path, license, or I/O failure and stop before repair.
- Use only verified released versions or immutable release SHAs in integrations. Never recommend `@main`, and never present a planned or locally installed version as already released. If network access was not authorized or no release ref exists yet, write `<RELEASE_TAG_OR_SHA>` and report that substitution as an open gate.

## Workflow

### 1. Ground in the repository

Read repository instructions, locate the repository root and nearest applicable configuration, and inspect the existing working-tree state. Preserve unrelated changes.

Probe the installed contract:

```console
lmh --version
lmh check --help
```

The workflow requires JSON schema version 1. Confirm multilingual availability through `lmh check --help` (`--languages`); do not assume a published 0.6.x release includes it. If `lmh` is missing or incompatible, explain the pinned installation command and stop unless installation was explicitly requested.

### 2. Establish explicit policy

Use `--config` when given; otherwise search upward from the invocation directory. Within each directory, priority is `.lmh.toml`, `pyproject.toml`, `Cargo.toml`, then `package.json`. Skip manifests without LMH settings; never merge configurations. Read top-level TOML, `[tool.lint-my-headers]`, package/workspace `metadata.lint-my-headers` (package first), or the `lint-my-headers` JSON object respectively. A runnable policy needs:

- one exact `owner`;
- one integer `starting-year`;
- exactly one `license` or `license-notice`;
- optional paths, exclusions, and a `languages` allowlist (`python`, `javascript`, `typescript`, `rust`, `go`, `swift`, `bash` (alias `shell`), `c`, `cpp` (alias `c++`); default Python only).

If a required value is missing, conflicting, or legally ambiguous, show the evidence and ask for that decision. Do not write configuration yet unless the user asked for setup or modification.

### 3. Check before changing anything

Run the narrowest applicable read-only command:

```console
lmh check --output-format json [PATH...]
```

Parse stdout as JSON. Require `schema_version == 1`; never scrape human stderr when structured output is available.

Interpret the result exactly:

- exit `0`: selected files comply;
- exit `1`: report each diagnostic path, code, message, and `fixable` flag;
- exit `2`: report `error.code`, `error.message`, and `error.path`, then stop.

Do not claim that a clean result proves license compatibility or legal compliance. It proves only that checked files match the configured header policy. Unsupported and disabled extensions are skipped even when explicitly supplied.

### 4. Repair only when explicitly requested

Before `fix`, record the pre-existing diff so unrelated changes remain distinguishable. Scope the command to the requested paths whenever possible:

```console
lmh fix --output-format json [PATH...]
```

Published 0.7.0 updates only recognized stale years. The unreleased source checkout can also insert missing headers when the owner, notice, and `creation-year` are explicitly declared and the diagnostic is fixable. Never substitute the policy's earliest accepted `starting-year` for a file's creation year. Existing legal text, malformed, ambiguous, future-dated, wrong-owner, wrong-license, symlinked, reparse-point, and multi-link targets stay unresolved.

After `fix`:

1. Read `changed` and `diagnostics` from JSON.
2. Run the same `lmh check --output-format json [PATH...]` command again.
3. Inspect only the targeted diff.
4. Verify that changed files match `changed`, unresolved files match diagnostics, and unrelated pre-existing changes remain untouched.
5. Never commit or push unless the user separately asks.

### 5. Configure integrations only when requested

- Published 0.7.0 uses the existing prose layout. The unreleased source checkout also supports common prose copyright forms, ordinary block headers, and single-owner SPDX pairs; see `docs/configuration.md`. Verify the source revision before using those layouts. Do not infer their availability from the version string alone or claim REUSE compliance.
- Missing-header insertion is also unreleased. Verify `--creation-year` in the source CLI, obtain the year from explicit project/user policy, scope the selected files, run `check`, then use `fix` only when authorized. Without `creation-year`, missing headers remain unfixable.
- Put policy in one supported project configuration; do not duplicate it in each integration. Header wording and blank lines are shared; Python and Bash/shell use `#`; other supported languages use `//`. Shell scripts use `.sh` or `.bash` with `languages = ["bash"]` (alias `shell`) and a blank separator after a shebang; extensionless scripts are skipped. C/C++ projects use `.lmh.toml` with `languages = ["c", "cpp"]` (`c++` aliases `cpp`); shared `.h` headers are checked with either selector. Put include guards and `#pragma once` after the ordinary `//` header. Prefer plain-text custom license notices. Rust packages may use `[package.metadata.lint-my-headers]` and virtual workspaces `[workspace.metadata.lint-my-headers]` in `Cargo.toml`, with `languages = ["rust"]`. Go and Swift projects use `.lmh.toml` with their language selected; do not place policy in `go.mod`, `go.work`, or `Package.swift`. Preserve the SwiftPM tools-version directive as the first line with a blank separator before the header.
- Prefer the README's wheel-only local `lmh` hook with an exact verified PyPI version. The first-party Rust hook is for source checkouts. Use explicit placeholders when publication cannot be verified without unauthorized network access.
- Give pull-request checks read-only `contents` permission.
- Treat annual year refresh as an optional project convention. Use a deterministic review branch and pull request, never a direct default-branch write.
- Preserve existing Action inputs for compatibility, but omit overrides when repository config is authoritative.
- The Action defaults to the exact package version declared by its ref, without source-build fallback. Use `version: source` explicitly for unreleased code; never infer that a checked-out package version has been published.

## Report format

Return a concise operational report:

```markdown
## Header policy
- Config: <path or missing>
- Owner / starting year / license source: <explicit values or unresolved decision>

## Check result
- Checked: <count>
- Status: clean | findings | command error
- Findings: <path, code, message, fixable>

## Changes
- Changed: <paths or none>
- Unresolved: <paths and reasons or none>
- Validation: <recheck result and targeted diff status>

## Remaining gate
- <only a real unresolved decision, external action, or unrun check>
```

Omit the Changes section for a read-only request. Clearly separate local evidence from unrun CI, published-package, remote-Action, or live-provider checks.
