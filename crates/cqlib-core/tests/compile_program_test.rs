//! compile（编译流水线）模块的测试程序
//!
//! 被测目标：`crates/cqlib-core/src/compile/`（compile / CompileConfig / CompileTarget /
//!           CompilerWorkflow / 13 阶段流水线 / SABRE 路由 / Pareto 束选择）
//! 测试方式：integration test —— 仅通过 `cqlib_core` 公开 API 访问实现。
//!
//! 覆盖内容：
//! 1. Logical 目标契约（步骤报告、酉等价、空线路）
//! 2. 逻辑优化（冗余门消除）
//! 3. Basis 目标（显式基翻译、输出门集合封闭）
//! 4. Device 目标（SABRE 路由插 SWAP、原生能力集、设备终校验）
//! 5. TopologyBasis 目标（仅借拓扑 + 显式基）
//! 6. Enhanced + Device（SABRE Pareto 束选择、种子确定性）
//! 7. 指定初始布局
//! 8. compile 与 compile_owned 一致性
//! 9. 边界与错误路径（空基、容量超限、原生能力不足）
//!
//! 运行命令：cargo test -p cqlib-core --test compile_program_test -- --nocapture

use cqlib_core::circuit::{Circuit, Instruction, Qubit, StandardGate};
use cqlib_core::compile::resource::ResourcePolicy;
use cqlib_core::compile::{
    compile, compile_owned, CompileConfig, CompileMode, CompileResult, CompileTarget,
    CompilerError, DeviceCompileTarget,
};
use cqlib_core::device::{Device, Layout};

/// 逻辑配置（Normal + Logical）
fn logical_config(mode: CompileMode) -> CompileConfig {
    CompileConfig {
        mode,
        target: CompileTarget::Logical,
        resource_policy: ResourcePolicy::default(),
    }
}

/// Bell 态线路（H + CX）。
fn bell_circuit() -> Circuit {
    let mut circuit = Circuit::new(2);
    circuit.h(Qubit::new(0)).expect("append H");
    circuit.cx(Qubit::new(0), Qubit::new(1)).expect("append CX");
    circuit
}

/// 两条线路酉矩阵的最大逐元素偏差。
fn max_unitary_delta(a: &Circuit, b: &Circuit) -> f64 {
    let ma = a.to_matrix(None).expect("matrix a");
    let mb = b.to_matrix(None).expect("matrix b");
    let mut max = 0.0f64;
    for (x, y) in ma.iter().zip(mb.iter()) {
        let z = *x - *y;
        max = max.max(z.re.hypot(z.im));
    }
    max
}

