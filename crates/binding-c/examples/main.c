// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

#include <stdio.h>

#include "cqlib_c.h"

int main(void) {
    CCircuit* circuit = circuit_new(2);
    if (circuit == NULL) {
        return 1;
    }

    CParameter* theta = param_parse("theta");
    CParameter* phi = param_parse("phi");
    if (theta == NULL || phi == NULL) {
        circuit_free(circuit);
        param_free(theta);
        param_free(phi);
        return 1;
    }

    if (circuit_h(circuit, 0) != 0 || circuit_rx_param(circuit, 0, theta) != 0 ||
        circuit_cx(circuit, 0, 1) != 0 || circuit_rz_param(circuit, 1, phi) != 0 ||
        circuit_validate(circuit) != 0) {
        circuit_free(circuit);
        param_free(theta);
        param_free(phi);
        return 1;
    }

    printf("qubits=%zu operations=%zu parameters=%zu\n", circuit_num_qubits(circuit),
           circuit_num_operations(circuit), circuit_num_parameters(circuit));

    CCircuit* assigned = circuit_assign_params(circuit, "theta:0.5,phi:1.25");
    if (assigned == NULL) {
        circuit_free(circuit);
        param_free(theta);
        param_free(phi);
        return 1;
    }

    printf("assigned_parameters=%zu\n", circuit_num_parameters(assigned));

    circuit_free(assigned);
    circuit_free(circuit);
    param_free(theta);
    param_free(phi);
    return 0;
}
