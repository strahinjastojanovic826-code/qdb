# qdb

`qdb` is a ultra-fast, dependency-free embedded database engine designed specifically for **4-state (2-bit quat) logic storage**. Built directly on top of `std` primitives, `qdb` provides high-density bit-packed records, zero-copy byte retrieval, Write-Ahead Logging (WAL) fault tolerance, and hardware-level bitwise indexing.

---

## Features

* **High-Performance Memory-Mapped I/O** ⚡ Zero-copy persistence engine powered by `memmap2` for high-throughput disk I/O.
* **Thread-Safe & Concurrent** 🔒 Multi-threaded read/write safety using `Arc<RwLock>` (parking_lot) for fast concurrent operations.
* **Dense Bit-Packing** 📦 Stores data in 64-bit chunks, holding 32 individual 4-state 2-bit quat values.
* **WAL Persistence & Recovery** 🛡️ Includes Write-Ahead Logging to guarantee crash resistance and instant cold-start recovery.
* **In-Memory Offset Indexing** 🗺️ Fast key-to-offset index mapping rebuilt dynamically upon initialization.
* **Bitwise Mask Searching** 🔍 Query high-throughput 4-state bit patterns directly over packed records.
* **Custom Error Handling** 🚦 Strongly-typed error variants powered by `thiserror` (`QuatDbError`).
* **Range Queries** 📊 Fast retrieval of data chunks and records within specified key boundaries.
* **Batch Transactions** ⚡ Atomic group-write operations optimizing performance via single-batch disk synchronization.
* **MVCC Transactions** 🔄 Multi-version concurrency control allowing non-blocking concurrent reads during writes.
* **Robust WAL Checksum Recovery** 🛡️ Enhanced crash-safety with automatic CRC/hash validation on log entries.
* **Genomics & DNA Support** 🧬 Native 2-bit packing and high-speed bitwise pattern querying for DNA/RNA sequences (A, C, G, T).
* **Quaternary Bitwise Logic** — Parallel 2-bit `AND`, `OR`, and `XOR` operations across all 32 packed quaternary states inside a single `u64` chunk.
* **Wildcard & Cross-Chunk Pattern Search** — Position-level allowed-state bitmasks (`Q0..Q3`) and sub-sequence matching across adjacent `u64` chunk boundaries.
* **Batch API & Vectorized Scans** — High-throughput `insert_batch`, `get_batch`, and multi-pattern `query_pattern_batch` over memory-mapped slices.
* **Transaction Rollback & Conflict Handling** — Explicit staged rollback (`rollback`) and version conflict reporting (`MvccConflict`) for transactional writes.
* **Versioned Binary Headers & Checksums** — Magic-byte and version-validated file/WAL headers (`UnsupportedVersion`) paired with deterministic FNV-1a checksums.
* **Structured Logging & Metrics** — Atomic runtime counters (`DbMetrics`) and key-value diagnostic logging for observability.

---

## Data Representation

Data states are tightly aligned in 2-bit quats inside standard 64-bit unsigned blocks:

| Quat State | Binary Pattern | Logical Value |
| :--- | :--- | :--- |
| `Q0` | `0b00` | `0` |
| `Q1` | `0b01` | `1` |
| `Q2` | `0b10` | `2` |
| `Q3` | `0b11` | `3` |

A single `u64` block houses **32 quat states**, eliminating redundant padding and optimizing disk page cache layout.

---

## Licensing

`quat_db` is dual-licensed under the following terms:

1. **Open Source License (AGPLv3):**  
   Free for open-source projects, research, and non-commercial use. Any derivative work or network-hosted service (SaaS) using this library under AGPLv3 must make its source code public. See the [LICENSE](LICENSE) file for details.

2. **Commercial License:**  
   For closed-source applications, proprietary software integration, or organizations wishing to avoid AGPLv3 copyleft obligations. Commercial licenses grant full rights to embed `quat_db` without source disclosure.

For commercial licensing inquiries, pricing, or custom SLA support, please contact: **strahinjastojanovic826@gmail.com**

---

### Key Performance Metrics

