use std::num::ParseIntError;

use crate::{
    board::{Board, BoardError},
    game::Direction::{Horizontal, LeftDiagonal, Offset, RightDiagonal, Vertical},
    types::{CastleSide, CastlingRights, Color, ColorError, Coordinate, Move, PieceType, Square},
    zobrist::{castling_hash, compute_hash, ep_hash, piece_sq_hash, side_hash},
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
    pub kings: (Coordinate, Coordinate),
    pub zobrist: u64,
    pub history: Vec<u64>,
    material_count: [u8; 10],
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

        let kings = (board.find_king(Color::White), board.find_king(Color::Black));

        let mut gs = GameState {
            board,
            turn,
            castling_rights,
            en_passant,
            halfmove_clock,
            fullmove_number,
            kings,
            history: Vec::new(),
            zobrist: 0,
            material_count: [0; 10],
        };
        gs.count_material();

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

        if dir.as_ref().is_none_or(|&dir| dir == Offset) {
            for &(df, dr) in KNIGHT_OFFSETS {
                if let Some(target) = coord.checked_offset(df, dr) {
                    match self.board.get(target) {
                        Square::Occupied { color, piece }
                            if color == by && piece == PieceType::Knight =>
                        {
                            if exclusion_list.is_none_or(|e| !e.contains(&target)) {
                                return Some((target, piece));
                            }
                        }
                        _ => continue,
                    }
                }
            }
        }

        let bishop = match dir.as_ref() {
            Some(&dir) => match dir {
                Vertical | Horizontal | Offset => EMPTY,
                RightDiagonal => RIGHT_DIAGONAL,
                LeftDiagonal => LEFT_DIAGONAL,
            },
            None => DIAGONAL,
        };

        for &(df, dr) in bishop {
            let mut c = coord;
            while let Some(next) = c.checked_offset(df, dr) {
                c = next;
                let sq = self.board.get(c);
                match sq {
                    Square::Empty => continue,
                    Square::Occupied { color, piece } => match piece {
                        PieceType::Bishop if color == by => {
                            if exclusion_list.is_none_or(|e| !e.contains(&c)) {
                                return Some((c, piece));
                            }
                        }
                        _ => break,
                    },
                }
            }
        }

        let rook = match dir.as_ref() {
            Some(&dir) => match dir {
                RightDiagonal | LeftDiagonal | Offset => EMPTY,
                Horizontal => HORIZONTAL,
                Vertical => VERTICAL,
            },
            None => STRAIGHT,
        };

        for &(df, dr) in rook {
            let mut c = coord;
            while let Some(next) = c.checked_offset(df, dr) {
                c = next;
                let sq = self.board.get(c);
                match sq {
                    Square::Empty => continue,
                    Square::Occupied { color, piece } => match piece {
                        PieceType::Rook if color == by => {
                            if exclusion_list.is_none_or(|e| !e.contains(&c)) {
                                return Some((c, piece));
                            }
                        }
                        _ => break,
                    },
                }
            }
        }

        let queen = match dir.as_ref() {
            None => QUEEN,
            Some(dir) => match dir {
                RightDiagonal => RIGHT_DIAGONAL,
                LeftDiagonal => LEFT_DIAGONAL,
                Horizontal => HORIZONTAL,
                Vertical => VERTICAL,
                _ => EMPTY,
            },
        };

        for &(df, dr) in queen {
            let mut c = coord;
            while let Some(next) = c.checked_offset(df, dr) {
                c = next;
                let sq = self.board.get(c);
                match sq {
                    Square::Empty => continue,
                    Square::Occupied { color, piece } => match piece {
                        PieceType::Queen if color == by => {
                            if exclusion_list.is_none_or(|e| !e.contains(&c)) {
                                return Some((c, piece));
                            }
                        }
                        _ => break,
                    },
                }
            }
        }

        // treating directions as offsets
        for &(df, dr) in queen {
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
            Color::White => self.kings.0,
            Color::Black => self.kings.1,
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
            Color::White => self.kings.0,
            Color::Black => self.kings.1,
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
        pin_map: &PinMap,
        move_gen_mode: MoveGenMode,
    ) {
        let color = self.turn;
        let pinned = pin_map.get(from);
        if pinned.is_some() {
            return;
        }
        for &(df, dr) in KNIGHT_OFFSETS {
            if let Some(to) = from.checked_offset(df, dr) {
                if target_squares
                    .as_ref()
                    .is_some_and(|sqrs| !sqrs.contains(&to))
                {
                    continue;
                }
                match self.board.get(to) {
                    Square::Occupied { color: c, .. } if c == color => continue,
                    Square::Occupied { piece, .. } => {
                        moves.push(Move::new_capture(from, to, PieceType::Knight, piece))
                    }
                    _ => {
                        if move_gen_mode == MoveGenMode::All {
                            moves.push(Move::new(from, to, PieceType::Knight))
                        }
                    }
                }
            }
        }
    }

    fn sliding_moves(
        &self,
        from: Coordinate,
        piece: PieceType,
        moves: &mut Vec<Move>,
        target_squares: &Option<Vec<Coordinate>>,
        pin_map: &PinMap,
        move_gen_mode: MoveGenMode,
    ) {
        let color = self.turn;
        let pinned = pin_map.get(from);
        let directions = match piece {
            PieceType::Bishop => match pinned {
                None => DIAGONAL,
                Some(dir) => match dir {
                    RightDiagonal => RIGHT_DIAGONAL,
                    LeftDiagonal => LEFT_DIAGONAL,
                    _ => EMPTY,
                },
            },
            PieceType::Rook => match pinned {
                None => STRAIGHT,
                Some(dir) => match dir {
                    Vertical => VERTICAL,
                    Horizontal => HORIZONTAL,
                    _ => EMPTY,
                },
            },
            PieceType::Queen => match pinned {
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
                match self.board.get(cur) {
                    Square::Occupied { color: c, .. } if c == color => {
                        break;
                    }
                    sq => {
                        if target_squares
                            .as_ref()
                            .is_some_and(|sqrs| !sqrs.contains(&to))
                        {
                            if sq != Square::Empty {
                                break;
                            }
                            continue;
                        }
                        match sq {
                            Square::Empty => {
                                if move_gen_mode == MoveGenMode::All {
                                    moves.push(Move::new(from, cur, piece))
                                }
                            }
                            Square::Occupied {
                                piece: captured, ..
                            } => {
                                moves.push(Move::new_capture(from, cur, piece, captured));
                                break;
                            }
                        }
                    }
                }
            }
        }
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
                Color::White => self.kings.0,
                Color::Black => self.kings.1,
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
                        Color::White => self.kings.0,
                        Color::Black => self.kings.1,
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
                    &pin_map,
                    move_gen_mode,
                ),
                piece => self.sliding_moves(
                    from,
                    piece,
                    &mut legal_moves,
                    &target_squares,
                    &pin_map,
                    move_gen_mode,
                ),
            }
        }

        legal_moves
    }

    fn apply_kingside_castle(&mut self, us: Color, rank: u8) {
        match us {
            Color::White => self.kings.0 = Coordinate::new(6, 0),
            Color::Black => self.kings.1 = Coordinate::new(6, 7),
        }
        self.board.set(Coordinate::new(4, rank), Square::Empty);
        self.zobrist ^= piece_sq_hash(PieceType::King, us, Coordinate::new(4, rank));
        self.board.set(Coordinate::new(7, rank), Square::Empty);
        self.zobrist ^= piece_sq_hash(PieceType::Rook, us, Coordinate::new(7, rank));

        self.board.set(
            Coordinate::new(6, rank),
            Square::Occupied {
                color: us,
                piece: PieceType::King,
            },
        );
        self.zobrist ^= piece_sq_hash(PieceType::King, us, Coordinate::new(6, rank));
        self.board.set(
            Coordinate::new(5, rank),
            Square::Occupied {
                color: us,
                piece: PieceType::Rook,
            },
        );
        self.zobrist ^= piece_sq_hash(PieceType::Rook, us, Coordinate::new(5, rank));
    }

    fn apply_queenside_castle(&mut self, us: Color, rank: u8) {
        match us {
            Color::White => self.kings.0 = Coordinate::new(2, 0),
            Color::Black => self.kings.1 = Coordinate::new(2, 7),
        }
        self.board.set(Coordinate::new(4, rank), Square::Empty);
        self.zobrist ^= piece_sq_hash(PieceType::King, us, Coordinate::new(4, rank));
        self.board.set(Coordinate::new(0, rank), Square::Empty);
        self.zobrist ^= piece_sq_hash(PieceType::Rook, us, Coordinate::new(0, rank));

        self.board.set(
            Coordinate::new(2, rank),
            Square::Occupied {
                color: us,
                piece: PieceType::King,
            },
        );
        self.zobrist ^= piece_sq_hash(PieceType::King, us, Coordinate::new(2, rank));
        self.board.set(
            Coordinate::new(3, rank),
            Square::Occupied {
                color: us,
                piece: PieceType::Rook,
            },
        );
        self.zobrist ^= piece_sq_hash(PieceType::Rook, us, Coordinate::new(3, rank));
    }

    pub fn make_move(&mut self, m: Move) -> Undo {
        let undo = Undo {
            castling_rights: self.castling_rights,
            en_passant: self.en_passant,
            halfmove_clock: self.halfmove_clock,
            move_info: m,
        };
        self.history.push(self.zobrist);
        self.zobrist ^= side_hash();

        let us = self.turn;
        let them = us.opponent();
        self.turn = them;

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
                CastleSide::Kingside => self.apply_kingside_castle(us, rank),
                CastleSide::Queenside => self.apply_queenside_castle(us, rank),
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

        let moving_piece = m.piece;
        let is_pawn_move: bool = matches!(moving_piece, PieceType::Pawn);

        let is_capture = match m.captured {
            Some(piece) => {
                let coord = if m.en_passant {
                    Coordinate::new(m.to.file(), m.from.rank())
                } else {
                    m.to
                };
                self.zobrist ^= piece_sq_hash(piece, them, coord);
                self.remove_material(piece, them, 1);

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
                Color::White => self.kings.0 = m.to,
                Color::Black => self.kings.1 = m.to,
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

        self.board.set(m.to, self.board.get(m.from));
        self.board.set(m.from, Square::Empty);
        self.zobrist ^= piece_sq_hash(moving_piece, us, m.from);

        if let Some(promo) = m.promotion {
            self.board.set(
                m.to,
                Square::Occupied {
                    color: us,
                    piece: promo,
                },
            );
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

        undo
    }

    fn unapply_kingside_castle(&mut self, us: Color, rank: u8) {
        match us {
            Color::White => self.kings.0 = Coordinate::new(4, 0),
            Color::Black => self.kings.1 = Coordinate::new(4, 7),
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
            Color::White => self.kings.0 = Coordinate::new(4, 0),
            Color::Black => self.kings.1 = Coordinate::new(4, 7),
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
                Coordinate::new(m.to.file(), m.from.rank())
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

#[cfg(test)]
mod tests {
    use crate::{
        game::{
            Direction::{Horizontal, LeftDiagonal, RightDiagonal, Vertical},
            GameState,
            MoveGenMode::All,
        },
        notation::parse_algebraic,
        types::{
            Color::{Black, White},
            Coordinate, Move,
            PieceType::{Bishop, King, Knight, Pawn, Queen, Rook},
        },
    };

    #[test]
    fn test_fen() {
        let mut gs =
            GameState::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
                .unwrap();
        assert_eq!(
            gs.to_fen(),
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
        );
        let m = parse_algebraic(&mut gs, "e4").unwrap();
        gs.make_move(m);
        assert_eq!(
            gs.to_fen(),
            "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1"
        );
    }

    #[test]
    fn test_legal_moves() {
        let mut gs =
            GameState::from_fen("4k2r/1p3ppp/p7/2bPp3/5B2/5P2/PP4PP/R2nRrK1 w - - 10 38").unwrap();
        let moves = gs.legal_moves(All);
        assert!(moves.contains(&Move::new_capture(
            Coordinate::new(6, 0),
            Coordinate::new(5, 0),
            King,
            Rook
        )));
    }

    #[test]
    fn test_from_fen() {
        let gs = GameState::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
            .unwrap();
        assert_eq!(
            gs.to_fen(),
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
        );
    }

    #[test]
    fn test_unmake_move_kingside_castle() {
        let mut gs =
            GameState::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQK2R w KQkq - 0 1").unwrap();
        let m = parse_algebraic(&mut gs, "o-o").unwrap();
        let material_count = gs.material_count;
        let undo = gs.make_move(m);
        assert_eq!(
            gs.to_fen(),
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQ1RK1 b kq - 0 1"
        );
        gs.unmake_move(undo);
        assert_eq!(
            gs.to_fen(),
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQK2R w KQkq - 0 1"
        );
        assert_eq!(gs.material_count, material_count);
    }

    #[test]
    fn test_unmake_move_queenside_castle() {
        let mut gs =
            GameState::from_fen("r3kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1").unwrap();
        let m = parse_algebraic(&mut gs, "o-o-o").unwrap();
        let material_count = gs.material_count;
        let undo = gs.make_move(m);
        assert_eq!(
            gs.to_fen(),
            "2kr1bnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQ - 0 2"
        );
        gs.unmake_move(undo);
        assert_eq!(
            gs.to_fen(),
            "r3kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1"
        );
        assert_eq!(gs.material_count, material_count);
    }

    #[test]
    fn test_unmake_move_rook_move_kingside() {
        let mut gs =
            GameState::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQK2R w KQkq - 0 1").unwrap();
        let m = parse_algebraic(&mut gs, "Rg1").unwrap();
        let material_count = gs.material_count;
        let undo = gs.make_move(m);
        assert_eq!(
            gs.to_fen(),
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQK1R1 b Qkq - 1 1"
        );
        gs.unmake_move(undo);
        assert_eq!(
            gs.to_fen(),
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQK2R w KQkq - 0 1"
        );
        assert_eq!(gs.material_count, material_count);
    }

    #[test]
    fn test_unmake_move_rook_move_queenside() {
        let mut gs =
            GameState::from_fen("r3kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1").unwrap();
        let m = parse_algebraic(&mut gs, "Rb8").unwrap();
        let material_count = gs.material_count;
        let undo = gs.make_move(m);
        assert_eq!(
            gs.to_fen(),
            "1r2kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQk - 1 2"
        );
        gs.unmake_move(undo);
        assert_eq!(
            gs.to_fen(),
            "r3kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1"
        );
        assert_eq!(gs.material_count, material_count);
    }

    #[test]
    fn test_unmake_move_capture() {
        let mut gs =
            GameState::from_fen("rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 1")
                .unwrap();
        let m = parse_algebraic(&mut gs, "exd5").unwrap();
        let material_count = gs.material_count;
        let undo = gs.make_move(m);
        assert_eq!(
            gs.to_fen(),
            "rnbqkbnr/ppp1pppp/8/3P4/8/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1"
        );
        gs.unmake_move(undo);
        assert_eq!(
            gs.to_fen(),
            "rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 1"
        );
        assert_eq!(gs.material_count, material_count);
    }

    #[test]
    fn test_unmake_move_promotion() {
        let mut gs = GameState::from_fen("8/7P/8/8/8/8/8/K1k5 w - - 0 1").unwrap();
        let m = parse_algebraic(&mut gs, "h8=Q").unwrap();
        let material_count = gs.material_count;
        let undo = gs.make_move(m);
        assert_eq!(gs.to_fen(), "7Q/8/8/8/8/8/8/K1k5 b - - 0 1");
        gs.unmake_move(undo);
        assert_eq!(gs.to_fen(), "8/7P/8/8/8/8/8/K1k5 w - - 0 1");
        assert_eq!(gs.material_count, material_count);
    }

    #[test]
    fn test_unmake_move_promotion_capture() {
        let mut gs = GameState::from_fen("6r1/7P/8/8/8/8/8/K1k5 w - - 0 1").unwrap();
        let m = parse_algebraic(&mut gs, "hxg8=Q").unwrap();
        let material_count = gs.material_count;
        let undo = gs.make_move(m);
        assert_eq!(gs.to_fen(), "6Q1/8/8/8/8/8/8/K1k5 b - - 0 1");
        gs.unmake_move(undo);
        assert_eq!(gs.to_fen(), "6r1/7P/8/8/8/8/8/K1k5 w - - 0 1");
        assert_eq!(gs.material_count, material_count);
    }

    #[test]
    fn test_unmake_move_enpassant() {
        let mut gs =
            GameState::from_fen("rnbqkbnr/ppp1pppp/8/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 1")
                .unwrap();
        let m = parse_algebraic(&mut gs, "exd6").unwrap();
        let material_count = gs.material_count;
        let undo = gs.make_move(m);
        assert_eq!(
            gs.to_fen(),
            "rnbqkbnr/ppp1pppp/3P4/8/8/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1"
        );
        gs.unmake_move(undo);
        assert_eq!(
            gs.to_fen(),
            "rnbqkbnr/ppp1pppp/8/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 1"
        );
        assert_eq!(gs.material_count, material_count);
    }

    #[test]
    fn test_count_material_capture_promotion() {
        let mut gs = GameState::from_fen("7r/6P1/8/8/8/8/8/k1K5 w KQkq - 0 1").unwrap();
        let m = parse_algebraic(&mut gs, "gxh8=Q+").unwrap();
        let undo = gs.make_move(m);
        let material_count = gs.material_count;
        assert_eq!(material_count[gs.material_idx(Queen, White)], 1);
        assert_eq!(material_count[gs.material_idx(Queen, Black)], 0);
        assert_eq!(material_count[gs.material_idx(Rook, White)], 0);
        assert_eq!(material_count[gs.material_idx(Rook, Black)], 0);
        assert_eq!(material_count[gs.material_idx(Bishop, White)], 0);
        assert_eq!(material_count[gs.material_idx(Bishop, Black)], 0);
        assert_eq!(material_count[gs.material_idx(Knight, White)], 0);
        assert_eq!(material_count[gs.material_idx(Knight, Black)], 0);
        assert_eq!(material_count[gs.material_idx(Pawn, Black)], 0);
        assert_eq!(material_count[gs.material_idx(Pawn, White)], 0);
        gs.unmake_move(undo);

        let material_count = gs.material_count;
        assert_eq!(material_count[gs.material_idx(Queen, White)], 0);
        assert_eq!(material_count[gs.material_idx(Queen, Black)], 0);
        assert_eq!(material_count[gs.material_idx(Rook, White)], 0);
        assert_eq!(material_count[gs.material_idx(Rook, Black)], 1);
        assert_eq!(material_count[gs.material_idx(Bishop, White)], 0);
        assert_eq!(material_count[gs.material_idx(Bishop, Black)], 0);
        assert_eq!(material_count[gs.material_idx(Knight, White)], 0);
        assert_eq!(material_count[gs.material_idx(Knight, Black)], 0);
        assert_eq!(material_count[gs.material_idx(Pawn, White)], 1);
        assert_eq!(material_count[gs.material_idx(Pawn, Black)], 0);
    }

    #[test]
    fn test_count_material_en_passant() {
        let mut gs = GameState::from_fen("8/8/8/3pP3/8/8/8/k1K5 w KQkq d6 0 1").unwrap();
        let m = parse_algebraic(&mut gs, "exd6").unwrap();
        let undo = gs.make_move(m);
        let material_count = gs.material_count;
        assert_eq!(material_count[gs.material_idx(Queen, White)], 0);
        assert_eq!(material_count[gs.material_idx(Queen, Black)], 0);
        assert_eq!(material_count[gs.material_idx(Rook, White)], 0);
        assert_eq!(material_count[gs.material_idx(Rook, Black)], 0);
        assert_eq!(material_count[gs.material_idx(Bishop, White)], 0);
        assert_eq!(material_count[gs.material_idx(Bishop, Black)], 0);
        assert_eq!(material_count[gs.material_idx(Knight, White)], 0);
        assert_eq!(material_count[gs.material_idx(Knight, Black)], 0);
        assert_eq!(material_count[gs.material_idx(Pawn, Black)], 0);
        assert_eq!(material_count[gs.material_idx(Pawn, White)], 1);
        gs.unmake_move(undo);

        let material_count = gs.material_count;
        assert_eq!(material_count[gs.material_idx(Queen, White)], 0);
        assert_eq!(material_count[gs.material_idx(Queen, Black)], 0);
        assert_eq!(material_count[gs.material_idx(Rook, White)], 0);
        assert_eq!(material_count[gs.material_idx(Rook, Black)], 0);
        assert_eq!(material_count[gs.material_idx(Bishop, White)], 0);
        assert_eq!(material_count[gs.material_idx(Bishop, Black)], 0);
        assert_eq!(material_count[gs.material_idx(Knight, White)], 0);
        assert_eq!(material_count[gs.material_idx(Knight, Black)], 0);
        assert_eq!(material_count[gs.material_idx(Pawn, White)], 1);
        assert_eq!(material_count[gs.material_idx(Pawn, Black)], 1);
    }

    #[test]
    fn test_pinned_direction1() {
        let gs = GameState::from_fen("7k/8/8/4r3/8/8/8/QK6 b - - 0 1").unwrap();
        let pin_map = gs.compute_pins();
        let pinned = pin_map.get(Coordinate::new(4, 4));
        println!("{:?}", pinned);
        assert!(pinned.is_some_and(|dir| dir == RightDiagonal));
    }

    #[test]
    fn test_pinned_direction2() {
        let gs = GameState::from_fen("k7/8/8/3r4/8/8/8/6KQ b - - 0 1").unwrap();
        let pin_map = gs.compute_pins();
        let pinned = pin_map.get(Coordinate::new(3, 4));
        println!("{:?}", pinned);
        assert!(pinned.is_some_and(|dir| dir == LeftDiagonal));
    }

    #[test]
    fn test_pinned_direction3() {
        let gs = GameState::from_fen("7k/8/8/7r/8/8/8/6KQ b - - 0 1").unwrap();
        let pin_map = gs.compute_pins();
        let pinned = pin_map.get(Coordinate::new(7, 4));
        println!("{:?}", pinned);
        assert!(pinned.is_some_and(|dir| dir == Vertical));
    }

    #[test]
    fn test_pinned_direction4() {
        let gs = GameState::from_fen("KQ3r1k/8/8/8/8/8/8/8 b - - 0 1").unwrap();
        let pin_map = gs.compute_pins();
        let pinned = pin_map.get(Coordinate::new(5, 7));
        println!("{:?}", pinned);
        assert!(pinned.is_some_and(|dir| dir == Horizontal));
    }
}
