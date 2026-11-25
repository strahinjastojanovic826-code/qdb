use std::collections::HashMap;
use crate::errors::{QuatDbError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuatVal {
    Q0 = 0,
    Q1 = 1,
    Q2 = 2,
    Q3 = 3,
}

pub struct QuatIndex {
    pub primary_map: HashMap<u64, u64>,
    pub presence_filter: Vec<u8>,
}

impl QuatIndex {
    pub fn new() -> Self {
        Self {
            primary_map: HashMap::new(),
            presence_filter: Vec::new(),
        }
    }

    pub fn insert_primary(&mut self, key: u64, offset: u64) {
        self.primary_map.insert(key, offset);
    }

    pub fn lookup(&self, key: u64) -> Option<u64> {
        self.primary_map.get(&key).copied()
    }

    pub fn update_presence(&mut self, chunk: u64) {
        let mut mask = 0u8;
        for i in 0..32 {
            let val = (chunk >> (i * 2)) & 0b11;
            mask |= 1 << val;
        }
        self.presence_filter.push(mask);
    }

    pub fn match_quat_mask_optimized(chunks: &[u64], pattern: u64, results: &mut [u32]) {
    // LLVM automatski vektorizuje ovu petlju ako je napisana linearno i bez nuspojava
    for (i, &chunk) in chunks.iter().enumerate() {
        let xnor = !(chunk ^ pattern);
        let even_bits = xnor & 0x5555_5555_5555_5555;
        let (odd_bits, _) = xnor.overflowing_shr(1);
        let combined = even_bits & (odd_bits & 0x5555_5555_5555_5555);
        results[i] = combined.count_ones();
    }
}

    #[inline(always)]
    pub fn has_state(&self, chunk_idx: usize, val: QuatVal) -> bool {
        if let Some(&mask) = self.presence_filter.get(chunk_idx) {
            (mask & (1 << (val as u8))) != 0
        } else {
            true
        }
    }

    #[inline(always)]
    pub fn match_quat_mask(chunk: u64, pattern: u64) -> u32 {
        let xnor = !(chunk ^ pattern);
        let even_bits = xnor & 0x5555_5555_5555_5555;
        let odd_bits = (xnor >> 1) & 0x5555_5555_5555_5555;
        (even_bits & odd_bits).count_ones()
    }

    #[inline(always)]
    pub fn extract_quat(chunk: u64, quat_idx: usize) -> Result<QuatVal> {
        if quat_idx >= 32 {
            return Err(QuatDbError::IndexOutOfBounds {
                chunk_idx: 0,
                quat_idx,
            });
        }

        let shift = quat_idx * 2;
        let bits = (chunk >> shift) & 0b11;

        match bits {
            0 => Ok(QuatVal::Q0),
            1 => Ok(QuatVal::Q1),
            2 => Ok(QuatVal::Q2),
            3 => Ok(QuatVal::Q3),
            _ => unreachable!(),
        }
    }

    /// Bitwise quaternary AND across all 32 quats in two packed u64 chunks.
    /// Produces `0b11` (Q3) at quat `i` iff both `a` and `b` have `0b11` at quat `i`.
    #[inline(always)]
    pub fn quat_and(a: u64, b: u64) -> u64 {
        let a_lo = a & 0x5555_5555_5555_5555;
        let a_hi = (a >> 1) & 0x5555_5555_5555_5555;
        let b_lo = b & 0x5555_5555_5555_5555;
        let b_hi = (b >> 1) & 0x5555_5555_5555_5555;

        let out_lo = a_lo & b_lo;
        let out_hi = a_hi & b_hi;
        (out_hi << 1) | out_lo
    }

    /// Bitwise quaternary OR across all 32 quats in two packed u64 chunks.
    /// Combines bit planes so any set bit in `a` or `b` is preserved per lane.
    #[inline(always)]
    pub fn quat_or(a: u64, b: u64) -> u64 {
        a | b
    }

    /// Bitwise quaternary XOR across all 32 quats in two packed u64 chunks.
    /// Useful for detecting differing quaternary states at each position.
    #[inline(always)]
    pub fn quat_xor(a: u64, b: u64) -> u64 {
        a ^ b
    }

    /// Searches a single u64 chunk (32 quats) using per-position allowed-state bitmasks.
    /// `allowed_per_pos[pos]` is a 4-bit mask where bit `q` (0..=3) means `QuatVal` `q` is allowed.
    /// Returns a 32-bit hit mask where bit `pos` is set if all positions in `[pos..pos + pattern_len)` match.
    pub fn find_wildcard_pattern(chunk: u64, allowed_per_pos: &[u8], pattern_len: usize) -> u32 {
        if pattern_len == 0 || pattern_len > 32 || allowed_per_pos.len() < pattern_len {
            return 0;
        }

        let mut hit_mask: u32 = 0;
        let max_start = 32 - pattern_len;

        for start in 0..=max_start {
            let mut matched = true;
            for offset in 0..pattern_len {
                let pos = start + offset;
                let quat = ((chunk >> (pos * 2)) & 0b11) as u8;
                let allowed = allowed_per_pos[offset];
                if (allowed & (1u8 << quat)) == 0 {
                    matched = false;
                    break;
                }
            }
            if matched {
                hit_mask |= 1u32 << start;
            }
        }

        hit_mask
    }

    /// Searches for an exact sub-sequence of `sub_len` quats (1..=16) crossing from `chunk_a` into `chunk_b`.
    /// `target_packed` holds the `sub_len` quats packed in the lowest `2 * sub_len` bits.
    /// Returns the starting quat index in `chunk_a` (0..32) if found across the boundary.
    pub fn find_cross_chunk_match(
        chunk_a: u64,
        chunk_b: u64,
        target_packed: u64,
        sub_len: usize,
    ) -> Option<usize> {
        if sub_len == 0 || sub_len > 16 {
            return None;
        }

        let combined: u128 = ((chunk_b as u128) << 64) | (chunk_a as u128);
        let lane_mask: u128 = if sub_len == 64 {
            u128::MAX
        } else {
            (1u128 << (sub_len * 2)) - 1
        };
        let target: u128 = (target_packed as u128) & lane_mask;

        // Only check starts that actually cross the 32-quat boundary of `chunk_a`
        let min_start = 32usize.saturating_sub(sub_len - 1);
        for start in min_start..32 {
            let shift = start * 2;
            let window = (combined >> shift) & lane_mask;
            if window == target {
                return Some(start);
            }
        }

        None
    }
    
}

use std::fs::File;

const BTREE_ORDER: usize = 64;

#[derive(Clone, Debug)]
pub struct BTreeNode {
    pub is_leaf: bool,
    pub num_keys: u16,
    pub keys: [u64; BTREE_ORDER],
    pub values: [u64; BTREE_ORDER], // Offsets in primary storage or child node offsets
    pub children: [u64; BTREE_ORDER + 1],
}

pub struct DiskBTreeIndex {
    file: File,
    root_offset: u64,
}

impl DiskBTreeIndex {
    pub fn new(file: File) -> Self {
        Self { file, root_offset: 0 }
    }

    pub fn search(&mut self, _key: u64) -> Result<Option<u64>> {
        // Implement disk-based node traversal here
        Ok(None)
    }

    pub fn search_disk(&mut self, _key: u64) -> Result<Option<u64>> {
        let current_offset = self.root_offset;
        
        if current_offset == 0 && self.file.metadata()?.len() == 0 {
            return Ok(None);
        }

        // Simplified traversal loop for B-Tree on disk
        // In a full implementation, you read BTreeNode from `self.file` at `current_offset`
        Ok(None)
    }

    pub fn insert_disk(&mut self, _key: u64, _value: u64) -> Result<()> {
        // Disk-based split and insert logic goes here
        Ok(())
    }

}