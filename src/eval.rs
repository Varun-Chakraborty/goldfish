use crate::{
    game::GameState,
    types::{Color, Coordinate, PieceType, Square},
};

// [
//      0,  0,  0,  0,  0,  0,  0,  0,
//      5, 10, 10,-10,-10, 10, 10,  5,
//      5,  5, 10, 15, 15, 10,  5,  5,
//      0,  0,  0, 20, 20,  0,  0,  0,
//      5,  5, 10, 25, 25, 10,  5,  5,
//     10, 10, 20, 30, 30, 20, 10, 10,
//     50, 50, 50, 50, 50, 50, 50, 50,
//      0,  0,  0,  0,  0,  0,  0,  0,
// ]
static PAWN_PST: [i32; 64] = [
    0, 0, 0, 0, 0, 0, 0, 0, 5, 10, 10, -10, -10, 10, 10, 5, 5, 5, 10, 15, 15, 10, 5, 5, 0, 0, 0,
    20, 20, 0, 0, 0, 5, 5, 10, 25, 25, 10, 5, 5, 10, 10, 20, 30, 30, 20, 10, 10, 50, 50, 50, 50,
    50, 50, 50, 50, 0, 0, 0, 0, 0, 0, 0, 0,
];

// [
//     -50,-40,-30,-30,-30,-30,-40,-50,
//     -40,-20,  0,  5,  5,  0,-20,-40,
//     -30,  5, 10, 15, 15, 10,  5,-30,
//     -30,  0, 15, 20, 20, 15,  0,-30,
//     -30,  5, 15, 20, 20, 15,  5,-30,
//     -30,  0, 10, 15, 15, 10,  0,-30,
//     -40,-20,  0,  0,  0,  0,-20,-40,
//     -50,-40,-30,-30,-30,-30,-40,-50,
// ]
static KNIGHT_PST: [i32; 64] = [
    -50, -40, -30, -30, -30, -30, -40, -50, -40, -20, 0, 5, 5, 0, -20, -40, -30, 5, 10, 15, 15, 10,
    5, -30, -30, 0, 15, 20, 20, 15, 0, -30, -30, 5, 15, 20, 20, 15, 5, -30, -30, 0, 10, 15, 15, 10,
    0, -30, -40, -20, 0, 0, 0, 0, -20, -40, -50, -40, -30, -30, -30, -30, -40, -50,
];

// [
//     -20,-10,-10,-10,-10,-10,-10,-20,
//     -10,  5,  0,  0,  0,  0,  5,-10,
//     -10, 10, 10, 10, 10, 10, 10,-10,
//     -10,  0, 10, 10, 10, 10,  0,-10,
//     -10,  5,  5, 10, 10,  5,  5,-10,
//     -10,  0,  5, 10, 10,  5,  0,-10,
//     -10,  0,  0,  0,  0,  0,  0,-10,
//     -20,-10,-10,-10,-10,-10,-10,-20,
// ]
static BISHOP_PST: [i32; 64] = [
    -20, -10, -10, -10, -10, -10, -10, -20, -10, 5, 0, 0, 0, 0, 5, -10, -10, 10, 10, 10, 10, 10,
    10, -10, -10, 0, 10, 10, 10, 10, 0, -10, -10, 5, 5, 10, 10, 5, 5, -10, -10, 0, 5, 10, 10, 5, 0,
    -10, -10, 0, 0, 0, 0, 0, 0, -10, -20, -10, -10, -10, -10, -10, -10, -20,
];

// [
//      0,  0,  5, 10, 10,  5,  0,  0,
//      5, 10, 10, 10, 10, 10, 10,  5,
//     -5,  0,  0,  0,  0,  0,  0, -5,
//     -5,  0,  0,  0,  0,  0,  0, -5,
//     -5,  0,  0,  0,  0,  0,  0, -5,
//     -5,  0,  0,  0,  0,  0,  0, -5,
//     -5,  0,  0,  0,  0,  0,  0, -5,
//      0,  0,  5, 10, 10,  5,  0,  0,
// ]
static ROOK_PST: [i32; 64] = [
    0, 0, 5, 10, 10, 5, 0, 0, 5, 10, 10, 10, 10, 10, 10, 5, -5, 0, 0, 0, 0, 0, 0, -5, -5, 0, 0, 0,
    0, 0, 0, -5, -5, 0, 0, 0, 0, 0, 0, -5, -5, 0, 0, 0, 0, 0, 0, -5, -5, 0, 0, 0, 0, 0, 0, -5, 0,
    0, 5, 10, 10, 5, 0, 0,
];

