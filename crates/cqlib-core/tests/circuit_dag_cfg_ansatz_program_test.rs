//! circuit 模块全量功能测试 —— 文件 4：DAG、CFG、符号矩阵与 Ansatz
//!
//! 被测目标：crates/cqlib-core/src/circuit/{dag, cfg, symbolic_matrix, ansatz}.rs
//! 与 circuit_to_matrix.rs 的全部公开 API。
//!
//! 运行：cargo test -p cqlib-core --test circuit_dag_cfg_ansatz_program_test -- --nocapture

use std::collections::HashMap;
use std::f64::consts::PI;
use std::sync::Arc;

use cqlib_core::circuit::ansatz::{
    AngleEncoding, Ansatz, BasicEntanglerLayers, BasisEncoding, EntanglementTopology,
    IQPFeatureMap, PauliEvolutionAnsatz, QAOAAnsatz, StronglyEntanglingLayers, TwoLocal,
    ZFeatureMap, ZZFeatureMap, efficient_su2, real_amplitudes, zz_feature_map,
};
use cqlib_core::circuit::cfg::{BasicBlock, CircuitCFG, FlowEdge, Terminator};
use cqlib_core::circuit::dag::{CircuitDag, DagControlFlow, DagWire};
use cqlib_core::circuit::symbolic_matrix::{
    SymbolicComplex, circuits_equivalent, circuit_to_symbolic_matrix, evaluate_symbolic_matrix,
    standard_gate_symbolic_matrix, substitute_symbolic_matrix, symbolic_eye,
    symbolic_matrices_equivalent,
};
use cqlib_core::circuit::gate::FrozenCircuit;
use cqlib_core::circuit::{
    Circuit, CircuitError, ClassicalExpr, ClassicalType, Instruction, Operation, Parameter,
    Qubit, StandardGate, ValueOperation,
};
use cqlib_core::qis::{Hamiltonian, PauliString};
use ndarray::Array2;
use num_complex::Complex64;
use rustworkx_core::petgraph::prelude::NodeIndex;
use smallvec::smallvec;

fn q(id: u32) -> Qubit {
    Qubit::new(id)
}

fn max_delta(a: &Array2<Complex64>, b: &Array2<Complex64>) -> f64 {
    let mut max = 0.0f64;
    for (x, y) in a.iter().zip(b.iter()) {
        max = max.max((*x - *y).norm());
    }
    max
}

fn bell() -> Circuit {
    let mut c = Circuit::new(2);
    c.h(q(0)).unwrap();
    c.cx(q(0), q(1)).unwrap();
    c
}

fn raw_h_op(qubit: Qubit) -> Operation {
    Operation {
        instruction: Instruction::Standard(StandardGate::H),
        qubits: smallvec![qubit],
        params: smallvec![],
        label: None,
    }
}

fn raw_x_op(qubit: Qubit) -> Operation {
    Operation {
        instruction: Instruction::Standard(StandardGate::X),
        qubits: smallvec![qubit],
        params: smallvec![],
        label: None,
    }
}

fn raw_rz_op(qubit: Qubit, angle: f64) -> Operation {
    Operation {
        instruction: Instruction::Standard(StandardGate::RZ),
        qubits: smallvec![qubit],
        params: smallvec![cqlib_core::circuit::CircuitParam::Fixed(angle)],
        label: None,
    }
}

// ==================== 一、CircuitDag ====================

#[test]
fn dag_from_circuit_basic_properties() {
    let dag = CircuitDag::from_circuit(&bell()).unwrap();
    assert_eq!(dag.num_qubits(), 2);
    assert_eq!(dag.num_ops(), 2);
    assert!(!dag.is_empty());
    assert_eq!(dag.qubits(), vec![q(0), q(1)]);
    dag.validate().unwrap();
    // 空线路
    let empty = CircuitDag::from_circuit(&Circuit::new(2)).unwrap();
    assert!(empty.is_empty());
    assert_eq!(empty.num_qubits(), 2);
    empty.validate().unwrap();
    println!("[dag] from_circuit：Bell 2 op / 空 0 op，validate 均通过");
}

#[test]
fn dag_to_circuit_roundtrip() {
    let source = bell();
    let dag = CircuitDag::from_circuit(&source).unwrap();
    let rebuilt = dag.to_circuit().unwrap();
    assert_eq!(rebuilt, source, "拓扑重建应保持结构相等");
    assert!(max_delta(
        &rebuilt.to_matrix(None).unwrap(),
        &source.to_matrix(None).unwrap()
    ) < 1e-14);
    // 带符号参数与经典表
    let mut src2 = Circuit::new(1);
    src2.rx(q(0), Parameter::symbol("θ")).unwrap();
    let m = src2.measure(q(0)).unwrap();
    let flag = src2.var(ClassicalType::Bool);
    src2
        .store(flag, ClassicalExpr::bit_to_bool(m.expr()).unwrap())
        .unwrap();
    src2.set_global_phase(Parameter::from(0.25));
    let dag2 = CircuitDag::from_circuit(&src2).unwrap();
    let rebuilt2 = dag2.to_circuit().unwrap();
    rebuilt2.validate().unwrap();
    assert_eq!(rebuilt2, src2);
    assert_eq!(rebuilt2.global_phase().evaluate(&None).unwrap(), 0.25);
    println!("[dag] to_circuit：纯/参数化/含经典表线路均无损往返");
}

#[test]
fn dag_from_operations_validation() {
    let ops = [raw_h_op(q(0)), raw_x_op(q(0))];
    let dag = CircuitDag::from_operations([q(0), q(1)], &ops).unwrap();
    assert_eq!(dag.num_ops(), 2);
    dag.validate().unwrap();
    // 重复比特
    assert!(matches!(
        CircuitDag::from_operations([q(0), q(0)], &ops),
        Err(CircuitError::DuplicateQubits)
    ));
    // 未知比特
    assert!(matches!(
        CircuitDag::from_operations([q(0)], &[raw_h_op(q(5))]),
        Err(CircuitError::QubitNotFound(5))
    ));
    println!("[dag] from_operations：正常构建 + 重复/未知比特拒绝");
}

