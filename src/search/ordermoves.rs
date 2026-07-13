use std::cmp::Reverse;

use crate::{
    GameState,
    history::HistoryHeuristic,
    see::see,
    types::{Color, Move},
};

const PV_BONUS: i32 = 2_000_000;
const TT_BONUS: i32 = 1_000_000;

pub fn ordermoves(
    gs: &mut GameState,
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
            let see = see(gs, m);
            let mvv_lva = captured.value() as i32 * 100 - m.piece.value() as i32;
            score += see * 100;
            score += mvv_lva;
        } else {
            score += history.score(side_to_move, m) as i32;
        }

        Reverse(score)
    });
}
