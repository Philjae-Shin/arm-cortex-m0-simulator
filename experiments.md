# Experiments — ACA Processor Simulator

This document describes five experiments conducted using the ARM Cortex-M0
simulator.  Each experiment follows a **hypothesis → experiment → result**
structure as requested by the assignment.

Run all experiments automatically with:
```
cargo run --release -- --experiment
```

---

## Experiment 1: Pipeline Speedup over Scalar Execution

### Hypothesis

A 5-stage pipelined processor should achieve a speedup of up to 3× over a
non-pipelined scalar processor for straight-line code, because the scalar
processor spends 3 cycles per instruction (fetch + decode + execute) whereas
the pipeline overlaps these stages.  Branch-heavy programs (gcd, bubble_sort)
should see less benefit because taken branches flush the pipeline.

### Experiment

All seven benchmarks were executed on both the scalar simulator and the
pipelined simulator (1 execution unit, static branch prediction, no
forwarding). Total cycle count was recorded and
speedup = scalar_cycles / pipeline_cycles.

### Result

| Program         | Scalar | Pipeline | Speedup |
|----------------|--------|----------|---------|
| fibonacci       | 423    | 187      | 2.26×   |
| bitcount        | 585    | 297      | 1.97×   |
| vector_add      | 384    | 196      | 1.96×   |
| vector_add_unrolled | 228 | 124      | 1.84×   |
| gcd             | 93     | 52       | 1.79×   |
| factorial       | 311    | 175      | 1.78×   |
| bubble_sort     | 1213   | 701      | 1.73×   |

fibonacci achieves the best speedup (2.26×) because it has a balanced mix
of arithmetic with relatively few data dependencies.  bubble_sort shows the
lowest speedup (1.73×) due to heavy data dependencies from its
comparison-and-swap pattern causing frequent pipeline stalls.

---

## Experiment 2: Branch Prediction Comparison

### Hypothesis

Dynamic branch predictors (1-bit, 2-bit saturating counter) should
outperform the static (always not-taken) predictor on programs with regular
loop structures.  The 2-bit predictor should be most robust because it
requires two consecutive mispredictions to flip state.

### Experiment

All seven benchmarks were run on the pipelined simulator (2 execution
units) with each of the three branch predictors: static, 1-bit, and 2-bit.
Total cycle count and prediction accuracy (correct / total predictions)
were recorded.

### Result

| Program     | Static         | 1-bit          | 2-bit          |
|------------|----------------|----------------|----------------|
| fibonacci   | 187 (49%)      | 169 (95%)      | 169 (95%)      |
| bitcount    | 297 (49%)      | 267 (97%)      | 267 (97%)      |
| vector_add  | 176 (48%)      | 176 (90%)      | 176 (90%)      |
| vector_add_unrolled | 108 (44%) | 108 (78%)    | 108 (78%)      |
| factorial   | 166 (30%)      | 119 (79%)      | 119 (79%)      |
| bubble_sort | 647 (45%)      | 632 (71%)      | 621 (81%)      |
| gcd         | 52 (44%)       | 49 (62%)       | 50 (56%)       |

Dynamic predictors dramatically improve accuracy for loop-heavy programs —
bitcount reaches 97% with both 1-bit and 2-bit.  The notable outlier is
GCD: the 2-bit predictor (50 cycles, 56%) actually performs slightly worse
than the 1-bit predictor (49 cycles, 62%).  This is because GCD's Euclidean
algorithm produces irregular, data-dependent branch patterns.  The 2-bit
predictor's hysteresis (requiring two mispredictions to switch) hurts when
the pattern changes frequently.

---

## Experiment 3: Execution Unit Scaling (Out-of-Order)

### Hypothesis

Increasing the number of reservation stations / execution units from 1 to 6
in the out-of-order simulator should reduce cycle count by exposing
instruction-level parallelism (ILP).  However, diminishing returns should
be observed beyond 3–4 units because the inherent ILP in our benchmarks is
limited by true data dependencies.

### Experiment

All seven benchmarks were run on the out-of-order simulator with 1, 2, 3, 4,
and 6 reservation stations, using a 2-bit branch predictor. Cycle counts
were recorded.

### Result

