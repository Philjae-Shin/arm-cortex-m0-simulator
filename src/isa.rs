/// Instruction Set Architecture definition for a reduced ARM Thumb-like processor.
///
/// This module defines all supported opcodes, operand types, condition codes,
/// and the core `Instruction` representation used throughout the simulator.

use std::fmt;

// ---------------------------------------------------------------------------
// Register identifiers
// ---------------------------------------------------------------------------

/// General-purpose and special registers (R0-R15).
/// R13 = SP, R14 = LR, R15 = PC
pub const NUM_REGS: usize = 16;
pub const SP: usize = 13;
pub const LR: usize = 14;
pub const PC: usize = 15;

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

// ---------------------------------------------------------------------------
// Condition codes  (used by conditional branches)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cond {
    AL,  // Always (unconditional)
    EQ,  // Equal           (Z == 1)
    NE,  // Not equal       (Z == 0)
    LT,  // Signed <        (N != V)
    GE,  // Signed >=       (N == V)
    GT,  // Signed >        (Z == 0 && N == V)
    LE,  // Signed <=       (Z == 1 || N != V)
    HI,  // Unsigned >      (C == 1 && Z == 0)
    LS,  // Unsigned <=     (C == 0 || Z == 1)
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
            Cond::HI => write!(f, "HI"),
            Cond::LS => write!(f, "LS"),
        }
    }
}

// ---------------------------------------------------------------------------
// Opcode enumeration
// ---------------------------------------------------------------------------

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
    Ldr,    // Rd = Mem[Rn + imm]
    Str,    // Mem[Rn + imm] = Rm
    LdrB,   // Rd = Mem[Rn + imm] (byte, zero-extended)
    StrB,   // Mem[Rn + imm] = Rm (byte)

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

    // Special
    Nop,
    Halt,
}

impl Opcode {
    /// Classify instruction into a functional-unit category.
    pub fn unit_kind(&self) -> UnitKind {
        match self {
            Opcode::Ldr | Opcode::Str | Opcode::LdrB | Opcode::StrB
            | Opcode::Push | Opcode::Pop => UnitKind::LoadStore,

            Opcode::B | Opcode::Beq | Opcode::Bne | Opcode::Blt
            | Opcode::Bge | Opcode::Bgt | Opcode::Ble
            | Opcode::Bl | Opcode::Bx => UnitKind::Branch,

            Opcode::Nop | Opcode::Halt => UnitKind::Alu,

            _ => UnitKind::Alu,
        }
    }

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
            Opcode::Ldr | Opcode::LdrB => 2,
            Opcode::Str | Opcode::StrB => 2,
            Opcode::Push | Opcode::Pop => 2, // simplified
            _ => 1,
        }
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
            Opcode::B => "B", Opcode::Beq => "BEQ", Opcode::Bne => "BNE",
            Opcode::Blt => "BLT", Opcode::Bge => "BGE",
            Opcode::Bgt => "BGT", Opcode::Ble => "BLE",
            Opcode::Bl => "BL", Opcode::Bx => "BX",
            Opcode::Push => "PUSH", Opcode::Pop => "POP",
            Opcode::Nop => "NOP", Opcode::Halt => "HALT",
        };
        write!(f, "{}", s)
    }
}

// ---------------------------------------------------------------------------
// Execution unit categories
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitKind {
    Alu,
    LoadStore,
    Branch,
}

// ---------------------------------------------------------------------------
// Instruction representation
// ---------------------------------------------------------------------------

/// A fully decoded instruction ready for execution.
#[derive(Debug, Clone)]
pub struct Instruction {
    pub opcode: Opcode,
    pub rd: Option<usize>,       // destination register
    pub rn: Option<usize>,       // first source register
    pub rm: Option<usize>,       // second source register
    pub imm: i32,                // immediate value / branch offset
    pub reg_list: Vec<usize>,    // for PUSH / POP
    pub addr: usize,             // instruction address (index in program)
    pub text: String,            // original assembly text for debug
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
            Opcode::Ldr | Opcode::LdrB => {
                if let Some(rn) = self.rn { srcs.push(rn); }
            }
            // Memory stores: base register + data register
            Opcode::Str | Opcode::StrB => {
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
            // Store writes no register
            Opcode::Str | Opcode::StrB => {}
            // Push modifies SP
            Opcode::Push => { dsts.push(SP); }
            // Pop modifies listed registers + SP
            Opcode::Pop => {
                dsts.push(SP);
                dsts.extend(self.reg_list.iter());
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
