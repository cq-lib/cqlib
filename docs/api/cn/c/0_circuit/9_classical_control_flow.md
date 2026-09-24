# 经典变量与控制流

本页介绍 C API 中与经典数据和结构化控制流相关的函数：经典变量的分配与查询、经典表达式的构造与内省、测量声明、经典存储，以及基于回调的 `if` / `if-else` / `while` / `for` / `switch` 控制流。错误码与内存管理约定见 [Overview](../0_overview.md)。

---

## 类型标签与快照结构

经典数据的静态类型由 (tag, width) 二元组描述，通过 `CClassicalType` 快照传递：

```c
typedef struct CClassicalType {
  uint32_t tag;    /* CQLIB_CLASSICAL_TYPE_* 之一 */
  uint32_t width;  /* 位宽，Bit/Bool 恒为 1 */
} CClassicalType;
```

| 类型标签 | 值 | 类型 | 说明 |
| --- | --- | --- | --- |
| `CQLIB_CLASSICAL_TYPE_BIT` | 0 | `Bit` | 单个 bit，取值 0 或 1，常用于单量子比特测量结果。 |
| `CQLIB_CLASSICAL_TYPE_BOOL` | 1 | `Bool` | 逻辑布尔值，用于 `if` / `while` 条件。 |
| `CQLIB_CLASSICAL_TYPE_UINT` | 2 | `UInt(width)` | 指定位宽的无符号整数。 |
| `CQLIB_CLASSICAL_TYPE_BIT_VEC` | 3 | `BitVec(width)` | 指定位宽的 bit 向量。 |

表达式读取的不可变经典值以 `CClassicalValueInfo` 快照表示：

```c
typedef struct CClassicalValueInfo {
  uint32_t index;  /* 线路内值表下标 */
  uint32_t tag;    /* CQLIB_CLASSICAL_TYPE_* 之一 */
  uint32_t width;  /* 位宽，Bit/Bool 恒为 1 */
} CClassicalValueInfo;
```

表达式节点种类由 `classical_expr_kind` 写出的标签描述：

| 节点标签 | 值 | 节点 |
| --- | --- | --- |
| `CQLIB_CLASSICAL_EXPR_VAR` | 0 | 变量读取 |
| `CQLIB_CLASSICAL_EXPR_VALUE` | 1 | 不可变值读取 |
| `CQLIB_CLASSICAL_EXPR_BOOL_LITERAL` | 2 | `Bool` 字面量 |
| `CQLIB_CLASSICAL_EXPR_BIT_LITERAL` | 3 | `Bit` 字面量 |
| `CQLIB_CLASSICAL_EXPR_UINT_LITERAL` | 4 | `UInt` 字面量 |
| `CQLIB_CLASSICAL_EXPR_BIT_VEC_LITERAL` | 5 | `BitVec` 字面量 |
| `CQLIB_CLASSICAL_EXPR_UNARY` | 6 | 一元运算（`not`） |
| `CQLIB_CLASSICAL_EXPR_BINARY` | 7 | 二元运算（`and` / `or` / `xor`） |
| `CQLIB_CLASSICAL_EXPR_COMPARE` | 8 | 比较（`eq` / `ne` / `lt` / `le` / `gt` / `ge`） |
| `CQLIB_CLASSICAL_EXPR_CAST` | 9 | 类型转换（`bit_to_bool` / `bit_vec_to_uint`） |
| `CQLIB_CLASSICAL_EXPR_SELECT` | 10 | 三元选择 |
| `CQLIB_CLASSICAL_EXPR_EXTRACT_BIT` | 11 | 单 bit 提取 |
| `CQLIB_CLASSICAL_EXPR_EXTRACT_BITS` | 12 | 连续 bit 区间提取 |
| `CQLIB_CLASSICAL_EXPR_CONCAT` | 13 | `BitVec` 拼接 |
| `CQLIB_CLASSICAL_EXPR_PACK_BITS` | 14 | `Bit` 打包 |

---

## 经典变量

### circuit_var(ptr, ty_tag, width)

在线路中分配一个可变经典变量，句柄归调用者所有。

