"""Small cross-platform helpers for CI; orchestration lives in workflows."""

import argparse
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile

try:
    import tomllib
except ModuleNotFoundError:
    import tomli as tomllib

from packaging.version import Version
from packaging.utils import parse_wheel_filename

ROOT = Path(__file__).resolve().parents[1]
BINDING = ROOT / "crates/binding-python"
DIST = ROOT / "target/ci-dist"
PLATFORMS = {
    "windows-x86_64-msvc": "win_amd64",
    "linux-glibc2.28-x86_64": "manylinux_2_28_x86_64",
    "linux-glibc2.28-aarch64": "manylinux_2_28_aarch64",
    "macos11-x86_64": "macosx_11_0_x86_64",
    "macos11-arm64": "macosx_11_0_arm64",
}


def read_toml(path):
    with Path(path).open("rb") as source:
        return tomllib.load(source)


def run(*args, cwd=ROOT):
    print("+", " ".join(map(str, args)), flush=True)
    subprocess.run(list(map(str, args)), cwd=cwd, check=True)


def require(condition, message):
    if not condition:
        raise ValueError(message)


def versions(tag=None):
    rust = read_toml(ROOT / "Cargo.toml")["workspace"]["package"]["version"]
    binding = read_toml(BINDING / "Cargo.toml")
    python = str(Version(binding["package"]["version"]))
    require(
        "abi3-py310" in binding["dependencies"]["pyo3"]["features"],
        "Require abi3-py310",
    )
    require(
        read_toml(BINDING / "pyproject.toml")["project"]["requires-python"] == ">=3.10",
        "Require Python >=3.10",
    )
    for member in ("binding-python", "binding-c", "cqlib"):
        manifest = read_toml(ROOT / "crates" / member / "Cargo.toml")
        require(
            manifest["dependencies"]["cqlib-core"]["version"] == rust,
            f"{member}: cqlib-core dependency version mismatch",
        )
    if tag is not None:
        require(
            re.fullmatch(r"v\d+\.\d+\.\d+(?:-(?:alpha|beta|rc)\.[1-9]\d*)?", tag),
            "Tag must be vX.Y.Z or vX.Y.Z-{alpha,beta,rc}.N",
        )
        require(Version(tag[1:]) == Version(python), "Tag must match Python version")
    print(f"Python: {python}; Rust/C: {rust}", flush=True)
    return python, rust


def library_files():
    if sys.platform == "win32":
        return {"lib": ["binding_c.lib", "binding_c.dll.lib"], "bin": ["binding_c.dll"]}
    suffix = "dylib" if sys.platform == "darwin" else "so"
    return {"lib": ["libbinding_c.a", f"libbinding_c.{suffix}"]}


def build_and_test(platform=None):
    release = platform is not None
    run(
        "cargo",
        "build",
        "--locked",
        "-p",
        "binding-c",
        *(["--release"] if release else []),
    )
    target = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target")).resolve()
    libraries = target / ("release" if release else "debug")
    version = versions()[1]
    name = f"cqlib-c-{version}-{platform or 'development'}"
    with tempfile.TemporaryDirectory(prefix="cqlib-c-") as temporary:
        sdk = Path(temporary) / name
        (sdk / "include").mkdir(parents=True)
        shutil.copy2(ROOT / "crates/binding-c/include/cqlib_c.h", sdk / "include")
        for directory, filenames in library_files().items():
            (sdk / directory).mkdir()
            for filename in filenames:
                shutil.copy2(libraries / filename, sdk / directory)
        shutil.copy2(ROOT / "LICENSE.txt", sdk)
        shutil.copy2(ROOT / "tools/c-sdk/README.md", sdk / "README.md")
        (sdk / "tests").mkdir()
        shutil.copy2(ROOT / "crates/binding-c/tests/test_circuit.c", sdk / "tests")
        shutil.copy2(ROOT / "tools/c-sdk/CMakeLists.txt", sdk / "tests")
        if sys.platform == "darwin":
            dylib = sdk / "lib/libbinding_c.dylib"
            run("install_name_tool", "-id", "@rpath/libbinding_c.dylib", dylib)
            if release:
                output = subprocess.check_output(["otool", "-l", dylib], text=True)
                # Inspect LC_BUILD_VERSION/LC_VERSION_MIN_MACOSX, not dylib versions.
                blocks = re.findall(
                    r"cmd LC_(?:BUILD_VERSION|VERSION_MIN_MACOSX)\b(.*?)(?=Load command|$)",
                    output,
                    re.S,
                )
                require(blocks, "Missing macOS deployment target")
                for block in blocks:
                    minimum = re.search(r"(?:minos|version)\s+(\d+\.\d+)", block)
                    require(
                        minimum and tuple(map(int, minimum[1].split("."))) <= (11, 0),
                        "C SDK requires macOS newer than 11",
                    )
        if release and sys.platform == "linux":
            output = subprocess.check_output(
                ["readelf", "--version-info", sdk / "lib/libbinding_c.so"], text=True
            )
            glibc_versions = [
                tuple(map(int, value.split(".")))
                for value in re.findall(r"GLIBC_(\d+\.\d+)", output)
            ]
            require(
                glibc_versions and max(glibc_versions) <= (2, 28),
                "C SDK requires glibc newer than 2.28",
            )
            run("ldd", sdk / "lib/libbinding_c.so")
        build = Path(temporary) / "build"
        run(
            "cmake",
            "-S",
            sdk / "tests",
            "-B",
            build,
            f"-DCQLIB_SDK_ROOT={sdk}",
            "-DCMAKE_BUILD_TYPE=Release",
        )
        run("cmake", "--build", build, "--config", "Release", "--parallel", "2")
        run("ctest", "--test-dir", build, "-C", "Release", "--output-on-failure")
        if release:
            DIST.mkdir(parents=True, exist_ok=True)
            shutil.make_archive(
                str(DIST / name),
                "zip" if sys.platform == "win32" else "gztar",
                temporary,
                name,
            )


