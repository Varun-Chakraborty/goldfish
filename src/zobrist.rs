use crate::{
    game::GameState,
    types::{CastleSide, Color, Coordinate, PieceType, Square},
};
use rand::{RngExt, SeedableRng, rngs::StdRng};
use std::sync::LazyLock;

pub struct Zobrist {
    piece_sq: [[u64; 64]; 12],
    black_to_move: u64,
    en_passant_files: [u64; 8],
    castling_rights: [u64; 4],
}

impl Zobrist {
    fn new_zobrist() -> Self {
        let mut rng = StdRng::seed_from_u64(0xCAFEBABE);

        let mut piece_sq = [[0; 64]; 12];
        for piece in &mut piece_sq {
            for sq in piece {
                *sq = rng.random();
            }
        }

        let black_to_move = rng.random();

        let mut en_passant_files = [0; 8];
        for file in &mut en_passant_files {
            *file = rng.random();
        }

        let mut castling_rights = [0; 4];
        for right in &mut castling_rights {
            *right = rng.random();
        }

        Zobrist {
            piece_sq,
            black_to_move,
            en_passant_files,
            castling_rights,
        }
    }
}

static ZOBRIST: LazyLock<Zobrist> = LazyLock::new(Zobrist::new_zobrist);

pub fn get_piece_sq(piece: PieceType, color: Color, coord: Coordinate) -> u64 {
    let piece_index = match (piece, color) {
        (PieceType::King, Color::White) => 0,
        (PieceType::Queen, Color::White) => 1,
        (PieceType::Rook, Color::White) => 2,
        (PieceType::Bishop, Color::White) => 3,
        (PieceType::Knight, Color::White) => 4,
        (PieceType::Pawn, Color::White) => 5,
        (PieceType::King, Color::Black) => 6,
        (PieceType::Queen, Color::Black) => 7,
        (PieceType::Rook, Color::Black) => 8,
        (PieceType::Bishop, Color::Black) => 9,
        (PieceType::Knight, Color::Black) => 10,
        (PieceType::Pawn, Color::Black) => 11,
    };
    ZOBRIST.piece_sq[piece_index][coord.idx()]
}

pub fn get_black_to_move() -> u64 {
    ZOBRIST.black_to_move
}

pub fn get_en_passant_files(file: u8) -> u64 {
    ZOBRIST.en_passant_files[file as usize]
}

pub fn get_castling_rights(color: Color, side: CastleSide) -> u64 {
    let index = match (color, side) {
        (Color::White, CastleSide::Kingside) => 0,
        (Color::White, CastleSide::Queenside) => 1,
        (Color::Black, CastleSide::Kingside) => 2,
        (Color::Black, CastleSide::Queenside) => 3,
    };
    ZOBRIST.castling_rights[index]
}

pub fn compute_hash(gs: &GameState) -> u64 {
    let mut hash = 0;
    for i in 0..64 {
        let coord = Coordinate::from_idx(i);
        let sq = gs.board.get(coord);
        match sq {
            Square::Empty => continue,
            Square::Occupied { color, piece } => {
                let sq_hash = get_piece_sq(piece, color, coord);
                hash ^= sq_hash;
            }
        }
    }

    if gs.turn == Color::Black {
        hash ^= get_black_to_move();
    }

    if let Some(coord) = gs.en_passant {
        hash ^= get_en_passant_files(coord.file());
    }

    if gs.castling_rights.kingside_white {
        hash ^= get_castling_rights(Color::White, CastleSide::Kingside);
    }
    if gs.castling_rights.queenside_white {
        hash ^= get_castling_rights(Color::White, CastleSide::Queenside);
    }
    if gs.castling_rights.kingside_black {
        hash ^= get_castling_rights(Color::Black, CastleSide::Kingside);
    }
    if gs.castling_rights.queenside_black {
        hash ^= get_castling_rights(Color::Black, CastleSide::Queenside);
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
}
