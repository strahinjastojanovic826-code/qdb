use std::path::Path;
use std::sync::Arc;
use parking_lot::RwLock;

use crate::errors::Result;
use crate::index::{QuatIndex, QuatVal};
use crate::storage::QuatStorage;

#[derive(Clone)]
pub struct QuatDb {
    storage: Arc<RwLock<QuatStorage>>,
    index: Arc<RwLock<QuatIndex>>,
}

impl QuatDb {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut storage = QuatStorage::open(path)?;
        let mut index = QuatIndex::new();

        Self::recover_wal(&mut storage)?;

        let total_chunks = storage.total_chunks()?;
        for i in 0..total_chunks {
            let offset = i * 8;
            index.insert_primary(i, offset);

            if let Ok(chunk) = storage.read_chunk(offset) {
                index.update_presence(chunk);
            }
        }

        Ok(Self {
            storage: Arc::new(RwLock::new(storage)),
            index: Arc::new(RwLock::new(index)),
        })
    }

    fn recover_wal(storage: &mut QuatStorage) -> Result<()> {
        let file_len = storage.total_chunks()? * 8;
        let wal_len = storage.total_chunks()?;

        if wal_len > file_len {
            storage.append_chunk(0)?;
        }

        storage.checkpoint_wal()?;
        Ok(())
    }

    /// Thread-safe insert
    pub fn insert(&self, chunk: u64) -> Result<u64> {
        let mut storage = self.storage.write();
        let mut index = self.index.write();

        let offset = storage.append_chunk(chunk)?;
        let key = offset / 8;

        index.insert_primary(key, offset);
        index.update_presence(chunk);

        Ok(key)
    }

    /// Fast lock-free-like parallel read using shared Read Lock
    pub fn get_by_key(&self, key: u64) -> Result<Option<u64>> {
        let index = self.index.read();
        if let Some(offset) = index.lookup(key) {
            let storage = self.storage.read();
            let chunk = storage.read_chunk(offset)?;
            Ok(Some(chunk))
        } else {
            Ok(None)
        }
    }

    pub fn range_query(&self, start_key: u64, end_key: u64) -> Result<Vec<(u64, u64)>> {
    let index = self.index.read();
    let storage = self.storage.read();
    let mut results = Vec::new();

    // Prolazimo kroz HashMap i biramo ključeve u opsegu
    for (&key, &offset) in &index.primary_map {
        if key >= start_key && key <= end_key {
            let chunk = storage.read_chunk(offset)?;
            results.push((key, chunk));
        }
    }

    Ok(results)
}

    pub fn get_single_quat(&self, key: u64, quat_idx: usize) -> Result<Option<QuatVal>> {
        if let Some(chunk) = self.get_by_key(key)? {
            let quat = QuatIndex::extract_quat(chunk, quat_idx)?;
            Ok(Some(quat))
        } else {
            Ok(None)
        }
    }

    /// Zero-copy mmap scan over parallel readers
    pub fn query_pattern(&self, pattern: u64, min_matches: u32) -> Result<Vec<u64>> {
        let storage = self.storage.read();
        let mut matches = Vec::new();

        if let Some(mmap_slice) = storage.get_mmap_slice() {
            for (idx, chunk_bytes) in mmap_slice.chunks_exact(8).enumerate() {
                let slice: [u8; 8] = chunk_bytes.try_into().unwrap();
                let chunk = u64::from_le_bytes(slice);

                if QuatIndex::match_quat_mask(chunk, pattern) >= min_matches {
                    matches.push(idx as u64);
                }
            }
        }

        Ok(matches)
    }
}

pub struct Transaction<'a> {
    db: &'a mut QuatDb,
    ops: Vec<(u64, u64)>, // (chunk, key)
}

impl<'a> Transaction<'a> {
    pub fn new(db: &'a mut QuatDb) -> Self {
        Self { db, ops: Vec::new() }
    }

    pub fn stage_insert(&mut self, chunk: u64) {
        let offset = 0; // calculated offset
        self.ops.push((chunk, offset));
    }

    pub fn commit(self) -> Result<()> {
    let mut storage = self.db.storage.write();
    let mut index = self.db.index.write();

    for (chunk, _offset) in self.ops {
        // Prepusti storage-u da odradi upis i vrati ključ/offset preko javne metode
        let key = storage.append_chunk(chunk)?; 
        let offset = storage.total_chunks()? * 8 - 8; // Primer izračunavanja offseta
        index.insert_primary(key, offset);
        index.update_presence(chunk);
    }

    // Automatska konverzija I/O greške zahvaljujući #[from] io::Error
    storage.sync_all()?;
    
    Ok(())
}
}