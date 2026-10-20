//! 误差缓解文档符合性测试（ZNE + Virtual Distillation + 统一接口）
//!
//! 参照文档：《Cqlib_Core_误差缓解算法总结.md》（release/2.0 分支）
//! 被测目标：`crates/cqlib-core/src/error_mitigation/`
//!   - zne_mitigation.rs（文档第一节：折叠变体、外推变体、限制）
//!   - virtual_distillation.rs（文档第二节：核心公式、资源公式、限制）
//!   - unified.rs（文档第三节：ErrorMitigation 统一接口契约）
//! 测试方式：integration test —— 仅通过 `cqlib_core` 公开 API 访问实现。
//!
//! 运行命令：cargo test -p cqlib-core --test error_mitigation_doc_conformance_test -- --nocapture

use std::cell::RefCell;

use num_complex::Complex64;

use cqlib_core::circuit::{Circuit, CircuitError, Instruction, Qubit, StandardGate};
use cqlib_core::error_mitigation::{
    ErrorMitigation, ErrorMitigationError, ExtrapolateMethod, MitigationMethod, ProcessArgs,
    RunArgs, VirtualDistillation, VirtualDistillationConfig, ZNEMitigation, ZneConfig,
};
use cqlib_core::qis::{DensityMatrix, Hamiltonian, PauliString};

/// Bell 态线路（H + CX），2 比特、2 操作。
fn bell_circuit() -> Circuit {
    let mut circuit = Circuit::new(2);
    circuit.h(Qubit::new(0)).expect("append H");
    circuit.cx(Qubit::new(0), Qubit::new(1)).expect("append CX");
    circuit
}

/// 哈密顿量 H = ZZ。
fn zz_hamiltonian() -> Hamiltonian {
    Hamiltonian::from_pauli("ZZ".parse::<PauliString>().expect("parse ZZ"))
}

/// ⟨ψ|H|ψ⟩：线路矩阵第一列（|0…0⟩ 输入）确定性计算。
fn ideal_expectation(circuit: &Circuit, hamiltonian: &Hamiltonian) -> f64 {
    let circuit_matrix = circuit.to_matrix(None).expect("circuit matrix");
    let h_matrix = hamiltonian.to_matrix().expect("hamiltonian matrix");
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

/// 文档第二节资源公式验证的样本线路（n 比特，可含任意标准门）。
fn base_circuit(n: usize) -> Circuit {
    let mut circuit = Circuit::new(n);
    if n >= 1 {
        circuit.h(Qubit::new(0)).unwrap();
    }
    if n >= 2 {
        circuit.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    }
    if n >= 3 {
        circuit.cx(Qubit::new(1), Qubit::new(2)).unwrap();
    }
    circuit
}

/// 去极化 Kraus 信道估计器（Pauli 信道：ρ 本征值 λ1=1-2p/3、λ2=2p/3）。
fn depolarizing_estimator(
    p: f64,
) -> impl Fn(&Circuit, Option<&Hamiltonian>, Option<usize>) -> (f64, f64) {
    move |circuit, hamiltonian, _shots| {
        let num_qubits = circuit.num_qubits();
        let mut state = DensityMatrix::new(num_qubits);
        let keep = (1.0 - p).sqrt();
        let flip = (p / 3.0).sqrt();
        let c = Complex64::new;
        let k0 = vec![c(keep, 0.0), c(0.0, 0.0), c(0.0, 0.0), c(keep, 0.0)];
        let k1 = vec![c(0.0, 0.0), c(flip, 0.0), c(flip, 0.0), c(0.0, 0.0)];
        let k2 = vec![c(0.0, 0.0), c(0.0, -flip), c(0.0, flip), c(0.0, 0.0)];
        let k3 = vec![c(flip, 0.0), c(0.0, 0.0), c(0.0, 0.0), c(-flip, 0.0)];

        let derangement_start = circuit
            .operations()
            .iter()
            .position(|op| matches!(op.instruction, Instruction::Standard(StandardGate::H)))
            .expect("VD circuit contains ancilla H");

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
                    state.apply_standard_gate(*gate, &qubit_indices, &params).unwrap();
                }
                Instruction::McGate(mc_gate) => {
                    let matrix = mc_gate.matrix(&params).unwrap();
                    state.apply_unitary_gate(&qubit_indices, &matrix).unwrap();
                }
                other => panic!("unexpected instruction {other:?}"),
            }
        }
        let hamiltonian = hamiltonian.expect("estimator receives a Hamiltonian");
        (state.expectation(hamiltonian).unwrap(), 0.0)
    }
}

