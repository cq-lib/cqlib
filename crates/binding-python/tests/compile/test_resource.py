# This code is part of Cqlib.
#
# (C) Copyright China Telecom Quantum Group 2026
#
# This code is licensed under the Apache License, Version 2.0. You may
# obtain a copy of this license in the LICENSE.txt file in the root directory
# of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.

import copy
import sys

import pytest

from cqlib.circuit import Circuit, CqlibError, MCGate, Qubit, StandardGate
from cqlib.compile import compile, resource
from cqlib.compile.resource import (
    AncillaRequirement,
    ResourceError,
    ResourceLease,
    ResourceLimits,
    ResourceManager,
    ResourcePlan,
    ResourcePolicy,
    ResourceRequest,
    ResourceUnavailableError,
)


def mcx_circuit(num_qubits: int = 4) -> Circuit:
    circuit = Circuit(num_qubits)
    circuit.append_mc_gate(MCGate(3, StandardGate.X), [0, 1, 2, 3])
    return circuit


def test_resource_module_registration_and_public_exports():
    assert resource.ResourceManager is ResourceManager
    assert "cqlib._native.compile.resource" in sys.modules
    assert ResourceManager.__module__ == "cqlib.compile.resource"
    assert ResourceError.__module__ == "cqlib.compile.resource"
    assert issubclass(ResourceError, CqlibError)
    assert issubclass(ResourceUnavailableError, ResourceError)
    assert set(resource.__all__) == {
        "AncillaRequirement",
        "ResourcePolicy",
        "ResourceLimits",
        "ResourceRequest",
        "ResourcePlan",
        "ResourceLease",
        "ResourceManager",
        "ResourceError",
        "ResourceUnavailableError",
    }


def test_resource_value_objects_are_readonly_and_copyable():
    clean = AncillaRequirement.clean_zero()
    assert clean == copy.copy(clean) == copy.deepcopy(clean)
    assert str(clean) == "clean-zero"
    assert clean != AncillaRequirement.dirty()

    policy = ResourcePolicy(
        max_pre_layout_clean_ancillas=2,
        allow_dirty_borrowing=True,
    )
    limits = ResourceLimits(max_total_qubits=7)
    request = ResourceRequest(clean, 2, excluded=[3, 1, 3])

    assert copy.copy(policy) == policy
    assert copy.deepcopy(limits) == limits
    assert policy.max_pre_layout_clean_ancillas == 2
    assert policy.allow_dirty_borrowing
    assert "allow_dirty_borrowing=True" in repr(policy)
    assert limits.max_total_qubits == 7
    assert request.requirement == clean
    assert request.count == 2
    assert request.excluded == [Qubit(1), Qubit(3)]

    with pytest.raises(AttributeError):
        policy.max_pre_layout_clean_ancillas = 3
    with pytest.raises(AttributeError):
        request.count = 1
    with pytest.raises(TypeError):
        ResourcePlan()
    with pytest.raises(TypeError):
        ResourceLease()
    with pytest.raises(OverflowError):
        ResourcePolicy(max_pre_layout_clean_ancillas=-1)


def test_clean_preview_commit_release_and_reuse_lifecycle():
    circuit = Circuit(2)
    manager = ResourceManager.from_circuit(
        circuit,
        policy=ResourcePolicy(max_pre_layout_clean_ancillas=1),
    )
    request = ResourceRequest(AncillaRequirement.clean_zero(), 1)

    first = manager.preview(request)
    stale = manager.preview(request)
    assert first.qubits == [Qubit(2)]
    assert first.num_new_qubits == 1
    assert circuit.num_qubits == 2

    lease = manager.commit(circuit, first)
    assert circuit.num_qubits == 3
    assert lease.qubits == [Qubit(2)]
    with pytest.raises(ResourceError, match="stale resource plan"):
        manager.commit(circuit, stale)
    with pytest.raises(ResourceError, match="active lease"):
        manager.verify_idle(circuit)

    manager.release(lease)
    manager.verify_idle(circuit)
    reused = manager.preview(request)
    assert reused.qubits == [Qubit(2)]
    assert reused.num_new_qubits == 0
    with pytest.raises(ResourceError, match="unknown or released"):
        manager.release(lease)


