# Circuit Control-Flow Graph (CCircuitCFG)

`CCircuitCFG*` is the control-flow graph (CFG) analysis view of a quantum circuit: basic blocks hold sequences of operations that execute in order, block terminators describe control transfers, and control-flow regions record the boundaries and entries of structured `if` / `while` / `for` / `switch` constructs. Blocks are addressed by stable node indices (`uint32_t`), with `UINT32_MAX` as the "missing block / unset entry" sentinel; besides the common error codes, the interfaces on this page use `-2` for a missing block or a sentinel index. The typical workflow is `circuit_cfg_from_circuit` → inspect blocks / terminators / edges / regions → `circuit_cfg_validate` → `circuit_cfg_to_circuit`. For the common conventions (error codes, handle release, two-step array output) see [Overview](../0_overview.md).

---

## Construction and conversion

### circuit_cfg_new(num_qubits)

Creates an empty CFG with densely numbered qubits `0..num_qubits`. The fresh graph has no blocks and no entry block; it must be edited before it can pass `circuit_cfg_validate`.

- `num_qubits` (`uintptr_t`): number of logical qubits.

Returns: a newly allocated `CCircuitCFG*`.

### circuit_cfg_from_qubits(qubits, qubits_len)

Creates an empty CFG from the supplied logical qubit ids in insertion order; duplicate ids are removed, which suits sparse logical numbering.

- `qubits` (`const uint32_t*`): array of logical qubit ids.
- `qubits_len` (`uintptr_t`): length of the array.

Returns: a newly allocated `CCircuitCFG*` (free with `circuit_cfg_free`), or NULL when `qubits` is NULL while `qubits_len > 0`.

### circuit_cfg_from_circuit(ptr)

Expands a structured circuit into a validated CFG. Every conditional, loop, and switch header owns a control-flow region recording body entries, merge blocks, and outer operation metadata; a linear circuit expands into a single entry block labeled `entry` with a `CFG_TERMINATOR_RETURN` terminator.

- `ptr` (`const struct CCircuit*`): source circuit handle.

Returns: a newly allocated `CCircuitCFG*` (free with `circuit_cfg_free`), or NULL when `ptr` is NULL or the expansion fails (control flow that cannot be structured).

```c
struct CCircuit* circuit = circuit_new(2);
circuit_h(circuit, 0);
circuit_cx(circuit, 0, 1);

struct CCircuitCFG* cfg = circuit_cfg_from_circuit(circuit);
/* circuit_cfg_num_blocks(cfg) == 1: a linear circuit has a single entry block */
```

### circuit_cfg_free(ptr)

Frees a CFG handle.

- `ptr` (`struct CCircuitCFG*`): handle to free; passing NULL is allowed.

### circuit_cfg_to_circuit(ptr)

Reconstructs a structured circuit from this CFG. The symbol table, the parameter table, and the global phase are restored along with it; a structure check identical to `circuit_cfg_validate` runs first.

- `ptr` (`const struct CCircuitCFG*`): CFG handle.

Returns: a newly allocated `CCircuit*` (free with `circuit_free`), or NULL when `ptr` is NULL or the lowering fails (the graph is damaged, or a region cannot be mapped back to a structured form).

### circuit_cfg_validate(ptr)

Checks the graph, operation, parameter, edge, terminator, and region invariants: whether the entry block is valid, whether every block has exactly one terminator whose outgoing edge count and types agree, whether `Return` has no outgoing edge, whether all blocks are reachable from the entry, whether the region metadata agrees with the graph, whether the `Branch` condition is a `Bool` expression, whether the `Switch` target is a `UInt` expression with outgoing edges equal to the case count plus one, and more.

- `ptr` (`const struct CCircuitCFG*`): CFG handle.

Returns: `0` on success; `-1` when `ptr` is NULL; `-3` on validation failure.

---

## Global information

### circuit_cfg_num_qubits(ptr)

Returns the number of logical qubits.

- `ptr` (`const struct CCircuitCFG*`): CFG handle.

Returns: the qubit count, or 0 when `ptr` is NULL.

### circuit_cfg_num_blocks(ptr)

Returns the number of blocks currently stored in the graph.

- `ptr` (`const struct CCircuitCFG*`): CFG handle.

Returns: the block count, or 0 when `ptr` is NULL.

### circuit_cfg_qubits(ptr, buffer, len)

