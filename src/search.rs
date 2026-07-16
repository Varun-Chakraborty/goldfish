use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

use crate::{
    eval::eval,
    game::{GameState, MoveGenMode, Undo},
    position::{DrawReason::*, Position, PositionStatus::*},
    transposition::{Bound, TTEntry, TranspositionTable},
    types::{Color, Move},
};
const MATE: i32 = 32000;
const MATE_THRESHOLD: i32 = 31744;
const DRAW: i32 = 0;

struct SearchContext<'a> {
    stopped: bool,
    search_stats: SearchStats,
    tt: &'a mut TranspositionTable,
    stop: &'a Arc<AtomicBool>,
}

impl<'a> SearchContext<'a> {
    #[inline]
    pub fn should_stop(&self) -> bool {
        if (self.search_stats.search_counters.nodes + self.search_stats.search_counters.qnodes).is_multiple_of(2048) {
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
    pub growth_factor: f64,
    pub tt_hit_rate: f64,
    pub tt_exact_hit_rate: f64,
}

impl SearchStats {
    pub fn new() -> Self {
        Self::default()
    }
}

fn quiescence(
    gs: &mut GameState,
    ply: u32,
    depth: u32,
    mut alpha: i32,
    beta: i32,
    search_context: &mut SearchContext,
) -> SearchResult {
    let position = Position::quick_draw_analysis(gs);
    search_context.search_stats.search_counters.qnodes += 1;

    if position != Ongoing {
        return SearchResult {
            score: match position {
                Draw(FiftyMoveRule) | Draw(ThreefoldRepetition) => DRAW,
                Draw(InsufficientMaterial) => DRAW,
                _ => unreachable!(),
            },
            best_move: None,
        };
    }

    let in_check = gs.in_check(gs.turn);

    if !in_check {
        let stand_pat = match gs.turn {
            Color::White => eval(gs).0,
            Color::Black => -eval(gs).0,
        };

        if stand_pat >= beta {
            return SearchResult {
                score: beta,
                best_move: None,
            };
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
            return SearchResult {
                score: -MATE + ply as i32,
                best_move: None,
            };
        } else {
            return SearchResult {
                score: DRAW,
                best_move: None,
            };
        }
    }
    /*  else if depth < 2 {
        gs.legal_moves(MoveGenMode::All)
    } */
    else {
        gs.legal_moves(MoveGenMode::CaptureOnly)
    };

    legal.sort_by_key(|m| m.captured.is_none());
    let total_moves = legal.len();
    search_context.search_stats.search_counters.available_moves += total_moves as u64;

    let mut search_result = SearchResult {
        score: alpha,
        best_move: None,
    };

    for m in legal {
        if search_context.should_stop() {
            search_context.stopped = true;
            return search_result;
        }
        let undo = gs.make_move(m);
        let result = quiescence(gs, ply + 1, depth + 1, -beta, -alpha, search_context);
        gs.unmake_move(undo);
        if search_context.stopped {
            return search_result;
        }
        let score = -result.score;

        search_context.search_stats.search_counters.examined_moves += 1;
        if score >= beta {
            search_context.search_stats.search_counters.cutoffs += 1;
            search_result.score = beta;
            return search_result;
        }
        if score > alpha {
            alpha = score;
        }
    }

    search_result.score = alpha;
    search_result
}

fn negamax(
    gs: &mut GameState,
    depth: u32,
    ply: u32,
    mut alpha: i32,
    mut beta: i32,
    best_line: Option<&[Move]>,
    pvpath: bool,
    search_context: &mut SearchContext,
) -> SearchResult {
    let org_alpha = alpha;
    let org_beta = beta;
    search_context.search_stats.search_counters.nodes += 1;

    if let Some(entry) = search_context.tt.probe(gs.zobrist, ply, Some(depth)) {
        match entry.bound {
            Bound::Exact => {
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
            return SearchResult {
                score: entry.score,
                best_move: entry.best_move,
            };
        }
    }

    if depth == 0 {
        search_context.search_stats.search_counters.leaf_nodes += 1;
        return quiescence(gs, ply, 0, alpha, beta, search_context);
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
            return search_result;
        }
        Draw(FiftyMoveRule) | Draw(ThreefoldRepetition) => {
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

    legal.sort_by_key(|m| m.captured.is_none());
    let total_moves = legal.len();

    search_context.search_stats.search_counters.available_moves += total_moves as u64;

    if let Some(entry) = search_context.tt.probe(gs.zobrist, ply, None)
        && let Some(bm) = entry.best_move
        && let Some(index) = legal.iter().position(|&m| m == bm)
    {
        let m = legal.remove(index);
        legal.insert(0, m);
    }

    if pvpath
        && let Some(best_line) = best_line
        && let Some(bm) = best_line.get(ply as usize)
        && let Some(index) = legal.iter().position(|m| *m == *bm)
    {
        let m = legal.remove(index);
        legal.insert(0, m);
    }

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
            best_line,
            pvpath
                && best_line
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
        }
        alpha = alpha.max(search_result.score);
        search_context.search_stats.search_counters.examined_moves += 1;
        if alpha >= beta {
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
            bound: if search_result.score >= org_beta {
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

pub enum EngineEvent {
    IterationInfo(IterationInfo),
    SearchFinished(Move),
    SearchStopped(Move),
}

pub struct EngineLimits {
    pub depth: Option<u32>,
}

pub fn iterative_deepening<F>(
    gs: &mut GameState,
    limits: EngineLimits,
    stop: &Arc<AtomicBool>,
    tt: &mut Option<TranspositionTable>,
    mut callback: F,
) where
    F: FnMut(EngineEvent),
{
    let depth = limits.depth.unwrap_or(256);
    let mut best_line: Option<Vec<Move>> = None;
    
    let tt = tt.get_or_insert_with(|| TranspositionTable::new_table(64, MATE_THRESHOLD));
    
    let mut nodes_in_last_iteration = 1;

    for depth in 1..=depth {
        let mut search_context = SearchContext {
            stopped: false,
            stop,
            tt,
            search_stats: SearchStats::new(),
        };
        let time = Instant::now();
        let result = negamax(
            gs,
            depth,
            0,
            -MATE,
            MATE,
            best_line.as_deref(),
            true,
            &mut search_context,
        );

        if search_context.stopped || stop.load(Ordering::Relaxed) {
            if let Some(best_line) = best_line {
                callback(EngineEvent::SearchStopped(best_line[0]));
            }
            return;
        }

        if let Some(best_line) = best_line.as_mut() {
            best_line.clear();
        }

        if let Some(m) = result.best_move {
            best_line.get_or_insert_default().push(m);

            let mut undo_stack: Vec<Undo> = Vec::with_capacity(depth as usize);
            let undo = gs.make_move(m);
            undo_stack.push(undo);

            let mut ply = 1;
            while ply < depth
                && let Some(entry) = search_context.tt.probe(gs.zobrist, 0, None)
                && let Some(bm) = entry.best_move
            {
                let undo = gs.make_move(bm);
                undo_stack.push(undo);
                best_line.get_or_insert_default().push(bm);

                ply += 1;
            }

            while let Some(undo) = undo_stack.pop() {
                gs.unmake_move(undo);
            }
        }

        let duration = time.elapsed();
        
        callback(EngineEvent::IterationInfo(IterationInfo {
            depth,
            seldepth: depth,
            score: score(result.score, MATE_THRESHOLD),
            raw_score: result.score,
            nodes: search_context.search_stats.search_counters.nodes,
            nps: ((search_context.search_stats.search_counters.nodes + search_context.search_stats.search_counters.qnodes) as f64
                / duration.as_secs_f64())
            .ceil() as u64,
            best_line: best_line.clone(),
            search_stats: Some(SearchStats {
                tt_hit_rate: search_context.tt.hit_rate(),
                tt_exact_hit_rate: search_context.tt.exact_hit_rate(),
                branching_factor: (search_context.search_stats.search_counters.nodes as f64)
                    .powf(1f64 / depth as f64),
                growth_factor: search_context.search_stats.search_counters.nodes as f64
                / nodes_in_last_iteration as f64,
                search_counters: search_context.search_stats.search_counters,
            }),
        }));
        
        nodes_in_last_iteration = search_context.search_stats.search_counters.nodes;
    }

    if let Some(best_line) = best_line {
        callback(EngineEvent::SearchFinished(best_line[0]));
    }
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
