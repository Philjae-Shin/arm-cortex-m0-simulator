Simulator for an ARM Cortex-M0 Thumb instruction subset,

Three simulation modes are supported:
  1. Scalar      - Non-pipelined, single-issue (fetch-decode-execute)
  2. Pipeline    - In-order, 5-stage (IF-ID-EX-MEM-WB) with multiple EX units
  3. OoO         - Out-of-order with Tomasulo algorithm, ROB, and register renaming

Prerequisites
-------------
  - Rust toolchain (rustc + cargo), edition 2021 or later
    Install from: https://rustup.rs

Build
-----
  make                     (uses the included Makefile)
  or:
  cargo build --release    (directly with cargo)

Run
---
  cargo run --release -- [OPTIONS] <program.asm>

Options:
  --sim <mode>        Simulator mode: scalar | pipeline | ooo  (default: scalar)
  --units <N>         Number of execution units / reservation stations (default: 2)
  --predictor <kind>  Branch predictor: static | 1bit | 2bit  (default: static)
  --forwarding        Enable data forwarding in pipelined mode
  --cache             Enable L1 data cache simulation
  --verbose           Print per-cycle trace for debugging
  --batch             Run all three simulators on one program and compare
  --experiment        Run the full experiment suite across all benchmark programs

Examples
--------
  # Run GCD on the scalar simulator
  cargo run --release -- src/programs/gcd.asm

  # Run fibonacci on the OoO simulator with 4 units and 2-bit predictor
  cargo run --release -- src/programs/fibonacci.asm --sim ooo --units 4 --predictor 2bit

  # Compare all three modes on bubble_sort
  cargo run --release -- src/programs/bubble_sort.asm --batch

  # Run the full experiment suite (results for all programs x configs)
  cargo run --release -- --experiment

Project Structure
-----------------
  src/
    main.rs          CLI, argument parsing, experiment runner
    isa.rs           Instruction set definition (41 ARM Thumb opcodes)
    assembler.rs     Two-pass assembler (labels, pseudo-instructions)
    memory.rs        Byte-addressable memory model
    regs.rs          Register file (R0-R12, SP, LR, PC, CPSR)
    execute.rs       ALU execution and flag computation
    branch_pred.rs   Static, 1-bit, and 2-bit saturating counter predictors
    cache.rs         L1 direct-mapped data cache simulation
    stats.rs         Cycle/instruction/IPC statistics collector
    sim_scalar.rs    Non-pipelined scalar simulator
    sim_pipeline.rs  In-order pipelined simulator (configurable EX units)
    sim_ooo.rs       Out-of-order simulator (Tomasulo + ROB + CDB)

  programs/
    gcd.asm                Euclidean GCD algorithm
    fibonacci.asm          Fibonacci sequence computation
    factorial.asm          Recursive factorial
    bitcount.asm           Population count (bit counting)
    vector_add.asm         Vector addition (10 elements)
    bubble_sort.asm        Bubble sort on a small array
    vector_add_plain.asm   Vector add without post-indexed (baseline)
    vector_ops.asm         SIMD vector operations (VADD, VMUL, VDOT)
    vector_ops_scalar.asm  Scalar-loop equivalent for SIMD comparison
    cache_sequential.asm   Sequential access pattern benchmark
    cache_strided.asm      Strided access pattern benchmark
    cache_conflict.asm     Cache conflict miss benchmark

Key Features
------------
  - Full ARM Cortex-M0 Thumb subset: 41 opcodes (arithmetic, memory, control flow, SIMD)
  - Post-indexed load/store (LDRPOST/STRPOST) for auto-increment loops
  - Separate vector register file (V0-V7) for SIMD operations
  - Vector SIMD instructions: VLDR/VSTR (memory↔vector register), VADD, VSUB, VMUL, VDOT
  - Vector Register Alias Table (VRAT) for independent vector register renaming in OoO
  - Custom two-pass assembler with label resolution and pseudo-instruction support
  - 5-stage pipeline with hazard detection (RAW data + CPSR flag hazards)
  - Sequence-number based in-order retirement for pipeline correctness
  - Tomasulo out-of-order execution with ROB, RAT, CDB broadcast
  - Speculative execution past branches with flush-on-misprediction
  - Three branch prediction strategies (static, 1-bit, 2-bit saturating)
  - Configurable 1-6 execution units for scaling experiments
  - L1 direct-mapped data cache simulation (256 lines, --cache flag)
  - Built-in experiment suite for automated benchmarking
