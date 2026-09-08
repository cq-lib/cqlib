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

"""
Quantum Information Science (QIS) module.

This module provides quantum information tools including:
- Pauli operators and strings
- Hamiltonian construction
- Statevector simulation
- Density matrix simulation
- Quantum metrics and entanglement measures
- Pauli evolution and Trotter decomposition

# Examples

```python
from cqlib.qis import Pauli, PauliString, Phase, Hamiltonian, TrotterMode

# Create Pauli operators
x = Pauli.x()
z = Pauli.z()

# Multiplication with phase tracking
result, phase = x.mul_with_phase(z)  # X * Z = -iY

# Create Pauli string
ps = PauliString.from_str("XZI")  # X ⊗ Z ⊗ I
print(ps)  # +XZI

# Check commutation
ps2 = PauliString.from_str("ZXI")
commutes = ps.commutes_with(ps2)

# Create Hamiltonian
h = Hamiltonian(2)
h.add_term(PauliString.from_str("ZZ"), 0.5)
h.add_term(PauliString.from_str("XX"), 0.3)
h.simplify()

# Trotter decomposition
circuit = h.to_trotter_circuit(1.0, 10, TrotterMode.first_order())
```
"""

from typing import Protocol, List, Tuple, Dict

from . import state

# Pauli module
from .pauli import Phase as Phase
from .pauli import Pauli as Pauli
from .pauli import PauliString as PauliString

# Hamiltonian module
from .hamiltonian import Hamiltonian as Hamiltonian

# Evolution module
from .evolution import TrotterMode as TrotterMode

# Entropy and metrics modules
from . import entropy as entropy
from . import metrics as metrics


# Expose key state classes at qis level for convenience
DensityMatrix = state.DensityMatrix
Statevector = state.Statevector
DensityMatrixNoise = state.DensityMatrixNoise
RuntimeValue = state.RuntimeValue
ClassicalState = state.ClassicalState
StabilizerState = state.StabilizerState
StabilizerCircuitResult = state.StabilizerCircuitResult


class Observable(Protocol):
    """Protocol for quantum observables."""

    def expectation_statevector(self, sv: Statevector) -> float: ...
    def expectation_density_matrix(self, dm: DensityMatrix) -> float: ...
    def expectation_probs(
        self, measurements: List[Tuple[PauliString, Dict[str, float]]]
    ) -> float: ...
    def variance_statevector(self, sv: Statevector) -> float: ...
    @property
    def num_qubits(self) -> int: ...


__all__ = [
    "Phase",
    "Pauli",
    "PauliString",
    "Hamiltonian",
    "TrotterMode",
    "state",
    "Statevector",
    "DensityMatrix",
    "DensityMatrixNoise",
    "RuntimeValue",
    "ClassicalState",
    "StabilizerState",
    "StabilizerCircuitResult",
    "Observable",
    "entropy",
    "metrics",
]
