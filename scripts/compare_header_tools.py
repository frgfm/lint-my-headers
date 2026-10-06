#!/usr/bin/env python3

# Copyright (C) 2026, François-Guillaume Fernandez.

# This program is licensed under the Apache License 2.0.
# See LICENSE or go to <https://www.apache.org/licenses/LICENSE-2.0> for full license details.

"""Measure installed header checkers on synthetic Python files; never real sources."""

# Explicit executable paths, no shell, and CLI progress output are intentional.
# ruff: file-ignore[suspicious-subprocess-import, subprocess-without-shell-equals-true, print]

import argparse
import csv
import datetime
import hashlib
import json
import os
import platform
import re
import shutil
import statistics
import subprocess
import tempfile
import time
from pathlib import Path

NOTICE = (
    "This program is licensed under the Apache License 2.0.\n"
    "See LICENSE or go to <https://www.apache.org/licenses/LICENSE-2.0> for full license details."
)
BODY = b"def benchmark():\n    return 1\n"
CHECKERS = ("lmh", "hawkeye", "addlicense", "license-eye", "nwa", "reuse")


def invoke(command: list[str], cwd: Path, log: Path, *, memory: bool = False) -> tuple[int, float, float | None, str]:
    """Time process launch and completion; profile RSS in a separate invocation."""
    profiled = list(command)
    if memory:
        profiled = (
            ["/usr/bin/time", "-l"]
            if platform.system() == "Darwin"
            else ["/usr/bin/time", "-f", "HEADER_BENCH_RSS_KIB=%M"]
        ) + profiled
    env = dict(os.environ, LC_ALL="C", TZ="UTC", NO_COLOR="1")
    for key in ("GITHUB_TOKEN", "GITHUB_OUTPUT", "GITHUB_ENV"):
        env.pop(key, None)
    with log.open("wb") as stream:
        start = time.perf_counter_ns()
        # ponytail: blocking wait avoids timing distortion; interrupt hung tools manually.
        result = subprocess.run(profiled, cwd=cwd, env=env, stdout=stream, stderr=stream, check=False)
        elapsed = (time.perf_counter_ns() - start) / 1_000_000
    text = log.read_text(errors="replace")
    rss = None
    if memory:
        pattern = (
            r"(\d+)\s+maximum resident set size" if platform.system() == "Darwin" else r"HEADER_BENCH_RSS_KIB=(\d+)"
        )
        match = re.search(pattern, text)
        if not match:
            raise RuntimeError(f"Missing RSS data: {text}")
        divisor = 1048576 if platform.system() == "Darwin" else 1024
        rss = int(match.group(1)) / divisor
    return result.returncode, elapsed, rss, text


def configure(tool: str, binary: str, fixture: Path, year: int, license_file: Path) -> tuple[bytes, list[str]]:
    """Use each tool's accepted header layout with the same declared policy."""
    source = fixture / "src" / "0" / "file0.py"
    source.parent.mkdir(parents=True)
    source.write_bytes(BODY)
    header = f"Copyright (C) {year - 2}-{year}, Benchmark.\n\n{NOTICE}\n"
    template = fixture / "HEADER.txt"
    template.write_text(header)
    if tool != "reuse":
        shutil.copyfile(license_file, fixture / "LICENSE")
    command = [binary]
    if tool == "lmh":
        config = fixture / ".lmh.toml"
        config.write_text(f'owner="Benchmark"\nstarting-year={year - 2}\nlicense="Apache-2.0"\npaths=["src"]\n')
        source.write_text(
            "\n".join("# " + line if line else "" for line in header.splitlines()) + "\n\n" + BODY.decode()
        )
        command += ["check", "--config", str(config)]
    elif tool == "hawkeye":
        config = fixture / ".licenserc.toml"
        config.write_text(
            "[header]\ntext="
            + json.dumps(header.strip())
            + '\n[files]\nroot="src"\n[git]\nignore="disable"\nfile_attrs="disable"\n'
        )
        setup = [binary, "format", "--config", str(config)]
        command += ["check", "--config", str(config)]
    elif tool == "addlicense":
        setup = [binary, "-f", str(template), "src"]
        command += ["-check", "-f", str(template), "src"]
    elif tool == "license-eye":
        config = fixture / ".licenserc.yaml"
        content = "\n".join("      " + line for line in header.splitlines())
        config.write_text(
            "header:\n  license:\n    content: |\n" + content + '\n  paths: ["src/**"]\n  comment: never\n'
        )
        setup = [binary, "--config", str(config), "header", "fix"]
        command += ["--config", str(config), "header", "check"]
    elif tool == "nwa":
        setup = [binary, "add", "--tmpltype", "static", "--tmpl", str(template), "src/**/*.py"]
        command += ["check", "--tmpltype", "static", "--tmpl", str(template), "--no-color", "src/**/*.py"]
    else:
        template.unlink()
        licenses = fixture / "LICENSES"
        licenses.mkdir()
        shutil.copyfile(license_file, licenses / "Apache-2.0.txt")
        setup = [
            binary,
            "annotate",
            "--copyright",
            "Benchmark",
            "--year",
            f"{year - 2}-{year}",
            "--license",
            "Apache-2.0",
            str(source),
        ]
        command += ["lint"]
    if tool != "lmh":
        result = subprocess.run(setup, cwd=fixture, capture_output=True, text=True, check=False)
        if result.returncode:
            raise RuntimeError(f"{tool} setup: {result.stdout}\n{result.stderr}")
    return source.read_bytes(), command


