# Debugging circuits with text diagrams

Text diagrams are suitable for quickly inspecting circuits in terminals, logs, Markdown and unit tests.

---

## Task: inspect a Bell-state circuit

First construct a Bell-state circuit with measurements:

```python
from cqlib import Circuit
from cqlib.visualization import draw_text

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)
circuit.measure(0)
circuit.measure(1)

print(draw_text(circuit))
```

Output:

```text

 Q0: ───H──■──M─
           │
 Q1: ──────X──M─

```

Three points matter when reading a text diagram:

- Whether `H` acts on qubit `0`;
- Whether the control of `CX` is on `0` and the target is on `1`;
- Whether the measurements come after the entangling operation.

When debugging algorithm circuits, a text diagram can be printed after each layer of structure is added, avoiding structural troubleshooting only after the whole circuit has been generated.

Text diagrams are better suited to quick debugging. Their advantages are being lightweight and copyable, and being easy to put into issues and test failure logs; when the full structure must be shown in a report or presentation material, a PNG diagram can be generated at the same time.

---

## Checking the bit display order

Some papers, backends or frontend interfaces draw high-order bits at the top. `reverse_bits=True` only changes the display order, not the circuit semantics.

```python
print(draw_text(circuit))
print(draw_text(circuit, reverse_bits=True))
```

The output after reversing the display order is as follows:

```text

 Q1: ──────X──M─
           │
 Q0: ───H──■──M─

```


---

## Debugging parameterized circuits

Parameterized circuits easily become hard to read because of symbolic names, binding order or overly long expressions. A text diagram can be used to confirm the circuit topology first, and a decision can then be made on whether to display parameters.

```python
from cqlib import Circuit, Parameter
from cqlib.visualization import draw_text

theta = Parameter("theta")
phi = Parameter("phi")

ansatz = Circuit(2)
ansatz.ry(0, theta)
ansatz.rz(0, phi)
ansatz.cx(0, 1)
ansatz.ry(1, theta + phi)

print(draw_text(ansatz))
print(draw_text(ansatz, show_params=False))
```

When parameters are displayed, the text diagram keeps the symbolic expressions:

```text

 Q0: ───RY(theta)──RZ(phi)──■──────────────────
                            │
 Q1: ───────────────────────X──RY(phi + theta)─

```

With parameters hidden, the diagram keeps only the gates and the circuit topology:

```text

 Q0: ───RY──RZ──■─────
                │
 Q1: ───────────X──RY─

```

This step is commonly used when debugging variational circuits: confirm the entangler connectivity first, then check the parameter table or the parameter vector passed by the optimizer separately. A diagram with hidden parameters must not be used to state specific angle values.

---

## Handling long circuits

When a circuit is deep, the default text diagram may exceed the terminal width. `line_width` controls folding.

```python
layer = Circuit(2)
for _ in range(8):
    layer.h(0)
    layer.cx(0, 1)
    layer.rz(1, 0.2)

print(draw_text(layer, line_width=80))
```

The output folds after exceeding the specified width:

```text
                                                                                     »
 Q0: ───H──■─────H─────■─────H─────■─────H─────■─────H─────■─────H─────■─────H─────■─»
           │           │           │           │           │           │           │ »
 Q1: ──────X──RZ(0.2)──X──RZ(0.2)──X──RZ(0.2)──X──RZ(0.2)──X──RZ(0.2)──X──RZ(0.2)──X─»
                                                                                     »

«
« Q0: ──────H─────■──────────
«                 │
« Q1: ───RZ(0.2)──X──RZ(0.2)─
«
```



---

## Inspecting the internal structure of composite gates

When a circuit contains composite gates wrapped by `to_gate()`, the default diagram keeps the module boundaries. To debug internal details, the display can be expanded.

```python
from cqlib import Circuit
from cqlib.visualization import draw_text

block = Circuit(2)
block.h(0)
block.cx(0, 1)
bell_gate = block.to_gate("Bell")

main = Circuit(4)
main.append_circuit_gate(bell_gate, [0, 1])
main.append_circuit_gate(bell_gate, [2, 3])

print(draw_text(main))
print(draw_text(main, decompose_circuit_gates=True))
```

The default display keeps the two `Bell` modules:

```text
        ┌──────┐
 Q0: ───│      │─
        │ Bell │
 Q1: ───│      │─
        └──────┘
 Q2: ───│      │─
        │ Bell │
 Q3: ───│      │─
        └──────┘
```

After decomposing the composite gates, the `H` and `CX` inside the modules become visible:

```text

 Q0: ───H──■─
           │
 Q1: ──────X─

 Q2: ───H──■─
           │
 Q3: ──────X─

```


---

## How to use text diagrams

- Use text diagrams in issues, logs and debug output;
- For small circuits of 2 to 4 qubits, a text diagram is usually clear enough;
- Parameters can be hidden when inspecting structure and shown again when checking parameter binding;
- Keep text diagram snapshots for complex circuits and verify behavior together with numerical tests.
- If a diagram involves the bit-order conventions of a backend, a paper or other frameworks, confirm that `reverse_bits` only changes the display order and not the circuit execution semantics.

---

## Next steps

- [Generating PNG circuit diagrams](2_draw_figure.md): save small circuits whose structure has been confirmed as images better suited to documentation and reports.
- [Notebook and documentation integration](3_notebook_and_docs.md): keep the code that generates figures and the Markdown references in the same set of experiment records.
- [Visualization strategies for complex circuits](4_visualization_practices.md): use segmentation, decomposition and comparison figures to locate structural problems as circuits get deeper or modules multiply.
