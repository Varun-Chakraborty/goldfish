use crate::types::Move;

#[derive(Clone, Copy, PartialEq, Default)]
pub enum Bound {
    #[default]
    Exact,
    Lower,
    Upper,
}

#[derive(Clone, Default)]
pub struct TTEntry {
    pub key: u64,
    pub depth: u32,
    pub score: i32,
    pub best_move: Option<Move>,
    pub bound: Bound,
}

#[derive(Default)]
pub struct TTStats {
    pub hits: u64,
    pub probes: u64,
}

pub struct TranspositionTable {
    entries: Vec<TTEntry>,
    mate_threshold: i32,
    tt_stats: TTStats,
}

impl TranspositionTable {
    pub fn new_table(size_mb: usize, mate_threshold: i32) -> Self {
        let size_of_entry = std::mem::size_of::<TTEntry>();
        let entries = size_mb * 1024 * 1024 / size_of_entry;

        println!(
            "Transposition table size: {entries} entries, {:.2} MB",
            size_of_entry * entries / 1024 / 1024
        );

        Self {
            entries: vec![TTEntry::default(); entries],
            mate_threshold,
            tt_stats: TTStats::default(),
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

        if let Some(stored_entry) = self.entries.get(idx) {
            if stored_entry.key == 0
                || entry.depth > stored_entry.depth
                || (entry.depth == stored_entry.depth
                    && (entry.bound == Bound::Exact && stored_entry.bound != Bound::Exact))
            {
                entry.score = self.score_to_tt(entry.score, ply);
                self.entries[idx] = entry;
            }
        } else {
            entry.score = self.score_to_tt(entry.score, ply);
            self.entries[idx] = entry;
        }
    }

    pub fn probe(&mut self, hash: u64, ply: u32, depth: Option<u32>) -> Option<TTEntry> {
        self.tt_stats.probes += 1;
        let idx = self.index(hash);
        match self.entries.get(idx) {
            Some(entry) if entry.key == hash => {
                if depth.is_none_or(|d| entry.depth >= d) {
                    self.tt_stats.hits += 1;
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

    pub fn stats(&mut self) -> &mut TTStats {
        &mut self.tt_stats
    }
}
