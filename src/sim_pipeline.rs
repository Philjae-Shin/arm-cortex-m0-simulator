/// Stage 3-2 / 3-3: Pipelined processor simulator with multiple execution units.
///
/// Models a 5-stage pipeline: IF → ID → EX → MEM → WB.
///
/// Features:
///   - Data hazard detection with pipeline stalling (and optional forwarding)
///   - CPSR (flag) hazard detection for conditional branches
///   - Control hazard handling via flush on mispredicted branch
///   - Configurable number of execution units (1 = scalar pipeline, >1 = superscalar)
///   - Sub-pipelined execution units with per-opcode latency
///   - In-order retirement via sequence numbers (oldest-first EX→MEM promotion)

use crate::isa::*;
use crate::regs::RegFile;
use crate::memory::Memory;
use crate::execute::{exec, commit, ExecResult};
use crate::branch_pred::{BranchPredictor, PredictorKind};
use crate::stats::Stats;

// ---------------------------------------------------------------------------
// Pipeline register between stages
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct PipeSlot {
    instr: Option<Instruction>,
    result: Option<ExecResult>,
    remaining_cycles: u32, // for multi-cycle execute
    seq: u64,              // issue order (monotonically increasing) for in-order retirement
}

impl PipeSlot {
    fn empty() -> Self {
        PipeSlot {
            instr: None,
            result: None,
            remaining_cycles: 0,
            seq: 0,
        }
    }
    fn is_empty(&self) -> bool { self.instr.is_none() }
    fn flush(&mut self) { *self = PipeSlot::empty(); }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn run(
    program: &[Instruction],
    regs: &mut RegFile,
    mem: &mut Memory,
    num_exec_units: usize,
    predictor_kind: PredictorKind,
    forwarding_enabled: bool,
    verbose: bool,
) -> Stats {
    let mut stats = Stats::new();
    let mut pc: usize = 0;
    let mut halted = false;
    let mut issue_seq: u64 = 0; // monotonically increasing sequence counter

    // Pipeline registers
    let mut fetch_slot = PipeSlot::empty();
    let mut decode_slot = PipeSlot::empty();
    // Multiple execution units
    let mut exec_slots: Vec<PipeSlot> = vec![PipeSlot::empty(); num_exec_units];
    let mut mem_slot = PipeSlot::empty();
    let mut wb_slot = PipeSlot::empty();

    // Branch predictor
    let mut predictor = BranchPredictor::new(predictor_kind);

    while !halted {
        stats.cycles += 1;

        // Process stages in reverse order so that freed slots are available
        // for downstream propagation within the same cycle.

        // ===== WB Stage: commit =====
        if !wb_slot.is_empty() {
            if let Some(ref result) = wb_slot.result {
                commit(result, regs);
                stats.instructions_executed += 1;

                if result.halt { halted = true; }
                if result.did_mem_read { stats.mem_reads += 1; }
                if result.did_mem_write { stats.mem_writes += 1; }
            }
            if verbose {
                println!("[cyc {:>5}] WB   : {}", stats.cycles,
                    wb_slot.instr.as_ref().unwrap());
            }
        }
        wb_slot.flush();

        // ===== MEM → WB =====
        wb_slot = mem_slot.clone();
        mem_slot.flush();

        // ===== EX Stage: advance execution units, move completed to MEM =====
        // Enforce in-order retirement: among all completed instructions,
        // select the one with the smallest sequence number (oldest first).
        // This prevents out-of-order completion from corrupting eff_regs
        // when a newer instruction's wb_slot values overwrite an older one's.
        let mut completed_idx: Option<usize> = None;
        let mut completed_seq: u64 = u64::MAX;
        for (i, slot) in exec_slots.iter_mut().enumerate() {
            if slot.is_empty() { continue; }
            if slot.remaining_cycles > 1 {
                slot.remaining_cycles -= 1;
                continue;
            }
            // This unit completes this cycle — candidate for MEM promotion
            if mem_slot.is_empty() && slot.seq < completed_seq {
                completed_idx = Some(i);
                completed_seq = slot.seq;
            }
        }
        if let Some(idx) = completed_idx {
            mem_slot = exec_slots[idx].clone();
            exec_slots[idx].flush();
        }

        // ===== Build effective register file =====
        // Always include wb_slot values (models standard WB-write-first convention
        // where register file write happens before read in the same cycle).
        let mut eff_regs = regs.clone();
        if let Some(ref result) = wb_slot.result {
            for &(r, val) in &result.reg_writes { eff_regs.set(r, val); }
            if let Some(cpsr) = result.new_cpsr { eff_regs.set_cpsr_raw(cpsr); }
        }
        if forwarding_enabled {
            // Forward from MEM stage
            if let Some(ref result) = mem_slot.result {
                for &(r, val) in &result.reg_writes { eff_regs.set(r, val); }
                if let Some(cpsr) = result.new_cpsr { eff_regs.set_cpsr_raw(cpsr); }
            }
            // Forward from completed EX slots
            for slot in &exec_slots {
                if !slot.is_empty() && slot.remaining_cycles <= 1 {
                    if let Some(ref result) = slot.result {
                        for &(r, val) in &result.reg_writes { eff_regs.set(r, val); }
                        if let Some(cpsr) = result.new_cpsr { eff_regs.set_cpsr_raw(cpsr); }
                    }
                }
            }
        }

        // ===== Detect data hazards (between decode_slot and in-flight instrs) =====
        let mut stall_fetch_decode = false;
        if let Some(ref dec_instr) = decode_slot.instr {
            let sources = dec_instr.sources();
            let needs_flags = dec_instr.opcode.is_conditional_branch();

            // Check all in-flight execution slots for RAW hazards
            for slot in &exec_slots {
                if let Some(ref ex_instr) = slot.instr {
                    let dests = ex_instr.destinations();
                    // Register hazards
                    for src in &sources {
                        if dests.contains(src) {
                            if forwarding_enabled && slot.remaining_cycles <= 1 {
                                // Result available via forwarding — no stall
                            } else {
                                stall_fetch_decode = true;
                            }
                        }
                    }
                    // CPSR hazard: conditional branch depends on flags
                    if needs_flags && ex_instr.opcode.updates_flags() {
                        if forwarding_enabled && slot.remaining_cycles <= 1 {
                            // Flags available via forwarding path
                        } else {
                            stall_fetch_decode = true;
                        }
                    }
                }
            }
            // Also check MEM slot (register + CPSR hazards)
            if let Some(ref mem_instr) = mem_slot.instr {
                let dests = mem_instr.destinations();
                for src in &sources {
                    if dests.contains(src) && !forwarding_enabled {
                        stall_fetch_decode = true;
                    }
                }
                if needs_flags && mem_instr.opcode.updates_flags() && !forwarding_enabled {
                    stall_fetch_decode = true;
                }
            }
            // wb_slot: values always available via eff_regs (no stall needed)
        }

        // ===== ID → EX (issue) =====
        if !stall_fetch_decode {
            if let Some(ref instr) = decode_slot.instr {
                let mut issued = false;
                for slot in exec_slots.iter_mut() {
                    if slot.is_empty() {
                        let mut new_slot = decode_slot.clone();
                        new_slot.remaining_cycles = instr.opcode.latency();
                        new_slot.seq = issue_seq;
                        issue_seq += 1;

                        // Execute the instruction using effective (forwarded) registers
                        let result = exec(instr, &eff_regs, mem);

                        // Branch handling
                        if instr.opcode.is_branch() {
                            stats.branches_total += 1;
                            let predicted = predictor.predict(instr.addr);
                            let actual = result.branch_taken;

                            if actual { stats.branches_taken += 1; }

                            if predicted == actual {
                                stats.branches_predicted += 1;
                            } else {
                                // Misprediction: flush speculative fetch, correct PC
                                stats.branches_mispredicted += 1;
                                stats.pipeline_flushes += 1;
                                fetch_slot.flush();

                                if actual {
                                    // Predicted not-taken but actually taken → redirect to target
                                    if let Some(target) = result.branch_target {
                                        pc = target;
                                    }
                                } else {
                                    // Predicted taken but actually not taken → fall through
                                    pc = instr.addr + 1;
                                }
                            }

                            predictor.update(instr.addr, actual);
                        }

                        new_slot.result = Some(result);
                        *slot = new_slot;
                        issued = true;
                        break;
                    }
                }
                if issued {
                    decode_slot.flush();
                } else {
                    // All execution units busy — structural hazard, stall
                    stall_fetch_decode = true;
                    stats.pipeline_stalls += 1;
                }
            }
        } else {
            stats.pipeline_stalls += 1;
        }

        // ===== IF → ID =====
        if !stall_fetch_decode && decode_slot.is_empty() {
            decode_slot = fetch_slot.clone();
            fetch_slot.flush();
        }

        // ===== Fetch =====
        if !stall_fetch_decode && fetch_slot.is_empty() {
            if pc < program.len() {
                let instr = program[pc].clone();
                fetch_slot = PipeSlot {
                    instr: Some(instr),
                    result: None,
                    remaining_cycles: 0,
                    seq: 0,
                };
                stats.instructions_fetched += 1;

                // Speculatively advance PC (Bx uses register, can't predict target)
                if program[pc].opcode.is_branch() && program[pc].opcode != Opcode::Bx {
                    let predicted = predictor.predict(pc);
                    if predicted {
                        pc = program[pc].imm as usize;
                    } else {
                        pc += 1;
                    }
                } else {
                    pc += 1;
                }
            }
        }

        // Termination checks
        if halted { break; }
        let all_empty = fetch_slot.is_empty()
            && decode_slot.is_empty()
            && exec_slots.iter().all(|s| s.is_empty())
            && mem_slot.is_empty()
            && wb_slot.is_empty();
        if all_empty && pc >= program.len() { break; }

        if stats.cycles > 1_000_000 {
            eprintln!("[WARN] Cycle limit reached (1,000,000). Stopping.");
            break;
        }
    }

    if verbose {
        println!("{}", regs);
    }

    stats
}
