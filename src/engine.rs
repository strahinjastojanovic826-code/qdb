use crate::index::{QuatIndex, QuatVal};
use crate::storage::QuatStorage;
use std::io;
use std::path::Path;

pub struct QuatDb {
    storage: QuatStorage,
    index: QuatIndex,
}

impl QuatDb {
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let mut storage = QuatStorage::open(path)?;
        let mut index = QuatIndex::new();

        // Rekonstrukcija primarnog indeksa u memoriji pri pokretanju
        let total_chunks = storage.total_chunks()?;
        for i in 0..total_chunks {
            index.insert_primary(i, i * 8);
        }

        Ok(Self { storage, index })
    }

    pub fn insert(&mut self, chunk: u64) -> io::Result<u64> {
        let offset = self.storage.append_chunk(chunk)?;
        let key = offset / 8;
        self.index.insert_primary(key, offset);
        Ok(key)
    }

    pub fn get_by_key(&mut self, key: u64) -> io::Result<Option<u64>> {
        if let Some(offset) = self.index.lookup(key) {
            let chunk = self.storage.read_chunk(offset)?;
            Ok(Some(chunk))
        } else {
            Ok(None)
        }
    }

    pub fn get_single_quat(&mut self, key: u64, quat_idx: usize) -> io::Result<Option<QuatVal>> {
        if let Some(chunk) = self.get_by_key(key)? {
            Ok(Some(QuatIndex::extract_quat(chunk, quat_idx)))
        } else {
            Ok(None)
        }
    }

    pub fn query_pattern(&mut self, pattern: u64, min_matches: u32) -> io::Result<Vec<u64>> {
        let total = self.storage.total_chunks()?;
        let mut matches = Vec::new();

        let batch_size = 512; // Čitanje u stranicama po 4KB
        let mut current = 0;

        while current < total {
            let count = std::cmp::min(batch_size as u64, total - current) as usize;
            let chunks = self.storage.read_batch(current * 8, count)?;

            for (idx, &chunk) in chunks.iter().enumerate() {
                if QuatIndex::match_quat_mask(chunk, pattern) >= min_matches {
                    matches.push((current + idx as u64) * 8);
                }
            }

            current += count as u64;
        }

        Ok(matches)
    }
}