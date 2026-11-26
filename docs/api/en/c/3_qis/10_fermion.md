# Fermionic algebra and Jordan–Wigner (C)

`CFermionicOp` is a sparse complex sum of ordered ladder products. Modes are zero-based; in `A B`, B acts first. Identical ordered products merge and only exact zeros disappear. No implicit normal ordering or numerical pruning occurs. Zero and identity retain the declared mode count, including zero modes.

## ABI and data

New `int32_t` functions return 0 on success. Objects use `**out` and scalar queries use output pointers; errors leave outputs and mutable inputs unchanged. Results are independently owned and freed with `fermionic_op_free`; inputs are borrowed and copied. Pass an empty output slot and valid, aligned, live storage. Output slots must not overlap one another or handle internals, and a handle must not be read concurrently with mutation. Empty arrays allow NULL; nonempty arrays require sufficient storage.

Errors: -1 required NULL pointer; -2 mode/term index out of bounds; -8 invalid tag, mode-count mismatch, nonfinite coefficient/arithmetic overflow, invalid tolerance, insufficient capacity or unrepresentable array length; -7 unsupported Pauli representation width. Predicates write 0/1 to `uint8_t *out`; their successful return code is always 0.

```c
typedef struct CFermionicOp CFermionicOp;
#define FERMIONIC_CREATE 0
#define FERMIONIC_ANNIHILATE 1
typedef struct CFermionicLadder {
    size_t mode;
    uint8_t kind;
} CFermionicLadder;
typedef struct CFermionicTerm {
    const CFermionicLadder *ladders;
    size_t len;
    Complex64 coefficient;
} CFermionicTerm;
```

## Construction

```c
int32_t fermionic_op_new(size_t num_modes, CFermionicOp **out);

int32_t fermionic_op_identity(size_t num_modes, CFermionicOp **out);

int32_t fermionic_op_from_term(size_t num_modes,
                               const CFermionicLadder *term,
                               size_t len,
                               Complex64 coefficient,
                               CFermionicOp **out);

int32_t fermionic_op_from_terms(size_t num_modes,
                                const CFermionicTerm *terms,
                                size_t len,
                                CFermionicOp **out);

int32_t fermionic_op_creation(size_t num_modes, size_t mode, CFermionicOp **out);

int32_t fermionic_op_annihilation(size_t num_modes, size_t mode, CFermionicOp **out);

int32_t fermionic_op_number(size_t num_modes, size_t mode, CFermionicOp **out);

int32_t fermionic_op_total_number(size_t num_modes, CFermionicOp **out);

int32_t fermionic_op_excitation_generator(size_t num_modes,
                                          const size_t *sources,
                                          size_t sources_len,
                                          const size_t *targets,
                                          size_t targets_len,
                                          CFermionicOp **out);
```

- `new` creates zero and `identity` creates one. An empty `from_term` product is identity; an empty `from_terms` sum is zero. Batch descriptors and nested ladder arrays are borrowed only during the call. Even zero-weight products validate tags and modes.
- `creation/annihilation/number` construct a†, a, and a†a; `total_number` sums the number operator over all modes.
- Excitation generates G=i(T−T†), with target creators in input order and source annihilators in reverse order. Lists must be nonempty, equally long, individually unique and disjoint. An odd permutation changes the sign. No sorting, factorial or spin-orbital convention is imposed.

## Lifetime and queries

```c
void fermionic_op_free(CFermionicOp *ptr);

int32_t fermionic_op_copy(const CFermionicOp *ptr, CFermionicOp **out);

int32_t fermionic_op_num_modes(const CFermionicOp *ptr, size_t *out);

int32_t fermionic_op_num_terms(const CFermionicOp *ptr, size_t *out);

int32_t fermionic_op_is_zero(const CFermionicOp *ptr, uint8_t *out);

int32_t fermionic_op_equals(const CFermionicOp *left,
                            const CFermionicOp *right,
                            uint8_t *out);

int32_t fermionic_op_term_len(const CFermionicOp *ptr, size_t index, size_t *out);

int32_t fermionic_op_get_term(const CFermionicOp *ptr,
                              size_t index,
                              CFermionicLadder *buffer,
                              size_t capacity,
                              Complex64 *out_coefficient);
```

