# 知识库（C）

`CKnowledgeLibrary` 是验证后的编译规则库句柄：一组按类别登记、按名称索引的知识规则；`CKnowledgeRule` 是单条规则的独立句柄，可解析、校验后插入任意库。知识规则描述一个相邻操作模式（match）、可选符号参数约束与替换序列（rewrite），是知识重写与门分解的规则来源。一次匹配产生记录绑定比特与参数的 `CKnowledgeBindings*`；在这些绑定下实例化的替换目标以 `CKnowledgeReplacements*` 返回。错误码、字符串与句柄的释放约定遵循 [Overview](../0_overview.md)。

---

## 常量

规则类别标签（`kind` 参数与 `knowledge_library_rule_kind` 的返回值）：

| 常量 | 值 | 类别 |
| --- | --- | --- |
| `RULE_KIND_SIMPLIFY` | 0 | 化简规则。 |
| `RULE_KIND_CANCEL` | 1 | 抵消规则（相邻互逆操作删除）。 |
| `RULE_KIND_MERGE` | 2 | 合并规则。 |
| `RULE_KIND_COMMUTE` | 3 | 显式对易规则（`A; B -> B; A`）。 |
| `RULE_KIND_DECOMPOSE` | 4 | 分解或降级规则。 |
| `RULE_KIND_CANONICALIZE` | 5 | 规范化规则。 |
| `RULE_KIND_HARDWARE_NATIVE` | 6 | 硬件原生规则。 |
| `RULE_KIND_OTHER` | 7 | 其他类别。 |

---

## DSL 规则

`knowledge_library_from_dsl_str` / `knowledge_library_from_dsl_file` / `knowledge_rule_from_dsl` 接受 `.rule` DSL 文本：

```text
rule merge_rz {
    match {
        RZ(a) 0
        RZ(b) 0
    }
    rewrite {
        RZ(a + b) 0
    }
}
```

- `rule <name>`：规则名，库内唯一。
- `match { ... }`：被匹配的相邻操作模式，至少一条。
- `rewrite { ... }`：替换序列，可为空（空替换表示抵消删除）。
- 每条操作为 `门名 (参数) 比特标签...`；无参数时省略括号（如 `H 0`）。
- 比特标签是非负整数占位符，必须从 0 开始连续排列，匹配时绑定到具体比特。
- 参数可为数值、符号（`a`、`b`）或表达式（`a + b`）。

---

## 库构造与释放

### knowledge_library_new()

```c
struct CKnowledgeLibrary *knowledge_library_new(void);
```

创建空规则库。

返回值：

| 返回 | 场景 |
| --- | --- |
| 非 NULL | 新建的空库句柄，用 `knowledge_library_free` 释放。 |

### knowledge_library_builtin()

```c
struct CKnowledgeLibrary *knowledge_library_builtin(void);
```

创建包含内置编译规则的库。规则按各自的编译类别登记，可用 `knowledge_library_rules_by_kind_len` 按类别查询。

返回值：

| 返回 | 场景 |
| --- | --- |
| 非 NULL | 新建的内置库句柄，用 `knowledge_library_free` 释放。 |
| NULL | 内置规则源验证失败。 |

### knowledge_library_from_dsl_str(source, kind, out)

```c
int32_t knowledge_library_from_dsl_str(const char *source,
                                       uint8_t kind,
                                       struct CKnowledgeLibrary **out);
```

解析并验证 DSL 源文本构造规则库；`kind`（`RULE_KIND_*` 之一）作为全部解析规则的类别。

参数：

- `source` (`const char*`)：DSL 源文本。
- `kind` (`uint8_t`)：规则类别标签，取 `RULE_KIND_*` 之一。
- `out` (`struct CKnowledgeLibrary**`)：输出指针，成功时接收新建库句柄。

返回：0 成功并把新建的 `CKnowledgeLibrary*`（用 `knowledge_library_free` 释放）写入 `*out`。

错误码：

| 值 | 场景 |
| --- | --- |
| 0 | 成功。 |
| -1 | `source` 或 `out` 为 NULL。 |
| -4 | `source` 非法 UTF-8，或 DSL 解析、下降、验证失败。 |
| -8 | `kind` 不是合法的 `RULE_KIND_*` 标签。 |

### knowledge_library_from_dsl_file(path, kind, out)

```c
int32_t knowledge_library_from_dsl_file(const char *path,
                                        uint8_t kind,
                                        struct CKnowledgeLibrary **out);
```

从 DSL 文件加载、解析并验证规则构造规则库；`kind` 作为全部规则的类别。

参数：

- `path` (`const char*`)：DSL 文件路径。
- `kind` (`uint8_t`)：规则类别标签，取 `RULE_KIND_*` 之一。
- `out` (`struct CKnowledgeLibrary**`)：输出指针，成功时接收新建库句柄。

返回：0 成功并把新建的 `CKnowledgeLibrary*` 写入 `*out`。

错误码：

| 值 | 场景 |
| --- | --- |
| 0 | 成功。 |
| -1 | `path` 或 `out` 为 NULL。 |
| -4 | `path` 非法 UTF-8，或 DSL 解析、下降、验证失败。 |
| -5 | 文件读取失败。 |
| -8 | `kind` 不是合法的 `RULE_KIND_*` 标签。 |

### knowledge_library_free(ptr)

```c
void knowledge_library_free(struct CKnowledgeLibrary *ptr);
```

释放规则库句柄，允许传 NULL。

---

## 库查询

### knowledge_library_len(ptr)

```c
uintptr_t knowledge_library_len(const struct CKnowledgeLibrary *ptr);
```

返回库中规则数。

返回值：

| 返回 | 场景 |
| --- | --- |
| 非负整数 | 规则数。 |
| 0 | `ptr` 为 NULL 或库为空。 |

