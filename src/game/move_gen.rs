use crate::{
    GameState,
    game::{
        Direction::{self, LeftDiagonal, RightDiagonal, Vertical},
        MoveGenMode, PinMap,
    },
    types::{CastleSide, Color, Coordinate, Move, PieceType, Square},
};

impl GameState {
    fn pawn_moves(
        &mut self,
        from: Coordinate,
        moves: &mut Vec<Move>,
        target_squares: &Option<Vec<Coordinate>>,
        pin_map: &PinMap,
        move_gen_mode: MoveGenMode,
    ) {
        let color = self.turn;
        let (dir, start_rank, promo_rank) = match color {
            Color::White => (1i8, 1u8, 7u8),
            Color::Black => (-1i8, 6u8, 0u8),
        };

        let pinned = pin_map.get(from);

        if pinned.is_none_or(|dir| dir == Vertical) && move_gen_mode == MoveGenMode::All {
            if let Some(to) = from.checked_offset(0, dir)
                && self.board.get(to) == Square::Empty
            {
                if target_squares
                    .as_ref()
                    .is_none_or(|sqrs| sqrs.contains(&to))
                {
                    if to.rank() == promo_rank {
                        for &p in &[
                            PieceType::Queen,
                            PieceType::Rook,
                            PieceType::Bishop,
                            PieceType::Knight,
                        ] {
                            moves.push(Move::new_promotion(from, to, p));
                        }
                    } else {
                        moves.push(Move::new(from, to, PieceType::Pawn));
                    }
                }
                if from.rank() == start_rank
                    && let Some(to2) = from.checked_offset(0, 2 * dir)
                    && self.board.get(to2) == Square::Empty
                {
                    if target_squares
                        .as_ref()
                        .is_none_or(|sqrs| sqrs.contains(&to2))
                    {
                        moves.push(Move::new(from, to2, PieceType::Pawn));
                    }
                }
            }
        }

        if pinned.is_none_or(|dir| dir == RightDiagonal) {
            let df = dir;
            if let Some(to) = from.checked_offset(df, dir) {
                if target_squares
                    .as_ref()
                    .is_none_or(|sqrs| sqrs.contains(&to))
                {
                    match self.board.get(to) {
                        Square::Occupied { color: c, piece } if c != color => {
                            if to.rank() == promo_rank {
                                for &p in &[
                                    PieceType::Queen,
                                    PieceType::Rook,
                                    PieceType::Bishop,
                                    PieceType::Knight,
                                ] {
                                    moves.push(Move::new_promotion_and_capture(from, to, p, piece));
                                }
                            } else {
                                moves.push(Move::new_capture(from, to, PieceType::Pawn, piece));
                            }
                        }
                        _ => {}
                    }
                }
                if self.en_passant == Some(to)
                    && target_squares
                        .as_ref()
                        .is_none_or(|sqrs| sqrs.contains(&Coordinate::new(to.file(), from.rank())))
                {
                    let m = Move::new_en_passant(from, to);
                    let undo = self.make_move(m);
                    if !self.in_check(color) {
                        moves.push(m);
                    }
                    self.unmake_move(undo);
                }
            }
        }

        if pinned.is_none_or(|dir| dir == LeftDiagonal) {
            let df = -dir;
            if let Some(to) = from.checked_offset(df, dir) {
                if target_squares
                    .as_ref()
                    .is_none_or(|sqrs| sqrs.contains(&to))
                {
                    match self.board.get(to) {
                        Square::Occupied { color: c, piece } if c != color => {
                            if to.rank() == promo_rank {
                                for &p in &[
                                    PieceType::Queen,
                                    PieceType::Rook,
                                    PieceType::Bishop,
                                    PieceType::Knight,
                                ] {
                                    moves.push(Move::new_promotion_and_capture(from, to, p, piece));
                                }
                            } else {
                                moves.push(Move::new_capture(from, to, PieceType::Pawn, piece));
                            }
                        }
                        _ => {}
                    }
                }
                if self.en_passant == Some(to)
                    && target_squares
                        .as_ref()
                        .is_none_or(|sqrs| sqrs.contains(&Coordinate::new(to.file(), from.rank())))
                {
                    let m = Move::new_en_passant(from, to);
                    let undo = self.make_move(m);
                    if !self.in_check(color) {
                        moves.push(m);
                    }
                    self.unmake_move(undo);
                }
            }
        }
    }