// ============ 一、ZNE（文档第一节） ============

#[test]
fn doc_zne_accessors_and_noise_factor_mapping() {
    // 文档：折叠层级 l 对应噪声放大因子 λ=2l+1；关键接口 circuit()/fold_levels()/noise_factors()
    let zne = ZNEMitigation::new(bell_circuit(), vec![0, 1, 2]);
    assert_eq!(zne.fold_levels(), &[0, 1, 2]);
    assert_eq!(zne.noise_factors(), &[1, 3, 5]);
    assert_eq!(zne.circuit().operations().len(), 2);

    let zne2 = ZNEMitigation::new(bell_circuit(), vec![0, 1, 3]);
    assert_eq!(zne2.noise_factors(), &[1, 3, 7]);
    println!("[doc-zne] fold_levels [0,1,2]→noise_factors {:?}；[0,1,3]→{:?}",
             zne.noise_factors(), zne2.noise_factors());
}

#[test]
fn doc_zne_global_folding_identity() {
    // 文档：全局折叠 U(U†U)^l —— 等价线路不改理想结果
    let base = bell_circuit();
    let zne = ZNEMitigation::new(base.clone(), vec![0, 1, 2]);
    let folded = zne.fold_circuits(None).expect("fold");
    let ops: Vec<usize> = folded.iter().map(|c| c.operations().len()).collect();
    let delta = max_unitary_delta(&folded[2], &base);
    println!("[doc-zne] 全局折叠 ops={ops:?}（期望 2/6/10）level2 酉偏差={delta:.2e}");
    assert_eq!(ops, vec![2, 6, 10]);
    assert!(delta < 1e-10, "U(U†U)^l 应与 U 等价（偏差 {delta}）");
}

#[test]
fn doc_zne_selective_folding_variant() {
    // 文档：选择性折叠只折叠 gate_set 中指定的门
    let mut base = Circuit::new(2);
    base.h(Qubit::new(0)).unwrap();
    base.cx(Qubit::new(0), Qubit::new(1)).unwrap();
    base.t(Qubit::new(0)).unwrap();
    let zne = ZNEMitigation::new(base.clone(), vec![0, 1, 2]);

    let folded = zne
        .fold_circuits(Some(&[Instruction::Standard(StandardGate::H)]))
        .expect("selective fold");
    let ops: Vec<usize> = folded.iter().map(|c| c.operations().len()).collect();
    let delta = max_unitary_delta(&folded[2], &base);
    println!("[doc-zne] 选择性折叠(H) ops={ops:?}（期望 3/5/7）酉偏差={delta:.2e}");
    assert_eq!(ops, vec![3, 5, 7]);
    assert!(delta < 1e-10);
}

#[test]
fn doc_zne_extrapolation_variants() {
    // 文档：多项式外推支持一/二次及更高阶（阶数必须小于数据点数）；指数外推要求全正
    let zne = ZNEMitigation::new(bell_circuit(), vec![0, 1, 2]); // λ=[1,3,5]

    let linear = [3.3, 1.9, 0.5]; // y = 4 - 0.7λ
    let deg1 = zne.poly_extrapolate(&linear, 1).unwrap();
    let deg2 = zne.poly_extrapolate(&linear, 2).unwrap();
    println!("[doc-zne] 多项式外推：deg1={deg1:.15} deg2={deg2:.15}（期望 4.0）");
    assert!((deg1 - 4.0).abs() < 1e-12);
    assert!((deg2 - 4.0).abs() < 1e-8);

    let err = zne.poly_extrapolate(&linear, 3).unwrap_err();
    assert!(matches!(err, ErrorMitigationError::InvalidPolynomialDegree { degree: 3, num_points: 3 }));

    let exp_data: Vec<f64> = [1.0f64, 3.0f64, 5.0f64]
        .iter()
        .map(|&l| 2.5 * (-l / 3.0).exp())
        .collect();
    let fitted = zne.extrapolate(&exp_data, ExtrapolateMethod::Exponential, 0).unwrap();
    println!("[doc-zne] 指数外推：{fitted:.15}（期望 2.5）");
    assert!((fitted - 2.5).abs() < 1e-9);

    let err = zne.exp_extrapolate(&[0.5, -0.25, 0.1]).unwrap_err();
    assert!(matches!(err, ErrorMitigationError::NonPositiveNoisyResults));
    println!("[doc-zne] 外推变体契约全部符合（含阶数与正数校验）");
}

