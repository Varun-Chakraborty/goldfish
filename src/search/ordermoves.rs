use std::cmp::Reverse;

use crate::{
    history::HistoryHeuristic,
    types::{Color, Move},
};

const PV_BONUS: i32 = 2_000_000;
const TT_BONUS: i32 = 1_000_000;
const CAPTURE_BONUS: i32 = 100_000;

pub fn ordermoves(
    moves: &mut [Move],
    side_to_move: Color,
    tt_best_move: Option<Move>,
    pv_best_move: Option<Move>,
    history: &HistoryHeuristic,
) {
    moves.sort_unstable_by_key(|m| {
        let mut score = 0;
        if pv_best_move == Some(*m) {
            score += PV_BONUS;
        }
        if tt_best_move == Some(*m) {
            score += TT_BONUS;
        }
        if let Some(captured) = m.captured {
            score += CAPTURE_BONUS;
            score += captured.value() * 100 - m.piece.value();
        } else {
            score += history.score(side_to_move, m) as i32;
        }

        Reverse(score)
    });
}
