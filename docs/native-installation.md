---
description: Install the upcoming standalone native archives for Linux, macOS, and Windows.
---

# Native installation

!!! warning "Unreleased"
    This page describes native GitHub Release archives planned for an upcoming
    release. Version **0.8.0 has no native archive assets**. Install 0.8.0 with
    [uv or pip](getting-started.md#install) until a later release lists these files.

Each archive contains the `lmh` and `lint-my-headers` commands plus the project
license. The commands run without Python, Rust, or uv. Linux and macOS archives
are `.tar.gz`; Windows archives are `.zip`. Each archive has a `.sha256` file.

| Platform | Target |
| --- | --- |
| Linux x86_64, glibc 2.28+ | `x86_64-unknown-linux-gnu` |
| Linux ARM64, glibc 2.28+ | `aarch64-unknown-linux-gnu` |
| Linux x86_64, musl 1.2+ | `x86_64-unknown-linux-musl` |
| Linux ARM64, musl 1.2+ | `aarch64-unknown-linux-musl` |
| macOS Intel | `x86_64-apple-darwin` |
| macOS Apple silicon | `aarch64-apple-darwin` |
| Windows x86_64 | `x86_64-pc-windows-msvc` |
| Windows ARM64 | `aarch64-pc-windows-msvc` |

Download the archive and its checksum from a future release page. Asset names
use `lint-my-headers-<version>-<target>`, followed by `.tar.gz` or `.zip`;
the checksum filename adds `.sha256`. In the commands below, set `VERSION`
without the leading `v` and `TARGET` to the exact target shown in the table.

On Linux, verify and install the archive into a directory on `PATH`:

```shell
VERSION="REPLACE_WITH_VERSION"
TARGET="x86_64-unknown-linux-gnu"
ARCHIVE="lint-my-headers-${VERSION}-${TARGET}.tar.gz"
sha256sum -c "$ARCHIVE.sha256"
mkdir -p "$HOME/.local/bin"
tar -xzf "$ARCHIVE" -C "$HOME/.local/bin" lmh lint-my-headers
export PATH="$HOME/.local/bin:$PATH"
lmh --version
```

On macOS, set `ARCHIVE` to the downloaded `.tar.gz` filename and run
`shasum -a 256 -c "$ARCHIVE.sha256"`, then extract `lmh` and
`lint-my-headers` into a directory on `PATH`:

```shell
shasum -a 256 -c "$ARCHIVE.sha256"
mkdir -p "$HOME/.local/bin"
tar -xzf "$ARCHIVE" -C "$HOME/.local/bin" lmh lint-my-headers
export PATH="$HOME/.local/bin:$PATH"
lmh --version
```

On Windows, set `$Archive` to the downloaded `.zip` file and verify it before
extracting. The archive contains `lmh.exe` and `lint-my-headers.exe`:

```powershell
$Archive = "path\to\downloaded.zip"
$Expected = ((Get-Content "${Archive}.sha256") -split '\s+')[0]
$Actual = (Get-FileHash $Archive -Algorithm SHA256).Hash.ToLowerInvariant()
if ($Expected -ne $Actual) { throw "SHA-256 mismatch" }
$Destination = "$env:USERPROFILE\bin\lmh"
Expand-Archive $Archive $Destination
$env:Path = "$Destination;$env:Path"
lmh --version
```

For the current 0.8.0 release, follow the [published installation steps](getting-started.md#install).
