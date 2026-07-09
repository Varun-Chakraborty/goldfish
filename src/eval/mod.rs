mod king_safety;
mod mobility;
mod pawn_structure;
mod pst;

use crate::{
    eval::{
        king_safety::king_safety, mobility::piece_mobility, pawn_structure::pawn_structure,
        pst::pst,
    },
    game::GameState,
    types::{Color, Coordinate, PieceType, Square},
};

#[derive(Copy, Clone, Debug)]
pub struct Evaluation {
    material: i32,
    pst: i32,
    bishop_pairs: i32,
    king_safety: i32,
    mobility: i32,
    passed_pawns: i32,
    isolated_pawns: i32,
    doubled_pawns: i32,
}

impl std::fmt::Display for Evaluation {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Evaluation:\n\tmaterial: {},\n\tpst: {},\n\tbishop pairs: {},\n\tking safety: {}\n\tmobility: {}\n\tpassed pawns: {}\n\tisolated pawns: {}\n\tdoubled pawns: {}",
            self.material,
            self.pst,
            self.bishop_pairs,
            self.king_safety,
            self.mobility,
            self.passed_pawns,
            self.isolated_pawns,
            self.doubled_pawns
        )
    }
}

impl Evaluation {
    pub fn score(&self) -> i32 {
        self.material
            + self.pst
            + self.bishop_pairs
            + self.king_safety
            + self.mobility
            + self.passed_pawns
            + self.isolated_pawns
            + self.doubled_pawns
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
            mobility: 0,
            passed_pawns: 0,
            isolated_pawns: 0,
            doubled_pawns: 0,
        };
        let mut material_on_board = 0;

        let mut wpawns: [u8; 8] = [0; 8];
        let mut bpawns: [u8; 8] = [0; 8];

        for idx in 0..64 {
            let coord = Coordinate::from_idx(idx);
            let sq = board.get(coord);
            match sq {
                Square::Occupied { color, piece } if color == Color::White => {
                    let material = piece.value();
                    eval.material += material;
                    let pst = pst(&piece, color, idx);
                    eval.pst += pst;
                    if piece == PieceType::Bishop {
                        white_bishops += 1;
                    }
                    
                    material_on_board += material;
                    eval.mobility += piece_mobility(self, coord, &sq);
                    wpawns[idx % 8] |= 1 << (idx / 8);
                }
                Square::Occupied { piece, color } => {
                    let material = piece.value();
                    eval.material -= material;
                    let pst = pst(&piece, color, idx);
                    eval.pst -= pst;
                    if piece == PieceType::Bishop {
                        black_bishops += 1;
                    }
                    
                    material_on_board += material;
                    eval.mobility -= piece_mobility(self, coord, &sq);
                    bpawns[idx % 8] |= 1 << (idx / 8);
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

        eval.king_safety = king_safety(self, material_on_board);

        let pawn_structure = pawn_structure(&wpawns, &bpawns);
        eval.passed_pawns = pawn_structure.passed;
        eval.isolated_pawns = pawn_structure.isolated;
        eval.doubled_pawns = pawn_structure.doubled;

        eval
    }
}

#[cfg(test)]
mod tests {
    use crate::game::GameState;
    #[test]
    fn test_eval1() {
        let gs = GameState::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
            .unwrap();
        let eval = gs.eval();
        let score = eval.score();
        assert_eq!(score, 0);
    }

    #[test]
    fn test_eval2() {
        let gs = GameState::from_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1")
            .unwrap();
        let eval = gs.eval();
        let score = eval.score();
        println!("{eval}");
        assert!(score > 0);
    }

    #[test]
    fn test_eval3() {
        let gs = GameState::from_fen("rnbqkbnr/pppppppp/8/8/8/2N5/PPPPPPPP/R1BQKBNR b KQkq - 0 1")
            .unwrap();
        let eval = gs.eval();
        let score = eval.score();
        println!("{eval}");
        assert!(score > 0);
    }

    #[test]
    fn test_eval4() {
        let gs =
            GameState::from_fen("rn1qkb1r/pppppppp/5n2/8/4P3/8/PPPP1PPP/RNB1KBNR b KQkq - 0 1")
                .unwrap();
        let eval = gs.eval();
        let score = eval.score();
        assert!(score < 0);
    }
}