def hashes(paths: list[Path]) -> list[str]:
    return [hashlib.sha256(path.read_bytes()).hexdigest() for path in paths]


def probe_headers(
    work: Path, tools: dict[str, str], year: int, license_file: Path, output: Path
) -> tuple[dict[str, bytes], dict[str, list[str]]]:
    probes, templates, commands = [], {}, {}
    for tool in CHECKERS:
        fixture = work / tool
        fixture.mkdir()
        template, command = configure(tool, tools[tool], fixture, year, license_file)
        templates[tool], commands[tool] = template, command
        source = fixture / "src" / "0" / "file0.py"
        states = {
            "clean": template,
            "missing": BODY,
            "wrong_owner": template.replace(b"Benchmark", b"OtherOwner"),
            "stale_year": template.replace(str(year).encode(), str(year - 1).encode(), 1),
        }
        for state, content in states.items():
            source.write_bytes(content)
            before = source.read_bytes()
            status, _, _, text = invoke(command, fixture, output / "probe.log")
            if source.read_bytes() != before:
                raise RuntimeError(f"{tool} check wrote source bytes")
            probes.append({"tool": tool, "case": state, "exit": status})
            if state == "clean" and status != 0:
                raise RuntimeError(f"{tool} rejects its clean fixture: {text}")
            if state == "missing" and status == 0:
                raise RuntimeError(f"{tool} skipped a selected source: {text}")
        source.write_bytes(template)
        (output / f"{tool}-header.txt").write_bytes(template.split(BODY)[0])
        print(f"Validated {tool}", flush=True)
    (output / "feature-probes.json").write_text(json.dumps(probes, indent=2) + "\n")
    return templates, commands


