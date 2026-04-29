/// ISA definition for a reduced ARM Thumb-like processor.
///
/// Defines all supported opcodes, operand types, condition codes,
/// and the core `Instruction` representation used throughout the simulator.

use std::fmt;

// == Register identifiers

/// General-purpose and special registers (R0-R15).
/// R13 = SP, R14 = LR, R15 = PC
pub const NUM_REGS: usize = 16;
pub const SP: usize = 13;
pub const LR: usize = 14;
pub const PC: usize = 15;

/// Vector register file: V0-V7, each up to MAX_VEC_LEN elements of u32.
pub const NUM_VREGS: usize = 8;
pub const MAX_VEC_LEN: usize = 64;

/// Human-readable register name.
pub fn reg_name(r: usize) -> &'static str {
    match r {
        0 => "R0", 1 => "R1", 2 => "R2", 3 => "R3",
        4 => "R4", 5 => "R5", 6 => "R6", 7 => "R7",
        8 => "R8", 9 => "R9", 10 => "R10", 11 => "R11",
        12 => "R12", 13 => "SP", 14 => "LR", 15 => "PC",
        _ => "??",
    }
}

/// Human-readable vector register name.
pub fn vreg_name(v: usize) -> &'static str {
    match v {
        0 => "V0", 1 => "V1", 2 => "V2", 3 => "V3",
        4 => "V4", 5 => "V5", 6 => "V6", 7 => "V7",
        _ => "V?",
    }
}

// == Condition codes  (used by conditional branches)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cond {
    AL,  // Always (unconditional)
    EQ,  // Equal           (Z == 1)
    NE,  // Not equal       (Z == 0)
    LT,  // Signed <        (N != V)
    GE,  // Signed >=       (N == V)
    GT,  // Signed >        (Z == 0 && N == V)
    LE,  // Signed <=       (Z == 1 || N != V)
}

impl fmt::Display for Cond {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Cond::AL => write!(f, ""),
            Cond::EQ => write!(f, "EQ"),
            Cond::NE => write!(f, "NE"),
            Cond::LT => write!(f, "LT"),
            Cond::GE => write!(f, "GE"),
            Cond::GT => write!(f, "GT"),
            Cond::LE => write!(f, "LE"),
        }
    }
}

// == Opcode enumeration

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opcode {
    // Arithmetic
    Add,    // Rd = Rn + Rm
    AddI,   // Rd = Rn + imm
    Sub,    // Rd = Rn - Rm
    SubI,   // Rd = Rn - imm
    Mul,    // Rd = Rn * Rm

    // Compare (sets flags, no destination)
    Cmp,    // flags <- Rn - Rm
    CmpI,   // flags <- Rn - imm

    // Logical
    And,    // Rd = Rn & Rm
    Orr,    // Rd = Rn | Rm
    Eor,    // Rd = Rn ^ Rm
    Mvn,    // Rd = ~Rn

    // Shift
    Lsl,    // Rd = Rn << imm
    Lsr,    // Rd = Rn >> imm (logical)
    Asr,    // Rd = Rn >> imm (arithmetic)

    // Data movement
    Mov,    // Rd = Rm
    MovI,   // Rd = imm16

    // Memory access (word = 32-bit)
    Ldr,      // Rd = Mem[Rn + imm]
    Str,      // Mem[Rn + imm] = Rm
    LdrB,     // Rd = Mem[Rn + imm] (byte, zero-extended)
    StrB,     // Mem[Rn + imm] = Rm (byte)
    LdrPost,  // Rd = Mem[Rn]; Rn = Rn + imm  (post-indexed)
    StrPost,  // Mem[Rn] = Rm; Rn = Rn + imm  (post-indexed)

    // Branching
    B,      // PC = target  (unconditional)
    Beq,    // PC = target  if Z
    Bne,    // PC = target  if !Z
    Blt,    // PC = target  if N != V
    Bge,    // PC = target  if N == V
    Bgt,    // PC = target  if !Z && N == V
    Ble,    // PC = target  if Z || N != V
    Bl,     // LR = PC+1; PC = target
    Bx,     // PC = Rm (return)

    // Stack
    Push,   // push register list onto stack
    Pop,    // pop register list from stack

    // Vector (extension) — SIMD with separate vector register file (V0-V7)
    VLdr,   // VLDR Vd, Rn, #len — Vd[i] = Mem[Rn + i*4], i=0..len-1
    VStr,   // VSTR Vs, Rn, #len — Mem[Rn + i*4] = Vs[i], i=0..len-1
    VAdd,   // VADD Vd, Vn, Vm, #len — Vd[i] = Vn[i] + Vm[i]
    VSub,   // VSUB Vd, Vn, Vm, #len — Vd[i] = Vn[i] - Vm[i]
    VMul,   // VMUL Vd, Vn, Vm, #len — Vd[i] = Vn[i] * Vm[i]
    VDot,   // VDOT Rd, Vn, Vm, #len — Rd = sum(Vn[i] * Vm[i])

    // Special
    Nop,
    Halt,
}

