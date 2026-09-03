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

import copy
import math
import sys

import pytest

from cqlib.circuit import (
    Instruction,
    Parameter,
    Qubit,
    StandardGate,
    ValueInstruction,
    ValueOperation,
)
from cqlib.compile import commutation
from cqlib.compile.commutation import (
    Commutation,
    CommutationChecker,
    CommutationConfig,
    algebraic_commutation,
    check_commutation,
)
from cqlib.compile.knowledge import RuleKind, RuleLibrary


def operation(gate: StandardGate, qubits: list[int]) -> ValueOperation:
    return ValueOperation.from_standard_gate(gate, [Qubit(index) for index in qubits])


def test_commutation_modules_and_public_exports_are_registered():
    assert commutation.check_commutation is check_commutation
    assert "cqlib._native.compile.commutation" in sys.modules
    assert Commutation.__module__ == "cqlib.compile.commutation"
    assert CommutationConfig.__module__ == "cqlib.compile.commutation"
    assert CommutationChecker.__module__ == "cqlib.compile.commutation"


def test_builtin_checker_proves_exact_and_global_phase_commutation():
    disjoint = check_commutation(
        operation(StandardGate.H, [0]), operation(StandardGate.X, [1])
    )
    assert disjoint == Commutation.exact()
    assert disjoint.is_exact()
    assert disjoint.phase.evaluate() == 0.0

    global_phase = check_commutation(
        operation(StandardGate.X, [0]), operation(StandardGate.Z, [0])
    )
    assert global_phase is not None
    assert not global_phase.is_exact()
    assert math.isclose(global_phase.phase.evaluate(), math.pi, abs_tol=1e-10)
    assert "Parameter(" in repr(global_phase)


def test_symbolic_parameters_are_preserved_for_algebraic_proofs():
    lhs = operation(StandardGate.RZ(Parameter("a")), [0])
    rhs = operation(StandardGate.RZ(Parameter("b")), [0])

    proof = algebraic_commutation(lhs, rhs)

    assert proof == Commutation.exact()


def test_unproven_and_malformed_applications_return_none():
    assert (
        check_commutation(
            operation(StandardGate.H, [0]), operation(StandardGate.X, [0])
        )
        is None
    )
    assert (
        check_commutation(
            operation(StandardGate.CX, [0]), operation(StandardGate.X, [0])
        )
        is None
    )

    delay = ValueOperation(ValueInstruction.from_instruction(Instruction.delay()), [])
    assert check_commutation(delay, delay) is None


def test_checker_configuration_and_copy_protocols():
    config = CommutationConfig(
        enable_rule_oracle=False,
        enable_matrix_fallback=False,
        max_matrix_qubits=2,
    )
    checker = CommutationChecker.with_config(config)

    assert checker.config == config
    assert checker.config is not config
    assert copy.copy(config) == config
    assert copy.deepcopy(config) == config
    assert copy.copy(checker).config == config
    assert copy.deepcopy(checker).config == config
    assert "max_matrix_qubits=2" in repr(checker)

    proof = checker.check(
        operation(StandardGate.X, [0]), operation(StandardGate.Z, [0])
    )
    assert proof is not None
    assert copy.copy(proof) == proof
    assert copy.deepcopy(proof) == proof


def test_from_library_uses_custom_commute_rules():
    # U(theta, 0, 0) rotations stay on the X axis, so two of them commute.
    # The symbolic algebra oracle has no U-gate family, which makes this pair
    # provable only through the custom rule below once matrix fallback is off.
    library = RuleLibrary.from_dsl(
        "rule comm_u_xaxis {"
        " match { U(p, q, r) 0, U(s, t, v) 0 }"
        " require { q == 0, t == 0 }"
        " rewrite { U(s, t, v) 0, U(p, q, r) 0 }"
        " }",
        RuleKind.commute(),
    )
    config = CommutationConfig(enable_matrix_fallback=False)
    lhs = operation(StandardGate.U(0.3, 0.0, 0.0), [0])
    rhs = operation(StandardGate.U(0.7, 0.0, 0.0), [0])

    checker = CommutationChecker.from_library(library, config)
    assert checker.check(lhs, rhs) == Commutation.exact()

    # Without the custom rule the pair is unproven: the algebra oracle has no
    # U family and matrix fallback is disabled.
    assert (
        CommutationChecker.from_library(RuleLibrary(), config).check(lhs, rhs) is None
    )
    assert CommutationChecker.with_config(config).check(lhs, rhs) is None

    # The rule condition keeps the proof sound for off-axis U rotations.
    off_axis = operation(StandardGate.U(0.3, 0.5, 0.0), [0])
    assert checker.check(off_axis, rhs) is None


def test_from_library_default_config_matches_builtin():
    checker = CommutationChecker.from_library(RuleLibrary.builtin())

    assert checker.config == CommutationChecker.builtin().config
    assert checker.config == CommutationConfig()

    lhs = operation(StandardGate.S, [0])
    rhs = operation(StandardGate.T, [0])
    assert checker.check(lhs, rhs) == Commutation.exact()
    assert checker.check(lhs, rhs) == CommutationChecker.builtin().check(lhs, rhs)


def test_from_library_rejects_disabled_rule_oracle():
    config = CommutationConfig(enable_rule_oracle=False)

    with pytest.raises(ValueError, match="enable_rule_oracle"):
        CommutationChecker.from_library(RuleLibrary.builtin(), config)