// [
//     -20,-10,-10, -5, -5,-10,-10,-20,
//     -10,  0,  0,  0,  0,  0,  0,-10,
//     -10,  0,  5,  5,  5,  5,  0,-10,
//      -5,  0,  5,  5,  5,  5,  0, -5,
//       0,  0,  5,  5,  5,  5,  0, -5,
//     -10,  5,  5,  5,  5,  5,  0,-10,
//     -10,  0,  5,  0,  0,  0,  0,-10,
//     -20,-10,-10, -5, -5,-10,-10,-20,
// ]
static QUEEN_PST: [i32; 64] = [
    -20, -10, -10, -5, -5, -10, -10, -20, -10, 0, 0, 0, 0, 0, 0, -10, -10, 0, 5, 5, 5, 5, 0, -10,
    -5, 0, 5, 5, 5, 5, 0, -5, 0, 0, 5, 5, 5, 5, 0, -5, -10, 5, 5, 5, 5, 5, 0, -10, -10, 0, 5, 0, 0,
    0, 0, -10, -20, -10, -10, -5, -5, -10, -10, -20,
];

// [
//     -30,-40,-40,-50,-50,-40,-40,-30,
//     -30,-40,-40,-50,-50,-40,-40,-30,
//     -30,-40,-40,-50,-50,-40,-40,-30,
//     -30,-40,-40,-50,-50,-40,-40,-30,
//     -20,-30,-30,-40,-40,-30,-30,-20,
//     -10,-20,-20,-20,-20,-20,-20,-10,
//      20, 20,  0,  0,  0,  0, 20, 20,
//      20, 30, 10,  0,  0, 10, 30, 20,
// ]
static KING_MG_PST: [i32; 64] = [
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
        PieceType::King => KING_MG_PST[idx],
        PieceType::Pawn => PAWN_PST[idx],
    }
}

fn phase_factor(material: i32) -> f32 {
    let p = material as f32 / 8000.0;
    p.clamp(0.0, 1.0)
}

fn king_pawn_shield_penalty(gs: &GameState, color: Color) -> i32 {
    let board = &gs.board;
    let mut penalty = 0;

    let king_sq = match color {
        Color::White => gs.kings.0,
        Color::Black => gs.kings.1,
    }
    .idx();

    let dir: i32 = if color == Color::White { 8 } else { -8 };

    for file_offset in [-1, 0, 1] {
        let idx = king_sq as i32 + dir + file_offset;
        if idx >= 0 && idx < 64 {
            if let Square::Occupied {
                piece: PieceType::Pawn,
                color: c,
            } = board.get(Coordinate::from_idx(idx as usize))
            {
                if c == color {
                    penalty += 10;
                }
            } else {
                penalty -= 10;
            }
        }
    }

    penalty
}

#[derive(Copy, Clone, Debug)]
pub struct Evaluation {
    material: i32,
    pst: i32,
    bishop_pairs: i32,
    king_safety: i32,
}

impl std::fmt::Display for Evaluation {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Evaluation:\n\tmaterial: {},\n\tpst: {},\n\tbishop pairs: {},\n\tking safety: {}",
            self.material, self.pst, self.bishop_pairs, self.king_safety
        )
    }
}

impl Evaluation {
    pub fn score(&self) -> i32 {
        self.material + self.pst + self.bishop_pairs + self.king_safety
    }
}

impl GameState {
    pub fn eval(&self) -> Evaluation {
        let board = &self.board;
        let mut white_bishops = 0;
        let mut black_bishops = 0;
        let mut eval: Evaluation = Evaluation {
            material: 0,
            pst: 0,
            bishop_pairs: 0,
            king_safety: 0,
        };
        let mut material_on_board = 0;

        for idx in 0..64 {
            let sq = board.get(Coordinate::from_idx(idx));
            match sq {
                Square::Occupied { color, piece } if color == Color::White => {
                    let material = piece.value();
                    eval.material += material;
                    material_on_board += material;
                    let pst = pst(&piece, color, idx);
                    eval.pst += pst;
                    if piece == PieceType::Bishop {
                        white_bishops += 1;
                    }
                }
                Square::Occupied { piece, color } => {
                    let material = piece.value();
                    eval.material -= material;
                    material_on_board += material;
                    let pst = pst(&piece, color, idx);
                    eval.pst -= pst;

                    if piece == PieceType::Bishop {
                        black_bishops += 1;
                    }
                }
                _ => (),
            }
        }

        if white_bishops >= 2 {
            eval.bishop_pairs += 30;
        }
        if black_bishops >= 2 {
            eval.bishop_pairs -= 30;
        }

        let phase = phase_factor(material_on_board);

        let white_king_safety = king_pawn_shield_penalty(self, Color::White);
        let black_king_safety = king_pawn_shield_penalty(self, Color::Black);
        eval.king_safety = ((white_king_safety - black_king_safety) as f32 * phase) as i32;

        eval
    }
}

#[cfg(test)]
mod tests {
    use crate::game::GameState;
    #[test]
    fn test_eval() {
        let gs = GameState::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
            .unwrap();
        let eval = gs.eval();
        let score = eval.score();
        println!("{score} {eval}");
        assert_eq!(score, 0);
    }

    #[test]
    fn test_eval2() {
        let gs =
            GameState::from_fen("rn1qkb1r/pppppppp/5n2/8/4P3/8/PPPP1PPP/RNB1KBNR b KQkq - 0 1")
                .unwrap();
        let eval = gs.eval();
        let score = eval.score();
        println!("{score} {eval}");
        assert!(score < 0);
    }
}
