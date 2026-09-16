# cqlib 1.x 基础流程兼容

本兼容层从 cqlib 1.4 开始提供，覆盖旧版 1.3.x 常见的构建线路、追加标准门、
绑定参数、测量、复制组合、导入导出和文本绘图流程。兼容代码全部位于
`crates/binding-python`，无需修改 `cqlib-core`。

## 对象与转发方式

只有一个原生 `Circuit` 类型；新旧导入路径得到相同的类，兼容调用创建的线路可以
直接传给新编译器、IR 模块和绘图函数。`Qubit`、`Parameter`、`Instruction` 同样是
原生类型的别名，没有包装类，也不需要转换线路对象。

```python
from cqlib import Circuit
from cqlib.circuits import Circuit as OldCircuit

assert Circuit is OldCircuit
```

Python 层提供导入路径、门工厂、操作工厂和废弃提示。Rust 的 Python 绑定层增加
旧调用入口及参数适配。原有方法实现保留为内部方法，由入口调用；普通门方法与
core 算法不变。线路额外保存可选的参数声明顺序，只服务旧版序列绑定。

## 支持的导入路径

| 旧入口                                                                | 新入口 / 行为                                          |
|-----------------------------------------------------------------------|--------------------------------------------------------|
| `cqlib.circuits.Circuit`、`.circuit.Circuit`                          | `cqlib.circuit.Circuit`，同一类型                      |
| `cqlib.circuits.Qubit`、`.qubit.Qubit`                                | `cqlib.circuit.Qubit`，同一类型                        |
| `cqlib.circuits.Parameter`、`.parameter.Parameter`                    | `cqlib.circuit.Parameter`，同一类型                    |
| `cqlib.circuits.Instruction`、`.instruction.Instruction`              | 原生 `Instruction`；不恢复旧的任意指令构造函数         |
| `cqlib.circuits.InstructionData`、`.instruction_data.InstructionData` | 工厂返回原生 `ValueOperation`                          |
| `cqlib.circuits.gates`；`from cqlib import gates`                     | 标准门工厂，返回原生 `StandardGate`                    |
| `from cqlib import InstructionData`                                   | 同一个操作工厂                                         |
| `cqlib.utils.qasm2`                                                   | 直接重导出 `cqlib.ir.qasm2` 的 `load/loads/dump/dumps` |
| `cqlib.exceptions.CqlibError`                                         | 原生基础异常，类型身份不变                             |

顶层兼容入口支持显式导入；没有把 `gates` 和 `InstructionData` 加入新的顶层
`__all__`。旧路径废弃提示在导入时触发，Python 模块缓存生效后不会重复执行导入。

### 门与操作工厂

支持 H/X/Y/Z/S/T、SD/TD、RX/RY/RZ、CX/CY/CZ/CCX、CNOT/CCNOT、
CRX/CRY/CRZ、SWAP、U、FSIM、X2P/X2M、Y2P/Y2M、XY/XY2P/XY2M。
SD/TD 转发到 SDG/TDG，CNOT/CCNOT 转发到 CX/CCX。
RXY 工厂直接使用当前原生 `(theta, phi)` 顺序，本次不处理旧参数顺序兼容。

工厂接受原生 `Parameter` 或数值，支持例如 `RX(theta=...)`、`U(theta, phi, lam)`。
返回的门保留绑定参数，追加时通过 `ValueOperation.from_standard_gate` 转换，
不会先转换为不持有参数的 `Instruction`。

```python
from cqlib.circuits import Circuit, InstructionData, Parameter, gates

theta = Parameter("theta")
circuit = Circuit(2, parameters=[theta])
circuit.append(gates.RX(theta), 0)
circuit.append_instruction_data(InstructionData(gates.CNOT(), [0, 1]))
bound = circuit.assign_parameters([0.3])
bound.measure_all()
print(bound.qcis)
```

这些入口是函数，不支持 `isinstance(gate, RX)`、继承旧门类或修改门的 `params`。
非空旧门标签会报错，改用 `append_gate(gate, qubits, label=...)` 或操作级标签。
不支持未绑定的 `FSIM()` 参数占位形式；传入两个具体数值或符号参数。
`InstructionData` 不再是 NamedTuple；通过 `operation.params` 读取参数。

## Circuit 方法

