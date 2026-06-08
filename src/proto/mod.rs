//! Protocol port. M1 implements the read path (cmd 0x06 block) for 8k_nordic;
//! other variants are detected and reported Unsupported until hardware-verified.

pub mod block;
pub mod dpi;
pub mod polling;
pub mod sensor;
pub mod system;

use crate::hid::enumerate::USAGE_PAGE_CONFIG;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Variant {
    EightKNordic,
    Unsupported,
}

impl Variant {
    pub fn label(self) -> &'static str {
        match self {
            Variant::EightKNordic => "8k_nordic",
            Variant::Unsupported => "unsupported",
        }
    }
}

/// Runtime detection. The M6 / Ultra-Link 8K exposes config on usage 0xFFC1 and
/// is the only hardware-verified variant; everything else is Unsupported.
pub fn detect(usage_page: u16) -> Variant {
    if usage_page == USAGE_PAGE_CONFIG {
        Variant::EightKNordic
    } else {
        Variant::Unsupported
    }
}
