/// Instruction execution logic.
///
/// This module contains the pure function `exec` which takes an instruction,
/// the current register file snapshot, and a mutable reference to memory,
/// and produces an `ExecResult` describing all side-effects.

use crate::isa::*;
use crate::regs::RegFile;
use crate::memory::Memory;

// Execution result
/// Outcome of executing a single instruction.
#[derive(Debug, Clone)]
pub struct ExecResult {
    /// GP register writes: (register_index, new_value)
    pub reg_writes: Vec<(usize, u32)>,
    /// Vector register writes: (vreg_index, element_data)
    pub vreg_writes: Vec<(usize, Vec<u32>)>,
    /// Whether CPSR flags were updated (snapshot of new CPSR).
    pub new_cpsr: Option<u32>,
    /// If the instruction changes control flow, the new PC (instruction index).
    pub branch_target: Option<usize>,
    /// Whether this was a taken conditional branch.
    pub branch_taken: bool,
    /// Whether we should halt.
    pub halt: bool,
    /// Memory read performed (for stats).
    pub did_mem_read: bool,
    /// Memory write performed (for stats).
    pub did_mem_write: bool,
}

impl ExecResult {
    fn empty() -> Self {
        ExecResult {
            reg_writes: Vec::new(),
            vreg_writes: Vec::new(),
            new_cpsr: None,
            branch_target: None,
            branch_taken: false,
            halt: false,
            did_mem_read: false,
            did_mem_write: false,
        }
    }
}

// Condition evaluation
/// Evaluate a branch condition against the current CPSR flags.
pub fn eval_condition(cond: Cond, regs: &RegFile) -> bool {
    match cond {
        Cond::AL => true,
        Cond::EQ => regs.flag_z(),
        Cond::NE => !regs.flag_z(),
        Cond::LT => regs.flag_n() != regs.flag_v(),
        Cond::GE => regs.flag_n() == regs.flag_v(),
        Cond::GT => !regs.flag_z() && (regs.flag_n() == regs.flag_v()),
        Cond::LE => regs.flag_z() || (regs.flag_n() != regs.flag_v()),
    }
}

// Main execution function