Copies the logical qubit ids in insertion order into `buffer` (two-step: query the length with `circuit_cfg_num_qubits` first).

- `ptr` (`const struct CCircuitCFG*`): CFG handle.
- `buffer` (`uint32_t*`): output buffer.
- `len` (`uintptr_t`): buffer capacity.

Returns: `0` on success; `-1` when `ptr` is NULL, or when `buffer` is NULL while the id set is non-empty; `-8` when `len` is smaller than the qubit count.

### circuit_cfg_classical_vars_len(ptr)

Returns the length of the static type table of mutable classical variables.

- `ptr` (`const struct CCircuitCFG*`): CFG handle.

Returns: the variable count, or 0 when `ptr` is NULL.

### circuit_cfg_classical_vars(ptr, buffer, len)

Copies the mutable classical variable types as `(tag, width)` snapshots into `buffer` (two-step: query the length with `circuit_cfg_classical_vars_len`). `tag` is one of `CQLIB_CLASSICAL_TYPE_BIT` / `CQLIB_CLASSICAL_TYPE_BOOL` / `CQLIB_CLASSICAL_TYPE_UINT` / `CQLIB_CLASSICAL_TYPE_BIT_VEC`; the table is empty for a hand-built CFG and holds the source circuit's static snapshot for one built by `circuit_cfg_from_circuit`.

- `ptr` (`const struct CCircuitCFG*`): CFG handle.
- `buffer` (`struct CClassicalType*`): output array whose elements are `{ uint32_t tag; uint32_t width; }`.
- `len` (`uintptr_t`): buffer capacity.

Returns: `0` on success; `-1` when `ptr` is NULL, or when `buffer` is NULL while the table is non-empty; `-8` when `len` is too small.

### circuit_cfg_classical_values_len(ptr)

Returns the length of the static type table of immutable classical values.

- `ptr` (`const struct CCircuitCFG*`): CFG handle.

Returns: the value count, or 0 when `ptr` is NULL.

### circuit_cfg_classical_values(ptr, buffer, len)

Copies the immutable classical value types (such as the `BitVec` produced by a measurement) as `(tag, width)` snapshots into `buffer`, with the same conventions as `circuit_cfg_classical_vars`.

- `ptr` (`const struct CCircuitCFG*`): CFG handle.
- `buffer` (`struct CClassicalType*`): output array.
- `len` (`uintptr_t`): buffer capacity.

Returns: `0` on success; `-1` when `ptr` is NULL, or when `buffer` is NULL while the table is non-empty; `-8` when `len` is too small.

### circuit_cfg_entry_block(ptr)

Returns the designated entry block of the graph.

- `ptr` (`const struct CCircuitCFG*`): CFG handle.

Returns: the entry block index, or `UINT32_MAX` when none has been set or `ptr` is NULL.

---

## Blocks and operations

### circuit_cfg_add_block(ptr)

Appends an empty block to the graph.

- `ptr` (`struct CCircuitCFG*`): CFG handle.

Returns: the new block's stable node index, or `UINT32_MAX` when `ptr` is NULL.

### circuit_cfg_push_operation(ptr, node, op)

Appends one ordinary (non-control-flow) operation to the end of a block; the operation is cloned and the source handle stays owned by the caller.

- `ptr` (`struct CCircuitCFG*`): CFG handle.
- `node` (`uint32_t`): block index.
- `op` (`const struct COperation*`): operation handle (created with `operation_new`, see [Operations and Instructions](4_operation_instruction.md)).

Returns: `0` on success; `-1` when `ptr` or `op` is NULL; `-2` when the block does not exist.

### circuit_cfg_extend_operations(ptr, node, ops, len)

Appends a batch of cloned operations to the end of block `node`. Every handle in `ops` is validated and cloned before any mutation, so the block is left untouched when any entry is NULL.

- `ptr` (`struct CCircuitCFG*`): CFG handle.
- `node` (`uint32_t`): block index.
- `ops` (`const struct COperation* const*`): array of operation handles.
- `len` (`uintptr_t`): number of operations (may be 0).

Returns: `0` on success; `-1` on NULL input; `-2` when `node` does not address a block.

### circuit_cfg_blocks(ptr, buffer, len)

Copies the stable node indices of all blocks in graph order into `buffer` (two-step: query the count with `circuit_cfg_num_blocks` first).