/// 从 route.sabre 的 reason 中解析插入的 SWAP 数（“inserted N swap operations…”）。
fn swap_count_from(result: &CompileResult) -> Option<usize> {
    let step = result.steps.iter().find(|s| s.name == "route.sabre")?;
    let reason = step.reason.as_deref()?;
    let idx = reason.find("inserted ")? + "inserted ".len();
    let digits: String = reason[idx..]
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

/// 断言输出线路只包含给定标准门集合（Standard），返回实际门名列表。
fn assert_only_gates(result: &CompileResult, allowed: &[StandardGate]) -> Vec<String> {
    let mut names = Vec::new();
    for op in result.circuit.operations() {
        match &op.instruction {
            Instruction::Standard(gate) => {
                assert!(
                    allowed.contains(gate),
                    "输出含越基门 {gate:?}（允许 {allowed:?}）"
                );
                names.push(format!("{gate:?}"));
            }
            other => panic!("输出含非标准指令 {other:?}"),
        }
    }
    names
}

#[test]
fn compile_logical_contract() {
    let circuit = bell_circuit();
    let result = compile(&circuit, logical_config(CompileMode::Normal)).expect("compile");

    assert_eq!(result.mode, CompileMode::Normal);
    assert!(!result.steps.is_empty());
    assert!(result.step("resolve.target").is_some(), "应有 resolve.target 步骤");
    let routing = result.step("route.sabre").expect("应有 route.sabre 步骤");
    assert!(routing.skipped, "Logical 目标应跳过路由");
    assert_eq!(result.circuit.qubits().len(), 2);

    // Logical 编译保持酉矩阵不变
    let delta = max_unitary_delta(&result.circuit, &circuit);
    println!(
        "[compile] Logical: steps={} changed={} 酉偏差={delta:.2e}",
        result.steps.len(),
        result.changed
    );
    assert!(delta < 1e-10, "Logical 编译应保持酉矩阵（实际偏差 {delta}）");
}

#[test]
fn compile_empty_circuit_ok() {
    // 空线路：Enhanced + Logical 也能成功编译，路由跳过
    let result = compile(&Circuit::new(1), logical_config(CompileMode::Enhanced))
        .expect("空线路应可编译");
    assert_eq!(result.circuit.operations().len(), 0);
    assert!(result.step("route.sabre").expect("route.sabre").skipped);
    println!(
        "[compile] 空线路：steps={} changed={}",
        result.steps.len(),
        result.changed
    );
}

#[test]
fn compile_cancels_redundant_ops() {
    // h·h ≡ I，cx·cx ≡ I：整条线路为恒等，优化器应显著削减操作数
    let mut circuit = Circuit::new(2);
    circuit.h(Qubit::new(0)).unwrap();
    circuit.h(Qubit::new(0)).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    let ops_in = circuit.operations().len();

    let result = compile(&circuit, logical_config(CompileMode::Normal)).expect("compile");
    let ops_out = result.circuit.operations().len();

    println!(
        "[compile] 冗余门消除：ops {ops_in} → {ops_out}，changed={}（cancel.changed={}）",
        result.changed,
        result.step_changed("optimize.commutative_cancellation")
    );
    assert!(ops_out < ops_in, "操作数应减少（实际 {ops_in} → {ops_out}）");
    assert!(result.changed, "至少一个步骤应产生变更");
    // 语义不变：仍为恒等
    let identity = Circuit::new(2);
    let delta = max_unitary_delta(&result.circuit, &identity);
    assert!(delta < 1e-10, "优化后应为恒等（偏差 {delta}）");
}

#[test]
fn compile_to_explicit_basis() {
    // Bell 线路降到显式基 {H, CZ}：CX = H(t)·CZ·H(t)
    let circuit = bell_circuit();
    let config = CompileConfig {
        mode: CompileMode::Normal,
        target: CompileTarget::Basis(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::CZ),
        ]),
        resource_policy: ResourcePolicy::default(),
    };
    let result = compile(&circuit, config).expect("compile to basis");

    assert!(
        result.step_changed("translate.target_basis"),
        "translate.target_basis 应产生变更"
    );
    let gates = assert_only_gates(&result, &[StandardGate::H, StandardGate::CZ]);
    let delta = max_unitary_delta(&result.circuit, &circuit);
    println!(
        "[compile] Basis[H,CZ]：输出门 {gates:?}（{} 个），酉偏差 {delta:.2e}",
        gates.len()
    );
    assert!(delta < 1e-10, "基翻译应保持酉矩阵（偏差 {delta}）");
}

#[test]
fn compile_device_routing_inserts_swaps() {
    let mut circuit = Circuit::new(3);
    circuit.cx(Qubit::new(0), Qubit::new(2)).unwrap();

    let make_device = || {
        Device::bidirectional_line("line-3", 3)
            .unwrap()
            .with_native_gates(vec![Instruction::Standard(StandardGate::CX)])
            .unwrap()
    };

    // 情形 A：自由布局 —— SABRE 可用布局搜索把远距离交互直接映射到相邻物理对，
    // 单门线路完全不需要 SWAP（这是正确行为，而非缺陷）
    let config_a = CompileConfig {
        mode: CompileMode::Normal,
        target: CompileTarget::Device(DeviceCompileTarget {
            device: make_device(),
            initial_layout: None,
            seed: Some(7),
        }),
        resource_policy: ResourcePolicy::default(),
    };
    let result_a = compile(&circuit, config_a).expect("device compile (free layout)");
    let routing_a = result_a.step("route.sabre").expect("route.sabre");
    assert!(!routing_a.skipped, "Device 目标必须路由");
    let swaps_a = swap_count_from(&result_a).expect("reason 应含 SWAP 数");
    println!(
        "[compile] A 自由布局：{}（SWAP={}，布局搜索可规避）",
        routing_a.reason.as_deref().unwrap_or("<无 reason>"),
        swaps_a
    );
    assert_eq!(result_a.circuit.qubits().len(), 3);
    let gates_a = assert_only_gates(&result_a, &[StandardGate::CX]);
    println!("[compile] A 输出门统计：{gates_a:?}");
    let metadata_a = result_a.device_metadata.as_ref().expect("device_metadata");
    println!(
        "[compile] A initial_layout={:?} final_layout={:?}",
        metadata_a.initial_layout, metadata_a.final_layout
    );
    make_device().validate_circuit(&result_a.circuit).expect("A 输出必须被设备接受");

    // 情形 B：固定恒等布局 —— 物理 0 与 2 在线形拓扑上不相邻，必须插入 SWAP
    let layout = Layout::from_pairs(&[(0, 0), (1, 1), (2, 2)], 3).expect("identity layout");
    let config_b = CompileConfig {
        mode: CompileMode::Normal,
        target: CompileTarget::Device(DeviceCompileTarget {
            device: make_device(),
            initial_layout: Some(layout),
            seed: Some(7),
        }),
        resource_policy: ResourcePolicy::default(),
    };
    let result_b = compile(&circuit, config_b).expect("device compile (pinned layout)");
    let routing_b = result_b.step("route.sabre").expect("route.sabre");
    let swaps_b = swap_count_from(&result_b).expect("reason 应含 SWAP 数");
    println!(
        "[compile] B 固定布局：{}（SWAP={}）",
        routing_b.reason.as_deref().unwrap_or("<无 reason>"),
        swaps_b
    );
    assert!(swaps_b > 0, "固定布局下远距离交互必须插入 SWAP（实际 {swaps_b}）");

    let gates_b = assert_only_gates(&result_b, &[StandardGate::CX]);
    println!("[compile] B 输出门统计：{gates_b:?}");
    let metadata_b = result_b.device_metadata.as_ref().expect("device_metadata");
    println!(
        "[compile] B initial_layout={:?} final_layout={:?}",
        metadata_b.initial_layout, metadata_b.final_layout
    );
    make_device().validate_circuit(&result_b.circuit).expect("B 输出必须被设备接受");
}

