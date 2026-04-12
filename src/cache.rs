/// L1 data cache simulation (direct-mapped).
///
/// Models a simple direct-mapped cache to demonstrate the impact of memory
/// hierarchy on processor performance.  Configurable size; default is 256
/// lines of 1 word (4 bytes) each = 1 KiB.
///
/// Parameters:
///   - Hit:  no additional penalty (base load latency applies)
///   - Miss: +8 extra cycles (modelling main-memory fetch)

const DEFAULT_NUM_LINES: usize = 256;
pub const MISS_PENALTY: u32 = 8;

#[derive(Clone)]
struct CacheLine {
    valid: bool,
    tag: u32,
}

#[derive(Clone)]
pub struct Cache {
    lines: Vec<CacheLine>,
    num_lines: usize,
    pub hits: u64,
    pub misses: u64,
}

impl Cache {
    pub fn new() -> Self {
        Self::with_size(DEFAULT_NUM_LINES)
    }

    pub fn with_size(num_lines: usize) -> Self {
        Cache {
            lines: vec![CacheLine { valid: false, tag: 0 }; num_lines],
            num_lines,
            hits: 0,
            misses: 0,
        }
    }

    /// Access the cache for a word-aligned address.
    /// Returns `true` on a hit, `false` on a miss (and installs the line).
    pub fn access(&mut self, addr: u32) -> bool {
        let word_addr = addr >> 2; // word-aligned
        let index = (word_addr as usize) % self.num_lines;
        let tag = word_addr / (self.num_lines as u32);

        let line = &mut self.lines[index];
        if line.valid && line.tag == tag {
            self.hits += 1;
            true
        } else {
            // Miss — install the line
            line.valid = true;
            line.tag = tag;
            self.misses += 1;
            false
        }
    }

    #[allow(dead_code)]
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 { 0.0 } else { self.hits as f64 / total as f64 }
    }

    /// Reset cache state (but keep configuration).
    #[allow(dead_code)]
    pub fn reset(&mut self) {
        for line in &mut self.lines {
            line.valid = false;
            line.tag = 0;
        }
        self.hits = 0;
        self.misses = 0;
    }
}
