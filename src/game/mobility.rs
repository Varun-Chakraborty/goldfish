use crate::{
    GameState,
    game::mobility,
    types::{
        Color, Coordinate,
        PieceType::{self, Bishop, Knight, Queen, Rook},
        Square,
    },
};

pub fn piece_mobility(gs: &GameState, coord: Coordinate) -> i32 {
    if let Square::Occupied { color, piece } = gs.board.get(coord) {
        let mut mobility = 0;
        match piece {
            PieceType::Knight => {
                gs.knight_squares(coord, None, |_, sq| match sq {
                    Square::Occupied { color: c, .. } if c == color => {}
                    _ => mobility += 4,
                });
            }
            piece @ (Bishop | Rook | Queen) => {
                gs.ray_tracing(coord, piece, None, |_, sq| match sq {
                    Square::Occupied { color: c, .. } if c == color => {}
                    _ => {
                        mobility += match piece {
                            Bishop => 3,
                            Rook => 2,
                            Queen => 1,
                            _ => unreachable!(),
                        }
                    }
                });
            }
            _ => {}
        }
        return mobility * (if color == Color::White { 1 } else { -1 });
    }

    0
}

pub fn collect_affected_pieces(
    gs: &GameState,
    changing_coord: Coordinate,
    affected_pieces: &mut u64,
) {
    gs.ray_tracing(changing_coord, Rook, None, |coord, sq| {
        if let Square::Occupied { piece, .. } = sq {
            if piece == Queen || piece == Rook {
                *affected_pieces |= 1 << coord.idx();
            }
        }
    });
    gs.ray_tracing(changing_coord, Bishop, None, |coord, sq| {
        if let Square::Occupied { piece, .. } = sq {
            if piece == Queen || piece == Bishop {
                *affected_pieces |= 1 << coord.idx();
            }
        }
    });
    gs.knight_squares(changing_coord, None, |coord, sq| {
        if let Square::Occupied { piece, .. } = sq {
            if piece == Knight {
                *affected_pieces |= 1 << coord.idx();
            }
        }
    });
}

pub fn mobility_of_affected_pieces(gs: &GameState, mut affected_pieces: u64) -> i32 {
    let mut mobility = 0;
    while affected_pieces != 0 {
        let coord = Coordinate::from_idx(affected_pieces.trailing_zeros() as usize);
        mobility += mobility::piece_mobility(&gs, coord);
        affected_pieces &= affected_pieces - 1;
    }
    mobility
}
