//! Hotplug watcher. Listens on a `NETLINK_KOBJECT_UEVENT` socket (the source
//! udev itself reads) and pushes a refresh when a Keychron HID node is added or
//! removed — so the UI reacts to plug/unplug without ever polling the device.
//! recv() blocks until the kernel sends an event; zero HID traffic until then.

use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::{Duration, Instant};

use crate::worker::Cmd;

/// VID 0x3434 as it appears in uevent payloads (HID_ID / MODALIAS).
const KEYCHRON_VID: &[u8] = b"00003434";
/// Coalesce the burst of events one plug emits.
const DEBOUNCE: Duration = Duration::from_millis(250);
/// Let the just-added node settle before the first read.
const SETTLE: Duration = Duration::from_millis(200);

/// Spawn the watcher. Silently does nothing if the socket can't be opened
/// (manual refresh still works); never touches the device itself.
pub fn spawn(cmd_tx: Sender<Cmd>) {
    thread::spawn(move || {
        let _ = run(&cmd_tx);
    });
}

/// Open + bind a `NETLINK_KOBJECT_UEVENT` socket on the kernel multicast group.
fn open_uevent_socket() -> std::io::Result<OwnedFd> {
    // SAFETY: libc socket/bind, return values checked.
    let fd = unsafe {
        libc::socket(
            libc::AF_NETLINK,
            libc::SOCK_DGRAM | libc::SOCK_CLOEXEC,
            libc::NETLINK_KOBJECT_UEVENT,
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: fresh fd from socket(), owned by nobody else; OwnedFd closes it on
    // every exit path from here, including the bind error below.
    let sock = unsafe { OwnedFd::from_raw_fd(fd) };
    let mut addr: libc::sockaddr_nl = unsafe { std::mem::zeroed() };
    addr.nl_family = libc::AF_NETLINK as u16;
    addr.nl_groups = 1; // kernel uevent multicast group
    let rc = unsafe {
        libc::bind(
            sock.as_raw_fd(),
            &addr as *const libc::sockaddr_nl as *const libc::sockaddr,
            std::mem::size_of::<libc::sockaddr_nl>() as libc::socklen_t,
        )
    };
    if rc < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(sock)
}

fn run(cmd_tx: &Sender<Cmd>) -> std::io::Result<()> {
    let sock = open_uevent_socket()?;
    let mut buf = [0u8; 8192];
    let mut last: Option<Instant> = None;
    loop {
        let n = unsafe {
            libc::recv(sock.as_raw_fd(), buf.as_mut_ptr() as *mut libc::c_void, buf.len(), 0)
        };
        // Retrying an unrecoverable error (EBADF, ENOTCONN) would spin this
        // thread at 100% CPU forever, so only signals/would-block are retried.
        if n < 0 {
            let err = std::io::Error::last_os_error();
            if matches!(err.raw_os_error(), Some(libc::EINTR) | Some(libc::EAGAIN)) {
                continue;
            }
            return Err(err);
        }
        if n == 0 {
            continue;
        }
        let msg = &buf[..n as usize];
        let keychron = msg.windows(KEYCHRON_VID.len()).any(|w| w == KEYCHRON_VID);
        if keychron && last.is_none_or(|t| t.elapsed() > DEBOUNCE) {
            last = Some(Instant::now());
            thread::sleep(SETTLE);
            // Worker's connect probe finds the live transport; reconnect+retry
            // covers a node that isn't quite ready yet.
            let _ = cmd_tx.send(Cmd::ReadAll);
            let _ = cmd_tx.send(Cmd::ReadButtons);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The uevent socket opens + binds for the running user (the part that can
    /// fail on a locked-down system). Receiving real events needs a plug event,
    /// which can't be synthesized without root, so that's left to live use.
    #[test]
    #[ignore = "opens a NETLINK_KOBJECT_UEVENT socket"]
    fn uevent_socket_binds() {
        let _sock = open_uevent_socket().expect("bind NETLINK_KOBJECT_UEVENT group");
    }
}