impl Opcode {
    /// Whether this opcode is a branch / jump.
    pub fn is_branch(&self) -> bool {
        matches!(
            self,
            Opcode::B | Opcode::Beq | Opcode::Bne | Opcode::Blt
            | Opcode::Bge | Opcode::Bgt | Opcode::Ble
            | Opcode::Bl | Opcode::Bx
        )
    }

    /// Whether this opcode is a conditional branch.
    pub fn is_conditional_branch(&self) -> bool {
        matches!(
            self,
            Opcode::Beq | Opcode::Bne | Opcode::Blt
            | Opcode::Bge | Opcode::Bgt | Opcode::Ble
        )
    }

    /// Condition code implied by the branch opcode.
    pub fn branch_cond(&self) -> Cond {
        match self {
            Opcode::Beq => Cond::EQ,
            Opcode::Bne => Cond::NE,
            Opcode::Blt => Cond::LT,
            Opcode::Bge => Cond::GE,
            Opcode::Bgt => Cond::GT,
            Opcode::Ble => Cond::LE,
            _ => Cond::AL,
        }
    }

    /// Execution latency in cycles (for sub-pipelining).
    pub fn latency(&self) -> u32 {
        match self {
            Opcode::Mul => 3,
            Opcode::Ldr | Opcode::LdrB | Opcode::LdrPost => 2,
            Opcode::Str | Opcode::StrB | Opcode::StrPost => 2,
            Opcode::Push | Opcode::Pop => 2, // simplified
            // Vector ops: base latency (see Instruction::effective_latency for dynamic)
            Opcode::VLdr | Opcode::VStr => 2,
            Opcode::VAdd | Opcode::VSub | Opcode::VMul => 4,
            Opcode::VDot => 5,
            _ => 1,
        }
    }

    /// Whether this opcode is a load from memory.
    pub fn is_load(&self) -> bool {
        matches!(self, Opcode::Ldr | Opcode::LdrB | Opcode::LdrPost | Opcode::Pop
            | Opcode::VLdr)
    }

    /// Whether this opcode is a store to memory.
    pub fn is_store(&self) -> bool {
        matches!(self, Opcode::Str | Opcode::StrB | Opcode::StrPost | Opcode::Push
            | Opcode::VStr)
    }

    /// Whether this opcode is a vector (SIMD) instruction.
    #[allow(dead_code)]
    pub fn is_vector(&self) -> bool {
        matches!(self, Opcode::VLdr | Opcode::VStr
            | Opcode::VAdd | Opcode::VSub | Opcode::VMul | Opcode::VDot)
    }

    /// Whether this opcode uses the vector register file (reads or writes).
    #[allow(dead_code)]
    pub fn uses_vregs(&self) -> bool {
        matches!(self, Opcode::VLdr | Opcode::VStr
            | Opcode::VAdd | Opcode::VSub | Opcode::VMul | Opcode::VDot)
    }

    /// Whether this instruction updates CPSR flags by default.
    pub fn updates_flags(&self) -> bool {
        matches!(
            self,
            Opcode::Add | Opcode::AddI | Opcode::Sub | Opcode::SubI
            | Opcode::Mul
            | Opcode::Cmp | Opcode::CmpI
            | Opcode::And | Opcode::Orr | Opcode::Eor | Opcode::Mvn
            | Opcode::Lsl | Opcode::Lsr | Opcode::Asr
        )
    }
}

