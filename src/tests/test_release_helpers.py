# Copyright (C) 2026, François-Guillaume Fernandez.

# This program is licensed under the Apache License 2.0.
# See LICENSE or go to <https://www.apache.org/licenses/LICENSE-2.0> for full license details.

import importlib.util
import os
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest import mock


def load_helper(name):
    path = Path(__file__).parents[2] / f".github/{name}.py"
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise ImportError(f"Unable to load {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


pypi = load_helper("pypi_release")


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


if __name__ == "__main__":
    unittest.main()
