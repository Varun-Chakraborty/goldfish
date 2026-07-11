mod eval;
mod make_unmake;
mod mobility;
mod move_gen;
mod pst;
#[cfg(test)]
mod tests;

use std::num::ParseIntError;

use crate::{
    board::{Board, BoardError},
    game::{
        Direction::{Horizontal, LeftDiagonal, Offset, RightDiagonal, Vertical},
        mobility::piece_mobility,
        pst::pst,
    },
    types::{
        CastlingRights, Color, ColorError, Coordinate, Move,
        PieceType::{self, Bishop, Queen, Rook},
        Square,
    },
    zobrist::compute_hash,
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
    pst_score: i32,
    mobility_score: i32,
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Direction {
    Offset,
    RightDiagonal,
    LeftDiagonal,
    Horizontal,
    Vertical,
}

#[derive(Debug)]
pub struct PinMap {
    pins: [Option<Direction>; 64],
}

impl PinMap {
    pub fn new() -> Self {
        PinMap { pins: [None; 64] }
    }

    pub fn get(&self, coord: Coordinate) -> Option<Direction> {
        self.pins[coord.idx()]
    }

    pub fn set(&mut self, coord: Coordinate, dir: Option<Direction>) {
        self.pins[coord.idx()] = dir;
    }
}

#[derive(Clone)]
pub struct GameState {
    pub board: Board,
    pub turn: Color,
    pub castling_rights: CastlingRights,
    pub en_passant: Option<Coordinate>,
    halfmove_clock: u32,
    pub fullmove_number: u32,
    pub wk: Coordinate,
    pub bk: Coordinate,
    pub zobrist: u64,
    pub history: Vec<u64>,
    material_count: [u8; 10],
    pub wp: [u8; 8],
    pub bp: [u8; 8],
    pub wb: u64,
    pub bb: u64,
    pub pst_score: i32,
    pub mobility_score: i32,
}

#[derive(PartialEq, Clone, Copy)]
pub enum MoveGenMode {
    All,
    CaptureOnly,
}

static QUEEN: &[(i8, i8)] = &[
    (1, 1),
    (1, -1),
    (-1, 1),
    (-1, -1),
    (1, 0),
    (0, 1),
    (-1, 0),
    (0, -1),
];
static DIAGONAL: &[(i8, i8)] = &[(1, 1), (1, -1), (-1, 1), (-1, -1)];
static RIGHT_DIAGONAL: &[(i8, i8)] = &[(1, 1), (-1, -1)];
static LEFT_DIAGONAL: &[(i8, i8)] = &[(1, -1), (-1, 1)];
static STRAIGHT: &[(i8, i8)] = &[(1, 0), (0, 1), (-1, 0), (0, -1)];
static HORIZONTAL: &[(i8, i8)] = &[(1, 0), (-1, 0)];
static VERTICAL: &[(i8, i8)] = &[(0, 1), (0, -1)];
static EMPTY: &[(i8, i8)] = &[];
static KNIGHT_OFFSETS: &[(i8, i8)] = &[
    (1, 2),
    (2, 1),
    (2, -1),
    (1, -2),
    (-1, -2),
    (-2, -1),
    (-2, 1),
    (-1, 2),
];

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

        let wk = board.find_king(Color::White);
        let bk = board.find_king(Color::Black);
        let mut gs = GameState {
            board,
            turn,
            castling_rights,
            en_passant,
            halfmove_clock,
            fullmove_number,
            wk,
            bk,
            history: Vec::new(),
            zobrist: 0,
            material_count: [0; 10],
            wp: [0; 8],
            bp: [0; 8],
            wb: 0,
            bb: 0,
            pst_score: 0,
            mobility_score: 0,
        };

        for i in 0..8 {
            for j in 0..8 {
                let coord = Coordinate::new(i, j);
                let sq = gs.board.get(coord);
                if let Square::Occupied { color, piece } = sq {
                    match (color, piece) {
                        (Color::White, PieceType::Pawn) => gs.wp[i as usize] |= 1 << j,
                        (Color::Black, PieceType::Pawn) => gs.bp[i as usize] |= 1 << j,
                        (Color::White, PieceType::Bishop) => gs.wb |= 1 << coord.idx(),
                        (Color::Black, PieceType::Bishop) => gs.bb |= 1 << coord.idx(),
                        _ => {}
                    }

                    if piece != PieceType::King {
                        gs.material_count[gs.material_idx(piece, color)] += 1;
                    }

                    gs.pst_score += pst(&piece, color, coord.idx());
                    gs.mobility_score += piece_mobility(&gs, coord);
                }
            }
        }

        let zobrist = compute_hash(&gs);
        gs.zobrist = zobrist;
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
            p => unreachable!("{p:?}\n{}", self.representation()),
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

    pub fn knight_squares<F>(&self, from: Coordinate, direction: Option<Direction>, mut callback: F)
    where
        F: FnMut(Coordinate, Square),
    {
        match direction {
            Some(Offset) | None => {}
            _ => return,
        };

        for &(df, dr) in KNIGHT_OFFSETS {
            if let Some(to) = from.checked_offset(df, dr) {
                let sq = self.board.get(to);
                callback(to, sq);
            }
        }
    }

    pub fn ray_tracing<F>(
        &self,
        from: Coordinate,
        piece: PieceType,
        direction: Option<Direction>,
        mut callback: F,
    ) where
        F: FnMut(Coordinate, Square),
    {
        let directions = match piece {
            PieceType::Bishop => match direction {
                None => DIAGONAL,
                Some(dir) => match dir {
                    RightDiagonal => RIGHT_DIAGONAL,
                    LeftDiagonal => LEFT_DIAGONAL,
                    _ => EMPTY,
                },
            },
            PieceType::Rook => match direction {
                None => STRAIGHT,
                Some(dir) => match dir {
                    Vertical => VERTICAL,
                    Horizontal => HORIZONTAL,
                    _ => EMPTY,
                },
            },
            PieceType::Queen => match direction {
                None => QUEEN,
                Some(dir) => match dir {
                    RightDiagonal => RIGHT_DIAGONAL,
                    LeftDiagonal => LEFT_DIAGONAL,
                    Horizontal => HORIZONTAL,
                    Vertical => VERTICAL,
                    _ => EMPTY,
                },
            },
            _ => unreachable!(),
        };

        for &(df, dr) in directions {
            let mut cur = from;
            while let Some(to) = cur.checked_offset(df, dr) {
                cur = to;
                let sq = self.board.get(cur);
                callback(to, sq);
                if let Square::Occupied { .. } = sq {
                    break;
                }
            }
        }
    }

    pub fn lva(
        &self,
        coord: Coordinate,
        by: Color,
        dir: Option<Direction>,
        exclusion_list: Option<&[Coordinate]>,
    ) -> Option<(Coordinate, PieceType)> {
        let pawn_dir: i8 = if by == Color::White { -1 } else { 1 };
        if dir.as_ref().is_none_or(|&dir| dir == RightDiagonal) {
            let df = pawn_dir;
            if let Some(target) = coord.checked_offset(df, pawn_dir) {
                match self.board.get(target) {
                    Square::Occupied { color, piece }
                        if color == by && piece == PieceType::Pawn =>
                    {
                        if exclusion_list.is_none_or(|e| !e.contains(&target)) {
                            return Some((target, piece));
                        }
                    }
                    _ => {}
                }
            }
        }

        if dir.as_ref().is_none_or(|&dir| dir == LeftDiagonal) {
            let df = -pawn_dir;
            if let Some(target) = coord.checked_offset(df, pawn_dir) {
                match self.board.get(target) {
                    Square::Occupied { color, piece }
                        if color == by && piece == PieceType::Pawn =>
                    {
                        if exclusion_list.is_none_or(|e| !e.contains(&target)) {
                            return Some((target, piece));
                        }
                    }
                    _ => {}
                }
            }
        }

        let mut attacker = None;

        self.knight_squares(coord, dir, |coord, sq| match sq {
            Square::Occupied { color, piece } if color == by && piece == PieceType::Knight => {
                if exclusion_list.is_none_or(|e| !e.contains(&coord)) {
                    attacker = Some((coord, piece));
                }
            }
            _ => return,
        });

        if let Some(attacker) = attacker {
            return Some(attacker);
        }

        let mut attacker = None;

        self.ray_tracing(coord, Bishop, dir, |c, sq| match sq {
            Square::Empty => return,
            Square::Occupied { color, piece } => match piece {
                PieceType::Bishop if color == by => {
                    if exclusion_list.is_none_or(|e| !e.contains(&c)) {
                        attacker = Some((c, piece));
                        return;
                    }
                }
                _ => return,
            },
        });

        if let Some(attacker) = attacker {
            return Some(attacker);
        }

        self.ray_tracing(coord, Rook, dir, |c, sq| match sq {
            Square::Empty => return,
            Square::Occupied { color, piece } => match piece {
                PieceType::Rook if color == by => {
                    if exclusion_list.is_none_or(|e| !e.contains(&c)) {
                        attacker = Some((c, piece));
                        return;
                    }
                }
                _ => return,
            },
        });

        if let Some(attacker) = attacker {
            return Some(attacker);
        }

        self.ray_tracing(coord, Queen, dir, |c, sq| match sq {
            Square::Empty => return,
            Square::Occupied { color, piece } => match piece {
                PieceType::Queen if color == by => {
                    if exclusion_list.is_none_or(|e| !e.contains(&c)) {
                        attacker = Some((c, piece));
                        return;
                    }
                }
                _ => return,
            },
        });

        if let Some(attacker) = attacker {
            return Some(attacker);
        }

        let king = match dir {
            None => QUEEN,
            Some(dir) => match dir {
                RightDiagonal => RIGHT_DIAGONAL,
                LeftDiagonal => LEFT_DIAGONAL,
                Horizontal => HORIZONTAL,
                Vertical => VERTICAL,
                _ => EMPTY,
            },
        };

        for &(df, dr) in king {
            if let Some(target) = coord.checked_offset(df, dr) {
                match self.board.get(target) {
                    Square::Occupied { color, piece }
                        if color == by && piece == PieceType::King =>
                    {
                        if exclusion_list.is_none_or(|e| !e.contains(&target)) {
                            return Some((target, piece));
                        }
                    }
                    _ => continue,
                }
            }
        }

        None
    }

    pub fn compute_pins(&self) -> PinMap {
        let king = match self.turn {
            Color::White => self.wk,
            Color::Black => self.bk,
        };
        let mut pinmap = PinMap::new();
        let mut candidate_square = None;
        for direction in [
            (-1, -1),
            (-1, 0),
            (-1, 1),
            (0, -1),
            (0, 1),
            (1, -1),
            (1, 0),
            (1, 1),
        ] {
            let mut cell = king;
            while let Some(next) = cell.checked_offset(direction.0, direction.1) {
                cell = next;
                match self.board.get(next) {
                    Square::Empty => continue,
                    Square::Occupied { color, piece } if color == self.turn => {
                        if candidate_square.is_none() {
                            candidate_square = Some(cell);
                        } else {
                            break;
                        }
                    }
                    Square::Occupied { piece, .. } => {
                        if let Some(sq) = candidate_square {
                            let diagonal = direction.0.abs() == direction.1.abs();
                            let axis = direction.0.abs() == 0 || direction.1.abs() == 0;
                            if diagonal && (piece == PieceType::Queen || piece == PieceType::Bishop)
                            {
                                pinmap.set(
                                    sq,
                                    if direction.0 == direction.1 {
                                        Some(Direction::RightDiagonal)
                                    } else {
                                        Some(Direction::LeftDiagonal)
                                    },
                                );
                            } else if axis
                                && (piece == PieceType::Queen || piece == PieceType::Rook)
                            {
                                pinmap.set(
                                    sq,
                                    if direction.0 == 0 {
                                        Some(Direction::Vertical)
                                    } else {
                                        Some(Direction::Horizontal)
                                    },
                                );
                            }
                        }
                        break;
                    }
                }
            }
            candidate_square = None;
        }

        pinmap
    }

    pub fn checks(&self, to: Color) -> [Option<(Coordinate, Direction)>; 2] {
        let king = match to {
            Color::White => self.wk,
            Color::Black => self.bk,
        };

        let directions = [
            Direction::Offset,
            Direction::Horizontal,
            Direction::Vertical,
            Direction::LeftDiagonal,
            Direction::RightDiagonal,
        ];

        let mut count = 0;
        let mut checks = [None; 2];

        for d in directions {
            if let Some((c, piece)) = self.lva(king, to.opponent(), Some(d), None) {
                checks[count] = Some((c, if piece == PieceType::Pawn { Offset } else { d }));
                if count == 2 {
                    break;
                }
                count += 1;
            }
        }

        checks
    }

    pub fn in_check(&self, color: Color) -> bool {
        self.checks(color)[0].is_some()
    }

    pub fn halfmove_clock(&self) -> u32 {
        self.halfmove_clock
    }

    pub fn representation(&self) -> String {
        let mut repr = String::new();
        for i in (0..8).rev() {
            repr += &format!("{} ", i + 1);
            for j in 0..8 {
                repr += &format!(
                    "{}",
                    match self.board.get(Coordinate::new(j, i)) {
                        Square::Empty => " . ".to_string(),
                        Square::Occupied { color, piece } => {
                            let p = match piece {
                                PieceType::Rook => " R ",
                                PieceType::Knight => " N ",
                                PieceType::Bishop => " B ",
                                PieceType::Queen => " Q ",
                                PieceType::King => " K ",
                                PieceType::Pawn => " P ",
                            };
                            if color == Color::Black {
                                p.to_lowercase()
                            } else {
                                p.to_string()
                            }
                        }
                    }
                );
            }
            repr += "\n";
        }
        repr += &format!("   a  b  c  d  e  f  g  h\n");

        repr
    }
}
