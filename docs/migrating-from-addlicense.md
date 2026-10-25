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
| `-l apache` | `license = "Apache-2.0"` (SPDX identifier); check notice compatibility below |
| `-ignore` glob | Configured `paths`, exact `ignore-files` names, or `ignore-folders` subtrees ([details](configuration.md)) |

The `license` setting requires a project-root `LICENSE` file. Existing addlicense
boilerplate may not match LMH's notices. To retain it, copy the notice prose unchanged
(preserve blank lines; omit copyright and comment markers) into `NOTICE.txt`.
Set `license-notice = "NOTICE.txt"` instead of `license`, and check a representative
existing file before switching CI.

Do not copy `addlicense -y` to `starting-year`: that is an earliest accepted year,
not the file's creation year. For missing headers, declare their actual
`creation-year` and follow [the insertion guide](getting-started.md#insert-a-missing-header).
