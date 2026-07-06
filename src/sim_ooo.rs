/// Stage 4: Out-of-order superscalar simulator with Tomasulo-style scheduling.
///
/// Key micro-architectural features:
///   - **Reservation stations** — instructions wait here until operands are ready
///   - **Re-order buffer (ROB)** — ensures in-order retirement for precise exceptions
///   - **Register renaming** — ROB entries serve as physical registers, eliminating
///     WAR and WAW hazards
///   - **Common data bus (CDB)** — broadcasts completed results to all waiting
///     reservation stations (including CPSR forwarding for conditional branches)
///   - **Multiple functional units** — ALU, Load/Store, Branch (configurable count)
///   - **Branch prediction** — speculative fetch; misprediction triggers flush
///
/// Pipeline stages:  Fetch → Decode/Rename → Issue → Execute → Complete → Retire

use crate::isa::*;
use crate::regs::RegFile;
use crate::memory::Memory;
use crate::execute::{exec, ExecResult};
use crate::branch_pred::{BranchPredictor, PredictorKind};
use crate::stats::Stats;
use crate::cache::{Cache, MISS_PENALTY};
use std::collections::VecDeque;

// == ROB entry

#[derive(Clone, Debug)]
enum RobState {
    Issued,                  // waiting in reservation station
    Executing,               // in functional unit
    Complete(ExecResult),    // execution done, waiting for retirement
}

#[derive(Clone, Debug)]
struct RobEntry {
    id: usize,
    instr: Instruction,
    state: RobState,
    dest_regs: Vec<usize>,   // architectural GP destination registers
    dest_vregs: Vec<usize>,  // architectural vector destination registers
}

// == Reservation station entry

#[derive(Clone, Debug)]
struct RSEntry {
    rob_id: usize,
    instr: Instruction,
    /// Source operands: Some(value) = ready, None = waiting for ROB tag
    src_vals: Vec<Option<u32>>,
    /// ROB tags for pending sources
    src_tags: Vec<Option<usize>>,
    /// Whether this entry is currently in a functional unit
    executing: bool,
    remaining_cycles: u32,
    // CPSR dependency tracking (for conditional branches)
    cpsr_ready: bool,
    cpsr_tag: Option<usize>,   // ROB id of flag-producing instruction
    cpsr_val: Option<u32>,     // forwarded CPSR value
}

impl RSEntry {
    fn all_sources_ready(&self) -> bool {
        self.src_vals.iter().all(|v| v.is_some()) && self.cpsr_ready
    }
}

// == Register alias table (RAT) — maps arch regs to ROB tags for renaming

struct RenameTable {
    /// mapping[arch_reg] = Some(rob_id) if renamed, None if value is in RegFile
    mapping: Vec<Option<usize>>,
}

impl RenameTable {
    fn new() -> Self {
        RenameTable {
            mapping: vec![None; NUM_REGS],
        }
    }

    fn get(&self, reg: usize) -> Option<usize> {
        if reg < self.mapping.len() { self.mapping[reg] } else { None }
    }

    fn set(&mut self, reg: usize, rob_id: usize) {
        if reg < self.mapping.len() {
            self.mapping[reg] = Some(rob_id);
        }
    }

    fn clear(&mut self, reg: usize, rob_id: usize) {
        if reg < self.mapping.len() {
            if self.mapping[reg] == Some(rob_id) {
                self.mapping[reg] = None;
            }
        }
    }

    fn flush(&mut self) {
        for m in self.mapping.iter_mut() { *m = None; }
    }
}

// == Vector register alias table (VRAT) — maps vector regs to ROB tags

struct VRenameTable {
    mapping: Vec<Option<usize>>,
}

impl VRenameTable {
    fn new() -> Self {
        VRenameTable {
            mapping: vec![None; NUM_VREGS],
        }
    }

    fn get(&self, vreg: usize) -> Option<usize> {
        if vreg < self.mapping.len() { self.mapping[vreg] } else { None }
    }

    fn set(&mut self, vreg: usize, rob_id: usize) {
        if vreg < self.mapping.len() {
            self.mapping[vreg] = Some(rob_id);
        }
    }