### knowledge_library_is_empty(ptr)

```c
int32_t knowledge_library_is_empty(const struct CKnowledgeLibrary *ptr);
```

判断库是否不含任何规则。

返回值：

| 值 | 场景 |
| --- | --- |
| 1 | 库为空。 |
| 0 | 库非空。 |
| -1 | `ptr` 为 NULL。 |

### knowledge_library_contains(ptr, name)

```c
int32_t knowledge_library_contains(const struct CKnowledgeLibrary *ptr, const char *name);
```

判断库中是否存在名为 `name` 的规则。

参数：

- `ptr` (`const CKnowledgeLibrary*`)：规则库句柄。
- `name` (`const char*`)：规则名。

返回值：

| 值 | 场景 |
| --- | --- |
| 1 | 存在同名规则。 |
| 0 | 不存在同名规则。 |
| -1 | `ptr` 或 `name` 为 NULL。 |
| -4 | `name` 非法 UTF-8。 |

### knowledge_library_id_by_name(ptr, name)

```c
int64_t knowledge_library_id_by_name(const struct CKnowledgeLibrary *ptr, const char *name);
```

查询规则名对应的规则 id（即插入顺序下标，规则 id 等于插入时的库长度）。

参数：

- `ptr` (`const CKnowledgeLibrary*`)：规则库句柄。
- `name` (`const char*`)：规则名。

返回值：

| 返回 | 场景 |
| --- | --- |
| 非负整数 | 规则 id。 |
| -1 | `ptr` 或 `name` 为 NULL，或不存在同名规则。 |
| -4 | `name` 非法 UTF-8。 |

### knowledge_library_rules_by_kind_len(ptr, kind) / knowledge_library_rules_by_kind(ptr, kind, out, len)

```c
uintptr_t knowledge_library_rules_by_kind_len(const struct CKnowledgeLibrary *ptr, uint8_t kind);
uintptr_t knowledge_library_rules_by_kind(const struct CKnowledgeLibrary *ptr,
                                          uint8_t kind,
                                          uint32_t *out,
                                          uintptr_t len);
```

两步式读取指定类别的规则 id 列表：`*_len` 返回该类别的规则总数；填充接口把 id 拷入 `out`（实际拷贝 `min(总数, len)` 个），返回总数。`out` 为 NULL 时只返回总数、不拷贝。

参数：

- `ptr` (`const CKnowledgeLibrary*`)：规则库句柄。
- `kind` (`uint8_t`)：规则类别标签，取 `RULE_KIND_*` 之一。
- `out` (`uint32_t*`)：接收 id 的缓冲区。
- `len` (`uintptr_t`)：缓冲区长度。

返回值（两者一致）：

| 返回 | 场景 |
| --- | --- |
| 非负整数 | 该类别的规则总数。 |
| 0 | `ptr` 为 NULL，或 `kind` 不是合法的 `RULE_KIND_*` 标签。 |

### knowledge_rules_len(ptr) / knowledge_rules(ptr, out, len)

```c
uintptr_t knowledge_rules_len(const struct CKnowledgeLibrary *ptr);
uintptr_t knowledge_rules(const struct CKnowledgeLibrary *ptr, uint32_t *out, uintptr_t len);
```

两步式读取库中的规则 id（插入顺序下标）列表：`*_len` 返回规则总数；填充接口把 id 拷入 `out`（实际拷贝 `min(总数, len)` 个），返回总数。`out` 为 NULL 时只返回总数、不拷贝。

参数：

- `ptr` (`const CKnowledgeLibrary*`)：规则库句柄。
- `out` (`uint32_t*`)：接收 id 的缓冲区。
- `len` (`uintptr_t`)：缓冲区长度。

返回值（两者一致）：

| 返回 | 场景 |
| --- | --- |
| 非负整数 | 规则总数。 |
| 0 | `ptr` 为 NULL。 |

### knowledge_get_by_name(ptr, name)

```c
struct CKnowledgeRule *knowledge_get_by_name(const struct CKnowledgeLibrary *ptr, const char *name);
```

按名称查询规则并返回克隆句柄。

参数：

- `ptr` (`const CKnowledgeLibrary*`)：规则库句柄。
- `name` (`const char*`)：规则名。

返回值：

| 返回 | 场景 |
| --- | --- |
| 非 NULL | 新建的 `CKnowledgeRule*` 克隆，用 `knowledge_rule_free` 释放。 |
| NULL | `ptr` 或 `name` 为 NULL，或不存在同名规则。 |

---

## 规则元数据

以下接口按插入下标 `index` 读取库中规则的预计算元数据；`index` 的合法范围是 `[0, knowledge_library_len(ptr))`。

### knowledge_library_rule_name(ptr, index)

```c
char *knowledge_library_rule_name(const struct CKnowledgeLibrary *ptr, uintptr_t index);
```

返回第 `index` 条规则的名称。

返回值：

| 返回 | 场景 |
| --- | --- |
| 非 NULL | 堆上 C 字符串，用 `cqlib_string_free` 释放。 |
| NULL | `ptr` 为 NULL 或 `index` 越界。 |

### knowledge_library_rule_kind(ptr, index)

```c
int32_t knowledge_library_rule_kind(const struct CKnowledgeLibrary *ptr, uintptr_t index);
```

返回第 `index` 条规则的类别标签。

返回值：

| 值 | 场景 |
| --- | --- |
| 0–7 | `RULE_KIND_*` 类别标签。 |
| -1 | `ptr` 为 NULL。 |
| -8 | `index` 越界。 |

### knowledge_library_rule_first_instruction(ptr, index)

```c
char *knowledge_library_rule_first_instruction(const struct CKnowledgeLibrary *ptr,
                                               uintptr_t index);
```

返回第 `index` 条规则匹配模式的首条指令名（按首指令可筛选候选规则）。

