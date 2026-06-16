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
        self.history[side_to_move as usize][m.from.idx()][m.to.idx()] += depth * depth;
    }

    pub fn score(&self, side_to_move: Color, m: &Move) -> u16 {
        self.history[side_to_move as usize][m.from.idx()][m.to.idx()]
    }
}
