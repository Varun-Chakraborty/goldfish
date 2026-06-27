use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

use crate::{
    history::HistoryHeuristic,
    search::MAX_PLY,
    transposition::{TTStats, TranspositionTable},
    types::Move,
};

#[derive(Debug)]
pub struct PVTable {
    pub table: [[Option<Move>; MAX_PLY as usize]; MAX_PLY as usize],
    pub length: [u32; MAX_PLY as usize],
}

impl PVTable {
    pub fn new() -> Self {
        Self {
            table: [[None; MAX_PLY as usize]; MAX_PLY as usize],
            length: [0; MAX_PLY as usize],
        }
    }
}

pub struct SearchContext<'a> {
    pub stopped: bool,
    pub search_stats: SearchStats,
    pub pv_table: &'a mut PVTable,
    pub tt: &'a mut TranspositionTable,
    pub stop: &'a Arc<AtomicBool>,
    pub ponder: bool,
    pub ponderhit: &'a Arc<AtomicBool>,
    pub history_heuristics: &'a mut HistoryHeuristic,
    pub start_time: Instant,
    pub node_limit: Option<u64>,
}

impl<'a> SearchContext<'a> {
    #[inline]
    pub fn should_stop(&self) -> bool {
        if (self.search_stats.search_counters.nodes + self.search_stats.search_counters.qnodes)
            .is_multiple_of(2048)
        {
            return self.stop.load(Ordering::Relaxed);
        }

        false
    }
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub score: i32,
    pub best_move: Option<Move>,
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
    pub time: u128,
    pub nps: u64,
    pub hashfull: u64,
    pub search_stats: Option<SearchStats>,
}

#[derive(Default, Clone, Copy)]
pub struct SearchCounters {
    pub qnodes: u64,
    pub nodes: u64,
    pub leaf_nodes: u64,
    pub examined_moves: u64,
    pub cutoffs: u64,
    pub available_moves: u64,
}

#[derive(Default)]
pub struct SearchStats {
    pub search_counters: SearchCounters,
    pub branching_factor: f64,
    pub delta: i64,
    pub tt_stats: TTStats,
}

impl SearchStats {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Default)]
pub struct EngineLimits {
    pub ponder: bool,
    pub depth: Option<u32>,
    pub btime: Option<u128>,
    pub wtime: Option<u128>,
    pub binc: Option<u128>,
    pub winc: Option<u128>,
    pub movestogo: Option<u32>,
    pub movetime: Option<u128>,
    pub nodes: Option<u64>,
}
