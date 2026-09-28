# CQLib C SDK

This archive contains the CQLib C ABI header, static and shared libraries,
Apache-2.0 license, and executable consumer tests. Its version follows the Rust
workspace, independently of the Python package version.

## Supported platforms

- Windows x86_64, MSVC ABI. Use the Visual Studio 2022 C/C++ toolchain and the
  Microsoft Visual C++ runtime. `binding_c.lib` is the static library;
  `binding_c.dll.lib` is the import library for `bin/binding_c.dll`.
- Linux x86_64 or aarch64, glibc 2.28 or newer. Libraries are built and tested in
  the corresponding manylinux_2_28 image.
- macOS x86_64 or arm64, deployment target 11.0. These are separate archives,
  not universal binaries. CI tests on current hosted macOS runners; it does not
  execute on macOS 11 itself.

32-bit targets, musl/Alpine, other CPU architectures and the Windows MinGW ABI
are outside this release's support matrix.

## Verify the SDK

Install CMake 3.20+ and a native C compiler. From the unpacked SDK directory:

```sh
cmake -S tests -B build -DCQLIB_SDK_ROOT="$PWD" -DCMAKE_BUILD_TYPE=Release
cmake --build build --config Release
ctest --test-dir build -C Release --output-on-failure
```

On Windows, run from a Developer PowerShell and replace `"$PWD"` with
`"$($PWD.Path)"`. The tests explicitly link each library variant and retain
assertions in Release mode. Their CMake file is also a consumer-linking example.

## Integrate

Include `include/cqlib_c.h`. Never free CQLib-owned objects with the system
`free`: use their matching API functions such as `circuit_free` and `param_free`.

For static linking, pass the full static-library path and these platform system
libraries, as demonstrated in `tests/CMakeLists.txt`:

| Platform | Additional libraries |
| --- | --- |
| Linux | `pthread`, `dl`, `m` |
| macOS | `c++`, `CoreFoundation` framework |
| Windows | `ntdll`, `ws2_32`, `bcrypt`, `userenv`, `advapi32` |

For shared linking, make the library discoverable at runtime. On Windows place
`binding_c.dll` alongside the executable (and link `binding_c.dll.lib`). On Linux
configure an application-relative RPATH such as `$ORIGIN/../lib`, or set
`LD_LIBRARY_PATH` for development. On macOS configure an application-relative
RPATH such as `@executable_path/../lib`; the dylib uses
`@rpath/libbinding_c.dylib` as its install name. No checkout or Cargo build
directory is needed by SDK consumers.

The supported circuit construction and parameter APIs are documented in the
generated header and illustrated in `tests/test_circuit.c`.