- `ptr` (`CCircuit*`)：目标线路句柄。
- `ty_tag` (`uint32_t`)：类型标签，取 `CQLIB_CLASSICAL_TYPE_*` 之一。
- `width` (`uint32_t`)：位宽，仅对 `UInt` / `BitVec` 生效（0 会被拒绝）；对 `Bit` / `Bool` 忽略。

返回新分配的 `CClassicalVar*`；线路句柄为 NULL 或类型参数非法时返回 NULL。

### classical_var_free(ptr)

释放经典变量句柄。

- `ptr` (`CClassicalVar*`)：待释放句柄，允许传 NULL。

无返回值。

### classical_var_id(ptr)

返回变量在线路内的稳定身份标识。

- `ptr` (`const CClassicalVar*`)：变量句柄。

返回 `uint32_t` 身份 id；句柄为 NULL 时返回 `UINT32_MAX`。

### classical_var_index(ptr)

返回变量在线路变量表中的下标。

- `ptr` (`const CClassicalVar*`)：变量句柄。

返回 `uint32_t` 表下标；句柄为 NULL 时返回 `UINT32_MAX`。

### classical_var_ty(ptr, tag, width)

写出变量的类型（标签 + 位宽）。

- `ptr` (`const CClassicalVar*`)：变量句柄。
- `tag` (`uint32_t*`)：写出 `CQLIB_CLASSICAL_TYPE_*` 标签的出参。
- `width` (`uint32_t*`)：写出位宽的出参。

返回 0 表示成功；-1 表示指针为 NULL。

### classical_var_expr(ptr)

创建一个读取该变量当前运行时值的表达式。

- `ptr` (`const CClassicalVar*`)：变量句柄。

返回新分配的 `CClassicalExpr*`（用 `classical_expr_free` 释放）；句柄为 NULL 时返回 NULL。

### circuit_classical_vars_len(ptr)

返回线路已分配的经典变量数量。

- `ptr` (`const CCircuit*`)：目标线路句柄。

返回变量数量；句柄为 NULL 时返回 0。

### circuit_classical_vars(ptr, buffer, len)

按两步式把线路全部经典变量的类型快照拷入 `buffer`（先调用 `circuit_classical_vars_len` 获取长度）。

- `ptr` (`const CCircuit*`)：目标线路句柄。
- `buffer` (`CClassicalType*`)：接收 (tag, width) 快照的数组。
- `len` (`uintptr_t`)：数组长度，必须等于 `circuit_classical_vars_len`。

返回 0 表示成功；-1 表示指针为 NULL；-8 表示 `len` 与变量数量不符。

### circuit_validate_classical_var(ptr, var)

检查经典变量属于该线路且类型记录一致。

- `ptr` (`const CCircuit*`)：目标线路句柄。
- `var` (`const CClassicalVar*`)：待检查的变量句柄。

返回 0 表示通过；-1 表示指针为 NULL；-3 表示校验失败。

### circuit_validate_classical_expr(ptr, expr)

检查表达式读取的每个经典变量和经典值都属于该线路。

- `ptr` (`const CCircuit*`)：目标线路句柄。
- `expr` (`const CClassicalExpr*`)：待检查的表达式句柄。

返回 0 表示通过；-1 表示指针为 NULL；-3 表示校验失败。

---

## 经典表达式构造器

所有构造器都返回新分配的 `CClassicalExpr*`（用 `classical_expr_free` 释放），失败时返回 NULL。

### classical_expr_free(ptr)

释放经典表达式句柄。

- `ptr` (`CClassicalExpr*`)：待释放句柄，允许传 NULL。

无返回值。

### classical_expr_bit_literal(value)

创建 `Bit` 字面量表达式（false = 0，true = 1）。

- `value` (`bool`)：字面量取值。

返回新表达式句柄。

### classical_expr_bool_literal(value)

创建 `Bool` 字面量表达式。

- `value` (`bool`)：字面量取值。

返回新表达式句柄。

### classical_expr_uint_literal(width, lo, hi)

创建 `UInt(width)` 字面量。128 位数值以两个 64 位半字传递（`lo` 为最低有效位）。

