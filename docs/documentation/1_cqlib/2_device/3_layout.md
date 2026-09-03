# 布局映射

Layout 管理逻辑比特到物理比特的双向映射，是编译路由阶段的核心数据结构。

---

## 核心概念

- **逻辑比特（LogicalQubit）**：算法/电路中定义的虚拟比特
- **物理比特（PhysicalQubit）**：硬件芯片上真实的物理比特索引
- **空闲物理比特（Vacant PhysicalQubit）**：未被逻辑比特占用的物理位置，可用于后续绑定

---

## 构造映射

Layout 支持多种构造方式：

```python
from cqlib.device import Layout

# 方式一：自动顺序映射
# 逻辑比特 [0, 1] 自动映射到物理比特 [10, 11]
# 物理比特 12 保持空闲
layout = Layout(logical=[0, 1], physical=[10, 11, 12])
print("逻辑比特数:", layout.num_logical)       # 2
print("物理比特数:", layout.num_physical)      # 3
print("空闲物理比特数:", layout.num_vacant_physical)  # 1

# 方式二：通过 from_pairs 指定初始映射
# (逻辑, 物理) 对明确指定映射关系，其余物理比特保持空闲
layout2 = Layout.from_pairs([(0, 2), (1, 0)], physical_count=4)
print("from_pairs 空闲数:", layout2.num_vacant_physical)  # 2
```

**注意**：
- Layout.__init__ 的 init_map 参数要求 dict[Qubit, Qubit] 类型，直接传入 {0: 11} 会因类型不匹配抛出 TypeError
- 如需要指定初始映射关系，推荐使用 Layout.from_pairs() 替代
- logical 和 physical 列表长度应满足 len(logical) <= len(physical)，否则抛出 ValueError

---

## 查询映射

```python
from cqlib.device import Layout

layout = Layout.from_pairs([(0, 11), (1, 10)], physical_count=13)

print("逻辑比特列表:", layout.logical_qubits)
print("物理比特列表:", layout.physical_qubits)
print("空闲物理比特:", layout.vacant_physical_qubits)

# 正向查询：逻辑 → 物理
print("逻辑 0 映射到物理:", layout.get_physical(0))   # Qubit(11)

# 反向查询：物理 → 逻辑
print("物理 11 映射到逻辑:", layout.get_logical(11))  # Qubit(0)

# 查询物理比特是否空闲
print("物理 10 是否空闲:", layout.is_physical_vacant(10))  # False（被逻辑 1 占用）
print("物理 12 是否空闲:", layout.is_physical_vacant(12))  # True

# 获取完整映射字典
print("逻辑→物理映射:", layout.l2p_map)
print("物理→逻辑映射:", layout.p2l_map)
```

**说明**：
- get_physical(logical_id) 对未绑定的逻辑比特返回 None
- get_logical(physical_id) 对空闲的物理比特返回 None
- p2l_map 只包含已被逻辑比特占用的物理比特，空闲比特不会出现在其中

---

## 更新映射

```python
from cqlib.device import Layout

# 初始映射：逻辑 0→物理 11，逻辑 1→物理 12
layout = Layout.from_pairs([(0, 11), (1, 12)], physical_count=13)
print("初始空闲:", layout.num_vacant_physical)  # 11

# bind：将新的逻辑比特绑定到空闲物理比特
layout.bind(2, 10)
print("绑定后空闲:", layout.num_vacant_physical)  # 10

# unbind：解绑逻辑比特，释放物理比特
released = layout.unbind(0)
print("解绑后释放:", released)                    # Qubit(11)
print("解绑后空闲:", layout.num_vacant_physical)  # 11

# swap_physical：交换两个物理比特上承载的逻辑比特（核心路由操作）
layout3 = Layout.from_pairs([(0, 11), (1, 12)], physical_count=13)
layout3.swap_physical(11, 12)
print("SWAP 后物理 11→逻辑:", layout3.get_logical(11))  # Qubit(1)
print("SWAP 后物理 12→逻辑:", layout3.get_logical(12))  # Qubit(0)
```

**边界条件**：
- ind(logical, physical)：如果 physical 已被占用，或 logical 已绑定，抛出 ValueError
- unbind(logical)：如果 logical 未绑定，抛出 ValueError
- swap_physical(a, b)：如果  或  不在布局中，抛出 ValueError。允许其中一个为空闲比特（相当于移动逻辑比特）

---

## 健壮性保障

```python
from cqlib.device import Layout

layout = Layout.from_pairs([(0, 11), (1, 12)], physical_count=13)

try:
    layout.swap_physical(11, 99)  # 99 不在布局中
except ValueError as e:
    print("无效的物理比特:", e)

try:
    layout.bind(0, 11)  # 物理 11 已被占用
except ValueError as e:
    print("绑定已被占用的物理比特:", e)

try:
    layout.swap_physical(11, 12)  # 正常操作，不会抛出异常
    print("SWAP 操作成功")
    print("物理 11→逻辑:", layout.get_logical(11))
    print("物理 12→逻辑:", layout.get_logical(12))
```

---

## 下一步

- [噪声模型](4_noise.md)：了解 NoiseModel、SingleQubitNoise、TwoQubitNoise、ReadoutError 等噪声信道的用法
- [执行结果与状态](5_result.md)：熟悉 Outcome、Status、ExecutionResult 的完整生命周期及错误处理
