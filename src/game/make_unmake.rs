use crate::{
    GameState,
    game::{
        Undo,
        mobility::{collect_affected_pieces, mobility_of_affected_pieces, piece_mobility},
        pst::pst,
    },
    types::{CastleSide, Color, Coordinate, Move, PieceType, Square},
    zobrist::{castling_hash, ep_hash, piece_sq_hash, side_hash},
};

impl GameState {
    fn apply_kingside_castle(&mut self, us: Color, rank: u8, affected_pieces: &mut u64) {
        match us {
            Color::White => self.wk = Coordinate::new(6, 0),
            Color::Black => self.bk = Coordinate::new(6, 7),
        }

        let ex = Coordinate::new(4, rank);
        let fx = Coordinate::new(5, rank);
        let gx = Coordinate::new(6, rank);
        let hx = Coordinate::new(7, rank);

        collect_affected_pieces(self, ex, affected_pieces);
        collect_affected_pieces(self, fx, affected_pieces);
        collect_affected_pieces(self, gx, affected_pieces);
        collect_affected_pieces(self, hx, affected_pieces);

        *affected_pieces &= !(1 << ex.idx()) & !(1 << hx.idx());

        self.mobility_score -= piece_mobility(self, hx);
        self.pst_score -= pst(&PieceType::King, us, ex.idx());
        self.pst_score -= pst(&PieceType::Rook, us, hx.idx());
        self.mobility_score -= mobility_of_affected_pieces(self, *affected_pieces);

        self.board.set(ex, Square::Empty);
        self.zobrist ^= piece_sq_hash(PieceType::King, us, ex);
        self.board.set(hx, Square::Empty);
        self.zobrist ^= piece_sq_hash(PieceType::Rook, us, hx);

        let king = Square::Occupied {
            color: us,
            piece: PieceType::King,
        };
        let rook = Square::Occupied {
            color: us,
            piece: PieceType::Rook,
        };
        self.board.set(gx, king);
        self.zobrist ^= piece_sq_hash(PieceType::King, us, gx);
        self.board.set(fx, rook);
        self.zobrist ^= piece_sq_hash(PieceType::Rook, us, fx);

        self.mobility_score += piece_mobility(self, fx);
        self.pst_score += pst(&PieceType::Rook, us, fx.idx());
        self.pst_score += pst(&PieceType::King, us, gx.idx());
        self.mobility_score += mobility_of_affected_pieces(self, *affected_pieces);
    }

    fn apply_queenside_castle(&mut self, us: Color, rank: u8, affected_pieces: &mut u64) {
        match us {
            Color::White => self.wk = Coordinate::new(2, 0),
            Color::Black => self.bk = Coordinate::new(2, 7),
        }

        let ax = Coordinate::new(0, rank);
        let cx = Coordinate::new(2, rank);
        let dx = Coordinate::new(3, rank);
        let ex = Coordinate::new(4, rank);

        collect_affected_pieces(self, ax, affected_pieces);
        collect_affected_pieces(self, cx, affected_pieces);
        collect_affected_pieces(self, dx, affected_pieces);
        collect_affected_pieces(self, ex, affected_pieces);

        *affected_pieces &= !(1 << ax.idx()) & !(1 << ex.idx());

        self.mobility_score -= piece_mobility(self, ax);
        self.pst_score -= pst(&PieceType::King, us, ex.idx());
        self.pst_score -= pst(&PieceType::Rook, us, ax.idx());
        self.mobility_score -= mobility_of_affected_pieces(self, *affected_pieces);

        self.board.set(ex, Square::Empty);
        self.zobrist ^= piece_sq_hash(PieceType::King, us, ex);
        self.board.set(ax, Square::Empty);
        self.zobrist ^= piece_sq_hash(PieceType::Rook, us, ax);

        let king = Square::Occupied {
            color: us,
            piece: PieceType::King,
        };
        let rook = Square::Occupied {
            color: us,
            piece: PieceType::Rook,
        };
        self.board.set(cx, king);
        self.zobrist ^= piece_sq_hash(PieceType::King, us, cx);
        self.board.set(dx, rook);
        self.zobrist ^= piece_sq_hash(PieceType::Rook, us, dx);

        self.mobility_score += piece_mobility(self, dx);
        self.pst_score += pst(&PieceType::King, us, cx.idx());
        self.pst_score += pst(&PieceType::Rook, us, dx.idx());

        self.mobility_score += mobility_of_affected_pieces(self, *affected_pieces);
    }

