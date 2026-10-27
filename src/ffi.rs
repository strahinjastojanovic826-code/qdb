use std::ffi::CStr;
use std::os::raw::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;
use std::sync::RwLock;

use crate::engine::QuatDb;
use crate::errors::QuatDbError;

/// Kodovi statusa/grešaka za C-ABI granicu
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum QuatErrorCode {
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
    UnknownError = 99,
}

impl From<&QuatDbError> for QuatErrorCode {
    fn from(err: &QuatDbError) -> Self {
        match err {
            QuatDbError::Io(_) => QuatErrorCode::IoError,
            QuatDbError::InvalidOffset(_) => QuatErrorCode::InvalidOffset,
            QuatDbError::CorruptedWal => QuatErrorCode::CorruptedWal,
            QuatDbError::IndexOutOfBounds { .. } => QuatErrorCode::IndexOutOfBounds,
        }
    }
}

/// Neprozirna struktura bezbjedna za rad sa više niti (Thread-Safe)
pub struct QuatDbHandle {
    db: RwLock<QuatDb>,
}

/// Otvara bazu i upisuje pokazivač u `out_handle`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quat_db_open(
    path: *const c_char,
    out_handle: *mut *mut QuatDbHandle,
) -> QuatErrorCode { unsafe {
    if path.is_null() || out_handle.is_null() {
        return QuatErrorCode::NullPointer;
    }

    let result = catch_unwind(AssertUnwindSafe(|| {
        let c_str = CStr::from_ptr(path);
        let path_str = match c_str.to_str() {
            Ok(s) => s,
            Err(_) => return QuatErrorCode::InvalidString,
        };

        match QuatDb::open(Path::new(path_str)) {
            Ok(db) => {
                let handle = Box::into_raw(Box::new(QuatDbHandle {
                    db: RwLock::new(db),
                }));
                *out_handle = handle;
                QuatErrorCode::Ok
            }
            Err(ref err) => QuatErrorCode::from(err),
        }
    }));

    result.unwrap_or(QuatErrorCode::Panic)
}}

/// Zatvara bazu i oslobađa resurse.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quat_db_close(handle: *mut QuatDbHandle) { unsafe {
    if !handle.is_null() {
        let _ = catch_unwind(AssertUnwindSafe(|| {
            drop(Box::from_raw(handle));
        }));
    }
}}

/// Ubacuje chunk u bazu (zaključava bazu za pisanje).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quat_db_insert(
    handle: *mut QuatDbHandle,
    chunk: u64,
    out_key: *mut u64,
) -> QuatErrorCode { unsafe {
    if handle.is_null() {
        return QuatErrorCode::NullPointer;
    }

    let result = catch_unwind(AssertUnwindSafe(|| {
        let handle_ref = &*handle;
        let db_guard = match handle_ref.db.write() {
            Ok(guard) => guard,
            Err(_) => return QuatErrorCode::LockPoisoned,
        };

        match db_guard.insert(chunk) {
            Ok(key) => {
                if !out_key.is_null() {
                    *out_key = key;
                }
                QuatErrorCode::Ok
            }
            Err(ref err) => QuatErrorCode::from(err),
        }
    }));

    result.unwrap_or(QuatErrorCode::Panic)
}}

/// Dohvata chunk po ključu (zaključava bazu za čitanje).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quat_db_get_by_key(
    handle: *const QuatDbHandle,
    key: u64,
    out_chunk: *mut u64,
) -> QuatErrorCode { unsafe {
    if handle.is_null() || out_chunk.is_null() {
        return QuatErrorCode::NullPointer;
    }

    let result = catch_unwind(AssertUnwindSafe(|| {
        let handle_ref = &*handle;
        let db_guard = match handle_ref.db.read() {
            Ok(guard) => guard,
            Err(_) => return QuatErrorCode::LockPoisoned,
        };

        match db_guard.get_by_key(key) {
            Ok(Some(chunk)) => {
                *out_chunk = chunk;
                QuatErrorCode::Ok
            }
            Ok(None) => QuatErrorCode::NotFound,
            Err(ref err) => QuatErrorCode::from(err),
        }
    }));

    result.unwrap_or(QuatErrorCode::Panic)
}}

/// Dohvata pojedinačni quat po ključu i indeksu.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quat_db_get_single_quat(
    handle: *const QuatDbHandle,
    key: u64,
    quat_idx: usize,
    out_val: *mut u8,
) -> QuatErrorCode { unsafe {
    if handle.is_null() || out_val.is_null() {
        return QuatErrorCode::NullPointer;
    }

    let result = catch_unwind(AssertUnwindSafe(|| {
        let handle_ref = &*handle;
        let db_guard = match handle_ref.db.read() {
            Ok(guard) => guard,
            Err(_) => return QuatErrorCode::LockPoisoned,
        };

        match db_guard.get_single_quat(key, quat_idx) {
            Ok(Some(quat_val)) => {
                *out_val = quat_val as u8;
                QuatErrorCode::Ok
            }
            Ok(None) => QuatErrorCode::NotFound,
            Err(ref err) => QuatErrorCode::from(err),
        }
    }));

    result.unwrap_or(QuatErrorCode::Panic)
}}

/// Pretražuje uzorke i vraća alocirani niz ključeva.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quat_db_query_pattern(
    handle: *const QuatDbHandle,
    pattern: u64,
    min_matches: u32,
    out_keys: *mut *mut u64,
    out_len: *mut usize,
) -> QuatErrorCode { unsafe {
    if handle.is_null() || out_keys.is_null() || out_len.is_null() {
        return QuatErrorCode::NullPointer;
    }

    let result = catch_unwind(AssertUnwindSafe(|| {
        let handle_ref = &*handle;
        let db_guard = match handle_ref.db.read() {
            Ok(guard) => guard,
            Err(_) => return QuatErrorCode::LockPoisoned,
        };

        match db_guard.query_pattern(pattern, min_matches) {
            Ok(vec) => {
                let mut boxed_slice = vec.into_boxed_slice();
                *out_len = boxed_slice.len();
                *out_keys = boxed_slice.as_mut_ptr();
                std::mem::forget(boxed_slice);
                QuatErrorCode::Ok
            }
            Err(ref err) => QuatErrorCode::from(err),
        }
    }));

    result.unwrap_or(QuatErrorCode::Panic)
}}

/// Oslobađa memoriju alociranu u `quat_db_query_pattern`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quat_db_free_keys(keys: *mut u64, len: usize) { unsafe {
    if !keys.is_null() && len > 0 {
        let _ = catch_unwind(AssertUnwindSafe(|| {
            let _ = Box::from_raw(std::slice::from_raw_parts_mut(keys, len));
        }));
    }
}}