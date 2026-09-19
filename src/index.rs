use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuatVal {
    Q0 = 0,
    Q1 = 1,
    Q2 = 2,
    Q3 = 3,
}

pub struct QuatIndex {
    // Mapa koja povezuje ID zapisa ili hash sa offsetom u fajlu
    primary_map: HashMap<u64, u64>,
}

impl QuatIndex {
    pub fn new() -> Self {
        Self {
            primary_map: HashMap::new(),
        }
    }

    pub fn insert_primary(&mut self, key: u64, offset: u64) {
        self.primary_map.insert(key, offset);
    }

    pub fn lookup(&self, key: u64) -> Option<u64> {
        self.primary_map.get(&key).copied()
    }

    /// Izračunava koliko se 2-bitnih kvatova u bloku potpuno poklapa sa ciljanom maskom
    #[inline(always)]
    pub fn match_quat_mask(chunk: u64, pattern: u64) -> u32 {
        let xnor = !(chunk ^ pattern);
        let even_bits = xnor & 0x5555_5555_5555_5555;
        let odd_bits = (xnor >> 1) & 0x5555_5555_5555_5555;
        (even_bits & odd_bits).count_ones()
    }

    /// Ekstrakcija jednog kvata sa tačne pozicije (0-31) unutar 64-bitnog bloka
    #[inline(always)]
    pub fn extract_quat(chunk: u64, quat_idx: usize) -> QuatVal {
        debug_assert!(quat_idx < 32);
        let shift = quat_idx * 2;
        let bits = (chunk >> shift) & 0b11;
        match bits {
            0 => QuatVal::Q0,
            1 => QuatVal::Q1,
            2 => QuatVal::Q2,
            _ => QuatVal::Q3,
        }
    }
}