use crate::{
    EngineEvent, GameState,
    position::{
        DrawReason::Stalemate,
        Position,
        PositionStatus::{Checkmate, Draw, Ongoing},
    },
    search::{
        DRAW, MATE, MAX_PLY,
        negamax::negamax,
        ordermoves::ordermoves,
        search_types::{
            SearchContext,
            SearchType::{FullSearch, Scout},
        },
    },
    transposition::{Bound, TTEntry},
    types::Move,
};

#[derive(Copy, Clone, Debug)]
pub struct RootSearch {
    pub score: i32,
    pub pv: [Option<Move>; MAX_PLY as usize],
}

pub fn root_search(
    gs: &mut GameState,
    depth: u32,
    ctx: &mut SearchContext,
    lines: &mut Vec<RootSearch>,
    requested_number_of_pv: usize,
    mut callback: Option<&mut dyn FnMut(EngineEvent)>,
) {
    let mut alpha = -MATE;
    let mut best_scores = Vec::with_capacity(requested_number_of_pv);
    let org_alpha = alpha;
    ctx.search_stats.search_counters.nodes += 1;

    let position = Position::analyse(gs);
    let mut legal = position.legal_moves.expect("No legal lines");

    if legal.is_empty() {
        match position.status {
            Checkmate => {
                ctx.tt.store(
                    gs.zobrist,
                    0,
                    TTEntry {
                        key: gs.zobrist,
                        depth,
                        score: -MATE,
                        best_move: None,
                        bound: Bound::Exact,
                    },
                );
                return;
            }
            Draw(Stalemate) => {
                ctx.tt.store(
                    gs.zobrist,
                    0,
                    TTEntry {
                        key: gs.zobrist,
                        depth,
                        score: DRAW,
                        best_move: None,
                        bound: Bound::Exact,
                    },
                );
                return;
            }
            Draw(_) | Ongoing => {}
        }
    }

    let tt_best_move = ctx
        .tt
        .probe(gs.zobrist, 0, None)
        .and_then(|entry| entry.best_move);
    let pv_best_move = lines
        .first()
        .map(|line| line.pv.first().copied())
        .flatten()
        .flatten();

    ordermoves(
        &mut legal,
        gs.turn,
        tt_best_move,
        pv_best_move,
        ctx.history_heuristics,
    );

    let total_moves = legal.len();
    ctx.search_stats.search_counters.available_moves += total_moves as u64;

    for (i, &m) in legal.iter().enumerate() {
        if ctx.should_stop() {
            ctx.stopped = true;
            return;
        }
        if depth >= 10 {
            callback.as_mut().and_then(|callback| {
                callback(EngineEvent::CurrentMove {
                    depth,
                    move_: m,
                    number: i + 1,
                });
                Some(())
            });
        }
        let undo = gs.make_move(m);
        let pv = lines
            .iter()
            .take(requested_number_of_pv)
            .find(|r| r.pv[0].is_some_and(|move_| move_ == m))
            .map(|r| r.pv.as_slice());
        if i >= requested_number_of_pv && i > 0 && depth > 2 {
            let result = negamax(
                gs,
                depth - 1,
                1,
                -alpha - 1,
                -alpha,
                pv,
                pv.is_some(),
                Scout,
                ctx,
            );

            if -result.score <= alpha {
                gs.unmake_move(undo);
                continue;
            }
        }
        let mut result = negamax(
            gs,
            depth - 1,
            1,
            -MATE,
            MATE,
            pv,
            pv.is_some(),
            FullSearch,
            ctx,
        );
        gs.unmake_move(undo);

        if ctx.stopped {
            return;
        }

        result.score = -result.score;

        let mut pv = [None; MAX_PLY as usize];
        pv[0] = Some(m);
        for i in 1..=ctx.pv_table.length[1] {
            pv[(i) as usize] = ctx.pv_table.table[1][(i) as usize];
        }

        if let Some(idx) = lines.iter().position(|r| r.pv[0] == Some(m)) {
            lines.remove(idx);
        }

        let idx = match lines.binary_search_by(|r| r.score.cmp(&result.score).reverse()) {
            Ok(idx) | Err(idx) => idx,
        };
        lines.insert(
            idx,
            RootSearch {
                score: result.score,
                pv,
            },
        );

        let idx = match best_scores.binary_search_by(|s: &i32| s.cmp(&result.score).reverse()) {
            Ok(idx) | Err(idx) => idx,
        };
        best_scores.insert(idx, result.score);
        if best_scores.len() > requested_number_of_pv {
            best_scores.pop();
        }

        alpha = *best_scores.last().expect("Just inserted a value");

        ctx.search_stats.search_counters.examined_moves += 1;
    }

    let best = lines
        .first()
        .copied()
        .expect("There should be at least a legal move, if we reached here");
    ctx.tt.store(
        gs.zobrist,
        0,
        TTEntry {
            key: gs.zobrist,
            depth,
            score: best.score,
            best_move: best.pv[0],
            bound: if best.score >= MATE {
                Bound::Lower
            } else if best.score <= org_alpha {
                Bound::Upper
            } else {
                Bound::Exact
            },
        },
    );
}