impl fmt::Display for Opcode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Opcode::Add => "ADD", Opcode::AddI => "ADDI",
            Opcode::Sub => "SUB", Opcode::SubI => "SUBI",
            Opcode::Mul => "MUL",
            Opcode::Cmp => "CMP", Opcode::CmpI => "CMPI",
            Opcode::And => "AND", Opcode::Orr => "ORR",
            Opcode::Eor => "EOR", Opcode::Mvn => "MVN",
            Opcode::Lsl => "LSL", Opcode::Lsr => "LSR", Opcode::Asr => "ASR",
            Opcode::Mov => "MOV", Opcode::MovI => "MOVI",
            Opcode::Ldr => "LDR", Opcode::Str => "STR",
            Opcode::LdrB => "LDRB", Opcode::StrB => "STRB",
            Opcode::LdrPost => "LDRPOST", Opcode::StrPost => "STRPOST",
            Opcode::B => "B", Opcode::Beq => "BEQ", Opcode::Bne => "BNE",
            Opcode::Blt => "BLT", Opcode::Bge => "BGE",
            Opcode::Bgt => "BGT", Opcode::Ble => "BLE",
            Opcode::Bl => "BL", Opcode::Bx => "BX",
            Opcode::Push => "PUSH", Opcode::Pop => "POP",
            Opcode::VLdr => "VLDR", Opcode::VStr => "VSTR",
            Opcode::VAdd => "VADD", Opcode::VSub => "VSUB",
            Opcode::VMul => "VMUL", Opcode::VDot => "VDOT",
            Opcode::Nop => "NOP", Opcode::Halt => "HALT",
        };
        write!(f, "{}", s)
    }
}

// == Instruction representation

/// A fully decoded instruction ready for execution.
#[derive(Debug, Clone)]
pub struct Instruction {
    pub opcode: Opcode,
    pub rd: Option<usize>,       // destination GP register
    pub rn: Option<usize>,       // first source GP register
    pub rm: Option<usize>,       // second source GP register
    pub imm: i32,                // immediate value / branch offset / vector length
    pub reg_list: Vec<usize>,    // for PUSH / POP
    pub addr: usize,             // instruction address (index in program)
    pub text: String,            // original assembly text for debug
    // Vector register operands (separate namespace from GPRs)
    pub vd: Option<usize>,       // destination vector register (V0-V7)
    pub vn: Option<usize>,       // first source vector register
    pub vm: Option<usize>,       // second source vector register
}

impl Instruction {
    pub fn new(opcode: Opcode, addr: usize) -> Self {
        Instruction {
            opcode,
            rd: None,
            rn: None,
            rm: None,
            imm: 0,
            reg_list: Vec::new(),
            addr,
            text: String::new(),
            vd: None,
            vn: None,
            vm: None,
        }
    }

    /// Effective execution latency, accounting for vector length.
    /// For vector instructions, latency scales with the number of elements.
    pub fn effective_latency(&self) -> u32 {
        let len = self.imm.max(1) as u32;
        match self.opcode {
            // Vector load/store: 1 cycle setup + 1 cycle per element
            Opcode::VLdr | Opcode::VStr => 1 + len,
            Opcode::VAdd | Opcode::VSub => {
                // 1 cycle setup + 1 cycle per element
                1 + len
            }
            Opcode::VMul => {
                // 2 cycle setup + 1 cycle per element
                2 + len
            }
            Opcode::VDot => {
                // 2 cycle setup + 2 cycles per element (mul + accumulate)
                2 + 2 * len
            }
            _ => self.opcode.latency(),
        }
    }

    /// Source registers read by this instruction.
    pub fn sources(&self) -> Vec<usize> {
        let mut srcs = Vec::new();
        match self.opcode {
            // Reg-reg arithmetic / logical
            Opcode::Add | Opcode::Sub | Opcode::Mul
            | Opcode::And | Opcode::Orr | Opcode::Eor | Opcode::Cmp => {
                if let Some(rn) = self.rn { srcs.push(rn); }
                if let Some(rm) = self.rm { srcs.push(rm); }
            }
            // Reg-imm arithmetic / compare
            Opcode::AddI | Opcode::SubI | Opcode::CmpI
            | Opcode::Lsl | Opcode::Lsr | Opcode::Asr => {
                if let Some(rn) = self.rn { srcs.push(rn); }
            }
            // Unary
            Opcode::Mvn | Opcode::Mov => {
                if let Some(rn) = self.rn { srcs.push(rn); }
            }
            // Memory loads: base register
            Opcode::Ldr | Opcode::LdrB | Opcode::LdrPost => {
                if let Some(rn) = self.rn { srcs.push(rn); }
            }
            // Memory stores: base register + data register
            Opcode::Str | Opcode::StrB | Opcode::StrPost => {
                if let Some(rn) = self.rn { srcs.push(rn); }
                if let Some(rm) = self.rm { srcs.push(rm); } // rm = data reg for STR
            }
            // Branch to register
            Opcode::Bx => {
                if let Some(rm) = self.rm { srcs.push(rm); }
            }
            // Push reads listed registers + SP
            Opcode::Push => {
                srcs.push(SP);
                srcs.extend(self.reg_list.iter());
            }
            // Pop reads SP
            Opcode::Pop => {
                srcs.push(SP);
            }
            // VLDR reads base address from Rn (GP register)
            Opcode::VLdr => {
                if let Some(rn) = self.rn { srcs.push(rn); }
            }
            // VSTR reads base address from Rn (GP register)
            Opcode::VStr => {
                if let Some(rn) = self.rn { srcs.push(rn); }
            }
            // VADD/VSUB/VMUL: register-to-register vector ops (no GP source)
            Opcode::VAdd | Opcode::VSub | Opcode::VMul => {}
            // VDOT: writes scalar result to Rd (GP), no GP source
            Opcode::VDot => {}
            _ => {}
        }
        srcs
    }

