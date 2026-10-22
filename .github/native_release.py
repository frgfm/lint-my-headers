# Copyright (C) 2026, François-Guillaume Fernandez.

# This program is licensed under the Apache License 2.0.
# See LICENSE or go to <https://www.apache.org/licenses/LICENSE-2.0> for full license details.

import argparse
import gzip
import hashlib
import io
import json
import os
import re
import shutil
import subprocess  # ruff: ignore[suspicious-subprocess-import] - fixed GitHub CLI and extracted native executables
import tarfile
import zipfile
from datetime import date
from email.parser import BytesParser
from pathlib import Path, PurePosixPath
from tempfile import TemporaryDirectory

COMMANDS = ("lmh", "lint-my-headers")
TARGETS = {
    "x86_64-unknown-linux-gnu": ("manylinux_2_28_x86_64", ".tar.gz", "elf"),
    "aarch64-unknown-linux-gnu": ("manylinux_2_28_aarch64", ".tar.gz", "elf"),
    "x86_64-unknown-linux-musl": ("musllinux_1_2_x86_64", ".tar.gz", "elf"),
    "aarch64-unknown-linux-musl": ("musllinux_1_2_aarch64", ".tar.gz", "elf"),
    "x86_64-apple-darwin": ("_x86_64", ".tar.gz", "macho"),
    "aarch64-apple-darwin": ("_arm64", ".tar.gz", "macho"),
    "x86_64-pc-windows-msvc": ("win_amd64", ".zip", "pe"),
    "aarch64-pc-windows-msvc": ("win_arm64", ".zip", "pe"),
}


def fail(message: str) -> None:
    raise ValueError(message)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read_wheel(wheel: Path, target: str, version: str) -> dict[str, bytes]:
    if target not in TARGETS:
        fail(f"Unsupported native target: {target}")
    if wheel.is_dir():
        matches = sorted(wheel.glob("*.whl"))
        if len(matches) != 1:
            fail(f"Expected one wheel in {wheel}, found {len(matches)}")
        wheel = matches[0]
    if not wheel.is_file() or wheel.suffix != ".whl":
        fail(f"Expected one wheel file, got {wheel}")

    platform_tag = wheel.name[:-4].rsplit("-", 1)[-1]
    expected_platform, _, binary_format = TARGETS[target]
    platforms = platform_tag.split(".")
    if target.endswith("apple-darwin"):
        matches_platform = any(part.startswith("macosx_") and part.endswith(expected_platform) for part in platforms)
    else:
        matches_platform = expected_platform in platforms
    if not matches_platform:
        fail(f"Wheel {wheel.name} does not match target {target}")

    with zipfile.ZipFile(wheel) as archive:
        entries = archive.infolist()
        metadata_paths = [
            item.filename
            for item in entries
            if len(PurePosixPath(item.filename).parts) == 2
            and PurePosixPath(item.filename).parts[0].endswith(".dist-info")
            and PurePosixPath(item.filename).parts[1] == "METADATA"
        ]
        if len(metadata_paths) != 1:
            fail(f"Expected one wheel METADATA file, found {len(metadata_paths)}")
        metadata = BytesParser().parsebytes(archive.read(metadata_paths[0]))
        if re.sub(r"[-_.]+", "-", metadata.get("Name", "")).lower() != "lint-my-headers":
            fail(f"Unexpected wheel package: {metadata.get('Name')}")
        if metadata.get("Version") != version:
            fail(f"Wheel version {metadata.get('Version')} does not match release version {version}")

        suffix = ".exe" if binary_format == "pe" else ""
        payloads: dict[str, bytes] = {}
        for name in (*COMMANDS, "LICENSE"):
            matches = []
            for item in entries:
                parts = PurePosixPath(item.filename).parts
                if name == "LICENSE":
                    if len(parts) == 3 and parts[0].endswith(".dist-info") and parts[1:] == ("licenses", "LICENSE"):
                        matches.append(item)
                elif (
                    len(parts) == 3
                    and parts[0].endswith(".data")
                    and parts[1] == "scripts"
                    and parts[2] == f"{name}{suffix}"
                ):
                    matches.append(item)
            if len(matches) != 1:
                fail(f"Expected one wheel entry for {name}{suffix}, found {len(matches)}")
            item = matches[0]
            contents = archive.read(item)
            if name != "LICENSE":
                _validate_binary(contents, name, target)
                if binary_format != "pe" and (item.external_attr >> 16) & 0o111 == 0:
                    fail(f"Wheel executable is missing its executable mode: {item.filename}")
            payloads[f"{name}{suffix}" if name != "LICENSE" else name] = contents
        return payloads


