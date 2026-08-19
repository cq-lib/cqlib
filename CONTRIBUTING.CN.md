# Cqlib 贡献指南

[English](CONTRIBUTING.md)

感谢你愿意为 Cqlib 做贡献。Cqlib 是基于 Rust 构建的高性能量子计算 SDK，提供 Rust 核心库、Python 绑定和 C 绑定，面向量子线路构建、IR、编译优化、设备模型、噪声模型、量子态模拟和误差缓解等场景。

- 官网：<https://qc.zdxlz.com/>
- 中文介绍：[README.CN.md](README.CN.md)
- 英文介绍：[README.md](README.md)
- 许可证：[Apache License 2.0](LICENSE.txt)
- 安全问题：[SECURITY.CN.md](SECURITY.CN.md)

本文说明如何报告问题、搭建开发环境、提交代码、运行测试以及参与代码评审。

## 贡献前沟通

如果你计划修复小 bug、补充测试、改文档或改进错误信息，可以直接提交 Pull Request。

如果你计划做以下改动，请先提交 Issue 讨论方案：

- 新增或修改公开 API
- 调整 Rust、Python 或 C 绑定的行为
- 引入新依赖
- 修改构建、发布或 CI 流程
- 做大规模重构
- 改变量子线路、IR、编译优化或模拟器的核心语义

提前讨论可以帮助维护者确认方向，也能避免实现完成后因为接口设计、兼容性或项目边界问题返工。

## 报告问题

提交 Issue 前，请先搜索是否已有相同或相近问题。报告 bug 时请尽量包含：

- Cqlib 版本、commit 或安装来源
- 操作系统、CPU 架构、Rust 版本和 Python 版本
- 安装方式，例如 `pip install cqlib`、`maturin develop` 或源码构建
- 最小复现代码
- 期望结果和实际结果
- 完整错误信息、日志或堆栈

如果问题涉及数值计算，请同时说明输入线路、参数、随机种子、模拟器或后端配置，以及你认为合理的误差范围。

安全漏洞请不要公开提交 Issue。请发送邮件到 <tianyan@chinatelecom.cn>，并参考 [SECURITY.CN.md](SECURITY.CN.md)。

## 项目结构

仓库主要目录如下：

```text
crates/cqlib-core/        Rust 核心库
crates/cqlib/             Rust 对外 crate
crates/binding-python/    Python 绑定及其测试
crates/binding-c/         C 绑定
docs/                     文档相关文件
```

常见改动建议优先保持在对应模块内，不要在一个 PR 中同时修改多个无关领域。

## 开发环境

### 前置条件

本项目当前要求：

- Rust 1.85+
- Python 3.10+
- C 11+ 工具链，用于 C 绑定相关开发

建议使用 `rustup` 管理 Rust 工具链，并使用 Python 虚拟环境隔离开发依赖。

### 克隆仓库

```bash
git clone https://gitee.com/cq-lib/cqlib.git
cd cqlib
```

如果你通过 fork 贡献，请从你的 fork 创建分支，并保持分支基于最新的 `main`。

### Python 环境

```bash
python3 -m venv .venv
source .venv/bin/activate
python -m pip install -U pip
python -m pip install maturin pytest pre-commit
```

在当前 Python 环境中安装本地扩展：

```bash
maturin develop -m crates/binding-python/Cargo.toml
```

如果你需要接近发布构建的运行性能，可以使用：

```bash
maturin develop --release -m crates/binding-python/Cargo.toml
```

修改 Rust 代码后，需要重新运行 `maturin develop`，Python 才能加载新的本地扩展。

## 构建

构建整个 Rust workspace：

```bash
cargo build --all
```

构建核心库：

```bash
cargo build -p cqlib-core
```

构建 Python 绑定：

```bash
maturin build --release -m crates/binding-python/Cargo.toml
```

构建 C 绑定：

```bash
cargo build -p binding-c
```

## 测试

提交 PR 前，请至少运行与你改动相关的测试。

Rust 全量测试：

```bash
cargo test --all
```

指定 Rust crate 测试：

```bash
cargo test -p cqlib-core
cargo test -p binding-c
```

Python 测试：

```bash
maturin develop -m crates/binding-python/Cargo.toml
pytest crates/binding-python/tests/
```

