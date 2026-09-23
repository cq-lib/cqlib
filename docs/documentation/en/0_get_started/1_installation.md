# Cqlib Installation and Environment Setup

This guide covers the installation of Cqlib and the setup of a development environment.

Before installing Cqlib, ensure that the environment meets the following conditions:

- Python environment: Python 3.10 – 3.14 is supported (a 64-bit build is recommended);
- Operating system:
  - Linux
  - macOS
  - Windows

---

## Option A: quick installation with `pip` (recommended)

For most users, installing directly with `pip` is recommended.


> Applies to: algorithm development, prototype validation.
> Note: this option already includes prebuilt binaries; no Rust or C compiler is required.

### Step 1: create and activate a virtual environment

Working in an isolated environment is recommended to avoid dependency conflicts:

```bash
python -m venv cqlib-env

# activate the environment (Windows)
cqlib-env\Scripts\activate
# activate the environment (Linux/macOS)
source cqlib-env/bin/activate
```

### Step 2: one-step installation with `pip`

```bash
pip install cqlib
```

### Step 3: verify the installation

After installation completes, verify it as follows:

```bash
python -m pip show cqlib
```
If the version information is printed correctly, the installation succeeded.

---

## Option B: build from source (for developers)

To take part in the development of Cqlib, or to use the latest features that have not yet been released, build from source with the following method.

> Applies to: customized development, code contributions or the latest features.
> Note: this option requires a local compilation environment.

### Step 1: set up the compilation toolchain

Before building, install the following required components according to the operating system:

- Install the Rust toolchain (Stable 1.89+):

  - Windows: visit the official website: https://www.rust-lang.org/tools/install to download and run the installer.
  - Linux/macOS: run `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`

- Install a C compiler:
  - Windows: install the Visual Studio Build Tools and select "Desktop development with C++".
  - Linux: install `build-essential` (Ubuntu/Debian) or `base-devel` (Arch).
  - macOS: run `xcode-select --install` in the terminal.

After installation completes, verify with the following commands:

```bash
rustc --version
cargo --version
```
If the version numbers are printed correctly, Rust was installed successfully.

### Step 2: obtain the source code

Clone the repository from Gitee to the local machine:
```bash
git clone https://gitee.com/zdxlz/cqlib2.git
cd cqlib
```

### Step 3: build and install the Python bindings

Use the `maturin` cross-language tool to compile the Rust core into a Python module:

```bash
# 1. install the build tools
pip install -U maturin

# 2. compile and install into the current environment
# the --release flag ensures the best runtime performance
maturin develop --release -m crates/binding-python/Cargo.toml
```
This command installs the locally built version into the current virtual environment.

### Step 4: build the Rust core and the C interface (optional)

```bash
# build the core library
cargo build --release

# build the C interface ABI (optional)
cargo build -p binding-c --release
```

---

## Optional module: install the Tianyan quantum cloud platform client

To submit local circuits to the Tianyan quantum cloud platform for execution, install `cqlib-tianyan` as well. This module is independent of the Cqlib core package and is responsible for platform authentication, backend queries, task submission, task polling and result parsing.

```bash
pip install cqlib-tianyan
```

After installation, verify it as follows:

```python
from cqlib_tianyan import TianyanPlatform
print(TianyanPlatform)
```

`cqlib-tianyan` is usually used together with `cqlib`: build circuits with `cqlib.circuit`, export QCIS with `cqlib.ir.qcis`, and finally submit to a cloud backend through `cqlib_tianyan`.

---

## Next steps

After installation, the following content is recommended for further reading:

- [Quickstart](2_quickstart.md): the first quantum circuit, from "0" to "1"
- [Quantum Circuit](../1_cqlib/0_circuit/0_overview.md): learn about the basic module for describing quantum programs in Cqlib.
- [Quantum gates and instructions](../1_cqlib/0_circuit/1_gates.md): learn about built-in gates, custom gates, composite gates and non-unitary instructions.
- [Tianyan quantum cloud platform client](../1_cqlib/7_tianyan/0_overview.md): learn about cloud backend access, task submission and result retrieval.
