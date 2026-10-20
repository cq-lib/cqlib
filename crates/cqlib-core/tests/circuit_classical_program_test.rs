//! circuit 模块全量功能测试 —— 文件 2：经典计算与结构化控制流
//!
//! 被测目标：crates/cqlib-core/src/circuit/{classical, classical_expr, circuit_classical,
//! control_flow, circuit_verify}.rs 的全部公开 API。
//! 测试方式：integration test，仅通过 `cqlib_core` 公开 API 访问实现。
//!
//! 运行：cargo test -p cqlib-core --test circuit_classical_program_test -- --nocapture

use std::collections::HashMap;

use cqlib_core::circuit::value_instruction::{
    ValueClassicalControlOp, ValueControlBody, ValueInstruction,
};
use cqlib_core::circuit::{
    Circuit, CircuitError, CircuitId, ClassicalControlKind, ClassicalControlOp, ClassicalDataOp,
    ClassicalExpr, ClassicalExprKind, ClassicalType, ClassicalValue, ClassicalVar, ControlBody,
    ForOp, IfOp, Instruction, Measurement, Qubit, SwitchCase, ValueOperation,
};
use cqlib_core::device::Outcome;
use cqlib_core::qis::QisError;
use smallvec::smallvec;

fn q(id: u32) -> Qubit {
    Qubit::new(id)
}

fn u8ty() -> ClassicalType {
    ClassicalType::uint(8).unwrap()
}

// ==================== 一、ClassicalType ====================

#[test]
fn classical_type_constructors_and_widths() {
    assert_eq!(ClassicalType::Bit.width(), 1);
    assert_eq!(ClassicalType::Bool.width(), 1);
    assert_eq!(u8ty().width(), 8);
    assert_eq!(ClassicalType::bit_vec(3).unwrap().width(), 3);
    // 宽度为 0 直接拒绝
    assert_eq!(ClassicalType::uint(0), None);
    assert_eq!(ClassicalType::bit_vec(0), None);
    println!("[type] Bit/Bool 宽 1；UInt(8)/BitVec(3) 宽 8/3；uint(0)/bit_vec(0) → None");
}

#[test]
fn classical_type_zero_one_literals() {
    // Bool → bool_literal
    assert!(matches!(
        ClassicalType::Bool.zero_literal().unwrap().kind(),
        ClassicalExprKind::BoolLiteral(false)
    ));
    assert!(matches!(
        ClassicalType::Bool.one_literal().unwrap().kind(),
        ClassicalExprKind::BoolLiteral(true)
    ));
    // Bit → bit_literal
    assert!(matches!(
        ClassicalType::Bit.one_literal().unwrap().kind(),
        ClassicalExprKind::BitLiteral(true)
    ));
    // UInt(8) → uint_literal
    assert!(matches!(
        u8ty().one_literal().unwrap().kind(),
        ClassicalExprKind::UIntLiteral { width, value: 1 } if width.get() == 8
    ));
    // BitVec(8) → bit_vec_literal
    assert!(matches!(
        ClassicalType::bit_vec(8).unwrap().zero_literal().unwrap().kind(),
        ClassicalExprKind::BitVecLiteral { width, value: 0 } if width.get() == 8
    ));
    println!("[type] zero/one 字面量按类型正确分发（Bool/Bit/UInt/BitVec）");
}

#[test]
fn classical_type_measurement_width() {
    assert_eq!(ClassicalType::Bit.measurement_width(), Some(1));
    assert_eq!(ClassicalType::bit_vec(5).unwrap().measurement_width(), Some(5));
    assert_eq!(ClassicalType::Bool.measurement_width(), None);
    assert_eq!(u8ty().measurement_width(), None);
    println!("[type] measurement_width：Bit/BitVec 可测，Bool/UInt 不可");
}

#[test]
fn classical_expr_literal_width_and_overflow() {
    assert!(ClassicalExpr::uint_literal(200, 1).is_err(), "宽度 >128 拒绝");
    assert!(ClassicalExpr::uint_literal(8, 256).is_err(), "值超出宽度拒绝");
    assert!(ClassicalExpr::uint_literal(8, 255).is_ok());
    assert!(ClassicalExpr::uint_literal(128, u128::MAX).is_ok(), "128 位字面量支持");
    assert!(ClassicalExpr::bit_vec_literal(8, 256).is_err());
    println!("[literal] 宽度上限 128、值越界拒绝、边界 255/u128::MAX 通过");
}

// ==================== 二、句柄类型 ====================

#[test]
fn circuit_id_unique_and_display() {
    let a = CircuitId::new();
    let b = CircuitId::new();
    assert_ne!(a, b);
    let d = format!("{a}");
    assert!(d.starts_with("CircuitId("));
    // 默认构造等价于 new
    assert_ne!(a, CircuitId::default());
    println!("[circuit-id] 唯一性 + Display（{d}）");
}

#[test]
fn classical_var_handle_api() {
    let cid = CircuitId::new();
    let var = ClassicalVar::new(cid, 12, ClassicalType::bit_vec(4).unwrap());
    assert_eq!(var.id(), 12);
    assert_eq!(var.index(), 12);
    assert_eq!(var.circuit_id(), cid);
    assert_eq!(var.ty(), ClassicalType::bit_vec(4).unwrap());
    assert_eq!(var.expr().ty(), var.ty());
    // 身份 = (circuit_id, index, ty)
    assert_eq!(var, ClassicalVar::new(cid, 12, ClassicalType::bit_vec(4).unwrap()));
    assert_ne!(var, ClassicalVar::new(cid, 12, ClassicalType::Bool));
    assert_ne!(var, ClassicalVar::new(cid, 13, ClassicalType::bit_vec(4).unwrap()));
    println!("[var-handle] id/index/circuit_id/ty/expr 与身份语义全部正确");
}

#[test]
fn classical_value_handle_api() {
    let cid = CircuitId::new();
    let value = ClassicalValue::new(cid, 3, ClassicalType::Bit);
    assert_eq!(value.index(), 3);
    assert_eq!(value.circuit_id(), cid);
    assert_eq!(value.ty(), ClassicalType::Bit);
    assert_eq!(value.expr().ty(), ClassicalType::Bit);
    assert_eq!(value, ClassicalValue::new(cid, 3, ClassicalType::Bit));
    assert_ne!(value, ClassicalValue::new(cid, 3, ClassicalType::Bool));
    println!("[value-handle] index/circuit_id/ty/expr 与身份语义正确");
}

#[test]
fn measurement_handle_full_api() {
    let value = ClassicalValue::new(CircuitId::new(), 3, ClassicalType::bit_vec(2).unwrap());
    let measurement = Measurement::new(value, smallvec![q(1), q(0)]);
    assert_eq!(measurement.value(), value);
    assert_eq!(measurement.ty(), ClassicalType::bit_vec(2).unwrap());
    assert_eq!(measurement.width(), 2);
    assert_eq!(measurement.qubits(), &[q(1), q(0)]);
    assert_eq!(measurement.expr().ty(), ClassicalType::bit_vec(2).unwrap());
    // 克隆与相等
    assert_eq!(measurement, measurement.clone());
    println!("[measurement] value/ty/width/qubits/expr 全部按构造保留");
}

