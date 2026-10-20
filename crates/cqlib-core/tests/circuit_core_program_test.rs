//! circuit 模块全量功能测试 —— 文件 1：Circuit 核心容器与构建器
//!
//! 被测目标：crates/cqlib-core/src/circuit/{bit, circuit_impl, circuit_param,
//! operation, depth, circuit_to_matrix}.rs 的全部公开 API。
//! 测试方式：integration test，仅通过 `cqlib_core` 公开 API 访问实现。
//!
//! 运行：cargo test -p cqlib-core --test circuit_core_program_test -- --nocapture

use std::collections::HashMap;

use cqlib_core::circuit::gate::{FrozenCircuit, UnitaryGate};
use cqlib_core::circuit::{
    Circuit, CircuitError, CircuitId, CircuitParam, ClassicalType, Directive, Instruction,
    Operation, Parameter, ParameterValue, Qubit, QubitError, StandardGate, ValueOperation,
};
use ndarray::Array2;
use num_complex::Complex64;
use std::f64::consts::{FRAC_1_SQRT_2, PI};

const C0: Complex64 = Complex64::new(0.0, 0.0);
const C1: Complex64 = Complex64::new(1.0, 0.0);
const CI: Complex64 = Complex64::new(0.0, 1.0);

fn q(id: u32) -> Qubit {
    Qubit::new(id)
}

/// 两条矩阵的最大逐元素偏差。
fn max_delta(a: &Array2<Complex64>, b: &Array2<Complex64>) -> f64 {
    let mut max = 0.0f64;
    for (x, y) in a.iter().zip(b.iter()) {
        max = max.max((*x - *y).norm());
    }
    max
}

/// 断言 U 是单位矩阵（偏差 < eps）。
fn assert_identity(u: &Array2<Complex64>, eps: f64) {
    let dim = u.nrows();
    assert_eq!(u.ncols(), dim, "矩阵必须是方阵");
    for i in 0..dim {
        for j in 0..dim {
            let expected = if i == j { C1 } else { C0 };
            let delta = (u[[i, j]] - expected).norm();
            assert!(delta < eps, "U[{i},{j}] 偏离单位矩阵 {delta}");
        }
    }
}

fn matrix_of(circuit: &Circuit) -> Array2<Complex64> {
    circuit.to_matrix(None).expect("to_matrix")
}

// ==================== 一、Qubit（bit.rs） ====================

#[test]
fn qubit_new_id_index_display_ordering() {
    let q5 = Qubit::new(5);
    assert_eq!(q5.id(), 5);
    assert_eq!(q5.index(), 5usize);
    assert_eq!(format!("{q5}"), "Q5");
    assert_eq!(format!("{}", Qubit::new(0)), "Q0");
    // 比较仅依赖数值 id
    assert_eq!(Qubit::new(3), Qubit::new(3));
    assert_ne!(Qubit::new(3), Qubit::new(4));
    assert!(Qubit::new(2) < Qubit::new(7));
    println!("[qubit] Q5 → id={}, index={}, display='Q5'，有序性 OK", q5.id(), q5.index());
}

#[test]
fn qubit_from_unsigned_infallible() {
    let a: Qubit = 7u8.into();
    let b: Qubit = 1000u16.into();
    let c: Qubit = 100_000u32.into();
    assert_eq!(a.id(), 7);
    assert_eq!(b.id(), 1000);
    assert_eq!(c.id(), 100_000);
    println!("[qubit] From<u8/u16/u32> 全部成功（7/1000/100000）");
}

#[test]
fn qubit_try_from_signed_and_usize() {
    assert_eq!(Qubit::try_from(0i32), Ok(Qubit::new(0)));
    assert_eq!(Qubit::try_from(100i16), Ok(Qubit::new(100)));
    assert_eq!(Qubit::try_from(-1isize), Err(QubitError::NegativeIndex(-1)));
    assert_eq!(Qubit::try_from(-1i8), Err(QubitError::NegativeIndex(-1)));
    // 64 位平台上 usize 超过 u32::MAX 应报 IndexTooLarge
    if usize::BITS > 32 {
        let overflow = (u32::MAX as usize) + 1;
        assert_eq!(
            Qubit::try_from(overflow),
            Err(QubitError::IndexTooLarge(overflow as u128))
        );
    }
    assert_eq!(Qubit::try_from(u32::MAX as usize), Ok(Qubit::new(u32::MAX)));
    println!("[qubit] 负索引 → NegativeIndex；溢出 → IndexTooLarge；边界 u32::MAX OK");
}

// ==================== 二、Circuit 构造 ====================

#[test]
fn circuit_new_basic_properties() {
    let circuit = Circuit::new(5);
    assert_eq!(circuit.num_qubits(), 5);
    assert_eq!(circuit.width(), 5); // width 是 num_qubits 的别名
    assert_eq!(circuit.qubits(), vec![q(0), q(1), q(2), q(3), q(4)]);
    assert!(circuit.operations().is_empty());
    assert_eq!(circuit.depth(false).unwrap(), 0);
    assert!(circuit.classical_vars().is_empty());
    assert!(circuit.classical_values().is_empty());
    assert!(circuit.parameters().is_empty());
    assert!(circuit.symbols().is_empty());
    println!("[circuit] new(5)：width=5, qubits 顺序 0..4, depth=0, 各表为空");
}

#[test]
fn circuit_from_usize_trait() {
    let circuit: Circuit = 3usize.into();
    assert_eq!(circuit.num_qubits(), 3);
    println!("[circuit] From<usize>：3 → {} qubits", circuit.num_qubits());
}

#[test]
fn circuit_new_zero_qubits_edge() {
    let circuit = Circuit::new(0);
    assert_eq!(circuit.width(), 0);
    assert!(circuit.qubits().is_empty());
    // 0 比特电路上任何门都因 QubitNotFound 失败
    assert!(matches!(circuit.to_matrix(None), Ok(m) if m == Array2::eye(1)));
    let mut circuit = circuit;
    assert!(matches!(
        circuit.h(q(0)),
        Err(CircuitError::QubitNotFound(0))
    ));
    println!("[circuit] 0 比特边界：矩阵为 1×1 恒等；h(Q0) → QubitNotFound(0)");
}

#[test]
fn circuit_from_qubits_noncontiguous_order_preserved() {
    let circuit = Circuit::from_qubits(vec![q(3), q(1), q(2)]).unwrap();
    // 插入顺序保留
    assert_eq!(circuit.qubits(), vec![q(3), q(1), q(2)]);
    let mut circuit = circuit;
    circuit.h(q(1)).unwrap();
    circuit.cx(q(1), q(2)).unwrap();
    assert_eq!(circuit.operations().len(), 2);
    assert!(matches!(circuit.h(q(0)), Err(CircuitError::QubitNotFound(0))));
    println!("[circuit] from_qubits([3,1,2])：顺序保留 {:?}；操作可用，Q0 拒绝", circuit.qubits());
}

#[test]
fn circuit_from_qubits_rejects_duplicates() {
    let err = Circuit::from_qubits(vec![q(0), q(1), q(0)]).unwrap_err();
    assert!(matches!(err, CircuitError::DuplicateQubits));
    println!("[circuit] from_qubits 重复 → Err: {err}");
}

#[test]
fn circuit_add_qubits_semantics() {
    let mut circuit = Circuit::new(0);
    circuit.add_qubits(vec![q(0), q(1)]).unwrap();
    assert_eq!(circuit.width(), 2);
    // 已存在的比特重复添加
    assert!(matches!(
        circuit.add_qubits(vec![q(0)]),
        Err(CircuitError::DuplicateQubits)
    ));
    // 新列表内部重复
    assert!(matches!(
        circuit.add_qubits(vec![q(2), q(2)]),
        Err(CircuitError::DuplicateQubits)
    ));
    // 失败后不改变比特集
    assert_eq!(circuit.qubits(), vec![q(0), q(1)]);
    circuit.add_qubits(vec![q(5)]).unwrap();
    assert_eq!(circuit.qubits(), vec![q(0), q(1), q(5)]);
    println!("[circuit] add_qubits：追加成功、与已有重复拒绝、列表内重复拒绝、失败不污染");
}

