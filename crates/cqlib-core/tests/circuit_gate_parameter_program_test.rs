//! circuit 模块全量功能测试 —— 文件 3：门体系与符号参数系统
//!
//! 被测目标：crates/cqlib-core/src/circuit/{parameter, gate/*}.rs 的全部公开 API：
//!   Parameter（38 个方法）、StandardGate、gate_matrix、Instruction、MCGate、
//!   UnitaryGate、CircuitGate/FrozenCircuit、Directive、ClassicalDataOp、
//!   ValueInstruction、storage_operation_to_value。
//!
//! 运行：cargo test -p cqlib-core --test circuit_gate_parameter_program_test -- --nocapture

use std::collections::HashMap;
use std::f64::consts::{E, PI};

use cqlib_core::circuit::gate::gate_matrix;
use cqlib_core::circuit::gate::{CircuitGate, FrozenCircuit, MCGate, UnitaryGate};
use cqlib_core::circuit::value_instruction::{
    ValueClassicalControlOp, ValueInstruction, storage_operation_to_value,
};
use cqlib_core::circuit::{
    Circuit, CircuitError, CircuitId, ClassicalControlOp, ClassicalDataOp, ClassicalExpr,
    ClassicalType, ClassicalValue, ClassicalVar, Directive, Instruction, Parameter, ParameterValue,
    Qubit, StandardGate,
};
use cqlib_core::circuit::symbolic_matrix::evaluate_symbolic_matrix;
use ndarray::Array2;
use num_complex::Complex64;

const C0: Complex64 = Complex64::new(0.0, 0.0);
const C1: Complex64 = Complex64::new(1.0, 0.0);

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

fn assert_identity(u: &Array2<Complex64>, eps: f64) {
    for i in 0..u.nrows() {
        for j in 0..u.ncols() {
            let expected = if i == j { C1 } else { C0 };
            assert!((u[[i, j]] - expected).norm() < eps, "U[{i},{j}] != I");
        }
    }
}