#[test]
fn dag_wires_and_sentinels() {
    let dag = CircuitDag::from_circuit(&bell()).unwrap();
    assert!(dag.has_wire(DagWire::Qubit(q(0))));
    assert!(dag.has_wire(DagWire::Qubit(q(1))));
    assert!(dag.has_wire(DagWire::GlobalOrder));
    assert!(!dag.has_wire(DagWire::Qubit(q(9))));
    let wire_count = dag.wires().count();
    assert_eq!(wire_count, 3, "两条量子线 + 全局顺序线");
    assert!(dag.wire_in(DagWire::Qubit(q(0))).is_some());
    assert!(dag.wire_out(DagWire::Qubit(q(0))).is_some());
    assert!(dag.wire_in(DagWire::Qubit(q(9))).is_none());
    // 空闲判定
    let mut three = Circuit::new(3);
    three.h(q(0)).unwrap();
    let dag3 = CircuitDag::from_circuit(&three).unwrap();
    assert!(!dag3.is_wire_idle(DagWire::Qubit(q(0))).unwrap());
    assert!(dag3.is_wire_idle(DagWire::Qubit(q(2))).unwrap());
    assert!(dag3.is_wire_idle(DagWire::Qubit(q(9))).is_err(), "未知线应报错");
    println!("[dag] 线集合/哨兵节点/空闲判定全部正确");
}

#[test]
fn dag_node_kinds_and_operation_accessor() {
    let dag = CircuitDag::from_circuit(&bell()).unwrap();
    let h_node = dag.op_nodes().next().unwrap();
    assert!(dag.is_operation(h_node));
    assert_eq!(dag.node_kind(h_node), Some("operation"));
    assert_eq!(dag.operation(h_node).unwrap().instruction.name(), "H");
    // 哨兵节点
    let wire_in = dag.wire_in(DagWire::Qubit(q(0))).unwrap();
    assert!(!dag.is_operation(wire_in));
    assert_eq!(dag.node_kind(wire_in), Some("wire_in"));
    let wire_out = dag.wire_out(DagWire::Qubit(q(0))).unwrap();
    assert_eq!(dag.node_kind(wire_out), Some("wire_out"));
    assert!(dag.operation(wire_in).is_none());
    // 不存在的节点索引
    let missing = NodeIndex::new(9999);
    assert!(dag.operation(missing).is_none());
    assert_eq!(dag.node_kind(missing), None);
    println!("[dag] 节点种类（operation/wire_in/wire_out/缺失）全部正确");
}

#[test]
fn dag_topological_order_and_front_layer() {
    // 并行结构：h(0), rz(1) 互不依赖 → 前层两个节点
    let mut c = Circuit::new(2);
    c.h(q(0)).unwrap();
    c.rz(q(1), 0.5).unwrap();
    c.cx(q(0), q(1)).unwrap();
    let dag = CircuitDag::from_circuit(&c).unwrap();
    // 源顺序
    let src_names: Vec<String> = dag
        .op_nodes()
        .map(|n| dag.operation(n).unwrap().instruction.name())
        .collect();
    assert_eq!(src_names, vec!["H".to_string(), "RZ".into(), "CX".into()]);
    let topo = dag.topological_op_nodes().unwrap();
    assert_eq!(topo.len(), 3);
    let front = dag.front_layer().unwrap();
    assert_eq!(front.len(), 2, "H 与 RZ 均无操作前驱");
    for node in &front {
        assert!(
            dag.predecessors(*node).all(|p| !dag.is_operation(p)),
            "前层节点不应有操作前驱"
        );
    }
    println!("[dag] 源顺序 [H,RZ,CX] 保持；前层 2 节点（并行可见）");
}

#[test]
fn dag_layers_and_depth() {
    // 并行 → 1 层；随后串行 cx → 2 层
    let mut c = Circuit::new(2);
    c.h(q(0)).unwrap();
    c.h(q(1)).unwrap();
    let parallel = CircuitDag::from_circuit(&c).unwrap();
    assert_eq!(parallel.layers().unwrap().len(), 1);
    assert_eq!(parallel.depth().unwrap(), 1);
    let mut layered = Circuit::new(2);
    layered.h(q(0)).unwrap();
    layered.cx(q(0), q(1)).unwrap();
    let chain = CircuitDag::from_circuit(&layered).unwrap();
    assert_eq!(chain.layers().unwrap().len(), 2);
    assert_eq!(chain.depth().unwrap(), 2);
    // node_layers 映射
    let node_layers = chain.node_layers().unwrap();
    assert_eq!(node_layers.len(), 2);
    for (node, layer) in node_layers.iter() {
        let expect = if chain.operation(*node).unwrap().instruction.name() == "H" { 0 } else { 1 };
        assert_eq!(*layer, expect);
    }
    // 与 Circuit::depth 一致
    assert_eq!(chain.depth().unwrap(), layered.depth(false).unwrap());
    println!("[dag] layers/depth/node_layers 与 Circuit::depth 数值一致");
}

#[test]
fn dag_wire_neighbors() {
    let dag = CircuitDag::from_circuit(&bell()).unwrap();
    let nodes = dag.nodes_on_wire(DagWire::Qubit(q(0))).unwrap();
    assert_eq!(nodes.len(), 2);
    assert_eq!(dag.operation(nodes[0]).unwrap().instruction.name(), "H");
    assert_eq!(dag.operation(nodes[1]).unwrap().instruction.name(), "CX");
    // 前驱/后继（按线）
    let preds = dag.predecessors_on_wire(nodes[1], DagWire::Qubit(q(0))).unwrap();
    assert_eq!(preds, vec![nodes[0]]);
    let succs = dag.successors_on_wire(nodes[0], DagWire::Qubit(q(0))).unwrap();
    assert_eq!(succs, vec![nodes[1]]);
    // 未知线报错
    assert!(dag.nodes_on_wire(DagWire::Qubit(q(9))).is_err());
    assert!(dag.predecessors_on_wire(nodes[0], DagWire::Qubit(q(9))).is_err());
    println!("[dag] nodes_on_wire 顺序 + 按线前驱/后继 + 未知线拒绝");
}