    pub fn make_move(&mut self, m: Move) -> Undo {
        let undo = Undo {
            castling_rights: self.castling_rights,
            en_passant: self.en_passant,
            halfmove_clock: self.halfmove_clock,
            move_info: m,
            pst_score: self.pst_score,
            mobility_score: self.mobility_score,
        };
        self.history.push(self.zobrist);
        self.zobrist ^= side_hash();

        let us = self.turn;
        let them = us.opponent();
        self.turn = them;

        let mut affected_pieces = 0;

        if us == Color::Black {
            self.fullmove_number += 1;
        }

        if let Some(ep) = self.en_passant {
            self.zobrist ^= ep_hash(ep, &self.board, us);
        }
        self.en_passant = None;

        if let Some(castle) = m.castle {
            let rank = match us {
                Color::White => 0,
                Color::Black => 7,
            };
            match castle {
                CastleSide::Kingside => self.apply_kingside_castle(us, rank, &mut affected_pieces),
                CastleSide::Queenside => {
                    self.apply_queenside_castle(us, rank, &mut affected_pieces)
                }
            }
            if self.can_castle_kingside(us) {
                self.zobrist ^= castling_hash(us, CastleSide::Kingside);
            }
            if self.can_castle_queenside(us) {
                self.zobrist ^= castling_hash(us, CastleSide::Queenside);
            }
            self.set_castle(us, false, false);
            return undo;
        }

        collect_affected_pieces(self, m.from, &mut affected_pieces);
        collect_affected_pieces(self, m.to, &mut affected_pieces);
        if m.en_passant {
            let capture_coord = Coordinate::new(m.to.file(), m.from.rank());
            collect_affected_pieces(self, capture_coord, &mut affected_pieces);
            affected_pieces &= !(1 << capture_coord.idx());
        }
        affected_pieces &= !(1 << m.from.idx()) & !(1 << m.to.idx());

        self.mobility_score -= piece_mobility(self, m.from);
        self.mobility_score -= piece_mobility(self, m.to);

        self.mobility_score -= mobility_of_affected_pieces(self, affected_pieces);

        let moving_piece = m.piece;
        let is_pawn_move: bool = matches!(moving_piece, PieceType::Pawn);

        let is_capture = match m.captured {
            Some(piece) => {
                let coord = if m.en_passant {
                    Coordinate::new(m.to.file(), m.from.rank())
                } else {
                    m.to
                };
                self.wp[coord.file() as usize] &= !(1 << coord.rank());
                self.bp[coord.file() as usize] &= !(1 << coord.rank());
                self.wb &= !(1 << coord.idx());
                self.bb &= !(1 << coord.idx());

                self.zobrist ^= piece_sq_hash(piece, them, coord);
                self.remove_material(piece, them, 1);

                self.pst_score -= pst(&piece, them, coord.idx());

                self.board.set(coord, Square::Empty);

                let rank: u8 = match them {
                    Color::White => 0,
                    Color::Black => 7,
                };
                if m.to.file() == 0 && m.to.rank() == rank {
                    if self.can_castle_queenside(them) {
                        self.zobrist ^= castling_hash(them, CastleSide::Queenside);
                    }
                    match them {
                        Color::White => self.castling_rights.queenside_white = false,
                        Color::Black => self.castling_rights.queenside_black = false,
                    }
                }
                if m.to.file() == 7 && m.to.rank() == rank {
                    if self.can_castle_kingside(them) {
                        self.zobrist ^= castling_hash(them, CastleSide::Kingside);
                    }
                    match them {
                        Color::White => self.castling_rights.kingside_white = false,
                        Color::Black => self.castling_rights.kingside_black = false,
                    }
                }

                true
            }
            None => false,
        };

        if is_pawn_move {
            let dir = m.to.rank() as i8 - m.from.rank() as i8;
            if dir.abs() == 2 {
                let ep = Coordinate::new(m.from.file(), (m.from.rank() as i8 + dir.signum()) as u8);
                self.en_passant = Some(ep);
                self.zobrist ^= ep_hash(ep, &self.board, them);
            }
        }

        if moving_piece == PieceType::King {
            if self.can_castle_kingside(us) {
                self.zobrist ^= castling_hash(us, CastleSide::Kingside);
            }
            if self.can_castle_queenside(us) {
                self.zobrist ^= castling_hash(us, CastleSide::Queenside);
            }
            self.set_castle(us, false, false);
            match us {
                Color::White => self.wk = m.to,
                Color::Black => self.bk = m.to,
            }
        }

        if moving_piece == PieceType::Rook {
            let rank: u8 = match us {
                Color::White => 0,
                Color::Black => 7,
            };
            if m.from == Coordinate::new(0, rank) {
                if self.can_castle_queenside(us) {
                    self.zobrist ^= castling_hash(us, CastleSide::Queenside);
                }
                match us {
                    Color::White => self.castling_rights.queenside_white = false,
                    Color::Black => self.castling_rights.queenside_black = false,
                }
            }
            if m.from == Coordinate::new(7, rank) {
                if self.can_castle_kingside(us) {
                    self.zobrist ^= castling_hash(us, CastleSide::Kingside);
                }
                match us {
                    Color::White => self.castling_rights.kingside_white = false,
                    Color::Black => self.castling_rights.kingside_black = false,
                }
            }
        }

        self.pst_score -= pst(&moving_piece, us, m.from.idx());
        self.board.set(m.to, self.board.get(m.from));
        self.board.set(m.from, Square::Empty);
        self.zobrist ^= piece_sq_hash(moving_piece, us, m.from);
        self.pst_score += pst(&moving_piece, us, m.to.idx());

        match (moving_piece, us) {
            (PieceType::Pawn, Color::White) => {
                self.wp[m.from.file() as usize] &= !(1 << m.from.rank());
                self.wp[m.to.file() as usize] |= 1 << m.to.rank();
            }
            (PieceType::Pawn, Color::Black) => {
                self.bp[m.from.file() as usize] &= !(1 << m.from.rank());
                self.bp[m.to.file() as usize] |= 1 << m.to.rank();
            }
            (PieceType::Bishop, Color::White) => {
                self.wb &= !(1 << m.from.idx());
                self.wb |= 1 << m.to.idx();
            }
            (PieceType::Bishop, Color::Black) => {
                self.bb &= !(1 << m.from.idx());
                self.bb |= 1 << m.to.idx();
            }
            _ => (),
        }

        if let Some(promo) = m.promotion {
            self.board.set(
                m.to,
                Square::Occupied {
                    color: us,
                    piece: promo,
                },
            );
            match (promo, us) {
                (PieceType::Bishop, Color::White) => {
                    self.wb |= 1 << m.to.idx();
                }
                (PieceType::Bishop, Color::Black) => {
                    self.bb |= 1 << m.to.idx();
                }
                _ => (),
            }
            self.pst_score += pst(&promo, us, m.to.idx());
            self.zobrist ^= piece_sq_hash(promo, us, m.to);
            self.add_material(promo, us, 1);
            self.remove_material(moving_piece, us, 1);
        } else {
            self.zobrist ^= piece_sq_hash(moving_piece, us, m.to);
        }

        if is_pawn_move || is_capture {
            self.halfmove_clock = 0;
        } else {
            self.halfmove_clock += 1;
        }

        self.mobility_score += piece_mobility(self, m.to);
        self.mobility_score += mobility_of_affected_pieces(self, affected_pieces);

        undo
    }

