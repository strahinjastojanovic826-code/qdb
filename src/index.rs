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