返回值：

| 返回 | 场景 |
| --- | --- |
| 非 NULL | 堆上 C 字符串，用 `cqlib_string_free` 释放。 |
| NULL | `ptr` 为 NULL 或 `index` 越界。 |

### knowledge_library_rule_pattern_len(ptr, index)

```c
uintptr_t knowledge_library_rule_pattern_len(const struct CKnowledgeLibrary *ptr, uintptr_t index);
```

返回规则匹配模式的操作数。

返回值：

| 返回 | 场景 |
| --- | --- |
| 非负整数 | 模式操作数。 |
| 0 | `ptr` 为 NULL 或 `index` 越界。 |

### knowledge_library_rule_rewrite_len(ptr, index)

```c
uintptr_t knowledge_library_rule_rewrite_len(const struct CKnowledgeLibrary *ptr, uintptr_t index);
```

返回规则替换目标发出的操作数。

返回值：

| 返回 | 场景 |
| --- | --- |
| 非负整数 | 替换目标操作数。 |
| 0 | `ptr` 为 NULL 或 `index` 越界。 |

### knowledge_library_rule_qubit_count(ptr, index)

```c
uintptr_t knowledge_library_rule_qubit_count(const struct CKnowledgeLibrary *ptr, uintptr_t index);
```

返回规则使用的不同规则局部比特标签数。

返回值：

| 返回 | 场景 |
| --- | --- |
| 非负整数 | 比特标签数。 |
| 0 | `ptr` 为 NULL 或 `index` 越界。 |

### knowledge_library_rule_cost_delta(ptr, index)

```c
int64_t knowledge_library_rule_cost_delta(const struct CKnowledgeLibrary *ptr, uintptr_t index);
```

返回规则的静态操作数差 `rewrite_len - pattern_len`（可为负，表示规则减少操作数）。

返回值：

| 返回 | 场景 |
| --- | --- |
| 整数 | `rewrite_len - pattern_len`。 |
| 0 | `ptr` 为 NULL 或 `index` 越界。 |

### knowledge_library_rule_has_conditions(ptr, index)

```c
int32_t knowledge_library_rule_has_conditions(const struct CKnowledgeLibrary *ptr, uintptr_t index);
```

判断规则是否带非空符号参数条件。

返回值：

| 值 | 场景 |
| --- | --- |
| 1 | 带非空参数条件。 |
| 0 | 不带参数条件。 |
| -1 | `ptr` 为 NULL。 |
| -8 | `index` 越界。 |

---

## 单条规则解析与插入

### knowledge_rule_from_dsl(source, out)

```c
int32_t knowledge_rule_from_dsl(const char *source, struct CKnowledgeRule **out);
```

解析必须恰好定义一条规则的 DSL 源文本，构造单条规则句柄（由 `knowledge_library_add_rule` 插入库）。

参数：

- `source` (`const char*`)：DSL 源文本（恰好一条 `rule` 定义）。
- `out` (`struct CKnowledgeRule**`)：输出指针，成功时接收新句柄。

返回：0 成功并把新建的 `CKnowledgeRule*`（用 `knowledge_rule_free` 释放）写入 `*out`。

错误码：

| 值 | 场景 |
| --- | --- |
| 0 | 成功。 |
| -1 | `source` 或 `out` 为 NULL。 |
| -4 | `source` 非法 UTF-8，或 DSL 解析、下降、验证失败。 |
| -8 | 源文本定义的规则数不等于 1。 |

### knowledge_rule_free(ptr)

```c
void knowledge_rule_free(struct CKnowledgeRule *ptr);
```

释放单条规则句柄，允许传 NULL。

### knowledge_library_add_rule(ptr, rule, kind)

```c
int64_t knowledge_library_add_rule(struct CKnowledgeLibrary *ptr,
                                   const struct CKnowledgeRule *rule,
                                   uint8_t kind);
```

把解析好的规则插入库并登记到类别 `kind`，返回分配的规则 id。插入前执行结构校验；规则名在库内必须唯一。

参数：

- `ptr` (`CKnowledgeLibrary*`)：目标规则库。
- `rule` (`const CKnowledgeRule*`)：待插入规则（克隆使用，句柄不被消耗，调用方继续负责释放）。
- `kind` (`uint8_t`)：规则类别标签，取 `RULE_KIND_*` 之一。

返回值：

| 返回 | 场景 |
| --- | --- |
| 非负整数 | 分配的规则 id。 |
| -1 | `ptr` 或 `rule` 为 NULL。 |
| -4 | 结构验证失败或规则名重复。 |
| -6 | 规则首指令无法被索引。 |
| -8 | `kind` 不是合法的 `RULE_KIND_*` 标签。 |

---

## 单条规则访问器

### knowledge_rule_name(ptr)

```c
char *knowledge_rule_name(const struct CKnowledgeRule *ptr);
```

返回规则名。

返回值：

| 返回 | 场景 |
| --- | --- |
| 非 NULL | 堆上 C 字符串，用 `cqlib_string_free` 释放。 |
| NULL | `ptr` 为 NULL。 |

### knowledge_rule_num_qubits(ptr)

```c
uintptr_t knowledge_rule_num_qubits(const struct CKnowledgeRule *ptr);
```

返回规则涉及的比特标签数。

返回值：

| 返回 | 场景 |
| --- | --- |
| 非负整数 | 比特标签数。 |
| 0 | `ptr` 为 NULL。 |

### knowledge_rule_pattern_len(ptr)

```c
uintptr_t knowledge_rule_pattern_len(const struct CKnowledgeRule *ptr);
```

返回规则匹配模式的操作数。

返回值：

| 返回 | 场景 |
| --- | --- |
| 非负整数 | 模式操作数。 |
| 0 | `ptr` 为 NULL。 |