    fn unapply_kingside_castle(&mut self, us: Color, rank: u8) {
        match us {
            Color::White => self.wk = Coordinate::new(4, 0),
            Color::Black => self.bk = Coordinate::new(4, 7),
        }
        self.board.set(Coordinate::new(6, rank), Square::Empty);
        self.board.set(Coordinate::new(5, rank), Square::Empty);
        self.board.set(
            Coordinate::new(4, rank),
            Square::Occupied {
                color: us,
                piece: PieceType::King,
            },
        );
        self.board.set(
            Coordinate::new(7, rank),
            Square::Occupied {
                color: us,
                piece: PieceType::Rook,
            },
        );
    }

    fn unapply_queenside_castle(&mut self, us: Color, rank: u8) {
        match us {
            Color::White => self.wk = Coordinate::new(4, 0),
            Color::Black => self.bk = Coordinate::new(4, 7),
        }
        self.board.set(Coordinate::new(2, rank), Square::Empty);
        self.board.set(Coordinate::new(3, rank), Square::Empty);
        self.board.set(
            Coordinate::new(4, rank),
            Square::Occupied {
                color: us,
                piece: PieceType::King,
            },
        );
        self.board.set(
            Coordinate::new(0, rank),
            Square::Occupied {
                color: us,
                piece: PieceType::Rook,
            },
        );
    }

