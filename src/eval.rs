use crate::{
    game::GameState,
    types::{Color, Coordinate, PieceType, Square},
};

pub fn piece_value(piece: &PieceType) -> i32 {
    match piece {
        PieceType::Pawn => 100,
        PieceType::Knight => 320,
        PieceType::Bishop => 330,
        PieceType::Rook => 500,
        PieceType::Queen => 900,
        PieceType::King => 0,
    }
}

static PAWN_PST: [i32; 64] = [
    0, 0, 0, 0, 0, 0, 0, 0, 5, 10, 10, -20, -20, 10, 10, 5, 5, -5, -10, 0, 0, -10, -5, 5, 0, 0, 0,
    20, 20, 0, 0, 0, 5, 5, 10, 30, 30, 10, 5, 5, 10, 10, 10, 25, 25, 10, 10, 10, 10, 10, 10, 10,
    10, 10, 10, 10, 0, 0, 0, 0, 0, 0, 0, 0,
];

static KNIGHT_PST: [i32; 64] = [
    -50, -15, -30, -30, -30, -30, -15, -50, -40, -20, 0, 0, 0, 0, -20, -40, -30, 0, 10, 15, 15, 10,
    0, -30, -30, 5, 15, 20, 20, 15, 5, -30, -30, 0, 15, 20, 20, 15, 0, -30, -30, 5, 10, 15, 15, 10,
    5, -30, -40, -20, 0, 5, 5, 0, -20, -40, -50, -15, -30, -30, -30, -30, -15, -50,
];

static BISHOP_PST: [i32; 64] = [
    -20, -10, -10, -10, -10, -10, -10, -20, -10, 5, 0, 0, 0, 0, 5, -10, -10, 10, 10, 10, 10, 10,
    10, -10, -10, 0, 10, 10, 10, 10, 0, -10, -10, 5, 5, 10, 10, 5, 5, -10, -10, 0, 5, 10, 10, 5, 0,
    -10, -10, 0, 0, 0, 0, 0, 0, -10, -20, -10, -10, -10, -10, -10, -10, -20,
];

static ROOK_PST: [i32; 64] = [
    0, 0, 5, 10, 10, 5, 0, 0, 15, 15, 15, 15, 15, 15, 15, 15, -5, 0, 0, 0, 0, 0, 0, -5, -5, 0, 0,
    0, 0, 0, 0, -5, -5, 0, 0, 0, 0, 0, 0, -5, -5, 0, 0, 0, 0, 0, 0, -5, 5, 10, 10, 10, 10, 10, 10,
    5, 0, 0, 5, 10, 10, 5, 0, 0,
];

static QUEEN_PST: [i32; 64] = [
    -20, -10, -10, -5, -5, -10, -10, -20, -10, 0, 0, 0, 0, 0, 0, -10, -10, 0, 5, 5, 5, 5, 0, -10,
    -5, 0, 5, 5, 5, 5, 0, -5, 0, 0, 5, 5, 5, 5, 0, -5, -10, 5, 5, 5, 5, 5, 0, -10, -10, 0, 5, 0, 0,
    0, 0, -10, -20, -10, -10, -5, -5, -10, -10, -20,
];

static KING_PST: [i32; 64] = [
    -30, -40, -40, -50, -50, -40, -40, -30, -30, -40, -40, -50, -50, -40, -40, -30, -30, -40, -40,
    -50, -50, -40, -40, -30, -30, -40, -40, -50, -50, -40, -40, -30, -20, -30, -30, -40, -40, -30,
    -30, -20, -10, -20, -20, -20, -20, -20, -20, -10, 20, 20, 0, 0, 0, 0, 20, 20, 20, 30, 10, 0, 0,
    10, 30, 20,
];

fn pst(piece: &PieceType, color: Color, idx: usize) -> i32 {
    let idx = if color == Color::White { idx } else { idx ^ 56 };

    match piece {
        PieceType::Knight => KNIGHT_PST[idx],
        PieceType::Bishop => BISHOP_PST[idx],
        PieceType::Rook => ROOK_PST[idx],
        PieceType::Queen => QUEEN_PST[idx],
        PieceType::King => KING_PST[idx],
        PieceType::Pawn => PAWN_PST[idx],
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Evaluation {
    material: i32,
    pst: i32,
    castling: i32,
    bishop_pairs: i32,
}

impl std::fmt::Display for Evaluation {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Evaluation:\n\tmaterial: {},\n\tpst: {},\n\tcastling: {},\n\tbishop pairs: {}",
            self.material, self.pst, self.castling, self.bishop_pairs
        )
    }
}

pub fn eval(gs: &GameState) -> (i32, Evaluation) {
    let board = &gs.board;
    let mut white_bishops = 0;
    let mut black_bishops = 0;
    let mut score = 0;
    let mut eval: Evaluation = Evaluation {
        material: 0,
        pst: 0,
        castling: 0,
        bishop_pairs: 0,
    };

    for idx in 0..64 {
        let sq = board.get(Coordinate::from_idx(idx));
        match sq {
            Square::Occupied { color, piece } if color == Color::White => {
                let material = piece_value(&piece);
                score += material;
                eval.material += material;
                let pst = pst(&piece, color, idx);
                score += pst;
                eval.pst += pst;
                if piece == PieceType::Bishop {
                    white_bishops += 1;
                }
            }
            Square::Occupied { piece, color } => {
                let material = piece_value(&piece);
                score -= material;
                eval.material -= material;
                let pst = pst(&piece, color, idx);
                score -= pst;
                eval.pst -= pst;

                if piece == PieceType::Bishop {
                    black_bishops += 1;
                }
            }
            _ => (),
        }
    }

    if white_bishops >= 2 {
        score += 30;
        eval.bishop_pairs += 30;
    }
    if black_bishops >= 2 {
        score -= 30;
        eval.bishop_pairs -= 30;
    }

    if gs.castling_rights.kingside_white || gs.castling_rights.queenside_white {
        score += 20;
        eval.castling += 20;
    }
    if gs.castling_rights.kingside_black || gs.castling_rights.queenside_black {
        score -= 20;
        eval.castling -= 20;
    }

    (score, eval)
}

#[cfg(test)]
mod tests {
    use crate::{eval::eval, game::GameState};
    #[test]
    fn test_eval() {
        let gs = GameState::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
            .unwrap();
        let (score, eval) = eval(&gs);
        println!("{score} {eval}");
        assert_eq!(score, 0);
    }

    #[test]
    fn test_eval2() {
        let gs =
            GameState::from_fen("rn1qkb1r/pppppppp/5n2/8/4P3/8/PPPP1PPP/RNB1KBNR b KQkq - 0 1")
                .unwrap();
        let (score, eval) = eval(&gs);
        println!("{score} {eval}");
        assert!(score < 0);
    }
}