| Benchmark / Test Suite | Workload / Scale | Execution Time | Throughput / Key Metric | Output / Artifact Details |
| :--- | :--- | :--- | :--- | :--- |
| `test_quaternary_bitwise_ops_benchmark` | 50,000 iterations (4,800,000 quats across `AND` / `OR` / `XOR`) | `673.2µs` | `7,130.12 MQuats/sec` | Bitwise operations over 32 packed 2-bit quats per `u64` |
| `test_pattern_batch_scan` | 10,000 chunks (320,000 quats scanned) | `569.7µs` (scan) / `13.93s` (insert) | `561.70 MQuats/sec` scan | 5,000 matching chunks; DB size `78 KB` |
| `test_batch_api_and_pattern_scan_benchmark` | 5,000 chunks batch insert, get & multi-pattern scan | `8.995s` (insert) / `1.358ms` (get) / `458.8µs` (scan) | Sub-millisecond batch scan & ~1.36ms batch read | 2,500 all-`Q3` matches in batch scan |
| `test_wildcard_and_cross_chunk_pattern_benchmark` | 50,000 iterations (wildcard mask + cross-chunk match) | `61.6448ms` | `811.10 KIter/sec` | 50,000 cross-chunk matches verified |
| `test_genomics_dna_packing_benchmark` | 10,000 DNA sequences (32 nucleotides each) | `13.946s` | `717.03 Seq/sec` | Full pack, insert, retrieve & unpack round-trip validated |
| `test_mvcc_transaction_benchmark` | 1,000 staged operations in MVCC transaction | `2.502s` | `399.67 Ops/sec` | DB size `8,000 bytes` |
| `test_versioned_header_and_checksum_benchmark` | 1,000 versioned records + WAL entries | `4.471s` | `223.66 Rec/sec` | DB size `8,000 bytes`, WAL size `28,000 bytes` |
| `test_transaction_rollback_and_mvcc_conflict_benchmark` | 500 cycles (`rollback` + MVCC commit per cycle) | `3.273s` | `152.79 Cycles/sec` | 500 rollbacks and 500 MVCC commits completed |
| `test_wal_checksum_recovery_benchmark` | 500 WAL entries with checksum recovery | `1.296s` | ~`386 Entries/sec` recovery | Sample check value `0xDEADBEEFCAFEBABE` recovered |

---

## Quickstart

Add `qdb` to your project's `Cargo.toml`:

```toml
[dependencies]
quat_db = "0.7.0"

```

Basic Usage


```rust
use qdb::{QuatDb, QuatVal};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Open or create a database instance (triggers automatic WAL recovery if needed)
    let db = QuatDb::open("data.qdb")?;

    // 2. Insert a 64-bit chunk representing 32 packed quat states
    let chunk_payload: u64 = 0b11_10_01_00;
    let key = db.insert(chunk_payload)?;

    // 3. Extract individual 2-bit states from the database
    let q0 = db.get_single_quat(key, 0)?.ok_or("Key not found")?;
    let q3 = db.get_single_quat(key, 3)?.ok_or("Key not found")?;

    assert_eq!(q0, QuatVal::Q0);
    assert_eq!(q3, QuatVal::Q3);

    println!("Key {} successfully read and validated!", key);
    Ok(())
}
```

### Genomics & DNA Sequence Storage

Store and query biological sequences natively using 2-bit quaternary packing:

```rust
use qdb::{QuatDb, genomics::{pack_dna_sequence, unpack_dna_sequence}};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut db = QuatDb::open("genomics.qdb")?;

    // 1. Pack a 32-nucleotide DNA string into a single u64 chunk
    let dna_sample = "ACGTACGTACGTACGTACGTACGTACGTACGT";
    let chunk = pack_dna_sequence(dna_sample)?;
    
    // 2. Insert into the database
    let key = db.insert(chunk)?;

    // 3. Retrieve and unpack back to DNA string
    let retrieved_chunk = db.get_by_key(key)?.ok_or("Key not found")?;
    let unpacked_dna = unpack_dna_sequence(retrieved_chunk, 32)?;

    println!("Original DNA: {}", dna_sample);
    println!("Unpacked DNA: {}", unpacked_dna);
    Ok(())
}
```