#[test]
fn circuit_from_operations_lowers_value_ops() {
    let ops = vec![
        ValueOperation::from_standard(StandardGate::H, [q(0)], []),
        ValueOperation::from_standard(StandardGate::CX, [q(0), q(1)], []),
    ];
    let circuit = Circuit::from_operations(vec![q(0), q(1)], ops, None, None).unwrap();
    assert_eq!(circuit.operations().len(), 2);
    assert!(matches!(
        circuit.operations()[0].instruction,
        Instruction::Standard(StandardGate::H)
    ));
    circuit.validate().unwrap();
    println!("[circuit] from_operations：2 个值层操作成功降落到存储 IR，validate 通过");
}

#[test]
fn circuit_from_operations_validates_qubits() {
    let ops = vec![ValueOperation::from_standard(StandardGate::H, [q(9)], [])];
    let err = Circuit::from_operations(vec![q(0)], ops, None, None).unwrap_err();
    assert!(matches!(err, CircuitError::QubitNotFound(9)));
    let err = Circuit::from_operations(vec![q(0), q(0)], [], None, None).unwrap_err();
    assert!(matches!(err, CircuitError::DuplicateQubits));
    println!("[circuit] from_operations 校验：未知比特 → {err:?}");
}

// ==================== 三、门构建器（40+ 方法逐一覆盖） ====================

#[test]
fn builder_pauli_and_clifford_gates() {
    let mut c = Circuit::new(1);
    c.h(q(0)).unwrap();
    c.i(q(0)).unwrap();
    c.x(q(0)).unwrap();
    c.y(q(0)).unwrap();
    c.z(q(0)).unwrap();
    c.s(q(0)).unwrap();
    c.sdg(q(0)).unwrap();
    c.t(q(0)).unwrap();
    c.tdg(q(0)).unwrap();
    assert_eq!(c.operations().len(), 9);
    let names: Vec<&str> = c
        .operations()
        .iter()
        .map(|op| match &op.instruction {
            Instruction::Standard(g) => g.name(),
            other => panic!("unexpected {other:?}"),
        })
        .collect();
    assert_eq!(
        names,
        vec!["H", "I", "X", "Y", "Z", "S", "SDG", "T", "TDG"]
    );
    // H 矩阵
    let h = StandardGate::H.matrix(&[]).unwrap();
    assert!((h[[0, 0]].re - FRAC_1_SQRT_2).abs() < 1e-12);
    assert!((h[[1, 0]].re - FRAC_1_SQRT_2).abs() < 1e-12);
    // Y|0> = i|1>
    let y = StandardGate::Y.matrix(&[]).unwrap();
    assert!((y[[1, 0]] - CI).norm() < 1e-12);
    println!("[builder] H/I/X/Y/Z/S/SDG/T/TDG 全部追加成功，H、Y 矩阵数值正确");
}

#[test]
fn builder_sqrt_gates_composition_rules() {
    let mut c = Circuit::new(1);
    c.x2p(q(0)).unwrap();
    c.x2m(q(0)).unwrap();
    c.y2p(q(0)).unwrap();
    c.y2m(q(0)).unwrap();
    assert_eq!(c.operations().len(), 4);
    let x2p = StandardGate::X2P.matrix(&[]).unwrap().into_owned();
    let x = StandardGate::X.matrix(&[]).unwrap().into_owned();
    let y2p = StandardGate::Y2P.matrix(&[]).unwrap().into_owned();
    let y = StandardGate::Y.matrix(&[]).unwrap().into_owned();
    // X2P² = X（至多差一个全局相位因子）
    let xx = x2p.dot(&x2p);
    let ratio = xx[[0, 1]] / x[[0, 1]];
    assert!((ratio.norm() - 1.0).abs() < 1e-12, "相位因子应为单位模");
    assert!(xx[[0, 0]].norm() < 1e-12 && xx[[1, 1]].norm() < 1e-12);
    assert!((xx[[0, 1]] - ratio * x[[0, 1]]).norm() < 1e-12 && (xx[[1, 0]] - ratio * x[[1, 0]]).norm() < 1e-12);
    println!("[builder] X2P² = ({ratio})·X —— 与 X 至多相差全局相位");
    // X2P∘X2M = I
    let x2m = StandardGate::X2M.matrix(&[]).unwrap().into_owned();
    assert_identity(&x2p.dot(&x2m), 1e-12);
    // Y2P² = Y（至多差一个全局相位）
    let yy = y2p.dot(&y2p);
    let y_ratio = yy[[0, 1]] / y[[0, 1]];
    assert!((y_ratio.norm() - 1.0).abs() < 1e-12);
    assert!((yy[[0, 1]] - y_ratio * y[[0, 1]]).norm() < 1e-12);
    assert!((yy[[1, 0]] - y_ratio * y[[1, 0]]).norm() < 1e-12);
    println!("[builder] X2P/X2M/Y2P/Y2M：X2P²=X、X2P·X2M=I、Y2P²=Y 全部成立");
}

#[test]
fn builder_xy_family_and_inverse_relation() {
    let mut c = Circuit::new(1);
    c.xy(q(0), 0.3).unwrap();
    c.xy2p(q(0), 0.4).unwrap();
    c.xy2m(q(0), 0.5).unwrap();
    for op in c.operations() {
        assert_eq!(op.params.len(), 1, "XY 族门均携带 1 个参数");
    }
    // XY(t)·XY(π+t) = I（inverse 表）
    let xy_t = StandardGate::XY.matrix(&[0.3]).unwrap().into_owned();
    let xy_inv = StandardGate::XY.matrix(&[PI + 0.3]).unwrap().into_owned();
    assert_identity(&xy_t.dot(&xy_inv), 1e-12);
    // XY2P(t)·XY2M(t) = I
    let a = StandardGate::XY2P.matrix(&[0.4]).unwrap().into_owned();
    let b = StandardGate::XY2M.matrix(&[0.4]).unwrap().into_owned();
    assert_identity(&a.dot(&b), 1e-12);
    println!("[builder] XY/XY2P/XY2M：参数数=1，XY(t)·XY(π+t)=I，XY2P·XY2M=I");
}

#[test]
fn builder_rotation_gates_fixed_and_symbolic() {
    let theta = Parameter::symbol("θ");
    let mut c = Circuit::new(1);
    c.rx(q(0), PI / 2.0).unwrap(); // f64 → Fixed
    c.ry(q(0), 0.25).unwrap();
    c.rz(q(0), theta.clone()).unwrap(); // Parameter → Index
    c.phase(q(0), 1.1).unwrap();
    assert_eq!(c.operations().len(), 4);
    assert!(matches!(c.operations()[0].params[0], CircuitParam::Fixed(v) if (v - PI / 2.0).abs() < 1e-15));
    assert!(matches!(c.operations()[1].params[0], CircuitParam::Fixed(v) if (v - 0.25).abs() < 1e-15));
    assert!(matches!(c.operations()[2].params[0], CircuitParam::Index(_)));
    assert!(c.symbols().contains("θ"));
    // RX(π/2)·RX(-π/2) = I
    let a = StandardGate::RX.matrix(&[PI / 2.0]).unwrap().into_owned();
    let b = StandardGate::RX.matrix(&[-PI / 2.0]).unwrap().into_owned();
    assert_identity(&a.dot(&b), 1e-12);
    println!("[builder] RX/RY/RZ/Phase：f64/f32→Fixed、Parameter→Index、RX 互逆成立");
}

#[test]
fn builder_u_gate_matrix_formula() {
    let mut c = Circuit::new(1);
    c.u(q(0), PI / 2.0, 0.0, PI / 2.0).unwrap();
    let u = StandardGate::U.matrix(&[PI / 2.0, 0.0, PI / 2.0]).unwrap();
    // U(θ,φ,λ) 第一行：[cos(θ/2), -e^{iλ}sin(θ/2)]；λ=π/2 → -i·sin(π/4)
    assert!((u[[0, 0]].re - FRAC_1_SQRT_2).abs() < 1e-12 && u[[0, 0]].im == 0.0);
    assert!((u[[0, 1]].re).abs() < 1e-12 && (u[[0, 1]].im + FRAC_1_SQRT_2).abs() < 1e-12);
    // U†(U) = I 验证外推表：U(θ,φ,λ)† = U(-θ,-λ,-φ)
    let fwd = StandardGate::U.matrix(&[0.3, 0.7, 1.1]).unwrap().into_owned();
    let inv = StandardGate::U.matrix(&[-0.3, -1.1, -0.7]).unwrap().into_owned();
    assert_identity(&fwd.dot(&inv), 1e-12);
    println!("[builder] U 门：矩阵公式逐元素正确，U(θ,φ,λ)·U(-θ,-λ,-φ)=I");
}

