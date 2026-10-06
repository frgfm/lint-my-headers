---
description: Compare header-tool features, measured latency, throughput, peak process memory, runtime requirements, installation steps, and disk footprint.
---

# Compare header tools

First choose the checks and edits you need. Then compare their runtime cost.
A presence check, a fixed header policy, and a REUSE audit answer different questions.

## Features that change the choice

These capabilities describe the versions below, reviewed on **6 October 2026**.
Tool names link to the version's upstream documentation. The behavior tests and
performance measurements later on this page are our own results.

| Tool | Read-only check | Insert missing headers | Change existing headers | Remove headers |
| --- | --- | --- | --- | --- |
| **[LMH 0.7.0](configuration.md)** | Declared owner, years, and notice. | No. Reports missing headers. | Refresh one eligible end year. | No. |
| **[HawkEye 7.2.0](https://github.com/fast/hawkeye/tree/v7.2.0)** | Match a configured template. | Yes. | Replace a recognized header with the template. | Yes, recognized headers. |
| **[addlicense 1.2.0](https://github.com/google/addlicense/tree/v1.2.0)** | Check header presence. | Yes. | Leaves recognized existing headers in place. | No. |
| **[License Eye 0.9.0](https://github.com/apache/skywalking-eyes/tree/v0.9.0)** | Match header text or patterns. Also has dependency license checks. | Yes. | Can rewrite recognized headers with an existing-header pattern and new content. | No dedicated command. |
| **[REUSE 6.2.0](https://pypi.org/project/reuse/6.2.0/)** | Check per-file declarations and license texts against the REUSE specification. | Yes, with annotations. | Add or merge copyright and license declarations. | No dedicated command. |
| **[licenseheaders 0.8.8](https://github.com/johann-petrak/licenseheaders/tree/v0.8.8)** | Dry run of proposed edits; no dedicated checking command. | Yes. | Apply templates, or replace the year field without a template. | No dedicated command. |
| **[NWA 0.8.0](https://github.com/B1NARY-GR0UP/nwa/tree/v0.8.0)** | Match a template; optional year-insensitive matching. | Yes. | Rewrite headers; update has documented layout limits. | Yes. |

**LMH's edit limit is deliberate:** it keeps the creation year, other bytes, and
file mode. It refuses ambiguous, linked, or concurrently changed targets.
A template replacement can change more text. Use it when that is the job you need.

| Tool | Configuration | JSON check output | SPDX support | Other useful features |
| --- | --- | --- | --- | --- |
| LMH | TOML or existing Python/Cargo/JavaScript manifests. | Yes; versioned diagnostics. | 0.7.0 can use custom notice text; no built-in SPDX copyright/license pair parsing. | First-party pre-commit/prek hook and GitHub Action. Explicit exclusions; no Git ignore rules. |
| HawkEye | TOML, templates and styles. | Yes. | Can match SPDX text in a template. | Git ignore rules, optional Git history values, pre-commit hooks. |
| addlicense | CLI flags, custom header file, ignore patterns. | No dedicated JSON checker output. | Can emit SPDX license identifiers. | First-party pre-commit hook; small native binary. |
| License Eye | YAML, header text/patterns, comment styles. | No dedicated JSON header-check output. | SPDX license templates and dependency identification. | Git ignore rules, header diffs, dependency summaries, GitHub Actions. |
| REUSE | In-file declarations, sidecar files, REUSE.toml. | Yes. | Structured identifiers/expressions and license texts. | Non-code assets, multiple holders, SPDX bill of materials, CI integration. |
| licenseheaders | CLI flags, settings and templates. | No dedicated JSON checker output. | Can emit custom SPDX template text. | Dry runs, backups, extension mappings, pre-commit recipe. |
| NWA | CLI flags or YAML, templates and styles. | No dedicated JSON checker output. | SPDX identifiers and custom templates. | Edit dry runs, header diffs, pre-commit hooks, GitHub Actions. |

Matching SPDX text is not the same job as checking REUSE declarations, license
files, or dependency license compatibility. See the
[REUSE tutorial](https://reuse.software/tutorial/) for that workflow.

### Which changes does the check detect?

We tested one clean Python source, then removed its header, changed its owner,
or made its end year stale. Template checkers used a fixed policy: owner
**Benchmark**, years **2024–2026**, and an Apache-2.0 notice. REUSE used its
accepted SPDX declarations and a license text. Every check kept source bytes unchanged.

| Tool | Clean file passes | Missing header fails | Different owner fails | Stale end year fails |
| --- | --- | --- | --- | --- |
| LMH | Yes | Yes | Yes | Yes |
| HawkEye | Yes | Yes | Yes | Yes |
| addlicense | Yes | Yes | No | No |
| License Eye | Yes | Yes | Yes | Yes |
| REUSE | Yes | Yes | No | No |
| NWA | Yes | Yes | Yes | Yes |

These are results for the recorded configurations, not every available mode.
addlicense checks presence. REUSE accepts valid declarations by other holders
and earlier years; it does not enforce this project's fixed owner/year policy.
licenseheaders has no equivalent check command, so it is absent from this test.
[Raw behavior results](assets/benchmarks/tool-comparison-probes.json).

## Latency, throughput, and memory

**Measured on 6 October 2026:** Apple M3 Pro, macOS 15.7.7, 11 logical CPUs,
APFS. Each selected Python file is **1 KiB** and has one function plus padding.
The tools receive the same declared owner, years, license, file count, and
code body, in their accepted header layouts.

Latency is elapsed time for a fresh CLI process to finish a clean check,
including startup and output. Throughput is 10,000 files divided by that case's
median time. Peak RSS is the maximum process memory reported by the OS in
separate runs. The RSS column below is for the 10,000-file case.

### Fixed header policy

| Tool | 10 files | 1,000 files | 10,000 files | Bulk files/s | Peak RSS |
| --- | ---: | ---: | ---: | ---: | ---: |
| LMH 0.7.0 | 4.7 ms | 11.4 ms | 153.9 ms | 64,972 | 16.0 MiB |
| HawkEye 7.2.0 | 3.8 ms | 21.5 ms | 243.9 ms | 40,997 | 6.9 MiB |
| License Eye 0.9.0 | 19.5 ms | 262.0 ms | 2339.7 ms | 4,274 | 61.7 MiB |
| NWA 0.8.0 | 5.2 ms | 47.7 ms | 624.5 ms | 16,014 | 49.7 MiB |

### Different check scopes

| Tool | 10 files | 1,000 files | 10,000 files | Bulk files/s | Peak RSS |
| --- | ---: | ---: | ---: | ---: | ---: |
| addlicense 1.2.0 | 3.0 ms | 12.0 ms | 424.6 ms | 23,551 | 24.0 MiB |
| REUSE 6.2.0 | 333.0 ms | 400.1 ms | 811.0 ms | 12,331 | 98.8 MiB |

addlicense does less validation. REUSE also checks per-file declarations and
license texts. Compare these costs with the work you need; the rows do not
establish a speed ranking for equivalent checks.

HawkEye uses less peak process memory than LMH in this bulk run. LMH has the
highest measured bulk throughput among the fixed-policy checkers. HawkEye has
lower 10-file latency. Installation language alone does not explain these tradeoffs.

For REUSE, the RSS figure does **not** sum the memory of its parallel workers.
It must not be treated as the total memory budget for the entire job.

### Method and limits

- Seven latency trials after one discarded warmup. Each starts a fresh process.
  Filesystem caches are warm; no persistent result cache is used.
- Seven separate OS time invocations measure peak process RSS. The benchmark
  driver and installation are outside that measurement.
- Tool order rotates and reverses. Tools use normal CPU/thread defaults on
  the same machine. HawkEye's Git-derived values and ignore rules are disabled.
- Before timing each size, a missing header in the final directory must fail.
  SHA-256 hashes verify that checks leave source bytes unchanged.
- Fixtures, coverage tests and hash checks stay outside timing. Ranges are in
  the CSV; shared desktop load and filesystem costs can vary.
- These are clean Python checks. They do not measure mixed-language projects,
  repairs, hook orchestration, CI queueing, dependency audits, or cold storage.
  The [earlier Linux benchmark](benchmarks.md) measures nine languages and
  LMH/HawkEye repair tasks; its timings are a separate snapshot.

[Summary CSV](assets/benchmarks/tool-comparison-results.csv) ·
[Every timing and RSS sample](assets/benchmarks/tool-comparison-samples.csv) ·
[Versions, dependencies, commands, accepted headers and environment](assets/benchmarks/tool-comparison-environment.json)

## Runtime, installation, and disk space

Rust and Go tools run as compiled executables. Their compilers are needed only
for source builds. Python tools need a Python interpreter at runtime.

The disk figures below are **measured installed payloads on this Mac**.
Native rows count the binary. Python installation rows count the isolated
environment's regular files, including dependencies and bytecode. Shared
Python, compilers, build/package caches and fixtures are excluded. Different
platforms, source-build flags and package versions can change sizes.

| Tool | Implementation | Runtime or setup requirement | Measured payload | Measured installation |
| --- | --- | --- | ---: | --- |
| LMH 0.7.0 | Rust; optional Python launcher. | No Python needed to run the native binary. PyPI installation needs Python 3.11+. | 6.3 MiB | Published macOS arm64 wheel in an isolated environment. |
| HawkEye 7.2.0 | Rust. | Native executable; no Rust toolchain at runtime. | 10.9 MiB | Upstream macOS arm64 release binary. |
| addlicense 1.2.0 | Go. | Native executable; no Go toolchain at runtime. | 2.9 MiB | Upstream macOS arm64 release binary. |
| License Eye 0.9.0 | Go. | Native header checker. Dependency commands also need project tools. | 28.0 MiB | Source tag v0.9.0, Go 1.26.1, default build flags. |
| REUSE 6.2.0 | Python. | Python 3.10+ and an encoding backend; this run uses charset-normalizer. | 8.8 MiB | Python 3.11.13 environment, including dependencies. |
| licenseheaders 0.8.8 | Python. | Python and regex; this run uses Python 3.11.13. | 1.7 MiB | Isolated environment, including regex. |
| NWA 0.8.0 | Go. | Native executable; no Go toolchain at runtime. | 7.1 MiB | Upstream macOS arm64 release binary. |

These are allocated regular-file bytes, rounded to MiB. They are not download
sizes or the space needed to install a compiler. License Eye's source build
reports **dev** in its version command; Go's embedded module metadata identifies
v0.9.0. Python package versions and binary hashes are recorded above.

### Installation steps

For HawkEye, addlicense and NWA without a compiler:

1. Open the [HawkEye 7.2.0](https://github.com/fast/hawkeye/releases/tag/v7.2.0),
   [addlicense 1.2.0](https://github.com/google/addlicense/releases/tag/v1.2.0), or
   [NWA 0.8.0](https://github.com/B1NARY-GR0UP/nwa/releases/tag/v0.8.0) release page.
2. Download the archive for your OS and CPU.
3. Unpack it and put its executable in a directory on your PATH.

For Python package installation, these pinned commands select the measured releases:

~~~shell
uv tool install lint-my-headers==0.7.0
uv tool install 'reuse[charset-normalizer]==6.2.0'
uv tool install --python 3.11 licenseheaders==0.8.8
~~~

The encoding extra let REUSE run here without a system libmagic installation.
A bare REUSE 6.2.0 install failed on this Mac before that extra was added.

With the relevant compiler installed, the native tools also have source-build routes:

~~~shell
cargo install hawkeye --version 7.2.0 --locked
go install github.com/google/addlicense@v1.2.0
go install github.com/apache/skywalking-eyes/cmd/license-eye@v0.9.0
go install github.com/B1NARY-GR0UP/nwa@v0.8.0
~~~

Source builds can produce different payload sizes from release binaries.
Native CLI timings do not include pre-commit's environment setup. For example,
HawkEye's Python pre-commit hook needs Rust on its first install.

## Reproduce the comparison

The driver at **scripts/compare_header_tools.py** uses Python 3.11+'s standard library
and the OS time command on macOS or Linux.
Install the pinned tools, then write their absolute executable paths to a JSON file:

~~~json
{
  "lmh": "/path/to/lmh",
  "hawkeye": "/path/to/hawkeye",
  "addlicense": "/path/to/addlicense",
  "license-eye": "/path/to/license-eye",
  "nwa": "/path/to/nwa",
  "reuse": "/path/to/reuse"
}
~~~

From the repository root:

~~~shell
python3 scripts/compare_header_tools.py --tools tools.json --license-file LICENSE \
  --output target/tool-comparison --files 10 1000 10000 --runs 7
~~~

The driver creates and removes its own temporary fixtures. Setup commands may
add headers only in those fixtures. It does not run edit commands on your
repository. The probe-only option runs behavior tests without timings.
No benchmark or plotting package is required.

## Where LMH fits

Choose LMH when you need a declared owner/year/notice policy, structured
diagnostics, and a limited year repair. Published 0.7.0 requires its documented
prose layout and explicit exclusions. It does not insert missing headers,
audit dependency licenses, infer ownership, or check the full REUSE convention.

[PR #113](https://github.com/frgfm/lint-my-headers/pull/113) adds ordinary block
comments and SPDX copyright/license pairs. Those changes are unreleased and
are not included in the 0.7.0 feature claims or timings above.

Choose a template tool when you need to insert or rewrite headers. Choose
REUSE when the per-file declaration convention is your requirement. Choose
License Eye when dependency license work is part of the job. Check a sample
header against [LMH's layouts](configuration.md#header-layouts) before migrating;
accepted layouts and write guarantees differ.
