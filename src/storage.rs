use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

pub const PAGE_SIZE: usize = 4096; // Standardna veličina stranice na disku
pub const CHUNKS_PER_PAGE: usize = PAGE_SIZE / 8; // 512 x 64-bitna bloka po stranici

pub struct QuatStorage {
    file: File,
    wal_file: File,
}

impl QuatStorage {
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
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

        Ok(Self { file, wal_file })
    }

    /// WAL Write: Upisuje blok prvo u log pa na disk radi sigurnosti
    pub fn append_chunk(&mut self, chunk: u64) -> io::Result<u64> {
        let bytes = chunk.to_le_bytes();

        // 1. Upis u WAL
        self.wal_file.seek(SeekFrom::End(0))?;
        self.wal_file.write_all(&bytes)?;
        self.wal_file.flush()?;

        // 2. Upis u glavnu bazu
        self.file.seek(SeekFrom::End(0))?;
        self.file.write_all(&bytes)?;
        let offset = self.file.metadata()?.len() - 8;

        Ok(offset)
    }

    pub fn read_chunk(&mut self, offset: u64) -> io::Result<u64> {
        self.file.seek(SeekFrom::Start(offset))?;
        let mut buffer = [0u8; 8];
        self.file.read_exact(&mut buffer)?;
        Ok(u64::from_le_bytes(buffer))
    }

    pub fn total_chunks(&self) -> io::Result<u64> {
        Ok(self.file.metadata()?.len() / 8)
    }

    pub fn read_batch(&mut self, start_offset: u64, count: usize) -> io::Result<Vec<u64>> {
        self.file.seek(SeekFrom::Start(start_offset))?;
        let mut buffer = vec![0u8; count * 8];
        self.file.read_exact(&mut buffer)?;

        let chunks = buffer
            .chunks_exact(8)
            .map(|chunk| u64::from_le_bytes(chunk.try_into().unwrap()))
            .collect();

        Ok(chunks)
    }
}