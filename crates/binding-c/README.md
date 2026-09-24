# C Binding for Cqlib

This crate exposes a C ABI for `cqlib-core`. The surface now covers circuit
construction, symbolic parameters, IR conversion (QCIS / QASM2 / QASM3),
device / topology / layout / noise models, quantum simulators (statevector,
density matrix, stabilizer), compilation, visualization, and error mitigation
(ZNE, virtual distillation) — roughly 190 free functions.

## Build

```bash
cargo build -p binding-c --release
```

The C header is generated at `crates/binding-c/include/cqlib_c.h`.

### Linking with MinGW gcc

Build the gnu variant first, then compile. The source file must come
**before** the `-l` libraries — GNU ld resolves symbols left to right:

```bash
cargo build -p binding-c --release --target x86_64-pc-windows-gnu

gcc -I crates/binding-c/include \
    crates/binding-c/examples/main.c \
    -L target/x86_64-pc-windows-gnu/release \
    -lbinding_c -lntdll -lws2_32 -lbcrypt -luserenv -ladvapi32 \
    -o demo
```

At runtime the executable must find `binding_c.dll`: either add
`target/x86_64-pc-windows-gnu/release` (as an **absolute** path) to `PATH`
or copy the DLL next to the executable. Relative `PATH` entries are
resolved against the process working directory and can silently fail.

### Linking on macOS / Linux

No toolchain-matching is needed — the platform has a single ABI. Just
build with the default host target and link directly against the cdylib:

```bash
cargo build -p binding-c --release

gcc -I crates/binding-c/include \
    crates/binding-c/examples/main.c \
    -L target/release -lbinding_c \
    -o demo
```

At runtime the dynamic linker must find `libbinding_c.dylib` (macOS) or
`libbinding_c.so` (Linux). Either set `DYLD_LIBRARY_PATH` / `LD_LIBRARY_PATH`
to `target/release`, or copy the shared library next to the executable.

## Usage

```c
#include <cqlib_c.h>
#include <stdio.h>

int main(void) {
    /* Bell state circuit */
    CCircuit *qc = circuit_new(2);
    circuit_h(qc, 0);
    circuit_cx(qc, 0, 1);

    /* Exact simulation */
    CStatevector *sv = statevector_from_circuit(qc);
    double probs[4];
    statevector_probabilities(sv, probs, 4);   /* 0.5, 0, 0, 0.5 */

    statevector_free(sv);
    circuit_free(qc);
    return 0;
}
```

## Documentation

The full API manual lives in [`docs/api/en/c/`](../../docs/api/en/c/0_overview.md)
and mirrors the Python manual's structure: per-function signatures,
parameters, return values, error codes, and examples for every module
(circuit, IR, device, QIS, compile, visualization, error mitigation).

## Return Values

Integer-returning functions use `0` for success and negative error codes on
failure:

| Code | Meaning |
| --- | --- |
| `0` | Success |
| `-1` | Null pointer or invalid C string |
| `-2` | Qubit index out of bounds |
| `-3` | Circuit / structural error |
| `-4` | Parse error (unknown gate name, malformed IR, ...) |
| `-5` | IO error |
| `-6` | Compiler error |
| `-7` | Simulation error (non-Clifford gate, not run, ...) |
| `-8` | Invalid parameter (bad probability, buffer too small, ...) |

Pointer-returning functions use `NULL` for errors. `param_evaluate` returns
`NaN` when the bindings string is invalid or a symbol is unbound.

## Tests

```bash
cargo test -p binding-c
```

Rust FFI tests live in `crates/binding-c/tests/*.rs`. C-side smoke tests live
in `crates/binding-c/tests/test_*.c`, one per module (circuit, ir, device,
qis, compile, visualization, error_mitigation). Compile and run each
individually (build the gnu variant first, see Toolchain matching above):

```bash
gcc -I crates/binding-c/include \
    crates/binding-c/tests/test_circuit.c \
    -L target/x86_64-pc-windows-gnu/release \
    -lbinding_c -lntdll -lws2_32 -lbcrypt -luserenv -ladvapi32 -lm \
    -o test_circuit

./test_circuit        # prints "binding-c circuit tests passed"
```