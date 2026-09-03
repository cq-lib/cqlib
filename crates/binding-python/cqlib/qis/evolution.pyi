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

class TrotterMode:
    """Trotter-Suzuki decomposition modes for Hamiltonian time evolution.

    These modes determine how the time evolution operator :math:`U(t) = e^{-iHt}` is
    approximated as a product of Pauli rotations.

    Variants:
        - ``FirstOrder``: First-order Lie-Trotter decomposition. Error scales as :math:`O(t^2/n)`.
        - ``SecondOrder``: Second-order Strang splitting (symmetric). Error scales as :math:`O(t^3/n^2)`.
        - ``Randomized``: Randomized first-order Trotter with specified random seed.

    Examples:
        >>> from cqlib.qis import TrotterMode
        >>> mode1 = TrotterMode.first_order()
        >>> mode2 = TrotterMode.second_order()
        >>> mode3 = TrotterMode.randomized(42)  # with seed 42
    """

    @staticmethod
    def first_order() -> "TrotterMode":
        """Returns the first-order Trotter mode.

        First-order Lie-Trotter decomposition:
        :math:`U(t) \\approx [\\prod_k e^{-i c_k t/n \\cdot P_k}]^n`

        Error scales as :math:`O(t^2/n)`.

        Returns:
            TrotterMode: A new TrotterMode instance for first-order evolution.
        """
        ...

    @staticmethod
    def second_order() -> "TrotterMode":
        """Returns the second-order Trotter mode.

        Second-order Strang splitting (symmetric decomposition):
        :math:`U(t) \\approx [e^{-i c_1 t/2n \\cdot P_1} ... e^{-i c_m t/2n \\cdot P_m} \\cdot e^{-i c_m t/2n \\cdot P_m} ... e^{-i c_1 t/2n \\cdot P_1}]^n`

        Error scales as :math:`O(t^3/n^2)`.

        Returns:
            TrotterMode: A new TrotterMode instance for second-order evolution.
        """
        ...

    @staticmethod
    def randomized(seed: int) -> "TrotterMode":
        """Returns a randomized first-order Trotter mode with the given seed.

        In each Trotter step, the order of Pauli terms is randomly shuffled.
        This can help reduce systematic errors and improve convergence.

        Args:
            seed (int): The random seed for reproducibility.

        Returns:
            TrotterMode: A new TrotterMode instance for randomized evolution.
        """
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...
