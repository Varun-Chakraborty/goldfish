use crate::{
    GameState,
    types::{
        Coordinate,
        PieceType::{self, Bishop, Queen, Rook},
        Square,
    },
};

pub fn piece_mobility(gs: &GameState, coord: Coordinate, sq: &Square) -> i32 {
    if let Square::Occupied { color, piece } = sq {
        let mut mobility = 0;
        match piece {
            PieceType::Knight => {
                gs.knight_squares(coord, None, |_, sq| match sq {
                    Square::Occupied { color: c, .. } if c == *color => {}
                    _ => mobility += 4,
                });
            }
            piece @ (Bishop | Rook | Queen) => {
                gs.ray_tracing(coord, *piece, None, |_, sq| match sq {
                    Square::Occupied { color: c, .. } if c == *color => {}
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
        return mobility;
    }

    0
}
