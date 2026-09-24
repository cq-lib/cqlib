// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

use crate::circuit::{bit::PyIntListOrQubitList, circuit_impl::PyCircuit};
use crate::device::result::PyExecutionResult;
use crate::qis::qis_error_to_py_err;
use cqlib_core::circuit::Qubit;
use cqlib_core::qis::Simulator;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Sample a circuit from the all-zero state and return an ExecutionResult.
///
/// Args:
///     circuit: Circuit to simulate. The input is not modified.
///     shots: Positive number of samples, default 1000.
///     seed: Optional unsigned 64-bit seed. Counts are reproducible for the
///         same circuit, options, simulator and library version.
///     simulator: "statevector" (default), "density_matrix", or "stabilizer".
///     qubits: Optional ordered list of qubit IDs or Qubit objects to sample.
///         By default, terminal measurement declarations select the outputs
///         in first-occurrence order, with duplicates removed. If there are
///         no measurements, all circuit qubits are sampled in circuit order.
///
/// Measurement is in the Z basis. The first output qubit corresponds to the
/// rightmost bit of each counts key; result.qubits records the output order.
/// The result is completed, and result.probabilities contains shot frequencies.
///
/// Parameters must be bound. Only terminal measurements are supported;
/// classical control flow is rejected. Barrier and Delay are no-ops. Store
/// does not affect the quantum measurement counts. Reset requires
/// simulator="density_matrix". The stabilizer simulator only accepts its
/// supported Clifford gates. These simulators do not apply a noise model.
///
/// Raises:
///     ValueError: For invalid options, empty output selection, or unsupported
///         circuit operations.
///     TypeError: For invalid argument types.
///     OverflowError: For negative or out-of-range shots or seed.
///
/// Example:
///     >>> from cqlib import Circuit, sample
///     >>> circuit = Circuit(2)
///     >>> circuit.h(0)
///     >>> circuit.cx(0, 1)
///     >>> result = sample(circuit, shots=1000, seed=42)
///     >>> sum(result.counts.values())
///     1000
#[pyfunction]
#[pyo3(signature = (circuit, *, shots=1000, seed=None, simulator="statevector", qubits=None))]
pub fn sample(
    py: Python<'_>,
    circuit: &PyCircuit,
    shots: usize,
    seed: Option<u64>,
    simulator: &str,
    qubits: Option<PyIntListOrQubitList>,
) -> PyResult<PyExecutionResult> {
    let simulator = match simulator {
        "statevector" => Simulator::Statevector,
        "density_matrix" => Simulator::DensityMatrix,
        "stabilizer" => Simulator::Stabilizer,
        _ => {
            return Err(PyValueError::new_err(
                "simulator must be 'statevector', 'density_matrix', or 'stabilizer'",
            ));
        }
    };
    let qubits: Option<Vec<Qubit>> = qubits.map(Into::into);
    py.detach(|| cqlib_core::qis::sample(&circuit.inner, shots, seed, simulator, qubits.as_deref()))
        .map(PyExecutionResult::from)
        .map_err(qis_error_to_py_err)
}
