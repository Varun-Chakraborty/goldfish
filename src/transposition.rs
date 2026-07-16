use crate::types::Move;

#[derive(Clone, Copy, PartialEq)]
pub enum Bound {
    Exact,
    Lower,
    Upper,
}

#[derive(Clone)]
pub struct TTEntry {
    pub key: u64,
    pub depth: u32,
    pub score: i32,
    pub best_move: Option<Move>,
    pub bound: Bound,
}

pub struct TranspositionTable {
    entries: Vec<Option<TTEntry>>,
    probes: u64,
    hits: u64,
    exact_hits: u64,
    mate_threshold: i32,
}

impl TranspositionTable {
    pub fn new_table(size_mb: usize, mate_threshold: i32) -> Self {
        let bytes = size_mb * 1024 * 1024;
        let entries = bytes / std::mem::size_of::<Option<TTEntry>>();

        Self {
            entries: vec![None; entries],
            probes: 0,
            hits: 0,
            exact_hits: 0,
            mate_threshold,
        }
    }

    fn index(&self, hash: u64) -> usize {
        hash as usize % self.entries.len()
    }

    pub fn score_to_tt(&self, score: i32, ply: u32) -> i32 {
        if score > self.mate_threshold {
            score + ply as i32
        } else if score < -self.mate_threshold {
            score - ply as i32
        } else {
            score
        }
    }

    pub fn score_from_tt(&self, score: i32, ply: u32) -> i32 {
        if score > self.mate_threshold {
            score - (ply as i32)
        } else if score < -self.mate_threshold {
            score + (ply as i32)
        } else {
            score
        }
    }

    pub fn store(&mut self, hash: u64, ply: u32, mut entry: TTEntry) {
        let idx = self.index(hash);
        if let Some(cell) = self.entries.get(idx)
            && let Some(stored_entry) = cell
        {
            if entry.depth > stored_entry.depth
                || (entry.bound == Bound::Exact && stored_entry.bound != Bound::Exact)
            {
                entry.score = self.score_to_tt(entry.score, ply);
                self.entries[idx] = Some(entry);
            }
        } else {
            entry.score = self.score_to_tt(entry.score, ply);
            self.entries[idx] = Some(entry);
        }
    }

    pub fn probe(&mut self, hash: u64, ply: u32, depth: Option<u32>) -> Option<TTEntry> {
        self.probes += 1;
        let idx = self.index(hash);
        match &self.entries[idx] {
            Some(entry) if entry.key == hash => {
                self.hits += 1;
                if depth.is_none_or(|d| entry.depth >= d) {
                    if entry.bound == Bound::Exact {
                        self.exact_hits += 1;
                    }
                    let mut entry = entry.clone();
                    entry.score = self.score_from_tt(entry.score, ply);
                    Some(entry)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn hit_rate(&self) -> f64 {
        self.hits as f64 / self.probes as f64
    }

    pub fn exact_hit_rate(&self) -> f64 {
        self.exact_hits as f64 / self.probes as f64
    }
}