#[test]
fn dag_quantum_and_classical_neighbors() {
    let mut c = Circuit::new(1);
    let m = c.measure(q(0)).unwrap();
    let flag = c.var(ClassicalType::Bool);
    c.store(flag, ClassicalExpr::bit_to_bool(m.expr()).unwrap())
        .unwrap();
    c.x(q(0)).unwrap();
    let dag = CircuitDag::from_circuit(&c).unwrap();
    let mut names = vec![];
    for node in dag.op_nodes() {
        names.push(dag.operation(node).unwrap().instruction.name());
    }
    assert_eq!(names, vec!["measure_bit".to_string(), "store".into(), "X".into()]);
    let measure_node = dag.op_nodes().next().unwrap();
    let store_node = dag.op_nodes().nth(1).unwrap();
    // 经典后继：measure → store
    let classical_succ = dag.classical_successors(measure_node).unwrap();
    assert!(classical_succ.contains(&store_node));
    // 量子后继：measure → X
    let quantum_succ = dag.quantum_successors(measure_node).unwrap();
    let x_node = dag.op_nodes().nth(2).unwrap();
    assert!(quantum_succ.contains(&x_node));
    // store 的经典前驱是 measure
    let classical_pred = dag.classical_predecessors(store_node).unwrap();
    assert!(classical_pred.contains(&measure_node));
    // store 无量子邻居
    assert!(dag.quantum_predecessors(store_node).unwrap().is_empty());
    println!("[dag] 经典/量子邻接分离：measure→store（经典）、measure→X（量子）");
}

#[test]
fn dag_control_flow_payloads() {
    let mut c = Circuit::new(2);
    c.if_else(
        ClassicalExpr::bool_literal(true),
        |t| {
            t.x(q(0))?;
            Ok(())
        },
        |e| {
            e.z(q(1))?;
            Ok(())
        },
    )
    .unwrap();
    c.while_(ClassicalExpr::bool_literal(false), |w| {
        w.h(q(1))?;
        Ok(())
    })
    .unwrap();
    let dag = CircuitDag::from_circuit(&c).unwrap();
    assert!(dag.has_control_flow());
    assert!(!dag.has_nested_control_flow(), "本例无嵌套");
    let if_node = dag.op_nodes().next().unwrap();
    let Some(DagControlFlow::If { then_body, else_body }) = dag.control_flow(if_node) else {
        panic!("应为 If 载荷");
    };
    assert_eq!(then_body.num_ops(), 1);
    assert_eq!(else_body.as_ref().unwrap().num_ops(), 1);
    let while_node = dag.op_nodes().nth(1).unwrap();
    assert!(matches!(
        dag.control_flow(while_node),
        Some(DagControlFlow::While { .. })
    ));
    // 无控制流的普通节点无载荷
    let plain = CircuitDag::from_circuit(&bell()).unwrap();
    assert!(!plain.has_control_flow());
    assert!(plain.control_flow(plain.op_nodes().next().unwrap()).is_none());
    println!("[dag] 控制流载荷：If 双体 / While 体 / 普通节点无载荷");
}

#[test]
fn dag_nested_control_flow_and_measurement_detection() {
    // for 内嵌 if → has_nested_control_flow
    let mut c = Circuit::new(1);
    let counter = c.var(ClassicalType::uint(8).unwrap());
    c.for_uint(
        counter,
        ClassicalExpr::uint_literal(8, 0).unwrap(),
        ClassicalExpr::uint_literal(8, 2).unwrap(),
        ClassicalExpr::uint_literal(8, 1).unwrap(),
        |body, _| {
            body.if_(ClassicalExpr::bool_literal(true), |inner| {
                inner.x(q(0))?;
                Ok(())
            })?;
            Ok(())
        },
    )
    .unwrap();
    let dag = CircuitDag::from_circuit(&c).unwrap();
    assert!(dag.has_control_flow());
    assert!(dag.has_nested_control_flow());
    // 体内测量递归检测
    let mut m = Circuit::new(1);
    m.if_(ClassicalExpr::bool_literal(true), |body| {
        body.measure(q(0))?;
        Ok(())
    })
    .unwrap();
    let mdag = CircuitDag::from_circuit(&m).unwrap();
    assert!(mdag.has_measurement());
    let plain = CircuitDag::from_circuit(&bell()).unwrap();
    assert!(!plain.has_measurement());
    println!("[dag] 嵌套控制流 + 体内测量递归检测正确");
}

#[test]
fn dag_operation_count_by_name() {
    let mut c = Circuit::new(2);
    c.h(q(0)).unwrap();
    c.h(q(1)).unwrap();
    c.cx(q(0), q(1)).unwrap();
    let dag = CircuitDag::from_circuit(&c).unwrap();
    let counts = dag.operation_count_by_name();
    assert_eq!(counts.get("H"), Some(&2));
    assert_eq!(counts.get("CX"), Some(&1));
    assert_eq!(counts.len(), 2);
    // 递归计数：if 体内的 x
    let mut nested = Circuit::new(1);
    nested.h(q(0)).unwrap();
    nested
        .if_(ClassicalExpr::bool_literal(true), |body| {
            body.x(q(0))?;
            body.x(q(0))?;
            Ok(())
        })
        .unwrap();
    let ndag = CircuitDag::from_circuit(&nested).unwrap();
    let top = ndag.operation_count_by_name();
    assert_eq!(top.get("X"), None, "顶层计数不含体内操作");
    let recursive = ndag.operation_count_by_name_recursive();
    assert_eq!(recursive.get("X"), Some(&2));
    assert_eq!(recursive.get("H"), Some(&1));
    println!("[dag] 计数：顶层 vs 递归（if 体内 X×2 计入）");
}