### Quaternary Bitwise Ops, Batch API & Rollback

```rust
use quat_db::engine::QuatDb;
use quat_db::index::QuatIndex;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut db = QuatDb::open("records.qdb")?;

    // 1. Batch insert & batch read
    let keys = db.insert_batch(&[0x00E4_00E4_00E4_00E4, u64::MAX])?;
    let _batch_values = db.get_batch(&keys)?;

    // 2. Batch pattern scan across memory-mapped chunks
    let _matches = db.query_pattern_batch(&[u64::MAX], 32)?;

    // 3. Quaternary bitwise AND / OR / XOR on 32 packed quats per u64
    let a = 0b11_10_01_00_u64;
    let b = 0b11_00_11_00_u64;
    let _q_and = QuatIndex::quat_and(a, b);
    let _q_or = QuatIndex::quat_or(a, b);
    let _q_xor = QuatIndex::quat_xor(a, b);

    // 4. Staged transaction with rollback
    let mut tx = db.begin_transaction();
    tx.stage_insert(0xDEAD_BEEF_CAFE_BABE);
    tx.rollback();

    Ok(())
}
```

### Range Queries & Batch Transactions

You can query ranges of keys or group multiple insertions into a single atomic transaction:

```rust
use qdb::QuatDb;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut db = QuatDb::open("records.qdb")?;

    // 1. Range Query: Fetch all chunks between key 0 and 10
    let range_results = db.range_query(0, 10)?;
    println!("Found {} chunks in the given range.", range_results.len());

    // 2. Batch Transactions: Group multiple writes atomically
    let mut tx = qdb::Transaction::new(&mut db);
    tx.stage_insert(0x1111_2222_3333_4444);
    tx.stage_insert(0x5555_6666_7777_8888);
    tx.commit()?; // Flushed and synced efficiently in a single batch

    Ok(())
}
```

High-Throughput Pattern Scanning

Execute bitwise search masks directly over pages of packed records:

```rust
use qdb::QuatDb;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Open the database (WAL ensures all previous writes are safely recovered)
    let db = QuatDb::open("records.qdb")?;

    // 2. Search across all stored chunks for blocks having at least 32 matching Q3 states
    let pattern = u64::MAX; // 0xFFFF_FFFF_FFFF_FFFF
    let matches = db.query_pattern(pattern, 32)?;

    println!("Found {} matching chunk keys in storage: {:?}", matches.len(), matches);
    Ok(())
}

```

Audit & Benchmarks

Run the audit suite to inspect database metrics, throughput, and recovery times:

```rust
cargo test -- --nocapture

```

### C Example (`main.c`)

```c
#include <stdio.h>
#include <stdint.h>
#include "quat_db.h"

int main(void) {
    QuatDbHandle* db = NULL;

    // 1. Open database
    QuatErrorCode err = quat_db_open("my_db.db", &db);
    if (err != QUAT_OK) {
        printf("Error opening database: %d\n", err);
        return 1;
    }

    // 2. Insert chunk
    uint64_t chunk = 0x123456789ABCDEF0ULL;
    uint64_t generated_key = 0;
    err = quat_db_insert(db, chunk, &generated_key);
    if (err == QUAT_OK) {
        printf("Successfully inserted chunk! Generated key: %LLu\n", (unsigned long long)generated_key);
    }

    // 3. Get by key
    uint64_t fetched_chunk = 0;
    err = quat_db_get_by_key(db, generated_key, &fetched_chunk);
    if (err == QUAT_OK) {
        printf("Fetched chunk: 0x%LLX\n", (unsigned long long)fetched_chunk);
    }

    const char* dna_sequence = "ACGTACGTACGTACGTACGTACGTACGTACGT";
uint64_t dna_key = 0;
err = quat_db_insert_dna(db, dna_sequence, &dna_key);
if (err != QUAT_OK) {
    printf("Error inserting DNA sequence: %d\n", err);
} else {
    printf("Successfully inserted DNA sequence! Assigned key: %llu\n", (unsigned long long)dna_key);
}

    // 4. Close database
    quat_db_close(db);
    return 0;
}
```

