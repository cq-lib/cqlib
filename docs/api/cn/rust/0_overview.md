# Rust API 概览

## 欢迎查阅 Cqlib Rust API 参考手册！

## 文档导航

### 量子电路（`cqlib_core::circuit`）

- [Overview](0_circuit/0_overview.md)
- [Circuit](0_circuit/1_circuit.md)
- [Qubit](0_circuit/2_qubit.md)
- [Parameters](0_circuit/3_parameter.md)
- [Operations / Instructions](0_circuit/4_operation_instruction.md)
- [Standard Gates](0_circuit/5_gate_standard.md)
- [Unitary Gates](0_circuit/6_gate_unitary.md)
- [Multi-Controlled Gates](0_circuit/7_gate_mc_gate.md)
- [Circuit Gates](0_circuit/8_gate_circuit_gate.md)
- [Classical / Control Flow](0_circuit/9_classical_control_flow.md)
- [Symbolic Matrix](0_circuit/10_symbolic_matrix.md)
- [Ansatz](0_circuit/11_ansatz.md)
- [CFG](0_circuit/12_cfg.md)
- [Circuit to Matrix](0_circuit/13_circuit_to_matrix.md)
- [CircuitDag](0_circuit/14_circuit_dag.md)

### 中间表达（`cqlib_core::ir`）

- [Overview](1_ir/0_overview.md)
- [QCIS](1_ir/1_qcis.md)
- [OpenQASM 2.0](1_ir/2_qasm2.md)
- [OpenQASM 3.0](1_ir/3_qasm3.md)

### 设备模块（`cqlib_core::device`）

- [Overview](2_device/0_overview.md)
- [Topology](2_device/1_topology.md)
- [Device 与噪声属性](2_device/2_properties_device.md)
- [Layout](2_device/3_layout.md)
- [Noise](2_device/4_noise.md)
- [Result](2_device/5_result.md)
- [量子比特标识](2_device/6_qubits.md)

### 量子信息（`cqlib_core::qis`）

- [Overview](3_qis/0_overview.md)
- [Statevector](3_qis/1_statevector.md)
- [DensityMatrix](3_qis/2_density_matrix.md)
- [DensityMatrixNoise](3_qis/3_density_matrix_noise.md)
- [Stabilizer](3_qis/4_stabilizer.md)
- [ClassicalState](3_qis/5_classical_state.md)
- [Pauli](3_qis/6_pauli.md)
- [Hamiltonian](3_qis/7_hamiltonian.md)
- [Evolution](3_qis/8_evolution.md)
- [Metrics / Entropy](3_qis/9_metrics_entropy.md)

### 编译优化（`cqlib_core::compile`）

- [Overview](4_compile/0_overview.md)
- [Compiler](4_compile/1_compiler.md)
- [Transform](4_compile/2_transform.md)
- [Layout](4_compile/3_layout.md)
- [Routing](4_compile/4_routing.md)
- [SABRE](4_compile/5_sabre.md)
- [Decompose / Resynthesis](4_compile/6_decompose_resynthesis.md)
- [Knowledge](4_compile/7_knowledge.md)
- [Resource](4_compile/8_resource.md)
- [Commutation](4_compile/9_commutation.md)

### 可视化（`cqlib_core::visualization`）

- [Overview](5_visualization/0_overview.md)
- [文本线路图](5_visualization/1_draw_text.md)
- [SVG 线路图](5_visualization/2_draw_figure.md)
- [量子态可视化](5_visualization/3_state_plots.md)
- [测量结果可视化](5_visualization/4_result_plots.md)
- [落盘与输出](5_visualization/5_render_to_file.md)
- [视觉 IR](5_visualization/6_visual_ir.md)

### 错误缓解（`cqlib_core::error_mitigation`）

- [Overview](6_error_mitigation/0_overview.md)
- [零噪声外推](6_error_mitigation/1_zne.md)
- [虚拟蒸馏](6_error_mitigation/2_virtual_distillation.md)
- [统一流水线](6_error_mitigation/3_unified.md)
