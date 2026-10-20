//! ZNE（零噪声外推）错误缓解模块的测试程序
//!
//! 被测目标：`crates/cqlib-core/src/error_mitigation/zne_mitigation.rs`（ZNEMitigation）
//! 测试方式：integration test —— 仅通过 `cqlib_core` 对外公开 API 访问实现，
//!            因此同时验证库的接口契约与本体的物理正确性。
//!
//! 覆盖内容：
//! 1. 构造与访问器（fold_levels ↔ noise_factors 映射）
//! 2. 全局折叠 / 选择性折叠：操作数、酉矩阵等价性（U·(U†U)^k ≡ U）
//! 3. 非法输入：负折叠级别、含测量线路、哈密顿量宽度不匹配、坏外推输入
//! 4. 物理主链路：含噪估计器 + 多项式（Richardson）/ 指数外推恢复理想期望值
//! 5. 外推数值正确性（线性模型精确恢复、已知振幅指数模型恢复）
//!
//! 运行命令：cargo test -p cqlib-core --test zne_program_test -- --nocapture

use std::cell::RefCell;

use cqlib_core::circuit::{Circuit, CircuitError, Instruction, Qubit, StandardGate};
use cqlib_core::error_mitigation::{ErrorMitigationError, ExtrapolateMethod, ZNEMitigation};
use cqlib_core::qis::{Hamiltonian, PauliString};

/// Bell 态线路 |00⟩+|11⟩（H + CX），2 比特、2 个操作。
fn bell_circuit() -> Circuit {
    let mut circuit = Circuit::new(2);
    circuit.h(Qubit::new(0)).expect("append H");
    circuit.cx(Qubit::new(0), Qubit::new(1)).expect("append CX");
    circuit
}

/// 三操作线路：H(0)、CX(0,1)、T(0)，用于选择性折叠测试。
fn three_gate_circuit() -> Circuit {
    let mut circuit = Circuit::new(2);
    circuit.h(Qubit::new(0)).expect("append H");
    circuit.cx(Qubit::new(0), Qubit::new(1)).expect("append CX");
    circuit.t(Qubit::new(0)).expect("append T");
    circuit
}

/// 哈密顿量 H = ZZ（Bell 态上期望值恰为 +1）。
fn zz_hamiltonian() -> Hamiltonian {
    Hamiltonian::from_pauli("ZZ".parse::<PauliString>().expect("parse ZZ pauli string"))
}

/// ⟨ψ|H|ψ⟩：把线路矩阵作用于 |0…0⟩（即取矩阵第一列），确定性计算、无统计涨落。
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

/// 计算两条线路酉矩阵的最大逐元素偏差。
fn max_unitary_delta(a: &Circuit, b: &Circuit) -> f64 {
    let ma = a.to_matrix(None).expect("matrix of circuit a");
    let mb = b.to_matrix(None).expect("matrix of circuit b");
    let mut max = 0.0f64;
    for (x, y) in ma.iter().zip(mb.iter()) {
        let z = *x - *y;
        max = max.max(z.re.hypot(z.im));
    }
    max
}

/// 逐元素比较两条线路的酉矩阵是否一致（容差 1e-10）。
fn assert_same_unitary(a: &Circuit, b: &Circuit, what: &str) {
    let delta = max_unitary_delta(a, b);
    assert!(
        delta < 1e-10,
        "{what}: unitary matrix entries differ by {delta} (> 1e-10)"
    );
}

#[test]
fn zne_noise_factor_mapping_and_accessors() {
    let zne = ZNEMitigation::new(bell_circuit(), vec![0, 1, 3]);

    assert_eq!(zne.fold_levels(), &[0, 1, 3]);
    // 噪声因子固定映射：λ = 2·level + 1
    assert_eq!(zne.noise_factors(), &[1, 3, 7]);
    // 原始线路可经访问器取回，不被折叠过程修改
    assert_eq!(zne.circuit().operations().len(), 2);

    println!(
        "[1] fold_levels={:?} noise_factors={:?} base_ops={}",
        zne.fold_levels(),
        zne.noise_factors(),
        zne.circuit().operations().len()
    );
}

