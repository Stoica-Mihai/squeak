//! Raw hidraw transport for the Keychron config collection.

pub mod device;
pub mod enumerate;

pub use device::Device;
pub use enumerate::{DeviceInfo, find_config};

#[derive(thiserror::Error, Debug)]
pub enum HidError {
    #[error("open {0}: {1}")]
    Open(String, #[source] std::io::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("short write ({0} of {1} bytes)")]
    ShortWrite(usize, usize),
    #[error("poll: {0}")]
    Poll(#[source] rustix::io::Errno),
    #[error("timeout waiting for report 0x{0:02x}")]
    Timeout(u8),
    #[error("bad reply: {0}")]
    BadReply(String),
}