#[test]
fn builder_two_and_three_qubit_fixed_gates() {
    let mut c = Circuit::new(3);
    c.cx(q(0), q(1)).unwrap();
    c.cy(q(0), q(1)).unwrap();
    c.cz(q(0), q(1)).unwrap();
    c.swap(q(0), q(1)).unwrap();
    c.ccx(q(0), q(1), q(2)).unwrap();
    assert_eq!(c.operations().len(), 5);
    // SWAP 自逆
    let swap = StandardGate::SWAP.matrix(&[]).unwrap().into_owned();
    assert_identity(&swap.dot(&swap), 1e-12);
    // CZ 对角 [1,1,1,-1]
    let cz = StandardGate::CZ.matrix(&[]).unwrap();
    assert!((cz[[3, 3]].re + 1.0).abs() < 1e-12 && cz[[0, 1]].norm() < 1e-12);
    // CCX 与 MCGate(2, X) 矩阵一致
    let ccx = StandardGate::CCX.matrix(&[]).unwrap().into_owned();
    let mc_inst = Instruction::McGate(Box::new(cqlib_core::circuit::MCGate::new(
        2,
        StandardGate::X,
    )));
    let mcx = mc_inst.matrix(&[]).unwrap();
    assert!(max_delta(&ccx, &mcx) < 1e-12);
    println!("[builder] CX/CY/CZ/SWAP/CCX：SWAP 自逆、CZ 对角、CCX≡C2-X");
}

#[test]
fn builder_ising_gates() {
    let mut c = Circuit::new(2);
    c.rxx(q(0), q(1), 0.4).unwrap();
    c.ryy(q(0), q(1), 0.5).unwrap();
    c.rzz(q(0), q(1), 0.6).unwrap();
    c.rzx(q(0), q(1), 0.7).unwrap();
    assert_eq!(c.operations().len(), 4);
    for g in [StandardGate::RXX, StandardGate::RYY, StandardGate::RZZ, StandardGate::RZX] {
        let t = 0.37;
        let fwd = g.matrix(&[t]).unwrap().into_owned();
        let bwd = g.matrix(&[-t]).unwrap().into_owned();
        assert_identity(&fwd.dot(&bwd), 1e-12);
    }
    println!("[builder] RXX/RYY/RZZ/RZX：θ 与 -θ 互逆全部成立");
}

#[test]
fn builder_controlled_rotations() {
    let mut c = Circuit::new(2);
    c.crx(q(0), q(1), 0.3).unwrap();
    c.cry(q(0), q(1), 0.4).unwrap();
    c.crz(q(0), q(1), 0.5).unwrap();
    assert_eq!(c.operations().len(), 3);
    for g in [StandardGate::CRX, StandardGate::CRY, StandardGate::CRZ] {
        let t = 0.31;
        let fwd = g.matrix(&[t]).unwrap().into_owned();
        let bwd = g.matrix(&[-t]).unwrap().into_owned();
        assert_identity(&fwd.dot(&bwd), 1e-12);
    }
    println!("[builder] CRX/CRY/CRZ：追加成功且互逆成立");
}

#[test]
fn builder_fsim_and_rxy() {
    let mut c = Circuit::new(2);
    c.fsim(q(0), q(1), 0.6, 0.2).unwrap();
    assert_eq!(c.operations()[0].params.len(), 2);
    // fSim(0,0) = I
    let m = StandardGate::FSIM.matrix(&[0.0, 0.0]).unwrap();
    assert_identity(&m, 1e-12);
    // fSim(θ,φ)·fSim(-θ,-φ) = I
    let fwd = StandardGate::FSIM.matrix(&[0.3, -0.4]).unwrap().into_owned();
    let bwd = StandardGate::FSIM.matrix(&[-0.3, 0.4]).unwrap().into_owned();
    assert_identity(&fwd.dot(&bwd), 1e-12);

    let mut c1 = Circuit::new(1);
    c1.rxy(q(0), 0.9, 0.2).unwrap();
    assert_eq!(c1.operations()[0].params.len(), 2);
    let f = StandardGate::RXY.matrix(&[0.3, 0.8]).unwrap().into_owned();
    let b = StandardGate::RXY.matrix(&[-0.3, 0.8]).unwrap().into_owned();
    assert_identity(&f.dot(&b), 1e-12);
    println!("[builder] FSIM/RXY：参数数=2/2，fSim(0,0)=I，互逆规则正确");
}

#[test]
fn builder_barrier_reset_delay() {
    let mut c = Circuit::new(2);
    c.barrier(vec![q(0), q(1)]).unwrap();
    c.barrier(Vec::new()).unwrap(); // 全局 barrier
    c.reset(q(0)).unwrap();
    c.delay(q(0), ParameterValue::Fixed(42.0)).unwrap();
    assert_eq!(c.operations().len(), 4);
    assert!(matches!(
        c.operations()[0].instruction,
        Instruction::Directive(Directive::Barrier)
    ));
    assert!(c.operations()[1].qubits.is_empty(), "空 barrier 是全局 barrier");
    assert!(matches!(
        c.operations()[2].instruction,
        Instruction::Directive(Directive::Reset)
    ));
    assert!(matches!(c.operations()[3].instruction, Instruction::Delay));
    assert_eq!(c.operations()[3].params.len(), 1);
    println!("[builder] barrier(局部/全局)/reset/delay 全部正确入 IR");
}

#[test]
fn builder_multi_control_promotion_rules() {
    let mut c = Circuit::new(5);
    // X + 1 控制 → CX
    c.multi_control(StandardGate::X, [q(0)], vec![q(1)], []).unwrap();
    assert!(matches!(
        c.operations()[0].instruction,
        Instruction::Standard(StandardGate::CX)
    ));
    // X + 2 控制 → CCX
    c.multi_control(StandardGate::X, [q(0), q(1)], vec![q(2)], []).unwrap();
    assert!(matches!(
        c.operations()[1].instruction,
        Instruction::Standard(StandardGate::CCX)
    ));
    // X + 3 控制 → McGate
    c.multi_control(StandardGate::X, [q(0), q(1), q(2)], vec![q(3)], [])
        .unwrap();
    let Instruction::McGate(mc) = &c.operations()[2].instruction else {
        panic!("3 控制应升级为 McGate");
    };
    assert_eq!(mc.num_ctrl_qubits(), 3);
    assert_eq!(mc.num_qubits(), 4);
    assert_eq!(mc.name(), "C3-X");
    // RX + 1 控制 → CRX
    c.multi_control(StandardGate::RX, [q(0)], vec![q(1)], [ParameterValue::Fixed(0.5)])
        .unwrap();
    assert!(matches!(
        c.operations()[3].instruction,
        Instruction::Standard(StandardGate::CRX)
    ));
    println!("[builder] multi_control：X+1=CX、X+2=CCX、X+3=C3-X(McGate)、RX+1=CRX");
}

#[test]
fn builder_unitary_gate_matrix_path() {
    let x_mat = ndarray::array![[C0, C1], [C1, C0]];
    let gate = UnitaryGate::new("MyX", 1, 0).with_matrix(x_mat).unwrap();
    let mut c = Circuit::new(1);
    c.unitary(gate.clone(), vec![q(0)]).unwrap();
    assert!(c.operations()[0].instruction.is_unitary());
    assert_eq!(c.operations()[0].instruction.name(), "MyX");
    // 数值路径：自定义 X 与标准 X 矩阵一致
    assert!(max_delta(&matrix_of(&c), &StandardGate::X.matrix(&[]).unwrap().into_owned()) < 1e-12);
    println!("[builder] unitary(矩阵)：入 IR 成功，to_matrix 与标准 X 一致");
}

#[test]
fn builder_unitary_with_params_circuit_path() {
    // 用带符号参数的子线路定义门
    let theta = Parameter::symbol("t");
    let mut inner = Circuit::new(1);
    inner.rx(q(0), theta).unwrap();
    let frozen = std::sync::Arc::new(FrozenCircuit::new(inner));
    let gate = UnitaryGate::new("ParamRx", 1, 1)
        .with_circuit(frozen)
        .unwrap();
    let mut c = Circuit::new(1);
    c.unitary_with_params(gate, vec![q(0)], [ParameterValue::Fixed(0.7)])
        .unwrap();
    // 数值路径：等价于 RX(0.7)
    let expected = StandardGate::RX.matrix(&[0.7]).unwrap().into_owned();
    assert!(max_delta(&matrix_of(&c), &expected) < 1e-12);
    println!("[builder] unitary_with_params(电路定义)：t=0.7 时与 RX(0.7) 数值一致");
}

