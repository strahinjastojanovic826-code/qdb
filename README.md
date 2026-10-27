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

| Metric | Measured Value | Description |
| :--- | :--- | :--- |
| **Scan Throughput** | **`571.02 MQuats/sec`** | Scanned 320,000 quats (10,000 chunks) in `560.4 µs` |
| **Cold Load Time** | **`3.35 ms`** | Full index recovery & startup time after WAL restart |
| **Read Latency** | **`5.77 ms`** | Low-level 2-bit quat extraction latency |
| **Insertion Throughput** | **`10,000 chunks / 6.63s`** | Batch insertion rate into storage engine |
| **Database Overhead** | **`78 KB`** | Total database file size for 10,000 stored chunks |

---

## Quickstart

Add `qdb` to your project's `Cargo.toml`:

```toml
[dependencies]
quat_db = "0.5.0"

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

    // 4. Close database
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