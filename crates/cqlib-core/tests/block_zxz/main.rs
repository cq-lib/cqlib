// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

//! Block-ZXZ 分解集成测试（目录式测试目标）。
//!
//! 覆盖分步数值正确性（step1/step2/step3）、端到端重建（step4）、
//! 论文 A.1/A.2 优化的恒等式/等价性/CNOT 计数（optimizations）与
//! 门构成量化检查（opt_check）。共用工具见 `common`。

mod common;
mod test_opt_check;
mod test_optimizations;
mod test_step1;
mod test_step2;
mod test_step3;
mod test_step4;
