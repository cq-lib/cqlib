//! binding-c（C 绑定）模块的测试程序
//!
//! 被测目标：`crates/binding-c`（cqlib 的 C ABI 层，1300+ 个导出函数）
//! 测试方式：integration test —— 通过 `binding_c` crate 公开导出的 extern "C" 函数调用，
//!           以裸指针 + i32 错误码 + 手动 free 的方式完全模拟 C 调用方视角。
//!
//! 覆盖内容：
//! 1. 电路生命周期（new/free/查询/校验）
//! 2. 门追加与错误码（越界 / NaN / NULL）
//! 3. 空指针安全
//! 4. 态矢模拟（Bell 态概率、就地演化、测量、采样）
//! 5. 线路酉矩阵导出（复数交错缓冲）
//! 6. 编译流水线（compile + 步骤报告）
//! 7. QASM2 载入/导出与字符串释放纪律
//!
//! 运行命令：cargo test -p binding-c --test c_binding_program_test -- --nocapture

use std::ffi::{c_char, CStr, CString};

use binding_c::circuit::{
    circuit_cx, circuit_free, circuit_h, circuit_measure, circuit_new, circuit_num_operations,
    circuit_num_parameters, circuit_num_qubits, circuit_rx, circuit_to_matrix,
    circuit_to_matrix_len, circuit_validate, circuit_x, CCircuit,
};
use binding_c::compile::{
    compile, compile_result_changed, compile_result_circuit, compile_result_free,
    compile_result_num_steps, compile_result_step_name, CompileConfigC,
};
use binding_c::ir::qasm2::{qasm2_dumps, qasm2_loads};
use binding_c::qis::statevector::{
    outcome_list_free, outcome_list_get, outcome_list_len, statevector_apply_circuit,
    statevector_free, statevector_from_circuit, statevector_measure, statevector_measure_all,
    statevector_new, statevector_num_qubits, statevector_probabilities,
    statevector_probabilities_len, statevector_sample_shots,
};
use binding_c::cqlib_string_free;

/// 错误码（与 binding-c/src/error.rs 的 CqlibError 一致）
const OK: i32 = 0;
const NULL_PTR: i32 = -1;
const QUBIT_OUT_OF_BOUNDS: i32 = -2;
const CIRCUIT_ERROR: i32 = -3;

/// 把 FFI 返回的 C 字符串读为 String 并立即释放。
fn take_c_string(ptr: *mut c_char) -> String {
    if ptr.is_null() {
        return String::from("<null>");
    }
    let text = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned();
    cqlib_string_free(ptr);
    text
}

/// 构造 Bell 态线路（H(0); CX(0,1)）。
fn make_bell_circuit() -> *mut CCircuit {
    let circuit = circuit_new(2);
    assert!(!circuit.is_null(), "circuit_new(2) 失败");
    assert_eq!(circuit_h(circuit, 0), OK);
    assert_eq!(circuit_cx(circuit, 0, 1), OK);
    circuit
}

#[test]
fn c_lifecycle_and_queries() {
    let circuit = circuit_new(3);
    assert!(!circuit.is_null());
    assert_eq!(circuit_num_qubits(circuit), 3);
    assert_eq!(circuit_num_operations(circuit), 0);
    assert_eq!(circuit_num_parameters(circuit), 0);
    assert_eq!(circuit_validate(circuit), OK);
    println!(
        "[c] circuit_new(3): qubits={} ops={} params={} validate={}",
        circuit_num_qubits(circuit),
        circuit_num_operations(circuit),
        circuit_num_parameters(circuit),
        circuit_validate(circuit)
    );
    circuit_free(circuit);
}