#[test]
fn measurement_check_qubit_bounds() {
    let value = ClassicalValue::new(CircuitId::new(), 0, ClassicalType::bit_vec(2).unwrap());
    let measurement = Measurement::new(value, smallvec![q(0), q(2)]);
    assert!(measurement.check_qubits(3).is_ok());
    assert!(matches!(
        measurement.check_qubits(2),
        Err(QisError::IndexOutOfBounds { index: 2, max: 1 })
    ));
    println!("[measurement] check_qubits：3 比特态内 OK；越界 → IndexOutOfBounds{{2, max:1}}");
}

#[test]
fn measurement_projection_bit_order() {
    // 与 classical.rs 内部单测同构：qubits()[i] → 结果位 i（LSB-first）
    let value = ClassicalValue::new(CircuitId::new(), 0, ClassicalType::bit_vec(3).unwrap());
    let measurement = Measurement::new(value, smallvec![q(2), q(0), q(1)]);
    let full = Outcome::from_indices(3, [0, 2]); // q0=1, q2=1
    let projected = measurement.project(&full);
    assert_eq!(projected.to_bitstring(3), "011");
    assert_eq!(measurement.project_basis(0b101).to_bitstring(3), "011");
    println!("[measurement] project/project_basis：位序投影结果 '011' 验证通过");
}

// ==================== 三、Circuit::measure / store 家族 ====================

#[test]
fn measure_single_appends_op_and_allocates_value() {
    let mut c = Circuit::new(2);
    let m = c.measure(q(0)).unwrap();
    assert_eq!(c.operations().len(), 1);
    let Instruction::ClassicalData(ClassicalDataOp::MeasureBit { result }) =
        &c.operations()[0].instruction
    else {
        panic!("应为 MeasureBit");
    };
    assert_eq!(*result, m.value());
    assert_eq!(m.ty(), ClassicalType::Bit);
    assert_eq!(m.width(), 1);
    assert_eq!(m.qubits(), &[q(0)]);
    assert_eq!(c.classical_values(), &[ClassicalType::Bit]);
    c.validate().unwrap();
    println!("[measure] 操作入 IR、值表登记 Bit、句柄字段一致，validate 通过");
}

#[test]
fn measure_twice_produces_distinct_values() {
    let mut c = Circuit::new(1);
    let m0 = c.measure(q(0)).unwrap();
    let m1 = c.measure(q(0)).unwrap();
    assert_ne!(m0.value(), m1.value(), "每次测量产生不同的 SSA 值");
    assert_eq!(m0.value().index(), 0);
    assert_eq!(m1.value().index(), 1);
    assert_eq!(c.operations().len(), 2);
    println!("[measure] 两次测量 → 值索引 0/1，互不相等");
}

#[test]
fn measure_bits_order_and_width() {
    let mut c = Circuit::new(3);
    let m = c.measure_bits([q(2), q(0), q(1)]).unwrap();
    assert_eq!(m.ty(), ClassicalType::bit_vec(3).unwrap());
    assert_eq!(m.width(), 3);
    // 输入顺序即位序：q2 是 bit0（LSB）
    assert_eq!(m.qubits(), &[q(2), q(0), q(1)]);
    let Instruction::ClassicalData(ClassicalDataOp::MeasureBits { .. }) =
        &c.operations()[0].instruction
    else {
        panic!("应为 MeasureBits");
    };
    c.validate().unwrap();
    println!("[measure_bits] 3 比特测量 → BitVec(3)，输入顺序保留为位序");
}

#[test]
fn measure_bits_empty_rejected_with_rollback() {
    let mut c = Circuit::new(1);
    let err = c.measure_bits(Vec::new()).unwrap_err();
    assert!(matches!(err, CircuitError::InvalidOperation(_)));
    assert!(c.operations().is_empty(), "操作回滚");
    assert!(c.classical_values().is_empty(), "值表回滚");
    println!("[measure_bits] 空列表 → {err}，线路无副作用");
}

#[test]
fn measure_into_appends_measure_plus_store() {
    let mut c = Circuit::new(1);
    let target = c.var(ClassicalType::Bit);
    let m = c.measure_into(q(0), target).unwrap();
    // measure + store 两个操作
    assert_eq!(c.operations().len(), 2);
    assert!(matches!(
        c.operations()[1].instruction,
        Instruction::ClassicalData(ClassicalDataOp::Store { .. })
    ));
    assert_eq!(m.ty(), target.ty());
    c.validate().unwrap();
    println!("[measure_into] 追加 measure+store 两个操作，句柄类型一致");
}

#[test]
fn measure_bits_into_width_mismatch_atomic_rollback() {
    let mut c = Circuit::new(3);
    let target = c.var(ClassicalType::bit_vec(2).unwrap());
    let err = c.measure_bits_into([q(0), q(1), q(2)], target).unwrap_err();
    assert!(matches!(err, CircuitError::InvalidOperation(_)), "类型不匹配");
    // 原子回滚：测量操作与值都不应残留
    assert!(c.operations().is_empty(), "操作回滚");
    assert!(c.classical_values().is_empty(), "值表回滚");
    // 正确宽度则成功
    let ok = c.measure_bits_into([q(0), q(1)], target).unwrap();
    assert_eq!(ok.ty(), target.ty());
    assert_eq!(c.operations().len(), 2);
    println!("[measure_bits_into] 宽度不匹配 → {err} 且原子回滚；匹配成功追加 2 op");
}

#[test]
fn store_appends_classical_data_op() {
    let mut c = Circuit::new(1);
    let flag = c.var(ClassicalType::Bool);
    c.store(flag, ClassicalExpr::bool_literal(true)).unwrap();
    let Instruction::ClassicalData(ClassicalDataOp::Store { target, value }) =
        &c.operations()[0].instruction
    else {
        panic!();
    };
    assert_eq!(*target, flag);
    assert!(value.is_bool_true());
    assert!(c.operations()[0].qubits.is_empty(), "Store 不占用量子比特");
    c.validate().unwrap();
    println!("[store] Store 操作入 IR（0 比特），target/value 一致");
}

#[test]
fn store_type_mismatch_rejected() {
    let mut c = Circuit::new(1);
    let flag = c.var(ClassicalType::Bool);
    // Bool 目标存 Bit 值 → 类型不匹配
    let err = c.store(flag, ClassicalExpr::bit_literal(true)).unwrap_err();
    assert!(matches!(err, CircuitError::InvalidOperation(_)));
    assert!(c.operations().is_empty());
    // UInt(8) 目标存 UInt(4) 值
    let wide = c.var(u8ty());
    let err = c
        .store(wide, ClassicalExpr::uint_literal(4, 1).unwrap())
        .unwrap_err();
    assert!(matches!(err, CircuitError::InvalidOperation(_)));
    println!("[store] Bool←Bit 与 UInt(8)←UInt(4) 均拒绝且回滚");
}

// ==================== 四、句柄校验 ====================

#[test]
fn validate_classical_var_ownership_and_type() {
    let mut c = Circuit::new(1);
    let mine = c.var(ClassicalType::Bool);
    c.validate_classical_var(mine).unwrap();
    // 外来电路句柄
    let foreign = ClassicalVar::new(CircuitId::new(), 0, ClassicalType::Bool);
    assert!(matches!(
        c.validate_classical_var(foreign),
        Err(CircuitError::ForeignClassicalHandle { kind: "classical variable", index: 0 })
    ));
    // 未分配的索引
    let unallocated = ClassicalVar::new(c.id(), 5, ClassicalType::Bool);
    assert!(c.validate_classical_var(unallocated).is_err());
    // 同索引不同类型
    let mistyped = ClassicalVar::new(c.id(), 0, ClassicalType::Bit);
    assert!(c.validate_classical_var(mistyped).is_err());
    println!("[validate-var] 本电路 OK；外来/未分配/类型不符均拒绝");
}