#[test]
fn dag_collect_runs() {
    let mut c = Circuit::new(2);
    c.h(q(0)).unwrap();
    c.x(q(0)).unwrap();
    c.cx(q(0), q(1)).unwrap();
    c.z(q(0)).unwrap();
    c.s(q(0)).unwrap();
    let dag = CircuitDag::from_circuit(&c).unwrap();
    // 1q 连续段（q0）：[h,x] 与 [z,s]，两段
    let runs1q = dag.collect_1q_runs().unwrap();
    assert_eq!(runs1q.len(), 2, "被 CX 打断");
    assert_eq!(runs1q[0].len(), 2);
    assert_eq!(runs1q[1].len(), 2);
    // 2q 段：CX 出现在两条量子线上
    let runs2q = dag.collect_2q_runs().unwrap();
    assert_eq!(runs2q.len(), 2, "CX 在 q0 与 q1 各计入一次");
    assert_eq!(runs2q[0].len(), 1);
    // 谓词版：仅 1 比特 Standard 门 → 被 CX 打断成两段
    let single_qubit_runs = dag
        .collect_runs_on_wire(DagWire::Qubit(q(0)), |op| {
            matches!(op.instruction, Instruction::Standard(_)) && op.qubits.len() == 1
        })
        .unwrap();
    assert_eq!(single_qubit_runs.len(), 2);
    // 谓词含 CX（同为 Standard）→ 5 个操作连成 1 段
    let all_standard = dag
        .collect_runs_on_wire(DagWire::Qubit(q(0)), |op| {
            matches!(op.instruction, Instruction::Standard(_))
        })
        .unwrap();
    assert_eq!(all_standard.len(), 1);
    assert_eq!(all_standard[0].len(), 5);
    // 全拒谓词 → 空
    let none = dag
        .collect_runs_on_wire(DagWire::Qubit(q(0)), |_| false)
        .unwrap();
    assert!(none.is_empty());
    println!("[dag] 连续段收集：1q 两段、2q 双线、谓词过滤（含/不含 CX、全拒）全部正确");
}

#[test]
fn dag_apply_and_remove_operations() {
    let mut dag = CircuitDag::from_circuit(&bell()).unwrap();
    let before = dag.num_ops();
    // 尾部追加
    let node = dag.apply_operation_back(raw_rz_op(q(1), 0.4)).unwrap();
    assert_eq!(dag.num_ops(), before + 1);
    assert_eq!(dag.operation(node).unwrap().instruction.name(), "RZ");
    // 头部前插
    let front = dag.apply_operation_front(raw_x_op(q(0))).unwrap();
    assert_eq!(dag.operation(front).unwrap().instruction.name(), "X");
    // 值层追加/前插
    let vo_node = dag
        .apply_value_operation_back(ValueOperation::from_standard(
            StandardGate::S,
            [q(0)],
            [],
        ))
        .unwrap();
    assert_eq!(dag.operation(vo_node).unwrap().instruction.name(), "S");
    let vo_front = dag
        .apply_value_operation_front(ValueOperation::from_standard(
            StandardGate::T,
            [q(1)],
            [],
        ))
        .unwrap();
    assert_eq!(dag.operation(vo_front).unwrap().instruction.name(), "T");
    dag.validate().unwrap();
    // 重建线路：front 前插=成为新的第 0 序操作，back 追加=排最后
    let rebuilt = dag.to_circuit().unwrap();
    let names: Vec<String> = rebuilt
        .operations()
        .iter()
        .map(|op| op.instruction.name())
        .collect();
    assert_eq!(
        names,
        vec![
            "T".to_string(),
            "X".into(),
            "H".into(),
            "CX".into(),
            "RZ".into(),
            "S".into()
        ],
        "后插的 T 前于先插的 X；S 为最新 back 追加"
    );
    // 删除：apply_* 系列会全量重建图、使旧 NodeIndex 句柄失效，
    // 故删除前按名字重新定位 S 节点（行为发现，已记入报告质量观察）
    let s_node = dag
        .op_nodes()
        .find(|&n| dag.operation(n).unwrap().instruction.name() == "S")
        .expect("应存在 S 节点");
    let removed = dag.remove_op_node(s_node).unwrap();
    assert_eq!(removed.instruction.name(), "S");
    assert_eq!(dag.num_ops(), before + 3);
    dag.validate().unwrap();
    let after = dag.to_circuit().unwrap();
    assert_eq!(after.operations().last().unwrap().instruction.name(), "RZ");
    println!("[dag] apply front/back（双形态）+ remove + 重建顺序全部正确");
}

#[test]
fn dag_substitute_operations() {
    let mut dag = CircuitDag::from_circuit(&bell()).unwrap();
    let h_node = dag.op_nodes().next().unwrap();
    // 节点替换
    dag.substitute_node(h_node, raw_x_op(q(0))).unwrap();
    assert_eq!(dag.operation(h_node).unwrap().instruction.name(), "X");
    dag.validate().unwrap();
    // 值层替换
    dag.substitute_value_node(
        h_node,
        ValueOperation::from_standard(StandardGate::Y, [q(0)], []),
    )
    .unwrap();
    assert_eq!(dag.operation(h_node).unwrap().instruction.name(), "Y");
    dag.validate().unwrap();
    // 用 DAG 替换：Y → [H, X]（同比特）
    let replacement = CircuitDag::from_operations([q(0)], &[raw_h_op(q(0)), raw_x_op(q(0))]).unwrap();
    dag.substitute_node_with_dag(h_node, replacement).unwrap();
    let names: Vec<String> = dag
        .op_nodes()
        .map(|n| dag.operation(n).unwrap().instruction.name())
        .collect();
    assert_eq!(names, vec!["H".to_string(), "X".into(), "CX".into()]);
    dag.validate().unwrap();
    println!("[dag] substitute：单节点/值层/子 DAG 三种替换后 validate 均通过");
}