#[test]
fn zne_global_folding_counts_and_unitary_equivalence() {
    let base = bell_circuit();
    let zne = ZNEMitigation::new(base.clone(), vec![0, 1, 2]);

    let folded = zne.fold_circuits(None).expect("global folding");
    assert_eq!(folded.len(), 3, "one folded circuit per fold level");

    // 全局折叠操作数 = (2k+1) × 基础操作数
    assert_eq!(folded[0].operations().len(), 2, "level 0: 1×2 ops");
    assert_eq!(folded[1].operations().len(), 6, "level 1: 3×2 ops");
    assert_eq!(folded[2].operations().len(), 10, "level 2: 5×2 ops");

    // 物理不变性：U·(U†U)^k ≡ U —— 折叠线路的酉矩阵应与原线路完全一致
    for (level, circuit) in folded.iter().enumerate() {
        assert_same_unitary(circuit, &base, &format!("global folded circuit level {level}"));
    }
    let counts: Vec<usize> = folded.iter().map(|c| c.operations().len()).collect();
    println!("[2] global fold ops = {counts:?} (期望 2/6/10)");
    println!(
        "[2] max unitary delta (level 2 vs base) = {:.3e}",
        max_unitary_delta(&folded[2], &base)
    );
}

#[test]
fn zne_selective_folding_matches_gate_names_only() {
    let base = three_gate_circuit();
    let zne = ZNEMitigation::new(base.clone(), vec![0, 1, 2]);

    // 仅折叠 H（线路中出现 1 次）：操作数 = 3 + 2k
    let fold_h = zne
        .fold_circuits(Some(&[Instruction::Standard(StandardGate::H)]))
        .expect("selective H folding");
    assert_eq!(fold_h[0].operations().len(), 3);
    assert_eq!(fold_h[1].operations().len(), 5);
    assert_eq!(fold_h[2].operations().len(), 7);
    for (level, circuit) in fold_h.iter().enumerate() {
        assert_same_unitary(circuit, &base, &format!("selective H fold level {level}"));
    }

    // 门集合不命中任何线路操作（本线路没有 SWAP）→ 不产生任何折叠
    let fold_miss = zne
        .fold_circuits(Some(&[Instruction::Standard(StandardGate::SWAP)]))
        .expect("non-matching gate set");
    for (level, circuit) in fold_miss.iter().enumerate() {
        assert_eq!(
            circuit.operations().len(),
            3,
            "non-matching gate set must not fold (level {level})"
        );
    }

    let h_counts: Vec<usize> = fold_h.iter().map(|c| c.operations().len()).collect();
    let miss_counts: Vec<usize> = fold_miss.iter().map(|c| c.operations().len()).collect();
    println!("[3] selective H fold ops = {h_counts:?} (期望 3/5/7)");
    println!("[3] non-matching gate set ops = {miss_counts:?} (期望 3/3/3)");
    println!(
        "[3] max unitary delta (fold H vs base) = {:.3e}",
        max_unitary_delta(&fold_h[2], &base)
    );
}

#[test]
fn zne_fold_rejects_negative_level() {
    let zne = ZNEMitigation::new(bell_circuit(), vec![-1]);
    let error = zne.fold_circuits(None).expect_err("negative level must fail");
    println!("[4] fold_circuits(vec![−1]) -> Err: {error}");

    match &error {
        CircuitError::InvalidControlOperation(message) => {
            assert!(
                message.contains("non-negative"),
                "unexpected error message: {message}"
            );
        }
        other => panic!("expected InvalidControlOperation, got {other:?}"),
    }
}

#[test]
fn zne_fold_rejects_irreversible_circuit() {
    // 含测量的线路无法求逆，全局折叠必须失败
    let mut circuit = Circuit::new(2);
    circuit.h(Qubit::new(0)).expect("append H");
    let _measurement = circuit.measure(Qubit::new(0)).expect("mid-circuit measure");

    let zne = ZNEMitigation::new(circuit, vec![1]);
    let error = zne.fold_circuits(None).expect_err("measured circuit must not fold");
    println!("[5] fold_circuits(含测量线路) -> Err: {error}");
    assert!(
        matches!(error, CircuitError::IrreversibleOperation),
        "expected IrreversibleOperation, got {error:?}"
    );
}

