use thiserror::Error;

use crate::types::{Color, Coordinate, PieceType, Square, SquareError};

#[derive(Error, Debug)]
pub enum InvalidFen {
    #[error("invalid piece character: {0}")]
    InvalidPieceChar(char),
    #[error("incomplete rank")]
    IncompleteRank,
    #[error("incomplete board")]
    IncompleteBoard,
}

#[derive(Error, Debug)]
pub enum BoardError {
    #[error("Invalid FEN: {0}")]
    InvalidFen(#[from] InvalidFen),
    #[error("Square Error: {0}")]
    SquareError(#[from] SquareError),
}

#[derive(Clone, Debug)]
pub struct Board {
    squares: [Square; 64],
}

impl Board {
    pub fn from_fen(fen: &str) -> Result<Self, BoardError> {
        let mut squares = [Square::Empty; 64];
        let mut r: i8 = 7;
        let mut f: i8 = 0;
        for c in fen.chars() {
            if c == '/' {
                if f != 8 {
                    return Err(InvalidFen::IncompleteRank)?;
                }
                r -= 1;
                f = 0;
            } else if c.is_ascii_digit() {
                f += c.to_digit(10).ok_or(InvalidFen::InvalidPieceChar(c))? as i8;
            } else {
                let square = Square::from_char(c)?;
                squares[(r * 8 + f) as usize] = square;
                f += 1;
            }
        }

        if r != 0 || f != 8 {
            return Err(InvalidFen::IncompleteBoard)?;
        }
        Ok(Board { squares })
    }

    pub fn get(&self, c: Coordinate) -> Square {
        self.squares[c.idx()]
    }

    pub fn set(&mut self, c: Coordinate, sq: Square) {
        self.squares[c.idx()] = sq;
    }

    pub fn find_king(&self, color: Color) -> Coordinate {
        self.squares
            .iter()
            .enumerate()
            .find(|&(_, &sq)| {
                sq == Square::Occupied {
                    color,
                    piece: PieceType::King,
                }
            })
            .map(|(i, _)| Coordinate::from_idx(i))
            .expect("King not found")
    }

    pub fn to_fen(&self) -> String {
        let mut fen = String::new();
        for rank in (0..8).rev() {
            let mut empty = 0;
            for file in 0..8 {
                let sq = self.get(Coordinate::new(file, rank));
                if sq == Square::Empty {
                    empty += 1;
                } else {
                    if empty > 0 {
                        fen.push_str(&empty.to_string());
                        empty = 0;
                    }
                    fen.push(sq.to_char());
                }
            }
            if empty > 0 {
                fen.push_str(&empty.to_string());
            }
            if rank > 0 {
                fen.push('/');
            }
        }
        fen
    }
}
