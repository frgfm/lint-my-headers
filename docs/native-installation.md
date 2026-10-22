---
description: Run the native LMH executables by extracting them from a matching PyPI wheel.
---

# Native installation

The PyPI wheels include the `lmh` and `lint-my-headers` native executables. You can extract and run them without Python, Rust, or uv. Download the wheel for your operating system and CPU from the [0.8.0 PyPI files](https://pypi.org/project/lint-my-headers/0.8.0/#files); PyPI lists each file's SHA-256 digest.

| Platform | Wheel target |
| --- | --- |
| Linux x86_64, glibc 2.28+ | `x86_64-unknown-linux-gnu` |
| Linux ARM64, glibc 2.28+ | `aarch64-unknown-linux-gnu` |
| Linux x86_64, musl 1.2+ | `x86_64-unknown-linux-musl` |
| Linux ARM64, musl 1.2+ | `aarch64-unknown-linux-musl` |
| macOS Intel | `x86_64-apple-darwin` |
| macOS Apple silicon | `aarch64-apple-darwin` |
| Windows x86_64 | `x86_64-pc-windows-msvc` |
| Windows ARM64 | `aarch64-pc-windows-msvc` |

## Linux and macOS

For example, extract the Linux x86_64 wheel into a local folder, then run the binary from the wheel's `scripts` directory:

```shell
wheel=lint_my_headers-0.8.0-py3-none-manylinux_2_28_x86_64.whl
unzip -q "$wheel" -d lmh-wheel
PATH="$PWD/lmh-wheel/lint_my_headers-0.8.0.data/scripts:$PATH" lmh --version
```

Use the matching wheel file and target folder for your platform. Keep the extracted wheel directory intact; it also contains the project license.

## Windows

Rename the downloaded `.whl` file to `.zip`, then extract it with Windows Explorer or PowerShell:

```powershell
Copy-Item .\lint_my_headers-0.8.0-py3-none-win_amd64.whl .\lmh.zip
Expand-Archive .\lmh.zip -DestinationPath .\lmh-wheel
& .\lmh-wheel\lint_my_headers-0.8.0.data\scripts\lmh.exe --version
```

Choose the `win_arm64` wheel for Windows ARM64. You can add the extracted `scripts` directory to `PATH` to run LMH from any directory.

The usual [uv or pip install](getting-started.md#install) remains available. Direct extraction is useful when you only need the native commands.
