use crate::{
    GameState,
    game::MoveGenMode,
    position::{
        DrawReason::{FiftyMoveRule, InsufficientMaterial, ThreefoldRepetition},
        Position,
        PositionStatus::{Checkmate, Draw, Ongoing},
    },
    search::{DRAW, MATE, SearchContext, ordermoves::ordermoves},
    see::see,
    transposition::Bound,
    types::Color,
};
use std::cmp::max;

pub fn quiescence(
    gs: &mut GameState,
    ply: u32,
    depth: u32,
    mut alpha: i32,
    mut beta: i32,
    ctx: &mut SearchContext,
) -> i32 {
    ctx.seldepth = max(ctx.seldepth, ply);
    ctx.qsearch_stats.nodes += 1;
    if let Some(entry) = ctx.tt.probe(gs.zobrist, ply, None) {
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
            ctx.qsearch_stats.cutoffs += 1;
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
        let eval = gs.eval().score();
        let stand_pat = eval
            * match gs.turn {
                Color::White => 1,
                Color::Black => -1,
            };

        if stand_pat >= beta {
            ctx.qsearch_stats.cutoffs += 1;
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

    ordermoves(gs, &mut legal, gs.turn, None, None, ctx.history_heuristics);
    let total_moves = legal.len();
    ctx.qsearch_stats.available_moves += total_moves as u64;

    let mut score = alpha;

    for m in legal {
        if ctx.should_stop() {
            ctx.stopped = true;
            return score;
        }
        if !in_check && m.captured.is_some() {
            let see = see(gs, &m);
            if see < 0 {
                continue;
            }
        }
        let undo = gs.make_move(m);
        let result = -quiescence(gs, ply + 1, depth + 1, -beta, -alpha, ctx);
        gs.unmake_move(undo);

        if ctx.stopped {
            return score;
        }

        ctx.qsearch_stats.examined_moves += 1;
        score = score.max(result);
        alpha = alpha.max(score);
        if alpha >= beta {
            ctx.qsearch_stats.cutoffs += 1;
            break;
        }
    }

    score
}
