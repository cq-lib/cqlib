# Python API Overview

## Welcome to the Cqlib Python API reference!

## Documentation navigation

### Quantum Circuit (`cqlib.circuit`)

- [Overview](0_circuit/0_overview.md)
- [Circuit](0_circuit/1_circuit.md)
- [Qubit](0_circuit/2_qubit.md)
- [Parameter](0_circuit/3_parameter.md)
- [Operation / Instruction](0_circuit/4_operation_instruction.md)
- [StandardGate](0_circuit/5_gates_standard.md)
- [UnitaryGate](0_circuit/6_gates_unitary.md)
- [MCGate](0_circuit/7_gates_mc_gate.md)
- [CircuitGate / FrozenCircuit](0_circuit/8_gates_circuit_gate.md)
- [Classical / Control Flow](0_circuit/9_classical_control_flow.md)
- [SymbolicMatrix](0_circuit/10_symbolic_matrix.md)
- [Ansatz](0_circuit/11_ansatz.md)
- [Circuit To Matrix](0_circuit/12_circuit_to_matrix.md)
- [CircuitDag](0_circuit/13_circuit_dag.md)

### Intermediate Representation (`cqlib.ir`)

- [Overview](1_ir/0_overview.md)
- [QCIS](1_ir/1_qcis.md)
- [OpenQASM 2.0](1_ir/2_qasm2.md)
- [OpenQASM 3.0](1_ir/3_qasm3.md)

### Device (`cqlib.device`)

- [Overview](2_device/0_overview.md)
- [Topology](2_device/1_topology.md)
- [Device and noise properties](2_device/2_properties_device.md)
- [Layout](2_device/3_layout.md)
- [Noise](2_device/4_noise.md)
- [Result](2_device/5_result.md)
- [Qubit Identifiers](2_device/6_qubits.md)

### Quantum Information (`cqlib.qis`)

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

### Compilation (`cqlib.compile`)

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

### Visualization (`cqlib.visualization`)

- [Overview](5_visualization/0_overview.md)
- [Text circuit diagrams](5_visualization/1_draw_text.md)
- [SVG circuit diagrams](5_visualization/2_draw_figure.md)
- [Quantum state visualization](5_visualization/3_state_plots.md)
- [Measurement result visualization](5_visualization/4_result_plots.md)
- [Write to file and inline display](5_visualization/5_render_to_file.md)

### Error Mitigation (`cqlib.error_mitigation`)

- [Overview](6_error_mitigation/0_overview.md)
- [Zero-noise extrapolation (ZNE)](6_error_mitigation/1_zne.md)
- [Virtual distillation](6_error_mitigation/2_virtual_distillation.md)
- [Unified pipeline](6_error_mitigation/3_unified.md)
