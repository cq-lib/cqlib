# 可视化

Cqlib 的可视化功能用于在量子程序开发过程中检查三类对象：线路结构、测量结果和量子态。本章从实际使用场景出发，展示如何生成图形、保存结果，并根据图形检查量子语义。

开始前需要完成 [安装与环境配置](../../0_get_started/1_installation.md)，并确认 Cqlib 可以正常导入。

```python
import cqlib

print(cqlib.__file__)
```

如果机器上同时存在多个 Cqlib checkout，请先确认这里打印的是当前要使用的实现。可视化示例依赖 `cqlib.visualization` 的 Python 绑定和本地图形渲染能力，不会连接云平台，也不会提交真实量子硬件任务。

---

## 从 Bell 态开始

Bell 态是最适合入门可视化的例子：线路很短，但同时包含叠加、纠缠、测量和统计结果。

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

文本图输出如下：

```text

 Q0: ───H──■──M─
           │
 Q1: ──────X──M─

```

生成的 PNG 线路图如下：

![Bell state circuit](assets/bell.png)

这张图检查线路语义是否正确：

- `H` 门先作用在 `Q0`，创建叠加；
- `CX` 以 `Q0` 为控制位、`Q1` 为目标位，创建纠缠；
- 两个测量都位于纠缠操作之后，没有提前破坏态。

快速确认门顺序、控制位和目标位时，文本图通常最快。需要在 Gitee、Markdown、报告或演示材料中稳定展示时，PNG 更兼容；需要可缩放矢量图时，也可以把 `output_path` 改成 `.svg`。

---

## 本章学习路线

阅读顺序如下：

1. [用文本图调试线路](1_draw_text.md)：在终端中检查门顺序、比特顺序、参数和复合门。
2. [生成 PNG 线路图](2_draw_figure.md)：为 Notebook、文档站和报告生成图形文件。
3. [Notebook 与文档集成](3_notebook_and_docs.md)：在 Notebook 和 Markdown 中保存、引用可视化结果。
4. [复杂线路的可视化策略](4_visualization_practices.md)：处理参数化线路、映射前后对比和大线路展示。
5. [控制流与特殊线路结构](5_control_flow_and_special.md)：阅读动态控制流、非幺正指令和自定义门图形。
6. [可视化执行结果](6_result_visualization.md)：用柱状图和概率分布查看采样结果。
7. [可视化量子态](7_state_visualization.md)：用 Bloch、state city 和 Pauli vector 理解状态。

建议按这个顺序阅读。前五节解决“线路是否按预期构造”，后两节解决“运行或模拟后的对象如何解释”。

---

## 什么时候应该画图

在量子程序开发中，可视化通常不是最后一步，而是每个关键变换后的检查手段。以下位置适合主动画图：

- 手动写完多比特门后，检查控制位和目标位；
- 构造参数化 ansatz 后，检查每一层是否按预期重复；
- 将子线路封装为 `CircuitGate` 后，检查模块边界；
- 展开复合门后，检查底层门序列；
- 加入动态控制流后，检查分支、循环和控制转移标记；
- 编译或映射后，检查 SWAP 插入和双比特门位置；
- 采样或模拟后，检查结果分布是否符合预期。

可视化不能替代矩阵验证、概率验证或单元测试，但它能很快暴露比特顺序、测量位置、参数遗漏和线路过深等问题。

---

## 如何选择图形

| 开发任务 | 推荐图形 | 观察重点 |
|---|---|---|
| 快速检查线路结构 | 文本线路图 | 门顺序、控制位、目标位、测量位置 |
| 写 Notebook 或报告 | PNG 线路图 | 结构清晰度、模块边界、比特显示顺序 |
| 阅读动态控制流或特殊指令 | PNG 线路图 | 分支、循环、`barrier`、`reset`、`delay`、自定义门标签 |
| 查看采样结果 | Histogram / distribution | 主峰、低概率噪声项、shot 数和归一化概率 |
| 理解单比特态 | Bloch 图 | Bloch 向量方向和长度 |
| 理解多比特或密度矩阵 | State city / Pauli vector | 相干项、Pauli 期望值和纠缠态的全局结构 |

---

## 下一步

- [用文本图调试线路](1_draw_text.md)：先在终端中快速检查门顺序、控制位、目标位和测量位置。
- [生成 PNG 线路图](2_draw_figure.md)：把需要写进 Notebook、Markdown 或报告的线路保存成图片。
- [可视化执行结果](6_result_visualization.md)：在线路运行或采样后，用结果图检查主峰、噪声项和 bitstring 顺序。