#[test]
fn compile_device_respects_native_gate_set() {
    // Bell 线路在原生能力 {H, CX} 的 2 比特设备上
    let circuit = bell_circuit();
    let device = Device::bidirectional_line("line-2", 2)
        .unwrap()
        .with_native_gates(vec![
            Instruction::Standard(StandardGate::H),
            Instruction::Standard(StandardGate::CX),
        ])
        .unwrap();
    let config = CompileConfig {
        mode: CompileMode::Normal,
        target: CompileTarget::Device(DeviceCompileTarget {
            device,
            initial_layout: None,
            seed: None,
        }),
        resource_policy: ResourcePolicy::default(),
    };
    let result = compile(&circuit, config).expect("device compile");
    let gates = assert_only_gates(&result, &[StandardGate::H, StandardGate::CX]);
    println!("[compile] 原生 {{H,CX}} 设备输出门：{gates:?}");
}

#[test]
fn compile_enhanced_pareto_beam_selection() {
    // Enhanced + Device：应出现 select.sabre_pareto_beam 步骤；同 seed 结果确定
    let mut circuit = Circuit::new(3);
    circuit.cx(Qubit::new(0), Qubit::new(2)).unwrap();

    let make_config = || {
        let device = Device::bidirectional_line("line-3", 3)
            .unwrap()
            .with_native_gates(vec![Instruction::Standard(StandardGate::CX)])
            .unwrap();
        CompileConfig {
            mode: CompileMode::Enhanced,
            target: CompileTarget::Device(DeviceCompileTarget {
                device,
                initial_layout: None,
                seed: Some(7),
            }),
            resource_policy: ResourcePolicy::default(),
        }
    };

    let result1 = compile(&circuit, make_config()).expect("enhanced compile");
    assert_eq!(result1.mode, CompileMode::Enhanced);

    let selection = result1
        .step("select.sabre_pareto_beam")
        .expect("Enhanced+Device 应有 Pareto 束选择步骤");
    assert!(!selection.skipped);
    println!(
        "[compile] Pareto 束：{}",
        selection.reason.as_deref().unwrap_or("<无 reason>")
    );

    // 种子确定性：同 seed 两次编译，输出线路一致
    let result2 = compile(&circuit, make_config()).expect("enhanced compile #2");
    let same = result1.circuit == result2.circuit;
    println!(
        "[compile] 同 seed 复跑一致性：circuits_equal={same}（SWAP {}/{}）",
        swap_count_from(&result1).unwrap_or(0),
        swap_count_from(&result2).unwrap_or(0)
    );
    assert!(same, "同 seed 的 Enhanced 编译应产生相同输出线路");
}