def test_unavailable_foreign_and_inconsistent_states_are_distinct():
    circuit = Circuit(2)
    manager = ResourceManager.from_circuit(circuit)
    clean_request = ResourceRequest(AncillaRequirement.clean_zero(), 1)

    with pytest.raises(ResourceUnavailableError, match="insufficient clean-zero"):
        manager.preview(clean_request)
    with pytest.raises(ResourceError, match="count must be greater than zero"):
        manager.preview(ResourceRequest(AncillaRequirement.dirty(), 0))
    with pytest.raises(ResourceError, match="is not registered"):
        manager.preview(ResourceRequest(AncillaRequirement.dirty(), 1, excluded=[99]))

    borrowing = ResourceManager.from_circuit(
        circuit,
        policy=ResourcePolicy(allow_dirty_borrowing=True),
    )
    plan = borrowing.preview(ResourceRequest(AncillaRequirement.dirty(), 1))
    with pytest.raises(ResourceError, match="another resource manager"):
        manager.commit(circuit, plan)

    changed_circuit = Circuit(3)
    with pytest.raises(ResourceError, match="inconsistent"):
        manager.verify_consistency(changed_circuit)


def test_dirty_borrowing_respects_exclusions_and_post_layout_is_one_way():
    circuit = Circuit(3)
    manager = ResourceManager.from_circuit(
        circuit,
        policy=ResourcePolicy(allow_dirty_borrowing=True),
    )
    request = ResourceRequest(
        AncillaRequirement.dirty(),
        1,
        excluded=[0, 1],
    )

    assert manager.preview(request).qubits == [Qubit(2)]
    manager.enter_post_layout(circuit)
    with pytest.raises(ResourceError, match="requires phase pre-layout"):
        manager.enter_post_layout(circuit)

    clean_manager = ResourceManager.from_circuit(
        circuit,
        policy=ResourcePolicy(max_pre_layout_clean_ancillas=1),
    )
    clean_manager.enter_post_layout(circuit)
    with pytest.raises(ResourceUnavailableError, match="insufficient clean-zero"):
        clean_manager.preview(ResourceRequest(AncillaRequirement.clean_zero(), 1))


def test_resource_limits_reject_initial_and_planned_capacity_overflow():
    with pytest.raises(ResourceUnavailableError, match="capacity exceeded"):
        ResourceManager.from_circuit(
            Circuit(2),
            limits=ResourceLimits(max_total_qubits=1),
        )

    circuit = Circuit(2)
    manager = ResourceManager.from_circuit(
        circuit,
        policy=ResourcePolicy(max_pre_layout_clean_ancillas=1),
        limits=ResourceLimits(max_total_qubits=2),
    )
    with pytest.raises(ResourceUnavailableError, match="capacity exceeded"):
        manager.preview(ResourceRequest(AncillaRequirement.clean_zero(), 1))


def test_compile_resource_policy_controls_clean_and_dirty_ancillas():
    source = mcx_circuit()
    default_result = compile(source)
    clean_result = compile(
        source,
        resource_policy=ResourcePolicy(max_pre_layout_clean_ancillas=2),
    )

    assert source.num_qubits == 4
    assert default_result.circuit.num_qubits == 4
    assert clean_result.circuit.num_qubits == 5
    assert any(
        Qubit(4) in operation.qubits for operation in clean_result.circuit.operations
    )

    dirty_source = mcx_circuit(5)
    dirty_result = compile(
        dirty_source,
        resource_policy=ResourcePolicy(allow_dirty_borrowing=True),
    )
    assert dirty_result.circuit.num_qubits == 5
    assert any(
        Qubit(4) in operation.qubits for operation in dirty_result.circuit.operations
    )