`free(NULL)` is valid; `copy` is independent. `is_zero` tests empty storage and `equals` compares stored structure and mode count (different mode counts return false). Use `equiv` for algebraic equality.

Call `term_len`, allocate a buffer, then call `get_term`. Capacity may exceed the required length; trailing elements stay untouched. Empty products allow NULL buffer. Terms follow core deterministic structural order; requery lengths and indices after mutation. Output arrays are copies and contain no library-owned pointers to release.

## Algebra

```c
int32_t fermionic_op_add_term(CFermionicOp *ptr,
                              const CFermionicLadder *term,
                              size_t len,
                              Complex64 coefficient);

int32_t fermionic_op_add(const CFermionicOp *left,
                         const CFermionicOp *right,
                         CFermionicOp **out);

int32_t fermionic_op_sub(const CFermionicOp *left,
                         const CFermionicOp *right,
                         CFermionicOp **out);

int32_t fermionic_op_mul(const CFermionicOp *left,
                         const CFermionicOp *right,
                         CFermionicOp **out);

int32_t fermionic_op_scaled(const CFermionicOp *ptr,
                            Complex64 factor,
                            CFermionicOp **out);

int32_t fermionic_op_adjoint(const CFermionicOp *ptr, CFermionicOp **out);

int32_t fermionic_op_normal_order(const CFermionicOp *ptr, CFermionicOp **out);

int32_t fermionic_op_chop(CFermionicOp *ptr, double atol);

int32_t fermionic_op_equiv(const CFermionicOp *left,
                           const CFermionicOp *right,
                           double atol,
                           uint8_t *out);

int32_t fermionic_op_is_hermitian(const CFermionicOp *ptr, double atol, uint8_t *out);
```

`add_term/chop` mutate in place; other object operations return copies. `mul` does not normal order. `adjoint` conjugates coefficients, reverses products and exchanges ladder kinds. `normal_order` places creators before annihilators with ascending modes within each group, including anticommutation contractions.

`chop` removes coefficients with magnitude <= atol. `equiv` normal-orders the difference and compares coefficients, requiring equal mode counts; `is_hermitian` compares with the adjoint. atol must be finite and nonnegative; explicitly pass 0 for exact comparison. This is a coefficient tolerance, not a matrix operator-norm bound.

## Jordan–Wigner

```c
int32_t jordan_wigner_map_operator(const CFermionicOp *ptr, CHamiltonian **out);

int32_t jordan_wigner_num_qubits(size_t num_modes, size_t *out);

int32_t jordan_wigner_map_occupation(const uint8_t *occupation,
                                     size_t len,
                                     uint8_t *out_bits,
                                     size_t capacity);
```

Mapping is stateless and needs no mapper handle. Mode p maps to qubit p with Z on lower modes: creation = Z-chain (X−iY)/2 and annihilation = Z-chain (X+iY)/2. Qubit 0 is the least significant bit and rightmost displayed Pauli character.

`map_operator` returns an independent `CHamiltonian`, freed with `hamiltonian_free`. Non-Hermitian inputs are valid; downstream expectation and evolution functions check Hermiticity. Mapping builds no dense matrix, but a product of k ladders may expand to 2^k terms before merging.

`map_occupation` accepts only 0/1 bytes. Input length defines the mode count; output bits[p] addresses qubit p. Capacity must be at least len; input and output may overlap. Empty input is zero-mode vacuum.

## Example

```c
#include "cqlib_c.h"

int main(void) {
    const size_t sources[] = {0}, targets[] = {1};
    CFermionicOp *g = NULL;
    CHamiltonian *h = NULL;
    if (fermionic_op_excitation_generator(2, sources, 1, targets, 1, &g) != 0) {
        return 1;
    }
    if (jordan_wigner_map_operator(g, &h) != 0) {
        fermionic_op_free(g);
        return 1;
    }
    CCircuit *u = hamiltonian_to_evolution_circuit(
        h, 0.37, 1, TROTTER_MODE_SECOND_ORDER);
    const int result = u == NULL ? 1 : 0;
    circuit_free(u);
    hamiltonian_free(h);
    fermionic_op_free(g);
    return result;
}
```