def _validate_binary(contents: bytes, name: str, target: str) -> None:
    binary_format = TARGETS[target][2]
    valid = {
        "elf": contents.startswith(b"\x7fELF"),
        "macho": contents[:4] in (b"\xcf\xfa\xed\xfe", b"\xfe\xed\xfa\xcf"),
        "pe": contents.startswith(b"MZ"),
    }[binary_format]
    if not valid:
        fail(f"{name} is not a {binary_format} executable for {target}")


def archive_name(version: str, target: str) -> str:
    if not re.fullmatch(r"\d+\.\d+\.\d+(?:rc\d+)?", version):
        fail(f"Unsupported release version: {version}")
    try:
        extension = TARGETS[target][1]
    except KeyError:
        fail(f"Unsupported native target: {target}")
    return f"lint-my-headers-{version}-{target}{extension}"


def pack(wheel: Path, target: str, version: str, output: Path) -> Path:
    payloads = read_wheel(wheel, target, version)
    output.mkdir(parents=True, exist_ok=True)
    destination = output / archive_name(version, target)
    if destination.suffix == ".zip":
        with zipfile.ZipFile(destination, "w", compression=zipfile.ZIP_DEFLATED) as archive:
            for name, contents in payloads.items():
                info = zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0))
                info.create_system = 3
                info.external_attr = (0o100755 if name != "LICENSE" else 0o100644) << 16
                info.compress_type = zipfile.ZIP_DEFLATED
                archive.writestr(info, contents)
    else:
        with (
            destination.open("wb") as raw,
            gzip.GzipFile(fileobj=raw, mode="wb", filename="", mtime=0) as compressed,
            tarfile.open(fileobj=compressed, mode="w") as archive,
        ):
            for name, contents in payloads.items():
                info = tarfile.TarInfo(name)
                info.size = len(contents)
                info.mode = 0o755 if name != "LICENSE" else 0o644
                info.mtime = 0
                info.uid = info.gid = 0
                info.uname = info.gname = ""
                archive.addfile(info, io.BytesIO(contents))
    checksum = destination.with_name(f"{destination.name}.sha256")
    checksum.write_text(f"{sha256(destination)}  {destination.name}\n", encoding="ascii")
    return destination


def _expected_names(version: str) -> set[str]:
    return {
        name
        for target in TARGETS
        for name in (archive_name(version, target), f"{archive_name(version, target)}.sha256")
    }


def release_assets(directory: Path, version: str) -> list[Path]:
    if not directory.is_dir():
        fail(f"Native asset directory does not exist: {directory}")
    entries = list(directory.iterdir())
    files = sorted((path for path in entries if path.is_file()), key=lambda path: path.name)
    if len(files) != len(entries):
        fail(f"Native asset directory contains a non-file entry: {directory}")
    actual = {path.name for path in files}
    expected = _expected_names(version)
    if actual != expected:
        fail(
            f"Expected eight native archives and checksums for {version}; missing={sorted(expected - actual)}, extra={sorted(actual - expected)}"
        )
    for path in files:
        if path.name.endswith(".sha256"):
            archive = path.with_name(path.name.removesuffix(".sha256"))
            contents = path.read_text(encoding="ascii")
            if contents != f"{sha256(archive)}  {archive.name}\n":
                fail(f"SHA-256 checksum does not match {archive.name}")
    return files