    fn knight_moves(
        &self,
        from: Coordinate,
        moves: &mut Vec<Move>,
        target_squares: &Option<Vec<Coordinate>>,
        pin_map: Option<&PinMap>,
        move_gen_mode: MoveGenMode,
    ) {
        let color = self.turn;
        let pinned = pin_map.and_then(|pm| pm.get(from));
        self.knight_squares(from, pinned, |to, sq| {
            if target_squares
                .as_ref()
                .is_some_and(|sqrs| !sqrs.contains(&to))
            {
                return;
            }
            match sq {
                Square::Occupied { color: c, piece } if c != color => {
                    moves.push(Move::new_capture(from, to, PieceType::Knight, piece))
                }
                Square::Empty => {
                    if move_gen_mode == MoveGenMode::All {
                        moves.push(Move::new(from, to, PieceType::Knight))
                    }
                }
                _ => {}
            }
        })
    }

    fn sliding_moves(
        &self,
        from: Coordinate,
        piece: PieceType,
        moves: &mut Vec<Move>,
        target_squares: &Option<Vec<Coordinate>>,
        pin_map: Option<&PinMap>,
        move_gen_mode: MoveGenMode,
    ) {
        let color = self.turn;
        let pinned = pin_map.and_then(|pm| pm.get(from));

        self.ray_tracing(from, piece, pinned, |to, sq| {
            if target_squares
                .as_ref()
                .is_some_and(|sqrs| !sqrs.contains(&to))
            {
                return;
            }
            match sq {
                Square::Empty => {
                    if move_gen_mode == MoveGenMode::All {
                        moves.push(Move::new(from, to, piece))
                    }
                }
                Square::Occupied {
                    piece: captured,
                    color: c,
                } if c != color => {
                    moves.push(Move::new_capture(from, to, piece, captured));
                }
                _ => {}
            }
        });
    }

    fn king_moves(
        &self,
        from: Coordinate,
        moves: &mut Vec<Move>,
        checks: &[Option<(Coordinate, Direction)>; 2],
        move_gen_mode: MoveGenMode,
    ) {
        let color = self.turn;
        for df in -1i8..=1 {
            for dr in -1i8..=1 {
                if df == 0 && dr == 0 {
                    continue;
                }
                if let Some(to) = from.checked_offset(df, dr) {
                    let prohibited = !checks.iter().flatten().any(|(c, _)| c == &to)
                        && (df == dr
                            && checks
                                .iter()
                                .flatten()
                                .any(|(_, d)| d == &Direction::RightDiagonal)
                            || df == -dr
                                && checks
                                    .iter()
                                    .flatten()
                                    .any(|(_, d)| d == &Direction::LeftDiagonal)
                            || df == 0
                                && checks
                                    .iter()
                                    .flatten()
                                    .any(|(_, d)| d == &Direction::Vertical)
                            || dr == 0
                                && checks
                                    .iter()
                                    .flatten()
                                    .any(|(_, d)| d == &Direction::Horizontal));

                    if prohibited {
                        continue;
                    }

                    match self.board.get(to) {
                        Square::Occupied { color: c, .. } if c == color => continue,
                        Square::Occupied { piece, .. } => {
                            let is_attacked = self.lva(to, color.opponent(), None, None).is_some();
                            if is_attacked {
                                continue;
                            }
                            moves.push(Move::new_capture(from, to, PieceType::King, piece))
                        }
                        _ => {
                            if move_gen_mode != MoveGenMode::All {
                                continue;
                            }

                            let is_attacked = self.lva(to, color.opponent(), None, None).is_some();
                            if is_attacked {
                                continue;
                            }
                            moves.push(Move::new(from, to, PieceType::King))
                        }
                    }
                }
            }
        }

        let rank: u8 = match color {
            Color::White => 0,
            Color::Black => 7,
        };

        if self.can_castle_kingside(color) {
            let king_sq = Coordinate::new(4, rank);
            let between_empty = [Coordinate::new(5, rank), Coordinate::new(6, rank)];
            if from == king_sq
                && self.board.get(Coordinate::new(7, rank))
                    == (Square::Occupied {
                        color,
                        piece: PieceType::Rook,
                    })
                && between_empty
                    .iter()
                    .all(|&c| self.board.get(c) == Square::Empty)
                && !self
                    .lva(Coordinate::new(4, rank), color.opponent(), None, None)
                    .is_some()
                && !self
                    .lva(Coordinate::new(5, rank), color.opponent(), None, None)
                    .is_some()
                && !self
                    .lva(Coordinate::new(6, rank), color.opponent(), None, None)
                    .is_some()
            {
                moves.push(Move::new_castle(
                    from,
                    Coordinate::new(6, rank),
                    CastleSide::Kingside,
                ));
            }
        }

        if self.can_castle_queenside(color) {
            let king_sq = Coordinate::new(4, rank);
            let between_empty = [
                Coordinate::new(1, rank),
                Coordinate::new(2, rank),
                Coordinate::new(3, rank),
            ];
            if from == king_sq
                && self.board.get(Coordinate::new(0, rank))
                    == (Square::Occupied {
                        color,
                        piece: PieceType::Rook,
                    })
                && between_empty
                    .iter()
                    .all(|&c| self.board.get(c) == Square::Empty)
                && !self
                    .lva(Coordinate::new(4, rank), color.opponent(), None, None)
                    .is_some()
                && !self
                    .lva(Coordinate::new(3, rank), color.opponent(), None, None)
                    .is_some()
                && !self
                    .lva(Coordinate::new(2, rank), color.opponent(), None, None)
                    .is_some()
            {
                moves.push(Move::new_castle(
                    from,
                    Coordinate::new(2, rank),
                    CastleSide::Queenside,
                ));
            }
        }
    }

