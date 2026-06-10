//! Raw hidraw transport for the Keychron config collection.

pub mod device;
pub mod enumerate;

pub use device::Device;
pub use enumerate::{DeviceInfo, find_config};

/// Keychron command transport: long (GET) and short (SET) channels. Abstracted
/// so the protocol layer can run against a fake device in tests.
pub trait Hid {
    /// Long-channel READ: `0xB3[cmd, payload…]` -> `0xB4` reply (incl. report id).
    fn get(&mut self, cmd: u8, payload: &[u8]) -> Result<Vec<u8>, HidError>;
    /// Short-channel WRITE: `0xB5[cmd, payload…]` -> `0xB6` ack. Returns (ok, reply).
    fn set(&mut self, cmd: u8, payload: &[u8]) -> Result<(bool, Vec<u8>), HidError>;
    /// WRITE on the long channel (`0xB3`), ack on short (`0xB6` = `E4 00 cmd`).
    fn long_set(&mut self, cmd: u8, payload: &[u8]) -> Result<(bool, Vec<u8>), HidError>;
    /// WRITE an arbitrary payload on the long channel, return the short reply.
    fn long_raw(&mut self, payload: &[u8]) -> Result<Vec<u8>, HidError>;
}

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
