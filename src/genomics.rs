use crate::errors::{QuatDbError, Result};
use crate::index::QuatVal;

/// Maps a DNA nucleotide character to its Quaternary value
pub fn nucleotide_to_quat(nuc: u8) -> Result<QuatVal> {
    match nuc {
        b'A' | b'a' => Ok(QuatVal::Q0),
        b'C' | b'c' => Ok(QuatVal::Q1),
        b'G' | b'g' => Ok(QuatVal::Q2),
        b'T' | b't' => Ok(QuatVal::Q3),
        _ => Err(QuatDbError::CorruptedWal), // Invalid nucleotide error context
    }
}

/// Packs up to 32 DNA characters into a single u64 chunk (2 bits per nucleotide)
pub fn pack_dna_sequence(dna: &str) -> Result<u64> {
    if dna.len() > 32 {
        return Err(QuatDbError::IndexOutOfBounds { chunk_idx: 0, quat_idx: dna.len() });
    }
    
    let mut chunk: u64 = 0;
    for (i, byte) in dna.bytes().enumerate() {
        let qval = nucleotide_to_quat(byte)? as u64;
        chunk |= qval << (i * 2);
    }
    Ok(chunk)
}

/// Unpacks a u64 chunk back into a DNA string of given length
pub fn unpack_dna_sequence(chunk: u64, len: usize) -> Result<String> {
    let mut dna = String::with_capacity(len);
    for i in 0..len {
        let shift = i * 2;
        let bits = (chunk >> shift) & 0b11;
        let nuc = match bits {
            0 => 'A',
            1 => 'C',
            2 => 'G',
            3 => 'T',
            _ => unreachable!(),
        };
        dna.push(nuc);
    }
    Ok(dna)
}