#[test]
fn doc_zne_irreversible_folding_fails() {
    // 文档：折叠对象必须能够求逆，不可逆操作可能导致构造失败
    let mut circuit = Circuit::new(2);
    circuit.h(Qubit::new(0)).unwrap();
    let _m = circuit.measure(Qubit::new(0)).unwrap();

    let err = ZNEMitigation::new(circuit.clone(), vec![1])
        .fold_circuits(None)
        .unwrap_err();
    println!("[doc-zne] 含测量线路折叠 → Err: {err}");
    assert!(matches!(err, CircuitError::IrreversibleOperation));

    // 统一接口同样拒绝
    let err = ErrorMitigation::new(circuit, MitigationMethod::Zne(ZneConfig { fold_levels: vec![1] }))
        .unwrap()
        .run(&zz_hamiltonian(), RunArgs::Zne { gate_set: None, shots: None }, &|_, _, _| (1.0, 0.0))
        .unwrap_err();
    println!("[doc-zne] 统一接口含测量线路 → Err: {err}");
    assert!(matches!(err, ErrorMitigationError::Circuit(CircuitError::IrreversibleOperation)));
}

// ============ 二、Virtual Distillation（文档第二节） ============

#[test]
fn doc_vd_resource_formula() {
    // 文档资源公式：M 份副本 + 1 辅助比特 → Mn+1 个量子比特；约 (M-1)n 个受控 SWAP
    for (n, m) in [(1usize, 2usize), (2, 2), (2, 3), (3, 2)] {
        let vd = VirtualDistillation::new(base_circuit(n), m).unwrap();
        let circuit = vd.build_copy_swap_circuit().unwrap();
        let cswaps = circuit
            .operations()
            .iter()
            .filter(|op| matches!(op.instruction, Instruction::McGate(_)))
            .count();
        println!(
            "[doc-vd] n={n}, M={m}: width={}（期望 {}），CSWAP={}（期望 {}）",
            circuit.num_qubits(),
            m * n + 1,
            cswaps,
            (m - 1) * n
        );
        assert_eq!(circuit.num_qubits(), m * n + 1);
        assert_eq!(cswaps, (m - 1) * n);
    }
}

#[test]
fn doc_vd_pure_state_core_formula() {
    // 文档核心公式：⟨O⟩_VD = Tr(Oρ^M)/Tr(ρ^M)；纯态下退化为 ⟨O⟩
    let hamiltonian = zz_hamiltonian();
    let ideal = {
        let base = bell_circuit();
        ideal_expectation(&base, &hamiltonian)
    };
    for copies in [2usize, 3usize] {
        let vd = VirtualDistillation::new(bell_circuit(), copies).unwrap();
        let est = |c: &Circuit, h: Option<&Hamiltonian>, _: Option<usize>| {
            (ideal_expectation(c, h.expect("H")), 0.0)
        };
        let (num, _) = vd.run_numerator_circuit(&hamiltonian, 0, &est).unwrap();
        let (den, _) = vd.run_denominator_circuit(0, &est).unwrap();
        let (mu, var) = vd.run_vd(&hamiltonian, 0, 0, &est).unwrap();
        println!(
            "[doc-vd] 纯态 M={copies}: Tr(Oρ^M)={num:.12} Tr(ρ^M)={den:.12} 比值={mu:.12}（理想 {ideal:.12}）var={var:.2e}"
        );
        assert!((mu - ideal).abs() < 1e-9, "纯态矩比应等于 ⟨O⟩");
    }
}