- `width` (`uint32_t`)：位宽。
- `lo` (`uint64_t`)：数值低 64 位。
- `hi` (`uint64_t`)：数值高 64 位。

返回新表达式句柄；位宽为 0、超过 128 或数值不能容纳于该位宽时返回 NULL。

### classical_expr_bit_vec_literal(width, lo, hi)

创建 `BitVec(width)` 字面量。128 位按小端打包的数值以两个 64 位半字传递（`lo` 为最低有效位）。

- `width` (`uint32_t`)：位宽。
- `lo` (`uint64_t`)：打包值低 64 位。
- `hi` (`uint64_t`)：打包值高 64 位。

返回新表达式句柄；位宽为 0、超过 128 或数值不能容纳于该位宽时返回 NULL。

### classical_type_zero_literal(ty_tag, width)

创建某经典类型的零字面量。

- `ty_tag` (`uint32_t`)：类型标签，取 `CQLIB_CLASSICAL_TYPE_*` 之一。
- `width` (`uint32_t`)：位宽，用于 `UInt` / `BitVec`。

返回新表达式句柄；类型参数非法或位宽超出 128 位字面量表示范围时返回 NULL。

### classical_type_one_literal(ty_tag, width)

创建某经典类型的一字面量。

- `ty_tag` (`uint32_t`)：类型标签，取 `CQLIB_CLASSICAL_TYPE_*` 之一。
- `width` (`uint32_t`)：位宽，用于 `UInt` / `BitVec`。

返回新表达式句柄；类型参数非法或位宽超出 128 位字面量表示范围时返回 NULL。

### classical_expr_var(var)

创建读取 `var` 当前运行时值的表达式。

- `var` (`const CClassicalVar*`)：变量句柄。

返回新表达式句柄；句柄为 NULL 时返回 NULL。

### classical_expr_not(expr)

创建 `Bool` 或 `Bit` 操作数上的 `not` 表达式。

- `expr` (`const CClassicalExpr*`)：操作数。

返回新表达式句柄；输入为 NULL 或操作数类型非法时返回 NULL。

### classical_expr_and(lhs, rhs)

创建 `and` 表达式，要求两侧同为 `Bool` 或同为 `Bit`。

- `lhs` (`const CClassicalExpr*`)：左操作数。
- `rhs` (`const CClassicalExpr*`)：右操作数。

返回新表达式句柄；输入为 NULL 或类型不匹配时返回 NULL。

### classical_expr_or(lhs, rhs)

创建 `or` 表达式，要求两侧同为 `Bool` 或同为 `Bit`。

- `lhs` (`const CClassicalExpr*`)：左操作数。
- `rhs` (`const CClassicalExpr*`)：右操作数。

返回新表达式句柄；输入为 NULL 或类型不匹配时返回 NULL。

### classical_expr_xor(lhs, rhs)

创建 `xor` 表达式，要求两侧同为 `Bool` 或同为 `Bit`。

- `lhs` (`const CClassicalExpr*`)：左操作数。
- `rhs` (`const CClassicalExpr*`)：右操作数。

返回新表达式句柄；输入为 NULL 或类型不匹配时返回 NULL。

### classical_expr_eq(lhs, rhs)

创建相等比较，产出 `Bool`；要求两侧类型相同。

- `lhs` (`const CClassicalExpr*`)：左操作数。
- `rhs` (`const CClassicalExpr*`)：右操作数。

返回新表达式句柄；输入为 NULL 或类型不匹配时返回 NULL。

### classical_expr_ne(lhs, rhs)

创建不等比较，产出 `Bool`；要求两侧类型相同。

- `lhs` (`const CClassicalExpr*`)：左操作数。
- `rhs` (`const CClassicalExpr*`)：右操作数。

返回新表达式句柄；输入为 NULL 或类型不匹配时返回 NULL。

### classical_expr_lt(lhs, rhs)

创建无符号小于比较，产出 `Bool`；要求两侧为位宽一致的 `UInt`。

- `lhs` (`const CClassicalExpr*`)：左操作数。
- `rhs` (`const CClassicalExpr*`)：右操作数。

