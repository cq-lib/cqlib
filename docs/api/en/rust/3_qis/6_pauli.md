# Pauli Operators and Pauli Strings

`cqlib_core::qis::pauli`

This page covers the single-qubit Pauli operator `Pauli`, the Pauli group phase `Phase`, the multi-qubit Pauli string `PauliString`, and the iterator `PauliIter` that traverses a Pauli string qubit by qubit.

`PauliString` uses a symplectic encoding: each qubit is described by a pair of bits, where `x[i] = 1` means that qubit carries an X component and `z[i] = 1` means it carries a Z component, and `(0,0)`, `(1,0)`, `(1,1)`, `(0,1)` correspond to `I`, `X`, `Y`, `Z` respectively. The phase change of a multiplication is accumulated bit by bit through a lookup table and the bit vectors are XORed bit by bit, so the cost of a multiplication is constant time per qubit.

## Import

```rust
use cqlib_core::qis::pauli::{Pauli, PauliIter, PauliString, Phase};
```

`Pauli`, `PauliString`, `PauliIter` and `Phase` are also re-exported at the top level of `cqlib_core::qis`.

---

## Phase

The phase factor of the Pauli group, isomorphic to the cyclic group of order 4, representing $i^n$.

```rust
#[repr(u8)]
pub enum Phase {
    Plus = 0,   // i^0 = 1
    I = 1,      // i^1 = i
    Minus = 2,  // i^2 = -1
    MinusI = 3, // i^3 = -i
}
```

Variants:

| Variant | Complex value | Exponent |
| --- | --- | --- |
| `Phase::Plus` | `1` | 0 |
| `Phase::I` | `i` | 1 |
| `Phase::Minus` | `-1` | 2 |
| `Phase::MinusI` | `-i` | 3 |

### Methods

- `fn to_complex(&self) -> Complex64`: return the complex number corresponding to this phase.

### Other behavior

- `From<u8>`: conversion by remainder modulo 4, so `Phase::from(5)` equals `Phase::I`.
- `Add<Phase>`, `AddAssign<Phase>`: adding phases is multiplying phases, $i^a \cdot i^b = i^{(a+b) \bmod 4}$.
- `Add<u8>`, `AddAssign<u8>`: an unsigned integer is used as the exponent increment.
- `Mul<Phase>`: equivalent to addition.
- `Display`: outputs `1`, `i`, `-1`, `-i`, without quotes or debug decoration.
- Supports `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`.

Example:

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

The single-qubit Pauli operator.

```rust
pub enum Pauli {
    X,
    Y,
    Z,
    I,
}
```

Matrix representation:

| Variant | Matrix | Description |
| --- | --- | --- |
| `Pauli::I` | $\begin{pmatrix} 1 & 0 \\ 0 & 1 \end{pmatrix}$ | Identity operator |
| `Pauli::X` | $\begin{pmatrix} 0 & 1 \\ 1 & 0 \end{pmatrix}$ | Bit flip |
| `Pauli::Y` | $\begin{pmatrix} 0 & -i \\ i & 0 \end{pmatrix}$ | Y operator |
| `Pauli::Z` | $\begin{pmatrix} 1 & 0 \\ 0 & -1 \end{pmatrix}$ | Phase flip |

### Methods

- `fn to_symplectic(&self) -> (u8, u8)`: return the `(x, z)` symplectic components.
- `fn to_matrix(&self) -> Array2<Complex64>`: return the 2×2 complex matrix.
- `fn mul_with_phase(&self, other: Pauli) -> (Pauli, Phase)`: return the product operator and the phase, for example `X * Z = -iY`.

### Other behavior

- `TryFrom<char>`: accepts the uppercase characters `I`, `X`, `Y`, `Z`; other characters return `QisError::InvalidParameterValue`.
- `FromStr`: single-character parsing; an empty string or multiple characters return `QisError::InvalidParameterValue`.
- `Display`: outputs the operator character.
- Supports `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`.

Example:

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

A multi-qubit Pauli string, $P = \bigotimes_{i=0}^{N-1} P_i$.

```rust
pub struct PauliString {
    pub num_qubits: usize,
    pub phase: Phase,
    pub z: BitVec,
    pub x: BitVec,
}
```

Fields:

| Field | Type | Description |
| --- | --- | --- |
| `num_qubits` | `usize` | The number of qubits $N$ contained in the string. |
| `phase` | `Phase` | The global phase factor. |
| `z` | `BitVec` | The Z component bit vector, of length `num_qubits`. |
| `x` | `BitVec` | The X component bit vector, of length `num_qubits`. |

String and qubit index convention: the leftmost character of the string corresponds to the highest qubit index, that is, the string is written in tensor product order. Thus `"XZI"` means qubit 2 is `X`, qubit 1 is `Z` and qubit 0 is `I`. The fields themselves are stored in ascending qubit index order, and `x[0]` and `z[0]` describe qubit 0.

### Construction and queries

