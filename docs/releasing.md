# CI and Manual Releases

## File Organization

The structure follows Qiskit's separation of entry-point workflows, reusable
workflows, and tools scripts, without adopting its automated publishing steps
or modifying project source code, existing tests, or Cargo configuration.

| File | Responsibility |
| --- | --- |
| `.github/workflows/ci.yml` | PR entry point: calls lint and development tests on three operating systems |
| `.github/workflows/lint.yml` | Reusable Rust fmt / Clippy, Python Ruff, and C formatting checks |
| `.github/workflows/tests.yml` | Reusable Rust workspace, pytest, and C static/shared linking tests |
| `.github/workflows/wheels.yml` | Tag entry point: validates versions, runs five-platform tests, and calls the build workflow |
| `.github/workflows/wheels-build.yml` | Builds and stores wheels and C SDKs for five platforms, plus sdist and .crate packages |
| `tools/ci.py` | Reads versions, packages and tests C SDKs, tests installed wheels, and builds source packages |
| `tools/c-sdk/` | CMake test configuration and README distributed with each C SDK |

Routine commands live directly in workflows; cross-platform file handling lives
in the Python helper script. Rust 1.97.1 is pinned only in CI, without changing
developer toolchains or the project's declared MSRV. Ruff and clang-format
versions match the existing pre-commit configuration.

## Trigger Behavior

PRs run `cargo test --workspace --locked`, the full pytest suite against a
development installation, and C tests on Linux x86_64, Windows x86_64, and macOS
arm64. Formatting and Clippy checks run separately. `maturin develop` is used
only for a temporary development installation; no distribution packages are
built and no artifacts are uploaded. Cargo caching only accelerates tests.

Tags first undergo version checks and the same development tests on all five
platforms. Distribution packages are built only after these checks pass.
All workflows have only `contents: read` permissions. They require no publishing
credentials, create no Releases, and publish nothing to PyPI or crates.io.

## Tags and Versions

Tags use the Python version to identify a release batch:

- Stable releases: `vX.Y.Z`.
- Prereleases: `vX.Y.Z-alpha.N`, `vX.Y.Z-beta.N`, or `vX.Y.Z-rc.N`, with N starting at 1.
- Current tag: `v1.4.0-beta.1`.
- Current Python source version: `1.4.0-beta1`, normalized to `1.4.0b1` for PEP 440 and wheel filenames.
- Current Rust/C version: `0.1.0-beta.1`; both inherit the workspace version.

Python and Rust/C versions evolve independently and do not need matching
numbers. The Cargo.toml files at a given tag record the version mapping for that
release; no separate release.toml is maintained.
CI checks that the tag is equivalent to the Python version, core dependency
versions are consistent across bindings, the minimum Python version is 3.10,
and `abi3-py310` is enabled. Maintainers must update versions before tagging.

## Platforms and Artifacts

| Platform | Runner / Build Environment |
| --- | --- |
| Windows x86_64 | windows-2022 / MSVC |
| Linux glibc 2.28+ x86_64 | ubuntu-24.04 / manylinux_2_28_x86_64 |
| Linux glibc 2.28+ aarch64 | ubuntu-24.04-arm / manylinux_2_28_aarch64 |
| macOS 11+ x86_64 | macos-15-intel / deployment target 11.0 |
| macOS 11+ arm64 | macos-15 / deployment target 11.0 |

Linux wheels and C SDKs are built and tested inside architecture-native
manylinux containers. Ubuntu host binaries are not used as glibc 2.28 release
artifacts. Full workspace tests run on the host to support libpython linking
for the Python binding's Rust tests; core, C, and facade Rust tests also run
inside the container. Containers use a separate Cargo target directory.
Images use the latest tag, with the resolved digest recorded in logs.
On macOS, CI checks the shared library's minimum deployment version. Tests
actually run on macOS 15; this does not constitute execution testing on macOS 11.

Each platform produces one `cp310-abi3` wheel. That same wheel is installed into
separate CPython 3.10, 3.11, 3.12, 3.13, and 3.14 environments for import checks,
pip check, and the full pytest suite. Tests are copied alongside the installed
package to satisfy existing sibling-directory assertions. They do not import
cqlib from the checkout, and existing tests are not modified, skipped, or marked
xfail. Future Python versions require extending the validation matrix.

Each platform also produces one C SDK archive: ZIP on Windows and tar.gz
elsewhere. Each archive contains `cqlib_c.h`, static and shared libraries,
Windows import libraries where applicable, a README, LICENSE.txt, and runnable
C tests. Static and shared linking tests are compiled and run separately;
C assertions remain enabled in Release builds. Linux shared libraries must
not require GLIBC symbol versions newer than 2.28.

The source job generates one sdist and one `cqlib-core-<version>.crate`.
It runs `cargo package --locked -p cqlib-core`; Cargo's default verification
extracts and independently compiles the package. The command does not use
`--no-verify` and does not need to be executed twice.
The sdist is generated with maturin and checked for inclusion of the local core
dependency, without rewriting the checkout's Cargo.lock.

32-bit targets, musllinux/Alpine, other CPU architectures, universal2, PyPy,
and free-threaded Python are not currently supported.

## Downloading and Publishing

Wait for the entire tag workflow to succeed before publishing; an artifact
produced by a single successful job is not sufficient.
Download the five `release-<tag>-<platform>` artifacts and the single
`release-<tag>-sources` artifact. Together they contain 12 distribution files:
5 wheels, 1 sdist, 5 C SDK archives, and 1 .crate package.
GitHub artifacts are retained for 30 days, so release maintainers should
download and archive them promptly.

- PyPI: manually check and upload the 5 wheels and sdist using Twine.
- crates.io: this release publishes only `cqlib-core`, so there is no multi-crate
  publication order. `binding-python`, `binding-c`, and the `cqlib` facade are
  outside this release's scope. This is a release policy, not a change to their
  Cargo publish settings. Download and extract the .crate package, then run
  `cargo publish --locked --dry-run` in the directory containing its Cargo.toml.
  After verification, run `cargo publish --locked`. Cargo does not accept a
  .crate path directly as an argument to publish; it repackages the extracted
  sources. If the facade is published in the future, publish `cqlib-core` first,
  wait until it can be resolved through the registry index, then publish `cqlib`.
- GitHub/Gitee Releases: maintainers manually create a Release at the same tag
  and attach the C SDK archives and any other desired files.
