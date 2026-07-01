use crate::{
    GameState,
    game::Direction::{Horizontal, LeftDiagonal, RightDiagonal, Vertical},
    types::{
        Color::{Black, White},
        Move,
        PieceType::{Pawn, Queen},
    },
};

pub fn see(gs: &mut GameState, &m: &Move) -> i32 {
    let to = m.to;
    let mut score =
        m.captured.map_or(0, |p| p.value()) + m.promotion.map_or(0, |p| p.value() - 100);

    let undo = gs.make_move(m);
    let pin_map = gs.compute_pins();

    let mut exclusion_list = vec![];
    let mut lvas = vec![];
    let mut lva_piece = None;

    loop {
        let mut lva;
        loop {
            lva = gs.lva(m.to, gs.turn, None, Some(&exclusion_list));
            if let Some((from, _)) = lva
                && let Some(dir) = pin_map.get(from)
            {
                let dx = from.file() as i8 - to.file() as i8;
                let dy = from.rank() as i8 - to.rank() as i8;

                let dx = dx.signum();
                let dy = dy.signum();

                let legal = match dir {
                    RightDiagonal => dx == dy,
                    LeftDiagonal => dx == -dy,
                    Vertical => dx == 0,
                    Horizontal => dy == 0,
                    _ => unreachable!(),
                };

                if legal {
                    break;
                }

                exclusion_list.push(from);
            } else {
                break;
            }
        }

        if let Some(lva) = lva {
            if lva_piece.is_some_and(|piece| piece != lva.1) {
                break;
            }
            lvas.push(lva);
            lva_piece = Some(lva.1);
            exclusion_list.push(lva.0);
        } else {
            break;
        }
    }

    let mut lowest_score = None;

    for lva in lvas {
        let score;
        let (from, piece) = lva;
        let promotion = match piece {
            Pawn => match (gs.turn, from.rank()) {
                (White, 6) | (Black, 1) => true,
                _ => false,
            },
            _ => false,
        };
        if promotion {
            score = -see(
                gs,
                &Move::new_promotion_and_capture(from, to, Queen, m.piece),
            );
        } else {
            score = -see(
                gs,
                &Move::new_capture(from, to, piece, m.promotion.unwrap_or(m.piece)),
            );
        }
        if lowest_score.is_none_or(|ls| score < ls) {
            lowest_score = Some(score);
        }
    }

    gs.unmake_move(undo);

    if let Some(ls) = lowest_score {
        score += ls;
    }

    score
}

#[cfg(test)]
mod tests {
    use crate::{GameState, notation::parse_algebraic, see::see};

    #[test]
    fn test_see() {
        let mut gs = GameState::from_fen("rnr2k2/PR6/8/8/8/8/8/K7 w KQkq - 0 1").unwrap();
        let m = parse_algebraic(&mut gs, "Rxb8").unwrap();
        println!("{}", gs.representation());
        let score = see(&mut gs, &m);
        println!("score: {}", score);
        assert_eq!(score, -180);
    }
}
