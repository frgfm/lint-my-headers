# Copyright (C) 2026, François-Guillaume Fernandez.

# This program is licensed under the Apache License 2.0.
# See LICENSE or go to <https://www.apache.org/licenses/LICENSE-2.0> for full license details.

import hashlib
import importlib.util
import os
import tarfile
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest import mock
from zipfile import ZIP_DEFLATED, ZipFile, ZipInfo


def load_helper(name):
    path = Path(__file__).parents[2] / f".github/{name}.py"
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise ImportError(f"Unable to load {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


pypi = load_helper("pypi_release")
native = load_helper("native_release")


class ReleaseHelperTestCase(unittest.TestCase):
    def setUp(self):
        self.original_directory = Path.cwd()
        temporary = TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.addCleanup(os.chdir, self.original_directory)
        os.chdir(temporary.name)

    def artifacts(self):
        directory = Path("dist")
        directory.mkdir()
        wheel = directory / "package.whl"
        source = directory / "package.tar.gz"
        wheel.write_bytes(b"wheel")
        source.write_bytes(b"source")
        return directory, wheel, source

    def native_release_artifacts(self):
        directory = Path("native-release")
        directory.mkdir()
        for target in native.TARGETS:
            archive = directory / native.archive_name("0.8.0", target)
            archive.write_bytes(target.encode())
            checksum = archive.with_name(f"{archive.name}.sha256")
            checksum.write_text(
                f"{hashlib.sha256(archive.read_bytes()).hexdigest()}  {archive.name}\n", encoding="ascii"
            )
        return directory

    def test_platform_wheels_and_single_sdist(self):
        directory, wheel, source = self.artifacts()
        second_wheel = directory / "package_macos.whl"
        second_wheel.write_bytes(b"macos wheel")
        self.assertEqual(set(pypi.artifact_hashes(directory)), {wheel.name, second_wheel.name, source.name})
        source.unlink()
        with self.assertRaisesRegex(ValueError, "one sdist"):
            pypi.artifact_hashes(directory)

    def test_python_and_cargo_versions_match(self):
        Path("pyproject.toml").write_text('[project]\nversion = "0.6.0rc1"\n', encoding="utf-8")
        Path("Cargo.toml").write_text('[package]\nversion = "0.6.0-rc.1"\n', encoding="utf-8")
        pypi.verify_versions(Path.cwd())
        Path("Cargo.toml").write_text('[package]\nversion = "0.6.0"\n', encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "does not match"):
            pypi.verify_versions(Path.cwd())

    def test_publish_uploads_only_missing_artifact(self):
        directory, wheel, source = self.artifacts()
        existing = {wheel.name: pypi.artifact_hashes(directory)[wheel.name]}

        with (
            mock.patch.object(pypi, "published_hashes", return_value=existing),
            mock.patch.object(pypi.shutil, "which", return_value="/usr/bin/uv"),
            mock.patch.object(pypi.subprocess, "run") as run,
        ):
            pypi.publish(directory, "example", "1.0")

        command = run.call_args.args[0]
        self.assertEqual(
            command,
            ["/usr/bin/uv", "publish", "--trusted-publishing", "always", str(source)],
        )

    def test_publish_aborts_on_existing_hash_mismatch(self):
        directory, wheel, _ = self.artifacts()
        with (
            mock.patch.object(pypi, "published_hashes", return_value={wheel.name: "wrong"}),
            self.assertRaisesRegex(ValueError, "Published hash mismatch"),
        ):
            pypi.publish(directory, "example", "1.0")

    def test_publish_and_verify_abort_on_unexpected_remote_artifact(self):
        directory, _, _ = self.artifacts()
        remote = {**pypi.artifact_hashes(directory), "unreviewed.whl": "digest"}

        with mock.patch.object(pypi, "published_hashes", return_value=remote):
            for operation in (
                lambda: pypi.publish(directory, "example", "1.0"),
                lambda: pypi.verify(directory, "example", "1.0", 1, 0),
            ):
                with self.subTest(operation=operation), self.assertRaisesRegex(ValueError, "unexpected"):
                    operation()

    def test_verify_retries_until_all_hashes_match(self):
        directory, _, _ = self.artifacts()
        expected = pypi.artifact_hashes(directory)

        with (
            mock.patch.object(pypi, "published_hashes", side_effect=[{}, expected]),
            mock.patch.object(pypi.time, "sleep") as sleep,
        ):
            pypi.verify(directory, "example", "1.0", 2, 0)

        sleep.assert_called_once_with(0)

    def test_native_archive_contains_both_commands_and_a_matching_checksum(self):
        wheel = Path("lint_my_headers-0.8.0-py3-none-macosx_11_0_arm64.whl")
        with ZipFile(wheel, "w") as archive:
            archive.writestr(
                "lint_my_headers-0.8.0.dist-info/METADATA",
                "Metadata-Version: 2.1\nName: lint-my-headers\nVersion: 0.8.0\n\n",
            )
            archive.writestr("lint_my_headers-0.8.0.dist-info/licenses/LICENSE", b"license")
            for name in ("lmh", "lint-my-headers"):
                info = ZipInfo(f"lint_my_headers-0.8.0.data/scripts/{name}")
                info.create_system = 3
                info.external_attr = 0o100755 << 16
                archive.writestr(info, b"\xcf\xfa\xed\xfe executable")

        result = native.pack(wheel, "aarch64-apple-darwin", "0.8.0", Path("native"))
        self.assertEqual(result.name, "lint-my-headers-0.8.0-aarch64-apple-darwin.tar.gz")
        checksum = result.with_name(f"{result.name}.sha256")
        digest = hashlib.sha256(result.read_bytes()).hexdigest()
        self.assertEqual(checksum.read_text(encoding="ascii"), f"{digest}  {result.name}\n")
        with tarfile.open(result, "r:gz") as archive:
            self.assertEqual(set(archive.getnames()), {"LICENSE", "lmh", "lint-my-headers"})
            self.assertEqual(archive.getmember("lmh").mode, 0o755)

    def test_native_archive_rejects_a_wheel_for_another_target(self):
        wheel = Path("lint_my_headers-0.8.0-py3-none-macosx_11_0_arm64.whl")
        with ZipFile(wheel, "w") as archive:
            archive.writestr(
                "lint_my_headers-0.8.0.dist-info/METADATA",
                "Metadata-Version: 2.1\nName: lint-my-headers\nVersion: 0.8.0\n\n",
            )
        with self.assertRaisesRegex(ValueError, "does not match target"):
            native.pack(wheel, "x86_64-unknown-linux-gnu", "0.8.0", Path("native"))

    def test_native_windows_archive_is_a_compressed_zip(self):
        wheel = Path("lint_my_headers-0.8.0-py3-none-win_amd64.whl")
        with ZipFile(wheel, "w") as archive:
            archive.writestr(
                "lint_my_headers-0.8.0.dist-info/METADATA",
                "Metadata-Version: 2.1\nName: lint-my-headers\nVersion: 0.8.0\n\n",
            )
            archive.writestr("lint_my_headers-0.8.0.dist-info/licenses/LICENSE", b"license")
            archive.writestr("lint_my_headers-0.8.0.data/scripts/lmh.exe", b"MZ executable")
            archive.writestr("lint_my_headers-0.8.0.data/scripts/lint-my-headers.exe", b"MZ executable")

        result = native.pack(wheel, "x86_64-pc-windows-msvc", "0.8.0", Path("native"))
        self.assertEqual(result.suffix, ".zip")
        with ZipFile(result) as archive:
            self.assertEqual(set(archive.namelist()), {"LICENSE", "lmh.exe", "lint-my-headers.exe"})
            self.assertTrue(all(item.compress_type == ZIP_DEFLATED for item in archive.infolist()))

    def test_native_archive_rejects_a_unsafe_release_version(self):
        with self.assertRaisesRegex(ValueError, "Unsupported release version"):
            native.archive_name("../0.8.0", "aarch64-apple-darwin")

    def test_native_release_upload_rejects_a_different_existing_asset(self):
        directory = self.native_release_artifacts()
        conflicting = next(directory.glob("*.tar.gz"))
        calls = []

        def run_gh(arguments):
            calls.append(arguments)
            if arguments[1] == "view":
                return mock.Mock(stdout=f"{conflicting.name}\n")
            Path(arguments[arguments.index("--dir") + 1], conflicting.name).write_bytes(b"different")
            return mock.Mock(stdout="")

        with (
            mock.patch.object(native, "_run_gh", side_effect=run_gh),
            self.assertRaisesRegex(ValueError, "differs from prepared file"),
        ):
            native.upload(directory, "v0.8.0", "frgfm/lint-my-headers")
        self.assertEqual(len(calls), 2)
        self.assertTrue(all(arguments[1] != "upload" for arguments in calls))

    def test_native_release_upload_skips_an_identical_existing_asset(self):
        directory = self.native_release_artifacts()
        existing = next(directory.glob("*.tar.gz"))
        calls = []

        def run_gh(arguments):
            calls.append(arguments)
            if arguments[1] == "view":
                return mock.Mock(stdout=f"{existing.name}\n")
            if arguments[1] == "download":
                remote = Path(arguments[arguments.index("--dir") + 1], existing.name)
                remote.write_bytes(existing.read_bytes())
            return mock.Mock(stdout="")

        with mock.patch.object(native, "_run_gh", side_effect=run_gh):
            native.upload(directory, "v0.8.0", "frgfm/lint-my-headers")
        self.assertEqual([arguments[1] for arguments in calls], ["view", "download", "upload"])
        self.assertNotIn(str(existing), calls[-1])
        self.assertNotIn("--clobber", calls[-1])


if __name__ == "__main__":
    unittest.main()