返回新表达式句柄；输入为 NULL 或操作数类型非法时返回 NULL。

### classical_expr_le(lhs, rhs)

创建无符号小于等于比较，产出 `Bool`；要求两侧为位宽一致的 `UInt`。

- `lhs` (`const CClassicalExpr*`)：左操作数。
- `rhs` (`const CClassicalExpr*`)：右操作数。

返回新表达式句柄；输入为 NULL 或类型非法时返回 NULL。

### classical_expr_gt(lhs, rhs)

创建无符号大于比较，产出 `Bool`；要求两侧为位宽一致的 `UInt`。

- `lhs` (`const CClassicalExpr*`)：左操作数。
- `rhs` (`const CClassicalExpr*`)：右操作数。

返回新表达式句柄；输入为 NULL 或操作数类型非法时返回 NULL。

### classical_expr_ge(lhs, rhs)

创建无符号大于等于比较，产出 `Bool`；要求两侧为位宽一致的 `UInt`。

- `lhs` (`const CClassicalExpr*`)：左操作数。
- `rhs` (`const CClassicalExpr*`)：右操作数。

返回新表达式句柄；输入为 NULL 或类型非法时返回 NULL。

### classical_expr_select(condition, then_expr, else_expr)

创建三元选择表达式 `condition ? then_expr : else_expr`。条件必须是 `Bool`，两个分支类型必须相同。

- `condition` (`const CClassicalExpr*`)：条件表达式。
- `then_expr` (`const CClassicalExpr*`)：条件成立时取值的表达式。
- `else_expr` (`const CClassicalExpr*`)：条件不成立时取值的表达式。

返回新表达式句柄；输入为 NULL 或操作数类型非法时返回 NULL。

### classical_expr_extract_bit(value, index)

从 `UInt` 或 `BitVec` 表达式中提取单个 bit（下标 0 为最低有效位），产出 `Bit`。

- `value` (`const CClassicalExpr*`)：被提取的表达式。
- `index` (`uint32_t`)：bit 下标。

返回新表达式句柄；输入为 NULL、操作数非整数类型或下标越界时返回 NULL。

### classical_expr_extract_bits(value, offset, width)

从 `UInt` 或 `BitVec` 表达式中提取连续 bit 区间 `[offset, offset + width)`，产出 `BitVec(width)`。偏移 0 从最低有效位开始。

- `value` (`const CClassicalExpr*`)：被提取的表达式。
- `offset` (`uint32_t`)：起始偏移。
- `width` (`uint32_t`)：区间宽度。

返回新表达式句柄；输入为 NULL 或区间非法时返回 NULL。

### classical_expr_pack_bits(bits, len)

把一组 `Bit` 表达式打包为 `BitVec`；第一个 bit 成为输出 bit 下标 0。

- `bits` (`const CClassicalExpr* const*`)：表达式句柄数组。
- `len` (`uintptr_t`)：数组长度。

返回新表达式句柄；出现 NULL 元素、空输入或操作数不是 `Bit` 时返回 NULL。

### classical_expr_concat(parts, len)

把一组 `Bit` / `BitVec` 表达式拼接为更宽的 `BitVec`；第一段占据输出的最低有效位。

- `parts` (`const CClassicalExpr* const*`)：表达式句柄数组。
- `len` (`uintptr_t`)：数组长度。

返回新表达式句柄；出现 NULL 元素、空输入或段类型非法时返回 NULL。

### classical_expr_bit_to_bool(expr)

把 `Bit` 表达式显式转换为 `Bool`。

- `expr` (`const CClassicalExpr*`)：待转换表达式。

返回新表达式句柄；输入为 NULL 或操作数不是 `Bit` 时返回 NULL。

### classical_expr_bit_vec_to_uint(expr)

把 `BitVec` 表达式显式转换为等位宽的小端 `UInt`。

- `expr` (`const CClassicalExpr*`)：待转换表达式。

返回新表达式句柄；输入为 NULL 或操作数不是 `BitVec` 时返回 NULL。

### classical_expr_to_bool(expr)

把 `Bit` 表达式转换为 `Bool`（`classical_expr_bit_to_bool` 的别名）。

