/// Register file with CPSR flag management.
///
/// Models 16 general-purpose 32-bit registers (R0-R15) and a condition-flag
/// register (CPSR) with N, Z, C, V bits.

use crate::isa::{NUM_REGS, SP, PC, reg_name};
use std::fmt;

// ---------------------------------------------------------------------------
// CPSR flag positions
// ---------------------------------------------------------------------------

const FLAG_N: u32 = 1 << 31; // Negative
const FLAG_Z: u32 = 1 << 30; // Zero
const FLAG_C: u32 = 1 << 29; // Carry
const FLAG_V: u32 = 1 << 28; // Overflow

// ---------------------------------------------------------------------------
// Register file
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct RegFile {
    gpr: [u32; NUM_REGS],
    cpsr: u32,
}

impl RegFile {
    /// Create a fresh register file with SP initialised to top of data memory.
    pub fn new(stack_top: u32) -> Self {
        let mut rf = RegFile {
            gpr: [0u32; NUM_REGS],
            cpsr: 0,
        };
        rf.gpr[SP] = stack_top;
        rf
    }

    // -- GPR access ---------------------------------------------------------

    pub fn get(&self, r: usize) -> u32 {
        self.gpr[r]
    }

    pub fn set(&mut self, r: usize, val: u32) {
        if r == PC {
            // PC writes are handled by the simulator control flow;
            // silently allow but warn during debug.
        }
        self.gpr[r] = val;
    }

    // -- CPSR flag queries ---------------------------------------------------

    pub fn flag_n(&self) -> bool { self.cpsr & FLAG_N != 0 }
    pub fn flag_z(&self) -> bool { self.cpsr & FLAG_Z != 0 }
    pub fn flag_c(&self) -> bool { self.cpsr & FLAG_C != 0 }
    pub fn flag_v(&self) -> bool { self.cpsr & FLAG_V != 0 }

    // -- CPSR flag updates ---------------------------------------------------

    /// Update N and Z flags from a result value.
    pub fn set_nz(&mut self, result: u32) {
        // Clear N and Z
        self.cpsr &= !(FLAG_N | FLAG_Z);
        if result == 0 {
            self.cpsr |= FLAG_Z;
        }
        if result & 0x8000_0000 != 0 {
            self.cpsr |= FLAG_N;
        }
    }

    /// Update all four flags after an addition: result = a + b.
    pub fn set_flags_add(&mut self, a: u32, b: u32, result: u32) {
        self.set_nz(result);

        // Carry: unsigned overflow
        self.cpsr &= !FLAG_C;
        if (result as u64) < (a as u64) + (b as u64) {
            // This detects wraparound
        }
        // More robust: compute via 64-bit
        let wide = (a as u64).wrapping_add(b as u64);
        if wide > 0xFFFF_FFFF {
            self.cpsr |= FLAG_C;
        }

        // Overflow: signed overflow (both operands same sign, result different)
        self.cpsr &= !FLAG_V;
        let sa = (a as i32) < 0;
        let sb = (b as i32) < 0;
        let sr = (result as i32) < 0;
        if sa == sb && sr != sa {
            self.cpsr |= FLAG_V;
        }
    }

    /// Update all four flags after a subtraction: result = a - b.
    pub fn set_flags_sub(&mut self, a: u32, b: u32, result: u32) {
        self.set_nz(result);

        // Carry: ARM convention — carry is SET when there is NO borrow.
        self.cpsr &= !FLAG_C;
        if a >= b {
            self.cpsr |= FLAG_C;
        }

        // Overflow: signed
        self.cpsr &= !FLAG_V;
        let sa = (a as i32) < 0;
        let sb = (b as i32) < 0;
        let sr = (result as i32) < 0;
        if sa != sb && sr != sa {
            self.cpsr |= FLAG_V;
        }
    }

    /// Raw CPSR value (used by reservation-station snapshots).
    pub fn cpsr_raw(&self) -> u32 { self.cpsr }
    pub fn set_cpsr_raw(&mut self, v: u32) { self.cpsr = v; }
}

impl fmt::Display for RegFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "--- Register File ---")?;
        for i in 0..NUM_REGS {
            write!(f, "  {:>3} = 0x{:08X}", reg_name(i), self.gpr[i])?;
            if (i + 1) % 4 == 0 { writeln!(f)?; } else { write!(f, "  ")?; }
        }
        writeln!(f, "  CPSR = 0x{:08X}  [N={} Z={} C={} V={}]",
            self.cpsr,
            self.flag_n() as u8, self.flag_z() as u8,
            self.flag_c() as u8, self.flag_v() as u8)
    }
}
