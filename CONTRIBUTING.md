# Cqlib Contributing Guide

[简体中文](CONTRIBUTING.CN.md)

Thank you for contributing to Cqlib. Cqlib is a high-performance quantum computing SDK built in Rust, with a Rust core library, Python bindings, and C bindings. It covers quantum circuit construction, IR, compiler optimizations, device models, noise models, quantum state simulation, and error mitigation.

- Website: <https://qc.zdxlz.com/>
- English overview: [README.md](README.md)
- Chinese overview: [README.CN.md](README.CN.md)
- License: [Apache License 2.0](LICENSE.txt)
- Security issues: [SECURITY.md](SECURITY.md)

This guide explains how to report issues, set up a development environment, submit changes, run tests, and participate in code review.

## Discuss Before Contributing

If you plan to fix a small bug, add tests, improve documentation, or improve error messages, you can open a pull request directly.

Please open an issue for design discussion before working on changes that:

- Add or modify public APIs
- Change behavior in the Rust, Python, or C bindings
- Add new dependencies
- Modify build, release, or CI workflows
- Perform large-scale refactoring
- Change core semantics for circuits, IR, compiler optimizations, or simulators

Discussing these changes upfront helps maintainers confirm direction and avoids rework caused by API design, compatibility, or project-scope concerns.

## Reporting Issues

Before opening an issue, please search for existing related issues. When reporting a bug, include as much of the following information as possible:

- Cqlib version, commit, or installation source
- Operating system, CPU architecture, Rust version, and Python version
- Installation method, such as `pip install cqlib`, `maturin develop`, or source build
- Minimal reproduction code
- Expected result and actual result
- Full error message, logs, or stack trace

For numerical issues, also include the input circuit, parameters, random seed, simulator or backend configuration, and the tolerance you expect to be reasonable.

Do not report security vulnerabilities in public issues. Email <tianyan@chinatelecom.cn> and follow [SECURITY.md](SECURITY.md).

## Project Layout

The main repository layout is:

```text
crates/cqlib-core/        Rust core library
crates/cqlib/             Public Rust crate
crates/binding-python/    Python bindings and tests
crates/binding-c/         C bindings
docs/                     Documentation files
```

Keep common changes scoped to the relevant module when possible, and avoid combining unrelated changes in one PR.

## Development Environment

### Prerequisites

The project currently requires:

- Rust 1.89+
- Python 3.10+
- A C 11+ toolchain for C binding development

We recommend using `rustup` for Rust toolchains and a Python virtual environment for Python dependencies.

### Clone The Repository

```bash
git clone https://gitee.com/cq-lib/cqlib.git
cd cqlib
```

If you contribute through a fork, create your branch from your fork and keep it based on the latest `main`.

### Python Environment

```bash
python3 -m venv .venv
source .venv/bin/activate
python -m pip install -U pip
python -m pip install maturin pytest pre-commit
```

Install the local extension into the current Python environment:

```bash
maturin develop -m crates/binding-python/Cargo.toml
```

For performance closer to release builds, use:

```bash
maturin develop --release -m crates/binding-python/Cargo.toml
```

After modifying Rust code, rerun `maturin develop` so Python loads the updated native extension.

## Build

Build the full Rust workspace:

```bash
cargo build --all
```

Build the core library:

```bash
cargo build -p cqlib-core
```

Build the Python bindings:

```bash
maturin build --release -m crates/binding-python/Cargo.toml
```

Build the C bindings:

```bash
cargo build -p binding-c
```

## Testing

Before submitting a PR, run the tests relevant to your change.

Run all Rust tests:

```bash
cargo test --all
```

Run tests for specific Rust crates:

```bash
cargo test -p cqlib-core
cargo test -p binding-c
```

Run Python tests:

```bash
maturin develop -m crates/binding-python/Cargo.toml
pytest crates/binding-python/tests/
```

