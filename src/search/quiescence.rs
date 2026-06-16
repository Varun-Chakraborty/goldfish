use crate::{
    GameState,
    eval::eval,
    game::MoveGenMode,
    position::{
        DrawReason::{FiftyMoveRule, InsufficientMaterial, ThreefoldRepetition},
        Position,
        PositionStatus::{Checkmate, Draw, Ongoing},
    },
    search::{DRAW, MATE, SearchContext, ordermoves::ordermoves},
    transposition::Bound,
    types::Color,
};

pub fn quiescence(
    gs: &mut GameState,
    ply: u32,
    depth: u32,
    mut alpha: i32,
    mut beta: i32,
    search_context: &mut SearchContext,
) -> i32 {
    search_context.search_stats.search_counters.qnodes += 1;
    if let Some(entry) = search_context.tt.probe(gs.zobrist, ply, None) {
        match entry.bound {
            Bound::Exact => {
                return entry.score;
            }
            Bound::Lower => {
                alpha = alpha.max(entry.score);
            }
            Bound::Upper => {
                beta = beta.min(entry.score);
            }
        }
        if alpha >= beta {
            search_context.search_stats.search_counters.cutoffs += 1;
            return beta;
        }
    }

    let position = Position::quick_draw_analysis(gs);

    if position != Ongoing {
        return match position {
            Draw(FiftyMoveRule) | Draw(ThreefoldRepetition) => DRAW,
            Draw(InsufficientMaterial) => DRAW,
            _ => unreachable!(),
        };
    }

    let in_check = gs.in_check(gs.turn);

    if !in_check {
        let stand_pat = match gs.turn {
            Color::White => eval(gs).0,
            Color::Black => -eval(gs).0,
        };

        if stand_pat >= beta {
            search_context.search_stats.search_counters.cutoffs += 1;
            return beta;
        }

        alpha = alpha.max(stand_pat);
    }

    let mut legal = if in_check {
        let position = Position::analyse(gs);
        if position.status == Ongoing
            && let Some(moves) = position.legal_moves
        {
            moves
        } else if position.status == Checkmate {
            return -MATE + ply as i32;
        } else {
            return DRAW;
        }
    }
    /*  else if depth < 2 {
        gs.legal_moves(MoveGenMode::All)
    } */
    else {
        gs.legal_moves(MoveGenMode::CaptureOnly)
    };

    ordermoves(
        &mut legal,
        gs.turn,
        None,
        None,
        search_context.history_heuristics,
    );
    let total_moves = legal.len();
    search_context.search_stats.search_counters.available_moves += total_moves as u64;

    let mut score = alpha;

    for m in legal {
        if search_context.should_stop() {
            search_context.stopped = true;
            return score;
        }
        let undo = gs.make_move(m);
        let result = -quiescence(gs, ply + 1, depth + 1, -beta, -alpha, search_context);
        gs.unmake_move(undo);

        if search_context.stopped {
            return score;
        }

        search_context.search_stats.search_counters.examined_moves += 1;
        score = score.max(result);
        alpha = alpha.max(score);
        if alpha >= beta {
            search_context.search_stats.search_counters.cutoffs += 1;
            break;
        }
    }

    score
}
