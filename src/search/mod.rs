mod negamax;
mod ordermoves;
mod quiescence;
mod root_search;
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
    game::GameState,
    history::HistoryHeuristic,
    search::{
        root_search::root_search,
        search_types::{
            EngineLimits, IterationInfo, PVTable, Score, SearchContext, SearchCounters, SearchStats,
        },
    },
    transposition::TranspositionTable,
    types::Move,
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
    multipv: u8,
    mut callback: F,
) -> (Option<Move>, Option<Move>)
where
    F: FnMut(EngineEvent),
{
    let tt = tt.get_or_insert_with(|| TranspositionTable::new(64));
    let history_heuristics = history.get_or_insert_with(HistoryHeuristic::new);
    let mut pv_table = PVTable::new();
    let mut lines = vec![];
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

        root_search(
            gs,
            depth,
            &mut ctx,
            &mut lines,
            multipv as usize,
            Some(&mut callback),
        );

        let elapsed = ctx.start_time.elapsed().as_millis();
        if ctx.stopped
            || stop.load(Ordering::Relaxed)
            || (!ctx.ponder && ctx.budget.as_ref().is_some_and(|b| elapsed >= b.soft_limit))
        {
            break;
        }

        for i in 0..multipv {
            if i as usize >= lines.len() {
                break;
            }

            let line = lines[i as usize];

            let value = line.score;
            let line: Vec<_> = lines[i as usize]
                .pv
                .into_iter()
                .take_while(|x| x.is_some())
                .map(Option::unwrap)
                .collect();
            let line = (!line.is_empty()).then_some(line);

            let duration = start.elapsed();
            let tt_stats = ctx.tt.stats();

            callback(EngineEvent::IterationInfo(IterationInfo {
                depth,
                seldepth: ctx.seldepth,
                multipv: i + 1,
                score: score(value),
                raw_score: value,
                time: duration,
                best_line: line,
                hashfull: tt_stats.hashfull,
                tt_stats: mem::take(tt_stats),
                qsearch_stats: ctx.qsearch_stats,
                search_stats: SearchStats {
                    branching_factor: (ctx.search_stats.search_counters.nodes as f64)
                        .powf(1f64 / depth as f64),
                    delta: ctx.search_stats.search_counters.nodes as i64
                        - nodes_in_last_iteration as i64,
                    search_counters: ctx.search_stats.search_counters,
                },
            }));
        }

        depth += 1;
        if limits.depth.is_some_and(|max_depth| depth > max_depth) {
            break;
        }
        nodes_in_last_iteration = ctx.search_stats.search_counters.nodes;
    }

    let chosen_line: Vec<_> = lines[0]
        .pv
        .into_iter()
        .take_while(|x| x.is_some())
        .map(Option::unwrap)
        .collect();

    let best_move = chosen_line.first().copied();
    let ponder = chosen_line.get(1).copied();

    (best_move, ponder)
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
