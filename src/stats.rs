/// Performance counters collected during simulation.

use std::fmt;

#[derive(Clone, Default)]
pub struct Stats {
    pub cycles: u64,
    pub instructions_executed: u64,
    pub instructions_fetched: u64,
    pub branches_total: u64,
    pub branches_taken: u64,
    pub branches_predicted: u64,     // correct predictions
    pub branches_mispredicted: u64,  // wrong predictions
    pub pipeline_stalls: u64,
    pub pipeline_flushes: u64,
    pub mem_reads: u64,
    pub mem_writes: u64,
}

impl Stats {
    pub fn new() -> Self {
        Self::default()
    }

    /// Instructions per cycle.
    pub fn ipc(&self) -> f64 {
        if self.cycles == 0 { 0.0 }
        else { self.instructions_executed as f64 / self.cycles as f64 }
    }

    /// Cycles per instruction.
    #[allow(dead_code)]
    pub fn cpi(&self) -> f64 {
        if self.instructions_executed == 0 { 0.0 }
        else { self.cycles as f64 / self.instructions_executed as f64 }
    }

    /// Branch prediction accuracy (0.0 – 1.0).
    pub fn prediction_accuracy(&self) -> f64 {
        let total = self.branches_predicted + self.branches_mispredicted;
        if total == 0 { 0.0 }
        else { self.branches_predicted as f64 / total as f64 }
    }
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "=== Simulation Statistics ===")?;
        writeln!(f, "  Cycles executed       : {}", self.cycles)?;
        writeln!(f, "  Instructions executed  : {}", self.instructions_executed)?;
        writeln!(f, "  IPC                   : {:.4}", self.ipc())?;
        writeln!(f, "  CPI                   : {:.4}", self.cpi())?;
        writeln!(f, "  Branches (total)      : {}", self.branches_total)?;
        writeln!(f, "  Branches taken        : {}", self.branches_taken)?;
        if self.branches_predicted + self.branches_mispredicted > 0 {
            writeln!(f, "  Predictions correct   : {}", self.branches_predicted)?;
            writeln!(f, "  Predictions wrong     : {}", self.branches_mispredicted)?;
            writeln!(f, "  Prediction accuracy   : {:.2}%",
                self.prediction_accuracy() * 100.0)?;
        }
        if self.pipeline_stalls > 0 {
            writeln!(f, "  Pipeline stalls       : {}", self.pipeline_stalls)?;
        }
        if self.pipeline_flushes > 0 {
            writeln!(f, "  Pipeline flushes      : {}", self.pipeline_flushes)?;
        }
        writeln!(f, "  Memory reads          : {}", self.mem_reads)?;
        writeln!(f, "  Memory writes         : {}", self.mem_writes)?;
        Ok(())
    }
}