#[test]
fn doc_vd_variance_propagation_formula() {
    // 文档：方差采用分子与分母相互独立的近似传播公式（D11）
    // run_vd 先分子后分母，用调用序区分两者
    let call: RefCell<u32> = RefCell::new(0);
    let est = |_: &Circuit, _: Option<&Hamiltonian>, _: Option<usize>| {
        let n = { let mut n = call.borrow_mut(); *n += 1; *n };
        if n == 1 { (0.9, 0.05) } else { (1.2, 0.2) }
    };
    let vd = VirtualDistillation::new(bell_circuit(), 2).unwrap();
    let (mu, var) = vd.run_vd(&zz_hamiltonian(), 100, 100, &est).unwrap();
    let expected_mu = 0.9 / 1.2;
    let expected_var = 0.05 / 1.2_f64.powi(2) + 0.9_f64.powi(2) * 0.2 / 1.2_f64.powi(4);
    println!("[doc-vd] 矩比 mu={mu:.9}（期望 {expected_mu:.9}）；方差 var={var:.9}（期望 {expected_var:.9}）");
    assert!((mu - expected_mu).abs() < 1e-12);
    assert!((var - expected_var).abs() < 1e-12);
}

#[test]
fn doc_vd_noise_suppression_multi_copy() {
    // 文档：多副本噪声抑制能力可能增强（去极化 p=0.3，X|1⟩ + Z 观测）
    let p = 0.3f64;
    let lambda1 = 1.0 - 2.0 * p / 3.0;
    let lambda2 = 2.0 * p / 3.0;

    let mut base = Circuit::new(1);
    base.x(Qubit::new(0)).unwrap();
    let hamiltonian = Hamiltonian::from_pauli("Z".parse::<PauliString>().unwrap());

    let raw = (lambda1 * (-1.0) + lambda2 * 1.0) / (lambda1 + lambda2);
    println!("[doc-vd] M=1 原始含噪 ⟨Z⟩ = {raw:.9}（理想 -1，误差 {:.4}）", (raw + 1.0).abs());

    for copies in [2usize, 3usize] {
        let est = depolarizing_estimator(p);
        let vd = VirtualDistillation::new(base.clone(), copies).unwrap();
        let (mu, _) = vd.run_vd(&hamiltonian, 0, 0, &est).unwrap();
        let l1m = lambda1.powi(copies as i32);
        let l2m = lambda2.powi(copies as i32);
        let expected = (l1m * (-1.0) + l2m * 1.0) / (l1m + l2m);
        println!(
            "[doc-vd] M={copies}: vd={mu:.9}（解析 {expected:.9}）误差 {:.6} < M=1 误差 {:.4}",
            (mu + 1.0).abs(),
            (raw + 1.0).abs()
        );
        assert!((mu - expected).abs() < 1e-9);
        assert!((mu + 1.0).abs() < (raw + 1.0).abs(), "M={copies} 应优于 M=1");
    }
}

#[test]
fn doc_vd_zero_denominator_rejected() {
    // 文档：分母期望值不能为零
    let vd = VirtualDistillation::new(bell_circuit(), 2).unwrap();
    let err = vd
        .run_vd(&zz_hamiltonian(), 0, 0, &|_: &Circuit, _: Option<&Hamiltonian>, _: Option<usize>| (0.0, 0.0))
        .unwrap_err();
    println!("[doc-vd] 分母为 0 → Err: {err}");
    assert!(matches!(err, ErrorMitigationError::ZeroDenominatorMean));
}

// ============ 三、统一接口 ErrorMitigation（文档第三节） ============