#[test]
fn c_gate_append_and_error_codes() {
    let circuit = circuit_new(2);
    assert_eq!(circuit_h(circuit, 0), OK);
    assert_eq!(circuit_cx(circuit, 0, 1), OK);
    assert_eq!(circuit_rx(circuit, 0, std::f64::consts::FRAC_PI_2), OK);
    assert_eq!(circuit_num_operations(circuit), 3);
    assert_eq!(circuit_validate(circuit), OK);

    // 越界 qubit → -2（QubitOutOfBounds）
    let rc_oob = circuit_h(circuit, 2);
    assert_eq!(rc_oob, QUBIT_OUT_OF_BOUNDS, "越界门应返回 -2，实际 {rc_oob}");
    // NaN 参数 → -3（CircuitError）
    let rc_nan = circuit_rx(circuit, 0, f64::NAN);
    assert_eq!(rc_nan, CIRCUIT_ERROR, "NaN 参数应返回 -3，实际 {rc_nan}");
    // 测量合法
    assert_eq!(circuit_measure(circuit, 1), OK);
    assert_eq!(circuit_num_operations(circuit), 4);

    println!(
        "[c] 门追加后 ops={}，越界 h(2) → {rc_oob}，NaN rx → {rc_nan}，measure → OK",
        circuit_num_operations(circuit)
    );
    circuit_free(circuit);
}

#[test]
fn c_null_pointer_safety() {
    // 所有“NULL 允许”的查询应安全返回 0/哨兵值
    assert_eq!(circuit_num_qubits(std::ptr::null()), 0);
    assert_eq!(circuit_num_operations(std::ptr::null()), 0);
    assert_eq!(circuit_validate(std::ptr::null()), NULL_PTR);
    assert_eq!(circuit_to_matrix_len(std::ptr::null(), std::ptr::null(), 0), 0);
    assert_eq!(statevector_probabilities_len(std::ptr::null()), 0);
    assert_eq!(
        statevector_probabilities(std::ptr::null(), std::ptr::null_mut(), 0),
        NULL_PTR
    );
    assert_eq!(qasm2_dumps(std::ptr::null()), std::ptr::null_mut());

    // 所有 free 函数应容忍 NULL
    circuit_free(std::ptr::null_mut());
    statevector_free(std::ptr::null_mut());
    outcome_list_free(std::ptr::null_mut());
    compile_result_free(std::ptr::null_mut());
    cqlib_string_free(std::ptr::null_mut());
    println!("[c] 空指针安全：查询返回 0/-1，free(NULL) 全部放行 ✓");
}

#[test]
fn c_statevector_bell_probabilities() {
    let circuit = make_bell_circuit();
    let state = statevector_from_circuit(circuit);
    assert!(!state.is_null(), "statevector_from_circuit 失败");
    assert_eq!(statevector_num_qubits(state), 2);

    let len = statevector_probabilities_len(state);
    assert_eq!(len, 4);
    let mut probs = [0.0f64; 4];
    assert_eq!(statevector_probabilities(state, probs.as_mut_ptr(), len), OK);

    println!("[c] Bell 态概率 = {probs:?}");
    assert!((probs[0] - 0.5).abs() < 1e-12, "|00⟩ 概率应 0.5");
    assert!(probs[1].abs() < 1e-12 && probs[2].abs() < 1e-12);
    assert!((probs[3] - 0.5).abs() < 1e-12, "|11⟩ 概率应 0.5");

    circuit_free(circuit);
    statevector_free(state);
}

#[test]
fn c_statevector_apply_and_measure() {
    // 线路 X(0) → |01⟩（MSB=qubit1, LSB=qubit0）；就地应用后测量
    let circuit = circuit_new(2);
    assert_eq!(circuit_x(circuit, 0), OK);
    let state = statevector_new(2);
    assert!(!state.is_null());
    assert_eq!(statevector_apply_circuit(state, circuit), OK);

    assert_eq!(statevector_measure(state, 0), 1, "qubit0 应为 |1⟩");
    assert_eq!(statevector_measure(state, 1), 0, "qubit1 应为 |0⟩");

    let bitstring = take_c_string(statevector_measure_all(state));
    println!("[c] X(0) 后 measure_all = \"{bitstring}\"");
    assert_eq!(bitstring, "01", "MSB=qubit1(0), LSB=qubit0(1)");

    circuit_free(circuit);
    statevector_free(state);
}