#[test]
fn compile_topology_basis_target() {
    // TopologyBasis：借 line-3 拓扑路由，但输出基为显式 {H, CZ}
    let mut circuit = Circuit::new(3);
    circuit.h(Qubit::new(0)).unwrap();
    circuit.cx(Qubit::new(0), Qubit::new(2)).unwrap();

    let device = Device::bidirectional_line("line-3", 3).unwrap();
    let config = CompileConfig {
        mode: CompileMode::Normal,
        target: CompileTarget::TopologyBasis {
            device_target: DeviceCompileTarget {
                device,
                initial_layout: None,
                seed: Some(3),
            },
            basis: vec![
                Instruction::Standard(StandardGate::H),
                Instruction::Standard(StandardGate::CZ),
            ],
        },
        resource_policy: ResourcePolicy::default(),
    };
    let result = compile(&circuit, config).expect("topology-basis compile");

    let routing = result.step("route.sabre").expect("route.sabre");
    assert!(!routing.skipped, "TopologyBasis 必须路由");
    let gates = assert_only_gates(&result, &[StandardGate::H, StandardGate::CZ]);
    assert!(result.device_metadata.is_some(), "应有设备布局元数据");
    println!(
        "[compile] TopologyBasis：路由非跳过，输出门 {gates:?}（{} 个）",
        gates.len()
    );
}

#[test]
fn compile_with_supplied_initial_layout() {
    // 指定初始布局：逻辑 0 → 物理 2（3 比特线形设备）
    let mut circuit = Circuit::new(1);
    circuit.h(Qubit::new(0)).unwrap();
    let layout = Layout::from_pairs(&[(0, 2)], 3).expect("layout");

    let device = Device::line("line-3", 3)
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(StandardGate::H)])
        .unwrap();
    let config = CompileConfig {
        mode: CompileMode::Normal,
        target: CompileTarget::Device(DeviceCompileTarget {
            device,
            initial_layout: Some(layout),
            seed: Some(11),
        }),
        resource_policy: ResourcePolicy::default(),
    };
    let result = compile(&circuit, config).expect("compile with layout");

    assert_eq!(
        result.circuit.operations()[0].qubits.as_slice(),
        &[Qubit::new(2)],
        "输出应作用在指定物理比特 2 上"
    );
    println!(
        "[compile] 指定布局：首操作比特 = {:?}",
        result.circuit.operations()[0].qubits
    );
}

#[test]
fn compile_owned_matches_borrowed() {
    let circuit = bell_circuit();
    let borrowed = compile(&circuit, logical_config(CompileMode::Normal)).expect("borrowed");
    let owned = compile_owned(circuit, logical_config(CompileMode::Normal)).expect("owned");
    assert_eq!(borrowed.circuit, owned.circuit, "compile 与 compile_owned 输出应一致");
    assert_eq!(borrowed.changed, owned.changed);
    println!(
        "[compile] compile_owned 一致性：ops={} changed={}",
        owned.circuit.operations().len(),
        owned.changed
    );
}

#[test]
fn compile_boundary_and_error_paths() {
    // E02：空目标基 → 必须报错（不能静默通过）
    let err = compile(
        &bell_circuit(),
        CompileConfig {
            mode: CompileMode::Normal,
            target: CompileTarget::Basis(vec![]),
            resource_policy: ResourcePolicy::default(),
        },
    )
    .expect_err("空 basis 必须失败");
    println!("[compile][E02] 空 basis → Err: {err}");
    assert!(
        matches!(err, CompilerError::InvalidInput(_)),
        "空 basis 应为 InvalidInput，实际 {err:?}"
    );

    // E03：线路宽于设备（4 逻辑比特 vs 2 物理比特）→ 必须报错
    let mut wide = Circuit::new(4);
    wide.cx(Qubit::new(0), Qubit::new(3)).unwrap();
    let device = Device::bidirectional_line("line-2", 2)
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(StandardGate::CX)])
        .unwrap();
    let err = compile(
        &wide,
        CompileConfig {
            mode: CompileMode::Normal,
            target: CompileTarget::Device(DeviceCompileTarget {
                device,
                initial_layout: None,
                seed: None,
            }),
            resource_policy: ResourcePolicy::default(),
        },
    )
    .expect_err("容量超限必须失败");
    println!("[compile][E03] 4 比特线路 @ 2 比特设备 → Err: {err}");

    // E04：设备原生能力不足（只有 H，线路需要 CX）→ 必须报错
    let device = Device::bidirectional_line("line-2", 2)
        .unwrap()
        .with_native_gates(vec![Instruction::Standard(StandardGate::H)])
        .unwrap();
    let err = compile(
        &bell_circuit(),
        CompileConfig {
            mode: CompileMode::Normal,
            target: CompileTarget::Device(DeviceCompileTarget {
                device,
                initial_layout: None,
                seed: None,
            }),
            resource_policy: ResourcePolicy::default(),
        },
    )
    .expect_err("原生能力不足必须失败");
    println!("[compile][E04] CX 线路 @ 仅 H 设备 → Err: {err}");
}