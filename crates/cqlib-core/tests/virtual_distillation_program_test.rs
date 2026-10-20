//! Virtual Distillation（虚拟蒸馏）错误缓解模块的测试程序
//!
//! 被测目标：`crates/cqlib-core/src/error_mitigation/virtual_distillation.rs`
//! 测试方式：integration test —— 仅通过 `cqlib_core` 公开 API 访问实现。
//!
//! 覆盖内容：
//! 1. 构造校验（copies ≥ 2）与访问器
//! 2. 拷贝-交换线路结构（宽度、操作数、CSWAP 阶梯、辅助比特 H）
//! 3. 纯态精确性：Tr(Oρ^M)/Tr(ρ^M) 在纯态下恒等于 ⟨O⟩（M 取 2、3）
//! 4. 去极化噪声下 M 拷贝的误差压缩（含解析数值核对）
//! 5. 矩比与方差公式（run_vd 内部 mitigate_from_statistics）
//! 6. 错误路径（宽度不匹配 / 分母均值 0 / 控制流线路）
//! 7. shots 透传
//!
//! 运行命令：cargo test -p cqlib-core --test virtual_distillation_program_test -- --nocapture

use std::cell::RefCell;

use num_complex::Complex64;

use cqlib_core::circuit::{Circuit, CircuitError, Instruction, Qubit, StandardGate};
use cqlib_core::error_mitigation::{ErrorMitigationError, VirtualDistillation};
use cqlib_core::qis::{DensityMatrix, Hamiltonian, Pauli, PauliString};

/// Bell 态线路 |00⟩+|11⟩（H + CX），2 比特、2 个操作。
fn bell_circuit() -> Circuit {
    let mut circuit = Circuit::new(2);
    circuit.h(Qubit::new(0)).expect("append H");
    circuit.cx(Qubit::new(0), Qubit::new(1)).expect("append CX");
    circuit
}

/// 哈密顿量 H = ZZ（Bell 态上期望值恰为 +1）。
fn zz_hamiltonian() -> Hamiltonian {
    Hamiltonian::from_pauli("ZZ".parse::<PauliString>().expect("parse ZZ pauli string"))
}

/// ⟨ψ|H|ψ⟩：把线路矩阵作用于 |0…0⟩（取矩阵第一列），确定性计算、无统计涨落。
fn ideal_expectation(circuit: &Circuit, hamiltonian: &Hamiltonian) -> f64 {
    let circuit_matrix = circuit.to_matrix(None).expect("circuit unitary matrix");
    let h_matrix = hamiltonian.to_matrix().expect("hamiltonian dense matrix");
    let psi = circuit_matrix.column(0);
    let dim = circuit_matrix.nrows();
    let mut acc = 0.0f64;
    for i in 0..dim {
        for j in 0..dim {
            acc += (psi[i].conj() * h_matrix[(i, j)] * psi[j]).re;
        }
    }
    acc
}