| 兼容调用                                     | 转发 / 返回值                             | 废弃       |
|----------------------------------------------|-------------------------------------------|------------|
| `add_qubit(q)`                               | `add_qubits([q])`，返回 `None`            | 否         |
| `sd(q)` / `td(q)`                            | `sdg(q)` / `tdg(q)`                       | 是         |
| `i(q, t)`                                    | `delay(q, t)`；`i(q)` 仍为恒等门          | 仅旧形式   |
| `barrier(q0, q1, ...)` / `barrier()`         | 归一化后调用 `barrier(list)`              | 仅旧形式   |
| `barrier_all()`                              | 对当前所有比特调用 `barrier(list)`        | 否         |
| `measure([q0, q1])`                          | 按顺序逐个调用原生 `measure`，返回 `None` | 仅列表形式 |
| `measure(q)`                                 | 原有行为，返回原生 `Measurement`          | 否         |
| `measure_all()`                              | 补测尚未测量的比特，返回 `None`           | 否         |
| `append(gate, qubits)`                       | 构造原生操作后调用原有 `append`           | 仅旧形式   |
| `append(instruction=gate, qubits=...)`       | 同上                                      | 是         |
| `append(operation)`                          | 原有行为                                  | 否         |
| `append_instruction_data(operation)`         | `append(operation)`                       | 是         |
| `copy()` / `copy.copy()` / `copy.deepcopy()` | 原生克隆，独立线路                        | 否         |
| `a + b`                                      | 复制 a，再 `compose(b)`                   | 否         |
| `a += b`                                     | `compose(b)`，保留 a 的 Python 对象身份   | 否         |

列表测量和 `measure_all()` 只支持静态线路中的末端测量：某比特测量后再对它施加
门，不属于这个兼容范围。重复调用 `measure_all()` 不追加重复测量，非连续比特 ID
按线路中的真实 ID 处理。经典存储和控制流请继续使用原生测量接口。
原生 `measure_bits` 的 `Measurement` 返回值、位序和行为不变。

旧批量调用先完成输入校验；废弃警告被设置为异常时，不执行后续修改。
`+=` 先在临时副本上验证可组合性，再调用原有组合实现，保留已有经典句柄的所有者
身份。已有 `compose()` 的执行逻辑、返回值和错误语义不变。

## 参数绑定

支持下面的旧调用形式；原生 `assign_parameters(bindings={"theta": 0.2})`
继续走原实现，不发出废弃提示。

```python
from cqlib import Circuit, Parameter

theta, phi = Parameter("theta"), Parameter("phi")
circuit = Circuit(2, parameters=[phi, theta])
circuit.rx(0, theta + 2 * phi)

circuit.assign_parameters({theta: 0.2, phi: 0.3})
circuit.assign_parameters([0.3, 0.2])   # 显式声明顺序 phi, theta
circuit.assign_parameters(values={theta: 0.2}, phi=0.3)
circuit.assign_parameters(theta=0.2, phi=0.3)
assert circuit.assign_parameters(theta=0.2, inplace=True) is circuit
```

- `parameters=[...]` 和 `add_parameter(p)` 只接受单符号参数，拒绝重复声明和表达式。
- 序列绑定要求显式声明顺序，长度必须完全一致；支持 list、tuple、数值 NumPy 数组。
  不会推测参数表或符号集合的顺序。
- `bindings`、`values` 和关键字参数可以提供不同符号；同一符号重复赋值会报错。
- 支持部分绑定。复制、绑定、inverse、decompose 保留原声明顺序；已经绑定的项在
  后续序列里仍占据原位置，重新传值不会恢复已替换掉的符号。
- compose、`+`、`+=` 在顺序明确时按左侧优先合并并去重；任一来源有未声明的使用中
  符号时，清除兼容顺序，后续使用名称字典绑定。新增未声明符号后也会拒绝序列绑定。
- 导入、DAG 转换、编译输出、Ansatz 等重新生成的线路默认没有兼容声明顺序。
- 原生 `.parameters` 仍是驻留的参数表达式表；`.symbols`、`.used_symbols` 仍按新 API
  定义工作。不会增加旧的可变参数值字典。
- `cache_params=True` 和 Tensor 绑定不支持；旧默认值 `cache_params=False` 可接受。
- `inplace=True` 在绑定成功后替换内部线路，并返回同一个 Python 对象。其内部
  `CircuitId` 可能改变，请勿继续使用绑定前取得的经典句柄；动态线路不在旧绑定兼容范围。
- 当前流程应先绑定参数，再追加测量。测量后再绑定存在一个原生 core 校验问题，
  详见下文；本轮保留已有接口逻辑，没有修改该行为。

## 导入导出与绘图