- `expr` (`const CClassicalExpr*`)：待转换表达式。

返回新表达式句柄；输入为 NULL 或操作数不是 `Bit` 时返回 NULL。

### classical_expr_to_uint(expr)

把 `BitVec` 表达式转换为小端 `UInt`（`classical_expr_bit_vec_to_uint` 的别名）。

- `expr` (`const CClassicalExpr*`)：待转换表达式。

返回新表达式句柄；输入为 NULL 或操作数不是 `BitVec` 时返回 NULL。

### classical_expr_simplified(ptr)

返回表达式的结构化简副本。化简消除由字面量驱动的冗余，不会对运行时读取求值。

- `ptr` (`const CClassicalExpr*`)：原表达式句柄。

返回新分配的化简副本；输入为 NULL 时返回 NULL。

---

## 表达式查询

### classical_expr_kind(ptr, kind)

写出表达式节点种类标签（`CQLIB_CLASSICAL_EXPR_*` 之一）。

- `ptr` (`const CClassicalExpr*`)：表达式句柄。
- `kind` (`uint32_t*`)：接收标签的出参。

返回 0 表示成功；-1 表示指针为 NULL。

### classical_expr_ty(ptr, tag, width)

写出表达式的静态类型（标签 + 位宽）。

- `ptr` (`const CClassicalExpr*`)：表达式句柄。
- `tag` (`uint32_t*`)：写出 `CQLIB_CLASSICAL_TYPE_*` 标签的出参。
- `width` (`uint32_t*`)：写出位宽的出参。

返回 0 表示成功；-1 表示指针为 NULL。

### classical_expr_is_bit_true(ptr)

判断表达式是否为 `Bit` 字面量 `true`（1）。

- `ptr` (`const CClassicalExpr*`)：表达式句柄。

返回 `1` 是；`0` 否；`-1` NULL。

### classical_expr_is_bit_false(ptr)

判断表达式是否为 `Bit` 字面量 `false`（0）。

- `ptr` (`const CClassicalExpr*`)：表达式句柄。

返回 `1` 是；`0` 否；`-1` NULL。

### classical_expr_is_bool_true(ptr)

判断表达式是否为 `Bool` 字面量 `true`。

- `ptr` (`const CClassicalExpr*`)：表达式句柄。

返回 `1` 是；`0` 否；`-1` NULL。

### classical_expr_is_bool_false(ptr)

判断表达式是否为 `Bool` 字面量 `false`。

- `ptr` (`const CClassicalExpr*`)：表达式句柄。

返回 `1` 是；`0` 否；`-1` NULL。

### classical_expr_values_len(ptr)

返回表达式读取的不同不可变经典值的数量。

- `ptr` (`const CClassicalExpr*`)：表达式句柄。

返回数量；句柄为 NULL 时返回 0。

### classical_expr_values(ptr, buffer, len)

按两步式把表达式读取的全部不可变经典值快照 (index, tag, width) 拷入 `buffer`（先调用 `classical_expr_values_len` 获取长度）。

- `ptr` (`const CClassicalExpr*`)：表达式句柄。
- `buffer` (`CClassicalValueInfo*`)：接收快照的数组。
- `len` (`uintptr_t`)：数组长度，必须等于 `classical_expr_values_len`。

返回 0 表示成功；-1 表示指针为 NULL；-8 表示 `len` 与数量不符。

### classical_expr_vars_len(ptr)

返回表达式读取的不同经典变量的数量。

- `ptr` (`const CClassicalExpr*`)：表达式句柄。

返回数量；句柄为 NULL 时返回 0。

### classical_expr_vars(ptr, buffer, len)

按两步式把表达式读取的全部经典变量的自有句柄填入 `buffer`，按 id 升序排列（先调用 `classical_expr_vars_len` 获取长度）。每个返回的句柄必须用 `classical_var_free` 释放。

- `ptr` (`const CClassicalExpr*`)：表达式句柄。
- `buffer` (`CClassicalVar**`)：接收自有句柄的数组。
- `len` (`uintptr_t`)：数组长度，必须等于 `classical_expr_vars_len`。

