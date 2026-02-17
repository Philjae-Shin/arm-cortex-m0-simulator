/// Stage 3-1: Non-pipelined scalar processor simulator.
///
/// Each instruction goes through fetch → decode → execute sequentially
/// before the next instruction begins.  No parallelism, no hazard issues.

use crate::isa::Instruction;
use crate::regs::RegFile;
use crate::memory::Memory;
use crate::execute::{exec, commit};
use crate::stats::Stats;

pub fn run(
    program: &[Instruction],
    regs: &mut RegFile,
    mem: &mut Memory,
    verbose: bool,
) -> Stats {
    let mut stats = Stats::new();
    let mut pc: usize = 0;

    loop {
        // Bounds check
        if pc >= program.len() {
            break;
        }

        let instr = &program[pc];

        // -- Fetch (1 cycle) --
        stats.cycles += 1;
        stats.instructions_fetched += 1;

        // -- Decode (1 cycle) --
        stats.cycles += 1;

        // -- Execute (latency cycles) --
        let latency = instr.opcode.latency();
        stats.cycles += latency as u64;

        let result = exec(instr, regs, mem);

        if verbose {
            println!("[cyc {:>5}] pc={:>3}  {}", stats.cycles, pc, instr);
        }

        // Track branch statistics
        if instr.opcode.is_branch() {
            stats.branches_total += 1;
            if result.branch_taken {
                stats.branches_taken += 1;
            }
        }

        // Track memory access
        if result.did_mem_read { stats.mem_reads += 1; }
        if result.did_mem_write { stats.mem_writes += 1; }

        // Commit results to architectural state
        commit(&result, regs);
        stats.instructions_executed += 1;

        // Halt?
        if result.halt {
            break;
        }

        // Advance PC
        if let Some(target) = result.branch_target {
            pc = target;
        } else {
            pc += 1;
        }
    }

    if verbose {
        println!("{}", regs);
    }

    stats
}