#[test]
fn unified_config_validation() {
    // 文档：ZneConfig.fold_levels 非负；VirtualDistillationConfig.copies >= 2
    let err = ErrorMitigation::new(
        Circuit::new(1),
        MitigationMethod::Zne(ZneConfig { fold_levels: vec![0, -1] }),
    )
    .unwrap_err();
    println!("[unified] ZneConfig{{fold_levels:[0,-1]}} → Err: {err}");
    assert!(matches!(err, ErrorMitigationError::InvalidFoldLevel(-1)));

    let err = ErrorMitigation::new(
        Circuit::new(1),
        MitigationMethod::VirtualDistillation(VirtualDistillationConfig { copies: 1 }),
    )
    .unwrap_err();
    println!("[unified] VirtualDistillationConfig{{copies:1}} → Err: {err}");
    assert!(matches!(err, ErrorMitigationError::InvalidCopies(1)));
}

#[test]
fn unified_zne_full_pipeline() {
    // 文档：run 执行线路序列并保存原始统计 → get_mitigated 后处理；
    // 可重复调用不同后处理参数且不重新执行线路；ZNE 结果不传播方差（variance=None）
    const NOISE_RATE: f64 = 0.35;
    let base = bell_circuit();
    let base_ops = base.operations().len() as f64;
    let hamiltonian = zz_hamiltonian();
    let ideal = ideal_expectation(&base, &hamiltonian);

    let calls: RefCell<usize> = RefCell::new(0);
    let shots_seen: RefCell<Vec<Option<usize>>> = RefCell::new(Vec::new());
    let est = |circuit: &Circuit, h: Option<&Hamiltonian>, shots: Option<usize>| -> (f64, f64) {
        *calls.borrow_mut() += 1;
        shots_seen.borrow_mut().push(shots);
        let value = ideal_expectation(circuit, h.expect("H"));
        let lambda = circuit.operations().len() as f64 / base_ops;
        (value * (-NOISE_RATE * lambda).exp(), 1e-6)
    };

    let mut mitigation = ErrorMitigation::new(
        base,
        MitigationMethod::Zne(ZneConfig { fold_levels: vec![0, 1, 2] }),
    )
    .unwrap();

    // run：3 条折叠线路，shots 原样透传
    mitigation
        .run(&hamiltonian, RunArgs::Zne { gate_set: None, shots: Some(1024) }, &est)
        .unwrap();
    assert_eq!(*calls.borrow(), 3, "run 应恰好执行 3 条折叠线路");
    assert_eq!(shots_seen.borrow().as_slice(), &[Some(1024); 3]);
    println!("[unified-zne] run：estimator 调用 {} 次，shots 全部 Some(1024)", calls.borrow());

    // 后处理 1：二次多项式（Richardson）
    let poly2 = mitigation
        .get_mitigated(ProcessArgs::Zne { method: ExtrapolateMethod::Polynomial, degree: Some(2) })
        .unwrap();
    // 后处理 2：指数
    let expfit = mitigation
        .get_mitigated(ProcessArgs::Zne { method: ExtrapolateMethod::Exponential, degree: None })
        .unwrap();
    // 后处理 3：默认阶数（3 点数据 → degree=1）
    let poly_default = mitigation
        .get_mitigated(ProcessArgs::Zne { method: ExtrapolateMethod::Polynomial, degree: None })
        .unwrap();
    let poly1 = mitigation
        .get_mitigated(ProcessArgs::Zne { method: ExtrapolateMethod::Polynomial, degree: Some(1) })
        .unwrap();

    // 文档契约：重复后处理不重跑线路
    assert_eq!(*calls.borrow(), 3, "get_mitigated 不应重新执行线路");
    println!(
        "[unified-zne] 后处理×4 后 estimator 调用仍为 {} 次（不重跑）", calls.borrow()
    );

    println!(
        "[unified-zne] poly2={:.9} exp={:.9} default(deg1)={:.9}（理想 {ideal:.9}）",
        poly2.expectation, expfit.expectation, poly_default.expectation
    );
    assert!(poly2.variance.is_none(), "文档：ZNE 不传播方差（variance=None）");
    assert!(expfit.variance.is_none());
    assert!((expfit.expectation - ideal).abs() < 1e-9, "指数外推应恢复理想值");
    assert!((poly2.expectation - ideal).abs() < 0.06, "Richardson 外推误差 <0.06");
    // 默认阶数 = min(点数-1, 1) = 1：与显式 degree=1 完全一致
    assert_eq!(poly_default.expectation, poly1.expectation, "默认阶数应等价于 degree=1");
    // 且默认(线性)结果介于未缓解基线与理想值之间
    assert!(poly_default.expectation > expfit.expectation * 0.70 && poly_default.expectation < ideal + 1e-9);
}