返回 0 表示成功；-1 表示指针为 NULL；-8 表示 `len` 与数量不符。

### classical_expr_remap_classical_ids(ptr, old_var_ids, new_var_ids, var_len, old_value_indices, new_value_indices, value_len)

返回把线路内经典 id 重映射后的表达式副本。`old_var_ids[i]` 把该 id 的变量映射到 `new_var_ids[i]`，`old_value_indices[i]` 把该下标的不可变值映射到 `new_value_indices[i]`。旧 id 未在表达式中出现的表项会被忽略，但表达式实际读取的每个变量和值都必须被覆盖，否则重映射失败。

- `ptr` (`const CClassicalExpr*`)：原表达式句柄。
- `old_var_ids` (`const uint32_t*`)：旧变量 id 数组。
- `new_var_ids` (`const uint32_t*`)：新变量 id 数组，按下标配对。
- `var_len` (`uintptr_t`)：变量映射数组长度。
- `old_value_indices` (`const uint32_t*`)：旧不可变值下标数组。
- `new_value_indices` (`const uint32_t*`)：新值下标数组，按下标配对。
- `value_len` (`uintptr_t`)：值映射数组长度。

返回新表达式句柄（用 `classical_expr_free` 释放）；输入为 NULL（含长度非零时的 NULL 数组）或映射缺失时返回 NULL。

---

## 测量声明

### circuit_measure_bits(ptr, qubits, len)

测量多个量子比特并返回读取不可变 `BitVec` 结果的表达式。第一个量子比特对应 bit 下标 0（最低有效位）。

- `ptr` (`CCircuit*`)：目标线路句柄。
- `qubits` (`const uint32_t*`)：量子比特 id 数组。
- `len` (`uintptr_t`)：量子比特数量。

返回新分配的 `CClassicalExpr*`；指针为 NULL、列表为空或量子比特越界时返回 NULL。

### circuit_measure_into(ptr, qubit, target)

测量单个量子比特并把结果存入 `target`（`Bit` 类型变量），返回读取该不可变测量值的表达式。

- `ptr` (`CCircuit*`)：目标线路句柄。
- `qubit` (`uint32_t`)：被测量子比特 id。
- `target` (`const CClassicalVar*`)：接收结果的 `Bit` 变量句柄。

返回新分配的 `CClassicalExpr*`；指针为 NULL、量子比特越界或目标类型错误时返回 NULL。

### circuit_measure_bits_into(ptr, qubits, len, target)

测量多个量子比特并把 `BitVec` 结果存入 `target`。目标位宽必须等于量子比特数量。

- `ptr` (`CCircuit*`)：目标线路句柄。
- `qubits` (`const uint32_t*`)：量子比特 id 数组。
- `len` (`uintptr_t`)：量子比特数量。
- `target` (`const CClassicalVar*`)：接收结果的 `BitVec` 变量句柄。

返回读取不可变测量值的 `CClassicalExpr*`；出错时返回 NULL。

---

## 经典存储

### circuit_store(ptr, target, value)

把 `value` 的运行时值存入可变变量 `target`。表达式类型必须与目标类型一致。

- `ptr` (`CCircuit*`)：目标线路句柄。
- `target` (`const CClassicalVar*`)：写入目标变量。
- `value` (`const CClassicalExpr*`)：提供值的表达式。

返回 0 表示成功；-1 表示指针为 NULL；-3 表示校验失败。

---

## 控制流

控制流函数通过 C 回调构造分支体和循环体：

```c
typedef int32_t (*CqlibBodyFn)(struct CCircuit *circuit, void *user_data);
```

回调契约：

- `circuit` 与传给控制流函数的 `CCircuit*` 是同一个指针，仅在回调期间有效；分支体操作必须通过它用常规线路函数（`circuit_h`、`circuit_x`、`circuit_measure`、嵌套控制流构造器等）追加。
- `user_data` 原样透传，绑定层不会解引用，可为 NULL。
- 返回 0 表示成功；任何非零返回都会中止整个控制流操作：回调追加的全部操作被回滚，该值原样传播给控制流函数的调用者。直接返回门函数的错误码（如 `return circuit_x(circuit, 0);`）是惯用写法。

