mod king_safety;
mod pawn_structure;

use crate::{
    game::{
        GameState,
        eval::{king_safety::king_safety, pawn_structure::pawn_structure},
    },
    types::{Color, PieceType},
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

        eval.pst = self.pst_score;
        eval.mobility = self.mobility_score;

        let pawn = PieceType::Pawn;
        let knight = PieceType::Knight;
        let bishop = PieceType::Bishop;
        let rook = PieceType::Rook;
        let queen = PieceType::Queen;

        let wm = self.material_count[self.material_idx(pawn, Color::White)] as u16 * pawn.value()
            + self.material_count[self.material_idx(knight, Color::White)] as u16 * knight.value()
            + self.material_count[self.material_idx(bishop, Color::White)] as u16 * bishop.value()
            + self.material_count[self.material_idx(rook, Color::White)] as u16 * rook.value()
            + self.material_count[self.material_idx(queen, Color::White)] as u16 * queen.value();

        let bm = self.material_count[self.material_idx(pawn, Color::Black)] as u16 * pawn.value()
            + self.material_count[self.material_idx(knight, Color::Black)] as u16 * knight.value()
            + self.material_count[self.material_idx(bishop, Color::Black)] as u16 * bishop.value()
            + self.material_count[self.material_idx(rook, Color::Black)] as u16 * rook.value()
            + self.material_count[self.material_idx(queen, Color::Black)] as u16 * queen.value();

        eval.material = wm as i32 - bm as i32;

        if self.wb.count_ones() >= 2 {
            eval.bishop_pairs += 30;
        }
        if self.bb.count_ones() >= 2 {
            eval.bishop_pairs -= 30;
        }

        let material_on_board = wm + bm;

        eval.king_safety = king_safety(self, material_on_board);

        let pawn_structure = pawn_structure(self.wp, self.bp);
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
