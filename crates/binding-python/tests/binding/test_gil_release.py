import threading

import numpy as np

from cqlib import Circuit
from cqlib.qis.state import (
    DensityMatrix,
    DensityMatrixNoise,
    StabilizerState,
    Statevector,
)


def _assert_python_thread_progresses(action):
    ready = threading.Event()
    stop = threading.Event()
    progress = [0]

    def worker():
        ready.set()
        while not stop.is_set():
            progress[0] += 1

    thread = threading.Thread(target=worker)
    thread.start()
    ready.wait()
    before = progress[0]
    try:
        action()
    finally:
        stop.set()
        thread.join()

    assert progress[0] - before > 100


def test_circuit_to_matrix_releases_gil():
    circuit = Circuit(10)
    for _ in range(4):
        for qubit in range(10):
            circuit.h(qubit)
        for qubit in range(9):
            circuit.cx(qubit, qubit + 1)

    _assert_python_thread_progresses(circuit.to_matrix)


def test_statevector_apply_circuit_releases_gil():
    circuit = Circuit(20)
    for _ in range(6):
        for qubit in range(20):
            circuit.h(qubit)
        for qubit in range(19):
            circuit.cx(qubit, qubit + 1)
    state = Statevector(20)

    _assert_python_thread_progresses(lambda: state.apply_circuit(circuit))


def test_statevector_probabilities_releases_gil():
    state = Statevector(20)

    _assert_python_thread_progresses(state.probabilities)


def test_density_matrix_unitary_releases_gil():
    state = DensityMatrix(9)
    matrix = np.array([[0.0, 1.0], [1.0, 0.0]], dtype=complex)

    _assert_python_thread_progresses(lambda: state.apply_unitary_gate([0], matrix))


def test_noise_simulator_unitary_releases_gil():
    state = DensityMatrixNoise(9)
    matrix = np.array([[0.0, 1.0], [1.0, 0.0]], dtype=complex)

    _assert_python_thread_progresses(lambda: state.apply_unitary_gate([0], matrix))


def test_stabilizer_probabilities_releases_gil():
    state = StabilizerState(18)

    _assert_python_thread_progresses(state.probabilities)
