//! The HID bus: which hidraw nodes exist and a two-thread link to one of them.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread;
use std::time::{Duration, Instant};

use super::Error;

const SYSFS: &str = "/sys/class/hidraw";

/// A hidraw node and the USB identity of the interface behind it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub path: PathBuf,
    pub vendor: u16,
    pub product: u16,
    pub interface: u8,
}

/// Every hidraw node on the system, unreadable entries are skipped.
pub fn nodes() -> Vec<Node> {
    let Ok(dir) = fs::read_dir(SYSFS) else { return vec![] };
    dir.flatten()
        .filter_map(|e| {
            let uevent = fs::read_to_string(e.path().join("device/uevent")).ok()?;
            parse_uevent(&Path::new("/dev").join(e.file_name()), &uevent)
        })
        .collect()
}

/// Reads `HID_ID=0003:000037D7:00002401` and `HID_PHYS=usb-…/input1` out of a uevent file.
fn parse_uevent(path: &Path, uevent: &str) -> Option<Node> {
    let field = |key: &str| uevent.lines().find_map(|l| l.strip_prefix(key));
    let mut id = field("HID_ID=")?.split(':').skip(1);
    let vendor = u16::from_str_radix(id.next()?, 16).ok()?;
    let product = u16::from_str_radix(id.next()?, 16).ok()?;
    let interface = field("HID_PHYS=")?.rsplit_once("/input")?.1.parse().ok()?;
    Some(Node { path: path.to_path_buf(), vendor, product, interface })
}

/// One packet as read from the node.
pub type Packet = Vec<u8>;

/// An open hidraw node. Writes run on their own thread: a device that stops answering keeps a USB
/// write waiting in the kernel, and the window must not wait with it.
pub struct Link {
    writer: SyncSender<Vec<u8>>,
    replies: Receiver<Packet>,
    alive: Arc<AtomicBool>,
}

impl Link {
    /// Opens `path`. Every incoming packet goes to `report` first, packets it does not take
    /// (returns `false`) are queued as replies.
    ///
    /// # Errors
    /// [`Error::Access`] when the node cannot be opened, usually for a missing udev rule.
    pub fn open(path: &Path, mut report: impl FnMut(&[u8]) -> bool + Send + 'static) -> Result<Link, Error> {
        let access = |source| Error::Access { path: path.to_path_buf(), source };
        let file = OpenOptions::new().read(true).write(true).open(path).map_err(access)?;
        let mut reader = file.try_clone().map_err(access)?;
        let (tx, replies) = mpsc::channel();
        let alive = Arc::new(AtomicBool::new(true));
        let gone = alive.clone();
        thread::spawn(move || {
            let mut buf = [0u8; 64];
            while let Ok(n) = reader.read(&mut buf) {
                if !report(&buf[..n]) && tx.send(buf[..n].to_vec()).is_err() {
                    break;
                }
            }
            gone.store(false, Ordering::Relaxed);
        });
        let (writer, packets) = mpsc::sync_channel::<Vec<u8>>(4);
        thread::spawn(move || write_loop(file, &packets));
        Ok(Link { writer, replies, alive })
    }

    /// False once the node went away.
    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Relaxed)
    }

    /// Drops stale replies and queues `packet` behind a 0 report id, this protocol has none.
    ///
    /// # Errors
    /// [`Error::Disconnected`] when the node is gone or the write queue is stuck.
    pub fn send(&self, packet: &[u8]) -> Result<(), Error> {
        while self.replies.try_recv().is_ok() {}
        let mut out = Vec::with_capacity(packet.len() + 1);
        out.push(0);
        out.extend_from_slice(packet);
        self.writer.try_send(out).map_err(|_| Error::Disconnected)
    }

    /// The next reply `accept` takes within `wait`, others are skipped.
    pub fn reply(&self, wait: Duration, accept: impl Fn(&[u8]) -> bool) -> Option<Packet> {
        let deadline = Instant::now() + wait;
        while let Some(left) = deadline.checked_duration_since(Instant::now()) {
            match self.replies.recv_timeout(left) {
                Ok(p) if accept(&p) => return Some(p),
                Ok(_) => {}
                Err(_) => break,
            }
        }
        None
    }
}

fn write_loop(mut file: File, packets: &Receiver<Vec<u8>>) {
    while let Ok(out) = packets.recv() {
        if file.write_all(&out).is_err() {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uevent_of_the_vader_config_interface_is_parsed() {
        let uevent = "DRIVER=hid-generic\nHID_ID=0003:000037D7:00002401\nHID_NAME=Flydigi Vader 5 Pro\n\
                      HID_PHYS=usb-0000:00:14.0-7/input1\nHID_UNIQ=\n";
        let node = parse_uevent(Path::new("/dev/hidraw2"), uevent);
        let want = Node { path: PathBuf::from("/dev/hidraw2"), vendor: 0x37d7, product: 0x2401, interface: 1 };
        assert_eq!(node, Some(want));
    }

    #[test]
    fn uevent_without_a_usb_interface_is_skipped() {
        let uevent = "HID_ID=0005:0000046D:0000B023\nHID_PHYS=00:1a:7d:da:71:13\n";
        assert_eq!(parse_uevent(Path::new("/dev/hidraw9"), uevent), None);
    }
}