### knowledge_rule_rewrite_len(ptr)

```c
uintptr_t knowledge_rule_rewrite_len(const struct CKnowledgeRule *ptr);
```

返回规则替换目标发出的操作数。

返回值：

| 返回 | 场景 |
| --- | --- |
| 非负整数 | 替换目标操作数。 |
| 0 | `ptr` 为 NULL。 |

### knowledge_rule_operation_symbols_len(ptr) / knowledge_rule_operation_symbols(ptr, out, len)

```c
uintptr_t knowledge_rule_operation_symbols_len(const struct CKnowledgeRule *ptr);
int32_t knowledge_rule_operation_symbols(const struct CKnowledgeRule *ptr,
                                         char **out,
                                         uintptr_t len);
```

两步式读取规则匹配块绑定的不同符号：`*_len` 返回符号总数；填充接口把符号（已排序、去重）以新建 C 字符串形式拷入 `out`，返回 0。每个字符串须用 `cqlib_string_free` 释放。

参数：

- `ptr` (`const CKnowledgeRule*`)：规则句柄。
- `out` (`char**`)：接收符号字符串的缓冲区。
- `len` (`uintptr_t`)：缓冲区长度，必须等于 `knowledge_rule_operation_symbols_len` 返回的数量。

返回值（`*_len`）：

| 返回 | 场景 |
| --- | --- |
| 非负整数 | 符号总数。 |
| 0 | `ptr` 为 NULL。 |

错误码（填充接口）：

| 值 | 场景 |
| --- | --- |
| 0 | 成功；`out` 持有 `len` 个新建字符串。 |
| -1 | `ptr` 为 NULL，或 `len > 0` 时 `out` 为 NULL。 |
| -8 | `len` 与符号数量不符。 |

### knowledge_rule_target_symbols_len(ptr) / knowledge_rule_target_symbols(ptr, out, len)

```c
uintptr_t knowledge_rule_target_symbols_len(const struct CKnowledgeRule *ptr);
int32_t knowledge_rule_target_symbols(const struct CKnowledgeRule *ptr,
                                      char **out,
                                      uintptr_t len);
```

两步式读取规则替换目标引用的不同符号：`*_len` 返回符号总数；填充接口把符号（已排序、去重）以新建 C 字符串形式拷入 `out`，返回 0。每个字符串须用 `cqlib_string_free` 释放。

参数：

- `ptr` (`const CKnowledgeRule*`)：规则句柄。
- `out` (`char**`)：接收符号字符串的缓冲区。
- `len` (`uintptr_t`)：缓冲区长度，必须等于 `knowledge_rule_target_symbols_len` 返回的数量。

返回值（`*_len`）：

| 返回 | 场景 |
| --- | --- |
| 非负整数 | 符号总数。 |
| 0 | `ptr` 为 NULL。 |

错误码（填充接口）：

| 值 | 场景 |
| --- | --- |
| 0 | 成功；`out` 持有 `len` 个新建字符串。 |
| -1 | `ptr` 为 NULL，或 `len > 0` 时 `out` 为 NULL。 |
| -8 | `len` 与符号数量不符。 |

### knowledge_rule_condition_symbols_len(ptr) / knowledge_rule_condition_symbols(ptr, out, len)

```c
uintptr_t knowledge_rule_condition_symbols_len(const struct CKnowledgeRule *ptr);
int32_t knowledge_rule_condition_symbols(const struct CKnowledgeRule *ptr,
                                         char **out,
                                         uintptr_t len);
```

两步式读取规则参数条件引用的不同符号：`*_len` 返回符号总数；填充接口把符号（已排序、去重）以新建 C 字符串形式拷入 `out`，返回 0。每个字符串须用 `cqlib_string_free` 释放。

参数：

- `ptr` (`const CKnowledgeRule*`)：规则句柄。
- `out` (`char**`)：接收符号字符串的缓冲区。
- `len` (`uintptr_t`)：缓冲区长度，必须等于 `knowledge_rule_condition_symbols_len` 返回的数量。

返回值（`*_len`）：

| 返回 | 场景 |
| --- | --- |
| 非负整数 | 符号总数。 |
| 0 | `ptr` 为 NULL。 |

错误码（填充接口）：

| 值 | 场景 |
| --- | --- |
| 0 | 成功；`out` 持有 `len` 个新建字符串。 |
| -1 | `ptr` 为 NULL，或 `len > 0` 时 `out` 为 NULL。 |
| -8 | `len` 与符号数量不符。 |

### knowledge_rule_has_conditions(ptr)

```c
int32_t knowledge_rule_has_conditions(const struct CKnowledgeRule *ptr);
```

判断规则是否带非空符号参数条件。

返回值：

| 值 | 场景 |
| --- | --- |
| 1 | 带非空参数条件。 |
| 0 | 不带参数条件。 |
| -1 | `ptr` 为 NULL。 |

### knowledge_rule_validate(ptr)

```c
int32_t knowledge_rule_validate(const struct CKnowledgeRule *ptr);
```

运行规则的结构校验。

错误码：

| 值 | 场景 |
| --- | --- |
| 0 | 规则结构合法。 |
| -1 | `ptr` 为 NULL。 |
| -4 | 校验失败。 |

---

## 匹配引擎

匹配引擎用于手动驱动一条规则：把模式匹配到具体操作得到 `CKnowledgeBindings*`，在绑定上评估参数条件，再把替换目标实例化为 `CKnowledgeReplacements*`。操作以 `operation_new` 创建的 `COperation*` 句柄传入（见 [Operation / Instruction（操作与指令）](../0_circuit/4_operation_instruction.md)）。

### knowledge_bindings_new()

```c
struct CKnowledgeBindings *knowledge_bindings_new(void);
```

创建空的匹配绑定记录，供 `knowledge_match_rule_item` 使用。

