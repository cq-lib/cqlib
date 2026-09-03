# This code is part of Cqlib.
#
# (C) Copyright China Telecom Quantum Group 2026
#
# This code is licensed under the Apache License, Version 2.0. You may
# obtain a copy of this license in the LICENSE.txt file in the root directory
# of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.

"""Ancillary-resource contracts, policies, plans, leases, and manager."""

from cqlib.circuit import CqlibError
from .manager import ResourceManager as ResourceManager
from .model import AncillaRequirement as AncillaRequirement
from .model import ResourceLease as ResourceLease
from .model import ResourcePlan as ResourcePlan
from .model import ResourceRequest as ResourceRequest
from .policy import ResourceLimits as ResourceLimits
from .policy import ResourcePolicy as ResourcePolicy

class ResourceError(CqlibError):
    """Base exception for ancillary-resource management failures."""
    ...

class ResourceUnavailableError(ResourceError):
    """A request cannot be satisfied by the current policy or capacity.

    Compiler planners may catch this exception to reject one algorithm
    candidate and try another. Other ``ResourceError`` instances indicate an
    invalid request or inconsistent planner state and should not be ignored.
    """
    ...

__all__ = [
    "AncillaRequirement",
    "ResourcePolicy",
    "ResourceLimits",
    "ResourceRequest",
    "ResourcePlan",
    "ResourceLease",
    "ResourceManager",
    "ResourceError",
    "ResourceUnavailableError",
]