#[test]
fn c_statevector_sample_shots() {
    let circuit = make_bell_circuit();
    let state = statevector_from_circuit(circuit);
    let list = statevector_sample_shots(state, 2048);
    assert!(!list.is_null());
    assert_eq!(outcome_list_len(list), 2048);

    let mut zeros = 0usize;
    let mut ones = 0usize;
    for index in 0..2048 {
        let text = take_c_string(outcome_list_get(list, index));
        match text.as_str() {
            "00" => zeros += 1,
            "11" => ones += 1,
            other => panic!("非法采样结果 {other:?}"),
        }
    }
    println!("[c] 2048 次采样：\"00\"={zeros} 次、\"11\"={ones} 次（合计 {}）", zeros + ones);
    assert_eq!(zeros + ones, 2048);

    outcome_list_free(list);
    circuit_free(circuit);
    statevector_free(state);
}

#[test]
fn c_circuit_to_matrix_bell() {
    let circuit = make_bell_circuit();
    let complex_len = circuit_to_matrix_len(circuit, std::ptr::null(), 0);
    assert_eq!(complex_len, 16, "2 比特线路矩阵应为 4×4=16 个复数");

    let mut buffer = [0.0f64; 32];
    let written = circuit_to_matrix(circuit, std::ptr::null(), 0, buffer.as_mut_ptr(), 32);
    assert_eq!(written, 16);

    // 第一列 = U|00⟩ = (|00⟩+|11⟩)/√2：元素 0 与元素 12 为 1/√2
    let amplitude = std::f64::consts::FRAC_1_SQRT_2;
    assert!((buffer[0] - amplitude).abs() < 1e-12, "m[0,0].re = {}", buffer[0]);
    assert!(buffer[1].abs() < 1e-12);
    assert!((buffer[12] - amplitude).abs() < 1e-12, "m[3,0].re = {}", buffer[12]);
    assert!(buffer[13].abs() < 1e-12);
    println!(
        "[c] to_matrix: m[0,0]=({:.6},{:.6}) m[3,0]=({:.6},{:.6})（期望 1/√2）",
        buffer[0], buffer[1], buffer[12], buffer[13]
    );

    circuit_free(circuit);
}

#[test]
fn c_compile_pipeline_and_steps() {
    let circuit = make_bell_circuit();
    let config = CompileConfigC {
        mode: 0,
        target: 0,
        allow_dirty_ancilla: 0,
        allow_clean_ancilla: 0,
        _reserved: [0u8; 4],
    };
    let result = compile(circuit, config);
    assert!(!result.is_null(), "compile 失败（返回 NULL）");

    let steps = compile_result_num_steps(result);
    let changed = compile_result_changed(result);
    let compiled = compile_result_circuit(result);
    assert!(!compiled.is_null(), "compile 结果线路为空");
    assert_eq!(circuit_num_qubits(compiled), 2);

    let mut step_names = Vec::new();
    for index in 0..steps {
        let text = take_c_string(compile_result_step_name(result, index));
        assert!(!text.is_empty(), "步骤 {index} 名称不应为空");
        step_names.push(text);
    }

    println!("[c] compile(Bell, Logical) → 步骤数={steps} changed={changed} 步骤={step_names:?}");

    circuit_free(compiled);
    compile_result_free(result);
    circuit_free(circuit);
}

#[test]
fn c_qasm2_roundtrip_and_string_discipline() {
    // 载入 QASM2 文本 → 电路
    let source = CString::new(
        "OPENQASM 2.0;\ninclude \"qelib1.inc\";\nqreg q[2];\nh q[0];\ncx q[0],q[1];\n",
    )
    .unwrap();
    let circuit = qasm2_loads(source.as_ptr());
    assert!(!circuit.is_null(), "QASM2 载入失败");
    assert_eq!(circuit_num_qubits(circuit), 2);
    assert_eq!(circuit_num_operations(circuit), 2);

    // 导出 → 字符串；必须包含 h 与 cx
    let text = take_c_string(qasm2_dumps(circuit));
    assert!(text.contains("h q[0]"), "导出应含 h，实际: {text}");
    assert!(text.contains("cx q[0],q[1]"), "导出应含 cx，实际: {text}");
    println!("[c] QASM2 往返成功：载入 2 操作 → 导出文本长度 {}", text.len());

    // 非法 QASM → NULL（无崩溃）
    let bad = CString::new("this is not qasm").unwrap();
    let bad_circuit = qasm2_loads(bad.as_ptr());
    assert!(bad_circuit.is_null(), "非法 QASM 应返回 NULL");

    circuit_free(circuit);
}