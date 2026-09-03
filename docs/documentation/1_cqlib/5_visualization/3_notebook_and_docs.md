# Notebook 与文档集成

本节展示如何在 Notebook 中显示 Cqlib 生成的 PNG，并把同一张图保存到 Markdown 可引用的资源目录。

---

## 任务：在 Notebook 中展示并保存同一张图

```python
from pathlib import Path

from cqlib import Circuit
from cqlib.visualization import draw_figure

assets = Path("assets")
assets.mkdir(exist_ok=True)

circuit = Circuit(2)
circuit.h(0)
circuit.cx(0, 1)
circuit.measure(0)
circuit.measure(1)

draw_figure(circuit, output_path=str(assets / "bell.png"))
```

在 Notebook 中，上面最后一行会内联显示图形，同时把 PNG 保存到 `assets/bell.png`。这种方式适合实验记录、报告和论文补充材料。

生成结果如下：

![Bell state circuit](assets/bell.png)

---

## 在 Markdown 中引用

保存 PNG 后，在 Markdown 中直接引用：

```markdown
![Bell state circuit](assets/bell.png)
```

引用图片后，可以在图下方记录这张线路图的检查点：

```markdown
图 1 展示了 Bell 态制备线路：先对 q0 施加 H 门，再以 q0 为控制位、q1 为目标位施加 CX 门。
```

量子线路图通常需要明确控制位、目标位、测量位置和比特顺序。否则图本身只能说明线路形状，不能说明检查结论。

---

## 组织文档资产目录

为每组实验或每篇报告保留独立的资源目录，可以避免不同图片互相覆盖：

```text
docs/
  visualization/
    tutorial.md
    assets/
      bell.png
      ansatz_structure.png
      mapped_before.png
      mapped_after.png
```

文件名应描述任务，而不是使用 `figure1.png`、`test.png` 这类临时名称。

---

## 自动重生成图片

当一组实验包含多张图时，可以把生成逻辑集中放在同一个代码块或脚本里。

```python
from pathlib import Path

from cqlib import Circuit
from cqlib.visualization import draw_figure

assets = Path("assets")
assets.mkdir(exist_ok=True)

bell = Circuit(2)
bell.h(0)
bell.cx(0, 1)

draw_figure(bell, output_path=str(assets / "bell.png"))
draw_figure(bell, reverse_bits=True, output_path=str(assets / "bell_reverse_bits.png"))
```

生成的两张图可以在 Markdown 中分别引用：

![Bell state circuit](assets/bell.png)

![Bell state circuit with reversed bits](assets/bell_reverse_bits.png)

当线路构造代码变更时，重新运行这一段即可同步更新插图。如果生成的图不符合预期，优先检查线路构造和可视化参数，而不是手工修改图片内容。

---

## 保存可视化结果

- 小线路可以同时保留文本图和 PNG；
- 大线路优先保存关键阶段，不必保存每一个中间线路；
- 保存 PNG 原图，避免只保留截图；
- 文件名使用具体任务名，例如 `bell_reverse_bits.png`、`mapped_after.png`；
- 图中涉及比特顺序、测量顺序或后端映射时，需要在实验记录中写清楚对应约定。

---

## 常见问题

Notebook 中没有自动显示图形时，可以显式使用 IPython 显示返回的 SVG 字符串：

```python
from IPython.display import SVG, display
from cqlib.visualization import draw_figure

display(SVG(draw_figure(circuit)))
```

脚本只保存文件而不显示图时，需要检查 `output_path` 是否写到了预期目录，并确认生成文件可以被本地图片查看器打开。

---

## 更新图片后的检查

修改线路或绘图代码后，可以按下面顺序检查：

1. 重新运行生成 PNG 的代码块或脚本；
2. 确认 Markdown 中引用的文件名没有变化；
3. 打开生成的 PNG，检查控制位、目标位、测量位置和比特顺序是否仍然符合预期；
4. 如果包含采样结果或状态图，确认数据来源是构造结果、本地模拟还是硬件结果；
5. 检查文件名中没有临时名称，代码块中没有绝对路径或本机用户目录。

---

## 下一步

- [生成 PNG 线路图](2_draw_figure.md)：回到绘图参数，调整折叠、参数显示、初态标记和比特显示顺序。
- [复杂线路的可视化策略](4_visualization_practices.md)：为多阶段算法、映射结果和大线路组织一组可维护的图片。
- [可视化执行结果](6_result_visualization.md)：把采样结果图纳入同一套 Notebook 和 Markdown 资产目录。
