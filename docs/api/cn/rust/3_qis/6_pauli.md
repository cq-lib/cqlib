# Pauli 算子与 Pauli 串

`cqlib_core::qis::pauli`

本页介绍单比特 Pauli 算子 `Pauli`、Pauli 群相位 `Phase`、多比特 Pauli 串 `PauliString`，以及按比特遍历 Pauli 串的迭代器 `PauliIter`。

`PauliString` 采用辛表示（symplectic encoding）：每个比特用一对二进制位描述，`x[i] = 1` 表示该比特含 X 分量，`z[i] = 1` 表示该比特含 Z 分量，`(0,0)`、`(1,0)`、`(1,1)`、`(0,1)` 分别对应 `I`、`X`、`Y`、`Z`。乘法的相位变化按位查表累加，比特向量按位异或，因此乘法的代价是每比特常数时间。

## 导入

```rust
use cqlib_core::qis::pauli::{Pauli, PauliIter, PauliString, Phase};
```

`Pauli`、`PauliString`、`PauliIter` 与 `Phase` 也在 `cqlib_core::qis` 顶层重导出。

---

## Phase

Pauli 群的相位因子，与 4 阶循环群同构，表示 $i^n$。

```rust
#[repr(u8)]
pub enum Phase {
    Plus = 0,   // i^0 = 1
    I = 1,      // i^1 = i
    Minus = 2,  // i^2 = -1
    MinusI = 3, // i^3 = -i
}
```

变体：

| 变体 | 复数值 | 指数 |
| --- | --- | --- |
| `Phase::Plus` | `1` | 0 |
| `Phase::I` | `i` | 1 |
| `Phase::Minus` | `-1` | 2 |
| `Phase::MinusI` | `-i` | 3 |

### 方法

- `fn to_complex(&self) -> Complex64`：返回该相位对应的复数。

### 其他行为

- `From<u8>`：按模 4 取余转换，`Phase::from(5)` 等于 `Phase::I`。
- `Add<Phase>`、`AddAssign<Phase>`：相位相加即相位相乘，$i^a \cdot i^b = i^{(a+b) \bmod 4}$。
- `Add<u8>`、`AddAssign<u8>`：以无符号整数作为指数增量。
- `Mul<Phase>`：与相加等价。
- `Display`：输出 `1`、`i`、`-1`、`-i`，不带引号与调试修饰。
- 支持 `Debug`、`Clone`、`Copy`、`PartialEq`、`Eq`、`Hash`。

示例：

```rust
use cqlib_core::qis::pauli::Phase;
use num_complex::Complex64;

// i * i = -1
assert_eq!(Phase::I + Phase::I, Phase::Minus);
assert_eq!(Phase::I * Phase::I, Phase::Minus);

// 指数回绕
assert_eq!(Phase::MinusI + Phase::I, Phase::Plus);

// 按模 4 构造
assert_eq!(Phase::from(5), Phase::I);
assert_eq!(Phase::from(100), Phase::Plus);

assert_eq!(Phase::I.to_complex(), Complex64::new(0.0, 1.0));
assert_eq!(Phase::MinusI.to_string(), "-i");
```

---

## Pauli

单比特 Pauli 算子。

```rust
pub enum Pauli {
    X,
    Y,
    Z,
    I,
}
```

矩阵表示：

| 变体 | 矩阵 | 说明 |
| --- | --- | --- |
| `Pauli::I` | $\begin{pmatrix} 1 & 0 \\ 0 & 1 \end{pmatrix}$ | 恒等算子 |
| `Pauli::X` | $\begin{pmatrix} 0 & 1 \\ 1 & 0 \end{pmatrix}$ | 比特翻转 |
| `Pauli::Y` | $\begin{pmatrix} 0 & -i \\ i & 0 \end{pmatrix}$ | Y 算子 |
| `Pauli::Z` | $\begin{pmatrix} 1 & 0 \\ 0 & -1 \end{pmatrix}$ | 相位翻转 |

### 方法

- `fn to_symplectic(&self) -> (u8, u8)`：返回 `(x, z)` 辛表示分量。
- `fn to_matrix(&self) -> Array2<Complex64>`：返回 2×2 复矩阵。
- `fn mul_with_phase(&self, other: Pauli) -> (Pauli, Phase)`：返回乘积算子与相位，例如 `X * Z = -iY`。

### 其他行为

- `TryFrom<char>`：接受大写字符 `I`、`X`、`Y`、`Z`，其他字符返回 `QisError::InvalidParameterValue`。
- `FromStr`：单字符解析，空串或多字符返回 `QisError::InvalidParameterValue`。
- `Display`：输出算子字符。
- 支持 `Debug`、`Clone`、`Copy`、`PartialEq`、`Eq`、`Hash`。

示例：

```rust
use cqlib_core::qis::pauli::{Pauli, Phase};

// X * Y = iZ
let (product, phase) = Pauli::X.mul_with_phase(Pauli::Y);
assert_eq!(product, Pauli::Z);
assert_eq!(phase, Phase::I);

// Y * X = -iZ，与上式相差符号
let (product, phase) = Pauli::Y.mul_with_phase(Pauli::X);
assert_eq!(product, Pauli::Z);
assert_eq!(phase, Phase::MinusI);

assert_eq!(Pauli::Y.to_symplectic(), (1, 1));
assert_eq!("X".parse::<Pauli>().unwrap(), Pauli::X);
assert!("XY".parse::<Pauli>().is_err());
```

