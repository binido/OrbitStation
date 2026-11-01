//! The HID bus: which hidraw nodes exist, how their vendor reports are laid out, and a two-thread
//! link to one of them.

use std::collections::BTreeMap;
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
/// Usage page of the vendor collection every Flydigi command interface declares.
const VENDOR_PAGE: u32 = 0xffa0;

/// Where packets go in a node's vendor collection: report ids (0 when the reports are unnumbered)
/// and the size of the output report, without its id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reports {
    pub output: u8,
    pub output_len: usize,
    pub input: u8,
}

/// A hidraw node and the USB identity of the interface behind it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub path: PathBuf,
    pub vendor: u16,
    pub product: u16,
    /// `None` when the node has no vendor collection, a keyboard or mouse interface of the device.
    pub reports: Option<Reports>,
}

/// Every hidraw node on the system, unreadable entries are skipped.
pub fn nodes() -> Vec<Node> {
    let Ok(dir) = fs::read_dir(SYSFS) else { return vec![] };
    dir.flatten()
        .filter_map(|e| {
            let uevent = fs::read_to_string(e.path().join("device/uevent")).ok()?;
            let (vendor, product) = parse_ids(&uevent)?;
            let descriptor = fs::read(e.path().join("device/report_descriptor")).unwrap_or_default();
            let path = Path::new("/dev").join(e.file_name());
            Some(Node { path, vendor, product, reports: vendor_reports(&descriptor) })
        })
        .collect()
}

/// Vendor and product id out of `HID_ID=0003:000037D7:00002401` in a uevent file.
fn parse_ids(uevent: &str) -> Option<(u16, u16)> {
    let mut id = uevent.lines().find_map(|l| l.strip_prefix("HID_ID="))?.split(':').skip(1);
    Some((u16::from_str_radix(id.next()?, 16).ok()?, u16::from_str_radix(id.next()?, 16).ok()?))
}

/// The largest output and input report of the vendor collection, read from the report descriptor's
/// short items. `None` when there is no vendor collection with both.
fn vendor_reports(descriptor: &[u8]) -> Option<Reports> {
    let (mut page, mut id, mut size, mut count) = (0u32, 0u8, 0u32, 0u32);
    let mut collections: Vec<bool> = vec![];
    let mut outputs: BTreeMap<u8, u32> = BTreeMap::new();
    let mut inputs: BTreeMap<u8, u32> = BTreeMap::new();
    let mut pos = 0;
    while let Some(&prefix) = descriptor.get(pos) {
        let len = [0, 1, 2, 4][usize::from(prefix & 3)];
        let data = descriptor.get(pos + 1..pos + 1 + len)?;
        let value = data.iter().rev().fold(0u32, |v, b| v << 8 | u32::from(*b));
        pos += 1 + len;
        let in_vendor = collections.iter().any(|v| *v);
        match (prefix >> 2 & 3, prefix >> 4) {
            // global items: usage page, report size, report id, report count
            (1, 0) => page = value,
            (1, 7) => size = value,
            (1, 8) => id = u8::try_from(value).ok()?,
            (1, 9) => count = value,
            // main items: input, output, collection, end collection
            (0, 8) if in_vendor => *inputs.entry(id).or_default() += size * count,
            (0, 9) if in_vendor => *outputs.entry(id).or_default() += size * count,
            (0, 10) => collections.push(page == VENDOR_PAGE),
            (0, 12) => {
                collections.pop();
            }
            _ => {}
        }
    }
    let largest = |m: &BTreeMap<u8, u32>| m.iter().max_by_key(|e| *e.1).map(|(id, bits)| (*id, *bits));
    let (output, bits) = largest(&outputs)?;
    let (input, _) = largest(&inputs)?;
    Some(Reports { output, output_len: usize::try_from(bits / 8).ok()?, input })
}

/// One packet as read from the node, without its report id.
pub type Packet = Vec<u8>;

/// An open hidraw node. Writes run on their own thread: a device that stops answering keeps a USB
/// write waiting in the kernel, and the window must not wait with it.
pub struct Link {
    reports: Reports,
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
    pub fn open(
        path: &Path,
        reports: Reports,
        mut report: impl FnMut(&[u8]) -> bool + Send + 'static,
    ) -> Result<Link, Error> {
        let access = |source| Error::Access { path: path.to_path_buf(), source };
        let file = OpenOptions::new().read(true).write(true).open(path).map_err(access)?;
        let mut reader = file.try_clone().map_err(access)?;
        let (tx, replies) = mpsc::channel();
        let alive = Arc::new(AtomicBool::new(true));
        let gone = alive.clone();
        thread::spawn(move || {
            let mut buf = [0u8; 65];
            while let Ok(n) = reader.read(&mut buf) {
                let packet = match reports.input {
                    0 => &buf[..n],
                    id if buf.first() == Some(&id) => &buf[1..n],
                    // another report of the interface, not the protocol's
                    _ => continue,
                };
                if !report(packet) && tx.send(packet.to_vec()).is_err() {
                    break;
                }
            }
            gone.store(false, Ordering::Relaxed);
        });
        let (writer, packets) = mpsc::sync_channel::<Vec<u8>>(4);
        thread::spawn(move || write_loop(file, &packets));
        Ok(Link { reports, writer, replies, alive })
    }

    /// False once the node went away.
    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Relaxed)
    }

    /// Drops stale replies and queues `packet` as one output report, cut or padded to its size.
    ///
    /// # Errors
    /// [`Error::Disconnected`] when the node is gone or the write queue is stuck.
    pub fn send(&self, packet: &[u8]) -> Result<(), Error> {
        while self.replies.try_recv().is_ok() {}
        let mut out = Vec::with_capacity(self.reports.output_len + 1);
        // hidraw takes the report id first, 0 for an unnumbered report
        out.push(self.reports.output);
        out.extend(packet.iter().copied().chain(std::iter::repeat(0)).take(self.reports.output_len));
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

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("test data is hex")).collect()
    }

    #[test]
    fn ids_come_from_the_uevent() {
        let uevent = "DRIVER=hid-generic\nHID_ID=0003:000037D7:00002401\nHID_NAME=Flydigi Vader 5 Pro\n";
        assert_eq!(parse_ids(uevent), Some((0x37d7, 0x2401)));
    }

    #[test]
    fn vader_5_pro_reports_are_unnumbered() {
        // read from a Vader 5 Pro's interface 1
        let d = hex("06a0ff0901a1010903150026ff007508952081020905150026ff00750895209102c0");
        assert_eq!(vendor_reports(&d), Some(Reports { output: 0, output_len: 32, input: 0 }));
    }

    #[test]
    fn apex_5_reports_use_ids_3_and_4() {
        // the Apex 5 layout openflydigi measured: output 3 and 8, input 4, 31 and 12 bytes
        let d = hex(concat!(
            "06a0ff0901a101", // vendor collection
            "8503090115",
            "0026ff007508951f9102", // report 3, 31-byte output
            "8504750895",
            "1f8102", // report 4, 31-byte input
            "8508750895",
            "0c9102c0", // report 8, 12-byte output
        ));
        assert_eq!(vendor_reports(&d), Some(Reports { output: 3, output_len: 31, input: 4 }));
    }

    #[test]
    fn a_mouse_interface_has_no_vendor_reports() {
        let d = hex("05010902a1010901a10005091901290315002501950375018102c0c0");
        assert_eq!(vendor_reports(&d), None);
    }
}
