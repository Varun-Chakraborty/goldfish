use crate::{
    GameState,
    types::{Color, Coordinate, PieceType, Square},
};

fn phase_factor(material: u16) -> f32 {
    let p = material as f32 / 8000.0;
    p.clamp(0.0, 1.0)
}

fn pawn_shield(gs: &GameState, color: Color) -> i32 {
    let board = &gs.board;
    let mut score = 0;

    let (king, dr) = match color {
        Color::White => (gs.wk, 1),
        Color::Black => (gs.bk, -1),
    };

    for df in [-1, 0, 1] {
        if let Some(coord) = king.checked_offset(df, dr) {
            match board.get(coord) {
                Square::Occupied { piece, color } if color == Color::White => {
                    if piece == PieceType::Pawn {
                        score += 20;
                    } else {
                        score += 5;
                    }
                }
                _ => {}
            }
        }
    }

    score
}

pub fn king_safety(gs: &GameState, material_on_board: u16) -> i32 {
    let mut safety = 0;

    let g1 = Coordinate::new(6, 0);
    let c1 = Coordinate::new(2, 0);
    if gs.wk == g1 || gs.wk == c1 {
        safety += pawn_shield(gs, Color::White);
    }

    let g8 = Coordinate::new(6, 7);
    let c8 = Coordinate::new(2, 7);
    if gs.bk == g8 || gs.bk == c8 {
        safety -= pawn_shield(gs, Color::Black);
    }

    let phase = phase_factor(material_on_board);
    safety = (safety as f32 * phase) as i32;
    safety
}
