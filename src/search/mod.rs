mod negamax;
mod ordermoves;
mod quiescence;
pub mod search_types;

use std::{
    mem,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

use crate::{
    EngineEvent,
    game::{GameState, MoveGenMode},
    history::HistoryHeuristic,
    search::{
        negamax::negamax,
        quiescence::quiescence,
        search_types::{
            EngineLimits, IterationInfo, PVTable, Score, SearchContext, SearchStats,
        },
    },
    transposition::TranspositionTable,
};

pub const MATE: i32 = 32000;
pub const MAX_PLY: u32 = 256;
const DRAW: i32 = 0;

pub fn iterative_deepening<F>(
    gs: &mut GameState,
    limits: EngineLimits,
    stop: &Arc<AtomicBool>,
    ponderhit: &Arc<AtomicBool>,
    tt: &mut Option<TranspositionTable>,
    history: &mut Option<HistoryHeuristic>,
    mut callback: F,
) where
    F: FnMut(EngineEvent),
{
    let tt = tt.get_or_insert_with(|| TranspositionTable::new_table(64));
    let history_heuristics = history.get_or_insert_with(HistoryHeuristic::new);
    let mut pv_table = PVTable::new();
    pv_table.table[0][0] = gs.legal_moves(MoveGenMode::All).get(0).copied();
    let mut pv = None;
    let mut nodes_in_last_iteration = 0;
    let mut depth = 1;

    let start = Instant::now();
    let mut nodes = 0;

    let mut search_context = SearchContext {
        stopped: false,
        stop,
        ponderhit,
        tt,
        search_stats: SearchStats::new(),
        pv_table: &mut pv_table,
        history_heuristics,
        start_time: start,
        ponder: limits.ponder,
        node_limit: limits.nodes,
    };
    loop {
        search_context.stopped = false;
        search_context.search_stats = SearchStats::new();

        let result = negamax(
            gs,
            depth,
            0,
            -MATE,
            MATE,
            &pv,
            true,
            &mut search_context,
        );

        if search_context.stopped || stop.load(Ordering::Relaxed) {
            break;
        }

        pv = search_context.pv_table.table[0][..search_context.pv_table.length[0] as usize]
            .iter()
            .copied()
            .collect();

        let duration = start.elapsed();
        let tt_stats = search_context.tt.stats();

        nodes += search_context.search_stats.search_counters.nodes;

        callback(EngineEvent::IterationInfo(IterationInfo {
            depth,
            seldepth: depth,
            score: score(result.score),
            raw_score: result.score,
            nodes,
            nps: (nodes as f64 / duration.as_secs_f64()).ceil() as u64,
            time: duration.as_millis(),
            best_line: pv.clone(),
            hashfull: tt_stats.hashfull as u64,
            search_stats: Some(SearchStats {
                tt_stats: mem::take(tt_stats),
                branching_factor: (search_context.search_stats.search_counters.nodes as f64)
                    .powf(1f64 / depth as f64),
                delta: search_context.search_stats.search_counters.nodes as i64
                    - nodes_in_last_iteration as i64,
                search_counters: search_context.search_stats.search_counters,
            }),
        }));

        depth += 1;
        if limits.depth.is_some_and(|max_depth| depth > max_depth) {
            break;
        }
        nodes_in_last_iteration = search_context.search_stats.search_counters.nodes;
    }

    let best_move = pv.as_ref().and_then(|pv| pv.get(0).copied());
    let ponder = pv.as_ref().and_then(|pv| pv.get(1).copied());
    callback(EngineEvent::SearchFinished { best_move, ponder });
}

fn score(score: i32) -> Score {
    let mate_threshold = MATE - MAX_PLY as i32;
    if score > mate_threshold {
        Score::Mate((MATE - score + 1) / 2)
    } else if score < -mate_threshold {
        Score::Mate((-MATE - score - 1) / 2)
    } else {
        Score::CP(score)
    }
}