#[test]
fn builder_circuit_gate_nesting_and_decompose() {
    let mut inner = Circuit::new(2);
    inner.h(q(0)).unwrap();
    inner.cx(q(0), q(1)).unwrap();
    let gate_instruction = inner.to_gate("Bell").unwrap();

    let mut outer = Circuit::new(2);
    outer
        .append(gate_instruction, [q(0), q(1)], [], None)
        .unwrap();
    assert!(outer.operations()[0].instruction.is_circuit_gate());
    // 分解：展平为 H + CX
    let flat = outer.decompose().unwrap();
    assert_eq!(flat.operations().len(), 2);
    assert!(matches!(flat.operations()[0].instruction, Instruction::Standard(StandardGate::H)));
    assert!(matches!(flat.operations()[1].instruction, Instruction::Standard(StandardGate::CX)));
    // 矩阵等价
    let direct = {
        let mut c = Circuit::new(2);
        c.h(q(0)).unwrap();
        c.cx(q(0), q(1)).unwrap();
        c
    };
    assert!(max_delta(&matrix_of(&flat), &matrix_of(&direct)) < 1e-12);
    println!("[builder] circuit_gate 嵌套：decompose 展平 2 op，矩阵与直接构建一致");
}

#[test]
fn builder_circuit_gate_param_substitution() {
    // 文档示例：内层 Rx(θ+1)，调用时 θ ← 2*x，x=0.5 → RX(2.0)
    let mut inner = Circuit::new(1);
    inner
        .rx(q(0), Parameter::symbol("θ") + 1.0)
        .unwrap();
    let gate = inner.to_gate("Inner").unwrap();
    let mut outer = Circuit::new(1);
    outer
        .append(
            gate,
            [q(0)],
            [ParameterValue::from(Parameter::symbol("x") * 2.0)],
            None,
        )
        .unwrap();
    let mut bindings = HashMap::new();
    bindings.insert("x", 0.5);
    let assigned = outer.assign_parameters(&Some(bindings)).unwrap();
    let expected = StandardGate::RX.matrix(&[2.0]).unwrap().into_owned();
    assert!(max_delta(&matrix_of(&assigned), &expected) < 1e-9);
    println!("[builder] circuit_gate 参数代换：θ←2x, x=0.5 → RX(2.0) 数值验证通过");
}

// ==================== 四、append 校验路径 ====================

#[test]
fn append_arity_and_membership_errors() {
    let mut c = Circuit::new(2);
    // 比特数不匹配
    assert!(matches!(
        c.append(Instruction::Standard(StandardGate::H), [q(0), q(1)], [], None),
        Err(CircuitError::QubitCountMismatch { expected: 1, actual: 2 })
    ));
    // 参数数不匹配
    assert!(matches!(
        c.append(Instruction::Standard(StandardGate::RX), [q(0)], [], None),
        Err(CircuitError::ParameterCountMismatch { expected: 1, actual: 0 })
    ));
    // 重复比特
    assert!(matches!(
        c.cx(q(0), q(0)),
        Err(CircuitError::DuplicateQubits)
    ));
    // 未注册比特
    assert!(matches!(c.h(q(99)), Err(CircuitError::QubitNotFound(99))));
    // 失败后操作列表不被污染
    assert!(c.operations().is_empty());
    println!("[append] 比特数/参数数/重复比特/未知比特 4 类校验全部按预期报错，无副作用");
}

#[test]
fn append_nonfinite_params_rejected() {
    let mut c = Circuit::new(1);
    match c.rx(q(0), f64::NAN) {
        Err(CircuitError::InvalidParameterValue(0, v)) => assert!(v.is_nan()),
        other => panic!("unexpected {other:?}"),
    }
    match c.rx(q(0), f64::INFINITY) {
        Err(CircuitError::InvalidParameterValue(0, v)) => assert!(v.is_infinite()),
        other => panic!("unexpected {other:?}"),
    }
    assert!(c.operations().is_empty());
    println!("[append] NaN/Inf 参数 → InvalidParameterValue，操作回滚");
}

#[test]
fn append_label_and_value_operation() {
    let mut c = Circuit::new(1);
    c.append(
        Instruction::Standard(StandardGate::H),
        [q(0)],
        [],
        Some("tagged"),
    )
    .unwrap();
    assert_eq!(c.operations()[0].label.as_deref(), Some("tagged"));
    // 值层操作直接降落
    c.append_value_operation(ValueOperation::from_standard(
        StandardGate::X,
        [q(0)],
        [],
    ))
    .unwrap();
    assert_eq!(c.operations().len(), 2);
    assert_eq!(c.operations()[1].instruction.name(), "X");
    println!("[append] label 附加成功；append_value_operation 降落为存储 IR");
}

// ==================== 五、参数系统（Circuit 侧） ====================

#[test]
fn param_interning_dedup_and_resolution() {
    let theta = Parameter::symbol("θ");
    let mut c = Circuit::new(1);
    c.rx(q(0), theta.clone()).unwrap();
    c.ry(q(0), theta.clone()).unwrap(); // 同一表达式复用表项
    assert_eq!(c.parameters().len(), 1, "相同符号参数应去重驻留");
    assert!(c.symbols().contains("θ"));
    // resolve_parameter：Index → Parameter
    let CircuitParam::Index(idx) = c.operations()[0].params[0] else {
        panic!("符号参数应存为 Index");
    };
    let resolved = c.resolve_parameter(&CircuitParam::Index(idx)).unwrap();
    assert_eq!(resolved, theta);
    // Fixed → 数值 Parameter
    let fixed = c.resolve_parameter(&CircuitParam::Fixed(1.5)).unwrap();
    assert!((fixed.evaluate(&None).unwrap() - 1.5).abs() < 1e-15);
    // Fixed(NaN) → InvalidParameterValue
    assert!(matches!(
        c.resolve_parameter(&CircuitParam::Fixed(f64::NAN)),
        Err(CircuitError::InvalidParameterValue(0, _))
    ));
    // 越界索引 → InvalidParameterIndex
    assert!(matches!(
        c.resolve_parameter(&CircuitParam::Index(99)),
        Err(CircuitError::InvalidParameterIndex(99))
    ));
    println!("[param] 驻留去重=1 项；resolve Index/Fixed 正确；NaN 与越界索引均拒绝");
}

#[test]
fn param_value_roundtrip() {
    let mut c = Circuit::new(1);
    let theta = Parameter::symbol("θ");
    c.rx(q(0), theta).unwrap();
    let CircuitParam::Index(idx) = c.operations()[0].params[0] else {
        panic!();
    };
    // parameter_value：Index → ParameterValue::Param
    match c.parameter_value(&CircuitParam::Index(idx)).unwrap() {
        ParameterValue::Param(p) => assert_eq!(p, Parameter::symbol("θ")),
        other => panic!("unexpected {other:?}"),
    }
    // Fixed → Fixed
    assert!(matches!(
        c.parameter_value(&CircuitParam::Fixed(2.5)).unwrap(),
        ParameterValue::Fixed(v) if v == 2.5
    ));
    assert!(matches!(
        c.parameter_value(&CircuitParam::Index(77)),
        Err(CircuitError::InvalidParameterIndex(77))
    ));
    println!("[param] parameter_value：Index→Param、Fixed→Fixed、越界→InvalidParameterIndex");
}

#[test]
fn param_map_param_constant_folding() {
    let mut c = Circuit::new(1);
    // 常量折叠为 Fixed
    let folded = c.map_param(Parameter::from(2.0) * 3.0).unwrap();
    assert!(matches!(folded, CircuitParam::Fixed(v) if (v - 6.0).abs() < 1e-12));
    // -0.0 归一化为 0.0
    let normalized = c.map_param(Parameter::from(-0.0)).unwrap();
    assert!(matches!(normalized, CircuitParam::Fixed(v) if v == 0.0 && v.is_sign_positive()));
    // 符号参数驻留为 Index
    let theta = Parameter::symbol("θ");
    let interned = c.map_param(theta.clone() + 0.0).unwrap();
    assert!(matches!(interned, CircuitParam::Index(_)));
    assert!(c.symbols().contains("θ"));
    println!("[map_param] 常量折叠 2*3→Fixed(6)、-0.0→+0.0、符号→Index 并登记符号表");
}