涉及数值计算、量子态模拟、噪声模型、编译优化、参数化线路或 FFI 边界的改动，应补充正常路径、异常路径和边界条件测试。常见边界条件包括空线路、非法量子比特索引、重复量子比特、非有限参数、维度不匹配和误差容忍范围。

## 代码风格

本项目使用 `pre-commit` 管理基础检查、Rust 格式化和 lint、Ruff、clang-format 以及拼写检查。

首次开发时启用：

```bash
pre-commit install
```

提交前运行：

```bash
pre-commit run --all-files
```

也可以按语言分别运行：

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
ruff check --fix .
ruff format .
```

C 和 C 头文件相关改动应符合仓库根目录的 `.clang-format` 配置。

## 文档

如果你的改动影响用户可见行为，请同步更新相关文档、示例或 API 注释，包括但不限于：

- README 中的安装、构建或示例代码
- Rust crate 文档和公开 API 注释
- Python 公开 API 的说明和测试示例
- C 绑定 README、头文件注释或示例

文档示例应尽量可运行。涉及量子线路或数值结果时，请明确输入、输出和误差范围。

## 分支和提交

请从 `main` 创建功能分支：

```bash
git checkout main
git pull
git checkout -b fix/short-description
```

推荐使用清晰的分支名前缀：

- `fix/`：bug 修复
- `feat/`：新功能
- `docs/`：文档修改
- `test/`：测试补充
- `refactor/`：不改变行为的重构
- `chore/`：构建、依赖或维护工作

提交信息建议说明改动类型和范围：

```text
fix(circuit): reject duplicate qubits in controlled gates
feat(qis): add density matrix fidelity helper
docs: update Python binding build instructions
```

请避免把无关格式化、临时调试代码、大文件或 IDE 配置混入提交。

## Pull Request

提交 PR 前请确认：

- PR 基于最新 `main`
- PR 标题清楚描述改动
- 相关 Issue 已在 PR 描述中关联
- 已运行与改动相关的测试和格式化检查
- 新功能或 bug 修复包含相应测试
- 用户可见行为变化已更新文档
- 没有提交无关文件、调试代码或临时输出

PR 描述建议包含：

```md
## 变更内容

## 变更原因

## 测试方式

## 相关 Issue
```

如果 PR 还未完成，请标记为 Draft，并在描述中说明剩余工作。维护者可能会要求拆分过大的 PR，使每个 PR 保持清晰、可评审、可回滚。

## 代码评审

所有代码改动都需要经过评审。评审重点包括：

- API 设计是否清晰、稳定、符合项目已有风格
- Rust 核心实现是否安全、可维护、性能合理
- Python 和 C 绑定是否与 Rust 行为一致
- 错误处理是否明确，异常或错误类型是否合适
- 测试是否覆盖正常路径、异常路径和边界条件
- 文档是否与实际行为一致

请把评审意见视为对代码质量和项目一致性的讨论。维护者会在需要时要求修改、补充测试、拆分 PR，或关闭不适合当前项目方向的改动。

## 依赖和兼容性

新增依赖前请先说明理由，尤其是会影响编译时间、发布包体积、平台兼容性或安全维护成本的依赖。新增依赖应满足：

- 许可证与 Apache License 2.0 兼容
- 有明确用途，不能被标准库或现有依赖合理替代
- 在 Rust、Python 或 C 发布链路中不会引入不必要的平台限制

不要随意提高最低 Rust 或 Python 版本要求。确有必要时，请在 Issue 或 PR 中说明原因和影响范围。

## 贡献授权

Cqlib 使用 [Apache License 2.0](LICENSE.txt) 发布。向本项目提交贡献，即表示你有权提交该贡献，并同意按照 Apache License 2.0 授权项目和接收者使用、复制、修改和分发你的贡献，除非你在提交时明确说明其他安排且维护者接受。

请不要提交你无权授权的代码、文档、数据或第三方材料。使用 AI 工具辅助生成的内容，也需要由贡献者自行审查、测试并确认不存在版权、许可证、隐私或安全问题。

## 行为准则

请保持讨论专业、尊重和聚焦问题本身。不同意见应围绕技术事实、项目目标和用户影响展开。维护者可以删除不当内容、限制参与或关闭偏离主题的讨论。
