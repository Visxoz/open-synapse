//! Device arrival and removal, from udev's netlink broadcasts. They come
//! once udev's rules have run, the one granting access included, where the
//! kernel's own come before.

use std::{
    io, mem,
    net::Shutdown,
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::net::UnixStream,
    },
    thread::{self, JoinHandle},
};

use razer_protocol::VENDOR_ID;

use super::hid::ids;

/// The multicast group udev broadcasts on; the kernel's is 1.
const UDEV_GROUP: u32 = 2;

/// Calls back whenever a Razer hidraw node comes or goes, from a thread of
/// its own, until dropped.
pub struct Watcher {
    /// Shut down to wake the thread and end it.
    stop: UnixStream,
    thread: Option<JoinHandle<()>>,
}

impl Watcher {
    pub fn new(on_change: impl Fn() + Send + Sync + 'static) -> io::Result<Self> {
        let socket = udev_socket()?;
        let (stop, stopped) = UnixStream::pair()?;
        let thread = thread::Builder::new()
            .name("hot-plug".into())
            .spawn(move || listen(&socket, &stopped, &on_change))?;
        Ok(Self {
            stop,
            thread: Some(thread),
        })
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        let _ = self.stop.shutdown(Shutdown::Both);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn udev_socket() -> io::Result<OwnedFd> {
    // SAFETY: plain arguments; the result is checked.
    let fd = unsafe {
        libc::socket(
            libc::AF_NETLINK,
            libc::SOCK_DGRAM | libc::SOCK_CLOEXEC,
            libc::NETLINK_KOBJECT_UEVENT,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `fd` is a new socket that nothing else owns.
    let socket = unsafe { OwnedFd::from_raw_fd(fd) };
    // SAFETY: an all-zero `sockaddr_nl` is valid; it has private padding,
    // so it cannot be built field by field.
    let mut address: libc::sockaddr_nl = unsafe { mem::zeroed() };
    address.nl_family = libc::AF_NETLINK as libc::sa_family_t;
    address.nl_groups = UDEV_GROUP;
    // SAFETY: `address` is a `sockaddr_nl` of the size given.
    let bound = unsafe {
        libc::bind(
            socket.as_raw_fd(),
            (&raw const address).cast(),
            mem::size_of::<libc::sockaddr_nl>() as libc::socklen_t,
        )
    };
    if bound < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(socket)
}

/// Waits for messages until `stopped` wakes it.
fn listen(socket: &OwnedFd, stopped: &UnixStream, on_change: &dyn Fn()) {
    let mut message = vec![0u8; 8192];
    loop {
        let mut fds = [socket.as_raw_fd(), stopped.as_raw_fd()].map(|fd| libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        });
        // SAFETY: `fds` holds the two entries given.
        if unsafe { libc::poll(fds.as_mut_ptr(), 2, -1) } < 0 {
            if io::Error::last_os_error().kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return;
        }
        if fds[1].revents != 0 {
            return;
        }
        // SAFETY: `message` is valid for writes of its length.
        let len = unsafe {
            libc::recv(
                socket.as_raw_fd(),
                message.as_mut_ptr().cast(),
                message.len(),
                libc::MSG_DONTWAIT,
            )
        };
        match usize::try_from(len) {
            Ok(len) => {
                if announces(&message[..len], VENDOR_ID) {
                    on_change();
                }
            }
            Err(_) => match io::Error::last_os_error().raw_os_error() {
                // Messages were dropped while the queue was full; any may have mattered.
                Some(libc::ENOBUFS) => on_change(),
                Some(libc::EAGAIN | libc::EINTR) => {}
                _ => return,
            },
        }
    }
}

/// Whether a udev message announces a hidraw node of vendor `vid` coming or
/// going. Its properties follow a `libudev` header that says where they lie.
fn announces(message: &[u8], vid: u16) -> bool {
    if !message.starts_with(b"libudev\0") {
        return false;
    }
    let word = |at: usize| Some(u32::from_ne_bytes(message.get(at..at + 4)?.try_into().ok()?) as usize);
    let Some(properties) = word(16)
        .zip(word(20))
        .and_then(|(at, len)| message.get(at..at.checked_add(len)?))
    else {
        return false;
    };
    let (mut action, mut subsystem, mut devpath) = (None, None, None);
    for property in properties.split(|&b| b == 0) {
        let Some((key, value)) = std::str::from_utf8(property).ok().and_then(|p| p.split_once('=')) else {
            continue;
        };
        match key {
            "ACTION" => action = Some(value),
            "SUBSYSTEM" => subsystem = Some(value),
            "DEVPATH" => devpath = Some(value),
            _ => {}
        }
    }
    matches!(action, Some("add" | "remove"))
        && subsystem == Some("hidraw")
        && devpath.is_some_and(|p| vendor(p) == Some(vid))
}

/// The vendor in a hidraw node's path, from its HID device's name, as in
/// `/devices/…/0003:1532:00E6.0005/hidraw/hidraw3`.
fn vendor(devpath: &str) -> Option<u16> {
    ids(devpath.rsplit('/').nth(2)?).map(|(vid, _)| vid)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(properties: &str) -> Vec<u8> {
        let mut header = [0u8; 40];
        header[..8].copy_from_slice(b"libudev\0");
        header[16..20].copy_from_slice(&40u32.to_ne_bytes());
        header[20..24].copy_from_slice(&(properties.len() as u32).to_ne_bytes());
        [&header[..], properties.as_bytes()].concat()
    }

    const ADDED: &str = "ACTION=add\0DEVPATH=/devices/pci0000:00/0000:00:14.0/usb1/1-2/1-2:1.0/0003:1532:00E6.0005/hidraw/hidraw3\0SUBSYSTEM=hidraw\0DEVNAME=/dev/hidraw3\0";

    #[test]
    fn hears_a_razer_node_come() {
        assert!(announces(&message(ADDED), 0x1532));
        assert!(announces(&message(&ADDED.replace("add", "remove")), 0x1532));
    }

    #[test]
    fn ignores_everything_else() {
        assert!(!announces(&message(ADDED), 0x046D), "another vendor");
        assert!(!announces(&message(&ADDED.replace("add", "change")), 0x1532));
        assert!(!announces(&message(&ADDED.replace("=hidraw\0", "=input\0")), 0x1532));
        assert!(!announces(ADDED.as_bytes(), 0x1532), "no udev header");
        assert!(!announces(&message(ADDED)[..30], 0x1532), "cut short");
    }
}
