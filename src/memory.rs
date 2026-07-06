/// Flat byte-addressable memory model.
///
/// The processor uses a Harvard-style split where instructions are stored in a
/// separate `Vec<Instruction>` (the program image) and this `Memory` struct
/// holds only *data*.  This keeps the simulator simple while still modelling
/// realistic load/store latencies.

const MEM_SIZE: usize = 1 << 16; // 64 KiB data memory

pub struct Memory {
    data: Vec<u8>,
}

impl Memory {
    pub fn new() -> Self {
        Memory {
            data: vec![0u8; MEM_SIZE],
        }
    }

    // byte access

    pub fn read_byte(&self, addr: u32) -> u8 {
        let idx = addr as usize;
        if idx < self.data.len() {
            self.data[idx]
        } else {
            eprintln!("[MEM] read_byte out of bounds: 0x{:08X}", addr);
            0
        }
    }

    pub fn write_byte(&mut self, addr: u32, val: u8) {
        let idx = addr as usize;
        if idx < self.data.len() {
            self.data[idx] = val;
        } else {
            eprintln!("[MEM] write_byte out of bounds: 0x{:08X}", addr);
        }
    }

    // word (32-bit, little-endian) access

    pub fn read_word(&self, addr: u32) -> u32 {
        let idx = addr as usize;
        if idx + 3 < self.data.len() {
            u32::from_le_bytes([
                self.data[idx],
                self.data[idx + 1],
                self.data[idx + 2],
                self.data[idx + 3],
            ])
        } else {
            eprintln!("[MEM] read_word out of bounds: 0x{:08X}", addr);
            0
        }
    }

    pub fn write_word(&mut self, addr: u32, val: u32) {
        let idx = addr as usize;
        if idx + 3 < self.data.len() {
            let bytes = val.to_le_bytes();
            self.data[idx]     = bytes[0];
            self.data[idx + 1] = bytes[1];
            self.data[idx + 2] = bytes[2];
            self.data[idx + 3] = bytes[3];
        } else {
            eprintln!("[MEM] write_word out of bounds: 0x{:08X}", addr);
        }
    }

    /// Pre-load a block of data starting at `base_addr`.
    /// Used by the assembler's `.data` section to initialise memory contents.
    pub fn load_data(&mut self, base_addr: u32, words: &[u32]) {
        for (i, &w) in words.iter().enumerate() {
            self.write_word(base_addr + (i as u32) * 4, w);
        }
    }

    /// Dump a region for debug inspection.
    #[allow(dead_code)]
    pub fn dump(&self, start: u32, count: usize) {
        for i in 0..count {
            let addr = start + (i as u32) * 4;
            println!("  [0x{:04X}] = 0x{:08X}", addr, self.read_word(addr));
        }
    }
}
