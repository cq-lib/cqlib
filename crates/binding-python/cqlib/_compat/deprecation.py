# Copyright China Telecom Quantum Group 2026
# SPDX-License-Identifier: Apache-2.0

"""Deprecation notices shared by Python adapters and native entry points."""

import sys
import warnings


class CqlibDeprecationWarning(DeprecationWarning):
    """A supported legacy cqlib spelling should be migrated to its replacement."""


def warn(old: str, replacement: str) -> None:
    """Report the caller, including when invoked through a PyO3 adapter."""
    frame = sys._getframe()
    while frame is not None:
        module = frame.f_globals.get("__name__", "")
        if not (
            module in ("cqlib", "importlib")
            or module.startswith(("cqlib.", "importlib.", "_frozen_importlib"))
        ):
            break
        frame = frame.f_back
    message = f"{old} is deprecated since cqlib 1.4; use {replacement} instead."
    if frame is None:
        warnings.warn(message, CqlibDeprecationWarning, stacklevel=2)
        return
    # warn() itself skips importlib bootstrap frames. Using the resolved frame
    # explicitly avoids counting those frames twice during legacy imports.
    warnings.warn_explicit(
        message,
        CqlibDeprecationWarning,
        filename=frame.f_code.co_filename,
        lineno=frame.f_lineno,
        module=frame.f_globals.get("__name__", ""),
        registry=frame.f_globals.setdefault("__warningregistry__", {}),
    )
