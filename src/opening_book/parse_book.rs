use std::array::TryFromSliceError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseBookError {
    #[error("IO error: {0}")]
    IOError(#[from] std::io::Error),

    #[error("TryFromSliceError: {0}")]
    TryFromSlice(#[from] TryFromSliceError),

    #[error("Invalid book length")]
    InvalidLength,
}

#[derive(Debug, Clone, Copy)]
pub struct BookEntry {
    pub key: u64,
    pub move_: u16,
    pub weight: u16,
    #[allow(unused)]
    pub learn: u32,
}

pub fn parse_book(path: &str) -> Result<Vec<BookEntry>, ParseBookError> {
    let bytes = std::fs::read(path)?;
    if bytes.len() % 16 != 0 {
        return Err(ParseBookError::InvalidLength);
    }

    let len = bytes.len() / 16;
    let mut book = Vec::with_capacity(len);

    for chunk in bytes.chunks_exact(16) {
        let key = u64::from_be_bytes(chunk[0..8].try_into()?);
        let move_ = u16::from_be_bytes(chunk[8..10].try_into()?);
        let weight = u16::from_be_bytes(chunk[10..12].try_into()?);
        let learn = u32::from_be_bytes(chunk[12..16].try_into()?);

        let entry = BookEntry {
            key,
            move_,
            weight,
            learn,
        };
        book.push(entry);
    }

    Ok(book)
}
