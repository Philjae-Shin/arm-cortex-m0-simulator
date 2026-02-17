/// Branch prediction strategies.
///
/// Implements three common predictors:
///   1. **Static (always not-taken)** — baseline
///   2. **OneBit** — 1-bit saturating, flips on every misprediction
///   3. **TwoBit** — 2-bit saturating counter (the classic Smith predictor)

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Predictor selection
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PredictorKind {
    Static,   // always predict not-taken
    OneBit,   // 1-bit predictor per branch address
    TwoBit,   // 2-bit saturating counter per branch address
}

impl PredictorKind {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "static" | "none" => Some(PredictorKind::Static),
            "1bit" | "onebit" | "one" => Some(PredictorKind::OneBit),
            "2bit" | "twobit" | "two" => Some(PredictorKind::TwoBit),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            PredictorKind::Static => "Static (always not-taken)",
            PredictorKind::OneBit => "1-bit",
            PredictorKind::TwoBit => "2-bit saturating counter",
        }
    }
}

// ---------------------------------------------------------------------------
// Branch predictor
// ---------------------------------------------------------------------------

pub struct BranchPredictor {
    kind: PredictorKind,
    /// Per-PC state. For OneBit: 0 = not-taken, 1 = taken.
    /// For TwoBit: 0..3 where 0-1 = not-taken, 2-3 = taken.
    table: HashMap<usize, u8>,
}

impl BranchPredictor {
    pub fn new(kind: PredictorKind) -> Self {
        BranchPredictor {
            kind,
            table: HashMap::new(),
        }
    }

    /// Predict whether the branch at `pc` will be taken.
    pub fn predict(&self, pc: usize) -> bool {
        match self.kind {
            PredictorKind::Static => false, // always predict not-taken

            PredictorKind::OneBit => {
                let state = self.table.get(&pc).copied().unwrap_or(0);
                state == 1
            }

            PredictorKind::TwoBit => {
                let state = self.table.get(&pc).copied().unwrap_or(1);
                // 0 = strongly not-taken, 1 = weakly not-taken
                // 2 = weakly taken,       3 = strongly taken
                state >= 2
            }
        }
    }

    /// Update the predictor after the actual branch outcome is known.
    pub fn update(&mut self, pc: usize, taken: bool) {
        match self.kind {
            PredictorKind::Static => { /* immutable */ }

            PredictorKind::OneBit => {
                self.table.insert(pc, if taken { 1 } else { 0 });
            }

            PredictorKind::TwoBit => {
                let state = self.table.get(&pc).copied().unwrap_or(1);
                let new_state = if taken {
                    if state < 3 { state + 1 } else { 3 }
                } else {
                    if state > 0 { state - 1 } else { 0 }
                };
                self.table.insert(pc, new_state);
            }
        }
    }

    #[allow(dead_code)]
    pub fn kind(&self) -> PredictorKind { self.kind }
}