返回值：

| 返回 | 场景 |
| --- | --- |
| 非 NULL | 新建的绑定句柄，用 `knowledge_bindings_free` 释放。 |

### knowledge_bindings_free(ptr)

```c
void knowledge_bindings_free(struct CKnowledgeBindings *ptr);
```

释放匹配绑定，允许传 NULL。

### knowledge_match_rule_item(ptr, item_index, op, bindings)

```c
int32_t knowledge_match_rule_item(const struct CKnowledgeRule *ptr,
                                  uintptr_t item_index,
                                  const struct COperation *op,
                                  struct CKnowledgeBindings *bindings);
```

把规则匹配模式下标 `item_index` 处的条目匹配到具体操作，并把绑定的比特与参数记录进 `bindings`。匹配是事务性的：条目不匹配时 `bindings` 保持不变，失败的尝试不会破坏先前已绑定的内容。

参数：

- `ptr` (`const CKnowledgeRule*`)：规则句柄。
- `item_index` (`uintptr_t`)：匹配模式中的条目下标。
- `op` (`const COperation*`)：待匹配操作。
- `bindings` (`CKnowledgeBindings*`)：成功时被更新的绑定记录。

错误码：

| 值 | 场景 |
| --- | --- |
| 1 | 条目匹配；`bindings` 已记录绑定的比特与参数。 |
| 0 | 条目不匹配；`bindings` 不变。 |
| -1 | `ptr`、`op` 或 `bindings` 为 NULL。 |
| -2 | `item_index` 越界。 |
| -8 | 操作携带索引式（表解析）参数。 |
| -6 | 规则条目使用了匹配器无法表示的指令。 |

### knowledge_rule_matches_operations(ptr, operations, operations_len, out)

```c
int32_t knowledge_rule_matches_operations(const struct CKnowledgeRule *ptr,
                                          const struct COperation *const *operations,
                                          uintptr_t operations_len,
                                          struct CKnowledgeBindings **out);
```

把规则的完整匹配模式匹配到 `operations_len` 个相邻操作。成功时把新建的 `CKnowledgeBindings*` 写入 `*out` 并返回 1。参数条件不在此处评估；需用 `knowledge_conditions_hold` 在返回的绑定上单独检查。

参数：

- `ptr` (`const CKnowledgeRule*`)：规则句柄。
- `operations` (`const COperation* const*`)：相邻操作数组。
- `operations_len` (`uintptr_t`)：操作数量。
- `out` (`struct CKnowledgeBindings**`)：输出指针，成功时接收绑定句柄。

错误码：

| 值 | 场景 |
| --- | --- |
| 1 | 模式匹配；新建的 `CKnowledgeBindings*`（用 `knowledge_bindings_free` 释放）已写入 `*out`。 |
| 0 | 模式不匹配；`out` 保持不变。 |
| -1 | `ptr`、`operations` 或 `out` 为 NULL。 |
| -8 | `operations_len` 与模式长度不一致，或某操作携带索引式参数。 |

### knowledge_conditions_hold(ptr, bindings)

```c
int32_t knowledge_conditions_hold(const struct CKnowledgeRule *ptr,
                                  const struct CKnowledgeBindings *bindings);
```

在绑定下评估规则的符号参数条件。

返回值：

| 值 | 场景 |
| --- | --- |
| 1 | 全部参数条件成立。 |
| 0 | 至少一个条件不成立。 |
| -1 | `ptr` 或 `bindings` 为 NULL。 |

### knowledge_equivalent_to(lhs, lhs_item, rhs, rhs_item)

```c
int32_t knowledge_equivalent_to(const struct CKnowledgeRule *lhs,
                                uintptr_t lhs_item,
                                const struct CKnowledgeRule *rhs,
                                uintptr_t rhs_item);
```

判断 `lhs` 匹配模式下标 `lhs_item` 处的条目与 `rhs` 下标 `rhs_item` 处的条目是否等价。

参数：

- `lhs` (`const CKnowledgeRule*`)：第一条规则句柄。
- `lhs_item` (`uintptr_t`)：第一条规则模式中的条目下标。
- `rhs` (`const CKnowledgeRule*`)：第二条规则句柄。
- `rhs_item` (`uintptr_t`)：第二条规则模式中的条目下标。

返回值：

| 值 | 场景 |
| --- | --- |
| 1 | 两个条目等价。 |
| 0 | 条目不等价。 |
| -1 | `lhs` 或 `rhs` 为 NULL。 |
| -2 | 条目下标越界。 |

### knowledge_candidates_for_first_instruction_len(ptr, name) / knowledge_candidates_for_first_instruction(ptr, name, out, len)

```c
uintptr_t knowledge_candidates_for_first_instruction_len(const struct CKnowledgeLibrary *ptr,
                                                         const char *name);
uintptr_t knowledge_candidates_for_first_instruction(const struct CKnowledgeLibrary *ptr,
                                                     const char *name,
                                                     uint32_t *out,
                                                     uintptr_t len);
```

两步式读取首条匹配指令与门 `name` 具有相同匹配器键的规则 id 列表：`*_len` 返回候选规则总数；填充接口把 id 拷入 `out`（实际拷贝 `min(总数, len)` 个），返回总数。`out` 为 NULL 时只返回总数、不拷贝。

参数：

- `ptr` (`const CKnowledgeLibrary*`)：规则库句柄。
- `name` (`const char*`)：门名。
- `out` (`uint32_t*`)：接收 id 的缓冲区。
- `len` (`uintptr_t`)：缓冲区长度。

返回值（两者一致）：

| 返回 | 场景 |
| --- | --- |
| 非负整数 | 候选规则总数。 |
| 0 | `ptr` 或 `name` 为 NULL，或 `name` 为未知门名。 |