def prepare_sources(
    work: Path,
    templates: dict[str, bytes],
    commands: dict[str, list[str]],
    count: int,
    bytes_per_file: int,
    output: Path,
) -> tuple[dict[str, list[str]], dict[str, list[Path]]]:
    expected, paths_by_tool = {}, {}
    for tool in CHECKERS:
        fixture = work / tool
        shutil.rmtree(fixture / "src")
        template = templates[tool]
        if len(template) > bytes_per_file:
            raise RuntimeError(f"Increase --bytes; {tool} header is too large")
        content = template + b"\n" * (bytes_per_file - len(template))
        paths = []
        for index in range(count):
            path = fixture / "src" / str(index // 100) / f"file{index}.py"
            path.parent.mkdir(exist_ok=True, parents=True)
            path.write_bytes(content)
            paths.append(path)
        paths_by_tool[tool], expected[tool] = paths, hashes(paths)
        # A missing header in the final directory must also be checked.
        last = paths[-1]
        original = last.read_bytes()
        last.write_bytes(BODY)
        status, _, _, text = invoke(commands[tool], fixture, output / "coverage.log")
        if status != 1:
            raise RuntimeError(f"{tool} did not report the last selected file: {text}")
        last.write_bytes(original)
    return expected, paths_by_tool


def measure_files(
    work: Path,
    templates: dict[str, bytes],
    commands: dict[str, list[str]],
    counts: list[int],
    runs: int,
    bytes_per_file: int,
    output: Path,
) -> list[dict]:
    samples = []
    for count in counts:
        expected, paths_by_tool = prepare_sources(work, templates, commands, count, bytes_per_file, output)
        for run in range(runs + 1):
            order = list(CHECKERS)
            shift = run % len(order)
            order = order[shift:] + order[:shift]
            if run % 2:
                order.reverse()
            for tool in order:
                fixture = work / tool
                status, elapsed, _, text = invoke(commands[tool], fixture, output / "check.log")
                if status != 0:
                    raise RuntimeError(f"{tool}/{count} check failed: {text}")
                status, _, rss, text = invoke(commands[tool], fixture, output / "rss.log", memory=True)
                if status != 0:
                    raise RuntimeError(f"{tool}/{count} RSS run failed: {text}")
                if hashes(paths_by_tool[tool]) != expected[tool]:
                    raise RuntimeError(f"{tool} check changed sources")
                if run:
                    samples.append({
                        "tool": tool,
                        "files": count,
                        "bytes_per_file": bytes_per_file,
                        "run": run,
                        "elapsed_ms": elapsed,
                        "peak_rss_mib": rss,
                    })
            print(f"{count} files, trial {run}/{runs}", flush=True)
    return samples


def write_results(samples: list[dict], counts: list[int], bytes_per_file: int, output: Path) -> None:
    for filename, rows in (
        ("samples.csv", samples),
        (
            "results.csv",
            [
                {
                    "tool": tool,
                    "files": count,
                    "bytes_per_file": bytes_per_file,
                    "median_ms": statistics.median(
                        row["elapsed_ms"] for row in samples if row["tool"] == tool and row["files"] == count
                    ),
                    "min_ms": min(
                        row["elapsed_ms"] for row in samples if row["tool"] == tool and row["files"] == count
                    ),
                    "max_ms": max(
                        row["elapsed_ms"] for row in samples if row["tool"] == tool and row["files"] == count
                    ),
                    "peak_rss_mib": max(
                        row["peak_rss_mib"] for row in samples if row["tool"] == tool and row["files"] == count
                    ),
                }
                for tool in CHECKERS
                for count in counts
            ],
        ),
    ):
        if filename == "results.csv":
            for row in rows:
                row["files_per_s"] = row["files"] * 1000 / row["median_ms"]
        with (output / filename).open("w") as stream:
            writer = csv.DictWriter(stream, fieldnames=list(rows[0]))
            writer.writeheader()
            writer.writerows(rows)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tools", type=Path, required=True, help="JSON map of tool names to executable paths")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--license-file", type=Path, required=True)
    parser.add_argument("--files", type=int, nargs="+", default=[10, 1000, 10000])
    parser.add_argument("--runs", type=int, default=7)
    parser.add_argument("--bytes", type=int, default=1024)
    parser.add_argument("--probe-only", action="store_true")
    args = parser.parse_args()
    if min(args.files) < 1 or args.runs < 3 or args.bytes < 512:
        parser.error("Use positive file counts, at least 3 runs, and at least 512 bytes/file")
    tools = json.loads(args.tools.read_text())
    if not isinstance(tools, dict):
        parser.error("--tools must contain a JSON object")
    for tool in CHECKERS:
        value = tools.get(tool)
        if not isinstance(value, str) or not Path(value).is_absolute() or not os.access(value, os.X_OK):
            parser.error(f"Provide an absolute executable path for {tool}")
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    year = datetime.datetime.now(datetime.timezone.utc).year
    with tempfile.TemporaryDirectory(prefix="header-tools-") as temp:
        work = Path(temp)
        templates, commands = probe_headers(work, tools, year, args.license_file.resolve(), output)
        if args.probe_only:
            return
        samples = measure_files(work, templates, commands, args.files, args.runs, args.bytes, output)
    write_results(samples, args.files, args.bytes, output)
    metadata = {
        "recorded_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "platform": platform.platform(),
        "machine": platform.machine(),
        "python": platform.python_version(),
        "logical_cpus": os.cpu_count(),
        "runs": args.runs,
        "warmups": 1,
        "source_language": "Python",
        "current_year": year,
        "owner": "Benchmark",
        "harness_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "bytes_per_file": args.bytes,
        "clock": "perf_counter_ns; process launch through completion",
        "rss": "separate OS time invocations; maximum process RSS, not summed process-tree RSS",
        "commands": {
            tool: [Path(commands[tool][0]).name] + [str(a).replace(str(work), "$FIXTURE") for a in commands[tool][1:]]
            for tool in CHECKERS
        },
        "binary_sha256": {tool: hashlib.sha256(Path(tools[tool]).read_bytes()).hexdigest() for tool in CHECKERS},
    }
    (output / "environment.json").write_text(json.dumps(metadata, indent=2) + "\n")


if __name__ == "__main__":
    main()
