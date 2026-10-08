---
description: Measured check overhead, full-codebase year repair time and peak memory, with reproducible comparisons against HawkEye.
---

# What will header maintenance cost?

This page records the earlier Linux LMH/HawkEye source benchmark. The new
[tool comparison](comparison.md#latency-throughput-and-memory) measures published
CLI checks, throughput, peak process memory and installation payloads on macOS.
The two snapshots use different workloads and hardware.

A check adds its measured runtime to a serial development workflow. A full year
refresh takes the repair time below when every selected file has an eligible
stale header. Missing or conflicting headers still need review. Select a chart
to view it at full size.

Each panel has its own vertical scale starting at zero. Bar heights show the
relative gap at that codebase size; labels show median time in milliseconds.

## How much waiting does a check add?

[![Native check latency across codebase sizes](https://github.com/frgfm/lint-my-headers/releases/download/benchmark-5152bf26fb17/check.png)](https://github.com/frgfm/lint-my-headers/releases/download/benchmark-5152bf26fb17/check.png)

For **10,000 mixed-language files**, a routine check adds **36 ms** in this run: **0.36%** on top of a 10-second serial workflow. A check reporting 10,000 stale headers takes **62 ms**.

| Task | Previous LMH | LMH now | HawkEye 7.2.0 |
| --- | --- | ---: | ---: |
| Routine check | 120 ms | 36 ms | 76 ms |
| Check stale headers | 134 ms | 62 ms | 76 ms |
| Repair stale years | 428 ms | 147 ms | 125 ms |

For your workflow, use `check time / existing workflow time × 100` to estimate
the percentage added. The generated HTML report lets you enter your own duration.
Installation, hook orchestration and CI queueing are outside these measurements.

## How long will an entire codebase take to fix?

[![Time to repair stale years across a codebase](https://github.com/frgfm/lint-my-headers/releases/download/benchmark-5152bf26fb17/fix.png)](https://github.com/frgfm/lint-my-headers/releases/download/benchmark-5152bf26fb17/fix.png)

Repairing stale years in all **10,000 files** takes **147 ms**, down from **428 ms** in the previous LMH. HawkEye takes **125 ms**: LMH wins the two check cases here; its repair median remains **17.6% slower**.

Both tools get the same CPU budget. LMH uses up to four available workers for
trees of at least 256 files. LMH keeps year-only edits, exact-byte/identity
revalidation, mode preservation, atomic replacement and file sync. HawkEye
formats a fixed leading template and writes directly. These guarantees differ;
the comparison measures the stated tasks rather than equivalent feature sets.
CPU limits and filesystem write costs can change the ranking.

An earlier control with the same binary hashes restricted both tools to one CPU:
stale checks took 111.3 ms for LMH versus 74.9 ms for HawkEye, and repairs took
289.1 versus 121.9 ms. On the workspace overlay filesystem, 1,000-file repairs
took 165.5 versus 38.6 ms; its backing hardware is unspecified. That earlier Python-based
audit uses five timings and three RSS runs after warmup. See the
[CPU and storage results](assets/benchmarks/limits.csv) for all three tasks.

## How much memory does it need?

[![Peak native process memory across languages](https://github.com/frgfm/lint-my-headers/releases/download/benchmark-5152bf26fb17/memory.png)](https://github.com/frgfm/lint-my-headers/releases/download/benchmark-5152bf26fb17/memory.png)

Memory is peak native-process RSS, excluding the benchmark driver and compilation.
Worker threads increase check memory; planning edits before allocating replacement
bytes reduces repair memory compared with previous LMH.

## All nine languages

LMH medians for a separate 1,000-file tree in each language:

| Language | Routine check | Stale-header check | Repair all stale years |
| --- | --- | ---: | ---: |
| Python | 6 ms | 9 ms | 17 ms |
| JavaScript | 6 ms | 10 ms | 17 ms |
| TypeScript | 7 ms | 9 ms | 17 ms |
| Rust | 6 ms | 9 ms | 16 ms |
| Go | 6 ms | 10 ms | 17 ms |
| Swift | 6 ms | 10 ms | 17 ms |
| Bash | 6 ms | 9 ms | 16 ms |
| C | 6 ms | 9 ms | 14 ms |
| C++ | 7 ms | 10 ms | 17 ms |

## Method and recorded environment

Recorded **2026-10-02 UTC** from source [`59c4782`](https://github.com/frgfm/lint-my-headers/commit/59c4782cae2a35c40bcd3b75a7500f3317e0b9f5) versus previous source [`8d72bf1`](https://github.com/frgfm/lint-my-headers/commit/8d72bf1b81167a1e43e0e54e350e4e2351306bde) and locally built **HawkEye 7.2.0**. All use Rust 1.93.0 and locked release builds; LMH uses thin LTO, HawkEye its upstream release defaults. The shared runner is Linux x86_64, glibc 2.41, AMD EPYC 9V74. Fixtures use RAM-backed `/tmp` (tmpfs): **1 KiB/file**, mixing all nine languages. These measurements cover the source CLI, rather than the published 0.6.0 wheel.

Five timings per case follow one discarded warmup. Bash measures wall time with
millisecond precision; medians below 10 ms have limited resolution. Tool order
rotates and reverses deterministically; every timing launches a fresh native CLI.
Peak RSS is the maximum of
five separate GNU time invocations on restored fixtures. Generation, restoration,
validation and plotting stay outside timing. Each run verifies exit status,
checked counts, findings and final source SHA-256 hashes. Both tools receive identical code
statements, equal file counts/sizes, owner, license and years, using their accepted
comment separators. HawkEye's Git attributes are disabled. Every findings/repair
case has stale years in all files.

These are synthetic sources with warm filesystem caches and no persistent result
cache. They are examples of workflow costs on one shared runner; results vary by
hardware, file sizes, language mix and storage. Use your codebase's filesystem
before estimating disk repair time.

[Summary CSV](assets/benchmarks/results.csv) ·
[10,000-file comparison samples](assets/benchmarks/comparison-samples.csv) ·
[Versions, binary hashes and environment](assets/benchmarks/environment.json)

## Reproduce

From a checkout on Linux or macOS (Windows: WSL), install Git, the pinned Rust
toolchain and a native linker. The driver uses Bash, jq, gnuplot and GNU time:

```shell
# Debian/Ubuntu
sudo apt install jq gnuplot-nox time
# macOS
brew install jq gnuplot gnu-time
```

```shell
cargo install hawkeye --version 7.2.0 --locked --root target/hawkeye
bash scripts/benchmark.sh --compare target/hawkeye/bin/hawkeye
```

The script builds the locked release CLI, measures all nine languages and mixed
trees at 1, 100, 1,000 and 10,000 files, and writes CSVs, SVG/PNG charts and a
standalone `target/bench/report.html` with embedded charts. Open that report to
explore the slowdown for your workflow duration. Mixed cases below nine files
are skipped.

To include the previous LMH implementation used in this snapshot:

```shell
git fetch origin 8d72bf1b81167a1e43e0e54e350e4e2351306bde
git worktree add --detach /tmp/lmh-baseline 8d72bf1
cargo build --release --locked --manifest-path /tmp/lmh-baseline/Cargo.toml --target-dir /tmp/lmh-baseline/target --bin lmh
bash scripts/benchmark.sh --baseline /tmp/lmh-baseline/target/release/lmh --compare target/hawkeye/bin/hawkeye
```

Use `--files 1 100 --runs 3` for a quick run, `--bytes 16384` for larger sources,
or `--binary /path/to/lmh` for another native build. Set `TMPDIR` to an existing
directory on your codebase's filesystem to include its write costs. On Linux,
prefix the benchmark command with `taskset -c CPU` using one allowed CPU to
compare both tools on one CPU. `--help` lists all options.

Charts and the HTML report are GitHub release assets on a dedicated benchmark
prerelease; only small CSVs and environment metadata live in Git. The
[benchmark workflow](https://github.com/frgfm/lint-my-headers/actions/workflows/benchmark.yml)
renders committed snapshots on same-repository PRs and publishes their assets.
Its manual run measures the full suite and uploads a GitHub Actions artifact;
the optional `release` tag attaches that run to a selected existing release.