/// Execute `instr` using register values from `regs` and data `mem`.
/// Returns an `ExecResult` describing all side-effects.
///
/// **Important**: this function does NOT mutate `regs` directly — the caller
/// is responsible for applying the `ExecResult` to the architectural state.
/// This separation is critical for the out-of-order pipeline where results
/// must be buffered before commit.
pub fn exec(instr: &Instruction, regs: &RegFile, mem: &mut Memory) -> ExecResult {
    let mut res = ExecResult::empty();

    // Helper closures to read source registers
    let rn_val = || instr.rn.map(|r| regs.get(r)).unwrap_or(0);
    let rm_val = || instr.rm.map(|r| regs.get(r)).unwrap_or(0);

    match instr.opcode {
        // Arithmetic
        Opcode::Add => {
            let a = rn_val();
            let b = rm_val();
            let result = a.wrapping_add(b);
            let rd = instr.rd.unwrap();
            res.reg_writes.push((rd, result));
            // Update flags
            let mut tmp = regs.clone();
            tmp.set_flags_add(a, b, result);
            res.new_cpsr = Some(tmp.cpsr_raw());
        }

        Opcode::AddI => {
            let a = rn_val();
            let b = instr.imm as u32;
            let result = a.wrapping_add(b);
            let rd = instr.rd.unwrap();
            res.reg_writes.push((rd, result));
            let mut tmp = regs.clone();
            tmp.set_flags_add(a, b, result);
            res.new_cpsr = Some(tmp.cpsr_raw());
        }

        Opcode::Sub => {
            let a = rn_val();
            let b = rm_val();
            let result = a.wrapping_sub(b);
            let rd = instr.rd.unwrap();
            res.reg_writes.push((rd, result));
            let mut tmp = regs.clone();
            tmp.set_flags_sub(a, b, result);
            res.new_cpsr = Some(tmp.cpsr_raw());
        }

        Opcode::SubI => {
            let a = rn_val();
            let b = instr.imm as u32;
            let result = a.wrapping_sub(b);
            let rd = instr.rd.unwrap();
            res.reg_writes.push((rd, result));
            let mut tmp = regs.clone();
            tmp.set_flags_sub(a, b, result);
            res.new_cpsr = Some(tmp.cpsr_raw());
        }

        Opcode::Mul => {
            let a = rn_val();
            let b = rm_val();
            let result = a.wrapping_mul(b);
            let rd = instr.rd.unwrap();
            res.reg_writes.push((rd, result));
            // MUL updates N and Z only
            let mut tmp = regs.clone();
            tmp.set_nz(result);
            res.new_cpsr = Some(tmp.cpsr_raw());
        }

        // Compare
        Opcode::Cmp => {
            let a = rn_val();
            let b = rm_val();
            let result = a.wrapping_sub(b);
            let mut tmp = regs.clone();
            tmp.set_flags_sub(a, b, result);
            res.new_cpsr = Some(tmp.cpsr_raw());
        }

        Opcode::CmpI => {
            let a = rn_val();
            let b = instr.imm as u32;
            let result = a.wrapping_sub(b);
            let mut tmp = regs.clone();
            tmp.set_flags_sub(a, b, result);
            res.new_cpsr = Some(tmp.cpsr_raw());
        }

        // Logical
        Opcode::And => {
            let result = rn_val() & rm_val();
            let rd = instr.rd.unwrap();
            res.reg_writes.push((rd, result));
            let mut tmp = regs.clone();
            tmp.set_nz(result);
            res.new_cpsr = Some(tmp.cpsr_raw());
        }

        Opcode::Orr => {
            let result = rn_val() | rm_val();
            let rd = instr.rd.unwrap();
            res.reg_writes.push((rd, result));
            let mut tmp = regs.clone();
            tmp.set_nz(result);
            res.new_cpsr = Some(tmp.cpsr_raw());
        }

        Opcode::Eor => {
            let result = rn_val() ^ rm_val();
            let rd = instr.rd.unwrap();
            res.reg_writes.push((rd, result));
            let mut tmp = regs.clone();
            tmp.set_nz(result);
            res.new_cpsr = Some(tmp.cpsr_raw());
        }

        Opcode::Mvn => {
            let result = !rn_val();
            let rd = instr.rd.unwrap();
            res.reg_writes.push((rd, result));
            let mut tmp = regs.clone();
            tmp.set_nz(result);
            res.new_cpsr = Some(tmp.cpsr_raw());
        }

        // Shifts
        Opcode::Lsl => {
            let val = rn_val();
            let shift = instr.imm as u32;
            let result = if shift >= 32 { 0 } else { val << shift };
            let rd = instr.rd.unwrap();
            res.reg_writes.push((rd, result));
            let mut tmp = regs.clone();
            tmp.set_nz(result);
            res.new_cpsr = Some(tmp.cpsr_raw());
        }

        Opcode::Lsr => {
            let val = rn_val();
            let shift = instr.imm as u32;
            let result = if shift >= 32 { 0 } else { val >> shift };
            let rd = instr.rd.unwrap();
            res.reg_writes.push((rd, result));
            let mut tmp = regs.clone();
            tmp.set_nz(result);
            res.new_cpsr = Some(tmp.cpsr_raw());
        }

        Opcode::Asr => {
            let val = rn_val() as i32;
            let shift = instr.imm as u32;
            let result = if shift >= 32 {
                if val < 0 { 0xFFFF_FFFFu32 } else { 0 }
            } else {
                (val >> shift) as u32
            };
            let rd = instr.rd.unwrap();
            res.reg_writes.push((rd, result));
            let mut tmp = regs.clone();
            tmp.set_nz(result);
            res.new_cpsr = Some(tmp.cpsr_raw());
        }

        // Data movement
        Opcode::Mov => {
            let val = rn_val();
            let rd = instr.rd.unwrap();
            res.reg_writes.push((rd, val));
        }

        Opcode::MovI => {
            let rd = instr.rd.unwrap();
            res.reg_writes.push((rd, instr.imm as u32));
        }

        // Memory access
        Opcode::Ldr => {
            let base = rn_val();
            let addr = (base as i32 + instr.imm) as u32;
            let val = mem.read_word(addr);
            let rd = instr.rd.unwrap();
            res.reg_writes.push((rd, val));
            res.did_mem_read = true;
        }

        Opcode::LdrB => {
            let base = rn_val();
            let addr = (base as i32 + instr.imm) as u32;
            let val = mem.read_byte(addr) as u32;
            let rd = instr.rd.unwrap();
            res.reg_writes.push((rd, val));
            res.did_mem_read = true;
        }

        Opcode::Str => {
            let base = rn_val();
            let addr = (base as i32 + instr.imm) as u32;
            let data = rm_val(); // rm holds the data register for STR
            mem.write_word(addr, data);
            res.did_mem_write = true;
        }

        Opcode::StrB => {
            let base = rn_val();
            let addr = (base as i32 + instr.imm) as u32;
            let data = rm_val();
            mem.write_byte(addr, data as u8);
            res.did_mem_write = true;
        }

        // Post-indexed load: Rd = Mem[Rn], then Rn = Rn + imm
        Opcode::LdrPost => {
            let base = rn_val();
            let val = mem.read_word(base);
            let rd = instr.rd.unwrap();
            let rn = instr.rn.unwrap();
            res.reg_writes.push((rd, val));
            res.reg_writes.push((rn, base.wrapping_add(instr.imm as u32)));
            res.did_mem_read = true;
        }

        // Post-indexed store: Mem[Rn] = Rm, then Rn = Rn + imm
        Opcode::StrPost => {
            let base = rn_val();
            let data = rm_val();
            mem.write_word(base, data);
            let rn = instr.rn.unwrap();
            res.reg_writes.push((rn, base.wrapping_add(instr.imm as u32)));
            res.did_mem_write = true;
        }

        // Branches
        Opcode::B => {
            let target = instr.imm as usize;
            res.branch_target = Some(target);
            res.branch_taken = true;
        }

        Opcode::Beq | Opcode::Bne | Opcode::Blt
        | Opcode::Bge | Opcode::Bgt | Opcode::Ble => {
            let cond = instr.opcode.branch_cond();
            let taken = eval_condition(cond, regs);
            if taken {
                res.branch_target = Some(instr.imm as usize);
            }
            res.branch_taken = taken;
        }

        Opcode::Bl => {
            // Save return address (next instruction) in LR
            let return_addr = (instr.addr + 1) as u32;
            res.reg_writes.push((LR, return_addr));
            res.branch_target = Some(instr.imm as usize);
            res.branch_taken = true;
        }

        Opcode::Bx => {
            let target = rm_val() as usize;
            res.branch_target = Some(target);
            res.branch_taken = true;
        }

        // Stack operations
        Opcode::Push => {
            let mut sp = regs.get(SP);
            // Push registers in descending order (full descending stack)
            for &r in instr.reg_list.iter().rev() {
                sp = sp.wrapping_sub(4);
                mem.write_word(sp, regs.get(r));
            }
            res.reg_writes.push((SP, sp));
            res.did_mem_write = true;
        }

        Opcode::Pop => {
            let mut sp = regs.get(SP);
            let mut branch_via_pop = false;
            for &r in &instr.reg_list {
                let val = mem.read_word(sp);
                sp = sp.wrapping_add(4);
                if r == PC {
                    // POP {PC} acts as a return
                    res.branch_target = Some(val as usize);
                    branch_via_pop = true;
                } else {
                    res.reg_writes.push((r, val));
                }
            }
            res.reg_writes.push((SP, sp));
            res.did_mem_read = true;
            if branch_via_pop {
                res.branch_taken = true;
            }
        }

        // Vector (extension) — SIMD with separate vector register file

        // VLDR: Vd[i] = Mem[Rn + i*4] for i in 0..len
        Opcode::VLdr => {
            let base = rn_val();
            let len = instr.imm.max(0) as usize;
            let vd = instr.vd.unwrap_or(0);
            let mut data = Vec::with_capacity(len);
            for i in 0..len {
                data.push(mem.read_word(base + (i as u32) * 4));
            }
            res.vreg_writes.push((vd, data));
            res.did_mem_read = true;
        }

        // VSTR: Mem[Rn + i*4] = Vs[i] for i in 0..len
        Opcode::VStr => {
            let base = rn_val();
            let len = instr.imm.max(0) as usize;
            let vs = instr.vd.unwrap_or(0);  // vd field holds source vreg for VSTR
            for i in 0..len {
                let val = regs.get_vreg_element(vs, i);
                mem.write_word(base + (i as u32) * 4, val);
            }
            res.did_mem_write = true;
        }

        // VADD: Vd[i] = Vn[i] + Vm[i] for i in 0..len
        Opcode::VAdd => {
            let len = instr.imm.max(0) as usize;
            let vn = instr.vn.unwrap_or(0);
            let vm = instr.vm.unwrap_or(0);
            let vd = instr.vd.unwrap_or(0);
            let mut data = Vec::with_capacity(len);
            for i in 0..len {
                let a = regs.get_vreg_element(vn, i);
                let b = regs.get_vreg_element(vm, i);
                data.push(a.wrapping_add(b));
            }
            res.vreg_writes.push((vd, data));
        }

        // VSUB: Vd[i] = Vn[i] - Vm[i] for i in 0..len
        Opcode::VSub => {
            let len = instr.imm.max(0) as usize;
            let vn = instr.vn.unwrap_or(0);
            let vm = instr.vm.unwrap_or(0);
            let vd = instr.vd.unwrap_or(0);
            let mut data = Vec::with_capacity(len);
            for i in 0..len {
                let a = regs.get_vreg_element(vn, i);
                let b = regs.get_vreg_element(vm, i);
                data.push(a.wrapping_sub(b));
            }
            res.vreg_writes.push((vd, data));
        }

        // VMUL: Vd[i] = Vn[i] * Vm[i] for i in 0..len
        Opcode::VMul => {
            let len = instr.imm.max(0) as usize;
            let vn = instr.vn.unwrap_or(0);
            let vm = instr.vm.unwrap_or(0);
            let vd = instr.vd.unwrap_or(0);
            let mut data = Vec::with_capacity(len);
            for i in 0..len {
                let a = regs.get_vreg_element(vn, i);
                let b = regs.get_vreg_element(vm, i);
                data.push(a.wrapping_mul(b));
            }
            res.vreg_writes.push((vd, data));
        }

        // VDOT: Rd = sum(Vn[i] * Vm[i]) for i in 0..len  (scalar result)
        Opcode::VDot => {
            let len = instr.imm.max(0) as usize;
            let vn = instr.vn.unwrap_or(0);
            let vm = instr.vm.unwrap_or(0);
            let mut acc: u32 = 0;
            for i in 0..len {
                let a = regs.get_vreg_element(vn, i);
                let b = regs.get_vreg_element(vm, i);
                acc = acc.wrapping_add(a.wrapping_mul(b));
            }
            let rd = instr.rd.unwrap();
            res.reg_writes.push((rd, acc));
        }

        // Special
        Opcode::Nop => { /* nothing */ }
        Opcode::Halt => { res.halt = true; }
    }

    res
}

/// Apply an `ExecResult` to the architectural register file.
pub fn commit(result: &ExecResult, regs: &mut RegFile) {
    for &(r, val) in &result.reg_writes {
        regs.set(r, val);
    }
    for (v, data) in &result.vreg_writes {
        regs.set_vreg_slice(*v, data);
    }
    if let Some(cpsr) = result.new_cpsr {
        regs.set_cpsr_raw(cpsr);
    }
}
