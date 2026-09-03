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

"""Noise models for device-aware compilation."""

from .._native import device as _device_module

SingleQubitNoise = _device_module.SingleQubitNoise
TwoQubitNoise = _device_module.TwoQubitNoise
ReadoutError = _device_module.ReadoutError
OperationKey = _device_module.OperationKey
NoiseModel = _device_module.NoiseModel

__all__ = [
    "SingleQubitNoise",
    "TwoQubitNoise",
    "ReadoutError",
    "OperationKey",
    "NoiseModel",
]
