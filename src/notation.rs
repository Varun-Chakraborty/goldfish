use crate::{
    game::{GameState, MoveGenMode},
    types::{CastleSide, Coordinate, Move, PieceType, Square},
};

pub fn fmt_pgn_moves(gs: &GameState, moves: &[Move]) -> Vec<String> {
    let mut gs = gs.clone();
    let mut moves_string: Vec<String> = vec![];

    for m in moves {
        if let Some(castle) = m.castle {
            match castle {
                CastleSide::Kingside => moves_string.push("O-O".to_string()),
                CastleSide::Queenside => moves_string.push("O-O-O".to_string()),
            }

            gs.make_move(*m);
            continue;
        }

        let piece_char = match m.piece {
            PieceType::King => "K",
            PieceType::Queen => "Q",
            PieceType::Rook => "R",
            PieceType::Bishop => "B",
            PieceType::Knight => "N",
            PieceType::Pawn => "",
        };

        // disambiguation: find peers that also attack the same square
        let disambig = if m.piece != PieceType::Pawn && m.piece != PieceType::King {
            let legal = gs.legal_moves(MoveGenMode::All);
            let peers: Vec<Move> = legal.into_iter()
                    .filter(|m1| {
                        let sq = gs.board.get(m.from);
                        matches!(sq, Square::Occupied { piece: p, .. } if p == m1.piece && m.from != m1.from && m.to == m1.to)
                    })
                    .collect();
            if peers.is_empty() {
                String::new()
            } else {
                let same_file = peers.iter().any(|c| c.from.file() == m.from.file());
                let same_rank = peers.iter().any(|c| c.from.rank() == m.from.rank());
                if !same_file {
                    m.from
                        .to_algebraic()
                        .chars()
                        .next()
                        .expect("algebraic has at least 1 char")
                        .to_string()
                } else if !same_rank {
                    m.from
                        .to_algebraic()
                        .chars()
                        .nth(1)
                        .expect("algebraic has at least 2 chars")
                        .to_string()
                } else {
                    m.from.to_algebraic()
                }
            }
        } else {
            String::new()
        };

        let is_capture = m.captured.is_some() || m.en_passant;

        let from_file = m
            .from
            .to_algebraic()
            .chars()
            .next()
            .expect("algebraic has at least 1 char");
        let capture_str = if is_capture {
            if m.piece == PieceType::Pawn {
                format!("{}x", from_file)
            } else {
                "x".to_string()
            }
        } else {
            String::new()
        };
        let to_str = m.to.to_algebraic();
        let promo_str = match m.promotion {
            Some(PieceType::Queen) => "=Q",
            Some(PieceType::Rook) => "=R",
            Some(PieceType::Bishop) => "=B",
            Some(PieceType::Knight) => "=N",
            _ => "",
        };

        gs.make_move(*m);

        let check_str = if gs.in_check(gs.turn) {
            if gs.legal_moves(MoveGenMode::All).is_empty() {
                "#"
            } else {
                "+"
            }
        } else {
            ""
        };

        moves_string.push(format!(
            "{piece_char}{disambig}{capture_str}{to_str}{promo_str}{check_str}"
        ));
    }

    moves_string
}

fn parse_san(gs: &mut GameState, s: &str) -> Option<Move> {
    let s = s.trim();
    let legal = gs.legal_moves(MoveGenMode::All);

    let lower = s.to_lowercase();
    for m in legal {
        let pgn_move = fmt_pgn_moves(gs, &[m])[0].to_lowercase();
        if pgn_move == lower {
            return Some(m);
        }
    }
    None
}

fn parse_coordinate(gs: &mut GameState, s: &str) -> Option<Move> {
    if !(4..=5).contains(&s.len()) {
        return None;
    }

    let legal = gs.legal_moves(MoveGenMode::All);
    let from = Coordinate::from_algebraic(&s[0..2])?;
    let to = Coordinate::from_algebraic(&s[2..4])?;
    let promo = match s.chars().nth(4) {
        Some(c) => match c {
            'q' | 'Q' => Some(PieceType::Queen),
            'r' | 'R' => Some(PieceType::Rook),
            'b' | 'B' => Some(PieceType::Bishop),
            'n' | 'N' => Some(PieceType::Knight),
            _ => None,
        },
        None => None,
    };

    legal
        .into_iter()
        .find(|&m| m.from == from && m.to == to && m.promotion == promo)
}

pub fn parse_algebraic(gs: &mut GameState, s: &str) -> Option<Move> {
    let s = match s {
        "O-O" | "o-o" | "0-0" => "O-O",
        "O-O-O" | "o-o-o" | "0-0-0" => "O-O-O",
        _ => s,
    };
    parse_coordinate(gs, s).or_else(|| parse_san(gs, s))
}
