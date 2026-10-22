---
description: Move an addlicense check to Lint My Headers and validate owner, notice, and year policy.
---

# Migrate from addlicense

[addlicense](https://github.com/google/addlicense) can add a header or check
that one is present. Lint My Headers (LMH) also checks whether a leading header
matches the owner, notice, and year policy you declare. `lmh check` is read-only.

Replace an `addlicense -check` command with a policy and a check. Use the
project's real owner, notice, and selected source paths:

```toml
[tool.lint-my-headers]
owner = "Example Organization"
starting-year = 2024
license = "Apache-2.0"
languages = ["python", "javascript", "typescript", "rust", "go", "swift", "bash", "c", "cpp"]
paths = ["src", "tests"]
ignore-files = ["generated.py"]
ignore-folders = ["vendor", "build"]
```

Then run:

```shell
lmh check
```

`owner` replaces `addlicense -c`. `license` uses an SPDX identifier such as
`Apache-2.0` or `MIT`, rather than addlicense's short license name. Use
`license-notice = "NOTICE.txt"` when the project needs a custom notice file.
LMH checks that each supported leading header has the declared owner and notice,
and that its year is valid. `addlicense -check` checks header presence.

Do not copy `addlicense -y` directly to `starting-year`. `starting-year` is the
earliest accepted copyright year. By default, LMH checks recognized end years
against the current year; `lmh fix` can refresh one eligible stale end year
while keeping the start year and other bytes.

For files without headers, declare their actual first copyright year before
inserting them. The policy must already supply the owner and notice:

```shell
lmh fix --creation-year 2024 src/new_file.py
lmh check src/new_file.py
git diff
```

Use one `creation-year` per group of files that really share that year. Without
it, `fix` leaves missing headers for review.

`addlicense -ignore` accepts glob patterns. LMH exclusions are explicit:
`ignore-files` matches exact basenames and `ignore-folders` excludes subtrees.
Set `paths` to the source directories you want checked. Version 0.8.0 does not
infer exclusions from `.gitignore`. See [configuration](configuration.md) for
the full option list and supported header layouts.
