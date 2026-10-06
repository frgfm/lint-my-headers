---
description: Interpret Lint My Headers diagnostics and exit codes, consume JSON schema version 1, and use the CLI safely in coding-agent tasks.
---

# Diagnostics & agents

Understand each finding, consume a stable JSON contract, and keep agent repairs within your declared policy.

| Exit code | Meaning |
| --- | --- |
| `0` | No unresolved findings. |
| `1` | Findings need review. |
| `2` | Invocation, configuration, or I/O failure. |

## Diagnostic codes

Text diagnostics include a sorted, project-relative path, line, column, code,
reason, and repair eligibility:

```text
src/example.py:1:1: LMH004 copyright year ends at 2025; expected 2026 [fixable]
```

Paths use `/`, including `..` for explicitly selected external files.

| Code | Meaning | Next step |
| --- | --- | --- |
| `LMH001` | Missing header. | Published 0.7.0 requires manual insertion. The unreleased source can use `fix` when a creation year is declared and `fixable` is true. |
| `LMH002` | Owner mismatch. | Verify the declared ownership manually. |
| `LMH003` | Invalid, reversed, future, or out-of-policy year. | Review the year policy and source. |
| `LMH004` | Recognized stale year. | Use `fix` only when `fixable` is true and writes are authorized. |
| `LMH005` | Missing or mismatched license notice. | Review the configured notice and source. |
| `LMH006` | Malformed, misplaced, duplicated, or ambiguous layout. | Resolve the layout manually. |
| `LMH007` | Unsupported encoding or invalid bytes. | Review the file's encoding. |
| `LMH008` | Unsafe or concurrently changed repair target. | Inspect links, identity, and concurrent writes. |
| `LMH900` | Command, configuration, or I/O error. | Stop and resolve the error; JSON places it in `error`. |

Exit **0** means clean, **1** means unresolved findings, and **2** means an
invocation/configuration/I/O failure. A failed repair may follow earlier completed
repairs, so inspect `changed` and the working-tree diff.

## JSON output

```shell
lmh check --output-format json
```

Successfully parsed JSON-mode commands write only JSON to stdout. Malformed CLI
syntax remains a stderr usage error. Schema version **1** is shared by `check`
and `fix`:

```json
{
  "schema_version": 1,
  "tool_version": "0.7.0",
  "command": "check",
  "config_path": "pyproject.toml",
  "checked": 1,
  "changed": [],
  "diagnostics": [
    {
      "path": "src/example.py",
      "line": 1,
      "column": 1,
      "code": "LMH004",
      "message": "copyright year ends at 2025; expected 2026",
      "fixable": true
    }
  ],
  "expected_header": "# Copyright (C) <FILE_CREATION_YEAR>-2026, Example Organization.\n\n# This program is licensed under the Apache License 2.0.\n# See LICENSE or go to <https://www.apache.org/licenses/LICENSE-2.0> for full license details.\n",
  "error": null
}
```

`expected_header` uses the first enabled language's comment marker. Its text and
blank lines are shared by all enabled languages. `changed` lists completed
writes; `diagnostics` lists unresolved findings. Command failures use `error`
instead of an `LMH900` diagnostic.

## Coding agents

Copy this instruction into an agent task:

```text
Read the declared owner, starting year, license, and paths; ask if any are missing.
Run lmh check --output-format json. Exit 1 means findings; exit 2 means stop.
Only run lmh fix when source changes are authorized, then recheck and inspect the
targeted diff. Never infer legal facts or install, commit, or push without permission.
```

The optional
[lint-my-headers agent skill](https://github.com/frgfm/lint-my-headers/blob/main/.agents/skills/lint-my-headers/SKILL.md)
uses the same contract. Its retained evaluation results are historical
Python-era evidence, not Rust evaluations.

An agent should use `fixable` and the exit status to guide repairs, and review
the actual diff before handoff. Tool output never authorizes choosing an owner,
license, or creation year.
