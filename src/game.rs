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
}