def verify_wheel(directory=DIST):
    wheels = list(Path(directory).glob("*.whl"))
    require(len(wheels) == 1, "Expected exactly one wheel per platform")
    wheel = wheels[0].resolve()
    name, version, _, tags = parse_wheel_filename(wheel.name)
    require(
        name == "cqlib" and version == Version(versions()[0]), "Wrong wheel version"
    )
    require(
        all(t.interpreter == "cp310" and t.abi == "abi3" for t in tags),
        "Wheel must use cp310-abi3",
    )
    with tempfile.TemporaryDirectory(prefix="cqlib-wheel-") as temporary:
        directory = Path(temporary)
        run(sys.executable, "-m", "venv", directory / "venv")
        python = (
            directory
            / "venv"
            / ("Scripts/python.exe" if os.name == "nt" else "bin/python")
        )
        run(
            python,
            "-m",
            "pip",
            "install",
            "--only-binary=:all:",
            wheel,
            "pytest==8.4.2",
            "resvg-py==0.5.0",
        )
        run(python, "-m", "pip", "check")
        run(
            python,
            "-I",
            "-c",
            "import cqlib, cqlib._native, pathlib, sys; "
            "root = pathlib.Path(sys.prefix).resolve(); "
            "assert pathlib.Path(cqlib.__file__).resolve().is_relative_to(root); "
            "assert pathlib.Path(cqlib._native.__file__).resolve().is_relative_to(root); "
            "print(cqlib.__file__, cqlib._native.__file__)",
            cwd=directory,
        )
        site = Path(
            subprocess.check_output(
                [
                    str(python),
                    "-I",
                    "-c",
                    "import sysconfig; print(sysconfig.get_path('purelib'))",
                ],
                text=True,
            ).strip()
        )
        # Existing location tests expect tests/ and cqlib/ to be siblings.
        # Copy tests beside the installed wheel, never replace it with checkout sources.
        tests = site / "tests"
        shutil.copytree(
            BINDING / "tests",
            tests,
            ignore=shutil.ignore_patterns("__pycache__", ".pytest_cache"),
        )
        run(
            python,
            "-I",
            "-m",
            "pytest",
            "--strict-markers",
            "--import-mode=importlib",
            tests,
            cwd=directory,
        )


def release(platform):
    require(platform in PLATFORMS, "Unsupported platform")
    require(
        not DIST.exists() or not any(DIST.iterdir()),
        "Release directory must start empty",
    )
    build_and_test(platform)
    args = (
        ["--compatibility", "manylinux_2_28", "--auditwheel", "check"]
        if sys.platform == "linux"
        else []
    )
    run(
        sys.executable,
        "-m",
        "maturin",
        "build",
        "--release",
        "--locked",
        "-i",
        sys.executable,
        "--out",
        DIST,
        *args,
        cwd=BINDING,
    )
    wheels = list(DIST.glob("*.whl"))
    require(len(wheels) == 1, "Expected exactly one wheel")
    _, _, _, tags = parse_wheel_filename(wheels[0].name)
    require(
        any(t.platform == PLATFORMS[platform] for t in tags), "Wrong wheel platform tag"
    )
    verify_wheel()


def sources():
    DIST.mkdir(parents=True, exist_ok=True)
    # Cargo's default verification extracts and compiles the .crate independently.
    run("cargo", "package", "--locked", "-p", "cqlib-core")
    shutil.copy2(ROOT / "target/package" / f"cqlib-core-{versions()[1]}.crate", DIST)
    # maturin sdist does not support --locked.
    lock = (ROOT / "Cargo.lock").read_bytes()
    run(sys.executable, "-m", "maturin", "sdist", "--out", DIST, cwd=BINDING)
    require(
        (ROOT / "Cargo.lock").read_bytes() == lock, "sdist changed checkout Cargo.lock"
    )
    sdist = DIST / f"cqlib-{versions()[0]}.tar.gz"
    with tarfile.open(sdist) as archive:
        require(
            any("/crates/cqlib-core/" in name for name in archive.getnames()),
            "sdist must include its local Rust core dependency",
        )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "command", choices=("versions", "ctest", "release", "wheel-test", "sources")
    )
    parser.add_argument("--tag")
    parser.add_argument("--wheel-dir", type=Path, default=DIST)
    parser.add_argument("--platform", choices=PLATFORMS)
    args = parser.parse_args()
    if args.command == "versions":
        versions(args.tag)
    elif args.command == "ctest":
        build_and_test()
    elif args.command == "release":
        release(args.platform)
    elif args.command == "wheel-test":
        verify_wheel(args.wheel_dir)
    else:
        sources()
