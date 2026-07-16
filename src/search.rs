use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

use crate::{GameState, eval::eval, game::MoveGenMode, types::Move};
const MATE: i32 = 32000;
const MATE_THRESHOLD: i32 = 31744;

struct SearchContext<'a> {
    stopped: bool,
    search_stats: SearchStats,
    stop: &'a Arc<AtomicBool>,
}

impl<'a> SearchContext<'a> {
    #[inline]
    pub fn should_stop(&self) -> bool {
        if (self.search_stats.search_counters.nodes).is_multiple_of(2048) {
            return self.stop.load(Ordering::Relaxed);
        }

        false
    }
}

#[derive(Debug, Clone)]
struct SearchResult {
    score: i32,
    best_move: Option<Move>,
}

pub enum Score {
    CP(i32),
    Mate(i32),
}

pub struct IterationInfo {
    pub depth: u32,
    pub seldepth: u32,
    pub score: Score,
    pub raw_score: i32,
    pub best_line: Option<Vec<Move>>,
    pub nodes: u64,
    pub nps: u64,
    pub search_stats: Option<SearchStats>,
}

#[derive(Default, Clone, Copy)]
pub struct SearchCounters {
    pub nodes: u64,
    pub examined_moves: u64,
    pub cutoffs: u64,
    pub available_moves: u64,
}

#[derive(Default)]
pub struct SearchStats {
    pub search_counters: SearchCounters,
    pub branching_factor: f64,
}

impl SearchStats {
    pub fn new() -> Self {
        Self::default()
    }
}

fn negamax(
    gs: &mut GameState,
    depth: u32,
    ply: u32,
    mut alpha: i32,
    beta: i32,
    search_context: &mut SearchContext,
) -> SearchResult {
    search_context.search_stats.search_counters.nodes += 1;
    if depth == 0 {
        return SearchResult {
            score: eval(gs).0,
            best_move: None,
        };
    }

    let mut search_result = SearchResult {
        score: -MATE,
        best_move: None,
    };

    let mut legal = gs.legal_moves(MoveGenMode::All);
    let total_moves = legal.len();

    legal.sort_by_key(|m| m.captured.is_none());

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
        let mut result = negamax(gs, depth - 1, ply + 1, -beta, -alpha, search_context);
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
        }
        alpha = alpha.max(search_result.score);
        search_context.search_stats.search_counters.examined_moves += 1;
        
        if alpha >= beta {
            search_context.search_stats.search_counters.cutoffs += 1;
            break;
        }
    }

    search_result
}

pub enum EngineEvent {
    IterationInfo(IterationInfo),
    SearchFinished(Move),
    SearchStopped(Move),
}

pub struct EngineLimits {
    pub depth: Option<u32>,
}

pub fn search<F>(gs: &mut GameState, limits: EngineLimits, stop: &Arc<AtomicBool>, mut callback: F)
where
    F: FnMut(EngineEvent),
{
    let depth = limits.depth.unwrap_or(256);

    let mut search_context = SearchContext {
        stopped: false,
        stop,
        search_stats: SearchStats::new(),
    };

    let start_time = Instant::now();

    let result = negamax(gs, depth, 0, -MATE, MATE, &mut search_context);

    let duration = start_time.elapsed();

    callback(EngineEvent::IterationInfo(IterationInfo {
        depth,
        seldepth: depth,
        score: score(result.score, MATE_THRESHOLD),
        raw_score: result.score,
        nodes: search_context.search_stats.search_counters.nodes,
        nps: (search_context.search_stats.search_counters.nodes as f64 / duration.as_secs_f64())
            .ceil() as u64,
        best_line: result.best_move.map(|m| vec![m]),
        search_stats: Some(SearchStats {
            branching_factor: (search_context.search_stats.search_counters.nodes as f64)
                .powf(1f64 / depth as f64),
            search_counters: search_context.search_stats.search_counters,
        }),
    }));
    callback(EngineEvent::SearchFinished(result.best_move.unwrap()));
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