### knowledge_collect_free_symbols_len(ptr) / knowledge_collect_free_symbols(ptr, out, len)

```c
uintptr_t knowledge_collect_free_symbols_len(const struct CKnowledgeRule *ptr);
uintptr_t knowledge_collect_free_symbols(const struct CKnowledgeRule *ptr,
                                         char **out,
                                         uintptr_t len);
```

两步式读取规则的自由符号（已排序）：`*_len` 返回符号总数；填充接口把符号以新建 C 字符串形式拷入 `out`（实际拷贝 `min(总数, len)` 个），返回总数。每个字符串须用 `cqlib_string_free` 释放。

参数：

- `ptr` (`const CKnowledgeRule*`)：规则句柄。
- `out` (`char**`)：接收符号字符串的缓冲区。
- `len` (`uintptr_t`)：缓冲区长度。

返回值（两者一致）：

| 返回 | 场景 |
| --- | --- |
| 非负整数 | 自由符号总数。 |
| 0 | `ptr` 为 NULL。 |

### knowledge_filter_rule_ids_by_instruction_keys_len(ptr, ops, ops_len, targets, targets_len) / knowledge_filter_rule_ids_by_instruction_keys(ptr, ops, ops_len, targets, targets_len, out, len)

```c
uintptr_t knowledge_filter_rule_ids_by_instruction_keys_len(const struct CKnowledgeLibrary *ptr,
                                                            const char *const *ops,
                                                            uintptr_t ops_len,
                                                            const char *const *targets,
                                                            uintptr_t targets_len);
uintptr_t knowledge_filter_rule_ids_by_instruction_keys(const struct CKnowledgeLibrary *ptr,
                                                        const char *const *ops,
                                                        uintptr_t ops_len,
                                                        const char *const *targets,
                                                        uintptr_t targets_len,
                                                        uint32_t *out,
                                                        uintptr_t len);
```

两步式读取匹配侧覆盖 `ops` 中每个门、且替换目标侧覆盖 `targets` 中每个门（均为门名数组）的规则 id 列表：`*_len` 返回符合条件且覆盖的规则总数；填充接口把 id 拷入 `out`（实际拷贝 `min(总数, len)` 个），返回总数。`out` 为 NULL 时只返回总数、不拷贝。

参数：

- `ptr` (`const CKnowledgeLibrary*`)：规则库句柄。
- `ops` (`const char* const*`)：匹配侧门名数组。
- `ops_len` (`uintptr_t`)：匹配侧门名数量。
- `targets` (`const char* const*`)：替换目标侧门名数组。
- `targets_len` (`uintptr_t`)：替换目标侧门名数量。
- `out` (`uint32_t*`)：接收 id 的缓冲区。
- `len` (`uintptr_t`)：缓冲区长度。

返回值（两者一致）：

| 返回 | 场景 |
| --- | --- |
| 非负整数 | 符合条件且覆盖的规则总数。 |
| 0 | `ptr` 为 NULL，或某门名为 NULL 或未知门名。 |

### knowledge_extend_rules(ptr, rules, rules_len, kind, out, out_len)

```c
int32_t knowledge_extend_rules(struct CKnowledgeLibrary *ptr,
                               const struct CKnowledgeRule *const *rules,
                               uintptr_t rules_len,
                               uint8_t kind,
                               uint32_t *out,
                               uintptr_t out_len);
```

把若干解析好的规则原子地插入库并登记到类别 `kind`，把分配的规则 id 写入 `out`（`out` 必须能容纳 `rules_len` 个条目）。任一规则失败时，库保持不变。

参数：

- `ptr` (`CKnowledgeLibrary*`)：目标规则库。
- `rules` (`const CKnowledgeRule* const*`)：解析好的规则句柄数组（克隆使用，句柄不被消耗）。
- `rules_len` (`uintptr_t`)：规则数量。
- `kind` (`uint8_t`)：规则类别标签，取 `RULE_KIND_*` 之一。
- `out` (`uint32_t*`)：接收分配规则 id 的缓冲区。
- `out_len` (`uintptr_t`)：缓冲区长度，至少为 `rules_len`。

错误码：

| 值 | 场景 |
| --- | --- |
| 0 | 成功；`out` 持有分配的 id。 |
| -1 | `ptr`、`rules` 或 `out` 为 NULL。 |
| -8 | `kind` 不是合法的 `RULE_KIND_*` 标签，或 `out_len` 小于 `rules_len`。 |
| -4 | 结构验证失败或规则名重复；库保持不变。 |

### knowledge_target_qubits_len(ptr) / knowledge_target_qubits(ptr, out, len)

```c
uintptr_t knowledge_target_qubits_len(const struct CKnowledgeRule *ptr);
uintptr_t knowledge_target_qubits(const struct CKnowledgeRule *ptr, uint32_t *out, uintptr_t len);
```

两步式读取规则替换目标使用的规则局部比特标签（已排序）：`*_len` 返回标签总数；填充接口把标签拷入 `out`（实际拷贝 `min(总数, len)` 个），返回总数。`out` 为 NULL 时只返回总数、不拷贝。

参数：

- `ptr` (`const CKnowledgeRule*`)：规则句柄。
- `out` (`uint32_t*`)：接收标签的缓冲区。
- `len` (`uintptr_t`)：缓冲区长度。

返回值（两者一致）：

| 返回 | 场景 |
| --- | --- |
| 非负整数 | 标签总数。 |
| 0 | `ptr` 为 NULL。 |

### knowledge_instantiate_target(ptr, bindings, out)

```c
int32_t knowledge_instantiate_target(const struct CKnowledgeRule *ptr,
                                     const struct CKnowledgeBindings *bindings,
                                     struct CKnowledgeReplacements **out);
```

在匹配绑定下实例化规则的替换目标，用绑定的值评估目标中的每个参数表达式。

