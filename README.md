# qdb

`qdb` is a ultra-fast, dependency-free embedded database engine designed specifically for **4-state (2-bit quat) logic storage**. Built directly on top of `std` primitives, `qdb` provides high-density bit-packed records, zero-copy byte retrieval, Write-Ahead Logging (WAL) fault tolerance, and hardware-level bitwise indexing.

---

## Features

* **Zero External Dependencies** — Built strictly using Rust's standard library (`std`).
* **Dense Bit-Packing** — Stores data in 64-bit chunks, holding 32 individual 4-state quats (`Q0`, `Q1`, `Q2`, `Q3`) per chunk.
* **WAL Persistence & Recovery** — Includes Write-Ahead Logging to guarantee crash-resilient append operations.
* **In-Memory Offset Indexing** — Fast key-to-offset index mapping rebuilt dynamically on cold startup.
* **Bitwise Mask Searching** — Query high-throughput 4-state bit patterns using hardware-native bitwise operations (XNOR/Popcount).
* **Detailed Audit Logging** — Built-in test suite providing precise execution metrics and throughput reporting.

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

## Quickstart

Add `qdb` to your project's `Cargo.toml`:

```toml
[dependencies]
qdb = "0.1.0"

```

Basic Usage

use qdb::{QuatDb, QuatVal};

```

```rust

fn main() -> std::io::Result<()> {
    // 1. Open or create a database instance
    let mut db = QuatDb::open("data.qdb")?;

    // 2. Insert a 64-bit chunk representing 32 quat states
    // Binary layout: Quat 0 = Q0 (00), Quat 1 = Q1 (01), Quat 2 = Q2 (10), Quat 3 = Q3 (11)
    let chunk_payload: u64 = 0b11_10_01_00;
    let key = db.insert(chunk_payload)?;

    // 3. Extract individual 2-bit states from the database
    let q0 = db.get_single_quat(key, 0)?.unwrap();
    let q3 = db.get_single_quat(key, 3)?.unwrap();

    assert_eq!(q0, QuatVal::Q0);
    assert_eq!(q3, QuatVal::Q3);

    println!("Key {} successfully read and validated!", key);
    Ok(())
}

```

High-Throughput Pattern Scanning

Execute bitwise search masks directly over pages of packed records:

```rust

use qdb::QuatDb;

fn main() -> std::io::Result<()> {
    let mut db = QuatDb::open("records.qdb")?;

    // Search across all stored chunks for blocks having at least 32 matching Q3 states (0xFFFF_FFFF_FFFF_FFFF)
    let matches = db.query_pattern(u64::MAX, 32)?;

    println!("Found {} matching chunk offsets in storage.", matches.len());
    Ok(())
}

```

Audit & Benchmarks

Run the audit suite to inspect database metrics, throughput, and recovery times:

```rust
cargo test -- --nocapture
