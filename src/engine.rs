use std::path::Path;
use std::sync::Arc;
use parking_lot::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::errors::Result;
use crate::index::{QuatIndex, QuatVal};
use crate::storage::QuatStorage;

#[derive(Clone)]
pub struct QuatDb {
    pub storage: Arc<RwLock<QuatStorage>>,
    pub index: Arc<RwLock<QuatIndex>>,
    global_version: Arc<AtomicU64>,
}

#[derive(Debug, Default)]
pub struct DbMetrics {
    pub inserts_total: AtomicU64,
    pub reads_total: AtomicU64,
    pub pattern_queries_total: AtomicU64,
    pub mvcc_commits_total: AtomicU64,
    pub mvcc_conflicts_total: AtomicU64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DbMetricsSnapshot {
    pub inserts_total: u64,
    pub reads_total: u64,
    pub pattern_queries_total: u64,
    pub mvcc_commits_total: u64,
    pub mvcc_conflicts_total: u64,
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
            global_version: Arc::new(std::sync::atomic::AtomicU64::new(0)),
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

    pub fn begin_transaction(&mut self) -> Transaction<'_> {
        let current_version = self.global_version.load(Ordering::Acquire);
        Transaction {
            db: self,
            read_version: current_version,
            ops: Vec::new(),
        }
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

    /// Batch pattern scan over memory-mapped chunks.
    /// Evaluates multiple `u64` patterns in a single pass over the storage slice,
    /// returning a matching `Vec<Vec<u64>>` of chunk indices per pattern.
    pub fn query_pattern_batch(
        &self,
        patterns: &[u64],
        min_matches: u32,
    ) -> Result<Vec<Vec<u64>>> {
        let mut results: Vec<Vec<u64>> = vec![Vec::new(); patterns.len()];
        if patterns.is_empty() {
            return Ok(results);
        }

        let storage = self.storage.read();
        let Some(mmap_slice) = storage.get_mmap_slice() else {
            return Ok(results);
        };

        for (chunk_idx, chunk_bytes) in mmap_slice.chunks_exact(8).enumerate() {
            let slice: [u8; 8] = chunk_bytes
                .try_into()
                .map_err(|_| crate::errors::QuatDbError::InvalidOffset((chunk_idx * 8) as u64))?;
            let chunk = u64::from_le_bytes(slice);

            for (p_idx, &pattern) in patterns.iter().enumerate() {
                if QuatIndex::match_quat_mask(chunk, pattern) >= min_matches {
                    results[p_idx].push(chunk_idx as u64);
                }
            }
        }

        Ok(results)
    }

    /// Inserts multiple packed quaternary chunks in a single write lock acquisition.
    /// Returns the assigned keys in input order.
    pub fn insert_batch(&self, chunks: &[u64]) -> Result<Vec<u64>> {
        let mut storage = self.storage.write();
        let mut index = self.index.write();

        let mut keys = Vec::with_capacity(chunks.len());
        for &chunk in chunks {
            let offset = storage.append_chunk(chunk)?;
            let key = offset / 8;
            index.insert_primary(key, offset);
            index.update_presence(chunk);
            keys.push(key);
        }

        Ok(keys)
    }

    /// Emits a structured key-value log entry for diagnostics and observability.
    pub fn log_event(&self, level: &str, event: &str, key: Option<u64>, detail: &str) {
        let ver = self.global_version.load(Ordering::Acquire);
        match key {
            Some(k) => eprintln!(
                "ts_version={ver} level={level} event={event} key={k} detail=\"{detail}\""
            ),
            None => eprintln!(
                "ts_version={ver} level={level} event={event} detail=\"{detail}\""
            ),
        }
    }

    /// Returns a point-in-time snapshot of database counters.
    pub fn metrics_snapshot(
        metrics: &DbMetrics,
    ) -> DbMetricsSnapshot {
        DbMetricsSnapshot {
            inserts_total: metrics.inserts_total.load(Ordering::Relaxed),
            reads_total: metrics.reads_total.load(Ordering::Relaxed),
            pattern_queries_total: metrics.pattern_queries_total.load(Ordering::Relaxed),
            mvcc_commits_total: metrics.mvcc_commits_total.load(Ordering::Relaxed),
            mvcc_conflicts_total: metrics.mvcc_conflicts_total.load(Ordering::Relaxed),
        }
    }

    /// Reads multiple keys in a single shared read lock acquisition.
    pub fn get_batch(&self, keys: &[u64]) -> Result<Vec<Option<u64>>> {
        let index = self.index.read();
        let storage = self.storage.read();

        let mut out = Vec::with_capacity(keys.len());
        for &key in keys {
            if let Some(offset) = index.lookup(key) {
                let chunk = storage.read_chunk(offset)?;
                out.push(Some(chunk));
            } else {
                out.push(None);
            }
        }

        Ok(out)
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
    pub read_version: u64,
}

impl<'a> Transaction<'a> {
    pub fn new(db: &'a mut QuatDb) -> Self {
        let current_version = db.global_version.load(std::sync::atomic::Ordering::Acquire);
        Self { db, ops: Vec::new(), read_version: current_version, }
    }

    pub fn stage_insert(&mut self, chunk: u64) {
        let offset = 0; // calculated offset
        self.ops.push((chunk, offset));
    }

    pub fn rollback(&mut self) {
       self.ops.clear();
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

pub fn commit_mvcc(&mut self) -> Result<()> {
        let mut storage = self.db.storage.write();
        let mut index = self.db.index.write();
        
        let _new_version = self.db.global_version.fetch_add(1, Ordering::Release) + 1;

        for &(chunk, offset) in &self.ops {
            let key = storage.append_chunk(chunk)?;
            index.insert_primary(key, offset);
            index.update_presence(chunk);
        }
        
        storage.sync_all()?;
        Ok(())
    }

}