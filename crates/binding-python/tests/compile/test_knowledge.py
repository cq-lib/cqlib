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

"""Contract tests for the public compiler knowledge-rule bindings."""

import copy
import sys

import pytest

from cqlib.circuit import Instruction, Parameter, Qubit, StandardGate, ValueOperation
from cqlib.compile import knowledge
from cqlib.compile.knowledge import library as library_module
from cqlib.compile.knowledge import matcher as matcher_module
from cqlib.compile.knowledge import rule as rule_module
from cqlib.compile.knowledge import (
    Condition,
    MatchBindings,
    Rule,
    RuleItem,
    RuleKind,
    RuleLibrary,
    conditions_hold,
    dump,
    dumps,
    instantiate_target,
    load,
    loads,
    match_rule_item,
    rule_matches_operations,
)


def test_module_is_registered_and_public_exports_are_complete():
    assert "cqlib._native.compile.knowledge" in sys.modules
    assert set(knowledge.__all__) == {
        "RuleItem",
        "Condition",
        "Rule",
        "VerifyResult",
        "RuleId",
        "RuleKind",
        "RuleMetadata",
        "RuleLibrary",
        "MatchBindings",
        "loads",
        "load",
        "dumps",
        "dump",
        "match_rule_item",
        "conditions_hold",
        "instantiate_target",
        "rule_matches_operations",
    }
    assert set(rule_module.__all__) == {"RuleItem", "Condition", "Rule", "VerifyResult"}
    assert set(library_module.__all__) == {
        "RuleId",
        "RuleKind",
        "RuleMetadata",
        "RuleLibrary",
        "loads",
        "load",
        "dumps",
        "dump",
    }
    assert set(matcher_module.__all__) == {
        "MatchBindings",
        "match_rule_item",
        "conditions_hold",
        "instantiate_target",
        "rule_matches_operations",
    }
    assert Rule.__module__ == "cqlib.compile.knowledge"


def test_construct_validate_and_verify_rule():
    item = RuleItem.standard(StandardGate.H, [0])
    rule = Rule("cancel_h", [item, copy.copy(item)], [])

    item.validate()
    rule.validate()
    result = rule.verify()

    assert item.qubits == [0]
    assert item.params == []
    assert item.equivalent_to(copy.deepcopy(item))
    assert rule.num_qubits == 1
    assert result.status == "equivalent"
    assert result.passed
    assert result.num_bindings is None


def test_rule_validation_reports_unbound_rewrite_symbol():
    invalid = Rule(
        "invalid",
        [RuleItem.standard(StandardGate.RZ(Parameter("theta")), [0])],
        [RuleItem.standard(StandardGate.RZ(Parameter("phi")), [0])],
    )

    with pytest.raises(ValueError, match="rewrite symbol phi"):
        invalid.validate()


def test_conditions_and_matcher_bind_symbolic_parameters_transactionally():
    theta = Parameter("theta")
    item = RuleItem.standard(StandardGate.RZ(theta), [0])
    bindings = MatchBindings()
    operation = ValueOperation.from_standard_gate(StandardGate.RZ(0.25), [Qubit(4)])

    assert match_rule_item(item, operation, bindings)
    assert bindings.qubit(0) == Qubit(4)
    assert bindings.param("theta") == Parameter(0.25)
    assert conditions_hold([Condition.equal(theta, Parameter(0.25))], bindings)

    before = copy.copy(bindings)
    mismatch = ValueOperation.from_standard_gate(StandardGate.X, [Qubit(4)])
    assert not match_rule_item(item, mismatch, bindings)
    assert bindings == before


def test_complete_match_and_target_instantiation_return_value_operations():
    source = """
    rule merge_rz {
        match {
            RZ(a) 0
            RZ(b) 0
        }
        rewrite {
            RZ(a + b) 0
        }
    }
    """
    rule = loads(source)[0]
    operations = [
        ValueOperation.from_standard_gate(StandardGate.RZ(0.2), [Qubit(2)]),
        ValueOperation.from_standard_gate(StandardGate.RZ(0.3), [Qubit(2)]),
    ]

    bindings = rule_matches_operations(rule, operations)
    assert bindings is not None
    replacements = instantiate_target(rule.target, bindings)

    assert len(replacements) == 1
    assert replacements[0].qubits == [Qubit(2)]
    assert replacements[0].params[0] == pytest.approx(0.5)
    assert rule_matches_operations(rule, operations[:1]) is None


def test_dsl_round_trip_for_string_and_file(tmp_path):
    rules = loads("rule cancel_x { match { X 0, X 0 } rewrite {} }")
    rendered = dumps(rules[0])
    assert loads(rendered)[0].name == "cancel_x"

    path = tmp_path / "rules.rule"
    dump(rules, path)
    loaded = load(path)
    assert [rule.name for rule in loaded] == ["cancel_x"]

    with pytest.raises(ValueError):
        loads("rule broken {")
    with pytest.raises(OSError):
        load(tmp_path / "missing.rule")
    with pytest.raises(OSError):
        RuleLibrary.from_dsl_file(tmp_path / "missing-library.rule", RuleKind.other())


def test_rule_library_queries_and_atomic_duplicate_rejection():
    rule = loads("rule cancel_x { match { X 0, X 0 } rewrite {} }")[0]
    library = RuleLibrary.from_rules([rule], RuleKind.cancel())
    rule_id = library.id_by_name("cancel_x")

    assert rule_id is not None
    assert rule_id.index == 0
    assert library.get(rule_id).name == "cancel_x"
    assert library.metadata(rule_id).cost_delta == -2
    assert library.rules_by_kind(RuleKind.cancel()) == [rule_id]
    assert "cancel_x" in library
    assert len(library) == 1

    x_instruction = Instruction.from_standard_gate(StandardGate.X)
    assert library.candidates_for_first_instruction(x_instruction) == [rule_id]
    assert library.filter_rule_ids_by_instruction_keys([x_instruction], []) == [rule_id]

    with pytest.raises(ValueError, match="duplicate rule name"):
        library.add_rule(rule, RuleKind.cancel())
    assert len(library) == 1


def test_builtin_library_is_available_and_owned():
    first = RuleLibrary.builtin()
    second = copy.deepcopy(first)

    assert len(first) > 0
    assert len(second) == len(first)
    assert first.rules()[0].name == second.rules()[0].name


def test_unsupported_candidate_instruction_raises_value_error():
    library = RuleLibrary()
    with pytest.raises(ValueError, match="unsupported instruction"):
        library.candidates_for_first_instruction(Instruction.delay())