    pub fn unmake_move(&mut self, undo: Undo) {
        let them = self.turn;
        let us = them.opponent();
        self.turn = us;
        self.zobrist = self
            .history
            .pop()
            .expect("No history found while unmaking move");

        if us == Color::Black {
            self.fullmove_number -= 1;
        }

        self.castling_rights = undo.castling_rights;
        self.en_passant = undo.en_passant;
        self.halfmove_clock = undo.halfmove_clock;
        self.pst_score = undo.pst_score;
        self.mobility_score = undo.mobility_score;

        let m = undo.move_info;

        if let Some(castle) = m.castle {
            let rank = match us {
                Color::White => 0,
                Color::Black => 7,
            };
            match castle {
                CastleSide::Kingside => self.unapply_kingside_castle(us, rank),
                CastleSide::Queenside => self.unapply_queenside_castle(us, rank),
            }
            return;
        };

        let moving_piece = m.piece;
        let sq = self.board.get(m.to);

        let sq = match sq {
            Square::Occupied { color, piece } if piece == moving_piece && color == self.turn => {
                (color, piece)
            }
            Square::Occupied { color, piece }
                if Some(piece) == m.promotion && color == self.turn =>
            {
                self.remove_material(piece, color, 1);
                self.add_material(moving_piece, color, 1);
                (color, PieceType::Pawn)
            }
            other => unreachable!(
                "Invalid undo: expected {moving_piece:?} on {} but found {other:?}",
                m.to.to_algebraic()
            ),
        };

        self.board.set(
            m.from,
            Square::Occupied {
                color: sq.0,
                piece: sq.1,
            },
        );
        self.board.set(m.to, Square::Empty);

        match (m.piece, us) {
            (PieceType::Pawn, Color::White) => {
                self.wp[m.to.file() as usize] &= !(1 << m.to.rank());
                self.wp[m.from.file() as usize] |= 1 << m.from.rank();
            }
            (PieceType::Pawn, Color::Black) => {
                self.bp[m.to.file() as usize] &= !(1 << m.to.rank());
                self.bp[m.from.file() as usize] |= 1 << m.from.rank();
            }
            (PieceType::Bishop, Color::White) => {
                self.wb &= !(1 << m.to.idx());
                self.wb |= 1 << m.from.idx();
            }
            (PieceType::Bishop, Color::Black) => {
                self.bb &= !(1 << m.to.idx());
                self.bb |= 1 << m.from.idx();
            }
            _ => (),
        }

        if sq.1 == PieceType::King {
            match self.turn {
                Color::White => self.wk = m.from,
                Color::Black => self.bk = m.from,
            }
        }

        if let Some(piece) = m.captured {
            let coord = if m.en_passant {
                Coordinate::new(m.to.file(), m.from.rank())
            } else {
                m.to
            };
            match (piece, them) {
                (PieceType::Pawn, Color::White) => {
                    self.wp[coord.file() as usize] |= 1 << coord.rank()
                }
                (PieceType::Pawn, Color::Black) => {
                    self.bp[coord.file() as usize] |= 1 << coord.rank()
                }
                (PieceType::Bishop, Color::White) => self.wb |= 1 << coord.idx(),
                (PieceType::Bishop, Color::Black) => self.bb |= 1 << coord.idx(),
                _ => (),
            }
            self.board
                .set(coord, Square::Occupied { color: them, piece });
            self.add_material(piece, them, 1);
        }
    }
}
