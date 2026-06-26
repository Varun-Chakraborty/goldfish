use crate::types::{Color, Move};

pub struct HistoryHeuristic {
    history: [[[u16; 64]; 64]; 2],
}

impl HistoryHeuristic {
    pub fn new() -> Self {
        HistoryHeuristic {
            history: [[[0; 64]; 64]; 2],
        }
    }

    pub fn reward(&mut self, side_to_move: Color, m: &Move, depth: u16) {
        let side = side_to_move as usize;
        let from = m.from.idx();
        let to = m.to.idx();

        let entry = &mut self.history[side][from][to];
        *entry = entry.saturating_add(depth.saturating_mul(depth));
    }

    pub fn score(&self, side_to_move: Color, m: &Move) -> u16 {
        self.history[side_to_move as usize][m.from.idx()][m.to.idx()]
    }
}