#[test]
fn validate_classical_value_ownership() {
    let mut c = Circuit::new(1);
    let m = c.measure(q(0)).unwrap();
    // 内部 validate_classical_value 是 pub(hidden)，借 validate_classical_expr 覆盖
    let expr = m.expr();
    c.validate_classical_expr(&expr).unwrap();
    // 外来值
    let foreign = ClassicalValue::new(CircuitId::new(), 0, ClassicalType::Bit).expr();
    assert!(c.validate_classical_expr(&foreign).is_err());
    // 未分配值
    let unallocated = ClassicalValue::new(c.id(), 9, ClassicalType::Bit).expr();
    assert!(c.validate_classical_expr(&unallocated).is_err());
    println!("[validate-value] 测量值 OK；外来/未分配拒绝");
}

#[test]
fn validate_classical_expr_walks_nested_handles() {
    let mut c = Circuit::new(2);
    let flag = c.var(ClassicalType::Bool);
    let m = c.measure(q(0)).unwrap();
    let expr = ClassicalExpr::try_and(
        flag.expr(),
        ClassicalExpr::bit_to_bool(m.expr()).unwrap(),
    )
    .unwrap();
    c.validate_classical_expr(&expr).unwrap();
    // 混入外来 var
    let foreign_flag = ClassicalVar::new(CircuitId::new(), 0, ClassicalType::Bool);
    let bad = ClassicalExpr::try_and(foreign_flag.expr(), expr).unwrap();
    assert!(c.validate_classical_expr(&bad).is_err());
    println!("[validate-expr] 嵌套表达式中所有 var/value 都被遍历校验");
}

// ==================== 五、if / if_else / while / for / switch 构建器 ====================

#[test]
fn if_appends_structured_control_op() {
    let mut c = Circuit::new(2);
    let m = c.measure(q(0)).unwrap();
    c.if_(ClassicalExpr::bit_to_bool(m.expr()).unwrap(), |body| {
        body.x(q(1))?;
        body.z(q(1))?;
        Ok(())
    })
    .unwrap();
    assert_eq!(c.operations().len(), 2, "测量 + if");
    let Instruction::ClassicalControl(ClassicalControlOp::If(op)) = &c.operations()[1].instruction
    else {
        panic!("应为 If");
    };
    assert_eq!(op.then_body().operations().len(), 2);
    assert!(op.else_body().is_none());
    assert!(op.condition().values().contains(&m.value()));
    assert_eq!(op.used_qubits(), std::iter::once(q(1)).collect());
    c.validate().unwrap();
    println!("[if] 结构入 IR：then 体 2 op、条件读取测量值、used_qubits 正确");
}

#[test]
fn if_else_builds_both_branches() {
    let mut c = Circuit::new(1);
    c.if_else(
        ClassicalExpr::bool_literal(true),
        |then_body| {
            then_body.x(q(0))?;
            Ok(())
        },
        |else_body| {
            else_body.z(q(0))?;
            Ok(())
        },
    )
    .unwrap();
    let Instruction::ClassicalControl(ClassicalControlOp::If(op)) = &c.operations()[0].instruction
    else {
        panic!();
    };
    assert_eq!(op.then_body().operations().len(), 1);
    assert_eq!(op.else_body().unwrap().operations().len(), 1);
    println!("[if_else] then/else 双分支均构建");
}

#[test]
fn if_condition_must_be_bool() {
    let mut c = Circuit::new(1);
    let err = c.if_(ClassicalExpr::bit_literal(true), |_| Ok(())).unwrap_err();
    assert!(matches!(err, CircuitError::InvalidOperation(_)), "Bit 条件应拒绝");
    assert!(c.operations().is_empty(), "失败回滚");
    println!("[if] Bit 类型条件 → {err}，无副作用");
}

#[test]
fn if_body_error_rolls_back_everything() {
    let mut c = Circuit::new(2);
    let baseline_ops = c.operations().len();
    let baseline_values = c.classical_values().len();
    let m = c.measure(q(0)).unwrap();
    let _ = m;
    let err = c
        .if_(ClassicalExpr::bool_literal(true), |body| {
            body.x(q(99))?; // 未注册比特 → 错误
            Ok(())
        })
        .unwrap_err();
    assert!(matches!(err, CircuitError::QubitNotFound(99)));
    assert_eq!(c.operations().len(), baseline_ops + 1, "仅测量保留");
    assert_eq!(c.classical_values().len(), baseline_values + 1);
    c.validate().unwrap();
    println!("[if] 体构建失败 → {err}，if 本身回滚，测量保留，validate 仍通过");
}

#[test]
fn while_loop_carried_state_pattern() {
    // 文档示例：while (running) { measure → store running }
    let mut c = Circuit::new(1);
    let running = c.var(ClassicalType::Bool);
    c.store(running, ClassicalExpr::bool_literal(true)).unwrap();
    c.while_(running.expr(), |body| {
        let measured = body.measure(q(0))?;
        let next = ClassicalExpr::bit_to_bool(measured.expr())?;
        body.store(running, next)?;
        Ok(())
    })
    .unwrap();
    let Instruction::ClassicalControl(ClassicalControlOp::While(op)) = &c.operations()[1].instruction
    else {
        panic!();
    };
    assert!(op.condition().vars().contains(&running));
    assert_eq!(op.body().operations().len(), 2, "测量 + 存储");
    c.validate().unwrap();
    println!("[while] 循环携带状态模式构建成功，体内 2 op");
}

#[test]
fn while_condition_must_be_bool() {
    let mut c = Circuit::new(1);
    let err = c
        .while_(ClassicalExpr::uint_literal(8, 0).unwrap(), |_| Ok(()))
        .unwrap_err();
    assert!(matches!(err, CircuitError::InvalidOperation(_)));
    assert!(c.operations().is_empty());
    println!("[while] UInt 条件 → {err}");
}

