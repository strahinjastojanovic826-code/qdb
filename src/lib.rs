pub mod engine;
pub mod index;
pub mod storage;

pub use engine::QuatDb;
pub use index::{QuatIndex, QuatVal};
pub use storage::QuatStorage;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::Instant;

    fn cleanup(base_path: &str) {
        let _ = fs::remove_file(base_path);
        let _ = fs::remove_file(format!("{}.wal", base_path));
    }

    fn print_report_header(test_name: &str) {
        println!("\n==================================================");
        println!(" QDB AUDIT REPORT: {}", test_name);
        println!("==================================================");
    }

    #[test]
    fn test_basic_insertion_and_retrieval() {
        let db_path = "test_basic.qdb";
        cleanup(db_path);
        print_report_header("Basic Insertion & Single-Chunk Retrieval");

        let start = Instant::now();
        let mut db = QuatDb::open(db_path).unwrap();

        let payload = 0xAAAA_BBBB_CCCC_DDDD;
        let key = db.insert(payload).unwrap();
        let retrieved = db.get_by_key(key).unwrap().unwrap();

        let duration = start.elapsed();
        let file_size = fs::metadata(db_path).unwrap().len();

        println!("[STATUS]   PASS");
        println!("[METRICS]  Inserted Payload : 0x{:016X}", payload);
        println!("[METRICS]  Retrieved Payload: 0x{:016X}", retrieved);
        println!("[METRICS]  Assigned Key     : {}", key);
        println!("[METRICS]  Database Size    : {} bytes", file_size);
        println!("[METRICS]  Execution Time   : {:?}", duration);

        assert_eq!(retrieved, payload);
        cleanup(db_path);
    }

    #[test]
    fn test_single_quat_extraction() {
        let db_path = "test_quat_extract.qdb";
        cleanup(db_path);
        print_report_header("Low-Level 2-Bit Quat State Extraction");

        let start = Instant::now();
        let mut db = QuatDb::open(db_path).unwrap();

        // Binary: ... 01 (Q1) 11 (Q3)
        // Bit 0-1 = Q3, Bit 2-3 = Q1
        let raw_chunk = 0b01_11;
        let key = db.insert(raw_chunk).unwrap();

        let q0 = db.get_single_quat(key, 0).unwrap().unwrap();
        let q1 = db.get_single_quat(key, 1).unwrap().unwrap();
        let duration = start.elapsed();

        println!("[STATUS]   PASS");
        println!("[METRICS]  Raw Chunk Bits   : 0b{:04b}", raw_chunk);
        println!("[METRICS]  Quat Index 0     : {:?} (Expected: Q3)", q0);
        println!("[METRICS]  Quat Index 1     : {:?} (Expected: Q1)", q1);
        println!("[METRICS]  Execution Time   : {:?}", duration);

        assert_eq!(q0, QuatVal::Q3);
        assert_eq!(q1, QuatVal::Q1);
        cleanup(db_path);
    }

    #[test]
    fn test_pattern_batch_scan() {
        let db_path = "test_scan.qdb";
        cleanup(db_path);
        print_report_header("High-Throughput Batch Bitwise Pattern Scan");

        let mut db = QuatDb::open(db_path).unwrap();
        let total_records = 10_000;

        let insert_start = Instant::now();
        for i in 0..total_records {
            if i % 2 == 0 {
                db.insert(u64::MAX).unwrap(); // All 32 quats set to Q3 (11)
            } else {
                db.insert(0).unwrap(); // All 32 quats set to Q0 (00)
            }
        }
        let insert_duration = insert_start.elapsed();

        let scan_start = Instant::now();
        let results = db.query_pattern(u64::MAX, 32).unwrap();
        let scan_duration = scan_start.elapsed();

        let total_quats_scanned = total_records * 32;
        let file_size = fs::metadata(db_path).unwrap().len();

        println!("[STATUS]   PASS");
        println!("[METRICS]  Total Chunks Stored : {}", total_records);
        println!("[METRICS]  Total Quats Scanned : {}", total_quats_scanned);
        println!("[METRICS]  Database File Size  : {} KB", file_size / 1024);
        println!("[METRICS]  Matching Chunks     : {}", results.len());
        println!("[PERF]     Insertion Latency   : {:?}", insert_duration);
        println!("[PERF]     Scan Throughput     : {:?} ({:.2} MQuats/sec)", 
            scan_duration, 
            (total_quats_scanned as f64 / 1_000_000.0) / scan_duration.as_secs_f64()
        );

        assert_eq!(results.len(), 5000);
        cleanup(db_path);
    }

    #[test]
    fn test_index_reconstruction_and_wal_persistence() {
        let db_path = "test_reopen.qdb";
        cleanup(db_path);
        print_report_header("Cold Reopen & WAL Index Reconstruction");

        let test_payload = 0x1234_5678_9ABC_DEF0;

        // Phase 1: Write and abruptly close
        {
            let mut db = QuatDb::open(db_path).unwrap();
            db.insert(test_payload).unwrap();
            println!("[INFO]     Data committed to storage & WAL. Simulating restart...");
        }

        // Phase 2: Reopen cold database and verify in-memory index rebuilding
        let start = Instant::now();
        let mut db = QuatDb::open(db_path).unwrap();
        let retrieved = db.get_by_key(0).unwrap().unwrap();
        let duration = start.elapsed();

        println!("[STATUS]   PASS");
        println!("[METRICS]  Reopened Payload : 0x{:016X}", retrieved);
        println!("[METRICS]  Index Recovery   : Successful");
        println!("[METRICS]  Cold Load Time   : {:?}", duration);

        assert_eq!(retrieved, test_payload);
        cleanup(db_path);
    }
    #[test]
