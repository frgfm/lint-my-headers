# Copyright (C) 2024-2026, François-Guillaume Fernandez.

# This program is licensed under the Apache License 2.0.
# See LICENSE or go to <https://www.apache.org/licenses/LICENSE-2.0> for full license details.

# /// script
# requires-python = ">=3.11"
# dependencies = [
#     "pyyaml>=6.0",
# ]
# ///

import tomllib
from pathlib import Path

import yaml

EXPECTED_UV_VERSION = "0.13.4"
EXPECTED_UV_REQUIREMENT = f"uv>={EXPECTED_UV_VERSION},<1.0.0"


def main():
    # Retrieve & parse all deps files
    deps_dict = {"uv": []}
    action = yaml.safe_load(Path("action.yml").read_text())
    for step in action["runs"]["steps"]:
        if step.get("uses", "").startswith("astral-sh/setup-uv@"):
            deps_dict["uv"].append({"file": "action.yml", "version": step["with"]["version"]})
    # Parse github/workflows/...
    for workflow_file in Path(".github/workflows").glob("*.yml"):
        with workflow_file.open("r") as f:
            workflow = yaml.safe_load(f)
            for name, version in workflow.get("env", {}).items():
                if name in {"UV_VERSION", "RELEASE_UV_VERSION"}:
                    deps_dict["uv"].append({"file": str(workflow_file), "version": version.lstrip("v")})

    troubles = []
    pyproject = tomllib.loads(Path("pyproject.toml").read_text())
    build_requirements = pyproject.get("dependency-groups", {}).get("build", [])
    uv_requirements = [requirement for requirement in build_requirements if requirement.startswith("uv")]
    if uv_requirements != [EXPECTED_UV_REQUIREMENT]:
        troubles.append(
            f"pyproject.toml [dependency-groups].build must contain {EXPECTED_UV_REQUIREMENT!r}, "
            f"found {uv_requirements!r}"
        )

    # Assert all CI pins are in sync.
    for dep, versions in deps_dict.items():
        versions_ = {v["version"] for v in versions}
        if len(versions_) != 1:
            inv_dict = {v: set() for v in versions_}
            for version in versions:
                inv_dict[version["version"]].add(version["file"])
            troubles.extend([
                f"{dep}:",
                "\n".join(f"- '{v}': {', '.join(files)}" for v, files in inv_dict.items()),
            ])

    uv_versions = {version["version"] for version in deps_dict["uv"]}
    if uv_versions != {EXPECTED_UV_VERSION}:
        troubles.append(
            f"uv CI pins must be {EXPECTED_UV_VERSION} to match {EXPECTED_UV_REQUIREMENT!r}; "
            f"found {sorted(uv_versions)}"
        )

    if len(troubles) > 0:
        raise ValueError("Some dependencies are out of sync:\n\n" + "\n".join(troubles))


if __name__ == "__main__":
    main()