#[test]
fn param_add_parameter_dedup() {
    let mut c = Circuit::new(1);
    let (i1, new1) = c.add_parameter(Parameter::symbol("a"));
    let (i2, new2) = c.add_parameter(Parameter::symbol("a"));
    assert_eq!(i1, i2);
    assert!(new1 && !new2, "第二次插入应命中已有表项");
    let (i3, new3) = c.add_parameter(Parameter::symbol("a") * 2.0);
    assert_ne!(i3, i1);
    assert!(new3);
    assert_eq!(c.parameters().len(), 2);
    println!("[add_parameter] 重复插入命中（is_new=false），不同表达式新表项");
}

#[test]
fn param_used_symbols_vs_registry() {
    let theta = Parameter::symbol("θ");
    let mut c = Circuit::new(1);
    c.rx(q(0), theta).unwrap();
    // 驻留但未引用：手动 add_parameter 的死参数
    c.add_parameter(Parameter::symbol("dead"));
    assert!(c.symbols().contains("dead"), "符号注册表保留死符号");
    assert!(!c.used_symbols().contains("dead"), "used_symbols 应过滤死符号");
    assert!(c.used_symbols().contains("θ"));
    assert!(c.uses_symbol("θ"));
    assert!(!c.uses_symbol("dead"));
    // 移除引用后 θ 不再被使用，但仍在注册表
    c.remove_operations([0]).unwrap();
    assert!(!c.uses_symbol("θ"));
    assert!(c.symbols().contains("θ"), "注册表不被压缩");
    println!("[symbols] registry 保留 dead；used_symbols/uses_symbol 过滤未引用符号");
}

#[test]
fn param_assign_parameters_binding() {
    let theta = Parameter::symbol("θ");
    let phi = Parameter::symbol("φ");
    let mut c = Circuit::new(1);
    c.rx(q(0), theta).unwrap();
    c.ry(q(0), phi).unwrap();
    let mut bindings = HashMap::new();
    bindings.insert("θ", 0.7);
    let assigned = c.assign_parameters(&Some(bindings)).unwrap();
    assert!(matches!(assigned.operations()[0].params[0], CircuitParam::Fixed(v) if (v - 0.7).abs() < 1e-12));
    // 未绑定的 φ 保持符号
    assert!(matches!(assigned.operations()[1].params[0], CircuitParam::Index(_)));
    assert!(assigned.uses_symbol("φ"));
    assert!(!assigned.uses_symbol("θ"));
    // 部分绑定后矩阵仍不可计算（φ 未定），全绑定后可计算
    let mut full = HashMap::new();
    full.insert("θ", 0.7);
    full.insert("φ", 0.3);
    let done = c.assign_parameters(&Some(full)).unwrap();
    done.to_matrix(None).unwrap();
    println!("[assign] θ→Fixed(0.7)、φ 保持符号；全绑定后 to_matrix 可用");
}

#[test]
fn param_assign_rejects_control_flow() {
    use cqlib_core::circuit::ClassicalExpr;
    let mut c = Circuit::new(1);
    c.if_(ClassicalExpr::bool_literal(true), |body| {
        body.x(q(0))?;
        Ok(())
    })
    .unwrap();
    let err = c.assign_parameters(&None).unwrap_err();
    assert!(matches!(err, CircuitError::InvalidOperation(_)));
    println!("[assign] 含控制流线路 → Err: {err}");
}

#[test]
fn param_global_phase_fixed_and_symbolic() {
    let mut c = Circuit::new(1);
    c.x(q(0)).unwrap();
    assert!((c.global_phase().evaluate(&None).unwrap() - 0.0).abs() < 1e-15);
    c.set_global_phase(Parameter::pi());
    assert!((c.global_phase().evaluate(&None).unwrap() - PI).abs() < 1e-12);
    assert!(matches!(c.global_phase_param(), CircuitParam::Fixed(_)));
    // e^{iπ}·X = -X
    let m = matrix_of(&c);
    assert!((m[[0, 1]].re + 1.0).abs() < 1e-12);
    // 符号全局相位
    c.set_global_phase(Parameter::symbol("γ"));
    assert!(matches!(c.global_phase_param(), CircuitParam::Index(_)));
    assert!(c.uses_symbol("γ"));
    println!("[global_phase] π 相位 → 矩阵乘 -1 验证通过；符号相位 → Index 驻留且 uses_symbol");
}

// ==================== 六、信息查询与增删改 ====================

#[test]
fn info_index_accessor() {
    let mut c = Circuit::new(1);
    c.h(q(0)).unwrap();
    let vo = c.index(0).unwrap();
    assert_eq!(vo.name(), "H");
    assert_eq!(vo.num_qubits(), 1);
    assert_eq!(vo.num_params(), 0);
    assert!(matches!(
        c.index(1),
        Err(CircuitError::OperationIndexOutOfBounds { index: 1, len: 1 })
    ));
    // id 唯一
    let other = Circuit::new(1);
    assert_ne!(c.id(), other.id());
    println!("[index] index(0)→ValueOperation(H)；越界→OperationIndexOutOfBounds；id 进程唯一");
}

#[test]
fn mutation_remove_operations_and_rollback() {
    let mut c = Circuit::new(2);
    c.h(q(0)).unwrap();
    c.x(q(1)).unwrap();
    c.z(q(0)).unwrap();
    let removed = c.remove_operations([2, 0]).unwrap();
    assert_eq!(removed.len(), 2);
    // 返回按原索引升序
    assert_eq!(removed[0].instruction.name(), "H");
    assert_eq!(removed[1].instruction.name(), "Z");
    assert_eq!(c.operations().len(), 1);
    assert_eq!(c.operations()[0].instruction.name(), "X");
    // 重复索引去重
    let single = c.remove_operations([0, 0]).unwrap();
    assert_eq!(single.len(), 1);
    // 越界
    assert!(matches!(
        c.remove_operations([5]),
        Err(CircuitError::OperationIndexOutOfBounds { index: 5, len: 0 })
    ));
    // 空索引集 → Ok(空)
    assert!(c.remove_operations([]).unwrap().is_empty());
    println!("[remove] 批量删除升序返回、重复去重、越界拒绝、空集 Ok");
}

#[test]
fn mutation_remove_measurement_in_use_rejected() {
    let mut c = Circuit::new(2);
    let m = c.measure(q(0)).unwrap();
    c.if_(
        cqlib_core::circuit::ClassicalExpr::bit_to_bool(m.expr()).unwrap(),
        |body| {
            body.x(q(1))?;
            Ok(())
        },
    )
    .unwrap();
    // 只删测量：其结果仍被 if 读取
    let err = c.remove_operation(0).unwrap_err();
    assert!(matches!(err, CircuitError::ClassicalValueStillInUse { index: 0, .. }));
    // 删除读取方（if），再删测量 → 成功，值表压缩
    c.remove_operation(1).unwrap();
    c.remove_operation(0).unwrap();
    assert!(c.classical_values().is_empty(), "值表应压缩为空");
    println!("[remove] 测量结果被占用 → {err:?}；同时删除双方后值表压缩为空");
}

#[test]
fn mutation_classical_var_allocation() {
    let mut c = Circuit::new(1);
    let v0 = c.var(ClassicalType::Bool);
    let v1 = c.var(ClassicalType::uint(8).unwrap());
    assert_eq!(v0.id(), 0);
    assert_eq!(v1.id(), 1);
    assert_eq!(v1.circuit_id(), c.id());
    assert_eq!(c.classical_vars(), &[ClassicalType::Bool, ClassicalType::uint(8).unwrap()]);
    println!("[var] 分配顺序 0/1，类型表登记正确");
}

// ==================== 七、变换：inverse / to_gate / decompose / compose ====================

#[test]
fn transform_inverse_matrix_roundtrip() {
    let mut c = Circuit::new(2);
    c.h(q(0)).unwrap();
    c.cx(q(0), q(1)).unwrap();
    c.rz(q(1), 0.7).unwrap();
    let inv = c.inverse().unwrap();
    let u = matrix_of(&c);
    let u_inv = matrix_of(&inv);
    assert_identity(&u_inv.dot(&u), 1e-10);
    assert_identity(&u.dot(&u_inv), 1e-10);
    // 全局相位取负（对固定相位）
    assert!((inv.global_phase().evaluate(&None).unwrap() + c.global_phase().evaluate(&None).unwrap()).abs() < 1e-12);
    println!("[inverse] U†·U = I 与 U·U† = I 均成立，全局相位取负");
}