### Extended Error Codes & Single-Quat Read (C / C++ / C#)

When calling `quat_db` across the C ABI (`ffi.rs`), handle both existing error codes and newer status codes (`UnsupportedVersion = 10`, `MvccConflict = 11`):

```c
#include <stdio.h>
#include <stdint.h>
#include "quat_db.h"

int main(void) {
    QuatDbHandle* db = NULL;
    if (quat_db_open("extended.qdb", &db) != QUAT_OK) {
        return 1;
    }

    // 0b11_10_01_00 -> quat[0]=0 (Q0), quat[1]=1 (Q1), quat[2]=2 (Q2), quat[3]=3 (Q3)
    uint64_t chunk = 0b11100100ULL;
    uint64_t key = 0;
    if (quat_db_insert(db, chunk, &key) == QUAT_OK) {
        uint8_t q0 = 0, q3 = 0;
        if (quat_db_get_single_quat(db, key, 0, &q0) == QUAT_OK &&
            quat_db_get_single_quat(db, key, 3, &q3) == QUAT_OK) {
            printf("key=%llu, q0=%u, q3=%u\n",
                   (unsigned long long)key, (unsigned)q0, (unsigned)q3);
        }
    }

    quat_db_close(db);
    return 0;
}
```

---

### C++ Modern RAII Example (`main.cpp`)

```cpp
#include <iostream>
#include <vector>
#include <string>
#include <stdexcept>
#include "quat_db.h"

namespace quatdb {

class Database {
private:
    QuatDbHandle* handle_ = nullptr;

public:
    explicit Database(const std::string& path) {
        QuatErrorCode err = quat_db_open(path.c_str(), &handle_);
        if (err != QUAT_OK) {
            throw std::runtime_error("Failed to open database, error code: " + std::to_string(err));
        }
    }

    ~Database() {
        if (handle_) {
            quat_db_close(handle_);
        }
    }

    // Prevent copying
    Database(const Database&) = delete;
    Database& operator=(const Database&) = delete;

    // Enable move semantics
    Database(Database&& other) noexcept : handle_(other.handle_) {
        other.handle_ = nullptr;
    }

    Database& operator=(Database&& other) noexcept {
        if (this != &other) {
            if (handle_) quat_db_close(handle_);
            handle_ = other.handle_;
            other.handle_ = nullptr;
        }
        return *this;
    }

    uint64_t insert(uint64_t chunk) {
        uint64_t key = 0;
        QuatErrorCode err = quat_db_insert(handle_, chunk, &key);
        if (err != QUAT_OK) {
            throw std::runtime_error("Insert failed with code: " + std::to_string(err));
        }
        return key;
    }

    uint64_t get_by_key(uint64_t key) const {
        uint64_t chunk = 0;
        QuatErrorCode err = quat_db_get_by_key(handle_, key, &chunk);
        if (err != QUAT_OK) {
            throw std::runtime_error("Get failed with code: " + std::to_string(err));
        }
        return chunk;
    }

    std::vector<uint64_t> query_pattern(uint64_t pattern, uint32_t min_matches) const {
        uint64_t* keys_ptr = nullptr;
        size_t len = 0;
        QuatErrorCode err = quat_db_query_pattern(handle_, pattern, min_matches, &keys_ptr, &len);
        if (err != QUAT_OK) {
            throw std::runtime_error("Query failed with code: " + std::to_string(err));
        }

        std::vector<uint64_t> result(keys_ptr, keys_ptr + len);
        quat_db_free_keys(keys_ptr, len);
        return result;
    }
};

} // namespace quatdb

int main() {
    try {
        quatdb::Database db("my_db.db");

        uint64_t key = db.insert(0xAABBCCDDEEFF0011ULL);
        std::cout << "Generated key: " << key << std::endl;

        uint64_t val = db.get_by_key(key);
        std::cout << "Fetched value: 0x" << std::hex << val << std::dec << std::endl;

    } catch (const std::exception& e) {
        std::cerr << "Exception: " << e.what() << std::endl;
        return 1;
    }

    return 0;
}
```