Changes involving numerical computation, quantum state simulation, noise models, compiler optimizations, parameterized circuits, or FFI boundaries should include normal-path, error-path, and edge-case tests. Common edge cases include empty circuits, invalid qubit indexes, duplicate qubits, non-finite parameters, dimension mismatches, and numerical tolerances.

## Code Style

This project uses `pre-commit` to run basic checks, Rust formatting and linting, Ruff, clang-format, and spell checks.

Enable hooks for the first time:

```bash
pre-commit install
```

Run all checks before submitting:

```bash
pre-commit run --all-files
```

You can also run checks by language:

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
ruff check --fix .
ruff format .
```

C and C header changes should follow the `.clang-format` configuration in the repository root.

## Documentation

If your change affects user-visible behavior, update the relevant documentation, examples, or API comments, including when applicable:

- Installation, build, or example code in README files
- Rust crate docs and public API comments
- Python public API descriptions and test examples
- C binding README, header comments, or examples

Documentation examples should be runnable when possible. For quantum circuits or numerical results, state the input, output, and expected tolerance clearly.

## Branches And Commits

Create feature branches from `main`:

```bash
git checkout main
git pull
git checkout -b fix/short-description
```

Recommended branch prefixes:

- `fix/`: bug fixes
- `feat/`: new features
- `docs/`: documentation changes
- `test/`: test additions or updates
- `refactor/`: behavior-preserving refactoring
- `chore/`: build, dependency, or maintenance work

Commit messages should describe the type and scope of the change:

```text
fix(circuit): reject duplicate qubits in controlled gates
feat(qis): add density matrix fidelity helper
docs: update Python binding build instructions
```

Avoid mixing unrelated formatting, temporary debugging code, large files, or IDE configuration in commits.

## Pull Requests

Before submitting a PR, confirm that:

- The PR is based on the latest `main`
- The PR title clearly describes the change
- Related issues are linked in the PR description
- Relevant tests and formatting checks have been run
- New features or bug fixes include tests
- User-visible behavior changes include documentation updates
- No unrelated files, debugging code, or temporary output are included

Recommended PR description:

```md
## What Changed

## Why

## How Tested

## Related Issues
```

If the PR is not ready, mark it as Draft and describe the remaining work. Maintainers may ask you to split large PRs so each PR remains clear, reviewable, and reversible.

## Code Review

All code changes require review. Reviews focus on:

- Whether API design is clear, stable, and consistent with the project style
- Whether Rust core implementation is safe, maintainable, and reasonably performant
- Whether Python and C bindings match Rust behavior
- Whether error handling is explicit and uses appropriate exceptions or error types
- Whether tests cover normal paths, error paths, and edge cases
- Whether documentation matches actual behavior

Treat review comments as discussion about code quality and project consistency. Maintainers may request changes, additional tests, PR splitting, or close changes that do not fit the current project direction.

## Dependencies And Compatibility

Explain the reason before adding a dependency, especially if it affects compile time, package size, platform compatibility, or security maintenance cost. New dependencies should:

- Have a license compatible with Apache License 2.0
- Serve a clear purpose that cannot reasonably be handled by the standard library or existing dependencies
- Avoid unnecessary platform limitations in the Rust, Python, or C release paths

Do not raise the minimum Rust or Python version casually. If a version bump is necessary, explain the reason and impact in the issue or PR.

## Contribution License

Cqlib is released under [Apache License 2.0](LICENSE.txt). By submitting a contribution, you represent that you have the right to submit it and agree to license it to the project and recipients under Apache License 2.0, unless you explicitly state a different arrangement and maintainers accept it.

Do not submit code, documentation, data, or third-party material that you do not have the right to license. Content assisted by AI tools must still be reviewed, tested, and checked by the contributor for copyright, license, privacy, and security issues.

## Code Of Conduct

Keep discussions professional, respectful, and focused on the issue. Disagreements should be based on technical facts, project goals, and user impact. Maintainers may remove inappropriate content, restrict participation, or close off-topic discussions.
