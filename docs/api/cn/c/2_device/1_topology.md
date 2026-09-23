# Topology

`CTopology` 描述硬件物理比特与耦合边关系。拓扑从设备克隆获得，为只读视图。

---

## 获取拓扑

### device_topology(device)

克隆并返回设备的拓扑对象。

参数：

- `device` (`const CDevice *`)：设备。

返回：

- `CTopology *`：**独立拥有**的新对象，与源设备的生命周期互不影响，需用 `topology_free` 释放；失败返回 NULL。

### topology_free(ptr)

释放拓扑对象。允许传 NULL。

---

## 属性

### topology_num_qubits(topology)

返回拓扑中的物理比特数量。

返回：

- `uintptr_t`；NULL 句柄返回 `0`。

### topology_num_couplings(topology)

返回拓扑中的耦合边数量。

返回：

- `uintptr_t`；NULL 句柄返回 `0`。

---

## 说明

拓扑按**有向边**语义存储：`(u, v)` 表示允许以 `u` 为控制、`v` 为目标的耦合。双向耦合存储为两条有向边，因此双向线 `0 - 1 - 2` 的 `num_couplings` 为 `4`。

拓扑的形状由设备的构造方式决定，见 [Device](2_properties_device.md) 的构造函数表。

---

## 示例

```c
CDevice *dev = device_bidirectional_line("line-4", 4);

CTopology *topo = device_topology(dev);
printf("qubits=%zu couplings=%zu\n",
       (size_t)topology_num_qubits(topo),
       (size_t)topology_num_couplings(topo));   // 4, 6

topology_free(topo);
device_free(dev);
```

设备销毁后克隆出的拓扑仍可独立使用：

```c
device_free(dev);
printf("%zu\n", (size_t)topology_num_qubits(topo));   // 仍然有效
topology_free(topo);
```