---

### C# / .NET Example (`Program.cs`)

```csharp
using System;
using System.Runtime.InteropServices;

namespace QuatDbWrapper
{
    public enum QuatErrorCode : int
    {
        Ok = 0,
        NullPointer = 1,
        InvalidString = 2,
        Panic = 3,
        IoError = 4,
        InvalidOffset = 5,
        CorruptedWal = 6,
        IndexOutOfBounds = 7,
        NotFound = 8,
        LockPoisoned = 9,
        UnknownError = 99
    }

    internal static class NativeMethods
    {
        private const string LibName = "quat_db";

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
        public static extern QuatErrorCode quat_db_open(string path, out IntPtr handle);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern void quat_db_close(IntPtr handle);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern QuatErrorCode quat_db_insert(IntPtr handle, ulong chunk, out ulong outKey);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern QuatErrorCode quat_db_get_by_key(IntPtr handle, ulong key, out ulong outChunk);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern QuatErrorCode quat_db_query_pattern(IntPtr handle, ulong pattern, uint minMatches, out IntPtr outKeys, out UIntPtr outLen);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern void quat_db_free_keys(IntPtr keys, UIntPtr len);
    }

    public class QuatDatabase : IDisposable
    {
        private IntPtr _handle;
        private bool _disposed;

        public QuatDatabase(string path)
        {
            QuatErrorCode err = NativeMethods.quat_db_open(path, out _handle);
            if (err != QuatErrorCode.Ok)
            {
                throw new Exception($"Failed to open database. Error: {err}");
            }
        }

        public ulong Insert(ulong chunk)
        {
            CheckDisposed();
            QuatErrorCode err = NativeMethods.quat_db_insert(_handle, chunk, out ulong key);
            if (err != QuatErrorCode.Ok)
            {
                throw new Exception($"Insert failed. Error: {err}");
            }
            return key;
        }

        public ulong GetByKey(ulong key)
        {
            CheckDisposed();
            QuatErrorCode err = NativeMethods.quat_db_get_by_key(_handle, key, out ulong chunk);
            if (err != QuatErrorCode.Ok)
            {
                throw new Exception($"Get failed. Error: {err}");
            }
            return chunk;
        }

        public unsafe ulong[] QueryPattern(ulong pattern, uint minMatches)
        {
            CheckDisposed();
            QuatErrorCode err = NativeMethods.quat_db_query_pattern(_handle, pattern, minMatches, out IntPtr keysPtr, out UIntPtr lenPtr);
            if (err != QuatErrorCode.Ok)
            {
                throw new Exception($"Query failed. Error: {err}");
            }

            int len = (int)lenPtr;
            if (len == 0 || keysPtr == IntPtr.Zero) return Array.Empty<ulong>();

            ulong[] result = new ulong[len];
            ulong* src = (ulong*)keysPtr;
            for (int i = 0; i < len; i++)
            {
                result[i] = src[i];
            }

            NativeMethods.quat_db_free_keys(keysPtr, lenPtr);
            return result;
        }

        private void CheckDisposed()
        {
            if (_disposed) throw new ObjectDisposedException(nameof(QuatDatabase));
        }

        public void Dispose()
        {
            if (!_disposed)
            {
                if (_handle != IntPtr.Zero)
                {
                    NativeMethods.quat_db_close(_handle);
                    _handle = IntPtr.Zero;
                }
                _disposed = true;
            }
            GC.SuppressFinalize(this);
        }

        ~QuatDatabase()
        {
            Dispose();
        }
    }

    class Program
    {
        static void Main()
        {
            using (var db = new QuatDatabase("my_db.db"))
            {
                ulong key = db.Insert(0x123456789ABCDEF0);
                Console.WriteLine($"Key: {key}");

                ulong chunk = db.GetByKey(key);
                Console.WriteLine($"Fetched: 0x{chunk:X}");
            }
        }
    }
}
```