---

## PauliString

多比特 Pauli 串，$P = \bigotimes_{i=0}^{N-1} P_i$。

```rust
pub struct PauliString {
    pub num_qubits: usize,
    pub phase: Phase,
    pub z: BitVec,
    pub x: BitVec,
}
```

字段：

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `num_qubits` | `usize` | 串包含的比特数 $N$。 |
| `phase` | `Phase` | 全局相位因子。 |
| `z` | `BitVec` | Z 分量比特向量，长度为 `num_qubits`。 |
| `x` | `BitVec` | X 分量比特向量，长度为 `num_qubits`。 |

字符串与比特下标约定：字符串中最左边的字符对应最高比特下标，即按张量积顺序书写。因此 `"XZI"` 表示比特 2 为 `X`、比特 1 为 `Z`、比特 0 为 `I`。字段本身按比特下标升序存储，`x[0]` 与 `z[0]` 描述比特 0。

### 构造与查询

- `fn new(num_qubits: usize) -> Self`：创建全为恒等算子的 Pauli 串，相位为 `Phase::Plus`。
- `fn set_pauli(&mut self, idx: usize, p: Pauli)`：设置指定比特上的算子；下标越界时 panic。
- `fn get_pauli(&self, idx: usize) -> Pauli`：读取指定比特上的算子；下标越界时 panic。
- `fn try_set_pauli(&mut self, idx: usize, pauli: Pauli) -> Result<(), QisError>`：越界时返回 `QisError::IndexOutOfBounds`，不修改原串。
- `fn try_get_pauli(&self, idx: usize) -> Result<Pauli, QisError>`：越界时返回 `QisError::IndexOutOfBounds`。
- `fn iter(&self) -> PauliIter<'_>`：按比特下标升序遍历。
- `fn support(&self) -> Vec<usize>`：返回非恒等算子的比特下标，升序排列。
- `fn x_mask(&self) -> usize`：把 X 分量比特向量压成无符号整数掩码，第 `i` 位对应比特 `i`。
- `fn z_mask(&self) -> usize`：把 Z 分量比特向量压成无符号整数掩码。

### 运算

- `fn commutes_with(&self, other: &Self) -> bool`：按辛内积判断对易性，内积为 0 即对易；比特数不同时 panic。
- 串间的乘法由 `Mul` 与 `MulAssign` 提供，相位随乘积累加，见「其他行为」。

### 矩阵、相位与期望值

- `fn to_matrix(&self) -> Array2<Complex64>`：返回 $2^N \times 2^N$ 稠密矩阵。比特 0 是最低有效张量因子，串按 $P_{N-1} \otimes \cdots \otimes P_0$ 展开，全局相位计入结果。
- `fn y_phase(&self) -> Complex64`：返回 Y 分量贡献的相位因子。辛表示下 $Y = iXZ$，$n$ 个 Y 贡献 $i^n$。
- `fn expectation(&self, probs: &HashMap<String, f64>) -> Result<f64, QisError>`：在计算基概率分布上计算期望值 $\langle P \rangle = \sum_s p(s)\langle s|P|s\rangle$。

`expectation` 的约定与错误：

- 概率字典的键按小端书写，最右字符对应比特 0。例如 `"01"` 表示比特 0 为 1、比特 1 为 0。
- 串中含 X 或 Y 时，计算基下的期望值恒为 0，直接返回 `Ok(0.0)`。
- 只含 Z 与 I 时，期望值为 $\langle P \rangle = \text{phase} \times \sum_s p(s) \times (-1)^{\sum_i z[i] s[i]}$。
- 键长度与 `num_qubits` 不一致时返回 `QisError::DimensionMismatch`；键中出现 `0`、`1` 以外的字符时返回 `QisError::PauliStringParseError`。

### 其他行为

- `Display`：输出形如 `+XYZ` 或 `-iZIX`，比特按最高下标在左的顺序打印。
- `FromStr`：接受 `[+|-][i|j]<算子序列>`，例如 `"XZI"`、`"+XYZ"`、`"-iZII"`、`"+jX"`；解析失败返回 `PauliStringParseError`。
- `Mul for &PauliString`：返回新的 Pauli 串；两边比特数不同时 panic。
- `MulAssign<&PauliString>`：原地相乘，相位按位累加、比特向量按位异或。
- `IntoIterator for &PauliString`：等价于 `iter()`。
- 支持 `Debug`、`Clone`、`PartialEq`、`Eq`、`Hash`。

示例：

