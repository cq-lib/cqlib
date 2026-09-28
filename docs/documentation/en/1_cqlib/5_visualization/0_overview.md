# Visualization

The visualization capabilities of Cqlib are used to inspect three kinds of object during quantum program development: circuit structure, measurement results and quantum states. This chapter starts from practical usage scenarios and shows how to generate figures, save results and check quantum semantics from the figures.

Before starting, complete the [Cqlib Installation and Environment Setup](../../0_get_started/1_installation.md) and confirm that Cqlib imports correctly.

```python
import cqlib

print(cqlib.__file__)
```

If multiple Cqlib checkouts exist on the machine, confirm first that the one printed here is the implementation to be used. The visualization examples rely on the Python bindings of `cqlib.visualization` and on local figure rendering capability; they do not connect to a cloud platform or submit tasks to real quantum hardware.

---

## Starting from the Bell state

The Bell state is the most suitable example for getting started with visualization: the circuit is short, yet it contains superposition, entanglement, measurement and statistical results at the same time.

```python
from cqlib import Circuit
from cqlib.visualization import draw_text, draw_figure

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)
circuit.measure(0)
circuit.measure(1)

print(draw_text(circuit))
draw_figure(circuit, output_path="assets/bell.png")
```

The text diagram output is as follows:

```text

 Q0: ───H──■──M─
           │
 Q1: ──────X──M─

```

The generated PNG circuit diagram is as follows:

![Bell state circuit](assets/bell.png)

This diagram checks whether the circuit semantics are correct:

- The `H` gate acts on `Q0` first, creating superposition;
- `CX` takes `Q0` as the control and `Q1` as the target, creating entanglement;
- Both measurements come after the entangling operation and do not destroy the state early.

When the gate order, control qubits and target qubits need to be confirmed quickly, the text diagram is usually fastest. When a stable display is needed in Gitee, Markdown, reports or presentation material, PNG is more compatible; when a scalable vector graphic is needed, `output_path` can be changed to `.svg`.

---

## Learning path in this chapter

The reading order is as follows:

1. [Debugging circuits with text diagrams](1_draw_text.md): inspect gate order, bit order, parameters and composite gates in the terminal.
2. [Generating PNG circuit diagrams](2_draw_figure.md): generate figure files for Notebooks, documentation sites and reports.
3. [Notebook and documentation integration](3_notebook_and_docs.md): save and reference visualization results in Notebooks and Markdown.
4. [Visualization strategies for complex circuits](4_visualization_practices.md): handle parameterized circuits, before-and-after mapping comparison and display of large circuits.
5. [Control flow and special circuit structures](5_control_flow_and_special.md): read dynamic control flow, non-unitary instructions and custom gate figures.
6. [Visualizing execution results](6_result_visualization.md): inspect sampling results with bar charts and probability distributions.
7. [Visualizing quantum states](7_state_visualization.md): understand states with Bloch, state city and Pauli vector figures.

Reading in this order is recommended. The first five sections address whether a circuit is constructed as expected; the last two address how to interpret objects after execution or simulation.

---

## When to draw figures

In quantum program development, visualization is usually not the last step but a means of inspection after each key transform. The following points are suitable places to draw figures:

- After writing a multi-qubit gate by hand, check the control qubits and target qubits;
- After constructing a parameterized ansatz, check whether each layer repeats as expected;
- After wrapping a sub-circuit as a `CircuitGate`, check the module boundary;
- After decomposing composite gates, check the underlying gate sequence;
- After adding dynamic control flow, check the branch, loop and control-transfer markers;
- After compilation or mapping, check SWAP insertion and two-qubit gate positions;
- After sampling or simulation, check whether the result distribution matches expectations.

Visualization cannot replace matrix verification, probability verification or unit tests, but it exposes problems such as bit order, measurement position, missing parameters and excessive circuit depth very quickly.

---

## How to choose a diagram

| Development task | Recommended diagram | Points to inspect |
|---|---|---|
| Quickly check circuit structure | Text circuit diagram | Gate order, control qubits, target qubits, measurement position |
| Write a Notebook or report | PNG circuit diagram | Clarity of structure, module boundaries, bit display order |
| Read dynamic control flow or special instructions | PNG circuit diagram | Branches, loops, `barrier`, `reset`, `delay`, custom gate labels |
| Inspect sampling results | Histogram / distribution | Dominant peaks, low-probability noise terms, shot count and normalized probabilities |
| Understand single-qubit states | Bloch diagram | Direction and length of the Bloch vector |
| Understand multi-qubit states or density matrices | State city / Pauli vector | Coherence terms, Pauli expectation values and the global structure of entangled states |

---

## Next steps

- [Debugging circuits with text diagrams](1_draw_text.md): check gate order, control qubits, target qubits and measurement positions quickly in the terminal.
- [Generating PNG circuit diagrams](2_draw_figure.md): save circuits that need to go into Notebooks, Markdown or reports as image files.
- [Visualizing execution results](6_result_visualization.md): after a circuit runs or is sampled, inspect the dominant peaks, noise terms and bitstring order with result figures.