| 旧入口                                               | 新调用                                             |
|------------------------------------------------------|----------------------------------------------------|
| `Circuit.load(text)`                                 | `cqlib.ir.qcis.loads(text)`，输入为字符串内容      |
| `circuit.qcis` / `circuit.as_str()`                  | `cqlib.ir.qcis.dumps(circuit)`                     |
| `circuit.to_qasm2()`                                 | `cqlib.ir.qasm2.dumps(circuit)`                    |
| `circuit.draw()` / `draw(category="text", **kwargs)` | `cqlib.visualization.draw_text(circuit, **kwargs)` |

采用新序列化器和绘图器的语义、支持范围及格式，不承诺旧文本逐字一致。
不复现旧 QCIS 自动门分解，也不接受 `as_str(qcis_compliant=...)`。
文本绘图选项使用新 `draw_text` 选项；旧 Matplotlib 类别和选项不支持。

## 废弃提示与静态标记

旧入口发出 `CqlibDeprecationWarning`，它继承 `DeprecationWarning`，说明替代调用并指向
用户代码位置。Python 通常隐藏这类提示，迁移时可显式打开：

```python
import warnings
from cqlib import CqlibDeprecationWarning

warnings.simplefilter("default", CqlibDeprecationWarning)
```

`.pyi` 对旧方法、工厂和旧重载使用 PEP 702 `@deprecated` 标记。共享原生类及普通
调用形式不会被标记废弃。纯导入别名通过文档与导入提示说明，不装饰原生类。
本轮不设定移除日期，后续移除安排另行发布。

## 本轮不支持

云平台、Torch、仿真机接口；完整旧 Gate/Instruction 继承模型、SymPy 内部属性、
可变 circuit_data / parameters_value、旧 DAG 图对象、旧映射返回值、insert、
独立 I(t) / Measure() / Barrier(n) 构造器，以及每个门独立的历史叶模块路径。
RXY 参数顺序不作兼容调整；已有门方法的关键字名称仍使用新 API。

上述限制不影响列出的基础构建流程。以后若发现某项需求需要修改 core，应单独列出
被阻塞的用法、绑定层替代方案及 core 修改理由，不在兼容层中静默改变原有接口语义。

## 单独列出的 core 问题：测量后绑定参数

**状态：本轮未修改，建议作为独立 core 修复处理。**

原生 `Circuit::assign_parameters` 创建新的 CircuitId，复制经典类型表和测量操作，
但没有把操作中经典值的所属线路 ID 重映射到新线路。这是原有名称字典绑定路径就能
复现的问题，不依赖兼容入口：

```python
from cqlib import Circuit, Parameter

circuit = Circuit(1)
circuit.rx(0, Parameter("theta"))
circuit.measure(0)
bound = circuit.assign_parameters({"theta": 0.2})
bound.validate()  # CircuitError: classical value 0 belongs to another circuit
```

- **影响的流程**：先建立包含原生测量的参数化线路，再绑定并交给需要经典所有权校验
  的新 API。兼容层中转发到原生测量的方法也受到这个已有问题影响。
- **当前替代方法**：在线路中追加测量前完成参数绑定。
- **为何本轮不在绑定层补丁修复**：只修复旧调用会让相同对象的新旧绑定形式产生
  不一致结果；同时修复所有形式会改变本轮要求保留的已有接口逻辑。将测量改写为
  不持有经典值的指令还会丢失原生测量语义。
- **建议的独立 core 修改**：在 `cqlib-core/src/circuit/circuit_impl.rs` 的
  `assign_parameters` 中，重用已有经典句柄重映射能力；为新线路重建操作中的经典值、
  变量和表达式引用。新线路内的引用应有效，原线路外部句柄仍不自动属于新线路。
- **独立验收**：测量后绑定得到的线路通过 `validate()`，经典数据引用正确，源线路
  不变，已有数值和部分绑定结果不变。控制流参数绑定继续维持原来的支持范围。

基础兼容层本身不要求 core 修改；要支持上述调用顺序，应先完成这项独立修复。

## 验证

兼容测试位于 `tests/compat`，覆盖类型身份、参数保留、数值矩阵等价、声明顺序、部分
绑定、失败不修改、组合和复制、非连续比特测量、原生编译器互操作、序列化与废弃提示。
开发包运行完整 Python 回归测试，并对安装后的 wheel 单独运行兼容测试。

```sh
cargo check --locked -p binding-python
cargo test --locked -p binding-python --lib
python -m pytest crates/binding-python/tests
```

已有图像回归测试需要其指定且经过 SHA-256 校验的字体，可通过 `CQLIB_VISUAL_FONT`
指定本地文件。wheel 测试应在独立目录运行，并使用 `--import-mode=importlib`，避免
测试目录让 Python 提前导入工作区中的开发包。