```rust
use cqlib_core::qis::pauli::{Pauli, PauliString};

// 逐位设置
let mut ps = PauliString::new(3);
ps.set_pauli(0, Pauli::X);
assert_eq!(ps.to_string(), "+IIX");

ps.set_pauli(2, Pauli::Z);
assert_eq!(ps.to_string(), "+ZIX");

// 字符串解析使用最高比特在左的顺序
let parsed: PauliString = "XZI".parse().unwrap();
assert_eq!(parsed.get_pauli(0), Pauli::I);
assert_eq!(parsed.get_pauli(1), Pauli::Z);
assert_eq!(parsed.get_pauli(2), Pauli::X);
assert_eq!(parsed.support(), vec![1, 2]);
```

```rust
use cqlib_core::qis::pauli::{Pauli, PauliString};

// X * Z = -iY，相位随乘积一并保存
let mut x = PauliString::new(1);
x.set_pauli(0, Pauli::X);
let mut z = PauliString::new(1);
z.set_pauli(0, Pauli::Z);

let product = &x * &z;
assert_eq!(product.to_string(), "-iY");

// 原地乘法：X * Y = iZ
let mut y = PauliString::new(1);
y.set_pauli(0, Pauli::Y);

x *= &y;
assert_eq!(x.to_string(), "+iZ");
```

对易性与掩码：

```rust
use cqlib_core::qis::pauli::{Pauli, PauliString};

// XX 与 ZZ 在两个比特上都反对易，反对易次数为偶数，因此整体对易
let mut xx = PauliString::new(2);
xx.set_pauli(0, Pauli::X);
xx.set_pauli(1, Pauli::X);

let mut zz = PauliString::new(2);
zz.set_pauli(0, Pauli::Z);
zz.set_pauli(1, Pauli::Z);

assert!(xx.commutes_with(&zz));

// 同一比特上的 X 与 Z 反对易
let mut x = PauliString::new(1);
x.set_pauli(0, Pauli::X);
let mut z = PauliString::new(1);
z.set_pauli(0, Pauli::Z);
assert!(!x.commutes_with(&z));

// 掩码：Y 同时置起 X 与 Z 分量
let mut ps = PauliString::new(3);
ps.set_pauli(0, Pauli::X);
ps.set_pauli(2, Pauli::Y);
assert_eq!(ps.x_mask(), 0b101);
```

从概率分布计算期望值：

```rust
use cqlib_core::qis::pauli::{Pauli, PauliString};
use std::collections::HashMap;

let mut zz = PauliString::new(2);
zz.set_pauli(0, Pauli::Z);
zz.set_pauli(1, Pauli::Z);

// Bell 态的概率分布，键按小端书写
let mut probs = HashMap::new();
probs.insert("00".to_string(), 0.5);
probs.insert("11".to_string(), 0.5);

assert!((zz.expectation(&probs).unwrap() - 1.0).abs() < 1e-10);
```

---

## PauliIter

按比特下标升序遍历 Pauli 串的迭代器。

```rust
pub struct PauliIter<'a> {
    // 字段不公开
}
```

- `Iterator<Item = Pauli>`：逐项返回比特上的算子，`size_hint` 精确。
- 实现 `ExactSizeIterator` 与 `FusedIterator`。

示例：

```rust
use cqlib_core::qis::pauli::{Pauli, PauliString};

let pauli: PauliString = "XYZI".parse().unwrap();
let collected: Vec<Pauli> = pauli.iter().collect();
let expected: Vec<Pauli> = (0..pauli.num_qubits)
    .map(|index| pauli.get_pauli(index))
    .collect();

assert_eq!(collected, expected);

// 借用 PauliString 也可以直接进入 for 循环
let mut count = 0;
for _ in &pauli {
    count += 1;
}
assert_eq!(count, 4);
```

---

## 校验与错误处理

`QisError` 与 `PauliStringParseError` 是本节各接口的错误来源：

| 错误 | 触发场景 |
| --- | --- |
| `QisError::InvalidParameterValue` | `Pauli` 由字符或字符串构造时遇到非 `I`、`X`、`Y`、`Z` 的输入，或输入为空串、多字符。 |
| `QisError::IndexOutOfBounds` | `try_get_pauli()` 与 `try_set_pauli()` 的比特下标不小于 `num_qubits`。 |
| `QisError::DimensionMismatch` | `expectation()` 的概率键长度与 `num_qubits` 不一致。 |
| `QisError::PauliStringParseError` | `expectation()` 的概率键中出现非 `0`、`1` 字符。 |
| `PauliStringParseError::EmptyString` | 解析 `PauliString` 时输入为空串。 |
| `PauliStringParseError::InvalidCharacter` | 解析 `PauliString` 时出现非 `I`、`X`、`Y`、`Z` 的字符。 |
| `PauliStringParseError::NoOperators` | 解析 `PauliString` 时只有相位前缀而没有算子，例如 `"-"`。 |

以下情况以 panic 表达，而不是返回错误：`set_pauli()` 与 `get_pauli()` 的下标越界，`commutes_with()`、`Mul`、`MulAssign` 两侧比特数不一致。需要以错误形式处理越界时，改用 `try_set_pauli()` 与 `try_get_pauli()`。

---

## 相关页面

- [QIS 概览](0_overview.md)：模块总览与术语表。
- [Hamiltonian 与 Observable](7_hamiltonian.md)：由 Pauli 串与系数构造可观测量并求期望值。
