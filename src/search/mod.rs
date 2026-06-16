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
    game::GameState,
    history::HistoryHeuristic,
    search::{
        negamax::negamax,
        quiescence::quiescence,
        search_types::{
            EngineEvent, EngineLimits, IterationInfo, PVTable, Score, SearchContext, SearchStats,
        },
    },
    transposition::{TTStats, TranspositionTable},
};

const MATE: i32 = 32000;
const MAX_PLY: i32 = 256;
const DRAW: i32 = 0;

pub fn iterative_deepening<F>(
    gs: &mut GameState,
    limits: EngineLimits,
    stop: &Arc<AtomicBool>,
    tt: &mut Option<TranspositionTable>,
    history: &mut Option<HistoryHeuristic>,
    mut callback: F,
) where
    F: FnMut(EngineEvent),
{
    let depth = limits.depth.unwrap_or(256);

    let tt = tt.get_or_insert_with(|| TranspositionTable::new_table(64, MATE - MAX_PLY));
    let history_heuristics = history.get_or_insert_with(|| HistoryHeuristic::new());
    let mut pv_table = PVTable::new();
    let mut pv = None;
    let mut nodes_in_last_iteration = 0;

    for depth in 1..=depth {
        let mut search_context = SearchContext {
            stopped: false,
            stop,
            tt,
            search_stats: SearchStats::new(),
            pv_table: &mut pv_table,
            history_heuristics,
        };
        let time = Instant::now();
        let result = negamax(gs, depth, 0, -MATE, MATE, &pv, true, &mut search_context);

        if search_context.stopped || stop.load(Ordering::Relaxed) {
            callback(EngineEvent::SearchStopped(
                search_context.pv_table.table[0][0],
            ));
            return;
        }

        pv = search_context.pv_table.table[0][..search_context.pv_table.length[0] as usize]
            .iter()
            .copied()
            .collect();

        let duration = time.elapsed();
        let tt_stats = search_context.tt.stats();
        callback(EngineEvent::IterationInfo(IterationInfo {
            depth,
            seldepth: depth,
            score: score(result.score, MATE - MAX_PLY),
            raw_score: result.score,
            nodes: search_context.search_stats.search_counters.nodes,
            nps: ((search_context.search_stats.search_counters.nodes
                + search_context.search_stats.search_counters.qnodes) as f64
                / duration.as_secs_f64())
            .ceil() as u64,
            best_line: pv.clone(),
            search_stats: Some(SearchStats {
                tt_stats: mem::replace(tt_stats, TTStats::default()),
                branching_factor: (search_context.search_stats.search_counters.nodes as f64)
                    .powf(1f64 / depth as f64),
                delta: search_context.search_stats.search_counters.nodes as i64
                    - nodes_in_last_iteration as i64,
                search_counters: search_context.search_stats.search_counters,
            }),
        }));

        nodes_in_last_iteration = search_context.search_stats.search_counters.nodes;
    }

    callback(EngineEvent::SearchFinished(pv_table.table[0][0]));
}

fn score(score: i32, mate_threshold: i32) -> Score {
    if score > mate_threshold {
        Score::Mate((MATE - score + 1) / 2)
    } else if score < -mate_threshold {
        Score::Mate((-MATE - score - 1) / 2)
    } else {
        Score::CP(score)
    }
}
