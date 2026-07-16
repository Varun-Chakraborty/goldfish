use std::num::ParseIntError;

use crate::{
    board::{Board, BoardError},
    types::{
        CastleSide, CastlingRights, Color, ColorError, Coordinate, Move,
        PieceType, Square,
    },
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GameStateError {
    #[error("FEN should have 6 space separated fields")]
    IncompleteFen,
    #[error("BoardError: {0}")]
    BoardError(#[from] BoardError),
    #[error("Invalid FEN turn field: {0}")]
    InvalidTurn(#[from] ColorError),
    #[error("Invalid FEN halfmove clock: {0}")]
    InvalidHalfmoveClock(#[source] ParseIntError),
    #[error("Invalid FEN fullmove number: {0}")]
    InvalidFullmoveNumber(#[source] ParseIntError),
    #[error("Invalid FEN en passant square: {0}")]
    InvalidEnPassant(String),
}

#[derive(Clone, Copy, Debug)]
pub struct Undo {
    castling_rights: CastlingRights,
    en_passant: Option<Coordinate>,
    halfmove_clock: u32,
    move_info: Move,
}

#[derive(Clone)]
pub struct GameState {
    pub board: Board,
    pub turn: Color,
    pub castling_rights: CastlingRights,
    pub en_passant: Option<Coordinate>,
    halfmove_clock: u32,
    pub fullmove_number: u32,
    pub kings: (Coordinate, Coordinate),
    material_count: [u8; 10],
}

#[derive(PartialEq)]
pub enum MoveGenMode {
    All,
    CaptureOnly,
    CapturesAndChecks,
}

impl GameState {
    pub fn from_fen(fen: &str) -> Result<Self, GameStateError> {
        let mut fen = fen.split(" ");
        let fen_board = fen.next().ok_or(GameStateError::IncompleteFen)?;
        let turn: Color = fen
            .next()
            .ok_or(GameStateError::IncompleteFen)?
            .parse()
            .map_err(GameStateError::InvalidTurn)?;
        let castling_rights =
            CastlingRights::from_fen_str(fen.next().ok_or(GameStateError::IncompleteFen)?);
        let en_passant = fen.next().ok_or(GameStateError::IncompleteFen)?;
        let halfmove_clock: u32 = fen
            .next()
            .ok_or(GameStateError::IncompleteFen)?
            .parse()
            .map_err(GameStateError::InvalidHalfmoveClock)?;
        let fullmove_number: u32 = fen
            .next()
            .ok_or(GameStateError::IncompleteFen)?
            .parse()
            .map_err(GameStateError::InvalidFullmoveNumber)?;

        let board = Board::from_fen(fen_board)?;
        let en_passant = match en_passant {
            "-" => None,
            _ => Some(
                Coordinate::from_algebraic(en_passant)
                    .ok_or_else(|| GameStateError::InvalidEnPassant(en_passant.to_string()))?,
            ),
        };

        let kings = (board.find_king(Color::White), board.find_king(Color::Black));

        let mut gs = GameState {
            board,
            turn,
            castling_rights,
            en_passant,
            halfmove_clock,
            fullmove_number,
            kings,
            material_count: [0; 10],
        };
        gs.count_material();

        Ok(gs)
    }

    pub fn to_fen(&self) -> String {
        let board_fen = self.board.to_fen();
        let turn = match self.turn {
            Color::White => 'w',
            Color::Black => 'b',
        };

        let mut castle = String::new();
        if self.castling_rights.kingside_white {
            castle.push('K');
        }
        if self.castling_rights.queenside_white {
            castle.push('Q');
        }
        if self.castling_rights.kingside_black {
            castle.push('k');
        }
        if self.castling_rights.queenside_black {
            castle.push('q');
        }
        if castle.is_empty() {
            castle.push('-');
        }

        let ep = match self.en_passant {
            Some(c) => c.to_algebraic(),
            None => "-".to_string(),
        };

        format!(
            "{board_fen} {turn} {castle} {ep} {} {}",
            self.halfmove_clock, self.fullmove_number
        )
    }

    pub fn count_material(&mut self) {
        self.material_count.fill(0);
        for i in 0..64 {
            let sq = self.board.get(Coordinate::from_idx(i));
            if let Square::Occupied { color: c, piece } = sq
                && piece != PieceType::King
            {
                self.material_count[self.material_idx(piece, c)] += 1;
            }
        }
    }

    pub fn material_idx(&self, piece: PieceType, color: Color) -> usize {
        match (piece, color) {
            (PieceType::Pawn, Color::White) => 0,
            (PieceType::Pawn, Color::Black) => 1,
            (PieceType::Knight, Color::White) => 2,
            (PieceType::Knight, Color::Black) => 3,
            (PieceType::Bishop, Color::White) => 4,
            (PieceType::Bishop, Color::Black) => 5,
            (PieceType::Rook, Color::White) => 6,
            (PieceType::Rook, Color::Black) => 7,
            (PieceType::Queen, Color::White) => 8,
            (PieceType::Queen, Color::Black) => 9,
            _ => unreachable!(),
        }
    }

    pub fn add_material(&mut self, piece: PieceType, color: Color, count: u8) {
        let idx = self.material_idx(piece, color);
        self.material_count[idx] += count;
    }

    pub fn remove_material(&mut self, piece: PieceType, color: Color, count: u8) {
        let idx = self.material_idx(piece, color);
        self.material_count[idx] -= count;
    }

    pub fn is_insufficient_material(&self) -> bool {
        // Any pawn, rook, or queen => sufficient material
        let major = [PieceType::Pawn, PieceType::Rook, PieceType::Queen];

        for piece in major {
            if self.material_count[self.material_idx(piece, Color::White)] > 0
                || self.material_count[self.material_idx(piece, Color::Black)] > 0
            {
                return false;
            }
        }

        let wb = self.material_count[self.material_idx(PieceType::Bishop, Color::White)];
        let bb = self.material_count[self.material_idx(PieceType::Bishop, Color::Black)];
        let wn = self.material_count[self.material_idx(PieceType::Knight, Color::White)];
        let bn = self.material_count[self.material_idx(PieceType::Knight, Color::Black)];

        // K vs K
        if wb + wn + bb + bn == 0 {
            return true;
        }

        // K+B vs K
        if wn + bn == 0 && ((wb == 1 && bb == 0) || (bb == 1 && wb == 0)) {
            return true;
        }

        // K+N vs K
        if wb + bb == 0 && ((wn == 1 && bn == 0) || (bn == 1 && wn == 0)) {
            return true;
        }

        // K+B vs K+B
        if wb == 1 && bb == 1 && wn + bn == 0 {
            return true;
        }

        false
    }

    fn can_castle_kingside(&self, color: Color) -> bool {
        match color {
            Color::White => self.castling_rights.kingside_white,
            Color::Black => self.castling_rights.kingside_black,
        }
    }

    fn can_castle_queenside(&self, color: Color) -> bool {
        match color {
            Color::White => self.castling_rights.queenside_white,
            Color::Black => self.castling_rights.queenside_black,
        }
    }

    fn set_castle(&mut self, color: Color, kingside: bool, queenside: bool) {
        match color {
            Color::White => {
                self.castling_rights.kingside_white = kingside;
                self.castling_rights.queenside_white = queenside;
            }
            Color::Black => {
                self.castling_rights.kingside_black = kingside;
                self.castling_rights.queenside_black = queenside;
            }
        }
    }

    pub fn is_attacked(&self, coord: Coordinate, by: Color) -> bool {
        let pawn_dir: i8 = if by == Color::White { -1 } else { 1 };
        for df in [-1i8, 1i8] {
            if let Some(target) = coord.checked_offset(df, pawn_dir) {
                match self.board.get(target) {
                    Square::Occupied { color, piece }
                        if color == by && piece == PieceType::Pawn =>
                    {
                        return true;
                    }
                    _ => continue,
                }
            }
        }

        for &(df, dr) in &[
            (1, 2),
            (2, 1),
            (2, -1),
            (1, -2),
            (-1, -2),
            (-2, -1),
            (-2, 1),
            (-1, 2),
        ] {
            if let Some(target) = coord.checked_offset(df, dr) {
                match self.board.get(target) {
                    Square::Occupied { color, piece }
                        if color == by && piece == PieceType::Knight =>
                    {
                        return true;
                    }
                    _ => continue,
                }
            }
        }

        for df in -1i8..=1 {
            for dr in -1i8..=1 {
                if df == 0 && dr == 0 {
                    continue;
                }
                if let Some(target) = coord.checked_offset(df, dr) {
                    match self.board.get(target) {
                        Square::Occupied { color, piece }
                            if color == by && piece == PieceType::King =>
                        {
                            return true;
                        }
                        _ => continue,
                    }
                }
            }
        }

        let straight = [(0, 1), (1, 0), (0, -1), (-1, 0)];
        let diagonal = [(1, 1), (1, -1), (-1, 1), (-1, -1)];

        for &(df, dr) in &straight {
            let mut c = coord;
            while let Some(next) = c.checked_offset(df, dr) {
                c = next;
                let sq = self.board.get(c);
                match sq {
                    Square::Empty => continue,
                    Square::Occupied { color, piece }
                        if color == by
                            && (piece == PieceType::Rook || piece == PieceType::Queen) =>
                    {
                        return true;
                    }
                    _ => break,
                }
            }
        }

        for &(df, dr) in &diagonal {
            let mut c = coord;
            while let Some(next) = c.checked_offset(df, dr) {
                c = next;
                let sq = self.board.get(c);
                match sq {
                    Square::Empty => continue,
                    Square::Occupied { color, piece }
                        if color == by
                            && (piece == PieceType::Bishop || piece == PieceType::Queen) =>
                    {
                        return true;
                    }
                    _ => break,
                }
            }
        }

        false
    }

    pub fn in_check(&self, color: Color) -> bool {
        let king = match color {
            Color::White => self.kings.0,
            Color::Black => self.kings.1,
        };
        self.is_attacked(king, color.opponent())
    }

    fn pawn_moves(&self, from: Coordinate, color: Color, moves: &mut Vec<Move>) {
        let (dir, start_rank, promo_rank) = match color {
            Color::White => (1i8, 1u8, 7u8),
            Color::Black => (-1i8, 6u8, 0u8),
        };

        if let Some(to) = from.checked_offset(0, dir)
            && self.board.get(to) == Square::Empty
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

            if from.rank() == start_rank
                && let Some(to2) = from.checked_offset(0, 2 * dir)
                && self.board.get(to2) == Square::Empty
            {
                moves.push(Move::new(from, to2, PieceType::Pawn));
            }
        }

        for df in [-1i8, 1i8] {
            if let Some(to) = from.checked_offset(df, dir) {
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

                if self.en_passant == Some(to) {
                    moves.push(Move::new_en_passant(from, to));
                }
            }
        }
    }

    fn knight_moves(&self, from: Coordinate, color: Color, moves: &mut Vec<Move>) {
        let offsets = [
            (1, 2),
            (2, 1),
            (2, -1),
            (1, -2),
            (-1, -2),
            (-2, -1),
            (-2, 1),
            (-1, 2),
        ];
        for (df, dr) in &offsets {
            if let Some(to) = from.checked_offset(*df, *dr) {
                match self.board.get(to) {
                    Square::Occupied { color: c, .. } if c == color => continue,
                    Square::Occupied { piece, .. } => {
                        moves.push(Move::new_capture(from, to, PieceType::Knight, piece))
                    }
                    _ => moves.push(Move::new(from, to, PieceType::Knight)),
                }
            }
        }
    }

    fn sliding_moves(
        &self,
        from: Coordinate,
        color: Color,
        piece: PieceType,
        moves: &mut Vec<Move>,
    ) {
        static BISHOP_DIRS: &[(i8, i8)] = &[(1, 1), (1, -1), (-1, 1), (-1, -1)];
        static ROOK_DIRS: &[(i8, i8)] = &[(0, 1), (1, 0), (0, -1), (-1, 0)];
        static QUEEN_DIRS: &[(i8, i8)] = &[
            (0, 1),
            (1, 0),
            (0, -1),
            (-1, 0),
            (1, 1),
            (1, -1),
            (-1, 1),
            (-1, -1),
        ];

        let directions = match piece {
            PieceType::Bishop => BISHOP_DIRS,
            PieceType::Rook => ROOK_DIRS,
            PieceType::Queen => QUEEN_DIRS,
            _ => unreachable!(),
        };

        for &(df, dr) in directions {
            let mut cur = from;
            while let Some(to) = cur.checked_offset(df, dr) {
                cur = to;
                match self.board.get(cur) {
                    Square::Empty => moves.push(Move::new(from, cur, piece)),
                    Square::Occupied {
                        color: c,
                        piece: captured,
                    } if c != color => {
                        moves.push(Move::new_capture(from, cur, piece, captured));
                        break;
                    }
                    _ => break,
                }
            }
        }
    }

    fn king_moves(&self, from: Coordinate, color: Color, moves: &mut Vec<Move>) {
        for df in -1i8..=1 {
            for dr in -1i8..=1 {
                if df == 0 && dr == 0 {
                    continue;
                }
                if let Some(to) = from.checked_offset(df, dr) {
                    match self.board.get(to) {
                        Square::Occupied { color: c, .. } if c == color => continue,
                        Square::Occupied { piece, .. } => {
                            moves.push(Move::new_capture(from, to, PieceType::King, piece))
                        }
                        _ => moves.push(Move::new(from, to, PieceType::King)),
                    }
                }
            }
        }

        let rank: u8 = match color {
            Color::White => 0,
            Color::Black => 7,
        };

        if self.can_castle_kingside(color) {
            let king_sq = Coordinate::new_coordinate(4, rank);
            let between_empty = [
                Coordinate::new_coordinate(5, rank),
                Coordinate::new_coordinate(6, rank),
            ];
            if from == king_sq
                && self.board.get(Coordinate::new_coordinate(7, rank))
                    == (Square::Occupied {
                        color,
                        piece: PieceType::Rook,
                    })
                && between_empty
                    .iter()
                    .all(|&c| self.board.get(c) == Square::Empty)
                && !self.is_attacked(Coordinate::new_coordinate(4, rank), color.opponent())
                && !self.is_attacked(Coordinate::new_coordinate(5, rank), color.opponent())
                && !self.is_attacked(Coordinate::new_coordinate(6, rank), color.opponent())
            {
                moves.push(Move::new_castle(
                    from,
                    Coordinate::new_coordinate(6, rank),
                    CastleSide::Kingside,
                ));
            }
        }

        if self.can_castle_queenside(color) {
            let king_sq = Coordinate::new_coordinate(4, rank);
            let between_empty = [
                Coordinate::new_coordinate(1, rank),
                Coordinate::new_coordinate(2, rank),
                Coordinate::new_coordinate(3, rank),
            ];
            if from == king_sq
                && self.board.get(Coordinate::new_coordinate(0, rank))
                    == (Square::Occupied {
                        color,
                        piece: PieceType::Rook,
                    })
                && between_empty
                    .iter()
                    .all(|&c| self.board.get(c) == Square::Empty)
                && !self.is_attacked(Coordinate::new_coordinate(4, rank), color.opponent())
                && !self.is_attacked(Coordinate::new_coordinate(3, rank), color.opponent())
                && !self.is_attacked(Coordinate::new_coordinate(2, rank), color.opponent())
            {
                moves.push(Move::new_castle(
                    from,
                    Coordinate::new_coordinate(2, rank),
                    CastleSide::Queenside,
                ));
            }
        }
    }

    pub fn legal_moves(&mut self, move_gen_mode: MoveGenMode) -> Vec<Move> {
        let us = self.turn;
        let mut moves = Vec::with_capacity(64);

        for i in 0..64 {
            let from = Coordinate::from_idx(i);
            let sq = self.board.get(from);
            let piece = match sq {
                Square::Occupied { color, piece } if color == us => piece,
                _ => continue,
            };

            match piece {
                PieceType::Pawn => self.pawn_moves(from, us, &mut moves),
                PieceType::Knight => self.knight_moves(from, us, &mut moves),
                PieceType::King => self.king_moves(from, us, &mut moves),
                piece => self.sliding_moves(from, us, piece, &mut moves),
            }
        }

        let mut legal_moves = Vec::with_capacity(64);

        for m in moves {
            if move_gen_mode == MoveGenMode::CaptureOnly && m.captured.is_none() {
                continue;
            }

            if move_gen_mode == MoveGenMode::CapturesAndChecks
                && (m.captured.is_none() && m.promotion.is_none() && !m.en_passant)
            {
                let undo = self.make_move(m);
                if !self.in_check(us) && self.in_check(us.opponent()) {
                    legal_moves.push(m);
                }
                self.unmake_move(undo);
                continue;
            }

            let undo = self.make_move(m);
            if !self.in_check(us) {
                legal_moves.push(m);
            }
            self.unmake_move(undo);
        }

        legal_moves
    }

    fn apply_kingside_castle(&mut self, us: Color, rank: u8) {
        match us {
            Color::White => self.kings.0 = Coordinate::new_coordinate(6, 0),
            Color::Black => self.kings.1 = Coordinate::new_coordinate(6, 7),
        }
        self.board
            .set(Coordinate::new_coordinate(4, rank), Square::Empty);
        self.board
            .set(Coordinate::new_coordinate(7, rank), Square::Empty);
        
        self.board.set(
            Coordinate::new_coordinate(6, rank),
            Square::Occupied {
                color: us,
                piece: PieceType::King,
            },
        );
        self.board.set(
            Coordinate::new_coordinate(5, rank),
            Square::Occupied {
                color: us,
                piece: PieceType::Rook,
            },
        );
    }

    fn apply_queenside_castle(&mut self, us: Color, rank: u8) {
        match us {
            Color::White => self.kings.0 = Coordinate::new_coordinate(2, 0),
            Color::Black => self.kings.1 = Coordinate::new_coordinate(2, 7),
        }
        self.board
            .set(Coordinate::new_coordinate(4, rank), Square::Empty);
        self.board
            .set(Coordinate::new_coordinate(0, rank), Square::Empty);
        
        self.board.set(
            Coordinate::new_coordinate(2, rank),
            Square::Occupied {
                color: us,
                piece: PieceType::King,
            },
        );
        self.board.set(
            Coordinate::new_coordinate(3, rank),
            Square::Occupied {
                color: us,
                piece: PieceType::Rook,
            },
        );
    }

    pub fn make_move(&mut self, m: Move) -> Undo {
        let undo = Undo {
            castling_rights: self.castling_rights,
            en_passant: self.en_passant,
            halfmove_clock: self.halfmove_clock,
            move_info: m,
        };

        let us = self.turn;
        let them = us.opponent();
        self.turn = them;

        if us == Color::Black {
            self.fullmove_number += 1;
        }

        self.en_passant = None;

        if let Some(castle) = m.castle {
            let rank = match us {
                Color::White => 0,
                Color::Black => 7,
            };
            match castle {
                CastleSide::Kingside => self.apply_kingside_castle(us, rank),
                CastleSide::Queenside => self.apply_queenside_castle(us, rank),
            }

            self.set_castle(us, false, false);
            return undo;
        }

        let moving_piece = m.piece;
        let is_pawn_move: bool = matches!(moving_piece, PieceType::Pawn);

        let is_capture = match m.captured {
            Some(piece) => {
                let coord = if m.en_passant {
                    Coordinate::new_coordinate(m.to.file(), m.from.rank())
                } else {
                    m.to
                };

                self.remove_material(piece, them, 1);

                self.board.set(coord, Square::Empty);

                let rank: u8 = match them {
                    Color::White => 0,
                    Color::Black => 7,
                };
                if m.to.file() == 0 && m.to.rank() == rank {
                    match them {
                        Color::White => self.castling_rights.queenside_white = false,
                        Color::Black => self.castling_rights.queenside_black = false,
                    }
                }
                if m.to.file() == 7 && m.to.rank() == rank {
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
                self.en_passant = Some(Coordinate::new_coordinate(
                    m.from.file(),
                    (m.from.rank() as i8 + dir.signum()) as u8,
                ));
            }
        }

        if moving_piece == PieceType::King {
            self.set_castle(us, false, false);
            match us {
                Color::White => self.kings.0 = m.to,
                Color::Black => self.kings.1 = m.to,
            }
        }

        if moving_piece == PieceType::Rook {
            let rank: u8 = match us {
                Color::White => 0,
                Color::Black => 7,
            };
            if m.from == Coordinate::new_coordinate(0, rank) {
                match us {
                    Color::White => self.castling_rights.queenside_white = false,
                    Color::Black => self.castling_rights.queenside_black = false,
                }
            }
            if m.from == Coordinate::new_coordinate(7, rank) {
                match us {
                    Color::White => self.castling_rights.kingside_white = false,
                    Color::Black => self.castling_rights.kingside_black = false,
                }
            }
        }

        self.board.set(m.to, self.board.get(m.from));
        self.board.set(m.from, Square::Empty);

        if let Some(promo) = m.promotion {
            self.board.set(
                m.to,
                Square::Occupied {
                    color: us,
                    piece: promo,
                },
            );
            
            self.add_material(promo, us, 1);
            self.remove_material(moving_piece, us, 1);
        }

        if is_pawn_move || is_capture {
            self.halfmove_clock = 0;
        } else {
            self.halfmove_clock += 1;
        }

        undo
    }

    fn unapply_kingside_castle(&mut self, us: Color, rank: u8) {
        match us {
            Color::White => self.kings.0 = Coordinate::new_coordinate(4, 0),
            Color::Black => self.kings.1 = Coordinate::new_coordinate(4, 7),
        }
        self.board
            .set(Coordinate::new_coordinate(6, rank), Square::Empty);
        self.board
            .set(Coordinate::new_coordinate(5, rank), Square::Empty);
        self.board.set(
            Coordinate::new_coordinate(4, rank),
            Square::Occupied {
                color: us,
                piece: PieceType::King,
            },
        );
        self.board.set(
            Coordinate::new_coordinate(7, rank),
            Square::Occupied {
                color: us,
                piece: PieceType::Rook,
            },
        );
    }

    fn unapply_queenside_castle(&mut self, us: Color, rank: u8) {
        match us {
            Color::White => self.kings.0 = Coordinate::new_coordinate(4, 0),
            Color::Black => self.kings.1 = Coordinate::new_coordinate(4, 7),
        }
        self.board
            .set(Coordinate::new_coordinate(2, rank), Square::Empty);
        self.board
            .set(Coordinate::new_coordinate(3, rank), Square::Empty);
        self.board.set(
            Coordinate::new_coordinate(4, rank),
            Square::Occupied {
                color: us,
                piece: PieceType::King,
            },
        );
        self.board.set(
            Coordinate::new_coordinate(0, rank),
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

        if us == Color::Black {
            self.fullmove_number -= 1;
        }

        self.castling_rights = undo.castling_rights;
        self.en_passant = undo.en_passant;
        self.halfmove_clock = undo.halfmove_clock;

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

        if sq.1 == PieceType::King {
            match self.turn {
                Color::White => self.kings.0 = m.from,
                Color::Black => self.kings.1 = m.from,
            }
        }

        if let Some(piece) = m.captured {
            let coord = if m.en_passant {
                Coordinate::new_coordinate(m.to.file(), m.from.rank())
            } else {
                m.to
            };
            self.board
                .set(coord, Square::Occupied { color: them, piece });
            self.add_material(piece, them, 1);
        }
    }

    pub fn halfmove_clock(&self) -> u32 {
        self.halfmove_clock
    }
}