- `ptr` (`const struct CCircuitCFG*`): CFG handle.
- `buffer` (`uint32_t*`): output buffer.
- `len` (`uintptr_t`): buffer capacity.

Returns: `0` on success; `-1` when `ptr` is NULL, or when `buffer` is NULL while the graph is non-empty; `-8` when `len` is smaller than the block count.

### circuit_cfg_block_label(ptr, node)

Reads the block's diagnostic label. Blocks produced by `circuit_cfg_from_circuit` carry structured labels (such as `entry`, `while_body_…`).

- `ptr` (`const struct CCircuitCFG*`): CFG handle.
- `node` (`uint32_t`): block index.

Returns: a newly allocated C string (free with `cqlib_string_free`), or NULL when the block is unlabeled, does not exist, or `ptr` is NULL.

### circuit_cfg_with_label(ptr, node, label)

Sets or replaces the label of block `node`.

- `ptr` (`struct CCircuitCFG*`): CFG handle.
- `node` (`uint32_t`): block index.
- `label` (`const char*`): new label string.

Returns: `0` on success; `-1` on NULL input; `-2` when `node` does not address a block; `-4` when `label` is not valid UTF-8.

### circuit_cfg_block_is_empty(ptr, node)

Returns whether the block has neither operations nor a terminator.

- `ptr` (`const struct CCircuitCFG*`): CFG handle.
- `node` (`uint32_t`): block index.

Returns: `1` when empty; `0` otherwise; `-1` when `ptr` is NULL; `-2` when the block does not exist.

### circuit_cfg_block_operations_len(ptr, node)

Returns the number of ordinary operations in the block (the terminator excluded).

- `ptr` (`const struct CCircuitCFG*`): CFG handle.
- `node` (`uint32_t`): block index.

Returns: the operation count, or 0 when `ptr` is NULL or the block does not exist.

### circuit_cfg_block_operations(ptr, node, out, len)

Clones the block's ordinary operations into `out` (two-step: query the count with `circuit_cfg_block_operations_len`). Each written element is a freshly allocated `COperation*` freed separately with `operation_free`.

- `ptr` (`const struct CCircuitCFG*`): CFG handle.
- `node` (`uint32_t`): block index.
- `out` (`struct COperation**`): output array.
- `len` (`uintptr_t`): array capacity.

Returns: `0` on success; `-1` when `ptr` is NULL, or when `out` is NULL while the operations are non-empty; `-2` when the block does not exist; `-8` when `len` is too small.

---

## Terminators

In a valid CFG every basic block must carry exactly one terminator, whose outgoing edge count and types agree with it; `Return` allows no outgoing edge. `Branch` / `ForLoop` / `Switch` terminators are produced by `circuit_cfg_from_circuit` when expanding structured circuits built with `circuit_if_else` / `circuit_for_uint` / `circuit_switch` and friends — see [Classical Data and Control Flow](9_classical_control_flow.md); `Jump` / `Return` / `Break` / `Continue` can be replaced through the four setters below.

### circuit_cfg_block_has_terminator(ptr, node)

Returns whether a terminator has been assigned to the block.

- `ptr` (`const struct CCircuitCFG*`): CFG handle.
- `node` (`uint32_t`): block index.

Returns: `1` when assigned; `0` when not; `-1` when `ptr` is NULL; `-2` when the block does not exist.

### circuit_cfg_block_terminator_tag(ptr, node)

Returns the block's terminator tag.

- `ptr` (`const struct CCircuitCFG*`): CFG handle.
- `node` (`uint32_t`): block index.

Returns: one of the `CFG_TERMINATOR_*` constants; `0` when no terminator is set; `-1` when `ptr` is NULL; `-2` when the block does not exist.

### circuit_cfg_terminator_target(ptr, node)

Returns the jump target of a `Jump`, `Break`, or `Continue` terminator.

- `ptr` (`const struct CCircuitCFG*`): CFG handle.
- `node` (`uint32_t`): block index.

Returns: the target block index, or `UINT32_MAX` for other terminators, missing terminators, unknown blocks, or a NULL handle.

### circuit_cfg_terminator_condition(ptr, node)

Clones the condition of a `Branch` or `Switch` terminator.

- `ptr` (`const struct CCircuitCFG*`): CFG handle.
- `node` (`uint32_t`): block index.

