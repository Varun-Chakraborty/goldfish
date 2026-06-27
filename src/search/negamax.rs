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
        search_types::{SearchContext, SearchResult},
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
    search_context: &mut SearchContext,
) -> SearchResult {
    let org_alpha = alpha;
    search_context.search_stats.search_counters.nodes += 1;

    if let Some(entry) = search_context.tt.probe(gs.zobrist, ply, Some(depth)) {
        match entry.bound {
            Bound::Exact => {
                search_context.pv_table.length[ply as usize] = 0;
                return SearchResult {
                    score: entry.score,
                    best_move: entry.best_move,
                };
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
            search_context.pv_table.length[ply as usize] = 0;
            return SearchResult {
                score: entry.score,
                best_move: entry.best_move,
            };
        }
    }

    if depth == 0 {
        search_context.search_stats.search_counters.leaf_nodes += 1;
        let result = quiescence(gs, ply, 0, alpha, beta, search_context);
        search_context.tt.store(
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
        search_context.pv_table.length[ply as usize] = 0;
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

            search_context.tt.store(
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

            search_context.pv_table.length[ply as usize] = 0;
            return search_result;
        }
        Draw(Stalemate) | Draw(InsufficientMaterial) => {
            let search_result = SearchResult {
                score: DRAW,
                best_move: None,
            };

            search_context.tt.store(
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

            search_context.pv_table.length[ply as usize] = 0;
            return search_result;
        }
        Draw(FiftyMoveRule) | Draw(ThreefoldRepetition) => {
            search_context.pv_table.length[ply as usize] = 0;
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

    let tt_best_move = search_context
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
        search_context.history_heuristics,
    );

    let total_moves = legal.len();
    search_context.search_stats.search_counters.available_moves += total_moves as u64;

    for m in legal {
        if search_context.should_stop() {
            search_context.stopped = true;
            return SearchResult {
                score: search_result.score,
                best_move: search_result.best_move,
            };
        }
        let undo = gs.make_move(m);
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
            search_context,
        );
        gs.unmake_move(undo);

        if search_context.stopped {
            return SearchResult {
                score: search_result.score,
                best_move: search_result.best_move,
            };
        }

        result.score = -result.score;

        if result.score > search_result.score {
            search_result.score = result.score;
            search_result.best_move = Some(m);
            search_context.pv_table.table[ply as usize][ply as usize] = search_result.best_move;
            search_context.pv_table.length[ply as usize] =
                search_context.pv_table.length[ply as usize + 1] + 1;
            for i in 1..search_context.pv_table.length[ply as usize] {
                search_context.pv_table.table[ply as usize][(ply + i) as usize] =
                    search_context.pv_table.table[ply as usize + 1][(ply + i) as usize];
            }
        }
        alpha = alpha.max(search_result.score);
        search_context.search_stats.search_counters.examined_moves += 1;
        if alpha >= beta {
            if m.captured.is_none() {
                search_context
                    .history_heuristics
                    .reward(gs.turn, &m, depth as u16);
            }
            search_context.search_stats.search_counters.cutoffs += 1;
            break;
        }
    }

    search_context.tt.store(
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
