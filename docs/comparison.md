---
description: Compare Lint My Headers with HawkEye, addlicense, License Eye, REUSE, licenseheaders, and NWA by workflow and edit scope.
---

# Compare header tools

Choose a tool by the work your project needs: checking an existing policy,
adding missing headers, formatting templates, or recording per-file licensing.
These jobs overlap, but their edit scopes differ.

## Workflow and edit scope

This comparison describes documented capabilities, reviewed on **6 October
2026**. Each tool name links to its upstream documentation. It is not a test of
every tool or a ranking of their safety or speed.

| Tool | Checks and configuration | Available edits |
| --- | --- | --- |
| **[Lint My Headers](configuration.md)** | Checks an explicit owner, year, and license-notice policy across nine languages; provides JSON findings, pre-commit/prek hooks, and a GitHub Action. | Refreshes one recognized stale year for the configured owner. Preserves the creation year, other bytes, and file mode; refuses ambiguous or unsafe targets. |
| **[HawkEye](https://github.com/fast/hawkeye#configuration)** | Checks configured templates; supports line/block comment styles, file patterns, Git ignore rules, and optional Git-derived template values. | Adds missing headers, replaces recognized non-canonical headers, or removes recognized headers. Offers dry runs. |
| **[addlicense](https://github.com/google/addlicense#usage)** | Checks header presence; accepts owner, years, a license/custom header file, SPDX options, and ignore patterns. | Inserts missing headers and leaves recognized existing headers in place. |
| **[License Eye](https://github.com/apache/skywalking-eyes#usage)** | Checks configured headers and shows mismatches; separate commands resolve or check dependency licenses. Offers CLI and GitHub Actions workflows. | Adds configured headers where missing; can collect dependency license texts and generate summaries. |
| **[REUSE](https://reuse.software/tutorial/)** | Checks the REUSE convention for per-file copyright and licensing. Uses SPDX identifiers/expressions, license texts, and declarations for non-code files too. | Annotates files with licensing declarations and downloads license texts. Supports multiple copyright holders. |
| **[licenseheaders](https://github.com/johann-petrak/licenseheaders#usage)** | Uses built-in/custom templates, explicit years, owners, and file selection. Offers a dry run and pre-commit recipe. | With a template, adds or replaces headers. With years and no template, replaces years in existing headers. |
| **[NWA](https://github.com/B1NARY-GR0UP/nwa#usage)** | Checks headers against templates; supports file patterns, comment-style settings, SPDX options, and optional year-insensitive checks. | Adds, updates, or removes headers. Offers dry runs; its README documents layout limits for updates. |

## When Lint My Headers fits

LMH fits repositories that already use its supported prose-header layout and
want read-only checks plus limited year repairs. Version **0.7.0** supports
**Python, JavaScript, TypeScript, Rust, Go, Swift, Bash, C, and C++**. Select the
languages in your policy; the default is Python.

Its [configuration guide](configuration.md#header-layouts) specifies the exact
copyright line, notice text, separators, and language preambles. Bash and Python
use `#`; the other supported languages use ordinary `//` comments. Leading
copyright-bearing block/doc comments refuse repair. Set source paths and
exclusions explicitly; LMH does not apply `.gitignore` rules.

Missing headers require manual insertion and review. Ownership, licensing, and
year updates follow the project's declared policy. LMH does not choose a
license, infer owners or creation years, audit dependency licenses, or certify
legal, SPDX, or REUSE compliance.

## Choose another workflow when needed

- **Insert missing headers:** consider addlicense or a template-based tool above.
- **Rewrite or remove headers:** compare HawkEye, NWA, and licenseheaders against
  your existing comment styles and templates.
- **Follow REUSE:** use the REUSE tool to annotate and lint declarations,
  including non-code assets and multiple holders.
- **Inspect dependency licenses:** consider License Eye's dependency commands.

Review each tool's configuration before enabling writes. Output from another
tool is not automatically compatible with LMH. Compare a representative header
with LMH's documented layout before migrating; interoperability was not tested
for this comparison.

## Measured performance

The [performance guide](benchmarks.md) contains the recorded LMH versus HawkEye
7.2.0 timings, environment, raw samples, and reproduction steps. In that warm-cache
synthetic workload, LMH had lower check medians and a higher repair median. The
tools' write guarantees differ, and installation/hook startup are excluded.

No timings for the other alternatives were collected. That benchmark does not
establish a universal speed ranking or equivalent edit behavior.
