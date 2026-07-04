mod consts;

use crate::{
    board::Board,
    game::GameState,
    types::{
        CastleSide, Color, Coordinate,
        PieceType::{self, Pawn},
        Square,
    },
    zobrist::consts::RANDOM64,
};

pub fn piece_sq_hash(piece: PieceType, color: Color, coord: Coordinate) -> u64 {
    let piece_index = match (piece, color) {
        (PieceType::Pawn, Color::Black) => 0,
        (PieceType::Pawn, Color::White) => 1,
        (PieceType::Knight, Color::Black) => 2,
        (PieceType::Knight, Color::White) => 3,
        (PieceType::Bishop, Color::Black) => 4,
        (PieceType::Bishop, Color::White) => 5,
        (PieceType::Rook, Color::Black) => 6,
        (PieceType::Rook, Color::White) => 7,
        (PieceType::Queen, Color::Black) => 8,
        (PieceType::Queen, Color::White) => 9,
        (PieceType::King, Color::Black) => 10,
        (PieceType::King, Color::White) => 11,
    };
    let idx = 64 * piece_index + 8 * coord.rank() as u32 + coord.file() as u32;
    RANDOM64[idx as usize]
}

pub fn castling_hash(color: Color, side: CastleSide) -> u64 {
    let index = match (color, side) {
        (Color::White, CastleSide::Kingside) => 0,
        (Color::White, CastleSide::Queenside) => 1,
        (Color::Black, CastleSide::Kingside) => 2,
        (Color::Black, CastleSide::Queenside) => 3,
    };
    RANDOM64[768 + index]
}

pub fn ep_hash(ep: Coordinate, board: &Board, side_to_move: Color) -> u64 {
    let dr = if side_to_move == Color::White { -1 } else { 1 };
    let ep_possible_by_opponent = ep.checked_offset(-1, dr).is_some_and(|c| {
        board.get(c)
            == Square::Occupied {
                color: side_to_move,
                piece: Pawn,
            }
    }) || ep.checked_offset(1, dr).is_some_and(|c| {
        board.get(c)
            == Square::Occupied {
                color: side_to_move,
                piece: Pawn,
            }
    });
    if ep_possible_by_opponent {
        return RANDOM64[772 + ep.file() as usize];
    }
    0
}

pub fn side_hash() -> u64 {
    RANDOM64[780]
}

pub fn compute_hash(gs: &GameState) -> u64 {
    let mut hash = 0;
    for i in 0..64 {
        let coord = Coordinate::from_idx(i);
        let sq = gs.board.get(coord);
        match sq {
            Square::Empty => continue,
            Square::Occupied { color, piece } => {
                let sq_hash = piece_sq_hash(piece, color, coord);
                hash ^= sq_hash;
            }
        }
    }

    if gs.turn == Color::White {
        hash ^= side_hash();
    }

    if let Some(ep) = gs.en_passant {
        hash ^= ep_hash(ep, &gs.board, gs.turn);
    }

    if gs.castling_rights.kingside_white {
        hash ^= castling_hash(Color::White, CastleSide::Kingside);
    }
    if gs.castling_rights.queenside_white {
        hash ^= castling_hash(Color::White, CastleSide::Queenside);
    }
    if gs.castling_rights.kingside_black {
        hash ^= castling_hash(Color::Black, CastleSide::Kingside);
    }
    if gs.castling_rights.queenside_black {
        hash ^= castling_hash(Color::Black, CastleSide::Queenside);
    }
    hash
}

#[cfg(test)]
mod tests {
    use crate::{game::GameState, notation::parse_algebraic, zobrist::compute_hash};

    #[test]
    fn test_zobrist() {
        let mut gs =
            GameState::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
                .unwrap();
        let m = parse_algebraic(&mut gs, "e4").expect("Illegal move in this position");
        gs.make_move(m);
        let hash = compute_hash(&gs);
        assert!(gs.zobrist == hash);
    }

