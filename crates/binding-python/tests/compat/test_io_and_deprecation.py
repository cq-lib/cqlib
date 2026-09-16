# Copyright China Telecom Quantum Group 2026
# SPDX-License-Identifier: Apache-2.0

import inspect
from pathlib import Path
import subprocess
import sys
import warnings

import numpy as np
import pytest

from cqlib import Circuit, CqlibDeprecationWarning, Parameter, compile_circuit
from cqlib.ir import qasm2, qcis
from cqlib.visualization import draw_text

pytestmark = pytest.mark.filterwarnings("ignore::cqlib.CqlibDeprecationWarning")


def test_old_workflow_exports_and_runs_in_native_compiler(tmp_path):
    from cqlib.circuits import Circuit as OldCircuit, gates
    from cqlib.utils import qasm2 as old_qasm2

    theta = Parameter("theta")
    circuit = OldCircuit(2, parameters=[theta])
    circuit.append(gates.RX(theta), [0])
    circuit.append(gates.CNOT(), [0, 1])
    bound = circuit.assign_parameters([0.3])
    compiled = compile_circuit(bound).circuit
    assert type(compiled) is Circuit
    np.testing.assert_allclose(compiled.to_matrix(), bound.to_matrix(), atol=1e-10)
    assert bound.draw() == draw_text(bound)
    assert bound.draw(category="text", show_params=False) == draw_text(
        bound, show_params=False
    )
    assert bound.qcis == bound.as_str() == qcis.dumps(bound)
    np.testing.assert_allclose(
        Circuit.load(bound.qcis).to_matrix(), bound.to_matrix(), atol=1e-10
    )
    assert bound.to_qasm2() == qasm2.dumps(bound)
    for name in ["load", "loads", "dump", "dumps"]:
        assert getattr(old_qasm2, name) is getattr(qasm2, name)
    text = old_qasm2.dumps(bound)
    np.testing.assert_allclose(
        old_qasm2.loads(text).to_matrix(), bound.to_matrix(), atol=1e-10
    )
    path = str(tmp_path / "circuit.qasm")
    old_qasm2.dump(bound, path)
    np.testing.assert_allclose(
        old_qasm2.load(path).to_matrix(), bound.to_matrix(), atol=1e-10
    )
    bound.measure_all()
    assert bound.qcis == qcis.dumps(bound)
    assert bound.to_qasm2() == qasm2.dumps(bound)


def test_unsupported_old_export_modes_raise_without_changing_circuit():
    circuit = Circuit(1)
    circuit.h(0)
    before = circuit.copy()
    with pytest.raises(TypeError, match="text"):
        circuit.draw(category="mpl")
    with pytest.raises(TypeError):
        circuit.as_str(qcis_compliant=True)
    assert circuit == before


def test_rust_forwarder_warning_points_to_user_call_once():
    circuit = Circuit(1)
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always", CqlibDeprecationWarning)
        line = inspect.currentframe().f_lineno + 1
        circuit.sd(0)
    assert len(caught) == 1
    assert caught[0].category is CqlibDeprecationWarning
    assert Path(caught[0].filename) == Path(__file__)
    assert caught[0].lineno == line
    assert "sdg" in str(caught[0].message)
    assert "1.4" in str(caught[0].message)


def test_python_factory_warning_points_to_user_and_does_not_deprecate_native_type():
    from cqlib.circuits import gates
    from cqlib import StandardGate

    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always", CqlibDeprecationWarning)
        line = inspect.currentframe().f_lineno + 1
        gates.RX(0.1)
    assert len(caught) == 1
    assert Path(caught[0].filename) == Path(__file__)
    assert caught[0].lineno == line
    with warnings.catch_warnings():
        warnings.simplefilter("error", CqlibDeprecationWarning)
        StandardGate.RX(0.1)
        Circuit(1).rx(0, 0.1)


@pytest.mark.parametrize(
    "module,replacement",
    [
        ("cqlib.circuits.circuit", "cqlib.circuit"),
        ("cqlib.exceptions", "cqlib.circuit"),
        ("cqlib.utils.qasm2", "cqlib.ir.qasm2"),
    ],
)
def test_legacy_import_warnings_are_filterable(module, replacement):
    script = f"""
import importlib, warnings
from cqlib import CqlibDeprecationWarning
with warnings.catch_warnings(record=True) as caught:
    warnings.simplefilter('always', CqlibDeprecationWarning)
    importlib.import_module({module!r})
assert len(caught) == 1, caught
assert {replacement!r} in str(caught[0].message)
assert caught[0].filename == '<string>', caught[0].filename
"""
    subprocess.run(
        [sys.executable, "-c", script], check=True, capture_output=True, text=True
    )
