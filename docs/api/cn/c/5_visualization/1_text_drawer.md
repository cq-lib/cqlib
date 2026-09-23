# TextDrawer

将线路渲染为 UTF-8 文本图（box-drawing 字符），适合日志输出与终端查看。

---

## 绘制选项 TextDrawerOptionsC

```c
typedef struct TextDrawerOptionsC {
  uint8_t show_params;              // 是否在门标签后附加参数文本
  uint8_t decompose_circuit_gates;  // 绘制前是否分解复合门
  intptr_t line_width;              // 折行宽度；<=0 不折行
  uint8_t initial_state;            // 比特线首是否显示 |0>
  uint8_t reverse_bits;             // 是否反转比特显示顺序
} TextDrawerOptionsC;
```

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `show_params` | `0/1` | 参数化门是否显示参数值（如 `RZ(0.5)`）。 |
| `decompose_circuit_gates` | `0/1` | 绘制前是否分解复合门；线路来自 IR 解析或编译结果且含复合门时使用。 |
| `line_width` | 整数 | 单段折行的最大宽度；`<= 0` 关闭折行。 |
| `initial_state` | `0/1` | 是否在每条比特线首显示 `\|0⟩`。 |
| `reverse_bits` | `0/1` | 是否反转比特显示顺序（qubit 0 显示在最下方）。 |

选项结构体按指针传入；`circuit_to_text` 的 `options` 传 **NULL** 时整体使用库默认值。零初始化（`= {0}`）表示各开关关闭、不折行。

---

## 函数

### circuit_to_text(circuit, options)

参数：

- `circuit` (`const CCircuit *`)：线路，不被修改。
- `options` (`const TextDrawerOptionsC *`)：绘制选项，NULL 使用默认值。

返回：

- `char *`：UTF-8 文本图，需 `cqlib_string_free` 释放；失败（NULL 线路等）返回 NULL。

---

## 示例

### 默认选项

```c
CCircuit *qc = circuit_new(2);
circuit_h(qc, 0);
circuit_cx(qc, 0, 1);

char *text = circuit_to_text(qc, NULL);
printf("%s\n", text);
cqlib_string_free(text);
```

输出形如：

```text
     ┌───┐
q_0: ┤ H ├──■──
     └───┘┌─┴─┐
q_1: ─────┤ X ├
          └───┘
```

### 显示参数与折行

```c
TextDrawerOptionsC opts = {0};
opts.show_params = 1;      // 显示门参数
opts.line_width = 40;      // 超宽时折行

char *text = circuit_to_text(qc, &opts);
printf("%s\n", text);
cqlib_string_free(text);
```

SVG 图形渲染见 [FigureDrawer](2_figure_drawer.md)。