- `fn new(num_qubits: usize) -> Self`: create a Pauli string of all identity operators, with phase `Phase::Plus`.
- `fn set_pauli(&mut self, idx: usize, p: Pauli)`: set the operator on the given qubit; panics when the index is out of range.
- `fn get_pauli(&self, idx: usize) -> Pauli`: read the operator on the given qubit; panics when the index is out of range.
- `fn try_set_pauli(&mut self, idx: usize, pauli: Pauli) -> Result<(), QisError>`: returns `QisError::IndexOutOfBounds` when out of range, without modifying the original string.
- `fn try_get_pauli(&self, idx: usize) -> Result<Pauli, QisError>`: returns `QisError::IndexOutOfBounds` when out of range.
- `fn iter(&self) -> PauliIter<'_>`: traverse in ascending qubit index order.
- `fn support(&self) -> Vec<usize>`: return the qubit indices of the non-identity operators, in ascending order.
- `fn x_mask(&self) -> usize`: pack the X component bit vector into an unsigned integer mask, where bit `i` corresponds to qubit `i`.
- `fn z_mask(&self) -> usize`: pack the Z component bit vector into an unsigned integer mask.

### Operations

- `fn commutes_with(&self, other: &Self) -> bool`: determine commutation by the symplectic inner product; an inner product of 0 means the two commute; panics when the qubit counts differ.
- Multiplication between strings is provided by `Mul` and `MulAssign`, and the phase accumulates with the product; see "Other behavior".

### Matrix, phase and expectation value

- `fn to_matrix(&self) -> Array2<Complex64>`: return a dense $2^N \times 2^N$ matrix. Qubit 0 is the least significant tensor factor, the string is expanded as $P_{N-1} \otimes \cdots \otimes P_0$, and the global phase is included in the result.
- `fn y_phase(&self) -> Complex64`: return the phase factor contributed by the Y components. In the symplectic encoding $Y = iXZ$, so $n$ Y's contribute $i^n$.
- `fn expectation(&self, probs: &HashMap<String, f64>) -> Result<f64, QisError>`: compute the expectation value $\langle P \rangle = \sum_s p(s)\langle s|P|s\rangle$ over a computational basis probability distribution.

Conventions and errors of `expectation`:

- The keys of the probability map are written in little-endian, with the rightmost character corresponding to qubit 0. For example `"01"` means qubit 0 is 1 and qubit 1 is 0.
- When the string contains X or Y, the expectation value in the computational basis is always 0, and `Ok(0.0)` is returned directly.
- When it contains only Z and I, the expectation value is $\langle P \rangle = \text{phase} \times \sum_s p(s) \times (-1)^{\sum_i z[i] s[i]}$.
- When the key length is inconsistent with `num_qubits`, `QisError::DimensionMismatch` is returned; when a key contains a character other than `0` and `1`, `QisError::PauliStringParseError` is returned.

### Other behavior

- `Display`: outputs a form such as `+XYZ` or `-iZIX`, printing qubits with the highest index on the left.
- `FromStr`: accepts `[+|-][i|j]<operator sequence>`, for example `"XZI"`, `"+XYZ"`, `"-iZII"`, `"+jX"`; a parse failure returns `PauliStringParseError`.
- `Mul for &PauliString`: returns a new Pauli string; panics when the qubit counts of the two sides differ.
- `MulAssign<&PauliString>`: multiplies in place, accumulating the phase bit by bit and XORing the bit vectors bit by bit.
- `IntoIterator for &PauliString`: equivalent to `iter()`.
- Supports `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`.

Example:

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

Commutation and masks:

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

Computing an expectation value from a probability distribution:

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

An iterator that traverses a Pauli string in ascending qubit index order.

```rust
pub struct PauliIter<'a> {
    // 字段不公开
}
```

- `Iterator<Item = Pauli>`: returns the operator on each qubit; `size_hint` is exact.
- Implements `ExactSizeIterator` and `FusedIterator`.

Example:

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

## Validation and error handling

`QisError` and `PauliStringParseError` are the error sources of the interfaces on this page:

| Error | When it occurs |
| --- | --- |
| `QisError::InvalidParameterValue` | When `Pauli` is constructed from a character or a string, the input is not `I`, `X`, `Y` or `Z`, or the input is an empty string or holds multiple characters. |
| `QisError::IndexOutOfBounds` | The qubit index of `try_get_pauli()` and `try_set_pauli()` is not smaller than `num_qubits`. |
| `QisError::DimensionMismatch` | The probability key length of `expectation()` is inconsistent with `num_qubits`. |
| `QisError::PauliStringParseError` | A probability key of `expectation()` contains a character other than `0` and `1`. |
| `PauliStringParseError::EmptyString` | The input is an empty string when parsing a `PauliString`. |
| `PauliStringParseError::InvalidCharacter` | A character other than `I`, `X`, `Y` and `Z` appears when parsing a `PauliString`. |
| `PauliStringParseError::NoOperators` | Only a phase prefix and no operator is present when parsing a `PauliString`, for example `"-"`. |

The following cases are expressed as a panic rather than a returned error: an out-of-range index in `set_pauli()` and `get_pauli()`, and inconsistent qubit counts on the two sides of `commutes_with()`, `Mul` and `MulAssign`. When out-of-range indices must be handled in error form, use `try_set_pauli()` and `try_get_pauli()` instead.

---

## Related pages

- [QIS Overview](0_overview.md): module overview and terminology.
- [Hamiltonian and Observable](7_hamiltonian.md): construct an observable from Pauli strings and coefficients and compute its expectation value.
