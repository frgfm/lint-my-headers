# Contributor guidance

`lint-my-headers` is a Rust CLI with a thin optional Python 3.11+ launcher. Keep changes focused and preserve these public contracts:

- commands `lmh` and `lint-my-headers` with `check` and `fix`;
- configuration `[tool.lint-my-headers]`;
- exits 0 clean, 1 findings, 2 invocation/configuration/I/O failure;
- JSON schema version 1 and `LMH` diagnostic meanings;
- GitHub Action inputs plus `issues` and `changed` outputs.

Runtime logic belongs in `rust/`; never add a second Python parser. Keep Cargo/PyPI versions aligned, preserve the SPDX snapshots, and retain the renamed agent skill and evaluation assets.

Keep every PR's net diff to the smallest complete change. Reuse fixtures and test each parser case at its owning layer; CLI tests cover command and file-write contracts. Preserve unique safety regressions and check the diff size before handoff.

`check` must never write. `fix` may refresh one recognized stale end year or insert a missing header only with an explicitly declared owner, notice, and creation year. Preserve existing source bytes and mode. Refuse ambiguous, symlinked, reparse-point, multi-link, or concurrently changed targets. Never infer an owner, license, starting year, creation year, or legal conclusion.

Run before handoff:

```shell
make test
make quality
make package-check
uv run --no-sync --group quality prek run --all-files
git diff --check
```

PyPI, GitHub releases, Marketplace changes, repository renames, downstream migrations, commits, and pushes require separate explicit authorization.
