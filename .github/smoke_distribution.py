# Copyright (C) 2026, François-Guillaume Fernandez.

# This program is licensed under the Apache License 2.0.
# See LICENSE or go to <https://www.apache.org/licenses/LICENSE-2.0> for full license details.

import importlib.metadata
import importlib.resources
import json
import os
import shutil
import subprocess  # ruff: ignore[suspicious-subprocess-import] - fixed local executables
import sys
from datetime import datetime
from pathlib import Path
from tempfile import TemporaryDirectory

PACKAGE = "lint-my-headers"


def fail(message: str) -> None:
    raise RuntimeError(message)


def run(command: list[str], cwd: Path | None = None) -> subprocess.CompletedProcess:
    return subprocess.run(  # ruff: ignore[subprocess-without-shell-equals-true] - fixed executable paths
        command,
        cwd=cwd,
        check=False,
        capture_output=True,
        text=True,
    )


def require_command(name: str) -> str:
    executable = shutil.which(name)
    if executable is None:
        fail(f"Missing installed command: {name}")
    return executable


def verify_version(command: list[str], expected: str) -> None:
    result = run([*command, "--version"])
    if result.returncode != 0 or not result.stdout.rstrip().endswith(expected):
        fail(f"Version smoke failed for {' '.join(command)}: {result.stdout}{result.stderr}")