def smoke(archive_path: Path, target: str, version: str) -> None:
    if archive_path.name != archive_name(version, target):
        fail(f"Unexpected native archive name: {archive_path.name}")
    checksum = archive_path.with_name(f"{archive_path.name}.sha256")
    expected_checksum = f"{sha256(archive_path)}  {archive_path.name}\n"
    if not checksum.is_file() or checksum.read_text(encoding="ascii") != expected_checksum:
        fail(f"Missing or invalid SHA-256 checksum for {archive_path.name}")

    windows = target.endswith("windows-msvc")
    expected_names = {"LICENSE", *(f"{name}{'.exe' if windows else ''}" for name in COMMANDS)}
    with TemporaryDirectory() as temporary:
        destination = Path(temporary)
        payloads: dict[str, bytes] = {}
        modes: dict[str, int] = {}
        if archive_path.suffix == ".zip":
            with zipfile.ZipFile(archive_path) as archive:
                for item in archive.infolist():
                    if item.is_dir() or item.filename not in expected_names or item.filename in payloads:
                        fail(f"Unexpected entry in {archive_path.name}: {item.filename}")
                    payloads[item.filename] = archive.read(item)
                    modes[item.filename] = item.external_attr >> 16 & 0o777
        else:
            with tarfile.open(archive_path, "r:gz") as archive:
                for item in archive.getmembers():
                    if not item.isfile() or item.name not in expected_names or item.name in payloads:
                        fail(f"Unexpected entry in {archive_path.name}: {item.name}")
                    stream = archive.extractfile(item)
                    if stream is None:
                        fail(f"Unable to read {item.name} from {archive_path.name}")
                    payloads[item.name] = stream.read()
                    modes[item.name] = item.mode
        if payloads.keys() != expected_names:
            fail(f"Incomplete native archive {archive_path.name}: found {sorted(payloads)}")

        suffix = ".exe" if target.endswith("windows-msvc") else ""
        for name in COMMANDS:
            filename = f"{name}{suffix}"
            _validate_binary(payloads[filename], name, target)
            executable = destination / filename
            if not windows and not modes[filename] & 0o111:
                fail(f"Native archive removed the executable mode from {filename}")
            executable.write_bytes(payloads[filename])
            if not suffix:
                executable.chmod(modes[filename] & 0o777)
        (destination / "LICENSE").write_bytes(payloads["LICENSE"])

        environment = os.environ.copy()
        environment["PATH"] = str(destination)
        project = destination / "smoke-project"
        source = project / "src" / "smoke.py"
        project.mkdir()
        source.parent.mkdir()
        year = date.today().year
        project.joinpath("LICENSE").write_text("Apache-2.0\n", encoding="utf-8")
        project.joinpath(".lmh.toml").write_text(
            'owner = "Native Smoke Test"\n'
            f"starting-year = {year}\n"
            'license = "Apache-2.0"\n'
            'languages = ["python"]\n'
            'paths = ["src"]\n'
            "ignore-files = []\n"
            "ignore-folders = []\n",
            encoding="utf-8",
        )
        source.write_text(
            f"# Copyright (C) {year}, Native Smoke Test.\n\n"
            "# This program is licensed under the Apache License 2.0.\n"
            "# See LICENSE or go to <https://www.apache.org/licenses/LICENSE-2.0> for full license details.\n\n"
            "value = 1\n",
            encoding="utf-8",
        )
        for name in COMMANDS:
            executable = shutil.which(name, path=str(destination))
            if executable is None:
                fail(f"Unable to find extracted command {name}")
            try:
                result = subprocess.run(  # ruff: ignore[subprocess-without-shell-equals-true] - fixed executable from the extracted archive
                    [executable, "--version"],
                    check=False,
                    capture_output=True,
                    text=True,
                    timeout=30,
                    env=environment,
                )
            except (OSError, subprocess.TimeoutExpired) as error:
                fail(f"Unable to run extracted command {name}: {error}")
            if result.returncode != 0 or result.stdout.strip() != f"lmh {version}":
                fail(f"Native command smoke failed for {name}: {result.stdout}{result.stderr}")
            try:
                check = subprocess.run(  # ruff: ignore[subprocess-without-shell-equals-true] - fixed executable from the extracted archive
                    [executable, "check", "--output-format", "json"],
                    check=False,
                    capture_output=True,
                    text=True,
                    timeout=30,
                    cwd=project,
                    env=environment,
                )
            except (OSError, subprocess.TimeoutExpired) as error:
                fail(f"Unable to run native check with {name}: {error}")
            if check.returncode != 0:
                fail(f"Native check failed for {name}: {check.stdout}{check.stderr}")
            payload = json.loads(check.stdout)
            if (payload["checked"], payload["diagnostics"], payload["error"]) != (1, [], None):
                fail(f"Native check returned an unexpected result with {name}: {payload}")


