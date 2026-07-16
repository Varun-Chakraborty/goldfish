use crate::{
    game::{GameState, MoveGenMode::All},
    types::Move,
};

#[derive(Debug, PartialEq)]
pub enum DrawReason {
    Stalemate,
    FiftyMoveRule,
    InsufficientMaterial,
    ThreefoldRepetition,
}

impl std::fmt::Display for DrawReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DrawReason::Stalemate => write!(f, "by Stalemate"),
            DrawReason::FiftyMoveRule => write!(f, "by 50 move rule"),
            DrawReason::InsufficientMaterial => write!(f, "by Insufficient Material"),
            DrawReason::ThreefoldRepetition => write!(f, "by Threefold Repetition"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum PositionStatus {
    Ongoing,
    Draw(DrawReason),
    Checkmate,
}

#[derive(Debug)]
pub struct Position {
    pub status: PositionStatus,
    pub legal_moves: Option<Vec<Move>>,
}

impl Position {
    pub fn quick_draw_analysis(gs: &GameState) -> PositionStatus {
        match gs.halfmove_clock() >= 50 {
            true => PositionStatus::Draw(DrawReason::FiftyMoveRule),
            false => match gs
                .history
                .iter()
                .rev()
                .take(gs.halfmove_clock() as usize)
                .filter(|hash| **hash == gs.zobrist)
                .take(2)
                .count()
                == 2
            {
                true => PositionStatus::Draw(DrawReason::ThreefoldRepetition),
                false => match gs.is_insufficient_material() {
                    true => PositionStatus::Draw(DrawReason::InsufficientMaterial),
                    false => PositionStatus::Ongoing,
                },
            },
        }
    }
    pub fn analyse(gs: &mut GameState) -> Position {
        let mut position = Position {
            status: Self::quick_draw_analysis(gs),
            legal_moves: None,
        };

        if !matches!(position.status, PositionStatus::Ongoing) {
            return position;
        }

        position.legal_moves = Some(gs.legal_moves(All));
        position.status = match position.legal_moves.as_ref().expect("legal_moves set above").is_empty() {
            true => match gs.in_check(gs.turn) {
                true => PositionStatus::Checkmate,
                false => PositionStatus::Draw(DrawReason::Stalemate),
            },
            false => PositionStatus::Ongoing,
        };

        position
    }
}
