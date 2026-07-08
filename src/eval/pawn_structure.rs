use crate::{
    GameState,
    types::{Color, Coordinate, PieceType::Pawn, Square},
};

static PASSED_PAWN_SCORE: &[i32; 6] = &[0, 10, 20, 35, 60, 100];

pub fn passed_pawns(gs: &GameState) -> i32 {
    let mut score = 0;

    for idx in 0..64 {
        let mut curr = Coordinate::from_idx(idx);
        match gs.board.get(curr) {
            Square::Occupied { color: c, piece } if piece == Pawn => {
                let (dr, mut passed) = match c {
                    Color::White => (1, PASSED_PAWN_SCORE[(curr.rank() - 1) as usize]),
                    Color::Black => (-1, PASSED_PAWN_SCORE[(6 - curr.rank()) as usize]),
                };
                while let Some(coord) = curr.checked_offset(0, dr) {
                    curr = coord;
                    for i in [-1, 0, 1] {
                        if let Some(to) = coord.checked_offset(i, 0) {
                            if gs.board.get(to)
                                == (Square::Occupied {
                                    piece: Pawn,
                                    color: c.opponent(),
                                })
                            {
                                passed = 0;
                                break;
                            }
                        }
                    }
                    if passed == 0 {
                        break;
                    }
                }
                score += passed * if c == Color::White { -1 } else { 1 };
            }
            _ => {}
        }
    }

    score
}

pub fn isolated_pawns(gs: &GameState) -> i32 {
    let mut score = 0;

    for idx in 0..64 {
        let curr = Coordinate::from_idx(idx);
        match gs.board.get(curr) {
            Square::Occupied { color: c, piece } if piece == Pawn => {
                let mut isolated = -15;
                for dr in 1..7 {
                    let coord = Coordinate::new(curr.file(), dr);
                    for i in [-1, 1] {
                        if let Some(to) = coord.checked_offset(i, 0) {
                            if gs.board.get(to)
                                == (Square::Occupied {
                                    piece: Pawn,
                                    color: c,
                                })
                            {
                                isolated = 0;
                                break;
                            }
                        }
                    }
                    if isolated == 0 {
                        break;
                    }
                }
                score -= isolated * if c == Color::White { -1 } else { 1 };
            }
            _ => {}
        }
    }

    score
}

pub fn doubled_pawns(gs: &GameState) -> i32 {
    let mut score = 0;

    for i in 0..8 {
        let mut w_candidate_pawn = None;
        let mut b_candidate_pawn = None;
        for j in 1..7 {
            let coord = Coordinate::new(i, j);
            let sq = gs.board.get(coord);
            match sq {
                Square::Occupied { color: c, piece } if piece == Pawn => {
                    if c == Color::White {
                        if let Some(_) = w_candidate_pawn {
                            score -= 12;
                        } else {
                            w_candidate_pawn = Some(coord);
                        }
                    } else {
                        if let Some(_) = b_candidate_pawn {
                            score += 12;
                        } else {
                            b_candidate_pawn = Some(coord);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    score
}
