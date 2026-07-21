# This code is part of Cqlib.
#
# (C) Copyright China Telecom Quantum Group 2026
#
# This code is licensed under the Apache License, Version 2.0. You may
# obtain a copy of this license in the LICENSE.txt file in the root directory
# of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
#
# Any modifications or derivative works of this code must retain this
# copyright notice, and modified files need to carry a notice indicating
# that they have been altered from the originals.

"""Type hints for OpenQASM 3.0 module.

This module provides type hints for parsing and serializing OpenQASM 3.0
programs.
"""

from typing import Literal

from ..circuit import Circuit

def loads(qasm: str) -> Circuit:
    """Parse an OpenQASM 3.0 string into a Circuit.

    Args:
        qasm: A string containing OpenQASM 3.0 code.

    Returns:
        A Circuit object representing the parsed quantum circuit.

    Raises:
        ValueError: If the QASM string is invalid, cannot be parsed, or uses
            OpenQASM 3.0 features unsupported by the current circuit IR.
            Pure physical-qubit programs using ``$n`` are supported, but
            mixing physical and declared logical qubits is not.

    Example:
        >>> qasm_code = '''
        ... OPENQASM 3;
        ... include "stdgates.inc";
        ... qubit[2] q;
        ... h q[0];
        ... cx q[0], q[1];
        ... '''
        >>> circuit = loads(qasm_code)
    """
    ...

def load(path: str) -> Circuit:
    """Load and parse an OpenQASM 3.0 file into a Circuit.

    Args:
        path: Path to the QASM file.

    Returns:
        A Circuit object.

    Raises:
        ValueError: If parsing fails or the source uses unsupported features.
            Pure physical-qubit programs using ``$n`` are supported, but
            mixing physical and declared logical qubits is not.
        OSError: If file cannot be read.

    Example:
        >>> circuit = load("/path/to/circuit.qasm")
    """
    ...

def dumps(
    circuit: Circuit, *, qubit_mode: Literal["auto", "logical", "physical"] = "auto"
) -> str:
    """Serialize a Circuit to an OpenQASM 3.0 string.

    Args:
        circuit: The Circuit object to serialize.
        qubit_mode: ``"auto"`` follows ``circuit.qubit_domain``. Explicit
            modes emit compact logical qubits or preserve circuit qubit indices
            as OpenQASM 3 physical qubits.

    Returns:
        A string containing the OpenQASM 3.0 representation.

    Raises:
        ValueError: If the circuit contains instructions that cannot be
            represented in OpenQASM 3.0.

    Example:
        >>> from cqlib import Circuit, Qubit
        >>> circuit = Circuit(2)
        >>> circuit.h(Qubit(0))
        >>> circuit.cx(Qubit(0), Qubit(1))
        >>> qasm_str = dumps(circuit)
        >>> print(qasm_str)
    """
    ...

def dump(
    circuit: Circuit,
    path: str,
    *,
    qubit_mode: Literal["auto", "logical", "physical"] = "auto",
) -> None:
    """Serialize a Circuit to an OpenQASM 3.0 file.

    Args:
        circuit: The Circuit object to serialize.
        path: Path to the output file.
        qubit_mode: ``"auto"`` follows ``circuit.qubit_domain``. Explicit
            modes emit compact logical qubits or preserve circuit qubit indices
            as OpenQASM 3 physical qubits.

    Raises:
        ValueError: If the circuit contains unsupported instructions.
        OSError: If file cannot be written.

    Example:
        >>> from cqlib import Circuit, Qubit
        >>> circuit = Circuit(2)
        >>> circuit.h(Qubit(0))
        >>> circuit.cx(Qubit(0), Qubit(1))
        >>> dump(circuit, "output.qasm")
    """
    ...
