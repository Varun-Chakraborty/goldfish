use std::cmp::max;
use crate::{
    GameState,
    position::{
        DrawReason::{FiftyMoveRule, InsufficientMaterial, Stalemate, ThreefoldRepetition},
        Position,
        PositionStatus::{Checkmate, Draw, Ongoing},
    },
    search::{
        DRAW, MATE,
        ordermoves::ordermoves,
        quiescence,
        search_types::{
            SearchContext, SearchResult,
            SearchType::{self, Scout},
        },
    },
    transposition::{Bound, TTEntry},
    types::Move,
};

pub fn negamax(
    gs: &mut GameState,
    depth: u32,
    ply: u32,
    mut alpha: i32,
    mut beta: i32,
    pv: &Option<Vec<Move>>,
    pvpath: bool,
    search_type: SearchType,
    ctx: &mut SearchContext,
) -> SearchResult {
    ctx.seldepth = max(ctx.seldepth, ply);
    let org_alpha = alpha;
    ctx.search_stats.search_counters.nodes += 1;

    if let Some(entry) = ctx.tt.probe(gs.zobrist, ply, Some(depth)) {
        match entry.bound {
            Bound::Exact if search_type == Scout => {
                return SearchResult {
                    score: entry.score,
                    best_move: entry.best_move,
                };
            }
            Bound::Exact => {}
            Bound::Lower => {
                alpha = alpha.max(entry.score);
            }
            Bound::Upper => {
                beta = beta.min(entry.score);
            }
        }
        if alpha >= beta {
            ctx.search_stats.search_counters.cutoffs += 1;
            ctx.pv_table.length[ply as usize] = 0;
            return SearchResult {
                score: entry.score,
                best_move: entry.best_move,
            };
        }
    }

    if depth == 0 {
        ctx.search_stats.search_counters.leaf_nodes += 1;
        let result = quiescence(gs, ply, 0, alpha, beta, ctx);
        if search_type != Scout {
            ctx.tt.store(
                gs.zobrist,
                ply,
                TTEntry {
                    key: gs.zobrist,
                    depth,
                    score: result,
                    best_move: None,
                    bound: Bound::Exact,
                },
            );
            ctx.pv_table.length[ply as usize] = 0;
        }
        return SearchResult {
            score: result,
            best_move: None,
        };
    }

    let position = Position::analyse(gs);
    match position.status {
        Checkmate => {
            let search_result = SearchResult {
                score: -MATE + ply as i32,
                best_move: None,
            };

            ctx.tt.store(
                gs.zobrist,
                ply,
                TTEntry {
                    key: gs.zobrist,
                    depth,
                    score: search_result.score,
                    best_move: search_result.best_move,
                    bound: Bound::Exact,
                },
            );

            ctx.pv_table.length[ply as usize] = 0;
            return search_result;
        }
        Draw(Stalemate) | Draw(InsufficientMaterial) => {
            let search_result = SearchResult {
                score: DRAW,
                best_move: None,
            };

            ctx.tt.store(
                gs.zobrist,
                ply,
                TTEntry {
                    key: gs.zobrist,
                    depth,
                    score: search_result.score,
                    best_move: search_result.best_move,
                    bound: Bound::Exact,
                },
            );

            ctx.pv_table.length[ply as usize] = 0;
            return search_result;
        }
        Draw(FiftyMoveRule) | Draw(ThreefoldRepetition) => {
            ctx.pv_table.length[ply as usize] = 0;
            return SearchResult {
                score: DRAW,
                best_move: None,
            };
        }
        Ongoing => {}
    }

    let mut search_result = SearchResult {
        score: -MATE,
        best_move: None,
    };

    let mut legal = position.legal_moves.expect("No legal moves");

    let tt_best_move = ctx
        .tt
        .probe(gs.zobrist, ply, None)
        .and_then(|entry| entry.best_move);
    let pv_best_move = pvpath
        .then(|| pv.as_ref().and_then(|pv| pv.get(ply as usize).copied()))
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

    let mut searched_quiets = vec![];

    for (i, &m) in legal.iter().enumerate() {
        if ctx.should_stop() {
            ctx.stopped = true;
            return SearchResult {
                score: search_result.score,
                best_move: search_result.best_move,
            };
        }
        let undo = gs.make_move(m);
        if i > 0 && depth > 2 {
            let reduction = if depth >= 8 && i >= 8 && m.captured.is_none() {
                2
            } else if depth >= 4 && i >= 6 && m.captured.is_none() {
                1
            } else {
                0
            };
            let result = negamax(
                gs,
                depth - 1 - reduction,
                ply + 1,
                -alpha - 1,
                -alpha,
                pv,
                pvpath
                    && pv
                        .as_ref()
                        .and_then(|pv| pv.get(ply as usize))
                        .is_some_and(|bm| *bm == m),
                Scout,
                ctx,
            );

            if reduction > 0 {
                ctx.search_stats.search_counters.reduced_searches += 1;
            }

            if -result.score <= alpha {
                if reduction > 0 {
                    ctx.search_stats.search_counters.reduced_fail_low += 1;
                }
                gs.unmake_move(undo);
                continue;
            }

            if reduction > 0 {
                ctx.search_stats.search_counters.reduced_fail_high += 1;
                let result = negamax(
                    gs,
                    depth - 1,
                    ply + 1,
                    -alpha - 1,
                    -alpha,
                    pv,
                    pvpath
                        && pv
                            .as_ref()
                            .and_then(|pv| pv.get(ply as usize))
                            .is_some_and(|bm| *bm == m),
                    Scout,
                    ctx,
                );

                if -result.score <= alpha {
                    ctx.search_stats.search_counters.verified_fail_low += 1;
                    gs.unmake_move(undo);
                    continue;
                }

                ctx.search_stats.search_counters.verified_fail_high += 1;
            }
        }
        let mut result = negamax(
            gs,
            depth - 1,
            ply + 1,
            -beta,
            -alpha,
            pv,
            pvpath
                && pv
                    .as_ref()
                    .and_then(|pv| pv.get(ply as usize))
                    .is_some_and(|bm| *bm == m),
            search_type,
            ctx,
        );
        gs.unmake_move(undo);

        if ctx.stopped {
            return SearchResult {
                score: search_result.score,
                best_move: search_result.best_move,
            };
        }

        result.score = -result.score;

        if result.score > search_result.score {
            search_result.score = result.score;
            search_result.best_move = Some(m);
            if search_type != Scout {
                ctx.pv_table.table[ply as usize][ply as usize] = search_result.best_move;
                ctx.pv_table.length[ply as usize] = ctx.pv_table.length[ply as usize + 1] + 1;
                for i in 1..ctx.pv_table.length[ply as usize] {
                    ctx.pv_table.table[ply as usize][(ply + i) as usize] =
                        ctx.pv_table.table[ply as usize + 1][(ply + i) as usize];
                }
            }
        }
        alpha = alpha.max(search_result.score);
        ctx.search_stats.search_counters.examined_moves += 1;
        if alpha >= beta {
            if i == 0 {
                ctx.search_stats.search_counters.first_move_cutoffs += 1;
            }
            if m.captured.is_none() && search_type != Scout {
                ctx.history_heuristics
                    .reward(gs.turn.opponent(), &m, depth as u16);

                for m in searched_quiets {
                    ctx.history_heuristics
                        .penalize(gs.turn.opponent(), &m, depth as u16);
                }
            }
            ctx.search_stats.search_counters.cutoffs += 1;
            break;
        }
        if m.captured.is_none() {
            searched_quiets.push(m);
        }
    }

    ctx.tt.store(
        gs.zobrist,
        ply,
        TTEntry {
            key: gs.zobrist,
            depth,
            score: search_result.score,
            best_move: search_result.best_move,
            bound: if search_result.score >= beta {
                Bound::Lower
            } else if search_result.score <= org_alpha {
                Bound::Upper
            } else {
                Bound::Exact
            },
        },
    );

    search_result
}