    #[test]
    fn test_key_after_start_position() {
        let gs = GameState::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
            .unwrap();
        let hash = compute_hash(&gs);
        println!("{hash}");
        let result = u64::from_str_radix("463b96181691fc9c", 16).unwrap();
        assert_eq!(hash, result);
    }

    #[test]
    fn test_key_after_e2e4() {
        let gs = GameState::from_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1")
            .unwrap();
        let hash = compute_hash(&gs);
        println!("{hash}");
        let result = u64::from_str_radix("823c9b50fd114196", 16).unwrap();
        assert_eq!(hash, result);
    }

    #[test]
    fn test_key_after_e2e4_d7d5() {
        let gs =
            GameState::from_fen("rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 2")
                .unwrap();
        let hash = compute_hash(&gs);
        println!("{hash}");
        let result = u64::from_str_radix("0756b94461c50fb0", 16).unwrap();
        assert_eq!(hash, result);
    }

    #[test]
    fn test_key_after_e2e4_d7d5_e4e5() {
        let gs = GameState::from_fen("rnbqkbnr/ppp1pppp/8/3pP3/8/8/PPPP1PPP/RNBQKBNR b KQkq - 0 2")
            .unwrap();
        let hash = compute_hash(&gs);
        println!("{hash}");
        let result = u64::from_str_radix("662fafb965db29d4", 16).unwrap();
        assert_eq!(hash, result);
    }

    #[test]
    fn test_key_after_e2e4_d7d5_e4e5_f7f5() {
        let gs =
            GameState::from_fen("rnbqkbnr/ppp1p1pp/8/3pPp2/8/8/PPPP1PPP/RNBQKBNR w KQkq f6 0 3")
                .unwrap();
        let hash = compute_hash(&gs);
        println!("{hash}");
        let result = u64::from_str_radix("22a48b5a8e47ff78", 16).unwrap();
        assert_eq!(hash, result);
    }

    #[test]
    fn test_key_after_e2e4_d7d5_e4e5_f7f5_e1e2() {
        let gs = GameState::from_fen("rnbqkbnr/ppp1p1pp/8/3pPp2/8/8/PPPPKPPP/RNBQ1BNR b kq - 0 3")
            .unwrap();
        let hash = compute_hash(&gs);
        println!("{hash}");
        let result = u64::from_str_radix("652a607ca3f242c1", 16).unwrap();
        assert_eq!(hash, result);
    }

    #[test]
    fn test_key_after_e2e4_d7d5_e4e5_f7f5_e1e2_e8f7() {
        let gs = GameState::from_fen("rnbq1bnr/ppp1pkpp/8/3pPp2/8/8/PPPPKPPP/RNBQ1BNR w - - 0 4")
            .unwrap();
        let hash = compute_hash(&gs);
        println!("{hash}");
        let result = u64::from_str_radix("00fdd303c946bdd9", 16).unwrap();
        assert_eq!(hash, result);
    }

    #[test]
    fn test_key_after_a2a4_b7b5_h2h4_b5b4_c2c4() {
        let gs =
            GameState::from_fen("rnbqkbnr/p1pppppp/8/8/PpP4P/8/1P1PPPP1/RNBQKBNR b KQkq c3 0 3")
                .unwrap();
        let hash = compute_hash(&gs);
        println!("{hash}");
        let result = u64::from_str_radix("3c8123ea7b067637", 16).unwrap();
        assert_eq!(hash, result);
    }

    #[test]
    fn test_key_after_a2a4_b7b5_h2h4_b5b4_c2c4_b4c3_a1a3() {
        let gs =
            GameState::from_fen("rnbqkbnr/p1pppppp/8/8/P6P/R1p5/1P1PPPP1/1NBQKBNR b Kkq - 0 4")
                .unwrap();
        let hash = compute_hash(&gs);
        println!("{hash}");
        let result = u64::from_str_radix("5c3f9b829b279560", 16).unwrap();
        assert_eq!(hash, result);
    }
}