def main() -> int:
    expected = os.environ.get("EXPECTED_VERSION")
    if not expected:
        fail("EXPECTED_VERSION is required")
    installed = importlib.metadata.version(PACKAGE)
    if installed != expected:
        fail(f"Installed version mismatch: expected {expected}, got {installed}")
    commands = {
        entry.name
        for entry in importlib.metadata.distribution(PACKAGE).entry_points
        if entry.group == "console_scripts"
    }
    if commands:
        fail(f"Expected native executables, found Python console wrappers: {commands}")
    if any(path.parts[0] == "tests" for path in importlib.metadata.files(PACKAGE) or ()):
        fail("Distribution unexpectedly contains the test suite")

    verify_version([require_command("lmh")], expected)
    verify_version([require_command("lint-my-headers")], expected)
    verify_version([sys.executable, "-m", "lint_my_headers"], expected)
    verify_version(
        [sys.executable, "-c", "from lint_my_headers import main; print('returned', main(['--version']))"], "returned 0"
    )
    for name in ("lmh", "lint-my-headers"):
        signature = Path(require_command(name)).read_bytes()[:4]
        if signature not in (b"\x7fELF", b"\xcf\xfa\xed\xfe", b"\xfe\xed\xfa\xcf") and not signature.startswith(b"MZ"):
            fail(f"{name} is not a native executable")

    package_files = importlib.resources.files("lint_my_headers")
    for name in ("supported-licenses.json", "legacy-license-notices.json"):
        if not package_files.joinpath(name).is_file():
            fail(f"Missing package data: {name}")

    with TemporaryDirectory() as temporary_directory:
        root = Path(temporary_directory)
        root.joinpath("LICENSE").write_text("Apache-2.0\n", encoding="utf-8")
        root.joinpath(".lmh.toml").write_text(
            """owner = "Example Owner"
starting-year = 2022
license = "Apache-2.0"
languages = ["python", "javascript", "typescript", "rust", "go", "swift", "bash", "c", "cpp"]
paths = ["src"]
ignore-files = []
ignore-folders = []
""",
            encoding="utf-8",
        )
        year = datetime.now().year
        for checked, (extension, marker, body) in enumerate(
            (
                ("py", "#", "value = 1"),
                ("js", "//", "value = 1"),
                ("tsx", "//", "value = 1"),
                ("rs", "//", "const VALUE: i32 = 1;"),
                ("go", "//", "package example\nconst value = 1"),
                ("swift", "//", "let value = 1"),
                ("sh", "#", "value=1"),
                ("bash", "#", "values=(one two)"),
                ("c", "//", "int value = 1;"),
                ("cpp", "//", "template <typename T> T value(T x) { return x; }"),
                ("h", "//", "#pragma once\nint value(void);"),
                ("hpp", "//", "#pragma once\ntemplate <typename T> T value(T x) { return x; }"),
            ),
            1,
        ):
            source = root / f"src/example.{extension}"
            source.parent.mkdir(exist_ok=True)
            source.write_text(
                f"""{marker} Copyright (C) {year}, Example Owner.

{marker} This program is licensed under the Apache License 2.0.
{marker} See LICENSE or go to <https://www.apache.org/licenses/LICENSE-2.0> for full license details.

{body}
""",
                encoding="utf-8",
            )
            result = run([require_command("lmh"), "check", "--output-format", "json"], root)
            if result.returncode != 0:
                fail(f"Installed check failed: {result.stdout}{result.stderr}")
            payload = json.loads(result.stdout)
            if (payload["checked"], payload["diagnostics"], payload["error"]) != (checked, [], None):
                fail(f"Unexpected installed check result: {payload}")
            stale = source.read_bytes().replace(str(year).encode(), str(year - 1).encode(), 1)
            source.write_bytes(stale)
            result = run([require_command("lmh"), "fix", "--output-format", "json"], root)
            if result.returncode != 0 or json.loads(result.stdout)["changed"] != [f"src/example.{extension}"]:
                fail(f"Installed fix failed: {result.stdout}{result.stderr}")
            repaired = stale.replace(str(year - 1).encode(), f"{year - 1}-{year}".encode(), 1)
            if source.read_bytes() != repaired:
                fail("Installed fix changed unexpected bytes")

        missing = root / "src/new.py"
        body = b"value = 2\n"
        missing.write_bytes(body)
        command = [require_command("lmh"), "fix", "--output-format", "json"]
        result = run([*command, "src/new.py"], root)
        if result.returncode != 1 or json.loads(result.stdout)["diagnostics"][0]["fixable"]:
            fail("Installed fix must require a declared creation year for insertion")
        if missing.read_bytes() != body:
            fail("Installed fix inserted a header without a declared creation year")
        result = run([*command, "--creation-year", str(year - 2), "src/new.py"], root)
        if result.returncode != 0 or json.loads(result.stdout)["changed"] != ["src/new.py"]:
            fail(f"Installed insertion failed: {result.stdout}{result.stderr}")
        header = (
            f"# Copyright (C) {year - 2}-{year}, Example Owner.\n\n"
            "# This program is licensed under the Apache License 2.0.\n"
            "# See LICENSE or go to <https://www.apache.org/licenses/LICENSE-2.0> for full license details.\n\n"
        ).encode()
        if missing.read_bytes() != header + body:
            fail("Installed insertion changed unexpected bytes")

        block = root / "src/spdx.c"
        original = (
            f"/*\n * SPDX-FileCopyrightText: {year - 2} Example Owner\n"
            " * SPDX-License-Identifier: Apache-2.0\n */\n\nint value = 1;\n"
        ).encode()
        block.write_bytes(original)
        result = run([*command, "src/spdx.c"], root)
        if result.returncode != 0 or json.loads(result.stdout)["changed"] != ["src/spdx.c"]:
            fail(f"Installed SPDX block repair failed: {result.stdout}{result.stderr}")
        if block.read_bytes() != original.replace(str(year - 2).encode(), f"{year - 2}-{year}".encode(), 1):
            fail("Installed SPDX block repair changed unexpected bytes")
        result = run([require_command("lmh"), "check", "--output-format", "json"], root)
        if result.returncode != 0 or json.loads(result.stdout)["checked"] != checked + 2:
            fail(f"Installed post-repair check failed: {result.stdout}{result.stderr}")

    sys.stdout.write(f"Distribution smoke passed for {PACKAGE} {expected}.\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