#[test]
fn dag_metadata_tables() {
    let mut c = Circuit::new(1);
    c.rx(q(0), Parameter::symbol("θ")).unwrap();
    let m = c.measure(q(0)).unwrap();
    let flag = c.var(ClassicalType::Bool);
    c.store(flag, ClassicalExpr::bit_to_bool(m.expr()).unwrap())
        .unwrap();
    let mut dag = CircuitDag::from_circuit(&c).unwrap();
    assert_eq!(dag.parameters().len(), 1);
    assert!(dag.symbols().contains("θ"));
    assert_eq!(dag.classical_vars(), &[ClassicalType::Bool]);
    assert_eq!(dag.classical_values(), &[ClassicalType::Bit]);
    // add_parameter 镜像 Circuit 语义
    let (idx, is_new) = dag.add_parameter(Parameter::symbol("θ"));
    assert!(!is_new, "已有参数命中");
    let (idx2, is_new2) = dag.add_parameter(Parameter::symbol("φ"));
    assert!(is_new2);
    assert_ne!(idx, idx2);
    assert!(dag.symbols().contains("φ"));
    println!("[dag] 参数/符号/经典表镜像 + add_parameter 去重语义正确");
}

// ==================== 二、CircuitCFG ====================

#[test]
fn cfg_from_circuit_linear_roundtrip() {
    let source = bell();
    let cfg = CircuitCFG::from_circuit(&source).unwrap();
    cfg.validate().unwrap();
    assert!(cfg.num_blocks() >= 1);
    assert!(cfg.entry_block().is_some());
    assert_eq!(cfg.num_qubits(), 2);
    assert_eq!(cfg.qubits(), vec![q(0), q(1)]);
    // 重建（矩阵等价性验证，避免对块布局的假设）
    let rebuilt = cfg.to_circuit().unwrap();
    assert!(max_delta(
        &rebuilt.to_matrix(None).unwrap(),
        &source.to_matrix(None).unwrap()
    ) < 1e-12);
    println!("[cfg] 线性线路：validate + 重建矩阵等价");
}

#[test]
fn cfg_from_circuit_control_flow_regions() {
    use cqlib_core::circuit::ClassicalExpr;
    let mut c = Circuit::new(1);
    c.if_(ClassicalExpr::bool_literal(true), |b| {
        b.x(q(0))?;
        Ok(())
    })
    .unwrap();
    c.while_(ClassicalExpr::bool_literal(false), |b| {
        b.h(q(0))?;
        Ok(())
    })
    .unwrap();
    let cfg = CircuitCFG::from_circuit(&c).unwrap();
    cfg.validate().unwrap();
    assert!(cfg.num_blocks() > 3, "if/while 应产生多个块");
    // while 头是循环头
    let loop_headers: Vec<_> = (0..cfg.num_blocks())
        .filter(|&i| cfg.is_loop_header(NodeIndex::new(i)))
        .collect();
    assert!(!loop_headers.is_empty(), "应存在循环头块");
    // 重建为线路：仍含控制流并通过 validate
    let rebuilt = cfg.to_circuit().unwrap();
    rebuilt.validate().unwrap();
    assert!(rebuilt
        .operations()
        .iter()
        .any(|op| op.instruction.is_classical_control()));
    println!("[cfg] if/while：多块 + 循环头标记 + 重建含控制流且 validate 通过");
}

#[test]
fn cfg_manual_construction_full_api() {
    let mut cfg = CircuitCFG::from_qubits(vec![q(0), q(1)]);
    let mut block0 = BasicBlock::new().with_label("entry");
    block0.push_operation(raw_h_op(q(0)));
    block0.set_terminator(Terminator::Jump(NodeIndex::new(1)));
    let mut block1 = BasicBlock::new();
    block1.push_operation(raw_x_op(q(0)));
    block1.set_terminator(Terminator::Return);
    assert!(!block0.is_empty());
    assert!(block0.has_terminator());
    assert_eq!(block0.len(), 1);
    assert_eq!(block0.label(), Some("entry"));
    assert!(block0.terminator().is_some());
    assert_eq!(block0.operations().len(), 1);

    let b0 = cfg.add_block(block0);
    let b1 = cfg.add_block(block1);
    cfg.set_entry_block(b0);
    assert_eq!(cfg.entry_block(), Some(b0));
    cfg.add_edge(b0, b1, FlowEdge::Unconditional);
    assert_eq!(cfg.num_blocks(), 2);
    // block_mut 修改
    cfg.block_mut(b1).unwrap().push_operation(raw_h_op(q(1)));
    assert_eq!(cfg.block_mut(b1).unwrap().len(), 2);
    // 出边查询
    let edges: Vec<_> = cfg.outgoing_edges(b0).collect();
    assert_eq!(edges.len(), 1);
    // 全体块遍历
    assert_eq!(cfg.blocks().count(), 2);
    // 与电路元数据
    assert_eq!(cfg.classical_vars(), &[] as &[ClassicalType]);
    assert!(cfg.classical_values().is_empty());
    cfg.validate().unwrap();
    let circuit = cfg.to_circuit().unwrap();
    assert_eq!(circuit.operations().len(), 3);
    println!("[cfg] 手工构建：块 API + 边 + 入口 + validate + 重建 3 op 全通过");
}

// ==================== 三、symbolic_matrix ====================

#[test]
fn symbolic_circuit_matrix_and_evaluate() {
    let theta = Parameter::symbol("θ");
    let mut c = Circuit::new(1);
    c.rx(q(0), theta).unwrap();
    let sym = circuit_to_symbolic_matrix(&c, None).unwrap();
    assert_eq!(sym.shape(), &[2, 2]);
    let mut bindings = HashMap::new();
    bindings.insert("θ", PI / 2.0);
    let numeric = evaluate_symbolic_matrix(&sym, &Some(bindings)).unwrap();
    // 数值参照
    let mut reference_c = Circuit::new(1);
    reference_c.rx(q(0), PI / 2.0).unwrap();
    assert!(max_delta(&numeric, &reference_c.to_matrix(None).unwrap()) < 1e-12);
    println!("[symbolic] 符号矩阵绑定 θ=π/2 与数值 RX(π/2) 一致");
}

