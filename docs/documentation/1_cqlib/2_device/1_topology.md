# 拓扑建模

Topology 是对量子芯片物理比特及其耦合连通性的图论抽象。它是所有高级编译与路由算法的核心输入：只有准确掌握了芯片的硬件约束，编译器才能在有限的连通路径下寻找最优的门映射方案。

Cqlib 的 Topology 采用**有向图**模型。这意味着 (0, 1, "CX") 并不等同于 (1, 0, "CX")——这条边只允许在比特 0 作控制位、比特 1 作目标位时执行 CX 门。有向建模精确反映了真实硬件中双比特门的方向依赖性。

---

## 构造拓扑结构

### 1. 显式建模

每条边必须使用 (control, target, name) 三元组：

```python
from cqlib.device import Topology

topo = Topology(
    [0, 1, 2],
    [
        (0, 1, "CX"),   # 比特 0 → 比特 1 方向执行 CX
        (1, 2, "CZ"),   # 比特 1 → 比特 2 方向执行 CZ
    ],
)

print("比特总数:", topo.num_qubits)
print("耦合总数:", topo.num_couplings)
print("节点列表:", topo.qubits)
```

**注意**：构造器的 couplings 参数要求 (control, target, name) 三元组，不支持二元组写法。

### 2. 工厂方法

Topology 目前提供 line 工厂，用于快速创建线型拓扑：

```python
from cqlib.device import Topology

# 线型: 0 → 1 → 2 → 3（单向链）
line_topo = Topology.line([0, 1, 2, 3])
print("线型耦合数:", line_topo.num_couplings)
```

**说明**：
- Topology 上仅有 line 工厂方法，没有 ring/star/grid/bidirectional_line
- 如需更丰富的拓扑结构，可使用 Device 的工厂方法（详见[设备属性建模](2_device.md)）或手动添加双向耦合

### 3. 手动添加双向耦合

```python
from cqlib.device import Topology

# 表示 0 和 1 互为控制/目标位（双向耦合）
topo = Topology([0, 1], [(0, 1, "CX"), (1, 0, "CX")])
print("支持 0→1:", topo.supports_directed_coupling(0, 1))   # True
print("支持 1→0:", topo.supports_directed_coupling(1, 0))   # True
```

---

## 拓扑查询与分析

```python
from cqlib.device import Topology

topo = Topology([0, 1, 2], [(0, 1, "CX"), (1, 2, "CZ")])

# 连通性判断
print("有向耦合 0→1:", topo.supports_directed_coupling(0, 1))  # True
print("有向耦合 1→0:", topo.supports_directed_coupling(1, 0))  # False
print("任一方向存在:", topo.supports_coupling_either_direction(0, 1))  # True
print("耦合名称 0→1:", topo.get_coupling_name(0, 1))    # "CX"
print("耦合名称 2→1（反向不存在）:", topo.get_coupling_name(2, 1))  # None

# 查询比特是否存在
print("是否包含比特 2:", topo.contains_qubit(2))    # True
print("是否包含比特 9:", topo.contains_qubit(9))    # False

# 邻接关系查询
print("比特 1 的后继（出边）:", topo.successors(1))           # [Qubit(2)]
print("比特 1 的前驱（入边）:", topo.predecessors(1))         # [Qubit(0)]
print("比特 1 的无向邻居:", topo.neighbors_undirected(1))     # [Qubit(0), Qubit(2)]
print("比特 1 的出度:", topo.out_degree(1))                   # 1
print("比特 1 的入度:", topo.in_degree(1))                    # 1

# 无向边列表（undirected_edges 是方法，需加括号调用）
edges = topo.undirected_edges()
print("无向边列表:", edges)
```

**说明**：
- successors(q) 返回从 q 出发通过有向边可直接到达的比特列表
- predecessors(q) 返回有向边指向 q 的比特列表
- 
eighbors_undirected(q) 合并两个方向，重复节点只出现一次
- contains_qubit(q) 对不存在的比特返回 False，不会抛出异常
- out_degree(q) 和 in_degree(q) 对不存在的比特返回 0

---

## 动态修改拓扑

```python
from cqlib.device import Topology

topo = Topology([0, 1, 2], [(0, 1, "CX"), (1, 2, "CZ")])

# 添加新比特
topo.add_qubits([3, 4])
print("添加后比特数:", topo.num_qubits)  # 5

# 添加新耦合（注意：新增边也需要三元组）
topo.add_couplings([(2, 3, "CX"), (3, 4, "CZ")])
print("添加后耦合数:", topo.num_couplings)  # 4

# 移除去向耦合（仅移除指定方向）
topo.remove_couplings([(2, 3)])
print("移除后耦合数:", topo.num_couplings)  # 2（移除了耦合 (3,4)）

# 移除比特（连带所有关联耦合一起移除）
topo.remove_qubits([4])
print("移除后比特数:", topo.num_qubits)     # 4
print("移除后耦合数:", topo.num_couplings)  # 2（移除了耦合 (3,4)）
```

**边界条件**：
- dd_qubits([...])：添加已有比特会抛出 ValueError
- dd_couplings([...])：如果耦合端点不在拓扑中，或添加已存在的耦合，抛出 ValueError
- 
emove_qubits([...])：如果比特不存在，抛出 ValueError
- 
emove_couplings([...]) 的 couplings 参数使用 (control, target) 二元组（**不含名称**）
- 移除一个比特会自动移除其所有入射和出射耦合边

---

## 有向耦合的工程含义

在真实量子硬件中，双比特门的方向性是一个重要的物理约束：

| 场景 | 有向语义 | 示例 |
|---|---|---|
| CX 门 | 仅允许固定方向 | CX(control=0, target=1) |
| CZ 门 | 通常无方向限制 | 正反向均可执行 |
| 硬件校准 | 不同方向可能有不同保真度 | (0→1) 误差率 1%，(1→0) 误差率 2% |

编译器在路由阶段会利用 supports_directed_coupling() 等查询方法来判断是否需要在当前耦合上插入 SWAP 或桥接门。

---

## 下一步

- [设备属性建模](2_device.md)：掌握 Device 的全局默认 + 局部覆盖标定策略
- [布局映射](3_layout.md)：学习 Layout 的逻辑-物理比特双向映射与 SWAP 路由操作
- [噪声模型](4_noise.md)：了解 NoiseModel、SingleQubitNoise、TwoQubitNoise、ReadoutError 等噪声信道的用法
- [执行结果与状态](5_result.md)：熟悉 Outcome、Status、ExecutionResult 的完整生命周期及错误处理