参数：

- `ptr` (`const CKnowledgeRule*`)：规则句柄。
- `bindings` (`const CKnowledgeBindings*`)：匹配产生的绑定。
- `out` (`struct CKnowledgeReplacements**`)：输出指针，成功时接收实例化结果。

错误码：

| 值 | 场景 |
| --- | --- |
| 0 | 成功；新建的 `CKnowledgeReplacements*`（用 `knowledge_replacements_free` 释放）已写入 `*out`。 |
| -1 | `ptr`、`bindings` 或 `out` 为 NULL。 |
| -2 | 替换目标引用了匹配未绑定的比特。 |
| -8 | 替换目标引用了匹配未绑定的符号。 |
| -6 | 替换目标使用了匹配器无法表示的指令。 |

### knowledge_replacements_len(ptr)

```c
uintptr_t knowledge_replacements_len(const struct CKnowledgeReplacements *ptr);
```

返回实例化替换条目的数量。

返回值：

| 返回 | 场景 |
| --- | --- |
| 非负整数 | 替换条目数。 |
| 0 | `ptr` 为 NULL。 |

### knowledge_replacement_instruction(ptr, index)

```c
char *knowledge_replacement_instruction(const struct CKnowledgeReplacements *ptr, uintptr_t index);
```

返回下标 `index` 处替换条目的指令名（如 `"RZ"`）。

参数：

- `ptr` (`const CKnowledgeReplacements*`)：实例化结果句柄。
- `index` (`uintptr_t`)：替换条目下标。

返回值：

| 返回 | 场景 |
| --- | --- |
| 非 NULL | 堆上 C 字符串，用 `cqlib_string_free` 释放。 |
| NULL | `ptr` 为 NULL 或 `index` 越界。 |

### knowledge_replacement_qubits_len(ptr, index) / knowledge_replacement_qubits(ptr, index, out, len)

```c
uintptr_t knowledge_replacement_qubits_len(const struct CKnowledgeReplacements *ptr,
                                           uintptr_t index);
uintptr_t knowledge_replacement_qubits(const struct CKnowledgeReplacements *ptr,
                                       uintptr_t index,
                                       uint32_t *out,
                                       uintptr_t len);
```

两步式读取下标 `index` 处替换条目的具体比特：`*_len` 返回比特总数；填充接口把比特拷入 `out`（实际拷贝 `min(总数, len)` 个），返回总数。`out` 为 NULL 时只返回总数、不拷贝。

参数：

- `ptr` (`const CKnowledgeReplacements*`)：实例化结果句柄。
- `index` (`uintptr_t`)：替换条目下标。
- `out` (`uint32_t*`)：接收比特的缓冲区。
- `len` (`uintptr_t`)：缓冲区长度。

返回值（两者一致）：

| 返回 | 场景 |
| --- | --- |
| 非负整数 | 比特总数。 |
| 0 | `ptr` 为 NULL 或 `index` 越界。 |

### knowledge_replacement_params_len(ptr, index) / knowledge_replacement_params(ptr, index, out, len)

```c
uintptr_t knowledge_replacement_params_len(const struct CKnowledgeReplacements *ptr,
                                           uintptr_t index);
uintptr_t knowledge_replacement_params(const struct CKnowledgeReplacements *ptr,
                                       uintptr_t index,
                                       double *out,
                                       uintptr_t len);
```

两步式读取下标 `index` 处替换条目的已求值 double 参数：`*_len` 返回参数总数；填充接口把参数拷入 `out`（实际拷贝 `min(总数, len)` 个），返回总数。`out` 为 NULL 时只返回总数、不拷贝。

参数：

- `ptr` (`const CKnowledgeReplacements*`)：实例化结果句柄。
- `index` (`uintptr_t`)：替换条目下标。
- `out` (`double*`)：接收已求值参数的缓冲区。
- `len` (`uintptr_t`)：缓冲区长度。

返回值（两者一致）：

| 返回 | 场景 |
| --- | --- |
| 非负整数 | 参数总数。 |
| 0 | `ptr` 为 NULL 或 `index` 越界。 |

### knowledge_replacements_free(ptr)

```c
void knowledge_replacements_free(struct CKnowledgeReplacements *ptr);
```

释放替换目标的实例化结果，允许传 NULL。

---

## 示例

内置库遍历与按类别查询：

```c
#include <stdio.h>
#include <stdlib.h>
#include "cqlib_c.h"

int main(void) {
    /* 1. Load the builtin rule library. */
    struct CKnowledgeLibrary *lib = knowledge_library_builtin();
    if (lib == NULL) {
        return 1;
    }

    /* 2. Walk the rule metadata by insertion index. */
    for (uintptr_t i = 0; i < knowledge_library_len(lib); i++) {
        char *name = knowledge_library_rule_name(lib, i);
        int32_t kind = knowledge_library_rule_kind(lib, i);
        uintptr_t pattern = knowledge_library_rule_pattern_len(lib, i);
        if (name != NULL) {
            printf("rule %s: kind=%d pattern_len=%llu\n",
                   name, kind, (unsigned long long)pattern);
            cqlib_string_free(name);
        }
    }

    /* 3. Two-step read of the decompose-rule ids. */
    uintptr_t n = knowledge_library_rules_by_kind_len(lib, RULE_KIND_DECOMPOSE);
    uint32_t *ids = malloc(n * sizeof(uint32_t));
    if (ids != NULL && n > 0) {
        uintptr_t written = knowledge_library_rules_by_kind(lib, RULE_KIND_DECOMPOSE, ids, n);
        printf("%llu decompose rules (first id %u)\n",
               (unsigned long long)written, ids[0]);
        free(ids);
    }

    knowledge_library_free(lib);
    return 0;
}
```