### circuit_if(ptr, condition, body_fn, user_data)

向线路追加一个由 `Bool` 经典表达式控制的 `if` 操作。`body_fn` 被调用一次，其追加的全部操作构成 then 分支体。

- `ptr` (`CCircuit*`)：目标线路句柄。
- `condition` (`const CClassicalExpr*`)：条件表达式。
- `body_fn` (`CqlibBodyFn`)：构造分支体的回调。
- `user_data` (`void*`)：传给回调的用户指针，可为 NULL。

返回 0 表示成功；-1 表示参数为 NULL；-3 表示条件校验失败（例如非 `Bool` 类型或句柄来自其他线路）；否则为 `body_fn` 返回的非零状态码（回滚后传播）。

### circuit_if_else(ptr, condition, then_fn, else_fn, user_data)

向线路追加一个由 `Bool` 经典表达式控制的 `if` / `else` 操作。`then_fn` 先执行、`else_fn` 后执行，各自追加的操作成为对应分支体。

- `ptr` (`CCircuit*`)：目标线路句柄。
- `condition` (`const CClassicalExpr*`)：条件表达式。
- `then_fn` (`CqlibBodyFn`)：构造 then 分支体的回调。
- `else_fn` (`CqlibBodyFn`)：构造 else 分支体的回调。
- `user_data` (`void*`)：传给两个回调的用户指针，可为 NULL。

返回 0 表示成功；-1 表示参数为 NULL；-3 表示校验失败；否则为任一回调返回的非零状态码（回滚后传播）。

### circuit_while(ptr, condition, body_fn, user_data)

向线路追加一个由 `Bool` 经典表达式控制的 `while` 循环。条件在运行时反复求值。`circuit_break_loop` / `circuit_continue_loop` 可以在回调中调用，但只能作为分支体的最后一个操作（终端控制转移）。

- `ptr` (`CCircuit*`)：目标线路句柄。
- `condition` (`const CClassicalExpr*`)：循环条件表达式。
- `body_fn` (`CqlibBodyFn`)：构造循环体的回调。
- `user_data` (`void*`)：传给回调的用户指针，可为 NULL。

返回 0 表示成功；-1 表示参数为 NULL；-3 表示校验失败；否则为 `body_fn` 返回的非零状态码（回滚后传播）。

### circuit_for_uint(ptr, var, start, stop, step, body_fn, user_data)

向线路追加无符号运行时区间循环，区间为半开 `[start, stop)`。`var` 必须是本线路的 `UInt` 变量；`start`、`stop`、`step` 必须是同位宽的 `UInt` 表达式。循环体内部需要读取循环变量时，可用 `classical_expr_var(var)` 重建其表达式。

- `ptr` (`CCircuit*`)：目标线路句柄。
- `var` (`const CClassicalVar*`)：循环变量。
- `start` (`const CClassicalExpr*`)：起点表达式。
- `stop` (`const CClassicalExpr*`)：终点表达式（不含）。
- `step` (`const CClassicalExpr*`)：步长表达式。
- `body_fn` (`CqlibBodyFn`)：构造循环体的回调。
- `user_data` (`void*`)：传给回调的用户指针，可为 NULL。

返回 0 表示成功；-1 表示参数为 NULL；-3 表示校验失败（类型不匹配或外部句柄）；否则为 `body_fn` 返回的非零状态码（回滚后传播）。

### circuit_switch(ptr, target, case_values_lo, case_values_hi, case_bodies, case_count, default_body, user_data)

向线路追加基于 `UInt` 表达式的精确值匹配 `switch` 操作。case 值为 128 位，拆成两个 64 位半字：`value = lo | (hi << 64)`。每个值必须能被目标位宽表示且至多出现一次；`default_body` 可为 NULL（无默认分支）。所有回调接收同一个 `user_data`。

