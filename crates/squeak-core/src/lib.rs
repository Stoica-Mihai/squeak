//! squeak-core: Keychron mouse HID protocol, transport, and device worker.
//! Frontend-agnostic — shared by the TUI and the desktop app.

pub mod hid;
pub mod proto;
pub mod update;
pub mod watch;
pub mod worker;
