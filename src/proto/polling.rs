//! Polling-rate code <-> Hz. Set path is M2; M1 needs only the read mapping.
//! Launcher "Levels: 6": 0=125 1=500 2=1000 3=2000 4=4000 5=8000 (no 250).

pub const RATES_HZ: [u32; 6] = [125, 500, 1000, 2000, 4000, 8000];

pub fn hz_from_code(code: u8) -> Option<u32> {
    RATES_HZ.get(code as usize).copied()
}
