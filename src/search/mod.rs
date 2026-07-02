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
    clock::Clock,
    game::{GameState, MoveGenMode},
    history::HistoryHeuristic,
    search::{
        negamax::negamax,
        quiescence::quiescence,
        search_types::{
            EngineLimits, IterationInfo, PVTable, Score, SearchContext, SearchCounters,
            SearchStats, SearchType::FullSearch,
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
    clock: &mut Clock,
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

    let mut ctx = SearchContext {
        seldepth: 0,
        stopped: false,
        stop,
        ponderhit,
        tt,
        search_stats: SearchStats::new(),
        qsearch_stats: SearchCounters::default(),
        pv_table: &mut pv_table,
        history_heuristics,
        start_time: start,
        budget: clock.calculate_allocated_time(),
        ponder: limits.ponder,
        node_limit: limits.nodes,
    };
    loop {
        ctx.seldepth = 0;
        ctx.stopped = false;

        let result = negamax(gs, depth, 0, -MATE, MATE, &pv, true, FullSearch, &mut ctx);

        let elapsed = ctx.start_time.elapsed().as_millis();
        if ctx.stopped
            || stop.load(Ordering::Relaxed)
            || (!ctx.ponder && ctx.budget.as_ref().is_some_and(|b| elapsed >= b.soft_limit))
        {
            break;
        }

        pv = ctx.pv_table.table[0][..ctx.pv_table.length[0] as usize]
            .iter()
            .copied()
            .collect();

        let duration = start.elapsed();
        let tt_stats = ctx.tt.stats();

        callback(EngineEvent::IterationInfo(IterationInfo {
            depth,
            seldepth: ctx.seldepth,
            score: score(result.score),
            raw_score: result.score,
            nodes: ctx.search_stats.search_counters.nodes + ctx.qsearch_stats.nodes,
            nps: ((ctx.search_stats.search_counters.nodes + ctx.qsearch_stats.nodes) as f64
                / duration.as_secs_f64())
            .ceil() as u64,
            time: duration,
            best_line: pv.clone(),
            hashfull: tt_stats.hashfull,
            tt_stats: mem::take(tt_stats),
            qsearch_stats: Some(ctx.qsearch_stats),
            search_stats: Some(SearchStats {
                branching_factor: (ctx.search_stats.search_counters.nodes as f64)
                    .powf(1f64 / depth as f64),
                delta: ctx.search_stats.search_counters.nodes as i64
                    - nodes_in_last_iteration as i64,
                search_counters: ctx.search_stats.search_counters,
            }),
        }));

        depth += 1;
        if limits.depth.is_some_and(|max_depth| depth > max_depth) {
            break;
        }
        nodes_in_last_iteration = ctx.search_stats.search_counters.nodes;
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