Returns: a newly allocated `CClassicalExpr*` (free with `classical_expr_free`), or NULL on a NULL handle, an unknown block, a missing terminator, or a non-branching terminator.

### circuit_cfg_terminator_for_info(ptr, node, var, start, stop, step)

Writes cloned handles describing a `ForLoop` terminator: the loop variable plus the start, stop, and step expressions (half-open range `[start, stop)`). All four handles are clones: release `var` with `classical_var_free` and the expressions with `classical_expr_free`.

- `ptr` (`const struct CCircuitCFG*`): CFG handle.
- `node` (`uint32_t`): block index.
- `var` (`struct CClassicalVar**`): loop variable output.
- `start` (`struct CClassicalExpr**`): start expression output.
- `stop` (`struct CClassicalExpr**`): stop expression output.
- `step` (`struct CClassicalExpr**`): step expression output.

Returns: `0` on success; `-1` when `ptr` is NULL or any output pointer is NULL; `-2` when the block does not exist; `-8` when the terminator is not a `ForLoop`.

### circuit_cfg_set_terminator_jump(ptr, node, target)

Replaces the block terminator with an unconditional jump to `target`.

- `ptr` (`struct CCircuitCFG*`): CFG handle.
- `node` (`uint32_t`): block index.
- `target` (`uint32_t`): target block index.

Returns: `0` on success; `-1` when `ptr` is NULL; `-2` when the block does not exist.

### circuit_cfg_set_terminator_return(ptr, node)

Replaces the block terminator with `Return` (end of execution).

- `ptr` (`struct CCircuitCFG*`): CFG handle.
- `node` (`uint32_t`): block index.

Returns: `0` on success; `-1` when `ptr` is NULL; `-2` when the block does not exist.

### circuit_cfg_set_terminator_break(ptr, node, target)

Replaces the block terminator with a structured `Break` referencing the header of the loop or `switch` being broken out of.

- `ptr` (`struct CCircuitCFG*`): CFG handle.
- `node` (`uint32_t`): block index.
- `target` (`uint32_t`): header block index.

Returns: `0` on success; `-1` when `ptr` is NULL; `-2` when the block does not exist.

### circuit_cfg_set_terminator_continue(ptr, node, target)

Replaces the block terminator with a structured `Continue` referencing the loop header.

- `ptr` (`struct CCircuitCFG*`): CFG handle.
- `node` (`uint32_t`): block index.
- `target` (`uint32_t`): loop header block index.

Returns: `0` on success; `-1` when `ptr` is NULL; `-2` when the block does not exist.

---

## Edges and traversal

### circuit_cfg_add_edge(ptr, source, target, flow, case_lo, case_hi)

Adds a directed semantic edge between two existing blocks.

- `ptr` (`struct CCircuitCFG*`): CFG handle.
- `source` (`uint32_t`): source block index.
- `target` (`uint32_t`): target block index.
- `flow` (`uint32_t`): one of the `CFG_FLOW_*` tags.
- `case_lo` (`uint64_t`): least-significant 64 bits of the 128-bit case value for `CFG_FLOW_CASE`; pass 0 for other tags.
- `case_hi` (`uint64_t`): most-significant 64 bits of the case value.

Returns: `0` on success; `-1` when `ptr` is NULL; `-2` when either endpoint does not exist (including the sentinel); `-8` on an unknown `flow` tag.

### circuit_cfg_set_entry_block(ptr, node)

Sets the graph entry block; whether the index points at a real block is checked by `circuit_cfg_validate`.

- `ptr` (`struct CCircuitCFG*`): CFG handle.
- `node` (`uint32_t`): entry block index.

Returns: `0` on success; `-1` when `ptr` is NULL; `-2` when `node` is the `UINT32_MAX` sentinel.

### circuit_cfg_outgoing_edges_len(ptr, node)

Returns the number of outgoing edges of a block.

- `ptr` (`const struct CCircuitCFG*`): CFG handle.
- `node` (`uint32_t`): block index.

Returns: the edge count, or 0 when `ptr` is NULL or the block does not exist.

### circuit_cfg_outgoing_edges(ptr, node, out_targets, flow_tags, case_lo, case_hi, len)

Copies the outgoing edges of a block into four parallel arrays (two-step: query the count with `circuit_cfg_outgoing_edges_len`). Edges are written in insertion order, with the true branch first for conditional headers.

