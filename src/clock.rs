use crate::{EngineLimits, types::Color};
use std::cmp::min;

#[derive(Default)]
pub struct Clock {
    my_time: Option<u128>,
    opp_time: Option<u128>,
    my_inc: Option<u128>,
    opp_inc: Option<u128>,
    movestogo: Option<u32>,
    movetime: Option<u128>,
}

#[derive(Debug)]
pub struct Budget {
    pub soft_limit: u128,
    pub hard_limit: u128,
}

impl Clock {
    pub fn load_clock(&mut self, turn: Color, limits: &EngineLimits) {
        match turn {
            Color::White => {
                self.my_time = limits.wtime;
                self.opp_time = limits.btime;
                self.my_inc = limits.winc;
                self.opp_inc = limits.binc;
            }
            Color::Black => {
                self.my_time = limits.btime;
                self.opp_time = limits.wtime;
                self.my_inc = limits.binc;
                self.opp_inc = limits.winc;
            }
        };

        self.movestogo = limits.movestogo;
        self.movetime = limits.movetime;
    }
    pub fn calculate_allocated_time(&self) -> Option<Budget> {
        if let Some(mt) = self.movetime {
            Some(Budget {
                soft_limit: mt * 98 / 100,
                hard_limit: mt.saturating_sub(10),
            })
        } else {
            if let Some(remaining) = self.my_time {
                let base = match self.movestogo {
                    Some(mtg) => remaining / mtg.max(1) as u128,
                    _ => remaining / 30,
                };

                let inc = self.my_inc.unwrap_or(0);
                let allocated_time = (base + inc) * 8 / 10;
                Some(Budget {
                    soft_limit: allocated_time,
                    hard_limit: min(allocated_time * 5 / 4, remaining - 50),
                })
            } else {
                None
            }
        }
    }
}
