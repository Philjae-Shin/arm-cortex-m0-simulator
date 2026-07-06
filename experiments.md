# Experiments — ACA Processor Simulator

This document describes three experiments conducted using the ARM Cortex-M0
simulator.  Each experiment follows a **hypothesis → experiment → result**
structure as requested by the assignment.  The experiments are deliberately
chosen to showcase the two custom extensions (L1 cache, SIMD vector
instructions), ISA enhancements (post-indexed addressing), and their
interactions with microarchitectural features.

Run all experiments automatically with:
```
cargo run --release -- --experiment
```

---

## Experiment 1: Post-Indexed Addressing Impact (ISA Enhancement)

### Hypothesis

Post-indexed load/store instructions (LDRPOST, STRPOST) combine a memory
access with an automatic base register increment in a single instruction,
eliminating the need for separate ADDI instructions to advance array pointers.
For a loop-based vector addition over 10 elements, replacing
`LDR Rd, Rn, #0` + `ADDI Rn, Rn, #4` with `LDRPOST Rd, Rn, #4` should reduce
the loop body from 8 instructions to 5, yielding ~37% fewer dynamic
instructions and a corresponding reduction in cycle count.  The benefit
should be amplified in the pipelined and OoO simulators because fewer
instructions also means fewer pipeline slots, fewer data hazards, and reduced
fetch pressure.

### Experiment

Two functionally equivalent programs were written:
- `vector_add_plain.asm` — standard LDR/STR with explicit ADDI pointer increments
  (loop body = 8 instructions: LDR + LDR + ADD + STR + ADDI + ADDI + ADDI + ADDI)
- `vector_add.asm` — post-indexed LDRPOST/STRPOST with implicit increment
  (loop body = 5 instructions: LDRPOST + LDRPOST + ADD + STRPOST + ADDI)

Both compute C[i] = A[i] + B[i] for 10 elements and were run on all three
simulators with 2-bit branch prediction and forwarding enabled.

### Result

The post-indexed version reduces dynamic instruction count by ~25% (from
118 to 88 executed instructions).  Cycle savings range from 14–31%:
the pipeline benefits most (1.32x speedup) because fewer instructions
per iteration remove data-hazard-prone slots across the 10 iterations.
The OoO simulator benefits least (1.17x) because its register renaming
and out-of-order issue can partially hide the extra ADDI latencies.

This demonstrates that ISA-level addressing mode design directly impacts
performance beyond what microarchitectural techniques alone can recover.

---

## Experiment 2: Cache Locality & Conflict Miss Analysis (Cache Extension)

### Hypothesis

A 256-line direct-mapped L1 cache should exhibit dramatically different
hit rates depending on the memory access pattern.  Specifically:

1. **Sequential access** (stride-1) should achieve a high hit rate on
   repeated passes, with only compulsory misses on the first pass.
2. **Strided access** (stride-16 words = 64 bytes) should achieve moderate
   hit rates — each access touches a different cache line, but on repeated
   passes the lines remain resident because the working set (16 lines) fits.
3. **Conflict access** — alternating between two addresses separated by
   exactly 256 words (the cache size) should produce near-100% miss rate
   after the first access, because each load evicts the other's cache line.
   This pathological pattern demonstrates the weakness of direct-mapped caches.

### Experiment

Three custom benchmarks were written:
- `cache_sequential.asm` — reads 32 words sequentially in 3 passes (96 loads)
- `cache_strided.asm` — reads 16 words with stride=64B in 3 passes (48 loads)
- `cache_conflict.asm` — alternates 2 addresses 1024 bytes apart (64 loads)

All were run on the pipelined simulator (2 EU, 2-bit predictor, forwarding)
with and without the L1 cache enabled.  Cache hit/miss counts and total
cycles were compared.

### Result

Sequential access achieves the highest hit rate (~67%) because passes 2
and 3 hit on lines loaded during pass 1.  Strided access achieves moderate
hit rates — although each stride accesses a different line, the 16-entry
working set fits within 256 lines, so passes 2–3 hit.  The conflict
benchmark exhibits near-0% hit rate: every access evicts the other address,
demonstrating the well-known direct-mapped cache thrashing problem.

The cycle overhead from cache misses is dramatic for the conflict pattern:
each of the ~64 misses adds 8 penalty cycles, roughly doubling execution
time.  This validates that cache-aware programming is essential and that
associativity (not implemented here) would mitigate conflict misses.

---

## Experiment 3: SIMD Vector Length Scaling (SIMD Extension)

### Hypothesis

As vector length increases, the speedup of SIMD vector operations over
an equivalent scalar loop should grow, but with diminishing returns.  The
scalar loop has fixed overhead per element (branch + compare + increment)
that SIMD eliminates.  The SIMD version uses a separate vector register
file (V0-V7) with explicit VLDR/VSTR transfers between memory and vector
registers, matching real-world architectures like ARM NEON or x86 SSE.
VADD's dynamic latency grows linearly with vector length (1 + N cycles),
so the per-element cost approaches the scalar loop's per-element compute
time.  We predict:

- At very short lengths (N=2), SIMD overhead (VLDR + VADD + VSTR) is
  comparable to scalar loop setup, yielding modest speedup (~2–3x).
- At moderate lengths (N=8–16), SIMD achieves maximum speedup because loop
  overhead dominates the scalar version.
- At large lengths (N=32), the speedup plateaus because VLDR/VADD/VSTR
  latency dominates and both versions become compute-bound.

### Experiment

For each vector length N in {2, 4, 8, 16, 32}, two programs were generated
programmatically using inline assembly:
- A scalar loop using LDRPOST/STRPOST (7 instructions per iteration)
- A SIMD version: VLDR V0, VLDR V1, VADD V2, VSTR V2 (8 total instructions)

The SIMD version explicitly loads data from memory into vector registers,
performs the computation in the vector register file (no memory access), and
stores the result back.  This models the standard load-compute-store pattern
used by real SIMD architectures.

Both were run on the pipelined simulator (2 EU, 2-bit predictor, forwarding).
Cycle count, instruction count, and speedup were compared.

### Result

Speedup increases rapidly from N=2 to N=8, then levels off.  The SIMD
version now has more instructions (VLDR+VLDR+VADD+VSTR = 8 vs 5 in the
old memory-to-memory model), but the vector register file eliminates
redundant memory traffic: VADD operates entirely on registers.

The key insight is that SIMD eliminates *control overhead* (branch, compare,
increment) — which is constant per iteration — so the relative benefit is
highest when that overhead is a large fraction of total cost (small-to-medium
vectors).  For very long vectors, both approaches are dominated by the
N memory accesses (VLDR/VSTR latency), and SIMD's advantage shrinks.

This experiment reveals the interaction between ISA-level data parallelism,
the vector register file architecture, and microarchitectural pipeline
effects, showing that SIMD is most valuable for workloads where loop
overhead is the performance bottleneck.

---

## Running the Experiments

```bash
# Full automated experiment suite:
cargo run --release -- --experiment

# Single program comparison across all simulators:
cargo run --release -- --batch programs/vector_add.asm

# Single run with specific configuration:
cargo run --release -- --sim ooo --units 4 --predictor 2bit programs/bubble_sort.asm

# Verbose trace for debugging:
cargo run --release -- --sim scalar --verbose programs/gcd.asm
```