- `ptr` (`const struct CCircuitCFG*`): CFG handle.
- `node` (`uint32_t`): block index.
- `out_targets` (`uint32_t*`): array of target block indices.
- `flow_tags` (`uint32_t*`): array of `CFG_FLOW_*` tags.
- `case_lo` (`uint64_t*`): least-significant 64 bits of the 128-bit case value for `CFG_FLOW_CASE` edges; zeros otherwise.
- `case_hi` (`uint64_t*`): array of most-significant case value halves.
- `len` (`uintptr_t`): array capacity.

Returns: `0` on success; `-1` when `ptr` is NULL, or when any array is NULL while the edges are non-empty; `-2` when the block does not exist; `-8` when `len` is too small.

---

## Regions and loops

Control-flow regions record the boundaries and semantics of structured control flow: graph edges alone cannot fully recover the structured meaning of `if` / `while` / `for` / `switch`, so region metadata describes each construct's branch entries, exit blocks, and merge blocks. Keep the region metadata in sync whenever the graph is edited — keeping regions consistent with the graph edges is the key to a successful `circuit_cfg_to_circuit` reconstruction.

### circuit_cfg_region_tag(ptr, node)

Returns the control-flow region tag owned by a block.

- `ptr` (`const struct CCircuitCFG*`): CFG handle.
- `node` (`uint32_t`): block index.

Returns: one of the `CFG_REGION_*` constants; `0` when the block owns no region; `-1` when `ptr` is NULL; `-2` when the block does not exist.

### circuit_cfg_is_loop_header(ptr, node)

Returns whether the block is a loop header: a block owning a `While` or `For` region returns `1`; a `Switch` region entry does not count as a loop header.

- `ptr` (`const struct CCircuitCFG*`): CFG handle.
- `node` (`uint32_t`): block index.

Returns: `1` when yes; `0` when no; `-1` when `ptr` is NULL; `-2` when the block does not exist.

### circuit_cfg_region_if(ptr, node, then_entry, else_entry, merge_block, has_else)

Writes the `If` region layout.

- `ptr` (`const struct CCircuitCFG*`): CFG handle.
- `node` (`uint32_t`): header block index.
- `then_entry` (`uint32_t*`): entry block of the then branch.
- `else_entry` (`uint32_t*`): entry block of the else branch; the merge block when no else body exists.
- `merge_block` (`uint32_t*`): merge block.
- `has_else` (`int32_t*`): whether the source circuit has an else body (1/0).

Returns: `0` on success; `-1` when `ptr` is NULL or any output pointer is NULL; `-2` when the block does not exist; `-8` when the block owns no `If` region.

### circuit_cfg_region_loop(ptr, node, body_entry, exit_block)

Writes the `While` / `For` region layout.

- `ptr` (`const struct CCircuitCFG*`): CFG handle.
- `node` (`uint32_t`): header block index.
- `body_entry` (`uint32_t*`): loop body entry block.
- `exit_block` (`uint32_t*`): exit block.

Returns: `0` on success; `-1` when `ptr` is NULL or an output pointer is NULL; `-2` when the block does not exist; `-8` when the block owns no loop region.

### circuit_cfg_region_switch_cases_len(ptr, node)

Returns the number of explicit cases of a `Switch` region.

- `ptr` (`const struct CCircuitCFG*`): CFG handle.
- `node` (`uint32_t`): header block index.

Returns: the case count, or 0 when `ptr` is NULL, the block does not exist, or the region is not a `Switch`.

### circuit_cfg_region_switch(ptr, node, case_lo, case_hi, case_entries, len, default_entry, merge_block, has_default)

Writes the `Switch` region layout (two-step: query the case count with `circuit_cfg_region_switch_cases_len` first).

- `ptr` (`const struct CCircuitCFG*`): CFG handle.
- `node` (`uint32_t`): header block index.
- `case_lo` (`uint64_t*`): least-significant 64 bits of case `i`'s 128-bit match value.
- `case_hi` (`uint64_t*`): most-significant 64 bits of case `i`'s match value.
- `case_entries` (`uint32_t*`): entry block of case `i`'s branch.
- `len` (`uintptr_t`): capacity of the case arrays.
- `default_entry` (`uint32_t*`): default branch entry block; the merge block when no default body exists.
- `merge_block` (`uint32_t*`): merge block.
- `has_default` (`int32_t*`): whether the source circuit has a default body (1/0).