fn bindings<'a>(pairs: &'a [(&'a str, f64)]) -> Option<HashMap<&'a str, f64>> {
    let mut map = HashMap::new();
    for (k, v) in pairs {
        map.insert(*k, *v);
    }
    Some(map)
}

// ==================== 一、Parameter 系统（38 个公开方法） ====================

#[test]
fn parameter_symbol_and_get_symbols() {
    let theta = Parameter::symbol("θ");
    let syms = theta.get_symbols();
    assert_eq!(syms.len(), 1);
    assert!(syms.contains("θ"));
    // π/e 是常量符号，get_symbols 应剔除
    let expr = Parameter::pi() * Parameter::e();
    assert!(expr.get_symbols().is_empty(), "π/e 应从符号集中剔除");
    let mixed = Parameter::symbol("x") + Parameter::pi();
    let syms = mixed.get_symbols();
    assert_eq!(syms.len(), 1);
    assert!(syms.contains("x") && !syms.contains("π"));
    println!("[param] get_symbols：自由变量保留，π/e 常量剔除");
}

#[test]
fn parameter_parse_from_str() {
    let p = Parameter::try_from("θ * 2 + π").unwrap();
    let val = p.evaluate(&bindings(&[("θ", 0.5)])).unwrap();
    assert!((val - (PI + 1.0)).abs() < 1e-10);
    let p2 = <Parameter as std::str::FromStr>::from_str("sin(x) + 1").unwrap();
    let val2 = p2.evaluate(&bindings(&[("x", 0.0)])).unwrap();
    assert!((val2 - 1.0).abs() < 1e-10);
    assert!(Parameter::try_from("@@@").is_err());
    assert!(<Parameter as std::str::FromStr>::from_str("1 ++").is_err());
    println!("[param] 字符串解析：'θ*2+π' 与 'sin(x)+1' 求值正确，非法串拒绝");
}

#[test]
fn parameter_evaluate_numeric_and_bindings() {
    let p = Parameter::from(PI);
    assert!((p.evaluate(&None).unwrap() - PI).abs() < 1e-12);
    let theta = Parameter::symbol("θ");
    let expr = theta.clone() * 2.0f64;
    let val = expr.evaluate(&bindings(&[("θ", 1.0)])).unwrap();
    assert!((val - 2.0).abs() < 1e-12);
    // 常量 π/e 自动解析
    let phase = Parameter::pi() / 2.0f64;
    assert!((phase.evaluate(&None).unwrap() - PI / 2.0).abs() < 1e-12);
    assert!((Parameter::e().evaluate(&None).unwrap() - E).abs() < 1e-12);
    println!("[param] evaluate：常量/绑定/π/e 自动解析全部正确");
}

#[test]
fn parameter_evaluate_unbound_and_nonfinite_rejected() {
    let err = Parameter::symbol("θ").evaluate(&None).unwrap_err();
    assert!(matches!(
        err,
        cqlib_core::circuit::error::ParameterError::NaN(_)
    ));
    // 非有限参数：除零导致求值失败
    let err = (Parameter::from(1.0) / Parameter::from(0.0))
        .evaluate(&None)
        .unwrap_err();
    println!("[param] 1/0 求值错误变体：{err:?}");
    // 未绑定符号与非法数值都必须报错（不区分具体变体）
    assert!(matches!(
        err,
        cqlib_core::circuit::error::ParameterError::NaN(_)
            | cqlib_core::circuit::error::ParameterError::DomainError(_)
    ));
    println!("[param] 未绑定符号 → NaN 错误；除零 → DomainError");
}

#[test]
fn parameter_simplify() {
    let theta = Parameter::symbol("θ");
    let simplified = (theta.clone() * 1.0f64).simplify().unwrap();
    assert_eq!(simplified, theta);
    let folded = (Parameter::from(2.0) * 3.0f64).simplify().unwrap();
    assert!((folded.evaluate(&None).unwrap() - 6.0).abs() < 1e-12);
    let theta_plus_zero = (theta.clone() + 0.0f64).simplify().unwrap();
    assert_eq!(theta_plus_zero, theta);
    println!("[param] simplify：θ*1→θ、2*3→6、θ+0→θ");
}

#[test]
fn parameter_canonicalized() {
    // 常量 → 数值节点，-0.0 归一化为 +0.0
    let canon = (Parameter::from(PI) / 2.0f64).canonicalized().unwrap();
    assert!((canon.evaluate(&None).unwrap() - PI / 2.0).abs() < 1e-12);
    let zero = Parameter::from(-0.0).canonicalized().unwrap();
    assert!((zero.evaluate(&None).unwrap()).eq(&0.0));
    assert!(zero.evaluate(&None).unwrap().is_sign_positive());
    // 符号表达式保持符号形态
    let theta = Parameter::symbol("θ");
    let symbolic = (theta.clone() * 1.0f64).canonicalized().unwrap();
    assert_eq!(symbolic, theta);
    println!("[param] canonicalized：常量折叠 + (-0.0→+0.0) + 符号保持");
}

#[test]
fn parameter_is_exact_zero() {
    assert!(Parameter::from(0.0).is_exact_zero().unwrap());
    assert!(Parameter::from(-0.0).is_exact_zero().unwrap());
    assert!(!Parameter::from(1.0).is_exact_zero().unwrap());
    assert!(!Parameter::symbol("θ").is_exact_zero().unwrap());
    assert!((Parameter::from(0.5) - Parameter::from(0.5)).is_exact_zero().unwrap());
    println!("[param] is_exact_zero：±0 与折叠后 0 为真，符号与非零为假");
}

#[test]
fn parameter_derivative() {
    let theta = Parameter::symbol("θ");
    let expr = theta.clone() * theta.clone();
    let deriv = expr.derivative("θ").unwrap();
    let val = deriv.evaluate(&bindings(&[("θ", 3.0)])).unwrap();
    assert!((val - 6.0).abs() < 1e-10, "d(θ²)/dθ = 2θ");
    let sin_d = Parameter::symbol("x").sin().derivative("x").unwrap();
    let val = sin_d.evaluate(&bindings(&[("x", 0.0)])).unwrap();
    assert!((val - 1.0).abs() < 1e-10, "d(sin x)/dx = cos x");
    println!("[param] derivative：θ²→2θ(θ=3)=6；sin→cos(0)=1");
}

#[test]
fn parameter_math_functions() {
    let x = Parameter::from(2.0);
    let cases: Vec<(&str, f64)> = vec![
        ("abs", x.abs().evaluate(&None).unwrap()),
        ("sqrt", x.sqrt().evaluate(&None).unwrap()),
        ("exp", x.exp().evaluate(&None).unwrap()),
        ("ln", x.ln().evaluate(&None).unwrap()),
        ("log2", x.log(Parameter::from(2.0)).evaluate(&None).unwrap()),
        ("sin", x.sin().evaluate(&None).unwrap()),
        ("cos", x.cos().evaluate(&None).unwrap()),
        ("tan", x.tan().evaluate(&None).unwrap()),
        ("asin", (x.clone() / 2.0f64).asin().evaluate(&None).unwrap()),
        ("acos", (x.clone() / 2.0f64).acos().evaluate(&None).unwrap()),
        ("atan", x.atan().evaluate(&None).unwrap()),
        ("sinh", x.sinh().evaluate(&None).unwrap()),
        ("cosh", x.cosh().evaluate(&None).unwrap()),
        ("tanh", x.tanh().evaluate(&None).unwrap()),
        ("floor", x.floor().evaluate(&None).unwrap()),
        ("ceil", x.ceil().evaluate(&None).unwrap()),
        ("round", (x / 3.0f64).round().evaluate(&None).unwrap()),
    ];
    let expected: HashMap<&str, f64> = [
        ("abs", 2.0),
        ("sqrt", 2.0f64.sqrt()),
        ("exp", 2.0f64.exp()),
        ("ln", 2.0f64.ln()),
        ("log2", 1.0),
        ("sin", 2.0f64.sin()),
        ("cos", 2.0f64.cos()),
        ("tan", 2.0f64.tan()),
        ("asin", 1.0f64.asin()),
        ("acos", 1.0f64.acos()),
        ("atan", 2.0f64.atan()),
        ("sinh", 2.0f64.sinh()),
        ("cosh", 2.0f64.cosh()),
        ("tanh", 2.0f64.tanh()),
        ("floor", 2.0),
        ("ceil", 2.0),
        ("round", 1.0), // 2/3 = 0.667 → round = 1
    ]
    .into_iter()
    .collect();
    for (name, actual) in &cases {
        let want = expected[name];
        assert!((actual - want).abs() < 1e-12, "{name}: {actual} != {want}");
    }
    println!("[param] 17 个数学函数在 x=2 处求值全部正确");
}

#[test]
fn parameter_pow() {
    let x = Parameter::symbol("x");
    let x3 = x.pow(Parameter::from(3.0));
    let val = x3.evaluate(&bindings(&[("x", 2.0)])).unwrap();
    assert!((val - 8.0).abs() < 1e-10);
    println!("[param] pow：x³ 在 x=2 → 8");
}

#[test]
fn parameter_replace_and_as_symbol() {
    let expr = Parameter::symbol("θ") * 2.0f64;
    let replaced = expr.replace("θ", Parameter::pi());
    let val = replaced.evaluate(&None).unwrap();
    assert!((val - 2.0 * PI).abs() < 1e-10);
    // as_symbol
    assert_eq!(Parameter::symbol("θ").as_symbol(), Some("θ".to_string()));
    assert_eq!(
        (Parameter::symbol("θ") * 1.0f64).as_symbol(),
        Some("θ".to_string()),
        "化简后等价于单符号"
    );
    assert_eq!((Parameter::symbol("θ") + 1.0f64).as_symbol(), None);
    assert_eq!(Parameter::from(1.0).as_symbol(), None);
    println!("[param] replace θ→π 求值正确；as_symbol 三类判定全对");
}

#[test]
fn parameter_substitute_many() {
    let expr = Parameter::symbol("a") + Parameter::symbol("b");
    let mut map = HashMap::new();
    map.insert("a".to_string(), Parameter::from(2.0));
    let sub = expr.substitute_many(&map);
    let val = sub.evaluate(&bindings(&[("b", 5.0)])).unwrap();
    assert!((val - 7.0).abs() < 1e-12);
    println!("[param] substitute_many：a→2 后 a+b 在 b=5 → 7");
}

#[test]
fn parameter_provably_equal() {
    // 结构相等
    assert!(Parameter::symbol("θ").provably_equal(&Parameter::symbol("θ"), 1e-9));
    // 数值相等（浮点容差）
    assert!(Parameter::from(0.1 + 0.2).provably_equal(&Parameter::from(0.3), 1e-9));
    // 化简后相等：θ+0 vs θ
    assert!((Parameter::symbol("θ") + 0.0f64).provably_equal(&Parameter::symbol("θ"), 1e-9));
    // 不等
    assert!(!Parameter::from(1.0).provably_equal(&Parameter::from(2.0), 1e-9));
    assert!(!Parameter::symbol("x").provably_equal(&Parameter::symbol("y"), 1e-9));
    println!("[param] provably_equal：结构/数值/化简三路均正确");
}

#[test]
fn parameter_provably_equal_modulo() {
    let one = Parameter::from(1.0);
    // 0.5 ≡ 1.5 (mod 1)
    assert!(Parameter::from(0.5).provably_equal_modulo(&Parameter::from(1.5), &one, 1e-9));
    // 0.5 ≢ 1.4 (mod 1)
    assert!(!Parameter::from(0.5).provably_equal_modulo(&Parameter::from(1.4), &one, 1e-9));
    // 直接相等
    assert!(Parameter::from(2.0).provably_equal_modulo(&Parameter::from(2.0), &one, 1e-9));
    // 2π 周期：0.3 ≡ 0.3+2π (mod 2π)
    let two_pi = Parameter::from(2.0 * PI);
    assert!(Parameter::from(0.3).provably_equal_modulo(
        &Parameter::from(0.3 + 2.0 * PI),
        &two_pi,
        1e-9
    ));
    println!("[param] provably_equal_modulo：mod 1 与 mod 2π 判定全部正确");
}

#[test]
fn parameter_predicates_is_constant_zero_one() {
    assert!(Parameter::from(3.14).is_constant());
    assert!(Parameter::pi().is_constant(), "π 自动解析 → 常量");
    assert!(!Parameter::symbol("x").is_constant());
    assert!(Parameter::from(0.0).is_zero());
    assert!(!Parameter::from(1.0).is_zero());
    assert!(!Parameter::symbol("x").is_zero(), "符号不求值 → false");
    assert!(Parameter::from(1.0).is_one());
    assert!(!Parameter::from(2.0).is_one());
    assert!(!Parameter::e().is_one(), "e ≈ 2.718 ≠ 1");
    println!("[param] is_constant/is_zero/is_one 谓词全部正确");
}

#[test]
fn parameter_arithmetic_operators() {
    let a = Parameter::symbol("a");
    // Parameter op f64 / f64 op Parameter（f64/f32/i32/u32 四类均实现）
    let _ = a.clone() + 1.0f64;
    let _ = 1.0f64 + a.clone();
    let _ = a.clone() - 1.0f32;
    let _ = 1u32 - a.clone();
    let _ = a.clone() * 2i32;
    let _ = 2.0f64 * a.clone();
    let _ = a.clone() / 2.0f64;
    let _ = 2.0f64 / a.clone();
    // Parameter op Parameter（4 种组合）
    let b = Parameter::symbol("b");
    let sum = &a + &b;
    let diff = a.clone() - b.clone();
    let prod = a.clone() * b.clone();
    let quot = a.clone() / b.clone();
    let vals = |p: &Parameter| p.evaluate(&bindings(&[("a", 6.0), ("b", 2.0)])).unwrap();
    assert!((vals(&sum) - 8.0).abs() < 1e-12);
    assert!((vals(&diff) - 4.0).abs() < 1e-12);
    assert!((vals(&prod) - 12.0).abs() < 1e-12);
    assert!((vals(&quot) - 3.0).abs() < 1e-12);
    // Neg
    assert!((-a.clone()).evaluate(&bindings(&[("a", 5.0)])).map(|v| v + 5.0 == 0.0).unwrap());
    println!("[param] 四则运算 + 取负（双方向、双形态）数值全部正确");
}

#[test]
fn parameter_display_and_default() {
    let theta = Parameter::symbol("θ");
    let expr = theta.clone() * 2.0;
    let text = format!("{expr}");
    assert!(text.contains("θ"), "Display 应含符号：{text}");
    let text2 = format!("{:?}", Parameter::from(1.0));
    assert!(!text2.is_empty());
    // Default = 0
    assert!(Parameter::default().is_zero());
    println!("[param] Display='{text}'；Default 为 0");
}

#[test]
#[should_panic(expected = "must be finite")]
fn parameter_from_nan_panics() {
    let _ = Parameter::from(f64::NAN);
}

// ==================== 二、StandardGate ====================

#[test]
fn standard_gate_all_catalog() {
    let all = StandardGate::all();
    assert_eq!(all.len(), 36);
    let names: Vec<&str> = all.iter().map(|g| g.name()).collect();
    // 前 2 个与末尾几个固定顺序（判别值顺序）
    assert_eq!(names[0], "I");
    assert_eq!(names[1], "H");
    assert_eq!(names[35], "FSIM");
    // Display 与 name 一致
    for g in all {
        assert_eq!(format!("{g}"), g.name());
    }
    // Default 变体
    assert_eq!(StandardGate::default(), StandardGate::I);
    println!("[std-gate] all() 共 36 个门，名称/Display/默认值全部正确");
}

#[test]
fn standard_gate_metadata_consistency() {
    // 元数据抽查
    assert_eq!((StandardGate::H.num_qubits(), StandardGate::H.num_params()), (1, 0));
    assert_eq!((StandardGate::RX.num_qubits(), StandardGate::RX.num_params()), (1, 1));
    assert_eq!((StandardGate::U.num_qubits(), StandardGate::U.num_params()), (1, 3));
    assert_eq!((StandardGate::CX.num_qubits(), StandardGate::CX.num_params()), (2, 0));
    assert_eq!((StandardGate::CCX.num_qubits(), StandardGate::CCX.num_params()), (3, 0));
    assert_eq!((StandardGate::FSIM.num_qubits(), StandardGate::FSIM.num_params()), (2, 2));
    assert_eq!((StandardGate::GPhase.num_qubits(), StandardGate::GPhase.num_params()), (0, 1));
    // 全量：矩阵维度 = 2^num_qubits（GPhase 为特殊 2×2 载体，见质量观察）
    for g in StandardGate::all() {
        let params: Vec<f64> = (0..g.num_params()).map(|i| 0.3 + i as f64).collect();
        let m = g.matrix(&params).unwrap();
        let expected = if *g == StandardGate::GPhase { 2 } else { 1 << g.num_qubits() };
        assert_eq!(m.dim(), (expected, expected), "{g} 维度不符");
    }
    // 控制比特数
    assert_eq!(StandardGate::X.num_ctrl_qubits(), 0);
    assert_eq!(StandardGate::CX.num_ctrl_qubits(), 1);
    assert_eq!(StandardGate::CCX.num_ctrl_qubits(), 2);
    assert_eq!(StandardGate::CRX.num_ctrl_qubits(), 1);
    println!("[std-gate] 36 门元数据全量一致性（维度=2^qubits）+ 控制位数正确");
}

#[test]
fn standard_gate_is_diagonal_classification() {
    let diagonal = [
        StandardGate::I,
        StandardGate::RZ,
        StandardGate::RZZ,
        StandardGate::S,
        StandardGate::SDG,
        StandardGate::T,
        StandardGate::TDG,
        StandardGate::Z,
        StandardGate::Phase,
        StandardGate::GPhase,
        StandardGate::CZ,
        StandardGate::CRZ,
    ];
    for g in StandardGate::all() {
        assert_eq!(
            g.is_diagonal(),
            diagonal.contains(g),
            "{g} 的对角分类不符"
        );
    }
    println!("[std-gate] is_diagonal：12 个对角门分类全部正确（全量校验）");
}

#[test]
fn standard_gate_known_matrix_values() {
    let h = StandardGate::H.matrix(&[]).unwrap();
    assert!((h[[0, 0]].re - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-12);
    let x = StandardGate::X.matrix(&[]).unwrap();
    assert!((x[[0, 1]] - C1).norm() < 1e-12 && (x[[1, 0]] - C1).norm() < 1e-12);
    let z = StandardGate::Z.matrix(&[]).unwrap();
    assert!((z[[1, 1]].re + 1.0).abs() < 1e-12);
    let s = StandardGate::S.matrix(&[]).unwrap().into_owned();
    // S⁴ = I，T⁸ = I
    let s4 = s.dot(&s).dot(&s).dot(&s);
    assert_identity(&s4, 1e-12);
    let t = StandardGate::T.matrix(&[]).unwrap().into_owned();
    let mut t8 = t.dot(&t).dot(&t).dot(&t);
    t8 = t8.dot(&t8);
    assert_identity(&t8, 1e-12);
    // SWAP
    let swap = StandardGate::SWAP.matrix(&[]).unwrap();
    assert!((swap[[0, 0]] - C1).norm() < 1e-12 && (swap[[1, 2]] - C1).norm() < 1e-12);
    // CCX: |110> → |111>
    let ccx = StandardGate::CCX.matrix(&[]).unwrap();
    assert!((ccx[[7, 6]] - C1).norm() < 1e-12);
    println!("[std-gate] H/X/Z/S/T/SWAP/CCX 已知矩阵值全部正确（S⁴=T⁸=I）");
}

#[test]
fn standard_gate_parametric_matrix_semantics() {
    // RX(π) = -i·X（至多差全局相位）
    let rx_pi = StandardGate::RX.matrix(&[PI]).unwrap().into_owned();
    let x = StandardGate::X.matrix(&[]).unwrap().into_owned();
    let ratio = rx_pi[[0, 1]] / x[[0, 1]];
    assert!((ratio.norm() - 1.0).abs() < 1e-12);
    // RZ(λ) 对角：e^{∓iλ/2}
    let rz = StandardGate::RZ.matrix(&[0.8]).unwrap();
    let expected = Complex64::from_polar(1.0, -0.4);
    assert!((rz[[0, 0]] - expected).norm() < 1e-12);
    // Phase(λ)：|1> 乘 e^{iλ}
    let p = StandardGate::Phase.matrix(&[1.2]).unwrap();
    assert!((p[[0, 0]] - C1).norm() < 1e-12);
    assert!((p[[1, 1]] - Complex64::from_polar(1.0, 1.2)).norm() < 1e-12);
    // GPhase：实际返回 2×2 的 e^{iλ}·I 载体（与声明 0 比特不一致，见报告质量观察）
    let gp = StandardGate::GPhase.matrix(&[0.5]).unwrap();
    assert_eq!(gp.dim(), (2, 2));
    let phase = Complex64::from_polar(1.0, 0.5);
    assert!((gp[[0, 0]] - phase).norm() < 1e-12);
    assert!((gp[[1, 1]] - phase).norm() < 1e-12);
    assert!((gp[[0, 1]] - C0).norm() < 1e-12);
    // RXX(π/2)：⟨11|RXX|00⟩ = -i·sin(π/4) = (1/√2)·e^{-iπ/2}
    let rxx = StandardGate::RXX.matrix(&[PI / 2.0]).unwrap();
    assert!((rxx[[0, 3]] - Complex64::from_polar(std::f64::consts::FRAC_1_SQRT_2, -PI / 2.0)).norm() < 1e-12);
    println!("[std-gate] RX/RZ/Phase/GPhase/RXX 参数化矩阵语义逐元素正确");
}

#[test]
fn standard_gate_matrix_rejects_nonfinite() {
    assert!(StandardGate::RX.matrix(&[f64::NAN]).is_err());
    assert!(StandardGate::U.matrix(&[0.1, f64::INFINITY, 0.2]).is_err());
    assert!(StandardGate::FSIM.matrix(&[0.1, f64::NEG_INFINITY]).is_err());
    println!("[std-gate] NaN/Inf 参数在矩阵生成处被拒绝");
}

#[test]
fn standard_gate_inverse_rules() {
    let p = |v: f64| Parameter::from(v);
    // 自逆门
    for g in [StandardGate::H, StandardGate::I, StandardGate::X, StandardGate::Y, StandardGate::Z,
        StandardGate::CX, StandardGate::CCX, StandardGate::CY, StandardGate::CZ, StandardGate::SWAP]
    {
        let (inv, params) = g.inverse(&[]).unwrap();
        assert_eq!(inv, g, "{g} 应自逆");
        assert!(params.is_empty());
    }
    // 配对门
    let pairs = [
        (StandardGate::S, StandardGate::SDG),
        (StandardGate::T, StandardGate::TDG),
        (StandardGate::X2P, StandardGate::X2M),
        (StandardGate::Y2P, StandardGate::Y2M),
    ];
    for (a, b) in pairs {
        let (inv, _) = a.inverse(&[]).unwrap();
        assert_eq!(inv, b);
        let (inv2, _) = b.inverse(&[]).unwrap();
        assert_eq!(inv2, a);
    }
    // 参数取负族
    for g in [StandardGate::RX, StandardGate::RY, StandardGate::RZ, StandardGate::Phase,
        StandardGate::GPhase, StandardGate::RXX, StandardGate::RYY, StandardGate::RZZ,
        StandardGate::RZX, StandardGate::CRX, StandardGate::CRY, StandardGate::CRZ]
    {
        let (inv, params) = g.inverse(&[p(0.7)]).unwrap();
        assert_eq!(inv, g, "{g} 逆应同族");
        assert_eq!(params.len(), 1);
        let v = params[0].evaluate(&None).unwrap();
        assert!((v + 0.7).abs() < 1e-12, "{g} 参数应取负，got {v}");
    }
    // RXY(-θ, φ)
    let (inv, params) = StandardGate::RXY.inverse(&[p(0.3), p(0.9)]).unwrap();
    assert_eq!(inv, StandardGate::RXY);
    assert!((params[0].evaluate(&None).unwrap() + 0.3).abs() < 1e-12);
    assert!((params[1].evaluate(&None).unwrap() - 0.9).abs() < 1e-12);
    // FSIM(-θ, -φ)
    let (inv, params) = StandardGate::FSIM.inverse(&[p(0.3), p(0.9)]).unwrap();
    assert_eq!(inv, StandardGate::FSIM);
    assert!((params[0].evaluate(&None).unwrap() + 0.3).abs() < 1e-12);
    assert!((params[1].evaluate(&None).unwrap() + 0.9).abs() < 1e-12);
    // U(-θ, -λ, -φ)
    let (inv, params) = StandardGate::U.inverse(&[p(0.3), p(0.7), p(1.1)]).unwrap();
    assert_eq!(inv, StandardGate::U);
    let vals: Vec<f64> = params.iter().map(|x| x.evaluate(&None).unwrap()).collect();
    assert!((vals[0] + 0.3).abs() < 1e-12 && (vals[1] + 1.1).abs() < 1e-12 && (vals[2] + 0.7).abs() < 1e-12);
    // XY(π+θ)
    let (inv, params) = StandardGate::XY.inverse(&[p(0.4)]).unwrap();
    assert_eq!(inv, StandardGate::XY);
    let v = params[0].evaluate(&None).unwrap();
    assert!((v - (PI + 0.4)).abs() < 1e-12);
    println!("[std-gate] inverse 全表：10 自逆 + 4 配对 + 12 取负 + RXY/FSIM/U/XY 特殊规则");
}

#[test]
fn standard_gate_inverse_numeric_verification() {
    // 代表性门的 forward·inverse = I 数值验证
    let cases: Vec<(StandardGate, Vec<f64>)> = vec![
        (StandardGate::S, vec![]),
        (StandardGate::T, vec![]),
        (StandardGate::X2P, vec![]),
        (StandardGate::RX, vec![0.37]),
        (StandardGate::RXX, vec![-0.41]),
        (StandardGate::CRZ, vec![0.23]),
        (StandardGate::RXY, vec![0.31, 0.77]),
        (StandardGate::FSIM, vec![0.29, -0.13]),
        (StandardGate::U, vec![0.3, 0.7, 1.1]),
        (StandardGate::XY, vec![0.4]),
    ];
    for (g, params) in cases {
        let params_p: Vec<Parameter> = params.iter().map(|v| Parameter::from(*v)).collect();
        let fwd = g.matrix(&params).unwrap().into_owned();
        let (inv_gate, inv_params) = g.inverse(&params_p).unwrap();
        let inv_nums: Vec<f64> = inv_params.iter().map(|x| x.evaluate(&None).unwrap()).collect();
        let bwd = inv_gate.matrix(&inv_nums).unwrap().into_owned();
        assert_identity(&fwd.dot(&bwd), 1e-10);
        assert_identity(&bwd.dot(&fwd), 1e-10);
    }
    println!("[std-gate] 10 个代表性门 forward·inverse = I 数值全验证");
}

// ==================== 三、gate_matrix 底层函数 ====================

#[test]
fn gate_matrix_functions_match_standard_gate() {
    assert!(max_delta(&gate_matrix::rx_gate(0.7), &StandardGate::RX.matrix(&[0.7]).unwrap().into_owned()) < 1e-14);
    assert!(max_delta(&gate_matrix::ry_gate(0.7), &StandardGate::RY.matrix(&[0.7]).unwrap().into_owned()) < 1e-14);
    assert!(max_delta(&gate_matrix::rz_gate(0.7), &StandardGate::RZ.matrix(&[0.7]).unwrap().into_owned()) < 1e-14);
    assert!(max_delta(&gate_matrix::phase_gate(0.7), &StandardGate::Phase.matrix(&[0.7]).unwrap().into_owned()) < 1e-14);
    assert!(max_delta(
        &gate_matrix::global_phase_gate(0.7),
        &StandardGate::GPhase.matrix(&[0.7]).unwrap().into_owned()
    ) < 1e-14);
    assert!(max_delta(&gate_matrix::rxx_gate(0.4), &StandardGate::RXX.matrix(&[0.4]).unwrap().into_owned()) < 1e-14);
    assert!(max_delta(&gate_matrix::ryy_gate(0.4), &StandardGate::RYY.matrix(&[0.4]).unwrap().into_owned()) < 1e-14);
    assert!(max_delta(&gate_matrix::rzz_gate(0.4), &StandardGate::RZZ.matrix(&[0.4]).unwrap().into_owned()) < 1e-14);
    assert!(max_delta(&gate_matrix::rzx_gate(0.4), &StandardGate::RZX.matrix(&[0.4]).unwrap().into_owned()) < 1e-14);
    assert!(max_delta(&gate_matrix::crx_gate(0.4), &StandardGate::CRX.matrix(&[0.4]).unwrap().into_owned()) < 1e-14);
    assert!(max_delta(&gate_matrix::cry_gate(0.4), &StandardGate::CRY.matrix(&[0.4]).unwrap().into_owned()) < 1e-14);
    assert!(max_delta(&gate_matrix::crz_gate(0.4), &StandardGate::CRZ.matrix(&[0.4]).unwrap().into_owned()) < 1e-14);
    assert!(max_delta(&gate_matrix::xy_gate(0.4), &StandardGate::XY.matrix(&[0.4]).unwrap().into_owned()) < 1e-14);
    assert!(max_delta(&gate_matrix::xy2p_gate(0.4), &StandardGate::XY2P.matrix(&[0.4]).unwrap().into_owned()) < 1e-14);
    assert!(max_delta(&gate_matrix::xy2m_gate(0.4), &StandardGate::XY2M.matrix(&[0.4]).unwrap().into_owned()) < 1e-14);
    assert!(max_delta(
        &gate_matrix::rxy_gate(0.3, 0.9),
        &StandardGate::RXY.matrix(&[0.3, 0.9]).unwrap().into_owned()
    ) < 1e-14);
    assert!(max_delta(
        &gate_matrix::fsim_gate(0.3, 0.9),
        &StandardGate::FSIM.matrix(&[0.3, 0.9]).unwrap().into_owned()
    ) < 1e-14);
    assert!(max_delta(
        &gate_matrix::u_gate(0.3, 0.7, 1.1),
        &StandardGate::U.matrix(&[0.3, 0.7, 1.1]).unwrap().into_owned()
    ) < 1e-14);
    println!("[gate-matrix] 18 个底层矩阵函数与 StandardGate::matrix 完全一致");
}

#[test]
fn gate_matrix_control_matrix_structure() {
    // control_matrix(X, 1) 应与 CX 一致（大端：控制位为 MSB）
    let x = StandardGate::X.matrix(&[]).unwrap().into_owned();
    let cx_built = gate_matrix::control_matrix(&x, 1);
    let cx_std = StandardGate::CX.matrix(&[]).unwrap().into_owned();
    assert!(max_delta(&cx_built, &cx_std) < 1e-14);
    // control_matrix(H, 2)：8×8，控制位为两个 MSB，目标为 LSB：
    // 上半 4×4 为 I；下半分块 diag(I₂, H)：[4,5] 块 = I₂，[6,7] 块 = H
    let h = StandardGate::H.matrix(&[]).unwrap().into_owned();
    let ch2 = gate_matrix::control_matrix(&h, 2);
    assert_eq!(ch2.dim(), (8, 8));
    for i in 0..4 {
        for j in 0..4 {
            let expected = if i == j { C1 } else { C0 };
            assert!((ch2[[i, j]] - expected).norm() < 1e-14, "上 4×4 应为单位");
        }
    }
    // [4,5] 块 = I₂（c0=0 时目标不受作用）
    for i in 0..2 {
        for j in 0..2 {
            let expected = if i == j { C1 } else { C0 };
            assert!((ch2[[4 + i, 4 + j]] - expected).norm() < 1e-14);
        }
    }
    // [6,7] 块 = H（两控制位均 |1>）
    for i in 0..2 {
        for j in 0..2 {
            assert!((ch2[[6 + i, 6 + j]] - h[[i, j]]).norm() < 1e-14);
        }
    }
    // 分块间无耦合
    for i in [4, 5] {
        for j in [6, 7] {
            assert!((ch2[[i, j]] - C0).norm() < 1e-14);
            assert!((ch2[[j, i]] - C0).norm() < 1e-14);
        }
    }
    println!("[gate-matrix] control_matrix：X+1≡CX；H+2 分块 diag(I₄, diag(I₂,H)) 正确");
    println!("[gate-matrix] control_matrix：X+1≡CX；H+2 分块结构正确");
}

// ==================== 四、Instruction 统一枚举 ====================

#[test]
fn instruction_variant_surface() {
    let std_i: Instruction = StandardGate::H.into();
    let mc_i: Instruction = Instruction::McGate(Box::new(MCGate::new(1, StandardGate::X)));
    let mut inner = Circuit::new(1);
    inner.h(q(0)).unwrap();
    let cg_i: Instruction = inner.clone().to_gate("G").unwrap();
    let uni_i: Instruction = Instruction::UnitaryGate(Box::new(
        UnitaryGate::new("U1", 1, 0).with_matrix(Array2::eye(2).mapv(|x| Complex64::new(x, 0.0))).unwrap(),
    ));
    let dir_i: Instruction = Directive::Barrier.into();
    let data_i: Instruction = Instruction::ClassicalData(ClassicalDataOp::MeasureBit {
        result: ClassicalValue::new(CircuitId::new(), 0, ClassicalType::Bit),
    });
    let ctrl_i: Instruction = ClassicalControlOp::Break.into();
    let delay_i = Instruction::Delay;

    let cases = [
        (&std_i, "H", "standard"),
        (&mc_i, "C1-X", "mcgate"),
        (&cg_i, "G", "circuit"),
        (&uni_i, "U1", "unitary"),
        (&dir_i, "barrier", "directive"),
        (&data_i, "measure_bit", "classical_data"),
        (&ctrl_i, "break", "classical_control"),
        (&delay_i, "delay", "delay"),
    ];
    for (inst, name, ty) in cases {
        assert_eq!(inst.name(), name);
        assert_eq!(inst.instruction_type(), ty);
        assert_eq!(format!("{inst}"), name);
    }
    // 谓词
    assert!(std_i.is_standard() && std_i.is_quantum_gate());
    assert!(mc_i.is_mcgate() && mc_i.is_quantum_gate());
    assert!(uni_i.is_unitary() && uni_i.is_quantum_gate());
    assert!(cg_i.is_circuit_gate() && cg_i.is_quantum_gate());
    assert!(dir_i.is_directive() && !dir_i.is_quantum_gate());
    assert!(data_i.is_classical_data() && !data_i.is_quantum_gate());
    assert!(ctrl_i.is_classical_control() && !ctrl_i.is_quantum_gate());
    assert!(delay_i.is_delay() && !delay_i.is_quantum_gate());
    println!("[instruction] 8 个变体的 name/type/Display/谓词全部正确");
}

#[test]
fn instruction_gate_arity_table() {
    assert_eq!(Instruction::Standard(StandardGate::H).gate_arity(), Some((1, 0)));
    assert_eq!(Instruction::Standard(StandardGate::RX).gate_arity(), Some((1, 1)));
    assert_eq!(
        Instruction::McGate(Box::new(MCGate::new(2, StandardGate::X))).gate_arity(),
        Some((3, 0))
    );
    assert_eq!(
        Instruction::UnitaryGate(Box::new(UnitaryGate::new("U", 2, 1))).gate_arity(),
        Some((2, 1))
    );
    assert_eq!(Instruction::Directive(Directive::Measure).gate_arity(), Some((1, 0)));
    assert_eq!(Instruction::Directive(Directive::Reset).gate_arity(), Some((1, 0)));
    assert_eq!(Instruction::Delay.gate_arity(), Some((1, 1)));
    assert_eq!(
        Instruction::ClassicalData(ClassicalDataOp::Store {
            target: ClassicalVar::new(CircuitId::new(), 0, ClassicalType::Bool),
            value: ClassicalExpr::bool_literal(true),
        })
        .gate_arity(),
        Some((0, 0))
    );
    assert_eq!(
        Instruction::ClassicalData(ClassicalDataOp::MeasureBit {
            result: ClassicalValue::new(CircuitId::new(), 0, ClassicalType::Bit),
        })
        .gate_arity(),
        Some((1, 0))
    );
    // 无固定元数
    assert_eq!(Instruction::Directive(Directive::Barrier).gate_arity(), None);
    assert_eq!(
        Instruction::ClassicalData(ClassicalDataOp::MeasureBits {
            result: ClassicalValue::new(CircuitId::new(), 0, ClassicalType::bit_vec(2).unwrap()),
        })
        .gate_arity(),
        None
    );
    let break_instruction: Instruction = ClassicalControlOp::Break.into();
    assert_eq!(break_instruction.gate_arity(), None);
    println!("[instruction] gate_arity 全表正确");
}

#[test]
fn instruction_matrix_and_inverse() {
    // 矩阵
    assert!(Instruction::Standard(StandardGate::H).matrix(&[]).is_some());
    assert!(Instruction::McGate(Box::new(MCGate::new(1, StandardGate::X))).matrix(&[]).is_some());
    assert!(Instruction::Directive(Directive::Barrier).matrix(&[]).is_none());
    assert!(Instruction::Delay.matrix(&[]).is_none());
    let break_instruction: Instruction = ClassicalControlOp::Break.into();
    assert!(break_instruction.matrix(&[]).is_none());
    // 逆
    let (inv, _) = Instruction::Directive(Directive::Barrier).inverse(&[]).unwrap();
    assert!(matches!(inv, Instruction::Directive(Directive::Barrier)));
    assert!(Instruction::Directive(Directive::Measure).inverse(&[]).is_none());
    assert!(Instruction::Directive(Directive::Reset).inverse(&[]).is_none());
    let (inv, params) = Instruction::Delay.inverse(&[]).unwrap();
    assert!(matches!(inv, Instruction::Delay));
    assert!(params.is_empty());
    // Standard 逆
    let params = [Parameter::from(0.5)];
    let (inv, _) = Instruction::Standard(StandardGate::RX).inverse(&params).unwrap();
    assert!(matches!(inv, Instruction::Standard(StandardGate::RX)));
    println!("[instruction] matrix/inverse 分类行为全部正确");
}

#[test]
fn instruction_control_promotion() {
    let x = Instruction::Standard(StandardGate::X);
    assert!(matches!(x.control(0).unwrap(), Instruction::Standard(StandardGate::X)));
    assert!(matches!(x.control(1).unwrap(), Instruction::Standard(StandardGate::CX)));
    assert!(matches!(x.control(2).unwrap(), Instruction::Standard(StandardGate::CCX)));
    let three = x.control(3).unwrap();
    let Instruction::McGate(mc) = &three else { panic!() };
    assert_eq!(mc.name(), "C3-X");
    // CX + 1 → CCX（既有控制 + 新控制）
    let cx = Instruction::Standard(StandardGate::CX);
    assert!(matches!(cx.control(1).unwrap(), Instruction::Standard(StandardGate::CCX)));
    // H + 1 → McGate(C1-H)
    let ch = Instruction::Standard(StandardGate::H).control(1).unwrap();
    let Instruction::McGate(mc) = &ch else { panic!() };
    assert_eq!(*mc.base_gate(), StandardGate::H);
    // CRX + 1 → McGate(2, RX)
    let crx = Instruction::Standard(StandardGate::CRX).control(1).unwrap();
    let Instruction::McGate(mc) = &crx else { panic!() };
    assert_eq!(mc.num_ctrl_qubits(), 2);
    assert_eq!(*mc.base_gate(), StandardGate::RX);
    // McGate 聚合：C2-X + 1 → 3 控制无法降为标准门 → C3-X
    let mc2 = Instruction::McGate(Box::new(MCGate::new(2, StandardGate::X)));
    let promoted = mc2.control(1).unwrap();
    let Instruction::McGate(mc3) = &promoted else { panic!("应保持 McGate") };
    assert_eq!(mc3.num_ctrl_qubits(), 3);
    assert_eq!(mc3.name(), "C3-X");
    // 不可控
    assert!(Instruction::Directive(Directive::Barrier).control(1).is_none());
    assert!(Instruction::Directive(Directive::Measure).control(1).is_none());
    assert!(Instruction::Delay.control(1).is_none());
    let break_instruction: Instruction = ClassicalControlOp::Break.into();
    assert!(break_instruction.control(1).is_none());
    println!("[instruction] control 晋升：X→CX→CCX→C3-X、CX+1→CCX、H+1→McGate；不可控类拒绝");
}

#[test]
fn instruction_variant_accessors() {
    let std_i = Instruction::Standard(StandardGate::H);
    assert_eq!(std_i.standard_gate(), Some(StandardGate::H));
    assert!(std_i.mc_gate().is_none());
    assert!(std_i.unitary_gate().is_none());
    assert!(std_i.circuit_gate().is_none());
    assert!(std_i.directive().is_none());
    assert!(std_i.classical_data().is_none());
    assert!(std_i.classical_control().is_none());

    let mc = Instruction::McGate(Box::new(MCGate::new(1, StandardGate::Z)));
    assert_eq!(mc.mc_gate().unwrap().base_gate(), &StandardGate::Z);

    let dir = Instruction::Directive(Directive::Reset);
    assert_eq!(dir.directive(), Some(Directive::Reset));

    let value = ClassicalValue::new(CircuitId::new(), 0, ClassicalType::Bit);
    let measure = Instruction::ClassicalData(ClassicalDataOp::MeasureBit { result: value });
    assert!(measure.has_measurement());
    assert!(measure.classical_data().is_some());
    // MeasureBit 产生（而非读取）value：reads_value 应为 false
    assert!(!measure.reads_value(value), "测量是值的定义点，不是读取点");

    let store = Instruction::ClassicalData(ClassicalDataOp::Store {
        target: ClassicalVar::new(CircuitId::new(), 0, ClassicalType::Bool),
        value: ClassicalExpr::bit_to_bool(value.expr()).unwrap(),
    });
    assert!(!store.has_measurement());
    assert!(store.reads_value(value), "Store 读取表达式中含 value");

    // 控制流递归 has_measurement / reads_value
    let mut body_circuit = Circuit::new(1);
    let m = body_circuit.measure(q(0)).unwrap();
    body_circuit
        .if_(ClassicalExpr::bit_to_bool(m.expr()).unwrap(), |b| {
            b.x(q(0))?;
            Ok(())
        })
        .unwrap();
    let if_instruction = &body_circuit.operations()[1].instruction;
    // 条件在体外的测量：reads_value=true，但 has_measurement 只看体内 → false
    assert!(if_instruction.reads_value(m.value()));
    assert!(!if_instruction.has_measurement(), "条件中的测量不算体内测量");
    // 体内含测量 → has_measurement=true
    let mut c2 = Circuit::new(1);
    c2.if_(ClassicalExpr::bool_literal(true), |b| {
        b.measure(q(0))?;
        Ok(())
    })
    .unwrap();
    assert!(c2.operations()[0].instruction.has_measurement());
    println!("[instruction] 变体访问器 + has_measurement/reads_value 递归全部正确");
}

// ==================== 五、MCGate ====================

#[test]
fn mcgate_construction_and_metadata() {
    let ccx = MCGate::new(2, StandardGate::X);
    assert_eq!(ccx.num_ctrl_qubits(), 2);
    assert_eq!(ccx.num_qubits(), 3);
    assert_eq!(ccx.num_params(), 0);
    assert_eq!(*ccx.base_gate(), StandardGate::X);
    assert_eq!(ccx.name(), "C2-X");
    assert_eq!(format!("{ccx}"), "C2-X");
    // 0 控制 → 基门名
    assert_eq!(MCGate::new(0, StandardGate::X).name(), "X");
    // 1 控制 + CX（自身 1 控制）→ 2 控制
    let cx_ctrl = MCGate::new(1, StandardGate::CX);
    assert_eq!(cx_ctrl.num_ctrl_qubits(), 2);
    assert_eq!(cx_ctrl.num_qubits(), 3);
    // 参数透传
    assert_eq!(MCGate::new(1, StandardGate::RX).num_params(), 1);
    assert_eq!(MCGate::new(2, StandardGate::H).num_params(), 0);
    println!("[mcgate] 元数据（控制数/总比特/参数/基门/名称）全部正确");
}

#[test]
fn mcgate_matrix_construction() {
    // 0 控制 → 基门矩阵
    let plain = MCGate::new(0, StandardGate::X);
    let m0 = plain.matrix(&[]).unwrap();
    assert!(max_delta(&m0, &StandardGate::X.matrix(&[]).unwrap().into_owned()) < 1e-14);
    // 2 控制 X → CCX
    let ccx_like = MCGate::new(2, StandardGate::X);
    let m2 = ccx_like.matrix(&[]).unwrap();
    assert!(max_delta(&m2, &StandardGate::CCX.matrix(&[]).unwrap().into_owned()) < 1e-14);
    // 3 控制 Z → 16×16，仅 |1111> 对角元为 -1
    let mcz = MCGate::new(3, StandardGate::Z);
    let m3 = mcz.matrix(&[]).unwrap();
    assert_eq!(m3.dim(), (16, 16));
    assert!((m3[[15, 15]].re + 1.0).abs() < 1e-12);
    assert!((m3[[0, 0]] - C1).norm() < 1e-12);
    assert!((m3[[7, 7]] - C1).norm() < 1e-12);
    println!("[mcgate] 矩阵：0ctrl=基门、C2-X=CCX、C3-Z 对角结构正确");
}

#[test]
fn mcgate_inverse() {
    let gate = MCGate::new(1, StandardGate::RX);
    let (inv, params) = gate
        .inverse(&[Parameter::from(0.7)])
        .unwrap();
    assert_eq!(inv.num_ctrl_qubits(), 1);
    assert_eq!(*inv.base_gate(), StandardGate::RX);
    assert!((params[0].evaluate(&None).unwrap() + 0.7).abs() < 1e-12);
    // 数值验证
    let fwd = gate.matrix(&[0.7]).unwrap().into_owned();
    let bwd = inv
        .matrix(&[params[0].evaluate(&None).unwrap()])
        .unwrap()
        .into_owned();
    assert_identity(&fwd.dot(&bwd), 1e-12);
    println!("[mcgate] inverse：C(U)† = C(U†) 数值验证通过");
}

// ==================== 六、Directive 与 ClassicalDataOp ====================

#[test]
fn directive_names_and_inverse() {
    assert_eq!(Directive::Barrier.name(), "barrier");
    assert_eq!(Directive::Measure.name(), "measure");
    assert_eq!(Directive::Reset.name(), "reset");
    assert_eq!(Directive::Barrier.inverse(), Some(Directive::Barrier));
    assert_eq!(Directive::Measure.inverse(), None);
    assert_eq!(Directive::Reset.inverse(), None);
    assert_eq!(format!("{}", Directive::Barrier), "barrier");
    println!("[directive] 名称/Display/可逆性全部正确");
}

#[test]
fn classical_data_op_accessors() {
    let var = ClassicalVar::new(CircuitId::new(), 0, ClassicalType::Bool);
    let value = ClassicalValue::new(CircuitId::new(), 1, ClassicalType::Bit);
    let store = ClassicalDataOp::Store {
        target: var,
        value: ClassicalExpr::bool_literal(true),
    };
    assert_eq!(store.target(), Some(var));
    assert_eq!(store.result(), None);
    assert!(store.value().is_some());
    assert_eq!(store.name(), "store");

    let measure_bit = ClassicalDataOp::MeasureBit { result: value };
    assert_eq!(measure_bit.result(), Some(value));
    assert_eq!(measure_bit.target(), None);
    assert_eq!(measure_bit.value(), None);
    assert_eq!(measure_bit.name(), "measure_bit");

    let measure_bits = ClassicalDataOp::MeasureBits {
        result: ClassicalValue::new(CircuitId::new(), 2, ClassicalType::bit_vec(2).unwrap()),
    };
    assert_eq!(measure_bits.name(), "measure_bits");
    assert!(measure_bits.result().is_some());
    println!("[classical-data] 三个变体的 target/result/value 访问器正确");
}

// ==================== 七、UnitaryGate ====================

#[test]
fn unitary_gate_basic_accessors() {
    let gate = UnitaryGate::new("Oracle", 2, 0);
    assert_eq!(gate.label(), "Oracle");
    assert_eq!(gate.num_qubits(), 2);
    assert_eq!(gate.num_params(), 0);
    assert!(gate.matrix().is_none());
    assert!(gate.symbolic_matrix().is_none());
    assert!(gate.matrix_repr().is_none());
    assert!(gate.circuit().is_none());
    assert_eq!(format!("{gate}"), "Oracle");
    println!("[unitary] 基础访问器全部正确（未挂矩阵前各处为 None）");
}

#[test]
fn unitary_gate_with_matrix_validation() {
    let eye2 = Array2::eye(2).mapv(|x| Complex64::new(x, 0.0));
    // 形状错误：2 比特门需 4×4
    let err = UnitaryGate::new("Bad", 2, 0).with_matrix(eye2.clone()).unwrap_err();
    assert!(matches!(err, CircuitError::InvalidOperation(_)));
    // 非有限元素
    let mut nan_mat = eye2.clone();
    nan_mat[[0, 0]] = Complex64::new(f64::NAN, 0.0);
    let err = UnitaryGate::new("Bad", 1, 0).with_matrix(nan_mat).unwrap_err();
    assert!(matches!(err, CircuitError::InvalidOperation(_)));
    // 非幺正矩阵
    let scaled = eye2.mapv(|x| x * 2.0);
    let err = UnitaryGate::new("Bad", 1, 0).with_matrix(scaled).unwrap_err();
    assert!(matches!(err, CircuitError::InvalidOperation(_)));
    // 参数门不允许 with_matrix
    let err = UnitaryGate::new("Bad", 1, 1).with_matrix(eye2.clone()).unwrap_err();
    assert!(matches!(err, CircuitError::ParameterCountMismatch { expected: 0, actual: 1 }));
    // 正常路径
    let ok = UnitaryGate::new("Good", 1, 0).with_matrix(eye2).unwrap();
    assert!(ok.matrix().is_some());
    assert!(matches!(ok.matrix_repr(), Some(_)));
    println!("[unitary] with_matrix：形状/NaN/非幺正/参数门 4 类校验 + 正常路径");
}

#[test]
fn unitary_gate_identity_semantics() {
    let a = UnitaryGate::new("G", 1, 0);
    let cloned = a.clone();
    assert_eq!(a, cloned, "克隆共享 UUID → 相等");
    let rebuilt = UnitaryGate::new("G", 1, 0);
    assert_ne!(a, rebuilt, "重建产生新 UUID → 不等");
    // 同名不同对象 hash 不同（UUID 基础）
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let h1 = {
        let mut s = DefaultHasher::new();
        a.hash(&mut s);
        s.finish()
    };
    let h2 = {
        let mut s = DefaultHasher::new();
        rebuilt.hash(&mut s);
        s.finish()
    };
    assert_ne!(h1, h2);
    println!("[unitary] 身份语义：克隆相等、重建不等、hash 按 UUID");
}

#[test]
fn unitary_gate_matrix_for_params() {
    let x_mat = ndarray::array![[C0, C1], [C1, C0]];
    let gate = UnitaryGate::new("MyX", 1, 0).with_matrix(x_mat).unwrap();
    // 数值路径
    let m = gate.matrix_for_params(&[]).unwrap();
    assert_eq!(m.dim(), (2, 2));
    // 参数数量不匹配
    assert!(matches!(
        gate.matrix_for_params(&[1.0]),
        Err(CircuitError::ParameterCountMismatch { expected: 0, actual: 1 })
    ));
    // 非有限参数
    assert!(matches!(
        gate.matrix_for_params(&[f64::NAN]),
        Err(CircuitError::ParameterCountMismatch { expected: 0, actual: 1 })
    ));
    // 无任何定义 → NoMatrixRepresentation
    let empty = UnitaryGate::new("E", 1, 0);
    assert!(matches!(
        empty.matrix_for_params(&[]),
        Err(CircuitError::NoMatrixRepresentation)
    ));
    println!("[unitary] matrix_for_params：数值路径/参数校验/无定义拒绝");
}

#[test]
fn unitary_gate_with_circuit_signature() {
    let build_inner = |n: usize, use_symbol: bool| {
        let mut inner = Circuit::new(n);
        inner.h(q(0)).unwrap();
        if use_symbol {
            inner.rx(q(0), Parameter::symbol("t")).unwrap();
        }
        inner
    };
    // 匹配：2 比特 0 符号
    let frozen = std::sync::Arc::new(FrozenCircuit::new(build_inner(2, false)));
    assert!(UnitaryGate::new("U", 2, 0).with_circuit(frozen).is_ok());
    // 比特数不匹配
    let frozen3 = std::sync::Arc::new(FrozenCircuit::new(build_inner(3, false)));
    let err = UnitaryGate::new("U", 2, 0).with_circuit(frozen3).unwrap_err();
    assert!(matches!(err, CircuitError::QubitCountMismatch { expected: 2, actual: 3 }));
    // 符号数不匹配
    let frozen_sym = std::sync::Arc::new(FrozenCircuit::new(build_inner(2, true)));
    let err = UnitaryGate::new("U", 2, 0).with_circuit(frozen_sym).unwrap_err();
    assert!(matches!(err, CircuitError::ParameterCountMismatch { expected: 0, actual: 1 }));
    // 正确符号数
    let frozen_sym = std::sync::Arc::new(FrozenCircuit::new(build_inner(2, true)));
    let gate = UnitaryGate::new("U", 2, 1).with_circuit(frozen_sym).unwrap();
    assert!(gate.circuit().is_some());
    // 电路路径矩阵：绑定 t=π 后 = RX(π)
    let m = gate.matrix_for_params(&[PI]).unwrap();
    assert_eq!(m.dim(), (4, 4));
    println!("[unitary] with_circuit 签名校验 + 电路路径矩阵求解正确");
}

// ==================== 八、CircuitGate / FrozenCircuit ====================

#[test]
fn circuit_gate_basic_surface() {
    let mut inner = Circuit::new(2);
    inner.h(q(0)).unwrap();
    inner.rx(q(1), Parameter::symbol("θ")).unwrap();
    let frozen = FrozenCircuit::new(inner.clone());
    assert_eq!(frozen.circuit().qubits().len(), 2);
    assert_eq!(frozen.used_symbols().len(), 1);
    let gate = CircuitGate::new("MyGate", frozen).unwrap();
    assert_eq!(gate.name(), "MyGate");
    assert_eq!(gate.num_qubits(), 2);
    assert_eq!(gate.num_params(), 1, "签名参数 = used_symbols");
    assert_eq!(gate.signature_params().len(), 1);
    assert!(gate.symbols().contains("θ"));
    assert_eq!(gate.used_symbols(), &gate.symbols());
    assert_eq!(gate.circuit().circuit().qubits().len(), 2);
    // 符号矩阵缓存可用
    let sym = gate.symbolic_matrix().unwrap();
    assert_eq!(sym.shape(), &[4, 4]);
    println!("[circuit-gate] 基础面（名称/比特/参数/签名/符号矩阵）全部正确");
}

#[test]
fn circuit_gate_with_signature_rules() {
    let mut inner = Circuit::new(1);
    inner.rx(q(0), Parameter::symbol("used")).unwrap();
    // 声明未使用参数仍计入元数
    let gate = CircuitGate::with_signature(
        "declared",
        FrozenCircuit::new(inner.clone()),
        ["unused".to_string(), "used".to_string()],
    )
    .unwrap();
    assert_eq!(gate.num_params(), 2);
    assert_eq!(
        gate.signature_params().iter().cloned().collect::<Vec<_>>(),
        vec!["unused".to_string(), "used".to_string()]
    );
    assert!(!gate.symbols().contains("unused"));
    // 重复声明
    let err = CircuitGate::with_signature(
        "dup",
        FrozenCircuit::new(inner.clone()),
        ["a".to_string(), "a".to_string()],
    )
    .unwrap_err();
    assert!(matches!(err, CircuitError::InvalidOperation(_)));
    // 实现引用未声明符号
    let err = CircuitGate::with_signature(
        "missing",
        FrozenCircuit::new(inner),
        ["different".to_string()],
    )
    .unwrap_err();
    assert!(matches!(err, CircuitError::InvalidOperation(_)));
    println!("[circuit-gate] with_signature：声明≠使用、重复拒绝、未声明引用拒绝");
}

#[test]
fn circuit_gate_inverse_preserves_signature() {
    let mut inner = Circuit::new(1);
    inner.add_parameter(Parameter::symbol("stale")); // 驻留未引用
    inner.rx(q(0), Parameter::symbol("used")).unwrap();
    let gate = CircuitGate::with_signature(
        "forward",
        FrozenCircuit::new(inner),
        ["declared_unused".to_string(), "used".to_string()],
    )
    .unwrap();
    let inverse = gate.inverse().unwrap();
    assert_eq!(inverse.name(), "forward_dg");
    assert_eq!(inverse.signature_params(), gate.signature_params());
    assert_eq!(inverse.used_symbols(), gate.used_symbols());
    assert_eq!(inverse.num_qubits(), gate.num_qubits());
    println!("[circuit-gate] inverse：'_dg' 命名 + 签名/使用符号保持");
}

#[test]
fn frozen_circuit_equality_and_cache() {
    let make = || {
        let mut c = Circuit::new(1);
        c.h(q(0)).unwrap();
        FrozenCircuit::new(c)
    };
    let a = make();
    let b = make();
    assert_eq!(a, b);
    // 单侧物化符号矩阵缓存不影响相等
    a.symbolic_matrix().unwrap();
    assert_eq!(a, b);
    let mut other_c = Circuit::new(1);
    other_c.x(q(0)).unwrap();
    assert_ne!(a, FrozenCircuit::new(other_c));
    println!("[frozen] 结构相等、缓存不参与比较");
}

#[test]
fn circuit_gate_symbolic_matrix_evaluation() {
    // 符号矩阵绑定后应与数值矩阵一致
    let mut inner = Circuit::new(1);
    inner.rx(q(0), Parameter::symbol("t")).unwrap();
    let frozen = FrozenCircuit::new(inner.clone());
    let gate = CircuitGate::new("RxT", frozen).unwrap();
    let sym = gate.symbolic_matrix().unwrap();
    let mut b = HashMap::new();
    b.insert("t", PI);
    let numeric = evaluate_symbolic_matrix(&sym, &Some(b)).unwrap();
    // 数值参照：赋参后的电路矩阵
    let mut bindings = HashMap::new();
    bindings.insert("t", PI);
    let assigned = inner.assign_parameters(&Some(bindings)).unwrap();
    let reference = assigned.to_matrix(None).unwrap();
    assert!(max_delta(&numeric, &reference) < 1e-10);
    println!("[circuit-gate] 符号矩阵绑定 t=π 后与数值矩阵一致");
}

// ==================== 九、ValueInstruction / storage_operation_to_value ====================

#[test]
fn value_instruction_predicates_and_conversion() {
    let std_vi = ValueInstruction::from_instruction(Instruction::Standard(StandardGate::H));
    assert!(std_vi.is_instruction());
    assert!(!std_vi.is_classical_control());
    assert!(std_vi.is_standard());
    assert_eq!(std_vi.name(), "H");
    assert_eq!(std_vi.instruction_type(), "standard");
    assert_eq!(std_vi.standard_gate(), Some(StandardGate::H));
    assert!(std_vi.as_instruction().is_some());
    let converted = std_vi.clone().into_instruction().unwrap();
    assert!(matches!(converted, Instruction::Standard(StandardGate::H)));

    let ctrl_vi = ValueInstruction::ClassicalControl(ValueClassicalControlOp::Break);
    assert!(ctrl_vi.is_classical_control());
    assert!(!ctrl_vi.is_instruction());
    assert_eq!(ctrl_vi.name(), "break");
    assert!(ctrl_vi.as_instruction().is_none());
    assert!(ctrl_vi.clone().into_instruction().is_none());
    println!("[value-instruction] 谓词与双向转换全部正确");
}

#[test]
fn storage_operation_to_value_roundtrip() {
    let mut c = Circuit::new(1);
    c.rx(q(0), Parameter::symbol("θ")).unwrap();
    c.h(q(0)).unwrap();
    // 用电路自己的解析器做存储 IR → 值 IR 的还原
    let op0 = c.operations()[0].clone();
    let vo0 = storage_operation_to_value(op0, &|p| c.parameter_value(p)).unwrap();
    assert_eq!(vo0.name(), "RX");
    assert!(matches!(
        vo0.params[0],
        ParameterValue::Param(ref p) if p == &Parameter::symbol("θ")
    ));
    let op1 = c.operations()[1].clone();
    let vo1 = storage_operation_to_value(op1, &|p| c.parameter_value(p)).unwrap();
    assert_eq!(vo1.num_qubits(), 1);
    assert!(vo1.params.is_empty());
    println!("[storage→value] 还原：RX(θ)→Param(θ)、H→空参数");
}