def _run_gh(arguments: list[str]) -> subprocess.CompletedProcess[str]:
    gh = shutil.which("gh")
    if gh is None:
        raise FileNotFoundError("gh is required to upload native release assets")
    return subprocess.run(  # ruff: ignore[subprocess-without-shell-equals-true] - fixed gh executable and argument list
        [gh, *arguments], check=True, capture_output=True, text=True
    )


def upload(directory: Path, tag: str, repository: str) -> None:
    if not tag.startswith("v"):
        fail(f"Release tag must start with v: {tag}")
    version = tag[1:]
    assets = release_assets(directory, version)
    listing = _run_gh(["release", "view", tag, "--repo", repository, "--json", "assets", "--jq", ".assets[].name"])
    existing = set(listing.stdout.splitlines())
    missing = []
    with TemporaryDirectory() as temporary:
        for asset in assets:
            if asset.name not in existing:
                missing.append(str(asset))
                continue
            _run_gh(["release", "download", tag, "--repo", repository, "--pattern", asset.name, "--dir", temporary])
            remote_asset = Path(temporary) / asset.name
            if not remote_asset.is_file() or remote_asset.read_bytes() != asset.read_bytes():
                fail(f"Existing GitHub release asset differs from prepared file: {asset.name}")
    if missing:
        _run_gh(["release", "upload", tag, *missing, "--repo", repository])


def main() -> None:
    parser = argparse.ArgumentParser(description="Prepare, smoke-test, and upload native release archives")
    commands = parser.add_subparsers(dest="command", required=True)
    pack_parser = commands.add_parser("pack")
    pack_parser.add_argument("--wheel", type=Path, required=True)
    pack_parser.add_argument("--target", required=True, choices=TARGETS)
    pack_parser.add_argument("--version", required=True)
    pack_parser.add_argument("--output", type=Path, required=True)
    smoke_parser = commands.add_parser("smoke")
    smoke_parser.add_argument("--archive", type=Path, required=True)
    smoke_parser.add_argument("--target", required=True, choices=TARGETS)
    smoke_parser.add_argument("--version", required=True)
    upload_parser = commands.add_parser("upload")
    upload_parser.add_argument("--artifacts", type=Path, required=True)
    upload_parser.add_argument("--tag", required=True)
    upload_parser.add_argument("--repository", required=True)
    args = parser.parse_args()
    try:
        if args.command == "pack":
            archive = pack(args.wheel, args.target, args.version, args.output)
            print(archive)
        elif args.command == "smoke":
            smoke(args.archive, args.target, args.version)
        else:
            upload(args.artifacts, args.tag, args.repository)
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        parser.exit(2, f"error: {error}\n")


if __name__ == "__main__":
    main()