#[test]
fn symbolic_standard_gate_matrix() {
    let sym_h = standard_gate_symbolic_matrix(StandardGate::H, &[]).unwrap();
    assert_eq!(sym_h.shape(), &[2, 2]);
    let numeric = evaluate_symbolic_matrix(&sym_h, &None).unwrap();
    let expected = StandardGate::H.matrix(&[]).unwrap().into_owned();
    assert!(max_delta(&numeric, &expected) < 1e-12);
    // 参数门
    let p = Parameter::from(0.6);
    let sym_rx = standard_gate_symbolic_matrix(StandardGate::RX, &[p]).unwrap();
    let numeric_rx = evaluate_symbolic_matrix(&sym_rx, &None).unwrap();
    let expected_rx = StandardGate::RX.matrix(&[0.6]).unwrap().into_owned();
    assert!(max_delta(&numeric_rx, &expected_rx) < 1e-12);
    // 参数不足
    assert!(standard_gate_symbolic_matrix(StandardGate::RX, &[]).is_err());
    println!("[symbolic] standard_gate_symbolic_matrix：H/RX 与数值矩阵一致，参数校验有效");
}

#[test]
fn symbolic_matrices_equivalence() {
    let x_sym = standard_gate_symbolic_matrix(StandardGate::X, &[]).unwrap();
    let z_sym = standard_gate_symbolic_matrix(StandardGate::Z, &[]).unwrap();
    // 自身
    assert!(symbolic_matrices_equivalent(&x_sym, &x_sym).unwrap());
    // X 与 -X：差全局相位 → 等价
    let neg_x = x_sym.mapv(|v: SymbolicComplex| {
        SymbolicComplex::from_complex(Complex64::new(-1.0, 0.0)) * v
    });
    assert!(symbolic_matrices_equivalent(&x_sym, &neg_x).unwrap());
    // X 与 Z 不等价
    assert!(!symbolic_matrices_equivalent(&x_sym, &z_sym).unwrap());
    // 维度不同 → false
    let h_sym = standard_gate_symbolic_matrix(StandardGate::H, &[]).unwrap();
    let cx_sym = standard_gate_symbolic_matrix(StandardGate::CX, &[]).unwrap();
    assert!(!symbolic_matrices_equivalent(&h_sym, &cx_sym).unwrap());
    println!("[symbolic] 等价判定：自等/相位等价/不等价/维度不同 全部正确");
}

#[test]
fn symbolic_circuits_equivalent_doc_example() {
    // 文档示例：X（带 π 全局相位）与 X 等价
    let mut lhs = Circuit::new(1);
    lhs.x(q(0)).unwrap();
    lhs.set_global_phase(Parameter::pi());
    let mut rhs = Circuit::new(1);
    rhs.x(q(0)).unwrap();
    assert!(circuits_equivalent(&lhs, &rhs, None).unwrap());
    // H 与 X 不等价
    let mut h = Circuit::new(1);
    h.h(q(0)).unwrap();
    assert!(!circuits_equivalent(&h, &rhs, None).unwrap());
    println!("[symbolic] circuits_equivalent：全局相位等价 = true，H vs X = false");
}

#[test]
fn symbolic_complex_scalar_api() {
    let zero = SymbolicComplex::zero();
    let one = SymbolicComplex::one();
    let i = SymbolicComplex::i();
    assert!(zero.is_zero_exact());
    assert!(one.is_one_exact());
    assert!(!one.is_zero_exact());
    assert!(zero.simplifies_to_zero().unwrap());
    // new/from_real/from_complex/exp_i
    let real = SymbolicComplex::from_real(2.0);
    let complex = SymbolicComplex::from_complex(Complex64::new(1.0, 2.0));
    let composed = SymbolicComplex::new(Parameter::symbol("a"), Parameter::from(1.0));
    let exp_phase = SymbolicComplex::exp_i(Parameter::from(PI / 2.0));
    // evaluate
    let mut bindings = HashMap::new();
    bindings.insert("a", 3.0);
    let v = composed.evaluate(&Some(bindings)).unwrap();
    assert!((v - Complex64::new(3.0, 1.0)).norm() < 1e-12);
    let v = exp_phase.evaluate(&None).unwrap();
    assert!((v - Complex64::new(0.0, 1.0)).norm() < 1e-12);
    let v = real.evaluate(&None).unwrap();
    assert!((v - Complex64::new(2.0, 0.0)).norm() < 1e-12);
    let v = complex.evaluate(&None).unwrap();
    assert!((v - Complex64::new(1.0, 2.0)).norm() < 1e-12);
    // simplify / replace
    let expr = SymbolicComplex::new(Parameter::symbol("a") * 1.0f64, Parameter::from(0.0));
    let simplified = expr.simplify().unwrap();
    assert!(
        simplified.evaluate(&None).is_err(),
        "自由符号 a 未绑定时求值应报错"
    );
    let replaced = zero.replace("x", Parameter::from(1.0));
    assert!(replaced.is_zero_exact());
    let _ = i.evaluate(&None).unwrap();
    println!("[symbolic] SymbolicComplex 标量 12 个方法全部验证");
}

#[test]
fn symbolic_eye_and_substitution() {
    let eye4 = symbolic_eye(4);
    assert_eq!(eye4.shape(), &[4, 4]);
    let numeric = evaluate_symbolic_matrix(&eye4, &None).unwrap();
    for i in 0..4 {
        for j in 0..4 {
            let expected = if i == j { Complex64::new(1.0, 0.0) } else { Complex64::new(0.0, 0.0) };
            assert!((numeric[[i, j]] - expected).norm() < 1e-12);
        }
    }
    // 替换：θ → π 后的符号矩阵数值化
    let theta = Parameter::symbol("θ");
    let mut c = Circuit::new(1);
    c.rz(q(0), theta).unwrap();
    let sym = circuit_to_symbolic_matrix(&c, None).unwrap();
    let mut replacements = HashMap::new();
    replacements.insert("θ".to_string(), Parameter::from(PI));
    let substituted = substitute_symbolic_matrix(sym, &replacements).unwrap();
    let numeric = evaluate_symbolic_matrix(&substituted, &None).unwrap();
    let mut reference = Circuit::new(1);
    reference.rz(q(0), PI).unwrap();
    assert!(max_delta(&numeric, &reference.to_matrix(None).unwrap()) < 1e-10);
    println!("[symbolic] symbolic_eye 恒等 + substitute_symbolic_matrix 代换正确");
}

