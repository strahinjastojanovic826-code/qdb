use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;
use memmap2::Mmap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::Read;

use crate::errors::{QuatDbError, Result};

pub const DEFAULT_PAGE_SIZE: usize = 4096;

pub struct QuatStorage {
    file: File,
    wal_file: File,
    mmap: Option<Mmap>,
}

impl QuatStorage {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let wal_path = path.as_ref().with_extension("wal");

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;

        let wal_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(wal_path)?;

        let mut storage = Self {
            file,
            wal_file,
            mmap: None,
        };

        storage.reload_mmap()?;
        Ok(storage)
    }

    /// Refresh the memory-mapped file buffer
    pub fn reload_mmap(&mut self) -> Result<()> {
        let len = self.file.metadata()?.len();
        if len > 0 {
            let mmap = unsafe { Mmap::map(&self.file)? };
            self.mmap = Some(mmap);
        } else {
            self.mmap = None;
        }
        Ok(())
    }

    pub fn append_wal_robust(&mut self, chunk: u64, offset: u64) -> Result<()> {
        let mut hasher = DefaultHasher::new();
        chunk.hash(&mut hasher);
        offset.hash(&mut hasher);
        let checksum = hasher.finish();

        let mut bytes = Vec::with_capacity(24);
        bytes.extend_from_slice(&chunk.to_le_bytes());
        bytes.extend_from_slice(&offset.to_le_bytes());
        bytes.extend_from_slice(&checksum.to_le_bytes());

        self.wal_file.seek(SeekFrom::End(0))?;
        self.wal_file.write_all(&bytes)?;
        self.wal_file.sync_all()?;
        
        Ok(())
    }

    // Recovers database state from WAL checking for corrupted entries
    pub fn recover_with_validation(&mut self) -> Result<u64> {
        self.wal_file.seek(SeekFrom::Start(0))?;
        let mut buffer = Vec::new();
        self.wal_file.read_to_end(&mut buffer)?;

        let entry_size = 24; // 8 bytes chunk + 8 bytes offset + 8 bytes checksum
        let total_chunks = buffer.len() / entry_size;
        let mut valid_chunks_count = 0;

        for i in 0..total_chunks {
            let start = i * entry_size;
            let chunk = u64::from_le_bytes(buffer[start..start+8].try_into().unwrap());
            let offset = u64::from_le_bytes(buffer[start+8..start+16].try_into().unwrap());
            let stored_checksum = u64::from_le_bytes(buffer[start+16..start+24].try_into().unwrap());

            let mut hasher = DefaultHasher::new();
            chunk.hash(&mut hasher);
            offset.hash(&mut hasher);
            let calculated_checksum = hasher.finish();

            if stored_checksum != calculated_checksum {
                return Err(QuatDbError::CorruptedWal);
            }
            valid_chunks_count += 1;
        }

        Ok(valid_chunks_count)
    }

    pub fn sync_all(&mut self) -> std::io::Result<()> {
        self.wal_file.sync_all()?;
        self.file.sync_all()?;
        Ok(())
    }

    /// Appends chunk to WAL with strict sync_all(), then writes to primary disk
    pub fn append_chunk(&mut self, chunk: u64) -> Result<u64> {
        let bytes = chunk.to_le_bytes();

        // 1. Write to WAL first
        self.wal_file.seek(SeekFrom::End(0))?;
        self.wal_file.write_all(&bytes)?;
        self.wal_file.sync_all()?;

        // 2. Append to primary database file
        let offset = self.file.seek(SeekFrom::End(0))?;
        self.file.write_all(&bytes)?;
        self.file.flush()?;

        // 3. Update memory map so new data is immediately readable
        self.reload_mmap()?;

        Ok(offset)
    }

    /// Zero-copy read directly from memory-mapped disk region
    pub fn read_chunk(&self, offset: u64) -> Result<u64> {
        let start = offset as usize;
        let end = start + 8;

        if let Some(ref mmap) = self.mmap {
            if end <= mmap.len() {
                let slice: [u8; 8] = mmap[start..end]
                    .try_into()
                    .map_err(|_| QuatDbError::InvalidOffset(offset))?;
                return Ok(u64::from_le_bytes(slice));
            }
        }

        Err(QuatDbError::InvalidOffset(offset))
    }

    pub fn total_chunks(&self) -> Result<u64> {
        Ok(self.file.metadata()?.len() / 8)
    }

    /// Zero-copy batch slice directly over mapped memory
    pub fn get_mmap_slice(&self) -> Option<&[u8]> {
        self.mmap.as_deref()
    }

    pub fn checkpoint_wal(&mut self) -> Result<()> {
        self.wal_file.set_len(0)?;
        self.wal_file.sync_all()?;
        Ok(())
    }
}