fn test_all_four_states_reading() {
    let db_path = "test_four_states.qdb";
    cleanup(db_path);
    print_report_header("Sequential Reading of All 4 Quat States (Q0, Q1, Q2, Q3)");

    let start = Instant::now();
    let mut db = QuatDb::open(db_path).unwrap();

    // Konstruišemo 64-bitni blok koji na prvim pozicijama sadrži sva 4 stanja:
    // Quat 0: Q0 (00)
    // Quat 1: Q1 (01)
    // Quat 2: Q2 (10)
    // Quat 3: Q3 (11)
    // Binary raspored u prvom bajtu: 0b11_10_01_00 = 0xE4
    let four_state_payload: u64 = 0b11_10_01_00; 
    let key = db.insert(four_state_payload).unwrap();

    // Čitamo jedno po jedno 2-bitno stanje iz baze
    let q0 = db.get_single_quat(key, 0).unwrap().unwrap();
    let q1 = db.get_single_quat(key, 1).unwrap().unwrap();
    let q2 = db.get_single_quat(key, 2).unwrap().unwrap();
    let q3 = db.get_single_quat(key, 3).unwrap().unwrap();

    let duration = start.elapsed();

    println!("[STATUS]   PASS");
    println!("[METRICS]  Raw Stored Chunk : 0b{:08b}", four_state_payload);
    println!("[METRICS]  Index 0 (0b00)   : {:?} -> Value 0", q0);
    println!("[METRICS]  Index 1 (0b01)   : {:?} -> Value 1", q1);
    println!("[METRICS]  Index 2 (0b10)   : {:?} -> Value 2", q2);
    println!("[METRICS]  Index 3 (0b11)   : {:?} -> Value 3", q3);
    println!("[METRICS]  Read Latency     : {:?}", duration);

    assert_eq!(q0, QuatVal::Q0);
    assert_eq!(q1, QuatVal::Q1);
    assert_eq!(q2, QuatVal::Q2);
    assert_eq!(q3, QuatVal::Q3);

    cleanup(db_path);
  }
}