Returns: `0` on success; `-1` when `ptr` is NULL or a required output pointer is NULL; `-2` when the block does not exist; `-8` when the block owns no `Switch` region or `len` is smaller than the case count.

### circuit_cfg_set_if_region(ptr, node, then_entry, else_entry, merge_block, has_else, outer_op)

Attaches an `If` region to a header block. Whether the entry indices point at real blocks is checked by `circuit_cfg_validate`.

- `ptr` (`struct CCircuitCFG*`): CFG handle.
- `node` (`uint32_t`): header block index.
- `then_entry` (`uint32_t`): entry block of the then branch.
- `else_entry` (`uint32_t`): entry block of the else branch.
- `merge_block` (`uint32_t`): merge block.
- `has_else` (`int32_t`): whether an else body exists (1/0).
- `outer_op` (`const struct COperation*`): outer operation metadata (acted qubits, parameters, label); pass NULL for empty metadata.

Returns: `0` on success; `-1` when `ptr` is NULL; `-2` when the header block does not exist.

### circuit_cfg_set_while_region(ptr, node, body_entry, exit_block, outer_op)

Attaches a `While` region to a header block.

- `ptr` (`struct CCircuitCFG*`): CFG handle.
- `node` (`uint32_t`): header block index.
- `body_entry` (`uint32_t`): loop body entry block.
- `exit_block` (`uint32_t`): exit block.
- `outer_op` (`const struct COperation*`): outer operation metadata; may be NULL.

Returns: `0` on success; `-1` when `ptr` is NULL; `-2` when the header block does not exist.

### circuit_cfg_set_for_region(ptr, node, body_entry, exit_block, outer_op)

Attaches a `For` region to a header block; the parameters mean the same as in `circuit_cfg_set_while_region`.

- `ptr` (`struct CCircuitCFG*`): CFG handle.
- `node` (`uint32_t`): header block index.
- `body_entry` (`uint32_t`): loop body entry block.
- `exit_block` (`uint32_t`): exit block.
- `outer_op` (`const struct COperation*`): outer operation metadata; may be NULL.

Returns: `0` on success; `-1` when `ptr` is NULL; `-2` when the header block does not exist.

### circuit_cfg_set_switch_region(ptr, node, case_lo, case_hi, case_entries, len, default_entry, merge_block, has_default, outer_op)

Attaches a `Switch` region to a header block: case `i`'s 128-bit match value is assembled from `case_lo[i]` (least-significant 64 bits) and `case_hi[i]` (most-significant 64 bits), with `case_entries[i]` as its branch entry block.

- `ptr` (`struct CCircuitCFG*`): CFG handle.
- `node` (`uint32_t`): header block index.
- `case_lo` (`const uint64_t*`): least-significant halves of the case match values.
- `case_hi` (`const uint64_t*`): most-significant halves of the case match values.
- `case_entries` (`const uint32_t*`): branch entry blocks of the cases.
- `len` (`uintptr_t`): number of cases.
- `default_entry` (`uint32_t`): default branch entry block.
- `merge_block` (`uint32_t`): merge block.
- `has_default` (`int32_t`): whether a default body exists (1/0).
- `outer_op` (`const struct COperation*`): outer operation metadata; may be NULL.

Returns: `0` on success; `-1` when `ptr` is NULL, or when any case array is NULL while `len > 0`; `-2` when the header block does not exist.

---

## Tag constants

Outgoing-edge tags (the `flow` argument of `circuit_cfg_add_edge`, the `flow_tags` array of `circuit_cfg_outgoing_edges`):

| Constant | Value | Meaning |
| --- | --- | --- |
| `CFG_FLOW_TRUE_BRANCH` | 1 | True edge out of `if` / `while` / `for` headers |
| `CFG_FLOW_FALSE_BRANCH` | 2 | False edge out of `if` / `while` / `for` headers |
| `CFG_FLOW_UNCONDITIONAL` | 3 | Fallthrough jump / structured merge |
| `CFG_FLOW_CASE` | 4 | Exact-value switch case edge (128-bit value in the case halves) |
| `CFG_FLOW_DEFAULT_CASE` | 5 | Switch default edge |
| `CFG_FLOW_BREAK` | 6 | Structured `break` edge |
| `CFG_FLOW_CONTINUE` | 7 | Structured `continue` edge |