- `ptr` (`CCircuit*`)：目标线路句柄。
- `target` (`const CClassicalExpr*`)：`UInt` 目标表达式。
- `case_values_lo` (`const uint64_t*`)：各 case 值的低 64 位。
- `case_values_hi` (`const uint64_t*`)：各 case 值的高 64 位。
- `case_bodies` (`const CqlibBodyFn*`)：与 case 值一一对应的回调数组。
- `case_count` (`uintptr_t`)：case 数量。
- `default_body` (`CqlibBodyFn`)：默认分支回调，可为 NULL。
- `user_data` (`void*`)：传给所有回调的用户指针。

返回 0 表示成功；-1 表示参数为 NULL（含 `case_count > 0` 时 case 体为 NULL）；-3 表示校验失败（目标非 `UInt`、case 值重复或越界）；否则为回调返回的第一个非零状态码（回滚后传播）。

### circuit_break_loop(ptr)

向最近的包围循环或 switch 体追加 `break`。仅可在 `circuit_while`、`circuit_for_uint` 或 `circuit_switch` 的体回调（或嵌套于其中的体）内调用，且只能作为该体的最后一个操作。

- `ptr` (`CCircuit*`)：目标线路句柄。

返回 0 表示成功；-1 表示线路为 NULL；-3 表示没有激活的包围循环或 switch 体，或该转移不是终端转移。

### circuit_continue_loop(ptr)

向最近的包围循环体追加 `continue`。仅可在 `circuit_while` 或 `circuit_for_uint` 的体回调（或嵌套于其中的体）内调用，且只能作为该体的最后一个操作。

- `ptr` (`CCircuit*`)：目标线路句柄。

返回 0 表示成功；-1 表示线路为 NULL；-3 表示没有激活的包围循环体，或该转移不是终端转移。

### circuit_append_control(ptr, source, index)

把 `source` 操作列表中下标为 `index` 的控制流操作经验证后的副本追加到 `ptr`。`ptr` 与 `source` 可以是同一线路（复制自身的某个控制操作）。跨线路复制要求该操作是自包含的（不引用 `source` 的经典句柄）。

- `ptr` (`CCircuit*`)：目标线路句柄。
- `source` (`const CCircuit*`)：来源线路句柄。
- `index` (`uintptr_t`)：来源操作列表中的下标。

返回 0 表示成功；-1 表示参数为 NULL；-8 表示 `index` 越界或该位置不是控制流操作；-3 表示副本校验失败（外部经典句柄、非法 `break` / `continue` 作用域或未知量子比特）。

---

## 完整示例

声明经典变量 → 构建布尔表达式 → `if` / `else` 分支：

```c
#include <stdio.h>
#include "cqlib_c.h"

static int32_t then_body(struct CCircuit *circuit, void *user_data) {
    (void)user_data;
    return circuit_x(circuit, 1);   /* 条件成立：对 qubit 1 施加 X */
}

static int32_t else_body(struct CCircuit *circuit, void *user_data) {
    (void)user_data;
    return circuit_z(circuit, 1);   /* 条件不成立：对 qubit 1 施加 Z */
}

int main(void) {
    struct CCircuit *circuit = circuit_new(2);
    if (circuit == NULL) {
        return 1;
    }

    /* 1. 声明一个 Bool 经典变量 */
    struct CClassicalVar *flag = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_BOOL, 0);

    /* 2. 构建布尔表达式：flag == true */
    struct CClassicalExpr *flag_read = classical_expr_var(flag);
    struct CClassicalExpr *true_lit = classical_expr_bool_literal(true);
    struct CClassicalExpr *cond = classical_expr_eq(flag_read, true_lit);

    /* 3. if / else 分支 */
    int32_t status = circuit_if_else(circuit, cond, then_body, else_body, NULL);
    if (status != 0) {
        fprintf(stderr, "circuit_if_else failed: %d\n", status);
    }

    /* 释放表达式与变量句柄 */
    classical_expr_free(cond);
    classical_expr_free(true_lit);
    classical_expr_free(flag_read);
    classical_var_free(flag);
    circuit_free(circuit);
    return status;
}
```

线路构造函数（`circuit_new` / `circuit_free`）见 [Circuit](1_circuit.md)，标准门函数（`circuit_x` / `circuit_z`）见 [标准门](5_gate_standard.md)。
