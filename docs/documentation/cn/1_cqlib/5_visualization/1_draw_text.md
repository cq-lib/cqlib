# 用文本图调试线路

文本图适合在终端、日志、Markdown 和单元测试中快速检查线路。

---

## 任务：检查一个 Bell 态线路

先构造一条带测量的 Bell 态线路：

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

输出结果：

```text

 Q0: ───H──■──M─
           │
 Q1: ──────X──M─

```

阅读文本图时，重点看三件事：

- `H` 是否作用在第 `0` 个量子比特；
- `CX` 的控制位是否在 `0`，目标位是否在 `1`；
- 测量是否在纠缠操作之后。

调试算法线路时，可以在每增加一层结构后打印一次文本图，避免在线路全部生成后才排查结构问题。

文本图更适合快速调试。它的优势是轻量、可复制、方便放进 issue 和测试失败日志；当需要在报告或演示材料中展示完整结构时，可以同时生成 PNG 图。

---

## 检查比特显示顺序

某些论文、后端或前端界面会把高位比特画在上方。`reverse_bits=True` 只改变显示顺序，不改变线路语义。

```python
print(draw_text(circuit))
print(draw_text(circuit, reverse_bits=True))
```

反转显示顺序后的输出如下：

```text

 Q1: ──────X──M─
           │
 Q0: ───H──■──M─

```


---

## 调试参数化线路

参数化线路容易因为符号名、绑定顺序或表达式过长而变得难读。文本图可以先用于确认线路拓扑，再决定是否显示参数。

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

显示参数时，文本图保留符号表达式：

```text

 Q0: ───RY(theta)──RZ(phi)──■──────────────────
                            │
 Q1: ───────────────────────X──RY(phi + theta)─

```

隐藏参数后，图只保留门和线路拓扑：

```text

 Q0: ───RY──RZ──■─────
                │
 Q1: ───────────X──RY─

```

这一步常用于变分线路调试：先确认 entangler 连接关系，再单独检查参数表或优化器传入的参数向量。不要用隐藏参数后的图说明具体角度取值。

---

## 处理长线路

线路较深时，默认文本图可能超过终端宽度。可以用 `line_width` 控制折行。

```python
layer = Circuit(2)
for _ in range(8):
    layer.h(0)
    layer.cx(0, 1)
    layer.rz(1, 0.2)

print(draw_text(layer, line_width=80))
```

输出会在超过指定宽度后折行：

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

## 检查复合门内部结构

当线路里包含由 `to_gate()` 封装的复合门时，默认图会保留模块边界。调试内部细节时，可以展开显示。

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

默认显示保留两个 `Bell` 模块：

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

展开复合门后，可以看到模块内部的 `H` 和 `CX`：

```text

 Q0: ───H──■─
           │
 Q1: ──────X─

 Q2: ───H──■─
           │
 Q3: ──────X─

```


---

## 文本图的使用方式

- 在 issue、日志和调试输出中使用文本图；
- 对 2 到 4 比特的小线路，文本图通常已经足够清晰；
- 检查结构时可以隐藏参数，检查参数绑定时再显示参数；
- 对复杂线路保留文本图快照，配合数值测试验证行为。
- 如果图中涉及后端、论文或其他框架的比特顺序约定，需要确认 `reverse_bits` 只改变显示顺序，不改变线路执行语义。

---

## 下一步

- [生成 PNG 线路图](2_draw_figure.md)：把已经确认结构的小线路保存成更适合文档和报告的图片。
- [Notebook 与文档集成](3_notebook_and_docs.md)：把生成图片的代码和 Markdown 引用放到同一套实验记录中。
- [复杂线路的可视化策略](4_visualization_practices.md)：在线路变深或模块变多时，用分段、展开和对比图定位结构问题。
