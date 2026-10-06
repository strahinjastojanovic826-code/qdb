use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum QuatDbError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Invalid chunk offset: {0}")]
    InvalidOffset(u64),

    #[error("Corrupted WAL entry detected")]
    CorruptedWal,

    #[error("Index out of bounds: chunk {chunk_idx}, quat {quat_idx}")]
    IndexOutOfBounds { chunk_idx: usize, quat_idx: usize },
}

pub type Result<T> = std::result::Result<T, QuatDbError>;