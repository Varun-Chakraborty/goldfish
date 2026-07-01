use std::str::FromStr;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ColorError {
    #[error("{0}")]
    CustomError(String),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Color {
    White,
    Black,
}

impl FromStr for Color {
    type Err = ColorError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "w" => Ok(Color::White),
            "b" => Ok(Color::Black),
            _ => Err(ColorError::CustomError(format!("invalid color: {}", s))),
        }
    }
}

impl Color {
    pub fn opponent(self) -> Self {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct CastlingRights {
    pub kingside_white: bool,
    pub queenside_white: bool,
    pub kingside_black: bool,
    pub queenside_black: bool,
}

impl CastlingRights {
    pub fn from_fen_str(s: &str) -> Self {
        CastlingRights {
            kingside_white: s.contains('K'),
            queenside_white: s.contains('Q'),
            kingside_black: s.contains('k'),
            queenside_black: s.contains('q'),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PieceType {
    King,
    Queen,
    Rook,
    Bishop,
    Knight,
    Pawn,
}

impl PieceType {
    pub fn to_char(self) -> char {
        match self {
            PieceType::King => 'K',
            PieceType::Queen => 'Q',
            PieceType::Rook => 'R',
            PieceType::Bishop => 'B',
            PieceType::Knight => 'N',
            PieceType::Pawn => 'P',
        }
    }

    #[inline(always)]
    pub fn value(self) -> i32 {
        match self {
            PieceType::Pawn => 100,
            PieceType::Knight => 320,
            PieceType::Bishop => 330,
            PieceType::Rook => 500,
            PieceType::Queen => 900,
            PieceType::King => 0,
        }
    }
}

#[derive(Error, Debug)]
pub enum SquareError {
    #[error("invalid square character: {0}")]
    InvalidPieceChar(char),
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Square {
    Empty,
    Occupied { color: Color, piece: PieceType },
}

impl Square {
    pub fn to_char(self) -> char {
        match self {
            Square::Empty => '.',
            Square::Occupied { color, piece } => {
                let c = match piece {
                    PieceType::King => 'K',
                    PieceType::Queen => 'Q',
                    PieceType::Rook => 'R',
                    PieceType::Bishop => 'B',
                    PieceType::Knight => 'N',
                    PieceType::Pawn => 'P',
                };
                match color {
                    Color::White => c,
                    Color::Black => c.to_ascii_lowercase(),
                }
            }
        }
    }

    pub fn from_char(c: char) -> Result<Self, SquareError> {
        match c {
            '.' => Ok(Square::Empty),
            'P' => Ok(Square::Occupied {
                color: Color::White,
                piece: PieceType::Pawn,
            }),
            'N' => Ok(Square::Occupied {
                color: Color::White,
                piece: PieceType::Knight,
            }),
            'B' => Ok(Square::Occupied {
                color: Color::White,
                piece: PieceType::Bishop,
            }),
            'R' => Ok(Square::Occupied {
                color: Color::White,
                piece: PieceType::Rook,
            }),
            'Q' => Ok(Square::Occupied {
                color: Color::White,
                piece: PieceType::Queen,
            }),
            'K' => Ok(Square::Occupied {
                color: Color::White,
                piece: PieceType::King,
            }),
            'p' => Ok(Square::Occupied {
                color: Color::Black,
                piece: PieceType::Pawn,
            }),
            'n' => Ok(Square::Occupied {
                color: Color::Black,
                piece: PieceType::Knight,
            }),
            'b' => Ok(Square::Occupied {
                color: Color::Black,
                piece: PieceType::Bishop,
            }),
            'r' => Ok(Square::Occupied {
                color: Color::Black,
                piece: PieceType::Rook,
            }),
            'q' => Ok(Square::Occupied {
                color: Color::Black,
                piece: PieceType::Queen,
            }),
            'k' => Ok(Square::Occupied {
                color: Color::Black,
                piece: PieceType::King,
            }),
            _ => Err(SquareError::InvalidPieceChar(c)),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Coordinate(u8, u8);

impl Coordinate {
    pub fn new_coordinate(f: u8, r: u8) -> Self {
        Coordinate(f, r)
    }

    pub fn idx(self) -> usize {
        (self.1 * 8 + self.0) as usize
    }

    pub fn file(self) -> u8 {
        self.0
    }

    pub fn rank(self) -> u8 {
        self.1
    }

    pub fn from_idx(idx: usize) -> Self {
        Coordinate::new_coordinate((idx % 8) as u8, (idx / 8) as u8)
    }

    pub fn from_algebraic(s: &str) -> Option<Self> {
        if s.len() != 2 {
            return None;
        }

        let mut s = s.chars();
        let file = s.next()?.to_ascii_lowercase();
        let file = match file {
            'a'..='h' => file as u8 - b'a',
            _ => return None,
        };
        let rank = s.next()?.to_digit(10)?;

        if !(1..=8).contains(&rank) {
            return None;
        }

        Some(Coordinate::new_coordinate(file, (rank - 1) as u8))
    }

    pub fn to_algebraic(self) -> String {
        let file = match self.0 {
            0 => 'a',
            1 => 'b',
            2 => 'c',
            3 => 'd',
            4 => 'e',
            5 => 'f',
            6 => 'g',
            7 => 'h',
            _ => unreachable!(),
        };
        format!("{}{}", file, self.1 + 1)
    }

    pub fn checked_offset(self, df: i8, dr: i8) -> Option<Self> {
        let f = self.0 as i8 + df;
        let r = self.1 as i8 + dr;
        if !(0..8).contains(&f) || !(0..8).contains(&r) {
            None
        } else {
            Some(Coordinate::new_coordinate(f as u8, r as u8))
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum CastleSide {
    Kingside,
    Queenside,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Move {
    pub from: Coordinate,
    pub to: Coordinate,
    pub piece: PieceType,
    pub captured: Option<PieceType>,
    pub promotion: Option<PieceType>,
    pub castle: Option<CastleSide>,
    pub en_passant: bool,
}

impl Move {
    pub fn new(from: Coordinate, to: Coordinate, piece: PieceType) -> Self {
        Move {
            from,
            to,
            piece,
            captured: None,
            promotion: None,
            castle: None,
            en_passant: false,
        }
    }

    pub fn new_capture(
        from: Coordinate,
        to: Coordinate,
        moving: PieceType,
        captured: PieceType,
    ) -> Self {
        Move {
            from,
            to,
            piece: moving,
            captured: Some(captured),
            promotion: None,
            castle: None,
            en_passant: false,
        }
    }

    pub fn new_en_passant(from: Coordinate, to: Coordinate) -> Self {
        Move {
            from,
            to,
            piece: PieceType::Pawn,
            captured: Some(PieceType::Pawn),
            promotion: None,
            castle: None,
            en_passant: true,
        }
    }

    pub fn new_castle(from: Coordinate, to: Coordinate, side: CastleSide) -> Self {
        Move {
            from,
            to,
            piece: PieceType::King,
            captured: None,
            promotion: None,
            castle: Some(side),
            en_passant: false,
        }
    }

    pub fn new_promotion(from: Coordinate, to: Coordinate, piece: PieceType) -> Self {
        Move {
            from,
            to,
            piece: PieceType::Pawn,
            captured: None,
            promotion: Some(piece),
            castle: None,
            en_passant: false,
        }
    }

    pub fn new_promotion_and_capture(
        from: Coordinate,
        to: Coordinate,
        promoting_to: PieceType,
        captured: PieceType,
    ) -> Self {
        Move {
            from,
            to,
            piece: PieceType::Pawn,
            captured: Some(captured),
            promotion: Some(promoting_to),
            castle: None,
            en_passant: false,
        }
    }

    pub fn to_algebraic(self) -> String {
        let mut s = String::new();
        s.push_str(&self.from.to_algebraic());
        s.push_str(&self.to.to_algebraic());
        if let Some(promotion) = self.promotion {
            s.push(promotion.to_char().to_ascii_lowercase());
        }
        s
    }
}