#[test]
fn for_uint_range_loop_var_and_callbacks() {
    let mut c = Circuit::new(1);
    let counter = c.var(u8ty());
    let seen_ty = std::cell::RefCell::new(None);
    c.for_uint(
        counter,
        ClassicalExpr::uint_literal(8, 0).unwrap(),
        ClassicalExpr::uint_literal(8, 4).unwrap(),
        ClassicalExpr::uint_literal(8, 1).unwrap(),
        |body, i_expr| {
            *seen_ty.borrow_mut() = Some(i_expr.ty());
            body.h(q(0))?;
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(seen_ty.into_inner(), Some(u8ty()), "闭包收到循环变量的读表达式");
    let Instruction::ClassicalControl(ClassicalControlOp::For(op)) = &c.operations()[0].instruction
    else {
        panic!();
    };
    assert_eq!(op.var(), counter);
    assert_eq!(op.body().operations().len(), 1);
    assert_eq!(op.classical_writes(), std::iter::once(counter).collect());
    c.validate().unwrap();
    println!("[for_uint] 循环变量类型传递、var/classical_writes/body 全部正确");
}

#[test]
fn for_uint_type_validation() {
    let mut c = Circuit::new(1);
    let counter = c.var(u8ty());
    // 循环变量必须 UInt：Bool 变量 → ForOp::new 报错
    let bool_var = c.var(ClassicalType::Bool);
    let err = c
        .for_uint(
            bool_var,
            ClassicalExpr::uint_literal(8, 0).unwrap(),
            ClassicalExpr::uint_literal(8, 4).unwrap(),
            ClassicalExpr::uint_literal(8, 1).unwrap(),
            |_, _| Ok(()),
        )
        .unwrap_err();
    assert!(matches!(err, CircuitError::InvalidOperation(_)));
    // start 宽度不一致
    let err = c
        .for_uint(
            counter,
            ClassicalExpr::uint_literal(4, 0).unwrap(),
            ClassicalExpr::uint_literal(8, 4).unwrap(),
            ClassicalExpr::uint_literal(8, 1).unwrap(),
            |_, _| Ok(()),
        )
        .unwrap_err();
    assert!(matches!(err, CircuitError::InvalidOperation(_)));
    assert!(c.operations().is_empty());
    println!("[for_uint] 非UInt变量/范围宽度不一致均拒绝");
}

#[test]
fn for_uint_body_error_rolls_back() {
    let mut c = Circuit::new(1);
    let counter = c.var(u8ty());
    let err = c
        .for_uint(
            counter,
            ClassicalExpr::uint_literal(8, 0).unwrap(),
            ClassicalExpr::uint_literal(8, 2).unwrap(),
            ClassicalExpr::uint_literal(8, 1).unwrap(),
            |body, _| {
                body.h(q(77))?; // 未注册比特
                Ok(())
            },
        )
        .unwrap_err();
    assert!(matches!(err, CircuitError::QubitNotFound(77)));
    assert!(c.operations().is_empty());
    println!("[for_uint] 体错误 → {err}，整体回滚");
}

#[test]
fn switch_cases_default_and_metadata() {
    let mut c = Circuit::new(2);
    let state = c.var(ClassicalType::uint(2).unwrap());
    c.store(state, ClassicalExpr::uint_literal(2, 1).unwrap()).unwrap();
    c.switch(state.expr(), |case| {
        case.value(0, |body| {
            body.x(q(0))?;
            Ok(())
        })?;
        case.value(2, |body| {
            body.h(q(1))?;
            Ok(())
        })?;
        case.default(|body| {
            body.z(q(0))?;
            Ok(())
        })?;
        Ok(())
    })
    .unwrap();
    let Instruction::ClassicalControl(ClassicalControlOp::Switch(op)) =
        &c.operations()[1].instruction
    else {
        panic!();
    };
    // case 保持源顺序（2, 0）
    let labels: Vec<u128> = op.cases().iter().map(SwitchCase::value).collect();
    assert_eq!(labels, vec![0, 2]);
    assert!(op.default().is_some());
    assert!(op.target().vars().contains(&state));
    c.validate().unwrap();
    println!("[switch] case 顺序保持、default 就位、target 读取变量");
}

#[test]
fn switch_duplicate_default_rejected() {
    let mut c = Circuit::new(1);
    let state = c.var(ClassicalType::uint(2).unwrap());
    let err = c
        .switch(state.expr(), |case| {
            case.default(|_| Ok(()))?;
            case.default(|_| Ok(()))?;
            Ok(())
        })
        .unwrap_err();
    assert!(matches!(err, CircuitError::InvalidOperation(_)));
    assert!(c.operations().is_empty());
    println!("[switch] 重复 default → {err}");
}

#[test]
fn switch_target_must_be_uint() {
    let mut c = Circuit::new(1);
    let err = c
        .switch(ClassicalExpr::bool_literal(true), |_| Ok(()))
        .unwrap_err();
    assert!(matches!(err, CircuitError::InvalidOperation(_)));
    println!("[switch] Bool target → {err}");
}

#[test]
fn switch_case_body_error_rolls_back() {
    let mut c = Circuit::new(1);
    let state = c.var(ClassicalType::uint(2).unwrap());
    let err = c
        .switch(state.expr(), |case| {
            case.value(0, |body| {
                body.h(q(88))?;
                Ok(())
            })?;
            Ok(())
        })
        .unwrap_err();
    assert!(matches!(err, CircuitError::QubitNotFound(88)));
    assert!(c.operations().is_empty(), "switch 整体回滚");
    println!("[switch] case 体错误 → {err}，整体回滚");
}

// ==================== 六、break / continue / append_control ====================

#[test]
fn break_and_continue_legal_inside_loop() {
    let mut c = Circuit::new(1);
    c.while_(ClassicalExpr::bool_literal(true), |body| {
        body.break_loop()?;
        Ok(())
    })
    .unwrap();
    // break 是 while 体内的子操作，顶层是 While
    let Instruction::ClassicalControl(ClassicalControlOp::While(op)) =
        &c.operations()[0].instruction
    else {
        panic!("顶层应为 While");
    };
    assert_eq!(op.body().operations().len(), 1);
    assert!(matches!(
        op.body().operations()[0].instruction,
        Instruction::ClassicalControl(ClassicalControlOp::Break)
    ));
    c.validate().unwrap();

    let mut c2 = Circuit::new(1);
    c2.while_(ClassicalExpr::bool_literal(true), |body| {
        body.continue_loop()?;
        Ok(())
    })
    .unwrap();
    let Instruction::ClassicalControl(ClassicalControlOp::While(op2)) =
        &c2.operations()[0].instruction
    else {
        panic!();
    };
    assert!(matches!(
        op2.body().operations()[0].instruction,
        Instruction::ClassicalControl(ClassicalControlOp::Continue)
    ));
    c2.validate().unwrap();
    println!("[break/continue] 位于 while 体内且为终止位置，validate 通过");
}

#[test]
fn break_legal_inside_switch_body() {
    let mut c = Circuit::new(1);
    let state = c.var(ClassicalType::uint(2).unwrap());
    c.switch(state.expr(), |case| {
        case.value(0, |body| {
            body.break_loop()?;
            Ok(())
        })?;
        Ok(())
    })
    .unwrap();
    c.validate().unwrap();
    println!("[break] switch 体内合法（退出 switch）");
}


#[test]
fn break_continue_at_top_level_rejected() {
    let mut c = Circuit::new(1);
    let err = c.break_loop().unwrap_err();
    assert!(matches!(err, CircuitError::InvalidOperation(_)));
    let err = c.continue_loop().unwrap_err();
    assert!(matches!(err, CircuitError::InvalidOperation(_)));
    assert!(c.operations().is_empty());
    println!("[break/continue] 顶层使用 → 均拒绝且回滚");
}

#[test]
fn continue_inside_switch_rejected() {
    let mut c = Circuit::new(1);
    let state = c.var(ClassicalType::uint(2).unwrap());
    let err = c
        .switch(state.expr(), |case| {
            case.value(0, |body| {
                body.continue_loop()?;
                Ok(())
            })?;
            Ok(())
        })
        .unwrap_err();
    assert!(matches!(err, CircuitError::InvalidOperation(_)));
    assert!(c.operations().is_empty());
    println!("[continue] switch 体内 → {err}（仅循环体允许）");
}

#[test]
fn non_terminal_break_rejected_by_builder() {
    let mut c = Circuit::new(1);
    let err = c
        .while_(ClassicalExpr::bool_literal(true), |body| {
            body.break_loop()?;
            body.x(q(0))?; // break 后还有操作 → 非终止位置
            Ok(())
        })
        .unwrap_err();
    assert!(matches!(err, CircuitError::NonTerminalControlTransfer { operation: "break", .. }));
    assert!(c.operations().is_empty(), "整体回滚");
    println!("[break] 非终止位置 → {err:?}");
}

#[test]
fn append_control_raw_operation() {
    // 手工构造 IfOp 并用 append_control 追加
    let mut c = Circuit::new(1);
    let flag = c.var(ClassicalType::Bool);
    let if_op = IfOp::new(
        flag.expr(),
        ControlBody::new(vec![]),
        None,
    )
    .unwrap();
    c.append_control(ClassicalControlOp::If(if_op)).unwrap();
    assert!(matches!(
        c.operations()[0].instruction,
        Instruction::ClassicalControl(ClassicalControlOp::If(_))
    ));
    c.validate().unwrap();
    println!("[append_control] 原始控制流操作追加成功");
}

// ==================== 七、控制流操作结构体元数据 ====================

#[test]
fn control_op_metadata_surface() {
    let mut c = Circuit::new(2);
    let flag = c.var(ClassicalType::Bool);
    let m = c.measure(q(0)).unwrap();
    c.if_(
        ClassicalExpr::try_and(flag.expr(), ClassicalExpr::bit_to_bool(m.expr()).unwrap())
            .unwrap(),
        |body| {
            body.x(q(1))?;
            Ok(())
        },
    )
    .unwrap();
    let Instruction::ClassicalControl(op) = &c.operations()[1].instruction else {
        panic!();
    };
    assert_eq!(op.kind(), ClassicalControlKind::If);
    assert_eq!(op.name(), "if");
    assert_eq!(op.to_string(), "if");
    assert!(op.classical_var_reads().contains(&flag));
    assert!(op.classical_value_reads().contains(&m.value()));
    assert!(op.classical_writes().is_empty(), "if 不写变量");
    assert_eq!(op.used_qubits(), std::iter::once(q(1)).collect());
    assert!(!op.has_measurement(), "then 体只含 x");
    assert!(op.reads_value(m.value()));
    println!("[control-op] kind/name/reads/writes/used_qubits/has_measurement/reads_value 全对");
}

#[test]
fn control_op_has_measurement_recursive() {
    let mut c = Circuit::new(1);
    c.while_(ClassicalExpr::bool_literal(true), |body| {
        body.measure(q(0))?;
        body.break_loop()?;
        Ok(())
    })
    .unwrap();
    let Instruction::ClassicalControl(op) = &c.operations()[0].instruction else {
        panic!();
    };
    assert!(op.has_measurement(), "while 体内的测量应递归可见");
    println!("[control-op] has_measurement 递归检测体内测量");
}

#[test]
fn for_op_and_switch_case_struct_accessors() {
    let cid = CircuitId::new();
    let var = ClassicalVar::new(cid, 0, u8ty());
    let start = ClassicalExpr::uint_literal(8, 1).unwrap();
    let stop = ClassicalExpr::uint_literal(8, 9).unwrap();
    let step = ClassicalExpr::uint_literal(8, 2).unwrap();
    let body = ControlBody::new(vec![]);
    let op = ForOp::new(var, start.clone(), stop.clone(), step.clone(), body).unwrap();
    assert_eq!(op.var(), var);
    assert_eq!(*op.start(), start);
    assert_eq!(*op.stop(), stop);
    assert_eq!(*op.step(), step);
    assert_eq!(op.body().operations().len(), 0);
    assert_eq!(op.classical_writes(), std::iter::once(var).collect());

    // SwitchCase
    let case = SwitchCase::new(7, ControlBody::new(vec![]));
    assert_eq!(case.value(), 7);
    assert_eq!(case.body().operations().len(), 0);
    println!("[struct] ForOp/SwitchCase 访问器全部正确");
}

#[test]
fn control_body_api() {
    let mut inner = Circuit::new(1);
    inner.h(q(0)).unwrap();
    let body = ControlBody::new(inner.operations().to_vec());
    assert_eq!(body.operations().len(), 1);
    assert_eq!(body.used_qubits(), std::iter::once(q(0)).collect());
    assert!(!body.has_measurement());
    // reads_value 对无经典操作体恒 false
    let value = ClassicalValue::new(CircuitId::new(), 0, ClassicalType::Bit);
    assert!(!body.reads_value(value));
    // 空体
    let empty = ControlBody::new(vec![]);
    assert!(empty.used_qubits().is_empty());
    println!("[control-body] operations/used_qubits/has_measurement/reads_value 正常");
}

// ==================== 八、ClassicalExpr 全量 API ====================

#[test]
fn expr_leaf_constructors_and_types() {
    assert_eq!(ClassicalExpr::bool_literal(true).ty(), ClassicalType::Bool);
    assert_eq!(ClassicalExpr::bit_literal(false).ty(), ClassicalType::Bit);
    assert_eq!(ClassicalExpr::uint_literal(8, 42).unwrap().ty(), u8ty());
    assert_eq!(
        ClassicalExpr::bit_vec_literal(4, 5).unwrap().ty(),
        ClassicalType::bit_vec(4).unwrap()
    );
    let var = ClassicalVar::new(CircuitId::new(), 0, ClassicalType::Bit);
    let value = ClassicalValue::new(CircuitId::new(), 0, ClassicalType::Bool);
    assert_eq!(ClassicalExpr::var(var).ty(), ClassicalType::Bit);
    assert_eq!(ClassicalExpr::value(value).ty(), ClassicalType::Bool);
    println!("[expr] 5 类叶子的类型全部正确");
}

#[test]
fn expr_kind_discriminants() {
    let cid = CircuitId::new();
    let var = ClassicalVar::new(cid, 0, ClassicalType::Bool);
    assert!(matches!(ClassicalExpr::var(var).kind(), ClassicalExprKind::Var(_)));
    let value = ClassicalValue::new(cid, 1, ClassicalType::Bit);
    assert!(matches!(ClassicalExpr::value(value).kind(), ClassicalExprKind::Value(_)));
    assert!(matches!(
        ClassicalExpr::bool_literal(true).kind(),
        ClassicalExprKind::BoolLiteral(true)
    ));
    assert!(matches!(
        ClassicalExpr::uint_literal(8, 3).unwrap().kind(),
        ClassicalExprKind::UIntLiteral { width, value: 3 } if width.get() == 8
    ));
    let not = ClassicalExpr::try_not(ClassicalExpr::bool_literal(true)).unwrap();
    assert!(matches!(not.kind(), ClassicalExprKind::Unary { .. }));
    assert!(matches!(
        ClassicalExpr::eq(
            ClassicalExpr::uint_literal(8, 1).unwrap(),
            ClassicalExpr::uint_literal(8, 1).unwrap()
        )
        .unwrap()
        .kind(),
        ClassicalExprKind::Compare { .. }
    ));
    assert!(matches!(
        ClassicalExpr::bit_to_bool(ClassicalExpr::bit_literal(true)).unwrap().kind(),
        ClassicalExprKind::Cast { .. }
    ));
    assert!(matches!(
        ClassicalExpr::select(
            ClassicalExpr::bool_literal(true),
            ClassicalExpr::bool_literal(false),
            ClassicalExpr::bool_literal(true)
        )
        .unwrap()
        .kind(),
        ClassicalExprKind::Select { .. }
    ));
    assert!(matches!(
        ClassicalExpr::extract_bit(ClassicalExpr::uint_literal(8, 1).unwrap(), 0)
            .unwrap()
            .kind(),
        ClassicalExprKind::ExtractBit { .. }
    ));
    assert!(matches!(
        ClassicalExpr::extract_bits(ClassicalExpr::uint_literal(8, 1).unwrap(), 0, 4)
            .unwrap()
            .kind(),
        ClassicalExprKind::ExtractBits { .. }
    ));
    assert!(matches!(
        ClassicalExpr::concat([
            ClassicalExpr::bit_literal(true),
            ClassicalExpr::bit_literal(false)
        ])
        .unwrap()
        .kind(),
        ClassicalExprKind::Concat { .. }
    ));
    assert!(matches!(
        ClassicalExpr::pack_bits([
            ClassicalExpr::bit_literal(true),
            ClassicalExpr::bit_literal(false)
        ])
        .unwrap()
        .kind(),
        ClassicalExprKind::PackBits { .. }
    ));
    println!("[expr] kind() 全部 11 类节点判别正确");
}

#[test]
fn expr_literal_predicates() {
    assert!(ClassicalExpr::bool_literal(true).is_bool_true());
    assert!(ClassicalExpr::bool_literal(false).is_bool_false());
    assert!(ClassicalExpr::bit_literal(true).is_bit_true());
    assert!(ClassicalExpr::bit_literal(false).is_bit_false());
    // 变量读取不判定为字面量
    let var = ClassicalVar::new(CircuitId::new(), 0, ClassicalType::Bool);
    assert!(!var.expr().is_bool_true() && !var.expr().is_bool_false());
    let bit_var = ClassicalVar::new(CircuitId::new(), 1, ClassicalType::Bit);
    assert!(!bit_var.expr().is_bit_true() && !bit_var.expr().is_bit_false());
    println!("[expr] is_bool/bit_true/false 谓词：字面量精确、变量恒 false");
}

#[test]
fn expr_vars_values_collection() {
    let cid = CircuitId::new();
    let a = ClassicalVar::new(cid, 0, ClassicalType::Bit);
    let b = ClassicalVar::new(cid, 1, ClassicalType::Bit);
    let v0 = ClassicalValue::new(cid, 5, ClassicalType::Bit);
    let expr = ClassicalExpr::try_and(
        ClassicalExpr::bit_to_bool(a.expr()).unwrap(),
        ClassicalExpr::try_or(
            ClassicalExpr::bit_to_bool(b.expr()).unwrap(),
            ClassicalExpr::bit_to_bool(v0.expr()).unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
    let vars = expr.vars();
    assert!(vars.contains(&a) && vars.contains(&b) && vars.len() == 2);
    let values = expr.values();
    assert!(values.contains(&v0) && values.len() == 1);
    // collect_* 增量收集
    let mut collected = std::collections::BTreeSet::new();
    expr.collect_vars(&mut collected);
    assert_eq!(collected.len(), 2);
    let mut collected_values = std::collections::BTreeSet::new();
    expr.collect_values(&mut collected_values);
    assert_eq!(collected_values.len(), 1);
    println!("[expr] vars/values/collect_vars/collect_values 递归收集正确");
}

#[test]
fn expr_remap_classical_ids() {
    let cid = CircuitId::new();
    let old_var = ClassicalVar::new(cid, 0, ClassicalType::Bool);
    let new_var = ClassicalVar::new(cid, 9, ClassicalType::Bool);
    let old_value = ClassicalValue::new(cid, 0, ClassicalType::Bit);
    let new_value = ClassicalValue::new(cid, 8, ClassicalType::Bit);
    let expr = ClassicalExpr::try_and(
        old_var.expr(),
        ClassicalExpr::bit_to_bool(old_value.expr()).unwrap(),
    )
    .unwrap();
    let mut var_map = HashMap::new();
    var_map.insert(old_var, new_var);
    let mut value_map = HashMap::new();
    value_map.insert(old_value, new_value);
    let remapped = expr.remap_classical_ids(&var_map, &value_map).unwrap();
    assert!(remapped.vars().contains(&new_var));
    assert!(!remapped.vars().contains(&old_var));
    assert!(remapped.values().contains(&new_value));
    // 原表达式不受影响
    assert!(expr.vars().contains(&old_var));
    println!("[expr] remap_classical_ids：克隆重映射，原式不变");
}

#[test]
fn expr_unary_and_binary_operations() {
    let a = ClassicalExpr::bool_literal(true);
    let b = ClassicalExpr::bool_literal(false);
    // try_not / try_and / try_or / try_xor
    let not_a = ClassicalExpr::try_not(a.clone()).unwrap();
    assert_eq!(not_a.ty(), ClassicalType::Bool);
    let _and = ClassicalExpr::try_and(a.clone(), b.clone()).unwrap();
    let _or = ClassicalExpr::try_or(a.clone(), b.clone()).unwrap();
    let _xor = ClassicalExpr::try_xor(a.clone(), b.clone()).unwrap();
    // Bit 上的按位运算
    let bit_a = ClassicalExpr::bit_literal(true);
    let bit_b = ClassicalExpr::bit_literal(false);
    assert_eq!(
        ClassicalExpr::try_xor(bit_a.clone(), bit_b.clone()).unwrap().ty(),
        ClassicalType::Bit
    );
    // 类型不匹配：Bool × Bit
    assert!(ClassicalExpr::try_and(a.clone(), bit_a.clone()).is_err());
    // UInt × Bool
    assert!(ClassicalExpr::try_or(
        ClassicalExpr::uint_literal(8, 1).unwrap(),
        a.clone()
    )
    .is_err());
    // 运算符形式（文档用法）
    let var_a = ClassicalVar::new(CircuitId::new(), 0, ClassicalType::Bool);
    let var_b = ClassicalVar::new(CircuitId::new(), 1, ClassicalType::Bool);
    let _ = !var_a;
    let _ = var_a & var_b;
    let _ = var_a | var_b;
    let _ = var_a ^ var_b;
    println!("[expr] 一元/二元运算 + 运算符重载 + 类型混用拒绝全部验证");
}

#[test]
fn expr_comparison_operations() {
    let x = ClassicalExpr::uint_literal(8, 3).unwrap();
    let y = ClassicalExpr::uint_literal(8, 7).unwrap();
    for built in [
        ClassicalExpr::eq(x.clone(), y.clone()).unwrap(),
        ClassicalExpr::ne(x.clone(), y.clone()).unwrap(),
        ClassicalExpr::lt(x.clone(), y.clone()).unwrap(),
        ClassicalExpr::le(x.clone(), y.clone()).unwrap(),
        ClassicalExpr::gt(x.clone(), y.clone()).unwrap(),
        ClassicalExpr::ge(x.clone(), y.clone()).unwrap(),
    ] {
        assert_eq!(built.ty(), ClassicalType::Bool, "比较结果必为 Bool");
    }
    // eq/ne 支持所有类型（含 BitVec）
    let bv1 = ClassicalExpr::bit_vec_literal(4, 3).unwrap();
    let bv2 = ClassicalExpr::bit_vec_literal(4, 9).unwrap();
    assert!(ClassicalExpr::eq(bv1.clone(), bv2.clone()).is_ok());
    // 有序比较仅限 UInt
    assert!(ClassicalExpr::lt(bv1, bv2).is_err(), "BitVec 不支持 lt");
    assert!(ClassicalExpr::lt(
        ClassicalExpr::bool_literal(true),
        ClassicalExpr::bool_literal(false)
    )
    .is_err());
    // 类型不一致
    assert!(ClassicalExpr::eq(x, ClassicalExpr::uint_literal(4, 3).unwrap()).is_err());
    println!("[expr] 比较运算：eq/ne 全类型、lt/le/gt/ge 仅 UInt、类型校验严格");
}

#[test]
fn expr_cast_operations() {
    // Bit → Bool
    let bit = ClassicalExpr::bit_literal(true);
    let bool_ = ClassicalExpr::bit_to_bool(bit.clone()).unwrap();
    assert_eq!(bool_.ty(), ClassicalType::Bool);
    assert_eq!(ClassicalExpr::to_bool(bit).unwrap().ty(), ClassicalType::Bool);
    // BitVec(w) → UInt(w)
    let bv = ClassicalExpr::bit_vec_literal(8, 5).unwrap();
    let uint_ = ClassicalExpr::bit_vec_to_uint(bv.clone()).unwrap();
    assert_eq!(uint_.ty(), u8ty());
    assert_eq!(ClassicalExpr::to_uint(bv).unwrap().ty(), u8ty());
    // 错误方向
    assert!(ClassicalExpr::to_bool(ClassicalExpr::bool_literal(true)).is_err(), "Bool→Bool 拒绝");
    assert!(ClassicalExpr::to_uint(uint_.clone()).is_err(), "UInt→UInt 拒绝");
    assert!(ClassicalExpr::to_bool(uint_).is_err());
    println!("[expr] cast：Bit→Bool、BitVec→UInt 正确；冗余/非法 cast 拒绝");
}

#[test]
fn expr_select_operation() {
    let a = ClassicalExpr::uint_literal(8, 10).unwrap();
    let b = ClassicalExpr::uint_literal(8, 20).unwrap();
    let chosen = ClassicalExpr::select(ClassicalExpr::bool_literal(true), a.clone(), b.clone()).unwrap();
    assert_eq!(chosen.ty(), u8ty());
    // 分支类型不一致
    let c4 = ClassicalExpr::uint_literal(4, 1).unwrap();
    assert!(ClassicalExpr::select(ClassicalExpr::bool_literal(true), a, c4).is_err());
    // 条件非 Bool
    assert!(ClassicalExpr::select(
        ClassicalExpr::bit_literal(true),
        b.clone(),
        b.clone()
    )
    .is_err());
    println!("[expr] select：类型一致通过、分支不一致/条件非Bool拒绝");
}

#[test]
fn expr_bit_extraction() {
    let x = ClassicalExpr::uint_literal(8, 0b1010_0001).unwrap();
    // 提取 bit0 = 1
    let lsb = ClassicalExpr::extract_bit(x.clone(), 0).unwrap();
    assert_eq!(lsb.ty(), ClassicalType::Bit);
    // 提取高位段 [4..8)
    let hi = ClassicalExpr::extract_bits(x.clone(), 4, 4).unwrap();
    assert_eq!(hi.ty(), ClassicalType::bit_vec(4).unwrap());
    // 越界
    assert!(ClassicalExpr::extract_bit(x.clone(), 8).is_err());
    assert!(ClassicalExpr::extract_bits(x.clone(), 4, 5).is_err(), "4+5 > 8");
    // BitVec 同样支持
    let bv = ClassicalExpr::bit_vec_literal(8, 3).unwrap();
    assert!(ClassicalExpr::extract_bit(bv, 7).is_ok());
    println!("[expr] extract_bit/extract_bits：类型/边界校验全部正确");
}

#[test]
fn expr_concat_and_pack_bits() {
    let bit = ClassicalExpr::bit_literal(true);
    let tail = ClassicalExpr::bit_vec_literal(8, 5).unwrap();
    let joined = ClassicalExpr::concat([bit.clone(), tail.clone()]).unwrap();
    assert_eq!(joined.ty(), ClassicalType::bit_vec(9).unwrap(), "1+8=9");
    // 首 part 在最低位
    let _ = joined;
    // Bool 不可参与拼接
    assert!(ClassicalExpr::concat([ClassicalExpr::bool_literal(true), bit.clone()]).is_err());
    // pack_bits：首 bit 在 index 0
    let packed = ClassicalExpr::pack_bits([bit.clone(), ClassicalExpr::bit_literal(false)]).unwrap();
    assert_eq!(packed.ty(), ClassicalType::bit_vec(2).unwrap());
    // 非 Bit 输入拒绝
    assert!(ClassicalExpr::pack_bits([tail]).is_err());
    println!("[expr] concat 1+8→BitVec(9)；pack_bits → BitVec(2)；非法输入拒绝");
}

#[test]
fn expr_simplify_documented_rules() {
    let x = ClassicalVar::new(CircuitId::new(), 0, ClassicalType::Bool);
    let u = ClassicalExpr::uint_literal(8, 42).unwrap();
    // 双重否定 → 原式
    let nn = ClassicalExpr::try_not(ClassicalExpr::try_not(x.expr()).unwrap()).unwrap();
    assert_eq!(nn.simplified(), x.expr());
    // and(x, true) → x
    let and_true = ClassicalExpr::try_and(x.expr(), ClassicalExpr::bool_literal(true)).unwrap();
    assert_eq!(and_true.simplified(), x.expr());
    // or(x, false) → x
    let or_false = ClassicalExpr::try_or(x.expr(), ClassicalExpr::bool_literal(false)).unwrap();
    assert_eq!(or_false.simplified(), x.expr());
    // xor(x, x) → false
    let xor_self = ClassicalExpr::try_xor(x.expr(), x.expr()).unwrap();
    assert_eq!(xor_self.simplified(), ClassicalExpr::bool_literal(false));
    // and(x, !x) → false
    let and_not = ClassicalExpr::try_and(
        x.expr(),
        ClassicalExpr::try_not(x.expr()).unwrap(),
    )
    .unwrap();
    assert_eq!(and_not.simplified(), ClassicalExpr::bool_literal(false));
    // or(x, !x) → true
    let or_not = ClassicalExpr::try_or(x.expr(), ClassicalExpr::try_not(x.expr()).unwrap()).unwrap();
    assert_eq!(or_not.simplified(), ClassicalExpr::bool_literal(true));
    // eq(x, x) → true；ne → false
    assert_eq!(
        ClassicalExpr::eq(u.clone(), u.clone()).unwrap().simplified(),
        ClassicalExpr::bool_literal(true)
    );
    assert_eq!(
        ClassicalExpr::ne(u.clone(), u.clone()).unwrap().simplified(),
        ClassicalExpr::bool_literal(false)
    );
    // select 折叠
    let a = ClassicalExpr::uint_literal(8, 1).unwrap();
    let b = ClassicalExpr::uint_literal(8, 2).unwrap();
    assert_eq!(
        ClassicalExpr::select(ClassicalExpr::bool_literal(true), a.clone(), b.clone())
            .unwrap()
            .simplified(),
        a
    );
    assert_eq!(
        ClassicalExpr::select(ClassicalExpr::bool_literal(false), a.clone(), b.clone())
            .unwrap()
            .simplified(),
        b
    );
    // cast 折叠
    assert_eq!(
        ClassicalExpr::bit_to_bool(ClassicalExpr::bit_literal(true))
            .unwrap()
            .simplified(),
        ClassicalExpr::bool_literal(true)
    );
    assert_eq!(
        ClassicalExpr::bit_vec_to_uint(ClassicalExpr::bit_vec_literal(8, 7).unwrap())
            .unwrap()
            .simplified(),
        ClassicalExpr::uint_literal(8, 7).unwrap()
    );
    println!("[simplify] 文档 23 条规则中 12 条核心规则全部复现");
}

#[test]
fn expr_simplify_idempotent() {
    let x = ClassicalVar::new(CircuitId::new(), 0, ClassicalType::Bool);
    let expr = ClassicalExpr::try_or(
        ClassicalExpr::try_and(x.expr(), ClassicalExpr::bool_literal(true)).unwrap(),
        ClassicalExpr::bool_literal(false),
    )
    .unwrap();
    let once = expr.simplified();
    let twice = once.simplified();
    assert_eq!(once, twice, "化简应幂等");
    println!("[simplify] 幂等性验证通过");
}

// ==================== 九、validate 深度校验（SSA / 作用域 / 终止位置） ====================

/// 构造值层经典操作的辅助函数。
fn classical_value_op(instruction: cqlib_core::circuit::Instruction, qubits: &[Qubit]) -> ValueOperation {
    ValueOperation {
        instruction: ValueInstruction::Instruction(instruction),
        qubits: qubits.iter().copied().collect(),
        params: smallvec![],
        label: None,
    }
}

#[test]
fn validate_undefined_value_in_table() {
    // 值表登记了 Bit，但没有任何操作定义它
    let err = Circuit::from_operations(
        vec![q(0)],
        [],
        None,
        Some(vec![ClassicalType::Bit]),
    )
    .unwrap_err();
    assert!(matches!(err, CircuitError::UndefinedClassicalValue { index: 0, .. }));
    println!("[validate] 值表存在未定义值 → {err:?}");
}

#[test]
fn validate_duplicate_measurement_definition() {
    // 两个测量写同一个值句柄
    let cid = CircuitId::new();
    let result = ClassicalValue::new(cid, 0, ClassicalType::Bit);
    let m1 = classical_value_op(
        Instruction::ClassicalData(ClassicalDataOp::MeasureBit { result }),
        &[q(0)],
    );
    let m2 = classical_value_op(
        Instruction::ClassicalData(ClassicalDataOp::MeasureBit { result }),
        &[q(0)],
    );
    let err = Circuit::from_operations(
        vec![q(0)],
        [m1, m2],
        None,
        Some(vec![ClassicalType::Bit]),
    )
    .unwrap_err();
    assert!(matches!(err, CircuitError::DuplicateClassicalValueDefinition { index: 0, .. }));
    println!("[validate] 同一值双定义 → {err:?}");
}

#[test]
fn validate_value_escapes_control_flow_region() {
    // if 体内测量定义的值，在顶层读取 → 越界
    let cid = CircuitId::new();
    let v0 = ClassicalValue::new(cid, 0, ClassicalType::Bit);
    let inner_measure = classical_value_op(
        Instruction::ClassicalData(ClassicalDataOp::MeasureBit { result: v0 }),
        &[q(0)],
    );
    let defining_if = ValueOperation {
        instruction: ValueInstruction::ClassicalControl(ValueClassicalControlOp::If {
            condition: ClassicalExpr::bool_literal(true),
            then_body: ValueControlBody::new(vec![inner_measure]),
            else_body: None,
        }),
        qubits: smallvec![q(0)],
        params: smallvec![],
        label: None,
    };
    let reading_if = ValueOperation {
        instruction: ValueInstruction::ClassicalControl(ValueClassicalControlOp::If {
            condition: ClassicalExpr::bit_to_bool(v0.expr()).unwrap(),
            then_body: ValueControlBody::new(vec![]),
            else_body: None,
        }),
        qubits: smallvec![],
        params: smallvec![],
        label: None,
    };
    let err = Circuit::from_operations(
        vec![q(0)],
        [defining_if, reading_if],
        None,
        Some(vec![ClassicalType::Bit]),
    )
    .unwrap_err();
    assert!(matches!(err, CircuitError::ClassicalValueOutOfScope { index: 0, .. }));
    println!("[validate] 分支局部值逃逸作用域 → {err:?}");
}

#[test]
fn validate_nonterminal_break_from_operations() {
    // while 体：[break, x] → break 必须是最后一条
    let cid = CircuitId::new();
    let _ = cid;
    let break_op = ValueOperation {
        instruction: ValueInstruction::ClassicalControl(ValueClassicalControlOp::Break),
        qubits: smallvec![],
        params: smallvec![],
        label: None,
    };
    let x_op = ValueOperation {
        instruction: ValueInstruction::Instruction(Instruction::Standard(
            cqlib_core::circuit::StandardGate::X,
        )),
        qubits: smallvec![q(0)],
        params: smallvec![],
        label: None,
    };
    let while_op = ValueOperation {
        instruction: ValueInstruction::ClassicalControl(ValueClassicalControlOp::While {
            condition: ClassicalExpr::bool_literal(true),
            body: ValueControlBody::new(vec![break_op, x_op]),
        }),
        qubits: smallvec![q(0)],
        params: smallvec![],
        label: None,
    };
    let err = Circuit::from_operations(vec![q(0)], [while_op], None, None).unwrap_err();
    assert!(matches!(
        err,
        CircuitError::NonTerminalControlTransfer { operation: "break", .. }
    ));
    println!("[validate] 非终止 break → {err:?}");
}

#[test]
fn validate_break_outside_loop_rejected_via_from_operations() {
    let break_op = ValueOperation {
        instruction: ValueInstruction::ClassicalControl(ValueClassicalControlOp::Break),
        qubits: smallvec![],
        params: smallvec![],
        label: None,
    };
    let err = Circuit::from_operations(vec![q(0)], [break_op], None, None).unwrap_err();
    assert!(matches!(err, CircuitError::InvalidOperation(_)));
    println!("[validate] 顶层 break（值层构造）→ {err:?}");
}

// ==================== 十、综合可靠性 ====================

#[test]
fn reliability_full_dynamic_circuit_roundtrip() {
    // 完整动态线路：测量 → 分支 → 循环 → 校验
    let mut c = Circuit::new(2);
    let running = c.var(ClassicalType::Bool);
    c.store(running, ClassicalExpr::bool_literal(true)).unwrap();
    c.while_(running.expr(), |body| {
        let measured = body.measure(q(0))?;
        body.store(running, ClassicalExpr::bit_to_bool(measured.expr())?)?;
        body.if_(ClassicalExpr::bit_to_bool(measured.expr())?, |inner| {
            inner.x(q(1))?;
            Ok(())
        })?;
        body.break_loop()?;
        Ok(())
    })
    .unwrap();
    c.validate().unwrap();
    // 深度估计可用
    let depth = c.depth(true).unwrap();
    assert!(depth >= 3);
    // 结构相等：克隆后一致
    assert_eq!(c, c.clone());
    println!("[reliability] 复合动态线路：validate + depth(true)={depth} + 克隆一致");
}

#[test]
fn reliability_builder_vs_from_operations_equivalence() {
    // 构建器构建的线路与 validate 后的语义一致：同构重建两遍相等
    let build = || {
        let mut c = Circuit::new(2);
        let m = c.measure(q(0)).unwrap();
        c.if_(ClassicalExpr::bit_to_bool(m.expr()).unwrap(), |body| {
            body.x(q(1))?;
            Ok(())
        })
        .unwrap();
        c
    };
    let a = build();
    let b = build();
    assert_eq!(a, b, "两次独立构建结构相等");
    a.validate().unwrap();
    b.validate().unwrap();
    println!("[reliability] 两次独立构建动态线路结构相等");
}