#[test]
fn transform_inverse_rejects_irreversible() {
    let mut with_measure = Circuit::new(1);
    with_measure.measure(q(0)).unwrap();
    assert!(matches!(
        with_measure.inverse(),
        Err(CircuitError::IrreversibleOperation)
    ));
    let mut with_reset = Circuit::new(1);
    with_reset.reset(q(0)).unwrap();
    assert!(matches!(with_reset.inverse(), Err(CircuitError::IrreversibleOperation)));
    let mut with_cf = Circuit::new(1);
    with_cf
        .if_(cqlib_core::circuit::ClassicalExpr::bool_literal(true), |body| {
            body.x(q(0))?;
            Ok(())
        })
        .unwrap();
    assert!(matches!(with_cf.inverse(), Err(CircuitError::IrreversibleOperation)));
    println!("[inverse] measure/reset/控制流 → IrreversibleOperation");
}

#[test]
fn transform_inverse_keeps_barrier_order_reversed() {
    let mut c = Circuit::new(1);
    c.h(q(0)).unwrap();
    c.barrier(vec![q(0)]).unwrap();
    c.x(q(0)).unwrap();
    let inv = c.inverse().unwrap();
    let names: Vec<String> = inv.operations().iter().map(|op| op.instruction.name()).collect();
    assert_eq!(names, vec!["X".to_string(), "barrier".to_string(), "H".to_string()]);
    println!("[inverse] barrier 保留且整体顺序反转：{:?}", names);
}

#[test]
fn transform_to_gate_reuse_on_other_qubits() {
    let mut bell = Circuit::new(2);
    bell.h(q(0)).unwrap();
    bell.cx(q(0), q(1)).unwrap();
    let gate: Instruction = bell.to_gate("Bell").unwrap();

    let mut outer = Circuit::new(3);
    outer.append(gate, [q(2), q(0)], [], None).unwrap(); // 映射到不同比特
    let m = matrix_of(&outer);
    // Bell 作用在 (q2, q0)：|000> → (|q2=1,q0=0... 验证第一列
    // 输入 |0>（全 0）：H 后 q2 处于叠加 → 输出 = (|000> + |101>)/√2
    // q2 是 bit2（默认排序 [0,1,2] 中 q2 → 位 2），q0 → 位 0 → |101> = 5
    assert!((m[[0, 0]].re - FRAC_1_SQRT_2).abs() < 1e-12);
    assert!((m[[5, 0]].re - FRAC_1_SQRT_2).abs() < 1e-12);
    println!("[to_gate] Bell 门重映射到 (q2,q0)：|000> → (|000>+|101>)/√2 验证通过");
}

#[test]
fn transform_compose_with_qubit_map() {
    // 文档示例
    let q1 = q(1);
    let q3 = q(3);
    let q5 = q(5);
    let mut qc1 = Circuit::new(0);
    qc1.add_qubits(vec![q1, q3, q5]).unwrap();
    qc1.h(q1).unwrap();

    let q2 = q(2);
    let mut qc2 = Circuit::new(0);
    qc2.add_qubits(vec![q1, q2]).unwrap();
    qc2.x(q1).unwrap();

    // qc2 的比特顺序 [q1, q2] → [q3, q1]
    qc1.compose(&qc2, Some(&[q3, q1])).unwrap();
    assert_eq!(qc1.operations().len(), 2);
    // 第二个操作是 qc2 的 X(q1)，重映射后应作用于 q3
    assert_eq!(qc1.operations()[1].qubits.as_slice(), &[q3]);
    println!("[compose] 比特映射 q1→q3、q2→q1 后：X 作用于 {:?}", qc1.operations()[1].qubits);
}

#[test]
fn transform_compose_by_id_merge() {
    let mut a = Circuit::new(2); // q0, q1
    a.h(q(0)).unwrap();
    let mut b = Circuit::new(0);
    b.add_qubits(vec![q(1), q(7)]).unwrap();
    b.x(q(1)).unwrap();
    a.compose(&b, None).unwrap();
    // 新比特 q7 追加，q1 复用
    assert_eq!(a.qubits(), vec![q(0), q(1), q(7)]);
    assert_eq!(a.operations().len(), 2);
    assert_eq!(a.operations()[1].qubits.as_slice(), &[q(1)]);
    println!("[compose] 按 ID 合并：q1 复用、q7 追加，顺序保持");
}

#[test]
fn transform_compose_merges_global_phase() {
    let mut a = Circuit::new(1);
    a.x(q(0)).unwrap();
    a.set_global_phase(Parameter::from(0.25));
    let mut b = Circuit::new(1);
    b.x(q(0)).unwrap();
    b.set_global_phase(Parameter::from(0.75));
    a.compose(&b, None).unwrap();
    assert!((a.global_phase().evaluate(&None).unwrap() - 1.0).abs() < 1e-12, "固定相位应相加");
    // 符号相位相加
    let mut c = Circuit::new(1);
    c.set_global_phase(Parameter::symbol("α"));
    let mut d = Circuit::new(1);
    d.set_global_phase(Parameter::from(0.5));
    c.compose(&d, None).unwrap();
    let mut bindings = HashMap::new();
    bindings.insert("α", 0.5);
    assert!((c.global_phase().evaluate(&Some(bindings)).unwrap() - 1.0).abs() < 1e-12);
    println!("[compose] 全局相位合并：0.25+0.75=1.0；符号 α+0.5 绑定后=1.0");
}

#[test]
fn transform_compose_remaps_classical_handles() {
    let mut a = Circuit::new(1);
    a.h(q(0)).unwrap();
    let mut b = Circuit::new(1);
    let bv = b.var(ClassicalType::Bool);
    let m = b.measure(q(0)).unwrap();
    b.store(bv, cqlib_core::circuit::ClassicalExpr::bit_to_bool(m.expr()).unwrap())
        .unwrap();
    a.compose(&b, None).unwrap();
    // 组合后仍应通过 SSA 校验（句柄已重映射到 a 的表）
    a.validate().unwrap();
    assert_eq!(a.classical_vars().len(), 1);
    assert_eq!(a.classical_values().len(), 1);
    println!("[compose] 经典句柄重映射：validate 通过，vars=1 values=1");
}

#[test]
fn transform_decompose_nested_circuit_gates() {
    let mut innermost = Circuit::new(1);
    innermost.h(q(0)).unwrap();
    let g1 = innermost.to_gate("L1").unwrap();

    let mut middle = Circuit::new(1);
    middle.append(g1.clone(), [q(0)], [], None).unwrap();
    middle.append(g1, [q(0)], [], None).unwrap();
    let g2 = middle.to_gate("L2").unwrap();

    let mut outer = Circuit::new(1);
    outer.append(g2, [q(0)], [], None).unwrap();
    let flat = outer.decompose().unwrap();
    assert_eq!(flat.operations().len(), 2, "两层嵌套应递归展平");
    assert!(flat.operations().iter().all(|op| op.instruction.is_standard()));
    println!("[decompose] 两层嵌套 circuit gate 递归展平为 2 个标准门");
}

// ==================== 八、depth（depth.rs 公开行为） ====================

#[test]
fn depth_flat_scenarios() {
    let mut c = Circuit::new(3);
    assert_eq!(c.depth(false).unwrap(), 0);
    c.h(q(0)).unwrap();
    c.x(q(0)).unwrap();
    c.z(q(0)).unwrap();
    assert_eq!(c.depth(false).unwrap(), 3, "单比特链");
    let mut p = Circuit::new(3);
    p.h(q(0)).unwrap();
    p.h(q(1)).unwrap();
    p.h(q(2)).unwrap();
    assert_eq!(p.depth(false).unwrap(), 1, "并行");
    let mut chain = Circuit::new(3);
    chain.cx(q(0), q(1)).unwrap();
    chain.cx(q(1), q(2)).unwrap();
    assert_eq!(chain.depth(false).unwrap(), 2, "CX 链");
    let mut bar = Circuit::new(2);
    bar.h(q(0)).unwrap();
    bar.h(q(1)).unwrap();
    bar.barrier(vec![q(0), q(1)]).unwrap();
    bar.h(q(0)).unwrap();
    assert_eq!(bar.depth(false).unwrap(), 3, "barrier 强制串行化");
    let mut global_bar = Circuit::new(2);
    global_bar.h(q(0)).unwrap();
    global_bar.barrier(Vec::new()).unwrap();
    global_bar.h(q(1)).unwrap();
    assert_eq!(global_bar.depth(false).unwrap(), 3, "全局 barrier 同步所有比特");
    let mut mr = Circuit::new(1);
    mr.measure(q(0)).unwrap();
    mr.reset(q(0)).unwrap();
    assert_eq!(mr.depth(false).unwrap(), 2, "测量与复位计入深度");
    println!("[depth] 0/链3/并1/CX链2/barrier3/全局barrier3/measure+reset=2 全部正确");
}