    fn clear(&mut self, vreg: usize, rob_id: usize) {
        if vreg < self.mapping.len() {
            if self.mapping[vreg] == Some(rob_id) {
                self.mapping[vreg] = None;
            }
        }
    }

    fn flush(&mut self) {
        for m in self.mapping.iter_mut() { *m = None; }
    }
}

// == Completed instruction descriptor (used to avoid double-mutable-borrow)

struct CompletedEntry {
    rs_idx: usize,
    rob_id: usize,
    instr: Instruction,
    src_vals: Vec<Option<u32>>,
    cpsr_val: Option<u32>,
}

// == Public API

pub fn run(
    program: &[Instruction],
    regs: &mut RegFile,
    mem: &mut Memory,
    num_rs: usize,
    predictor_kind: PredictorKind,
    verbose: bool,
) -> Stats {
    run_with_cache(program, regs, mem, num_rs, predictor_kind, verbose, None)
}

pub fn run_with_cache(
    program: &[Instruction],
    regs: &mut RegFile,
    mem: &mut Memory,
    num_rs: usize,         // number of reservation stations
    predictor_kind: PredictorKind,
    verbose: bool,
    mut cache: Option<Cache>,
) -> Stats {
    let mut stats = Stats::new();
    let mut pc: usize = 0;
    let mut halted = false;

    let rob_capacity = num_rs * 4; // ROB is larger than RS pool
    let mut rob: VecDeque<RobEntry> = VecDeque::with_capacity(rob_capacity);
    let mut rs_pool: Vec<Option<RSEntry>> = vec![None; num_rs];
    let mut rat = RenameTable::new();
    let mut vrat = VRenameTable::new();  // Vector register alias table
    let mut predictor = BranchPredictor::new(predictor_kind);
    let mut next_rob_id: usize = 0;

    // CPSR tracking: identifies the latest ROB entry that will modify flags
    let mut cpsr_tag: Option<usize> = None;

    // Fetch buffer (up to 2 instructions fetched per cycle for superscalar)
    let mut fetch_buf: VecDeque<Instruction> = VecDeque::new();
    let fetch_width: usize = 2.min(num_rs);

    while !halted {
        stats.cycles += 1;

        // 1. RETIRE — commit oldest ROB entries that are complete (in order)
        let mut retire_limit = fetch_width;
        while retire_limit > 0 && !rob.is_empty() {
            let front_complete = rob.front()
                .map(|e| matches!(e.state, RobState::Complete(_)))
                .unwrap_or(false);

            if !front_complete { break; }

            let entry = rob.pop_front().unwrap();
            let result = match entry.state {
                RobState::Complete(r) => r,
                _ => unreachable!(),
            };

            // Apply GP register writes
            for &(r, val) in &result.reg_writes {
                regs.set(r, val);
                rat.clear(r, entry.id);
            }
            // Apply vector register writes
            for (v, data) in &result.vreg_writes {
                regs.set_vreg_slice(*v, data);
                vrat.clear(*v, entry.id);
            }
            // Apply CPSR
            if let Some(cpsr) = result.new_cpsr {
                regs.set_cpsr_raw(cpsr);
                if cpsr_tag == Some(entry.id) {
                    cpsr_tag = None;
                }
            }

            stats.instructions_executed += 1;
            if result.did_mem_read { stats.mem_reads += 1; }
            if result.did_mem_write { stats.mem_writes += 1; }

            if result.halt {
                halted = true;
                break;
            }

            retire_limit -= 1;
        }

        if halted { break; }

        // 2. COMPLETE — collect finished RS entries, execute, broadcast CDB
        //
        // Two-pass design to avoid double-mutable-borrow on rs_pool:
        //   Pass 1: identify completed entries and extract their data
        //   Pass 2: execute, forward results, and free RS slots

        // Pass 1: gather completed RS entries (immutable iteration)
        let mut completed: Vec<CompletedEntry> = Vec::new();
        for (idx, rs_entry) in rs_pool.iter().enumerate() {
            if let Some(ref entry) = rs_entry {
                if entry.executing && entry.remaining_cycles == 0 {
                    completed.push(CompletedEntry {
                        rs_idx: idx,
                        rob_id: entry.rob_id,
                        instr: entry.instr.clone(),
                        src_vals: entry.src_vals.clone(),
                        cpsr_val: entry.cpsr_val,
                    });
                }
            }
        }

        // Pass 2: process each completed entry
        // Track whether a flush occurred; once a branch mispredicts and flushes,
        // all later entries in `completed` with higher rob_ids belong to the
        // speculative (now-squashed) path and must be skipped.
        let mut flushed_after: Option<usize> = None;

        for ce in &completed {
            // Skip entries that were squashed by an earlier misprediction flush
            if let Some(fid) = flushed_after {
                if ce.rob_id > fid {
                    // RS slot may already be freed by the flush; ensure it is
                    rs_pool[ce.rs_idx] = None;
                    continue;
                }
            }

            // Build a temporary register snapshot with resolved operands
            let mut tmp_regs = regs.clone();
            let src_regs_list = ce.instr.sources();
            for (i, &reg) in src_regs_list.iter().enumerate() {
                if let Some(val) = ce.src_vals.get(i).and_then(|v| *v) {
                    tmp_regs.set(reg, val);
                }
            }
            // Apply forwarded CPSR for conditional branches
            if let Some(cpsr) = ce.cpsr_val {
                tmp_regs.set_cpsr_raw(cpsr);
            }

            let result = exec(&ce.instr, &tmp_regs, mem);

            // Handle branches
            if ce.instr.opcode.is_branch() {
                stats.branches_total += 1;
                let predicted = predictor.predict(ce.instr.addr);
                let actual = result.branch_taken;

                if actual { stats.branches_taken += 1; }

                if predicted == actual {
                    stats.branches_predicted += 1;
                } else {
                    stats.branches_mispredicted += 1;
                    stats.pipeline_flushes += 1;

                    // Flush: keep only ROB entries up to this rob_id
                    let flush_after = ce.rob_id;
                    flushed_after = Some(flush_after);
                    rob.retain(|e| e.id <= flush_after);

                    // Clear RS entries belonging to flushed instructions
                    for slot in rs_pool.iter_mut() {
                        if let Some(ref s) = slot {
                            if s.rob_id > flush_after {
                                *slot = None;
                            }
                        }
                    }

                    // Reset and rebuild RAT + VRAT
                    rat.flush();
                    vrat.flush();
                    for rob_e in &rob {
                        for &d in &rob_e.dest_regs {
                            rat.set(d, rob_e.id);
                        }
                        for &vd in &rob_e.dest_vregs {
                            vrat.set(vd, rob_e.id);
                        }
                    }

                    // Rebuild cpsr_tag from surviving ROB entries
                    cpsr_tag = None;
                    for rob_e in &rob {
                        if rob_e.instr.opcode.updates_flags() {
                            cpsr_tag = Some(rob_e.id);
                        }
                    }

                    fetch_buf.clear();

                    // Redirect PC
                    if actual {
                        if let Some(target) = result.branch_target {
                            pc = target;
                        }
                    } else {
                        pc = ce.instr.addr + 1;
                    }
                }
                predictor.update(ce.instr.addr, actual);
            }

            // Mark ROB entry as complete
            let result_vals: Vec<(usize, u32)> = result.reg_writes.clone();
            let result_cpsr: Option<u32> = result.new_cpsr;
            for rob_e in rob.iter_mut() {
                if rob_e.id == ce.rob_id {
                    rob_e.state = RobState::Complete(result);
                    break;
                }
            }

            // CDB broadcast: forward values to waiting RS entries
            for rs_entry in rs_pool.iter_mut() {
                if let Some(ref mut other) = rs_entry {
                    // Forward GP register values
                    let other_sources = other.instr.sources();
                    let num_gp_sources = other_sources.len();
                    for (idx, tag) in other.src_tags.iter_mut().enumerate() {
                        if *tag == Some(ce.rob_id) {
                            if idx < num_gp_sources {
                                // GP source: look up actual value from CDB
                                if let Some(&reg) = other_sources.get(idx) {
                                    for &(wr, val) in &result_vals {
                                        if wr == reg {
                                            other.src_vals[idx] = Some(val);
                                            *tag = None;
                                            break;
                                        }
                                    }
                                }
                            } else {
                                // Vector source: just mark as ready (data via vreg file)
                                other.src_vals[idx] = Some(0); // dummy value
                                *tag = None;
                            }
                        }
                    }
                    // Forward CPSR to conditional branches waiting on this producer
                    if other.cpsr_tag == Some(ce.rob_id) {
                        if let Some(cpsr) = result_cpsr {
                            other.cpsr_val = Some(cpsr);
                            other.cpsr_ready = true;
                            other.cpsr_tag = None;
                        }
                    }
                }
            }

            // Free the completed RS slot
            rs_pool[ce.rs_idx] = None;
        }

        // 3. EXECUTE — tick down remaining cycles for executing RS entries
        for rs_entry in rs_pool.iter_mut() {
            if let Some(ref mut entry) = rs_entry {
                if entry.executing && entry.remaining_cycles > 0 {
                    entry.remaining_cycles -= 1;
                }
            }
        }

        // 4. ISSUE — pick ready RS entries and start executing
        for rs_entry in rs_pool.iter_mut() {
            if let Some(ref mut entry) = rs_entry {
                if !entry.executing && entry.all_sources_ready() {
                    entry.executing = true;
                    entry.remaining_cycles = entry.instr.effective_latency().saturating_sub(1);

                    // Cache check for memory instructions
                    if let Some(ref mut c) = cache {
                        if entry.instr.opcode.is_load() || entry.instr.opcode.is_store() {
                            if matches!(entry.instr.opcode, Opcode::VLdr | Opcode::VStr) {
                                // VLDR/VSTR: check cache for each element
                                let len = entry.instr.imm.max(0) as u32;
                                let src_base = entry.src_vals.get(0)
                                    .and_then(|v| *v).unwrap_or(0);
                                let mut miss_count = 0u32;
                                for el in 0..len {
                                    if !c.access(src_base + el * 4) {
                                        miss_count += 1;
                                    }
                                }
                                stats.cache_hits += (len - miss_count) as u64;
                                stats.cache_misses += miss_count as u64;
                                entry.remaining_cycles += miss_count * MISS_PENALTY;
                            } else {
                                // Scalar memory instruction
                                if let Some(Some(base)) = entry.src_vals.first() {
                                    let addr = match entry.instr.opcode {
                                        Opcode::LdrPost | Opcode::StrPost => *base,
                                        _ => (*base as i32 + entry.instr.imm) as u32,
                                    };
                                    if !c.access(addr) {
                                        entry.remaining_cycles += MISS_PENALTY;
                                        stats.cache_misses += 1;
                                    } else {
                                        stats.cache_hits += 1;
                                    }
                                }
                            }
                        }
                    }

                    // Update ROB state
                    for rob_e in rob.iter_mut() {
                        if rob_e.id == entry.rob_id {
                            rob_e.state = RobState::Executing;
                            break;
                        }
                    }
                }
            }
        }

        // 5. DECODE / RENAME — take instructions from fetch buffer,
        //    allocate ROB + RS, perform register renaming
        let mut dispatched = 0;
        while dispatched < fetch_width && !fetch_buf.is_empty() {
            // Check ROB capacity
            if rob.len() >= rob_capacity { break; }

            // Find free RS slot
            let free_rs = rs_pool.iter().position(|s| s.is_none());
            if free_rs.is_none() { break; }
            let rs_idx = free_rs.unwrap();

            let instr = fetch_buf.pop_front().unwrap();
            let rob_id = next_rob_id;
            next_rob_id += 1;

            let dest_regs = instr.destinations();
            let dest_vregs = instr.vdestinations();

            // Resolve source operands via RAT (GP registers)
            let source_regs = instr.sources();
            let mut src_vals: Vec<Option<u32>> = Vec::new();
            let mut src_tags: Vec<Option<usize>> = Vec::new();

            for &src in &source_regs {
                if let Some(tag) = rat.get(src) {
                    // Check if the ROB entry has already completed
                    let mut found_val = None;
                    for rob_e in &rob {
                        if rob_e.id == tag {
                            if let RobState::Complete(ref res) = rob_e.state {
                                for &(wr, val) in &res.reg_writes {
                                    if wr == src {
                                        found_val = Some(val);
                                        break;
                                    }
                                }
                            }
                            break;
                        }
                    }
                    if let Some(val) = found_val {
                        src_vals.push(Some(val));
                        src_tags.push(None);
                    } else {
                        src_vals.push(None);
                        src_tags.push(Some(tag));
                    }
                } else {
                    src_vals.push(Some(regs.get(src)));
                    src_tags.push(None);
                }
            }

            // Resolve vector register source dependencies via VRAT
            // We use a dummy value (0) for "ready" vector sources since actual
            // vector data flows through the register file, not the CDB scalar path.
            let vsource_regs = instr.vsources();
            for &vsrc in &vsource_regs {
                if let Some(tag) = vrat.get(vsrc) {
                    // Check if the ROB entry has already completed
                    let mut completed = false;
                    for rob_e in &rob {
                        if rob_e.id == tag {
                            if matches!(rob_e.state, RobState::Complete(_)) {
                                completed = true;
                            }
                            break;
                        }
                    }
                    if completed {
                        src_vals.push(Some(0)); // dummy: data read at exec time
                        src_tags.push(None);
                    } else {
                        src_vals.push(None);
                        src_tags.push(Some(tag));
                    }
                } else {
                    src_vals.push(Some(0)); // available in architectural vreg file
                    src_tags.push(None);
                }
            }

            // Resolve CPSR dependency for conditional branches
            let (rs_cpsr_ready, rs_cpsr_tag, rs_cpsr_val) =
                if instr.opcode.is_conditional_branch() {
                    if let Some(tag) = cpsr_tag {
                        // Check if the flag-producing ROB entry has completed
                        let mut found_cpsr = None;
                        for rob_e in &rob {
                            if rob_e.id == tag {
                                if let RobState::Complete(ref res) = rob_e.state {
                                    found_cpsr = res.new_cpsr;
                                }
                                break;
                            }
                        }
                        if let Some(cpsr) = found_cpsr {
                            (true, None, Some(cpsr))
                        } else {
                            (false, Some(tag), None)
                        }
                    } else {
                        // No in-flight flag producer; use architectural CPSR
                        (true, None, Some(regs.cpsr_raw()))
                    }
                } else {
                    // Non-conditional: doesn't need CPSR
                    (true, None, None)
                };

            // Update RAT for GP destination registers
            for &d in &dest_regs {
                rat.set(d, rob_id);
            }
            // Update VRAT for vector destination registers
            for &vd in &dest_vregs {
                vrat.set(vd, rob_id);
            }
            if instr.opcode.updates_flags() {
                cpsr_tag = Some(rob_id);
            }

            // Allocate ROB entry
            rob.push_back(RobEntry {
                id: rob_id,
                instr: instr.clone(),
                state: RobState::Issued,
                dest_regs,
                dest_vregs,
            });

            // Allocate RS entry
            rs_pool[rs_idx] = Some(RSEntry {
                rob_id,
                instr,
                src_vals,
                src_tags,
                executing: false,
                remaining_cycles: 0,
                cpsr_ready: rs_cpsr_ready,
                cpsr_tag: rs_cpsr_tag,
                cpsr_val: rs_cpsr_val,
            });

            dispatched += 1;
        }

        // 6. FETCH — fetch up to N instructions into fetch buffer
        for _ in 0..fetch_width {
            if fetch_buf.len() >= fetch_width * 2 { break; }
            if pc >= program.len() { break; }

            let instr = program[pc].clone();
            fetch_buf.push_back(instr.clone());
            stats.instructions_fetched += 1;

            // Speculative PC advance
            if instr.opcode.is_branch() && instr.opcode != Opcode::Bx {
                let predicted = predictor.predict(pc);
                if predicted {
                    pc = instr.imm as usize;
                } else {
                    pc += 1;
                }
            } else {
                pc += 1;
            }
        }

        // Termination check
        if halted { break; }
        let pipeline_empty = fetch_buf.is_empty()
            && rs_pool.iter().all(|s| s.is_none())
            && rob.is_empty()
            && pc >= program.len();
        if pipeline_empty { break; }

        if stats.cycles > 1_000_000 {
            eprintln!("[WARN] Cycle limit reached. Stopping.");
            break;
        }
    }

    if verbose {
        println!("{}", regs);
    }

    // Copy cache statistics into stats
    if let Some(ref c) = cache {
        stats.cache_hits = c.hits;
        stats.cache_misses = c.misses;
    }

    stats
}