Terminator tags (`circuit_cfg_block_terminator_tag`):

| Constant | Value | Meaning |
| --- | --- | --- |
| `CFG_TERMINATOR_BRANCH` | 1 | Boolean branch header (`Branch`) |
| `CFG_TERMINATOR_FOR_LOOP` | 2 | Unsigned range loop header (`ForLoop`) |
| `CFG_TERMINATOR_SWITCH` | 3 | Exact-value multi-way branch header (`Switch`) |
| `CFG_TERMINATOR_JUMP` | 4 | Unconditional jump |
| `CFG_TERMINATOR_BREAK` | 5 | Structured break |
| `CFG_TERMINATOR_CONTINUE` | 6 | Structured continue |
| `CFG_TERMINATOR_RETURN` | 7 | End of execution (`Return`) |

Region tags (`circuit_cfg_region_tag`):

| Constant | Value | Meaning |
| --- | --- | --- |
| `CFG_REGION_IF` | 1 | Structured conditional region |
| `CFG_REGION_WHILE` | 2 | Structured while-loop region |
| `CFG_REGION_FOR` | 3 | Structured range-loop region |
| `CFG_REGION_SWITCH` | 4 | Structured exact-value switch region |

---

## Example: building an if-else CFG and converting it back

First build `if (flag) { x(1) } else { z(1) }` at the circuit level, then expand it into a CFG, read the `If` region layout and the header's outgoing edges, and finally validate and reconstruct the structured circuit:

```c
#include <stdint.h>
#include <stdio.h>
#include "cqlib_c.h"

/* then body: X(1) */
static int32_t append_x(struct CCircuit* circuit, void* user_data) {
    (void)user_data;
    return circuit_x(circuit, 1);
}

/* else body: Z(1) */
static int32_t append_z(struct CCircuit* circuit, void* user_data) {
    (void)user_data;
    return circuit_z(circuit, 1);
}

int main(void) {
    struct CCircuit* circuit = circuit_new(2);
    struct CClassicalVar* flag = circuit_var(circuit, CQLIB_CLASSICAL_TYPE_BOOL, 0);
    struct CClassicalExpr* condition = classical_expr_var(flag);
    if (circuit_if_else(circuit, condition, append_x, append_z, NULL) != 0) {
        return 1;
    }

    /* 1. Expand into a CFG */
    struct CCircuitCFG* cfg = circuit_cfg_from_circuit(circuit);
    if (cfg == NULL) {
        return 1;
    }

    /* 2. Locate the if header and read the region layout and edges */
    uint32_t then_entry = UINT32_MAX;
    uint32_t else_entry = UINT32_MAX;
    uint32_t merge_block = UINT32_MAX;
    int32_t has_else = 0;
    uint32_t block_count = (uint32_t)circuit_cfg_num_blocks(cfg);

    for (uint32_t node = 0; node < block_count; node++) {
        if (circuit_cfg_region_tag(cfg, node) == CFG_REGION_IF) {
            circuit_cfg_region_if(cfg, node, &then_entry, &else_entry,
                                  &merge_block, &has_else);

            /* The header terminator is Branch; its two edges are written in
             * insertion order, the true branch first. */
            uint32_t targets[2] = {0, 0};
            uint32_t tags[2] = {0, 0};
            uint64_t lo[2] = {0, 0};
            uint64_t hi[2] = {0, 0};
            circuit_cfg_outgoing_edges(cfg, node, targets, tags, lo, hi, 2);
            /* targets[0] == then_entry && tags[0] == CFG_FLOW_TRUE_BRANCH,
               targets[1] == else_entry && tags[1] == CFG_FLOW_FALSE_BRANCH */
            break;
        }
    }

    /* 3. Validate and convert back to a structured circuit */
    struct CCircuit* lowered = NULL;
    if (circuit_cfg_validate(cfg) == 0) {
        lowered = circuit_cfg_to_circuit(cfg);
    }

    circuit_free(lowered);
    circuit_cfg_free(cfg);
    classical_expr_free(condition);
    classical_var_free(flag);
    circuit_free(circuit);
    return 0;
}
```

For the circuit-building entry points (`circuit_new`, `circuit_if_else`, and friends) see [Circuit](1_circuit.md); for the classical expression builders (`classical_expr_var` and friends) see [Classical Data and Control Flow](9_classical_control_flow.md).