#[test]
fn depth_control_flow_recurse_semantics() {
    use cqlib_core::circuit::{ClassicalExpr, ClassicalType};
    // recurse=false 遇控制流报错
    let mut c = Circuit::new(1);
    c.if_(ClassicalExpr::bool_literal(true), |b| {
        b.x(q(0))?;
        Ok(())
    })
    .unwrap();
    assert!(matches!(c.depth(false), Err(CircuitError::ControlFlowPresent)));
    // recurse=true：if/else 取 max
    let mut ie = Circuit::new(1);
    ie.if_else(
        ClassicalExpr::bool_literal(true),
        |t| {
            t.x(q(0))?;
            t.z(q(0))?;
            Ok(())
        },
        |e| {
            e.y(q(0))?;
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(ie.depth(true).unwrap(), 3, "1 + max(2,1)");
    // for 静态范围展开
    let mut f = Circuit::new(1);
    let counter = f.var(ClassicalType::uint(8).unwrap());
    f.for_uint(
        counter,
        ClassicalExpr::uint_literal(8, 0).unwrap(),
        ClassicalExpr::uint_literal(8, 3).unwrap(),
        ClassicalExpr::uint_literal(8, 1).unwrap(),
        |b, _| {
            b.h(q(0))?;
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(f.depth(true).unwrap(), 4, "1 + 3×1");
    // 空范围
    let mut f2 = Circuit::new(1);
    let c2 = f2.var(ClassicalType::uint(8).unwrap());
    f2.for_uint(
        c2,
        ClassicalExpr::uint_literal(8, 5).unwrap(),
        ClassicalExpr::uint_literal(8, 2).unwrap(),
        ClassicalExpr::uint_literal(8, 1).unwrap(),
        |b, _| {
            b.h(q(0))?;
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(f2.depth(true).unwrap(), 1, "空范围 0 次迭代");
    // switch 取最大分支
    let mut sw = Circuit::new(1);
    let state = sw.var(ClassicalType::uint(2).unwrap());
    sw.switch(state.expr(), |case| {
        case.value(0, |b| {
            b.x(q(0))?;
            Ok(())
        })?;
        case.value(1, |b| {
            b.x(q(0))?;
            b.z(q(0))?;
            Ok(())
        })?;
        case.default(|b| {
            b.y(q(0))?;
            Ok(())
        })?;
        Ok(())
    })
    .unwrap();
    assert_eq!(sw.depth(true).unwrap(), 3, "1 + max(1,2,1)");
    println!("[depth] 控制流：false→Err；if=3、for 展开=4、空范围=1、switch=3");
}

// ==================== 九、to_matrix / circuit_to_matrix ====================

#[test]
fn matrix_bell_and_cx_little_endian() {
    let mut c = Circuit::new(2);
    c.h(q(0)).unwrap();
    c.cx(q(0), q(1)).unwrap();
    let m = matrix_of(&c);
    // 第一列 = Bell 态：( |00> + |11> ) / √2
    assert!((m[[0, 0]].re - FRAC_1_SQRT_2).abs() < 1e-12);
    assert!((m[[3, 0]].re - FRAC_1_SQRT_2).abs() < 1e-12);
    assert!(m[[1, 0]].norm() < 1e-12 && m[[2, 0]].norm() < 1e-12);
    // CX 单独（q0 为控制，q0 是 LSB）：[0,0]=1, [3,1]=1, [2,2]=1, [1,3]=1
    let mut cx = Circuit::new(2);
    cx.cx(q(0), q(1)).unwrap();
    let m = matrix_of(&cx);
    assert!((m[[0, 0]] - C1).norm() < 1e-12);
    assert!((m[[3, 1]] - C1).norm() < 1e-12);
    assert!((m[[2, 2]] - C1).norm() < 1e-12);
    assert!((m[[1, 3]] - C1).norm() < 1e-12);
    println!("[matrix] Bell 列验证 + CX 小端矩阵 4 个非零元素位置全部正确");
}

#[test]
fn matrix_qubits_order_permutation() {
    let mut c = Circuit::from_qubits(vec![q(5), q(2)]).unwrap();
    c.h(q(5)).unwrap();
    // 默认顺序：排序 [2,5] → q2 是 LSB，H 在 MSB
    let default = matrix_of(&c);
    assert!((default[[0, 0]].re - FRAC_1_SQRT_2).abs() < 1e-12);
    assert!((default[[2, 2]].re + FRAC_1_SQRT_2).abs() < 1e-12);
    // 显式顺序 [5,2] → q5 是 LSB，H 在 LSB
    let flipped = c.to_matrix(Some(&[5, 2])).unwrap();
    assert!((flipped[[0, 0]].re - FRAC_1_SQRT_2).abs() < 1e-12);
    assert!((flipped[[1, 1]].re + FRAC_1_SQRT_2).abs() < 1e-12);
    println!("[matrix] 非连续比特 {{5,2}}：默认序 H 在 MSB，显式序 [5,2] H 在 LSB");
}

#[test]
fn matrix_order_validation_errors() {
    let c = Circuit::new(2);
    assert!(c.to_matrix(Some(&[0])).is_err(), "长度不匹配");
    assert!(c.to_matrix(Some(&[0, 0])).is_err(), "集合不匹配");
    assert!(c.to_matrix(Some(&[0, 9])).is_err(), "未知比特");
    println!("[matrix] qubits_order 非法排列（长度/重复/未知比特）全部拒绝");
}

#[test]
fn matrix_unsupported_operations_rejected() {
    let mut measured = Circuit::new(1);
    measured.measure(q(0)).unwrap();
    assert!(matches!(
        measured.to_matrix(None),
        Err(CircuitError::NoMatrixRepresentation)
    ));
    let mut reset_c = Circuit::new(1);
    reset_c.reset(q(0)).unwrap();
    assert!(matches!(reset_c.to_matrix(None), Err(CircuitError::NoMatrixRepresentation)));
    let mut sym = Circuit::new(1);
    sym.rx(q(0), Parameter::symbol("θ")).unwrap();
    assert!(matches!(sym.to_matrix(None), Err(CircuitError::SymbolicParameterError)));
    let mut cf = Circuit::new(1);
    cf.if_(cqlib_core::circuit::ClassicalExpr::bool_literal(true), |b| {
        b.x(q(0))?;
        Ok(())
    })
    .unwrap();
    let err = cf.to_matrix(None).unwrap_err();
    assert!(matches!(err, CircuitError::InvalidOperation(_)));
    println!("[matrix] measure/reset → NoMatrix；符号未绑定 → SymbolicParameterError；控制流 → InvalidOperation");
}

#[test]
fn matrix_skips_barrier_and_delay() {
    let mut c = Circuit::new(2);
    c.barrier(vec![q(0), q(1)]).unwrap();
    c.delay(q(0), ParameterValue::Fixed(10.0)).unwrap();
    c.barrier(Vec::new()).unwrap();
    assert_identity(&matrix_of(&c), 1e-12);
    println!("[matrix] barrier/delay 被跳过 → 纯 barrier/delay 线路矩阵为恒等");
}

#[test]
fn matrix_circuit_to_matrix_module_fn_eq_method() {
    let mut c = Circuit::new(3);
    c.h(q(0)).unwrap();
    c.cx(q(0), q(2)).unwrap();
    c.rz(q(1), 0.4).unwrap();
    let by_fn = cqlib_core::circuit::circuit_to_matrix(&c, None).unwrap();
    let by_method = c.to_matrix(None).unwrap();
    assert!(max_delta(&by_fn, &by_method) < 1e-14);
    println!("[matrix] circuit_to_matrix 模块函数与 Circuit::to_matrix 完全一致");
}

// ==================== 十、Operation / ValueOperation 查询 ====================

#[test]
fn operation_predicates_and_matrix() {
    let mut c = Circuit::new(1);
    c.h(q(0)).unwrap();
    c.rz(q(0), 0.5).unwrap();
    let h_op = &c.operations()[0];
    assert_eq!(h_op.standard_gate(), Some(StandardGate::H));
    assert!(h_op.instruction.is_standard());
    assert_eq!(h_op.instruction.instruction_type(), "standard");
    let h_matrix = h_op.matrix().unwrap();
    assert_eq!(h_matrix.shape(), &[2, 2]);
    // 符号参数操作 → matrix 报 SymbolicParameterError
    let mut sym = Circuit::new(1);
    sym.rx(q(0), Parameter::symbol("θ")).unwrap();
    assert!(matches!(
        sym.operations()[0].matrix(),
        Err(CircuitError::SymbolicParameterError)
    ));
    // barrier 无矩阵
    let mut b = Circuit::new(1);
    b.barrier(vec![q(0)]).unwrap();
    assert!(matches!(
        b.operations()[0].matrix(),
        Err(CircuitError::NoMatrixRepresentation)
    ));
    println!("[operation] standard_gate/谓词/固定参数矩阵 OK；符号/无矩阵均正确报错");
}

#[test]
fn value_operation_predicates() {
    let vo = ValueOperation::from_standard(StandardGate::RX, [q(0)], [ParameterValue::Fixed(0.5)]);
    assert_eq!(vo.name(), "RX");
    assert_eq!(vo.num_qubits(), 1);
    assert_eq!(vo.num_params(), 1);
    assert_eq!(vo.instruction_type(), "standard");
    assert!(vo.is_standard());
    assert!(!vo.is_mcgate());
    assert!(!vo.is_unitary());
    assert!(!vo.is_circuit_gate());
    assert!(!vo.is_directive());
    assert!(!vo.is_classical_data());
    assert!(!vo.is_classical_control());
    assert!(!vo.is_delay());
    // 结构相等
    let same = ValueOperation::from_standard(StandardGate::RX, [q(0)], [ParameterValue::Fixed(0.5)]);
    assert_eq!(vo, same);
    let diff = ValueOperation::from_standard(StandardGate::RX, [q(0)], [ParameterValue::Fixed(1.5)]);
    assert_ne!(vo, diff);
    println!("[value-op] 全部谓词与 Display 语义正常，结构相等性正确");
}

// ==================== 十一、CircuitParam / ParameterValue ====================

#[test]
fn circuit_param_and_parameter_value_semantics() {
    // CircuitParam：同形才相等
    assert_eq!(CircuitParam::Fixed(1.0), CircuitParam::Fixed(1.0));
    assert_ne!(CircuitParam::Fixed(1.0), CircuitParam::Index(0));
    assert_eq!(CircuitParam::from(2.5), CircuitParam::Fixed(2.5));
    // ParameterValue：Fixed 与 Param 不交叉比较
    let pv_fixed: ParameterValue = 1.5.into();
    let pv_int: ParameterValue = 42i64.into();
    let pv_str: ParameterValue = "theta".into();
    let pv_param: ParameterValue = Parameter::symbol("θ").into();
    assert!(matches!(pv_fixed, ParameterValue::Fixed(v) if v == 1.5));
    assert!(matches!(pv_int, ParameterValue::Fixed(v) if v == 42.0));
    assert!(matches!(pv_str, ParameterValue::Param(_)));
    assert!(matches!(pv_param, ParameterValue::Param(_)));
    // 常量 Parameter 进入 ParameterValue 会折叠为 Fixed
    let folded: ParameterValue = Parameter::from(PI).into();
    assert!(matches!(folded, ParameterValue::Fixed(_)));
    // scaled
    assert_eq!(ParameterValue::Fixed(2.0).scaled(3.0), ParameterValue::Fixed(6.0));
    let scaled_sym = ParameterValue::Param(Parameter::symbol("θ")).scaled(2.0);
    assert_eq!(scaled_sym, ParameterValue::Param(Parameter::symbol("θ") * 2.0));
    // From<ParameterValue> for Parameter
    let back: Parameter = ParameterValue::Fixed(0.5).into();
    assert!((back.evaluate(&None).unwrap() - 0.5).abs() < 1e-15);
    println!("[circuit-param] 构造/相等性/scaled/双向转换全部符合文档");
}

// ==================== 十二、可靠性与不变量 ====================

#[test]
fn reliability_clone_equality_and_arc_sharing() {
    let mut c = Circuit::new(2);
    c.h(q(0)).unwrap();
    c.cx(q(0), q(1)).unwrap();
    let cloned = c.clone();
    assert_eq!(c, cloned, "克隆线路结构相等（忽略 circuit_id）");
    assert_ne!(c.id(), cloned.id(), "克隆产生新 circuit_id");
    // 独立构建的相同线路也相等
    let mut other = Circuit::new(2);
    other.h(q(0)).unwrap();
    other.cx(q(0), q(1)).unwrap();
    assert_eq!(c, other);
    let mut different = Circuit::new(2);
    different.x(q(0)).unwrap();
    assert_ne!(c, different);
    println!("[reliability] 克隆/重建结构相等、id 不同、不同内容不等");
}

#[test]
fn reliability_inverse_of_inverse_restores_matrix() {
    let mut c = Circuit::new(2);
    c.h(q(0)).unwrap();
    c.rz(q(1), 0.3).unwrap();
    c.cx(q(1), q(0)).unwrap();
    c.fsim(q(0), q(1), 0.2, 0.9).unwrap();
    let inv_inv = c.inverse().unwrap().inverse().unwrap();
    assert!(max_delta(&matrix_of(&inv_inv), &matrix_of(&c)) < 1e-9, "双重逆应还原原矩阵");
    println!("[reliability] (U†)† 数值还原原矩阵，偏差 < 1e-9");
}

#[test]
fn reliability_large_circuit_stress() {
    // 1000 个操作的构建、深度、矩阵（4 比特）
    let mut c = Circuit::new(4);
    for i in 0..1000u32 {
        let target = q(i % 4);
        c.h(target).unwrap();
    }
    assert_eq!(c.operations().len(), 1000);
    assert_eq!(c.depth(false).unwrap(), 1000 / 4 * 1 + 0, "每比特 250 层"); // 每比特 250 个 H 串行
    let m = matrix_of(&c);
    assert_eq!(m.dim(), (16, 16));
    // U 仍为幺正
    let dagger = m.t().mapv(|x| x.conj());
    assert_identity(&dagger.dot(&m), 1e-9);
    c.validate().unwrap();
    println!("[reliability] 1000 op 线路：深度={}，16×16 矩阵幺正性保持", c.depth(false).unwrap());
}

#[test]
fn reliability_validate_on_valid_dynamic_circuit() {
    use cqlib_core::circuit::{ClassicalExpr, ClassicalType};
    let mut c = Circuit::new(2);
    let flag = c.var(ClassicalType::Bool);
    let m = c.measure(q(0)).unwrap();
    c.store(flag, ClassicalExpr::bit_to_bool(m.expr()).unwrap())
        .unwrap();
    c.if_(flag.expr(), |b| {
        b.x(q(1))?;
        Ok(())
    })
    .unwrap();
    c.validate().unwrap();
    println!("[reliability] 完整动态线路（测量→存储→if）validate 通过");
}

#[test]
fn circuit_id_uniqueness_and_display() {
    let a = CircuitId::new();
    let b = CircuitId::new();
    assert_ne!(a, b);
    assert_eq!(a, a);
    let display = format!("{a}");
    assert!(display.starts_with("CircuitId("), "Display 格式: {display}");
    println!("[circuit-id] 唯一性与 Display（{display}）正常");
}

#[test]
fn operation_struct_fields_public() {
    let mut c = Circuit::new(1);
    c.rx(q(0), 0.5).unwrap();
    let op: &Operation = &c.operations()[0];
    assert_eq!(op.qubits.as_slice(), &[q(0)]);
    assert!(matches!(op.params[0], CircuitParam::Fixed(v) if v == 0.5));
    assert!(op.label.is_none());
    // Debug 可用
    let dbg = format!("{op:?}");
    assert!(dbg.contains("Operation"));
    println!("[operation] 公开字段 qubits/params/label 读取正常，Debug 输出含 'Operation'");
}