    /// Destination registers written by this instruction.
    pub fn destinations(&self) -> Vec<usize> {
        let mut dsts = Vec::new();
        match self.opcode {
            // Comparisons write no destination register (only flags)
            Opcode::Cmp | Opcode::CmpI => {}
            // Branches write no GP register (except BL writes LR)
            Opcode::B | Opcode::Beq | Opcode::Bne
            | Opcode::Blt | Opcode::Bge | Opcode::Bgt | Opcode::Ble => {}
            Opcode::Bx => {}
            Opcode::Bl => { dsts.push(LR); }
            // Store writes no register (but StrPost writes back Rn)
            Opcode::Str | Opcode::StrB => {}
            Opcode::StrPost => {
                if let Some(rn) = self.rn { dsts.push(rn); }
            }
            // LdrPost writes both Rd (loaded value) and Rn (updated base)
            Opcode::LdrPost => {
                if let Some(rd) = self.rd { dsts.push(rd); }
                if let Some(rn) = self.rn { dsts.push(rn); }
            }
            // Push modifies SP
            Opcode::Push => { dsts.push(SP); }
            // Pop modifies listed registers + SP
            Opcode::Pop => {
                dsts.push(SP);
                dsts.extend(self.reg_list.iter());
            }
            // VLDR/VSTR/VADD/VSUB/VMUL write to vector registers (no GP dest)
            Opcode::VLdr | Opcode::VStr
            | Opcode::VAdd | Opcode::VSub | Opcode::VMul => {}
            // VDOT writes scalar result to GP Rd
            Opcode::VDot => {
                if let Some(rd) = self.rd { dsts.push(rd); }
            }
            // Nop / Halt write nothing
            Opcode::Nop | Opcode::Halt => {}
            // Everything else writes Rd
            _ => {
                if let Some(rd) = self.rd { dsts.push(rd); }
            }
        }
        dsts
    }

    /// Source vector registers read by this instruction.
    pub fn vsources(&self) -> Vec<usize> {
        let mut srcs = Vec::new();
        match self.opcode {
            // VLDR: no vector source (reads from memory)
            Opcode::VLdr => {}
            // VSTR: reads from Vs (stored in vd field)
            Opcode::VStr => {
                if let Some(vd) = self.vd { srcs.push(vd); }
            }
            // VADD/VSUB/VMUL: read Vn, Vm
            Opcode::VAdd | Opcode::VSub | Opcode::VMul => {
                if let Some(vn) = self.vn { srcs.push(vn); }
                if let Some(vm) = self.vm { srcs.push(vm); }
            }
            // VDOT: read Vn, Vm
            Opcode::VDot => {
                if let Some(vn) = self.vn { srcs.push(vn); }
                if let Some(vm) = self.vm { srcs.push(vm); }
            }
            _ => {}
        }
        srcs
    }

    /// Destination vector registers written by this instruction.
    pub fn vdestinations(&self) -> Vec<usize> {
        let mut dsts = Vec::new();
        match self.opcode {
            // VLDR: writes to Vd
            Opcode::VLdr => {
                if let Some(vd) = self.vd { dsts.push(vd); }
            }
            // VSTR: no vector destination (writes to memory)
            Opcode::VStr => {}
            // VADD/VSUB/VMUL: write to Vd
            Opcode::VAdd | Opcode::VSub | Opcode::VMul => {
                if let Some(vd) = self.vd { dsts.push(vd); }
            }
            // VDOT: no vector destination (writes scalar to GP Rd)
            Opcode::VDot => {}
            _ => {}
        }
        dsts
    }
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.text.is_empty() {
            write!(f, "{}", self.opcode)
        } else {
            write!(f, "{}", self.text)
        }
    }
}
