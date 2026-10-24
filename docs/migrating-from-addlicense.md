---
description: Move an addlicense check to Lint My Headers using your declared header policy.
---

# Migrate from addlicense

[Declare your project's established policy](getting-started.md#declare-the-policy),
then replace `addlicense -check` with `lmh check`. LMH checks the owner, notice,
and year policy; `check` never writes files.

| addlicense | Lint My Headers |
| --- | --- |
| `-c "Owner"` | `owner = "Owner"` in configuration |
| `-l apache` | `license = "Apache-2.0"`, using an SPDX identifier |
| `-ignore` glob | Configured `paths`, exact `ignore-files` names, or `ignore-folders` subtrees ([details](configuration.md)) |

Do not copy `addlicense -y` to `starting-year`: that setting is the earliest
accepted year, not the file's creation year. To insert a missing header, declare
its actual first copyright year; the policy must already supply the owner and notice:

```shell
lmh fix --creation-year 2024 src/new_file.py
lmh check src/new_file.py
git diff
```

Run separately for files with different creation years. Without `creation-year`,
`fix` leaves missing headers for review. Existing eligible headers can have their
stale end year refreshed with `lmh fix`.