#[test]
fn unified_vd_full_pipeline() {
    // 文档：VD 统一接口同时返回期望值和方差（均有效）
    let hamiltonian = zz_hamiltonian();
    let calls: RefCell<Vec<Option<usize>>> = RefCell::new(Vec::new());
    let est = |circuit: &Circuit, h: Option<&Hamiltonian>, shots: Option<usize>| -> (f64, f64) {
        calls.borrow_mut().push(shots);
        (ideal_expectation(circuit, h.expect("H")), 0.0)
    };

    let mut mitigation = ErrorMitigation::new(
        bell_circuit(),
        MitigationMethod::VirtualDistillation(VirtualDistillationConfig { copies: 2 }),
    )
    .unwrap();

    mitigation
        .run(
            &hamiltonian,
            RunArgs::VirtualDistillation { shots_numerator: 512, shots_denominator: 512 },
            &est,
        )
        .unwrap();
    assert_eq!(calls.borrow().as_slice(), &[Some(512), Some(512)], "分子/分母 shots 应分别透传");

    let first = mitigation.get_mitigated(ProcessArgs::VirtualDistillation).unwrap();
    let second = mitigation.get_mitigated(ProcessArgs::VirtualDistillation).unwrap();
    assert_eq!(*calls.borrow(), vec![Some(512), Some(512)], "重复 get_mitigated 不重跑");

    println!(
        "[unified-vd] expectation={:.12} variance={:?}（重复后处理一致：{}）",
        first.expectation,
        first.variance,
        first == second
    );
    assert!((first.expectation - 1.0).abs() < 1e-9, "纯态 VD 期望应为 1");
    assert!(first.variance.is_some(), "文档：VD 期望值和方差均有效");
    assert!(first.variance.unwrap().abs() < 1e-12, "精确估计方差应为 0");
    assert_eq!(first, second);
}

