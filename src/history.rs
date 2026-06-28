use crate::types::{Color, Move};

pub struct HistoryHeuristic {
    history: [[[i16; 64]; 64]; 2],
}

impl HistoryHeuristic {
    pub fn new() -> Self {
        HistoryHeuristic {
            history: [[[0; 64]; 64]; 2],
        }
    }

    pub fn reward(&mut self, side_to_move: Color, m: &Move, depth: u16) {
        if m.captured.is_some() {
            return;
        }
        let side = side_to_move as usize;
        let from = m.from.idx();
        let to = m.to.idx();

        let entry = &mut self.history[side][from][to];
        let bonus = depth.saturating_mul(depth) as i16;
        *entry = entry.saturating_add(bonus);
    }

    pub fn penalize(&mut self, side_to_move: Color, m: &Move, depth: u16) {
        let side = side_to_move as usize;
        let from = m.from.idx();
        let to = m.to.idx();

        let entry = &mut self.history[side][from][to];
        let penalty = depth.saturating_mul(depth) as i16;
        *entry = entry.saturating_sub(penalty);
    }

    pub fn score(&self, side_to_move: Color, m: &Move) -> i16 {
        self.history[side_to_move as usize][m.from.idx()][m.to.idx()]
    }
}