// ==================== 四、Ansatz 家族 ====================

#[test]
fn ansatz_two_local_full_surface() {
    let ansatz = TwoLocal::new(3)
        .reps(2)
        .rotation_gates(vec![StandardGate::RY])
        .entanglement_gate(StandardGate::CX)
        .entanglement(EntanglementTopology::Linear);
    ansatz.validate().unwrap();
    assert_eq!(ansatz.num_qubits(), 3);
    assert_eq!(ansatz.num_parameters(), 9, "(2+1)×3");
    let circuit = ansatz.build_circuit("theta").unwrap();
    assert!(!circuit.operations().is_empty());
    assert!(circuit.uses_symbol("theta_0") || !circuit.used_symbols().is_empty());
    let names = circuit
        .operations()
        .iter()
        .map(|op| op.instruction.name())
        .collect::<std::collections::BTreeSet<_>>();
    assert!(names.contains("RY"));
    assert!(names.contains("CX"));
    // 跳过末层旋转
    let skipped = TwoLocal::new(3)
        .reps(2)
        .skip_final_rotation_layer(true)
        .rotation_gates(vec![StandardGate::RY])
        .entanglement_gate(StandardGate::CX)
        .entanglement(EntanglementTopology::Linear);
    assert_eq!(skipped.num_parameters(), 6, "2×3");
    println!("[ansatz] TwoLocal：参数数 9 / 构建含 RY+CX / skip 末层 → 6");
}

#[test]
fn ansatz_entanglement_pair_generation() {
    let linear = EntanglementTopology::Linear.generate_pairs(3);
    assert_eq!(linear, vec![(0, 1), (1, 2)]);
    let circular = EntanglementTopology::Circular.generate_pairs(3);
    assert_eq!(circular.len(), 3, "环拓扑加环绕边");
    let full = EntanglementTopology::Full.generate_pairs(3);
    assert_eq!(full.len(), 3, "C(3,2)=3");
    // k 元组
    let tuples = EntanglementTopology::Full.generate_k_tuples(2, 3);
    assert_eq!(tuples.len(), 3);
    println!("[ansatz] 线性/环形/全连接边生成 + k 元组全部正确");
}

#[test]
fn ansatz_facades_real_amplitudes_and_efficient_su2() {
    let ra = real_amplitudes(4, 3, EntanglementTopology::Linear);
    assert_eq!(ra.num_qubits(), 4);
    assert_eq!(ra.num_parameters(), 16, "(3+1)×4");
    let circuit = ra.build_circuit("θ").unwrap();
    assert!(circuit.operations().iter().any(|op| op.instruction.name() == "RY"));

    let su2 = efficient_su2(3, 2, EntanglementTopology::Full);
    assert_eq!(su2.num_parameters(), 18, "3×2×(2+1)");
    let circuit = su2.build_circuit("φ").unwrap();
    let names: std::collections::BTreeSet<String> = circuit
        .operations()
        .iter()
        .map(|op| op.instruction.name())
        .collect();
    assert!(names.contains("RY") && names.contains("RZ") && names.contains("CX"));
    println!("[ansatz] 门面：RealAmplitudes 16 参数、EfficientSU2 18 参数且含 RY/RZ/CX");
}

#[test]
fn ansatz_feature_maps() {
    // ZZFeatureMap 门面
    let zz = zz_feature_map(3, 2, EntanglementTopology::Full);
    assert_eq!(zz.num_parameters(), 3);
    zz.validate().unwrap();
    let built = zz.build_circuit("x").unwrap();
    assert!(!built.operations().is_empty());
    // 直接构造各特征映射
    let basis = BasisEncoding::new(vec![true, false]);
    assert_eq!(basis.num_qubits(), 2);
    let built = basis.build_circuit("b").unwrap();
    assert!(built
        .operations()
        .iter()
        .any(|op| op.instruction.name() == "X"), "bit=1 的比特应有 X");
    let angle = AngleEncoding::new(2, StandardGate::RX);
    assert_eq!(angle.num_qubits(), 2);
    assert_eq!(angle.num_parameters(), 2);
    angle.build_circuit("a").unwrap();
    let z = ZFeatureMap::new(3).reps(2);
    z.build_circuit("z").unwrap();
    let iqp = IQPFeatureMap::new(3).reps(1).entanglement(EntanglementTopology::Linear);
    iqp.build_circuit("i").unwrap();
    let zz_direct = ZZFeatureMap::new(3).reps(2).entanglement(EntanglementTopology::Full);
    zz_direct.build_circuit("d").unwrap();
    println!("[ansatz] 特征映射：ZZ 门面 3 参数 + Basis/Angle/Z/IQP/ZZ 直构全部可构建");
}

#[test]
fn ansatz_layers() {
    let basic = BasicEntanglerLayers::new(3)
        .reps(2)
        .rotation_gate(StandardGate::RY)
        .entanglement_gate(StandardGate::CX);
    assert_eq!(basic.num_qubits(), 3);
    basic.validate().unwrap();
    let built = basic.build_circuit("l").unwrap();
    assert!(!built.operations().is_empty());

    let strong = StronglyEntanglingLayers::new(3)
        .reps(2)
        .entanglement_gate(StandardGate::CX)
        .ranges(vec![1, 2]);
    strong.validate().unwrap();
    strong.build_circuit("s").unwrap();
    println!("[ansatz] 层模板：BasicEntangler 与 StronglyEntangling 构建成功");
}

#[test]
fn ansatz_qaoa_basic_and_validation() {
    let cost = Hamiltonian::from_pauli("ZZ".parse::<PauliString>().unwrap());
    let qaoa = QAOAAnsatz::new(cost).unwrap();
    assert_eq!(qaoa.num_qubits(), 2);
    qaoa.validate().unwrap();
    let default_params = qaoa.num_parameters();
    assert_eq!(default_params, 2, "默认 reps=1 → 2 参数（γ, β）");
    let qaoa2 = qaoa.reps(2);
    assert_eq!(qaoa2.num_parameters(), 4);
    let circuit = qaoa2.build_circuit("p").unwrap();
    assert_eq!(circuit.num_qubits(), 2);
    assert!(!circuit.operations().is_empty());
    println!("[ansatz] QAOA：默认 2 参数、reps=2 → 4，构建成功");
}