/// 去极化信道估计器：按线路顺序逐步仿真，并在辅助比特 Hadamard 之前对
/// 除辅助比特外的所有寄存器施加去极化（Kraus），模拟每份拷贝的制备噪声。
///
/// 去极化参数 p 对应的 Kraus 算子：
///   k0 = sqrt(1-p) I, k1 = sqrt(p/3) X, k2 = sqrt(p/3) Y, k3 = sqrt(p/3) Z
fn depolarizing_estimator(
    p: f64,
) -> impl Fn(&Circuit, Option<&Hamiltonian>, Option<usize>) -> (f64, f64) {
    move |circuit, hamiltonian, _shots| {
        let num_qubits = circuit.num_qubits();
        let mut state = DensityMatrix::new(num_qubits);

        let keep = (1.0 - p).sqrt();
        let flip = (p / 3.0).sqrt();
        let k0 = vec![
            Complex64::new(keep, 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(keep, 0.0),
        ];
        let k1 = vec![
            Complex64::new(0.0, 0.0),
            Complex64::new(flip, 0.0),
            Complex64::new(flip, 0.0),
            Complex64::new(0.0, 0.0),
        ];
        let k2 = vec![
            Complex64::new(0.0, 0.0),
            Complex64::new(0.0, -flip),
            Complex64::new(0.0, flip),
            Complex64::new(0.0, 0.0),
        ];
        let k3 = vec![
            Complex64::new(flip, 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(-flip, 0.0),
        ];

        // 辅助比特上的 Hadamard 将“制备阶段”与“排列阶段”分开。
        let derangement_start = circuit
            .operations()
            .iter()
            .position(|op| matches!(op.instruction, Instruction::Standard(StandardGate::H)))
            .expect("VD circuit must contain the ancillary Hadamard");

        let mut noise_injected = false;
        for (index, op) in circuit.operations().iter().enumerate() {
            if !noise_injected && index == derangement_start {
                for qubit in 0..(num_qubits - 1) {
                    state
                        .apply_kraus(&[k0.clone(), k1.clone(), k2.clone(), k3.clone()], &[qubit])
                        .unwrap();
                }
                noise_injected = true;
            }

            let qubit_indices: Vec<usize> = op.qubits.iter().map(|q| q.id() as usize).collect();
            let params: Vec<f64> = op
                .params
                .iter()
                .map(|param| match param {
                    cqlib_core::circuit::CircuitParam::Fixed(value) => *value,
                    _ => panic!("test circuits use fixed parameters only"),
                })
                .collect();
            match &op.instruction {
                Instruction::Standard(gate) => {
                    state
                        .apply_standard_gate(*gate, &qubit_indices, &params)
                        .unwrap();
                }
                Instruction::McGate(mc_gate) => {
                    let matrix = mc_gate.matrix(&params).unwrap();
                    state.apply_unitary_gate(&qubit_indices, &matrix).unwrap();
                }
                instruction => panic!("unexpected instruction in VD circuit: {instruction:?}"),
            }
        }

        let hamiltonian = hamiltonian.expect("numerator and denominator both provide a Hamiltonian");
        (state.expectation(hamiltonian).unwrap(), 0.0)
    }
}

/// 单拷贝（M=1）去极化后 ⟨Z⟩ 的解析值：Tr(Z ρ)/Tr(ρ)。
/// 去极化 Kraus 信道（I/X/Y/Z）作用在 |1⟩ 上得到 ρ' = λ1|1⟩⟨1| + λ2|0⟩⟨0|，
/// 其中 λ1 = 1-2p/3（|1⟩ 方向）、λ2 = 2p/3（|0⟩ 方向，X 与 Y 两支贡献）。
fn raw_depolarized_expectation(p: f64) -> f64 {
    let lambda1 = 1.0 - 2.0 * p / 3.0;
    let lambda2 = 2.0 * p / 3.0;
    (lambda1 * (-1.0) + lambda2 * (1.0)) / (lambda1 + lambda2)
}

#[test]
fn vd_rejects_invalid_copies() {
    // copies < 2 必须被拒绝
    let err = VirtualDistillation::new(Circuit::new(1), 1).expect_err("copies=1");
    println!("[1] VirtualDistillation::new(copies=1) -> Err: {err}");
    assert!(matches!(err, ErrorMitigationError::InvalidCopies(1)));

    let err = VirtualDistillation::new(Circuit::new(1), 0).expect_err("copies=0");
    assert!(matches!(err, ErrorMitigationError::InvalidCopies(0)));

    // set_copies 同样校验
    let mut vd = VirtualDistillation::new(Circuit::new(1), 2).unwrap();
    assert_eq!(vd.copies(), 2);
    let err = vd.set_copies(1).expect_err("set_copies=1");
    assert!(matches!(err, ErrorMitigationError::InvalidCopies(1)));
    assert_eq!(vd.copies(), 2, "失败的 set_copies 不改变状态");

    vd.set_copies(3).expect("set_copies=3");
    println!("[vd] copies 校验通过：new(2) ok → set(1) Err → set(3) ok, copies={}", vd.copies());
}

#[test]
fn vd_copy_swap_circuit_structure() {
    // 1) Bell 基线路（2 qubits, 2 ops），M=2
    let vd = VirtualDistillation::new(bell_circuit(), 2).unwrap();
    let circuit = vd.build_copy_swap_circuit().unwrap();
    let cs_count = circuit
        .operations()
        .iter()
        .filter(|op| matches!(op.instruction, Instruction::McGate(_)))
        .count();
    println!(
        "[vd] Bell×2: width={} ops={} cs={} (期望 width=5, ops=7, cs=2)",
        circuit.num_qubits(),
        circuit.operations().len(),
        cs_count
    );
    assert_eq!(circuit.num_qubits(), 5);
    assert_eq!(circuit.operations().len(), 7);
    assert_eq!(cs_count, 2);
    circuit.validate().expect("Bell×2 结构合法");

    // 2) Bell 基线路，M=3
    let vd = VirtualDistillation::new(bell_circuit(), 3).unwrap();
    let circuit = vd.build_copy_swap_circuit().unwrap();
    assert_eq!(circuit.num_qubits(), 7);
    assert_eq!(circuit.operations().len(), 11);
    circuit.validate().expect("Bell×3 结构合法");
    println!("Bell×3: width={} ops={} (期望 width=7, ops=11)", circuit.num_qubits(), circuit.operations().len());

    // 3) 单比特 X 基线路，M=2
    let mut base = Circuit::new(1);
    base.x(Qubit::new(0)).unwrap();
    let vd = VirtualDistillation::new(base, 2).unwrap();
    let circuit = vd.build_copy_swap_circuit().unwrap();
    assert_eq!(circuit.num_qubits(), 3);
    assert_eq!(circuit.operations().len(), 4);
    circuit.validate().expect("X×2 结构合法");
    println!("[vd] X×2: width={} ops={} (期望 width=3, ops=4)", circuit.num_qubits(), circuit.operations().len());

    // CSWAP 作用比特：控制=辅助比特，目标=相邻拷贝
    let ops = circuit.operations();
    match &ops[3].instruction {
        Instruction::McGate(mc) => {
            assert_eq!(mc.num_ctrl_qubits(), 1);
            assert_eq!(*mc.base_gate(), StandardGate::SWAP);
            assert_eq!(ops[3].qubits.as_slice(), &[Qubit::new(2), Qubit::new(0), Qubit::new(1)]);
            println!("[vd] CSWAP 结构 = control {}(ancilla), targets [{}, {}] ✓",
                     ops[3].qubits[0].id(), ops[3].qubits[1].id(), ops[3].qubits[2].id());
        }
        other => panic!("expected CSWAP, got {other:?}"),
    }
}

#[test]
fn vd_pure_state_exact_expectation() {
    // 纯态下 ρ^M = ρ，Tr(Oρ^M)/Tr(ρ^M) = ⟨O⟩（与 M 无关）
    for copies in [2usize, 3usize] {
        let base = bell_circuit();
        let hamiltonian = zz_hamiltonian();
        let vd = VirtualDistillation::new(base, copies).unwrap();

        let est = |c: &Circuit, h: Option<&Hamiltonian>, _s: Option<usize>| {
            (ideal_expectation(c, h.expect("vd estimator must receive a Hamiltonian")), 0.0)
        };

        let (mu_num, _var_num) = vd.run_numerator_circuit(&hamiltonian, 0, &est).unwrap();
        let (mu_den, _var_den) = vd.run_denominator_circuit(0, &est).unwrap();
        let (mu, var) = vd.run_vd(&hamiltonian, 0, 0, &est).unwrap();

        println!("[vd] 纯态 Bell + ZZ, M={copies}: numerator={mu_num:.12} denominator={mu_den:.12} vd={mu:.12} var={var:.2e}");
        assert!((mu_num - 1.0).abs() < 1e-9, "numerator 应≈1, got {mu_num}");
        assert!((mu_den - 1.0).abs() < 1e-9, "denominator 应≈1, got {mu_den}");
        assert!((mu - 1.0).abs() < 1e-9, "vd 应≈1, got {mu}");
        assert!(var.abs() < 1e-15, "纯态精确估计方差应为 0, got {var}");
    }
}

#[test]
fn vd_hamiltonian_expansion_places_observable_and_ancilla_x() {
    // 通过估计器接收到的哈密顿量验证分子展开：原算符保持在第一份拷贝上，
    // 辅助比特（最末）为 X，总宽度 = copies × w + 1；分母只有辅助比特 X。
    let base = bell_circuit();
    let hamiltonian = zz_hamiltonian();
    let copies = 2usize;
    let base_width = base.num_qubits();
    let ancilla = copies * base_width;

    let seen: RefCell<Vec<(usize, Pauli)>> = RefCell::new(Vec::new());
    let est = |c: &Circuit, h: Option<&Hamiltonian>, _s: Option<usize>| {
        let h = h.expect("vd hamiltonian");
        let ancilla_x = h.terms[0].0.get_pauli(ancilla);
        seen.borrow_mut().push((h.num_qubits, ancilla_x));
        (ideal_expectation(c, h), 0.0)
    };

    let vd = VirtualDistillation::new(bell_circuit(), copies).unwrap();
    vd.run_numerator_circuit(&hamiltonian, 0, &est).unwrap();
    vd.run_denominator_circuit(0, &est).unwrap();

    let seen = seen.borrow();
    assert_eq!(seen[0].0, copies * base_width + 1, "分子哈密顿量宽度");
    assert_eq!(seen[0].1, Pauli::X, "辅助比特应为 X");
    assert_eq!(seen[1].0, copies * base_width + 1, "分母哈密顿量宽度");
    assert_eq!(seen[1].1, Pauli::X, "分母辅助比特应为 X");
    println!("[vd] 哈密顿量展开正确：宽度={}（期望 {}），辅助比特={:?}（期望 X）",
             seen[0].0, copies * base_width + 1, seen[0].1);
}

#[test]
fn vd_depolarizing_noise_reduction() {
    // 基线路 X（制备 |1⟩），观测 Z（理想 ⟨Z⟩ = -1）；去极化强度 p=0.3。
    let p = 0.3;
    let mut base = Circuit::new(1);
    base.x(Qubit::new(0)).unwrap();
    let hamiltonian = Hamiltonian::from_pauli("Z".parse::<PauliString>().expect("parse Z"));

    // M=1 原始（未缓解）期望值（解析公式）
    let raw = raw_depolarized_expectation(p);
    println!("[vd] M=1 原始含噪 ⟨Z⟩ = {raw:.9}（理想 -1，误差 {:.4}）", (raw + 1.0).abs());

    // M=2、M=3：解析值 Tr(Zρ^M)/Tr(ρ^M)
    for copies in [2usize, 3usize] {
        let est = depolarizing_estimator(p);
        let vd = VirtualDistillation::new(base.clone(), copies).unwrap();

        let (mu_den, _) = vd.run_denominator_circuit(0, &est).unwrap();
        let (mu_num, _) = vd.run_numerator_circuit(&hamiltonian, 0, &est).unwrap();
        let (mu, var) = vd.run_vd(&hamiltonian, 0, 0, &est).unwrap();

        // 解析期望值（ρ 本征值 λ1=(1-2p/3), λ2=2p/3，来自 Kraus 信道）
        let l1 = 1.0 - 2.0 * p / 3.0;
        let l2 = 2.0 * p / 3.0;
        let tr_rho_m = l1.powi(copies as i32) + l2.powi(copies as i32);
        let tr_z_rho_m = l1.powi(copies as i32) * (-1.0) + l2.powi(copies as i32) * 1.0;
        let expected_vd = tr_z_rho_m / tr_rho_m;
        let expected_den = tr_rho_m;

        println!("[vd] M={copies}: denominator={mu_den:.9}（解析 {expected_den:.9}） numerator={mu_num:.9} vd={mu:.9}（解析 {expected_vd:.9}） var={var:.2e}");
        assert!((mu_den - expected_den).abs() < 1e-9, "denominator M={copies}");
        assert!((mu_num - tr_z_rho_m).abs() < 1e-9, "numerator M={copies}");
        assert!((mu - expected_vd).abs() < 1e-9, "vd M={copies}");

        // 误差压缩：M 越大越接近理想值 -1
        let err = (mu + 1.0).abs();
        assert!(err < (raw + 1.0).abs(), "M={copies} 误差 {err} 应小于 M=1 误差 {}", (raw + 1.0).abs());
    }
}

#[test]
fn vd_ratio_and_variance_formula() {
    // 隔离验证 run_vd 内部的矩比与方差传播公式（Huggins 论文 D11）：
    //   mu_vd = mu_num / mu_den
    //   var_vd = var_num/mu_den² + mu_num²·var_den/mu_den⁴
    // 用调用顺序区分分子/分母：run_vd 先算分子、后算分母。
    let call: RefCell<u32> = RefCell::new(0);
    let est = |_: &Circuit, _: Option<&Hamiltonian>, _: Option<usize>| {
        let n = { let mut n = call.borrow_mut(); *n += 1; *n };
        if n == 1 {
            (0.9, 0.0) // numerator
        } else {
            (1.2, 0.2) // denominator
        }
    };

    let base = bell_circuit();
    let vd = VirtualDistillation::new(base, 2).unwrap();
    let (mu, var) = vd.run_vd(&zz_hamiltonian(), 1000, 1000, &est).unwrap();

    let expected_mu = 0.9 / 1.2;
    let expected_var = 0.0 / 1.2_f64.powi(2) + 0.9_f64.powi(2) * 0.2 / 1.2_f64.powi(4);
    println!("[vd] ratio: mu={mu:.9}（期望 {expected_mu:.9}） var={var:.9}（期望 {expected_var:.9}）");
    assert!((mu - expected_mu).abs() < 1e-12, "mu");
    assert!((var - expected_var).abs() < 1e-12, "var");
}

#[test]
fn vd_error_paths() {
    // 1) 哈密顿量宽度不匹配（run_vd 入口校验）
    let base = bell_circuit();
    let vd = VirtualDistillation::new(base.clone(), 2).unwrap();
    let h3 = Hamiltonian::from_pauli("ZZI".parse::<PauliString>().expect("parse ZZI"));
    let est = |_: &Circuit, _: Option<&Hamiltonian>, _: Option<usize>| (0.0, 0.0);
    let err = vd.run_vd(&h3, 0, 0, &est).expect_err("width mismatch");
    println!("[vd] run_vd(3-qubit H, 2-qubit base) -> Err: {err}");
    assert!(
        matches!(err, ErrorMitigationError::HamiltonianQubitCountMismatch { expected: 2, actual: 3 }),
        "got {err:?}"
    );

    // 2) 展开后宽度与 VD 电路宽度不符（run_numerator_circuit 内部校验）
    let err = vd.run_numerator_circuit(&h3, 0, &est).expect_err("expanded width mismatch");
    println!("[vd] run_numerator_circuit(3-qubit H, copies=2) -> Err: {err}");
    assert!(
        matches!(err, ErrorMitigationError::Circuit(CircuitError::QubitCountMismatch { expected: 5, actual: 7 })),
        "got {err:?}"
    );

    // 3) 分母均值 0 → ZeroDenominatorMean
    let est_zero = |_: &Circuit, _: Option<&Hamiltonian>, _: Option<usize>| (0.0, 0.0);
    let err = vd.run_vd(&zz_hamiltonian(), 0, 0, &est_zero).expect_err("zero denominator");
    println!("[vd] run_vd(分母均值=0) -> Err: {err}");
    assert!(matches!(err, ErrorMitigationError::ZeroDenominatorMean), "got {err:?}");
}

#[test]
fn vd_build_rejects_control_flow_circuit() {
    // 含经典控制流的基线路无法 decompose，build 必须失败
    let mut base = Circuit::new(1);
    base.h(Qubit::new(0)).unwrap();
    base.if_(cqlib_core::circuit::ClassicalExpr::bool_literal(true), |body| {
        body.x(Qubit::new(0))
    })
    .unwrap();

    let vd = VirtualDistillation::new(base, 2).unwrap();
    let err = vd.build_copy_swap_circuit().expect_err("control flow must fail");
    println!("[vd] build_copy_swap_circuit(含 if) -> Err: {err}");
    assert!(
        matches!(err, CircuitError::InvalidOperation(_)),
        "got {err:?}"
    );
}

#[test]
fn vd_shots_passthrough() {
    let seen: RefCell<Vec<Option<usize>>> = RefCell::new(Vec::new());
    let est = |_: &Circuit, _: Option<&Hamiltonian>, shots: Option<usize>| {
        seen.borrow_mut().push(shots);
        (0.5, 0.0)
    };

    let base = bell_circuit();
    let vd = VirtualDistillation::new(base, 2).unwrap();
    vd.run_denominator_circuit(500, &est).unwrap();
    vd.run_numerator_circuit(&zz_hamiltonian(), 2000, &est).unwrap();

    let seen = seen.borrow();
    assert_eq!(seen[0], Some(500), "denominator shots");
    assert_eq!(seen[1], Some(2000), "numerator shots");
    println!("[vd] shots 透传: denominator=Some(500), numerator=Some(2000) ✓");
}