DSL 构造与查询：

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* 1. Build a library from DSL text (merge two adjacent RZ rotations). */
    const char *source =
        "rule merge_rz {\n"
        "    match {\n"
        "        RZ(a) 0\n"
        "        RZ(b) 0\n"
        "    }\n"
        "    rewrite {\n"
        "        RZ(a + b) 0\n"
        "    }\n"
        "}";
    struct CKnowledgeLibrary *lib = NULL;
    if (knowledge_library_from_dsl_str(source, RULE_KIND_MERGE, &lib) != 0) {
        return 1;
    }

    /* 2. Query by name. */
    if (knowledge_library_contains(lib, "merge_rz") == 1) {
        printf("id=%lld\n", (long long)knowledge_library_id_by_name(lib, "merge_rz"));
    }

    /* 3. Read the metadata of rule 0. */
    char *first = knowledge_library_rule_first_instruction(lib, 0);  /* "RZ" */
    printf("first=%s pattern=%llu rewrite=%llu cost_delta=%lld\n",
           first,
           (unsigned long long)knowledge_library_rule_pattern_len(lib, 0),
           (unsigned long long)knowledge_library_rule_rewrite_len(lib, 0),
           (long long)knowledge_library_rule_cost_delta(lib, 0));
    cqlib_string_free(first);

    knowledge_library_free(lib);
    return 0;
}
```

单条规则解析与插入：

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* 1. Parse exactly one rule: cancel two adjacent H gates. */
    const char *source =
        "rule cancel_h {\n"
        "    match {\n"
        "        H 0\n"
        "        H 0\n"
        "    }\n"
        "    rewrite {}\n"
        "}";
    struct CKnowledgeRule *rule = NULL;
    if (knowledge_rule_from_dsl(source, &rule) != 0) {
        return 1;
    }

    /* 2. Inspect and validate the standalone rule. */
    if (knowledge_rule_validate(rule) != 0) {
        knowledge_rule_free(rule);
        return 1;
    }
    char *name = knowledge_rule_name(rule);  /* "cancel_h" */
    printf("rule %s: qubits=%llu pattern=%llu rewrite=%llu conditions=%d\n",
           name,
           (unsigned long long)knowledge_rule_num_qubits(rule),
           (unsigned long long)knowledge_rule_pattern_len(rule),
           (unsigned long long)knowledge_rule_rewrite_len(rule),
           knowledge_rule_has_conditions(rule));
    cqlib_string_free(name);

    /* 3. Insert into a fresh library (assigns id 0), then reject duplicates. */
    struct CKnowledgeLibrary *lib = knowledge_library_new();
    if (knowledge_library_add_rule(lib, rule, RULE_KIND_CANCEL) != 0) {
        knowledge_rule_free(rule);
        knowledge_library_free(lib);
        return 1;
    }
    if (knowledge_library_add_rule(lib, rule, RULE_KIND_CANCEL) == -4) {
        printf("duplicate name rejected\n");
    }

    knowledge_rule_free(rule);
    knowledge_library_free(lib);
    return 0;
}
```

把规则匹配到具体操作并实例化其替换目标：

```c
#include <stdio.h>
#include "cqlib_c.h"

int main(void) {
    /* 1. 构造 merge_rz 规则。 */
    const char *source =
        "rule merge_rz {\n"
        "    match {\n"
        "        RZ(a) 0\n"
        "        RZ(b) 0\n"
        "    }\n"
        "    rewrite {\n"
        "        RZ(a + b) 0\n"
        "    }\n"
        "}";
    struct CKnowledgeRule *rule = NULL;
    if (knowledge_rule_from_dsl(source, &rule) != 0) {
        return 1;
    }

    /* 2. 构造两个相邻 RZ 操作。 */
    uint32_t q0[1] = {7};
    double p1[1] = {0.25};
    double p2[1] = {0.5};
    struct COperation *a = operation_new("RZ", q0, 1, p1, 1);
    struct COperation *b = operation_new("RZ", q0, 1, p2, 1);
    const struct COperation *ops[2] = {a, b};

    /* 3. 匹配完整模式（参数条件另行检查）。 */
    struct CKnowledgeBindings *bindings = NULL;
    if (knowledge_rule_matches_operations(rule, ops, 2, &bindings) == 1) {
        /* 4. 实例化替换目标：RZ(a + b)，其中 a + b == 0.75。 */
        struct CKnowledgeReplacements *repl = NULL;
        if (knowledge_instantiate_target(rule, bindings, &repl) == 0) {
            for (uintptr_t i = 0; i < knowledge_replacements_len(repl); i++) {
                char *name = knowledge_replacement_instruction(repl, i);  /* "RZ" */
                uint32_t qubits[1];
                double params[1];
                knowledge_replacement_qubits(repl, i, qubits, 1);       /* {7} */
                knowledge_replacement_params(repl, i, params, 1);       /* {0.75} */
                printf("repl %s q=%u p=%f\n", name, qubits[0], params[0]);
                cqlib_string_free(name);
            }
            knowledge_replacements_free(repl);
        }
        knowledge_bindings_free(bindings);
    }

    /* 5. 查看规则内部：自由符号与替换目标比特标签。 */
    uintptr_t nsym = knowledge_collect_free_symbols_len(rule);
    char **symbols = malloc(nsym * sizeof(char *));
    if (symbols != NULL && knowledge_collect_free_symbols(rule, symbols, nsym) == nsym) {
        for (uintptr_t i = 0; i < nsym; i++) {
            printf("symbol %s\n", symbols[i]);
            cqlib_string_free(symbols[i]);
        }
        free(symbols);
    }

    operation_free(a);
    operation_free(b);
    knowledge_rule_free(rule);
    return 0;
}
```

由规则库驱动的对易检查器见 [对易检查](9_commutation.md)。