#[test]
fn ansatz_qaoa_mixer_and_initial_state() {
    let cost = Hamiltonian::from_pauli("ZZ".parse::<PauliString>().unwrap());
    // 宽度不匹配 mixer → QubitCountMismatch
    let wide_mixer = Hamiltonian::from_pauli("ZZI".parse::<PauliString>().unwrap());
    let err = QAOAAnsatz::new(cost.clone())
        .unwrap()
        .mixer(wide_mixer)
        .unwrap_err();
    assert!(matches!(err, CircuitError::QubitCountMismatch { expected: 2, actual: 3 }));
    // 合法 mixer
    let mixer = Hamiltonian::from_pauli("XX".parse::<PauliString>().unwrap());
    let qaoa = QAOAAnsatz::new(cost.clone()).unwrap().mixer(mixer).unwrap();
    qaoa.validate().unwrap();
    // 初态宽度校验
    let wrong_state = Circuit::new(3);
    let err = QAOAAnsatz::new(cost.clone())
        .unwrap()
        .initial_state(wrong_state)
        .unwrap_err();
    assert!(matches!(err, CircuitError::QubitCountMismatch { expected: 2, actual: 3 }));
    let right_state = Circuit::new(2);
    let qaoa = QAOAAnsatz::new(cost).unwrap().initial_state(right_state).unwrap();
    qaoa.validate().unwrap();
    println!("[ansatz] QAOA：mixer/initial_state 宽度校验拒绝 + 合法配置通过");
}

#[test]
fn ansatz_pauli_evolution_exact_and_trotterized() {
    // 单项 → 精确
    let zz = Hamiltonian::from_pauli("ZZ".parse::<PauliString>().unwrap());
    let evolution = PauliEvolutionAnsatz::new(zz).unwrap();
    let info = evolution.evolution_info();
    assert!(info.is_exact, "单一 ZZ 项应精确");
    evolution.validate().unwrap();
    let built = evolution.build_circuit("t").unwrap();
    assert!(!built.operations().is_empty());
    // 时间参数名
    let named = evolution.clone().with_time_param_name("time");
    let built_named = named.build_circuit("ev").unwrap();
    assert!(built_named.operations().len() >= built.operations().len());
    // 互不对易项 → 只能 Trotter 化
    let mixed = Hamiltonian::from_pauli("XX".parse::<PauliString>().unwrap())
        + Hamiltonian::from_pauli("ZI".parse::<PauliString>().unwrap());
    let trotter = PauliEvolutionAnsatz::new(mixed).unwrap();
    let info = trotter.evolution_info();
    assert!(!info.is_exact, "XX 与 ZI 不对易 → 非精确");
    println!("[ansatz] Pauli 演化：单项精确、非对易 Trotter 化、with_time_param_name 生效");
}

// ==================== 五、可靠性（跨组件） ====================

#[test]
fn reliability_dag_roundtrip_large_circuit() {
    // 40 操作参数化线路经 DAG 往返
    let mut c = Circuit::new(3);
    for i in 0..40u32 {
        match i % 4 {
            0 => c.h(q(i % 3)).unwrap(),
            1 => c.cx(q(i % 3), q((i + 1) % 3)).unwrap(),
            2 => c.rz(q(i % 3), 0.1 * i as f64).unwrap(),
            _ => c.rxx(q(i % 3), q((i + 2) % 3), Parameter::symbol("θ")).unwrap(),
        }
    }
    let dag = CircuitDag::from_circuit(&c).unwrap();
    assert_eq!(dag.num_ops(), 40);
    dag.validate().unwrap();
    let rebuilt = dag.to_circuit().unwrap();
    assert_eq!(rebuilt, c);
    rebuilt.validate().unwrap();
    // 数值一致性（绑定 θ）
    let mut bindings = HashMap::new();
    bindings.insert("θ", 0.5);
    let a = c.assign_parameters(&Some(bindings.clone())).unwrap();
    let b = rebuilt.assign_parameters(&Some(bindings)).unwrap();
    assert!(max_delta(&a.to_matrix(None).unwrap(), &b.to_matrix(None).unwrap()) < 1e-12);
    println!("[reliability] 40 op 线路：DAG 往返结构相等 + 绑定后矩阵一致");
}

#[test]
fn reliability_cfg_dag_matrix_agreement() {
    // 同一线路的三种视角：Circuit 矩阵 / DAG 重建 / CFG 重建 必须一致
    let source = bell();
    let matrix = source.to_matrix(None).unwrap();
    let dag_built = CircuitDag::from_circuit(&source)
        .unwrap()
        .to_circuit()
        .unwrap()
        .to_matrix(None)
        .unwrap();
    let cfg_built = CircuitCFG::from_circuit(&source)
        .unwrap()
        .to_circuit()
        .unwrap()
        .to_matrix(None)
        .unwrap();
    assert!(max_delta(&matrix, &dag_built) < 1e-12);
    assert!(max_delta(&matrix, &cfg_built) < 1e-12);
    println!("[reliability] Circuit/DAG/CFG 三视角重建矩阵两两一致");
}

#[test]
fn reliability_frozen_circuit_arc_sharing() {
    // FrozenCircuit 的 Arc 共享：克隆不复制内部线路
    let mut inner = Circuit::new(2);
    inner.h(q(0)).unwrap();
    let frozen = FrozenCircuit::new(inner);
    let arc: Arc<FrozenCircuit> = Arc::new(frozen);
    let cloned = arc.clone();
    assert!(Arc::strong_count(&cloned) >= 2);
    assert_eq!(arc.circuit().num_qubits(), 2);
    println!("[reliability] FrozenCircuit Arc 共享验证通过");
}
