# Hamiltonian

`CHamiltonian` represents a Hermitian observable `H = Σ c_i P_i` (a linear combination of Pauli strings with complex coefficients). Together with the `expectation` interfaces of [Statevector](1_statevector.md#probabilities-and-measurement) / [DensityMatrix](2_density_matrix.md) it computes `⟨H⟩`, and it can also serve as the observed object of [error mitigation](../6_error_mitigation/1_error_mitigation.md).

---

## Functions

### hamiltonian_new(num_qubits)

Creates an empty Hamiltonian acting on n qubits (0 terms).

Returns:

- `CHamiltonian *`: a heap-allocated object, released with `hamiltonian_free`; returns NULL on failure.

### hamiltonian_from_pauli(pauli)

Constructs from a single Pauli string (coefficient 1.0).

**Ownership**: this function **takes over and releases** `pauli`; the object must not be used after the call returns (and must not be released a second time).

Returns:

- `CHamiltonian *`; returns NULL on failure.

### hamiltonian_add_term(ptr, pauli, coeff_re, coeff_im)

Appends the complex-coefficient term `coeff * pauli`, where `coeff = coeff_re + coeff_im * i`.

**Ownership**: the Pauli string is **cloned**; the ownership of `pauli` stays with the caller, who must still call `pauli_string_free` afterwards.

Parameters:

- `pauli` (`const CPauliString *`): the Pauli string.
- `coeff_re`, `coeff_im` (`double`): the real and imaginary parts of the coefficient.

Returns:

- `int32_t`; `0` on success, `-8` when the number of qubits of `pauli` does not match that of the Hamiltonian.

### hamiltonian_simplify(ptr)

**In-place** simplification: merges terms with the same Pauli string and removes terms with a near-zero coefficient.

Returns:

- `int32_t`; `0` on success.

### hamiltonian_num_qubits(ptr) / hamiltonian_num_terms(ptr)

Return the number of qubits / the number of terms respectively; a NULL handle returns `0`.

### hamiltonian_free(ptr)

Releases the object. Accepts NULL.

---

## Example

### Constructing H = 0.5·Z₀ + 0.5·Z₁ + 0.25·X₀X₁

```c
CPauliString *z0 = pauli_string_parse("ZI");   // 首字符为最高位
CPauliString *z1 = pauli_string_parse("IZ");
CPauliString *xx = pauli_string_parse("XX");

CHamiltonian *h = hamiltonian_new(2);
hamiltonian_add_term(h, z0, 0.5, 0.0);
hamiltonian_add_term(h, z1, 0.5, 0.0);
hamiltonian_add_term(h, xx, 0.25, 0.0);
hamiltonian_simplify(h);

printf("terms=%zu\n", (size_t)hamiltonian_num_terms(h));   // 3

/* <H> = <psi|H|psi> */
CStatevector *sv = statevector_from_circuit(qc);
double energy;
statevector_expectation(sv, h, &energy);
printf("<H> = %f\n", energy);

statevector_free(sv);

hamiltonian_free(h);   // add_term 克隆了 Pauli 串，所有权仍在调用方
pauli_string_free(z0);
pauli_string_free(z1);
pauli_string_free(xx);
```

### Ownership transfer of from_pauli

```c
CPauliString *z = pauli_string_parse("Z");
CHamiltonian *h = hamiltonian_from_pauli(z);   // z 已被接管并释放
/* 此处不得再使用 z */

hamiltonian_free(h);
```

After the same Pauli string is added repeatedly, `simplify` merges the coefficients (for example two `0.5·Z` terms merge into `1.0·Z`), which can be used to accumulate energy terms.
