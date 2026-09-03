# Cqlib

<div align="center">

**高性能量子计算 SDK —— 基于 Rust 构建，跨语言支持**

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE.txt)
[![Rust](https://img.shields.io/badge/rust-1.89%2B-orange.svg)](https://www.rust-lang.org)
[![Python](https://img.shields.io/badge/python-3.10%2B-blue.svg)](https://www.python.org)
[![C](https://img.shields.io/badge/c-11%2B-lightgrey.svg)](https://en.cppreference.com/w/c/11)

[English](README.md)

</div>

## 概述

Cqlib 是一个高性能量子计算 SDK，为量子程序的构建、优化与执行提供统一基础。核心采用 Rust 实现，兼顾极致性能与内存安全，并原生支持
Python 3.10+ 和 C 11+ 语言接口。

Cqlib 从底层设计兼顾科研与生产需求，覆盖量子计算全流程：从线路构建与参数化，到多层 IR 变换、基于规则的编译优化
Pass，再到搭载噪声模型的真实设备模拟，并内置零噪声外推（ZNE）和虚拟蒸馏（Virtual
Distillation）等误差缓解技术。其模块化设计支持独立使用各组件或组合为完整流水线，适用于量子算法原型开发、教学以及集成到更大规模软件栈中。

了解更多请访问 **[qc.zdxlz.com/cqlib](https://qc.zdxlz.com/cqlib)**。

## 核心特性

- **量子线路构建** — 直观的 API 用于构建、组合和参数化量子线路
- **中间表示 (IR)** — 面向量子程序的多层 IR，支持多级优化
- **编译优化** — 门分解、布局映射、路由和调度等编译器 Pass
- **设备抽象** — 真实设备模型，支持校准数据、噪声模型和拓扑结构
- **误差缓解** — 内置零噪声外推 (ZNE) 和虚拟蒸馏 (Virtual Distillation)
- **可视化** — 丰富的线路图和结果图表（SVG / 文本）
- **多语言支持** — 原生 Rust、Python (PyO3)、C (cbindgen) 接口

## 快速开始

### Python

```bash
pip install cqlib
```

```python
# Bell 态线路
from cqlib import Circuit

qc = Circuit(2)
qc.h(0)
qc.cx(0, 1)

# 查看线路信息
print(qc.num_qubits)  # 2
print(len(qc.operations))  # 2

# 获取酉矩阵
matrix = qc.to_matrix()  # 4x4 numpy complex128 数组
```

参数化线路示例：

```python
from cqlib import Circuit, Parameter

theta = Parameter("theta")

qc = Circuit(2)
qc.rx(0, theta)
qc.ry(1, theta)
qc.cx(0, 1)

# 绑定符号到具体数值
bound = qc.assign_parameters({"theta": 0.5})
```

### Rust

```toml
[dependencies]
cqlib-core = "0.1"
```

```rust
use cqlib_core::circuit::{Circuit, Qubit};

// Bell 态线路
let mut qc = Circuit::new(2);
qc.h(Qubit::new(0)).unwrap();
qc.cx(Qubit::new(0), Qubit::new(1)).unwrap();

assert_eq!(qc.num_qubits(), 2);
assert_eq!(qc.operations().len(), 2);
```

参数化线路示例：

```rust
use cqlib_core::circuit::{Circuit, Qubit, Parameter};
use std::collections::HashMap;

let theta = Parameter::symbol("θ");

let mut qc = Circuit::new(2);
qc.rx(Qubit::new(0), theta.clone()).unwrap();
qc.ry(Qubit::new(1), theta).unwrap();
qc.cx(Qubit::new(0), Qubit::new(1)).unwrap();

// 绑定参数
let mut bindings = HashMap::new();
bindings.insert("θ", std::f64::consts::PI);
let evaluated = qc.assign_parameters( & Some(bindings)).unwrap();
```

### C

```c
#include <cqlib/circuit.h>

int main(void) {
    // 创建 2 比特 Bell 态线路
    cqlib_circuit_t *qc = cqlib_circuit_new(2);
    cqlib_circuit_h(qc, 0);
    cqlib_circuit_cx(qc, 0, 1);

    // 查看线路信息
    uint32_t n = cqlib_circuit_num_qubits(qc);    // 2
    size_t ops = cqlib_circuit_operation_count(qc); // 2

    cqlib_circuit_free(qc);
    return 0;
}
```

## 语言支持

| 语言     | 最低版本 | 绑定技术              |
|--------|------|-------------------|
| Rust   | 1.89 | 原生                |
| Python | 3.10 | PyO3 (abi3-py310) |
| C      | 11   | cbindgen          |

## 从源码构建

**前置条件：** Rust 1.89+，Python 3.10+（用于 Python 绑定）

```bash
git clone https://gitee.com/cq-lib/cqlib.git
cd cqlib

# 构建核心库
cargo build --release -p cqlib-core

# 构建 Python 绑定
pip install maturin
maturin develop --release -m crates/binding-python/Cargo.toml

# 构建 C 绑定
cargo build --release -p binding-c
```

运行测试：

```bash
cargo test --all
pytest crates/binding-python/tests/
```

## 文档

| 资源              | 链接                                                       |
|-----------------|----------------------------------------------------------|
| API 参考 (Rust)   | [docs.rs/cqlib-core](https://docs.rs/cqlib-core)         |
| API 参考 (Python) | [qc.zdxlz.com/docs](https://qc.zdxlz.com/)               |
| 源码仓库 (Gitee)    | [gitee.com/cq-lib/cqlib](https://gitee.com/cq-lib/cqlib) |
| 源码仓库 (GitHub)   | [github.com/cq-lib/cqlib](https://github.com/cq-lib/cqlib) |

## 参与贡献

欢迎贡献代码。提交改动前请阅读 [贡献指南](CONTRIBUTING.CN.md)，并遵守 [行为准则](CODE_OF_CONDUCT.CN.md)。如果你需要报告安全问题，请参考 [安全策略](SECURITY.CN.md)。

请通过仓库 Issue 进行讨论。提交 Pull Request 时，请基于 `main` 分支，并确保相关测试通过。

## 许可证

Cqlib 采用 [Apache License, Version 2.0](LICENSE.txt) 许可证。
