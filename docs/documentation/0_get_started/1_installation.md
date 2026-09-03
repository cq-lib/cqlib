# Cqlib 安装与环境配置

本指南旨在帮助您快速完成 Cqlib 的安装与开发环境配置。

在安装 Cqlib 之前，请确保您的计算机环境满足以下条件：

- Python 环境：支持 Python 3.10 – 3.14（建议使用 64 位版本）；
- 操作系统：
  - Linux
  - macOS
  - Windows

---

## 方式 A：通过 `pip` 快速安装（推荐）

对于大多数用户，推荐使用 `pip` 直接安装。


> 适用场景：算法研发、原型验证。
> 注意事项：此方式已包含预编译二进制文件，无需安装 Rust 或 C 编译器。

### 第一步：创建并激活虚拟环境

建议在独立的环境中进行操作，以避免依赖冲突：

```bash
python -m venv cqlib-env

# 激活环境 (Windows)
cqlib-env\Scripts\activate
# 激活环境 (Linux/macOS)
source cqlib-env/bin/activate
```

### 第二步：`pip` 一键安装

```bash
pip install cqlib
```

### 第三步：安装验证

安装完成后，可以通过以下方式验证：

```python
python -m pip show cqlib
```
如果能够正确输出版本信息，则说明安装成功。

---

## 方式 B：从源码构建（面向开发者）

如果您希望参与 Cqlib 的开发，或使用尚未发布的最新功能，可以使用下列方法从源码构建。

> 适用场景：定制化开发、贡献代码或使用最新特性。
> 注意事项：此方式要求本地具备编译环境。

### 第一步：配置编译链工具

在构建前，请根据您的操作系统安装以下必要组件：

- 安装 Rust 工具链（Stable 1.89+）：

  - Windows：访问官方网站： https://www.rust-lang.org/tools/install 下载并运行安装程序。
  - Linux/macOS：执行 `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`

- 安装 C 编译器：
  - Windows：安装 Visual Studio 生成工具，并勾选“使用 C++ 的桌面开发”。
  - Linux：安装 `build-essential`（Ubuntu/Debian）或 `base-devel`（Arch）。
  - macOS：终端执行 `xcode-select --install`。

安装完成后，可通过以下命令验证：

```bash
rustc --version
cargo --version
```
如果能够正确输出版本号，则说明 Rust 安装成功。

### 第二步：获取源代码

从 Gitee 克隆仓库到本地：
```bash
git clone https://gitee.com/zdxlz/cqlib2.git
cd cqlib
```

### 第三步：编译并安装 Python 绑定

我们使用 `maturin` 跨语言工具将 Rust 核心编译为 Python 模块：

```bash
# 1. 安装构建工具
pip install -U maturin

# 2. 编译并安装到当前环境
# --release 参数可确保获得最佳运行性能
maturin develop --release -m crates/binding-python/Cargo.toml
```
该命令将在当前虚拟环境中安装本地构建版本。

### 第四步：构建 Rust 核心与 C 接口（可选）

```bash
# 构建核心库
cargo build --release

# 构建 C 接口 ABI（可选）
cargo build -p binding-c --release
```

---

## 可选模块：安装天衍量子云平台客户端

如果需要将本地线路提交到天衍量子云平台执行，还需要安装 `cqlib-tianyan`。该模块独立于 Cqlib 核心包，主要负责平台认证、后端查询、任务提交、任务轮询和结果解析。

```bash
pip install cqlib-tianyan
```

安装后可通过以下方式验证：

```python
from cqlib_tianyan import TianyanPlatform
print(TianyanPlatform)
```

`cqlib-tianyan` 通常与 `cqlib` 配合使用：先用 `cqlib.circuit` 构建线路，再用 `cqlib.ir.qcis` 导出 QCIS，最后通过 `cqlib_tianyan` 提交到云端后端。

---

## 下一步

安装完成后，建议继续阅读以下内容：

- [快速开始](2_quickstart.md)：从“0”到“1”的第一个量子线路
- [量子线路](../1_cqlib/0_circuit/0_overview.md)：了解 Cqlib 中描述量子程序的基础模块。
- [量子门与指令](../1_cqlib/0_circuit/1_gates.md)：了解内置门、自定义门、复合门和非酉指令。
- [天衍量子云平台客户端](../1_cqlib/7_tianyan/0_overview.md)：了解云端后端接入、任务提交和结果获取流程。