#[test]
fn zne_run_em_sequence_restores_expectation_under_noise() {
    // 含噪模型：⟨O⟩(λ) = ⟨O⟩_ideal · exp(-0.35·λ)，λ = 2k+1 为噪声因子。
    // 折叠后操作数与 λ 严格成正比，估计器据此还原物理噪声。
    const NOISE_RATE: f64 = 0.35;

    let base = bell_circuit();
    let base_ops = base.operations().len() as f64;
    let hamiltonian = zz_hamiltonian();
    let ideal = ideal_expectation(&base, &hamiltonian);
    assert!((ideal - 1.0).abs() < 1e-12, "Bell 态 ⟨ZZ⟩ 应恰为 1，实际 {ideal}");

    let seen_shots: RefCell<Vec<Option<usize>>> = RefCell::new(Vec::new());
    let estimator = |circuit: &Circuit,
                     h: Option<&Hamiltonian>,
                     shots: Option<usize>|
     -> (f64, f64) {
        let hamiltonian = h.expect("estimator must receive a Hamiltonian");
        let value = ideal_expectation(circuit, hamiltonian);
        let lambda = circuit.operations().len() as f64 / base_ops;
        let noisy = value * (-NOISE_RATE * lambda).exp();
        seen_shots.borrow_mut().push(shots);
        (noisy, 1.0e-6)
    };

    let zne = ZNEMitigation::new(base, vec![0, 1, 2]);

    // 1) run_em_sequence：默认不传 shots
    let seq = zne
        .run_em_sequence(None, &hamiltonian, &estimator)
        .expect("run em sequence");
    assert_eq!(seen_shots.borrow().as_slice(), &[None, None, None][..]);

    let expected: Vec<f64> = [1.0, 3.0, 5.0]
        .iter()
        .map(|&lambda| ideal * (-NOISE_RATE * lambda).exp())
        .collect();
    for (i, (got, want)) in seq.iter().zip(expected.iter()).enumerate() {
        let delta = (got - want).abs();
        assert!(delta < 1e-12, "em sequence[{i}] = {got}, want {want}");
    }

    // λ=1 的原始含噪值（未缓解基线）：约 0.705，误差 ~30%
    let raw = seq[0];
    assert!(
        raw > 0.70 && raw < 0.71,
        "raw λ=1 expectation should be ≈0.705, got {raw}"
    );

    // 2) 指数外推：估计模型与衰减模型完全同型 → 机器精度恢复理想值
    let exp_fit = zne
        .extrapolate(&seq, ExtrapolateMethod::Exponential, 0)
        .expect("exponential extrapolation");
    let exp_err = (exp_fit - ideal).abs();
    assert!(
        exp_err < 1e-9,
        "exponential extrapolation = {exp_fit}, want ≈{ideal} (±1e-9)"
    );

    // 3) Richardson（2 阶多项式）外推：应显著逼近理想值并优于未缓解基线
    let poly_fit = zne
        .extrapolate(&seq, ExtrapolateMethod::Polynomial, 2)
        .expect("polynomial extrapolation");
    let poly_err = (poly_fit - ideal).abs();
    assert!(
        poly_err < 0.06,
        "Richardson extrapolation = {poly_fit}, want ≈{ideal} (±0.06)"
    );
    assert!(
        poly_err < (raw - ideal).abs(),
        "mitigated error {poly_err} must beat raw error {}",
        (raw - ideal).abs()
    );

    // 4) with_shots 变体：shots 原样透传给估计器，且不影响确定性结果
    seen_shots.borrow_mut().clear();
    let seq_shots = zne
        .run_em_sequence_with_shots(None, &hamiltonian, Some(4096), &estimator)
        .expect("run em sequence with shots");
    assert_eq!(seq_shots, seq, "shots must not change deterministic values");
    assert_eq!(seen_shots.borrow().as_slice(), &[Some(4096); 3][..]);

    println!("[6] ideal <ZZ> = {ideal:.15}");
    println!("[6] em sequence λ=(1,3,5) = {seq:?}");
    println!("[6] model expected values = {expected:?}");
    println!("[6] raw λ=1 baseline = {raw:.9} (误差 {:.9})", (raw - ideal).abs());
    println!("[6] exp extrapolation = {exp_fit:.15} (误差 {exp_err:.3e})");
    println!("[6] poly(Richardson, deg2) = {poly_fit:.9} (误差 {poly_err:.3e})");
    println!(
        "[6] with_shots(4096) -> {seq_shots:?} (shots 原样透传: {:?})",
        seen_shots.borrow().as_slice()
    );
}

#[test]
fn zne_run_em_rejects_hamiltonian_width_mismatch() {
    let zne = ZNEMitigation::new(bell_circuit(), vec![0, 1, 2]); // 2 比特线路
    let hamiltonian =
        Hamiltonian::from_pauli("ZZI".parse::<PauliString>().expect("parse ZZI")); // 3 比特算符
    let estimator = |_: &Circuit, _: Option<&Hamiltonian>, _: Option<usize>| (0.0, 0.0);

    let error = zne
        .run_em_sequence(None, &hamiltonian, &estimator)
        .expect_err("hamiltonian width mismatch must fail");
    println!("[7] run_em_sequence(3-qubit H on 2-qubit circuit) -> Err: {error}");
    assert!(
        matches!(
            error,
            ErrorMitigationError::HamiltonianQubitCountMismatch { expected: 2, actual: 3 }
        ),
        "expected HamiltonianQubitCountMismatch {{2, 3}}, got {error:?}"
    );
}

