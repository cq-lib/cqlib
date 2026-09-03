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

"""Public compiler knowledge API; detailed contracts live in the topic stubs."""

from .library import RuleId as RuleId
from .library import RuleKind as RuleKind
from .library import RuleLibrary as RuleLibrary
from .library import RuleMetadata as RuleMetadata
from .library import dump as dump
from .library import dumps as dumps
from .library import load as load
from .library import loads as loads
from .matcher import MatchBindings as MatchBindings
from .matcher import conditions_hold as conditions_hold
from .matcher import instantiate_target as instantiate_target
from .matcher import match_rule_item as match_rule_item
from .matcher import rule_matches_operations as rule_matches_operations
from .rule import Condition as Condition
from .rule import Rule as Rule
from .rule import RuleItem as RuleItem
from .rule import VerifyResult as VerifyResult

__all__ = [
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
]
