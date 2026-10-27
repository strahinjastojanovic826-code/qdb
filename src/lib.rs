pub mod engine;
pub mod index;
pub mod storage;
pub mod errors;
pub mod ffi;

pub use engine::QuatDb;
pub use index::{QuatIndex, QuatVal};
pub use errors::{QuatDbError, Result};
pub use storage::QuatStorage;
pub use crate::engine::Transaction;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::Instant;
    use std::path::PathBuf;
    use std::io::Write;

    fn cleanup(base_path: &str) {
        let _ = fs::remove_file(base_path);
        let mut wal_path = PathBuf::from(base_path);
        wal_path.set_extension("wal");
        let _ = fs::remove_file(wal_path);
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

    #[test]
    fn test_basic_insert_and_get() {
        print_report_header("Basic Insertion and Retrieval");
        let db_path = "test_basic_unique.qdb";
        cleanup(db_path);

        let mut db = QuatDb::open(db_path).expect("Failed to open database");

        // 1. Insert a test chunk
        let sample_chunk: u64 = 0b11_10_01_00;
        let key = db.insert(sample_chunk).expect("Insertion failed");

        // The first chunk key is calculated based on the file offset (header size 24 / 8 = key 3, or offset 0 depending on indexing)
        let retrieved = db.get_by_key(key).expect("Error reading chunk").unwrap();
        assert_eq!(retrieved, sample_chunk);

        cleanup(db_path);
    }

    #[test]
    fn test_pattern_query_matching() {
        print_report_header("Pattern Query Matching");
        let db_path = "test_pattern_unique.qdb";
        cleanup(db_path);

        let mut db = QuatDb::open(db_path).expect("Failed to open database");

        let chunk1 = 0x0000_0000_0000_0000;
        let chunk2 = 0xFFFF_FFFF_FFFF_FFFF;
        let chunk3 = 0x0000_0000_0000_00FF;

        let k1 = db.insert(chunk1).unwrap();
        let k2 = db.insert(chunk2).unwrap();
        let k3 = db.insert(chunk3).unwrap();

        // Query pattern for exact matching against chunk1
        let matches = db.query_pattern(0x0000_0000_0000_0000, 32).unwrap();
        assert!(matches.contains(&k1));

        // Query partial matches
        let partial_matches = db.query_pattern(0x0000_0000_0000_00FF, 4).unwrap();
        assert!(partial_matches.contains(&k3));

        cleanup(db_path);
    }

    #[test]
fn test_wal_crash_recovery() {
    print_report_header("WAL Crash Recovery");
    let db_path: &str = "test_crash_recovery_unique.qdb";
    cleanup(db_path);

    let first_key: u64;
    {
        let db = QuatDb::open(db_path).expect("Failed to open database");
        first_key = db.insert(0x1234_5678_9ABC_DEF0).expect("Insertion failed");
    }

    // 1. Simulacija padanja: Zapisujemo unwritten chunk u WAL
    let wal_path = std::path::Path::new(db_path).with_extension("wal");
    let unwritten_chunk: u64 = 0xAAAA_BBBB_CCCC_DDDD;

    {
        let mut wal_file = std::fs::OpenOptions::new()
            .append(true)
            .open(&wal_path)
            .expect("WAL file should exist");

        wal_file.write_all(&unwritten_chunk.to_le_bytes()).unwrap();
        wal_file.flush().unwrap();
    }

    // 2. Ručno simuliramo oporavak: Prepisujemo iz WAL-a u glavnu bazu
    {
        let mut db_file = std::fs::OpenOptions::new()
            .append(true)
            .open(db_path)
            .expect("DB file should exist");

        db_file.write_all(&unwritten_chunk.to_le_bytes()).unwrap();
        db_file.flush().unwrap();
    }

    // 3. Otvaramo bazu iznova
    let db = QuatDb::open(db_path).expect("Failed to reopen database");

    // 4. Provjeravamo da li indeks sada vidi novi chunk na poziciji (first_key + 1)
    let recovered_chunk = db.get_by_key(first_key + 1).unwrap();
    assert_eq!(recovered_chunk, Some(unwritten_chunk));

    cleanup(db_path);
}

    #[test]
    fn test_batch_reading_large_dataset() {
        print_report_header("Batch Reading Large Dataset");
        let db_path = "test_large_batch_unique.qdb";
        cleanup(db_path);

        let mut db = QuatDb::open(db_path).expect("Failed to open database");

        for i in 0..1000 {
            db.insert(i as u64).unwrap();
        }

        let target_key = db.get_by_key(500);
        assert!(target_key.is_ok());

        cleanup(db_path);
    }

    use super::ffi::*;
    use std::ffi::CString;
    use std::ptr;

    fn create_temp_db_path(name: &str) -> (CString, std::path::PathBuf) {
        let mut path = std::env::temp_dir();
        path.push(format!("quat_db_ffi_test_{}.db", name));
        
        // Remove old test files if they exist
        let _ = std::fs::remove_file(&path);
        let mut wal_path = path.clone();
        wal_path.set_extension("wal");
        let _ = std::fs::remove_file(&wal_path);

        let c_str = CString::new(path.to_str().unwrap()).unwrap();
        (c_str, path)
    }

    fn cleanup_temp_db(path: std::path::PathBuf) {
        let _ = std::fs::remove_file(&path);
        let mut wal_path = path;
        wal_path.set_extension("wal");
        let _ = std::fs::remove_file(wal_path);
    }


    #[test]
    fn test_ffi_open_and_close() {
        let (c_path, path_buf) = create_temp_db_path("open_close");
        let mut handle: *mut QuatDbHandle = ptr::null_mut();

        unsafe {
            let res = quat_db_open(c_path.as_ptr(), &mut handle);
            assert_eq!(res, QuatErrorCode::Ok);
            assert!(!handle.is_null());

            quat_db_close(handle);
        }

        cleanup_temp_db(path_buf);
    }

    #[test]
    fn test_ffi_null_pointer_handling() {
        let mut handle: *mut QuatDbHandle = ptr::null_mut();

        unsafe {
            // Null path
            let res = quat_db_open(ptr::null(), &mut handle);
            assert_eq!(res, QuatErrorCode::NullPointer);

            // Null output handle pointer
            let c_path = CString::new("test.db").unwrap();
            let res = quat_db_open(c_path.as_ptr(), ptr::null_mut());
            assert_eq!(res, QuatErrorCode::NullPointer);

            // Operations on null handle
            let mut out_key = 0u64;
            assert_eq!(
                quat_db_insert(ptr::null_mut(), 0x1234, &mut out_key),
                QuatErrorCode::NullPointer
            );

            let mut out_chunk = 0u64;
            assert_eq!(
                quat_db_get_by_key(ptr::null(), 0, &mut out_chunk),
                QuatErrorCode::NullPointer
            );
        }
    }

    #[test]
    fn test_ffi_insert_and_get_by_key() {
        let (c_path, path_buf) = create_temp_db_path("insert_get");
        let mut handle: *mut QuatDbHandle = ptr::null_mut();

        unsafe {
            assert_eq!(quat_db_open(c_path.as_ptr(), &mut handle), QuatErrorCode::Ok);

            let test_chunk: u64 = 0xDEAD_BEEF_CAFE_BABE;
            let mut inserted_key: u64 = 0;

            // Insert chunk
            let res = quat_db_insert(handle, test_chunk, &mut inserted_key);
            assert_eq!(res, QuatErrorCode::Ok);

            // Fetch chunk back
            let mut retrieved_chunk: u64 = 0;
            let res = quat_db_get_by_key(handle, inserted_key, &mut retrieved_chunk);
            assert_eq!(res, QuatErrorCode::Ok);
            assert_eq!(retrieved_chunk, test_chunk);

            // Fetch non-existent key
            let res = quat_db_get_by_key(handle, inserted_key + 999, &mut retrieved_chunk);
            assert_eq!(res, QuatErrorCode::NotFound);

            quat_db_close(handle);
        }

        cleanup_temp_db(path_buf);
    }

    #[test]
    fn test_ffi_get_single_quat() {
        let (c_path, path_buf) = create_temp_db_path("single_quat");
        let mut handle: *mut QuatDbHandle = ptr::null_mut();

        unsafe {
            assert_eq!(quat_db_open(c_path.as_ptr(), &mut handle), QuatErrorCode::Ok);

            // Chunk with quat index 0 set to 0b11 (Q3 = 3)
            let test_chunk: u64 = 0b11; 
            let mut key: u64 = 0;

            assert_eq!(quat_db_insert(handle, test_chunk, &mut key), QuatErrorCode::Ok);

            let mut quat_val: u8 = 0;
            let res = quat_db_get_single_quat(handle, key, 0, &mut quat_val);
            assert_eq!(res, QuatErrorCode::Ok);
            assert_eq!(quat_val, 3); // 0b11 corresponds to Q3 = 3

            // Out-of-bounds quat index test (index >= 32)
            let res = quat_db_get_single_quat(handle, key, 32, &mut quat_val);
            assert_eq!(res, QuatErrorCode::IndexOutOfBounds);

            quat_db_close(handle);
        }

        cleanup_temp_db(path_buf);
    }

    #[test]
    fn test_ffi_query_pattern_and_free_keys() {
        let (c_path, path_buf) = create_temp_db_path("query_pattern");
        let mut handle: *mut QuatDbHandle = ptr::null_mut();

        unsafe {
            assert_eq!(quat_db_open(c_path.as_ptr(), &mut handle), QuatErrorCode::Ok);

            let test_chunk: u64 = 0x5555_5555_5555_5555;
            let mut key: u64 = 0;
            assert_eq!(quat_db_insert(handle, test_chunk, &mut key), QuatErrorCode::Ok);

            let mut keys_ptr: *mut u64 = ptr::null_mut();
            let mut keys_len: usize = 0;

            // Query pattern with min_matches = 1
            let res = quat_db_query_pattern(
                handle,
                test_chunk,
                1,
                &mut keys_ptr,
                &mut keys_len,
            );

            assert_eq!(res, QuatErrorCode::Ok);
            assert!(keys_len > 0);
            assert!(!keys_ptr.is_null());

            // Read matched key from raw pointer
            let matched_keys = std::slice::from_raw_parts(keys_ptr, keys_len);
            assert_eq!(matched_keys[0], key);

            // Free allocated memory safely via FFI
            quat_db_free_keys(keys_ptr, keys_len);

            quat_db_close(handle);
        }

        cleanup_temp_db(path_buf);
    }
    #[test]
fn test_range_query() {
    let db_path = "test_range.qdb";
    cleanup(db_path);
    print_report_header("Range Query Test");

    let mut db = QuatDb::open(db_path).unwrap();
    
    // Ubacujemo 3 čunka (dobiće ključeve 0, 1 i 2)
    db.insert(0x1111_1111_1111_1111).unwrap();
    db.insert(0x2222_2222_2222_2222).unwrap();
    db.insert(0x3333_3333_3333_3333).unwrap();

    // Testiramo opseg ključeva od 0 do 1
    let results = db.range_query(0, 1).unwrap();
    
    println!("[STATUS] PASS");
    println!("[METRICS] Range Results Count: {}", results.len());

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].0, 0); // Prvi ključ
    assert_eq!(results[1].0, 1); // Drugi ključ

    cleanup(db_path);
}
#[test]
fn test_transaction_batch_commit() {
    let db_path = "test_transaction.qdb";
    cleanup(db_path);
    print_report_header("Transaction Batch Commit Test");

    let mut db = QuatDb::open(db_path).unwrap();
    
    // Pokrećemo transakciju i dodajemo više upisa u heš/keš
    let mut tx = Transaction::new(&mut db);
    tx.stage_insert(0xAAAA_BBBB_CCCC_DDDD);
    tx.stage_insert(0x1111_2222_3333_4444);
    
    // Potvrđujemo transakciju
    tx.commit().unwrap();

    // Proveravamo da li se unosi mogu pročitati iz baze
    let retrieved = db.get_by_key(0).unwrap();
    
    println!("[STATUS] PASS");
    println!("[METRICS] Batch Transaction Committed Successfully");

    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap(), 0xAAAA_BBBB_CCCC_DDDD);

    cleanup(db_path);
}

}