use crate::{
    GameState,
    types::{
        Color, Coordinate,
        PieceType::{self, Bishop, Queen, Rook},
        Square,
    },
};

pub fn piece_mobility(gs: &GameState) -> i32 {
    let board = &gs.board;
    let mut mobility_score = 0;
    for i in 0..64 {
        let coord = Coordinate::from_idx(i);
        let sq = board.get(coord);
        if let Square::Occupied { color, piece } = sq {
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
                _ => continue,
            }

            match color {
                Color::White => mobility_score += mobility,
                _ => mobility_score += -(mobility as i32),
            }
        }
    }

    mobility_score
}