#[test]
fn unified_state_and_args_mismatches() {
    let hamiltonian = zz_hamiltonian();
    let trivial = |_: &Circuit, _: Option<&Hamiltonian>, _: Option<usize>| (1.0, 0.0);

    // --- ZNE 侧 ---
    let mut zne = ErrorMitigation::new(
        bell_circuit(),
        MitigationMethod::Zne(ZneConfig { fold_levels: vec![0, 1, 2] }),
    )
    .unwrap();

    // 文档：run() 必须先于 get_mitigated()
    let err = zne
        .get_mitigated(ProcessArgs::Zne { method: ExtrapolateMethod::Polynomial, degree: Some(1) })
        .unwrap_err();
    println!("[unified] get_mitigated 先于 run → Err: {err}");
    assert!(matches!(err, ErrorMitigationError::RunRequiredBeforeMitigation));

    // 方法与运行参数不匹配
    let err = zne
        .run(&hamiltonian, RunArgs::VirtualDistillation { shots_numerator: 1, shots_denominator: 1 }, &trivial)
        .unwrap_err();
    println!("[unified] ZNE 方法 + VD RunArgs → Err: {err}");
    assert!(matches!(err, ErrorMitigationError::RunArgsMethodMismatch));

    // 正常 run
    zne.run(&hamiltonian, RunArgs::Zne { gate_set: None, shots: None }, &trivial).unwrap();

    // 文档隐含状态机：run 只能执行一次
    let err = zne.run(&hamiltonian, RunArgs::Zne { gate_set: None, shots: None }, &trivial).unwrap_err();
    println!("[unified] 重复 run → Err: {err}");
    assert!(matches!(err, ErrorMitigationError::AlreadyRun));

    // 方法与后处理参数不匹配
    let err = zne.get_mitigated(ProcessArgs::VirtualDistillation).unwrap_err();
    println!("[unified] ZNE 方法 + VD ProcessArgs → Err: {err}");
    assert!(matches!(err, ErrorMitigationError::ProcessArgsMethodMismatch));

    // 正常后处理仍可用
    let ok = zne
        .get_mitigated(ProcessArgs::Zne { method: ExtrapolateMethod::Polynomial, degree: Some(1) })
        .unwrap();
    println!("[unified] ZNE 后处理恢复正常：expectation={:.9}", ok.expectation);

    // --- VD 侧 ---
    let mut vd = ErrorMitigation::new(
        bell_circuit(),
        MitigationMethod::VirtualDistillation(VirtualDistillationConfig { copies: 2 }),
    )
    .unwrap();

    let err = vd
        .run(&hamiltonian, RunArgs::Zne { gate_set: None, shots: None }, &trivial)
        .unwrap_err();
    println!("[unified] VD 方法 + ZNE RunArgs → Err: {err}");
    assert!(matches!(err, ErrorMitigationError::RunArgsMethodMismatch));

    // 文档示例式估计器：分子 1.5/0.25，分母 2.0/1.0
    let call: RefCell<u32> = RefCell::new(0);
    let est = |_: &Circuit, h: Option<&Hamiltonian>, _: Option<usize>| {
        let _ = h.unwrap();
        let n = { let mut n = call.borrow_mut(); *n += 1; *n };
        if n == 1 { (1.5, 0.25) } else { (2.0, 1.0) }
    };
    vd.run(&hamiltonian, RunArgs::VirtualDistillation { shots_numerator: 256, shots_denominator: 256 }, &est)
        .unwrap();

    let err = vd
        .get_mitigated(ProcessArgs::Zne { method: ExtrapolateMethod::Polynomial, degree: Some(1) })
        .unwrap_err();
    println!("[unified] VD 方法 + ZNE ProcessArgs → Err: {err}");
    assert!(matches!(err, ErrorMitigationError::ProcessArgsMethodMismatch));

    let ok = vd.get_mitigated(ProcessArgs::VirtualDistillation).unwrap();
    let expected_var = 0.25 / 2.0_f64.powi(2) + 1.5_f64.powi(2) * 1.0 / 2.0_f64.powi(4);
    println!(
        "[unified-vd] 文档示例参数：expectation={:.9} variance={:.9}（期望 0.75 / {expected_var:.9}）",
        ok.expectation,
        ok.variance.unwrap()
    );
    assert!((ok.expectation - 0.75).abs() < 1e-12);
    assert!((ok.variance.unwrap() - expected_var).abs() < 1e-12);
}

#[test]
fn unified_hamiltonian_width_checks() {
    // 统一接口对哈密顿量宽度与线路一致性的校验（两种方法一致）
    let hamiltonian3 = Hamiltonian::from_pauli("ZZI".parse::<PauliString>().unwrap());
    let trivial = |_: &Circuit, _: Option<&Hamiltonian>, _: Option<usize>| (1.0, 0.0);

    let mut zne = ErrorMitigation::new(
        bell_circuit(),
        MitigationMethod::Zne(ZneConfig { fold_levels: vec![0, 1] }),
    )
    .unwrap();
    let err = zne
        .run(&hamiltonian3, RunArgs::Zne { gate_set: None, shots: None }, &trivial)
        .unwrap_err();
    println!("[unified] ZNE 宽度不匹配 → Err: {err}");
    assert!(matches!(err, ErrorMitigationError::HamiltonianQubitCountMismatch { expected: 2, actual: 3 }));

    let mut vd = ErrorMitigation::new(
        bell_circuit(),
        MitigationMethod::VirtualDistillation(VirtualDistillationConfig { copies: 2 }),
    )
    .unwrap();
    let err = vd
        .run(&hamiltonian3, RunArgs::VirtualDistillation { shots_numerator: 1, shots_denominator: 1 }, &trivial)
        .unwrap_err();
    println!("[unified] VD 宽度不匹配 → Err: {err}");
    assert!(matches!(err, ErrorMitigationError::HamiltonianQubitCountMismatch { expected: 2, actual: 3 }));
}