| Program     | 1 unit | 2 units | 3 units | 4 units | 6 units |
|------------|--------|---------|---------|---------|---------|
| gcd         | 72     | 46      | 38      | 37      | 36      |
| fibonacci   | 287    | 148     | 109     | 89      | 80      |
| bitcount    | 395    | 202     | 139     | 108     | 107     |
| vector_add  | 271    | 145     | 104     | 84      | 68      |
| vector_add_unrolled | 165 | 91      | 66      | 55      | 46      |
| factorial   | 168    | 91      | 70      | 65      | 55      |
| bubble_sort | 859    | 461     | 353     | 325     | 287     |

Confirmed — OoO scaling follows Amdahl's Law.  The biggest gain is 1→2
units (~2× speedup across the board).  From 2→4 units, ~1.5× additional
speedup.  From 4→6 units, only ~1.1× — diminishing returns as data
dependencies limit extractable ILP.  vector_add shows the largest total
improvement (3.99× at 6 units), while bubble_sort is lower (2.99×),
reflecting different available ILP in each workload.

---

## Experiment 4: Loop Unrolling Benefit

### Hypothesis

The 2× unrolled vector addition (`vector_add_unrolled.asm`) should
outperform the standard version by reducing branch overhead and increasing
ILP.  All execution modes should benefit, with the scalar simulator gaining
most from reduced branch cost since it pays the full penalty per branch.

### Experiment

Both `vector_add.asm` and `vector_add_unrolled.asm` were run on all three
simulators: scalar, pipelined (2 units, 2-bit predictor), and out-of-order
(4 units, 2-bit predictor).  We compared cycle counts, IPC, and branch
counts.

### Result

| Version / Simulator         | Cycles | IPC    | Branches |
|----------------------------|--------|--------|----------|
| vector_add / scalar         | 384    | 0.307  | 21       |
| vector_add / pipeline       | 176    | 0.671  | 21       |
| vector_add / ooo-4          | 84     | 1.405  | 21       |
| vector_add_unrolled / scalar | 228   | 0.298  | 9        |
| vector_add_unrolled / pipeline | 108 | 0.630  | 9        |
| vector_add_unrolled / ooo-4 | 55     | 1.236  | 9        |

Loop unrolling reduces cycles by 34–41% across all modes.  Branches decrease
from 21 to 9 (−57%).  Scalar benefits most (40.6%) because it pays the full
cost per branch.  OoO benefits least (34.5%) because speculative execution
already hides some branch penalty.  This demonstrates that the interaction
between hardware features and compiler optimisations is not always intuitive.

---

## Experiment 5: Data Forwarding Impact

### Hypothesis

Data forwarding (bypassing) should eliminate most pipeline stalls by routing
EX results directly to dependent instructions, avoiding the need to wait for
writeback.  Programs with dense data dependencies (e.g. bubble_sort's
compare-and-swap pattern) should benefit most.

### Experiment

All seven benchmarks were run on the pipelined simulator (2 execution units,
2-bit branch predictor) with forwarding disabled and then enabled.  Total
cycle counts and data-hazard stall counts were recorded.

### Result

| Program             | No Fwd | Stalls | Fwd  | Stalls | Δ Cycles |
|--------------------|--------|--------|------|--------|----------|
| bubble_sort         | 621    | 219    | 402  | 0      | −35.3%   |
| factorial           | 119    | 38     | 84   | 2      | −29.4%   |
| vector_add          | 176    | 51     | 125  | 0      | −29.0%   |
| vector_add_unrolled | 108    | 29     | 79   | 0      | −26.9%   |
| bitcount            | 267    | 65     | 202  | 0      | −24.3%   |
| gcd                 | 50     | 7      | 43   | 0      | −14.0%   |
| fibonacci           | 169    | 21     | 148  | 0      | −12.4%   |

Forwarding eliminates 90–100% of data-hazard stalls, reducing cycles by
12–35%.  bubble_sort benefits most (219 stalls → 0, −35.3%) due to its dense
compare-and-swap dependencies.  Programs with fewer data hazards (gcd,
fibonacci) show smaller gains as their bottleneck is branch mispredictions
rather than data stalls.  With forwarding + 2-bit prediction, bitcount
achieves near-ideal CPI of 1.04.

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
