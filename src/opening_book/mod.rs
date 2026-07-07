mod parse_book;

use crate::{
    opening_book::parse_book::{BookEntry, ParseBookError, parse_book},
    types::{
        CastleSide::{Kingside, Queenside},
        Coordinate, Move,
        PieceType::{self, Bishop, Knight, Queen, Rook},
    },
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OpeningBookError {
    #[error("ParseBookError: {0}")]
    ParseBook(#[from] ParseBookError),
}

pub struct OpeningBook {
    book: Vec<BookEntry>,
}

impl OpeningBook {
    pub fn new(path: &str) -> Result<Self, OpeningBookError> {
        Ok(Self {
            book: parse_book(path)?,
        })
    }

    pub fn lookup(&self, key: u64) -> &[BookEntry] {
        let start = self.book.partition_point(|e| e.key < key);
        let end = self.book.partition_point(|e| e.key <= key);

        &self.book[start..end]
    }

    fn parse_move(&self, move_: u16) -> (Coordinate, Coordinate, Option<PieceType>) {
        let tf = (move_ & 7) as u8;
        let tr = ((move_ >> 3) & 7) as u8;
        let to = Coordinate::new(tf, tr);

        let ff = ((move_ >> 6) & 7) as u8;
        let fr = ((move_ >> 9) & 7) as u8;
        let from = Coordinate::new(ff, fr);

        let piece = ((move_ >> 12) & 7) as u8;
        let piece = match piece {
            0 => None,
            1 => Some(Knight),
            2 => Some(Bishop),
            3 => Some(Rook),
            4 => Some(Queen),
            _ => unreachable!(),
        };

        (from, to, piece)
    }

    pub fn choose(&self, key: u64, legal_moves: &[Move]) -> Option<Move> {
        let lookup = self.lookup(key);

        let moves = lookup.iter().filter_map(|e| {
            let (from, to, promotion) = self.parse_move(e.move_);
            let legal = legal_moves.iter().find(|m| {
                let from_matches = m.from == from;
                let to_matches = if let Some(side) = m.castle {
                    match side {
                        Kingside if from == Coordinate::new(4, 0) && to == Coordinate::new(7, 0) => true,
                        Queenside if from == Coordinate::new(4, 0) && to == Coordinate::new(0, 0) => true,
                        Kingside if from == Coordinate::new(4, 7) && to == Coordinate::new(7, 7) => true,
                        Queenside if from == Coordinate::new(4, 7) && to == Coordinate::new(0, 7) => true,
                        _ => false,
                    }
                } else {
                    m.to == to
                };
                let promotion_matches = m.promotion == promotion;

                from_matches && to_matches && promotion_matches
            });
            legal.map(|&m| (m, e.weight))
        }).collect::<Vec<_>>();

        let total = moves.iter().map(|(_, w)| w).sum();

        if total == 0 {
            return None;
        }

        let mut random = rand::random_range(0..total);


        for m in moves {
            if random < m.1 {
                return Some(m.0);
            }

            random -= m.1;
        }

        None
    }
}