    pub fn legal_moves(&mut self, move_gen_mode: MoveGenMode) -> Vec<Move> {
        let us = self.turn;
        let mut legal_moves = Vec::with_capacity(70);
        let checks = self.checks(us);
        let mut target_squares = None;

        if let Some(_) = checks[1] {
            let king = match us {
                Color::White => self.wk,
                Color::Black => self.bk,
            };
            self.king_moves(king, &mut legal_moves, &checks, move_gen_mode);
            return legal_moves;
        } else if let Some(c) = checks[0] {
            let mut targets = vec![c.0];
            let direction = c.1;
            match direction {
                Direction::Offset => {}
                _ => {
                    let king = match us {
                        Color::White => self.wk,
                        Color::Black => self.bk,
                    };

                    let df = c.0.file() as i8 - king.file() as i8;
                    let dr = c.0.rank() as i8 - king.rank() as i8;

                    let dr = dr.signum();
                    let df = df.signum();

                    let mut cell = king;
                    while let Some(next_cell) = cell.checked_offset(df, dr) {
                        cell = next_cell;
                        match self.board.get(next_cell) {
                            Square::Empty => targets.push(next_cell),
                            _ => break,
                        }
                    }
                }
            }
            target_squares = Some(targets);
        }

        let pin_map = self.compute_pins();

        for i in 0..64 {
            let from = Coordinate::from_idx(i);
            let sq = self.board.get(from);
            let piece = match sq {
                Square::Occupied { color, piece } if color == us => piece,
                _ => continue,
            };

            match piece {
                PieceType::King => self.king_moves(from, &mut legal_moves, &checks, move_gen_mode),
                PieceType::Pawn => self.pawn_moves(
                    from,
                    &mut legal_moves,
                    &target_squares,
                    &pin_map,
                    move_gen_mode,
                ),
                PieceType::Knight => self.knight_moves(
                    from,
                    &mut legal_moves,
                    &target_squares,
                    Some(&pin_map),
                    move_gen_mode,
                ),
                piece => self.sliding_moves(
                    from,
                    piece,
                    &mut legal_moves,
                    &target_squares,
                    Some(&pin_map),
                    move_gen_mode,
                ),
            }
        }

        legal_moves
    }
}