#[test]
fn zne_extrapolate_rejects_invalid_inputs() {
    let zne = ZNEMitigation::new(bell_circuit(), vec![0, 1, 2]); // 噪声因子 [1, 3, 5]

    // 空数据
    let error = zne.poly_extrapolate(&[], 1).expect_err("empty input");
    println!("[8] poly_extrapolate(&[]) -> Err: {error}");
    assert!(
        matches!(error, ErrorMitigationError::EmptyNoisyResults),
        "got {error:?}"
    );

    // 长度不匹配：需 3 个点，只给 2 个
    let error = zne
        .poly_extrapolate(&[1.0, 2.0], 1)
        .expect_err("length mismatch");
    println!("[8] poly_extrapolate([1.0, 2.0]) -> Err: {error}");
    assert!(
        matches!(
            error,
            ErrorMitigationError::NoisyResultsLengthMismatch { expected: 3, actual: 2 }
        ),
        "got {error:?}"
    );

    // 多项式阶数必须小于数据点数
    let error = zne
        .poly_extrapolate(&[1.0, 2.0, 3.0], 3)
        .expect_err("degree too large");
    println!("[8] poly_extrapolate(deg=3, points=3) -> Err: {error}");
    assert!(
        matches!(
            error,
            ErrorMitigationError::InvalidPolynomialDegree { degree: 3, num_points: 3 }
        ),
        "got {error:?}"
    );
}

#[test]
fn zne_exp_extrapolate_rejects_non_positive_results() {
    let zne = ZNEMitigation::new(bell_circuit(), vec![0, 1, 2]);

    // 指数外推需对样本取对数，非正输入必须被拒绝
    let error = zne
        .exp_extrapolate(&[0.5, -0.25, 0.1])
        .expect_err("non-positive input");
    println!("[9] exp_extrapolate([0.5, -0.25, 0.1]) -> Err: {error}");
    assert!(
        matches!(error, ErrorMitigationError::NonPositiveNoisyResults),
        "got {error:?}"
    );
}

#[test]
fn zne_poly_extrapolate_recovers_exact_linear_model() {
    let zne = ZNEMitigation::new(bell_circuit(), vec![0, 1, 2]); // λ = [1, 3, 5]

    // 严格线性模型 y = 4 − 0.7λ → [3.3, 1.9, 0.5]， λ→0 处截距恰为 4
    let data = [3.3, 1.9, 0.5];

    let degree1 = zne.poly_extrapolate(&data, 1).expect("degree-1 fit");
    assert!(
        (degree1 - 4.0).abs() < 1e-12,
        "degree-1 fit gives {degree1}, want 4.0"
    );

    // 对恰好共线的数据，阶数 2 拟合收敛到同一截距
    let degree2 = zne.poly_extrapolate(&data, 2).expect("degree-2 fit");
    assert!(
        (degree2 - 4.0).abs() < 1e-8,
        "degree-2 fit gives {degree2}, want 4.0"
    );

    println!("[10] poly_extrapolate(data={data:?}, deg=1) = {degree1:.15} (期望 4.0)");
    println!("[10] poly_extrapolate(data={data:?}, deg=2) = {degree2:.15} (期望 4.0)");
}

#[test]
fn zne_exp_extrapolate_recovers_known_amplitudes() {
    let zne = ZNEMitigation::new(bell_circuit(), vec![0, 1, 2]); // λ = [1, 3, 5]

    // 模型 y = 2.5·exp(−λ/3)：对数空间线性回归应精确恢复振幅 2.5
    let tau: f64 = 3.0;

    let data: Vec<f64> = [1.0, 3.0, 5.0]
        .iter()
        .map(|&lambda| 2.5 * (-lambda / tau).exp())
        .collect();
    let fitted = zne
        .extrapolate(&data, ExtrapolateMethod::Exponential, 0)
        .expect("exponential fit");
    assert!(
        (fitted - 2.5).abs() < 1e-9,
        "exp fit recovers {fitted}, want 2.5"
    );

    // 常数序列：斜率退化为 0，截距即振幅本身
    let constant = zne
        .exp_extrapolate(&[1.2, 1.2, 1.2])
        .expect("constant-sequence fit");
    assert!(
        (constant - 1.2).abs() < 1e-12,
        "constant fit recovers {constant}, want 1.2"
    );

    println!("[11] exp_extrapolate(data={data:?}) = {fitted:.15} (期望 2.5)");
    println!("[11] exp_extrapolate([1.2, 1.2, 1.2]) = {constant:.15} (期望 1.2)");
}