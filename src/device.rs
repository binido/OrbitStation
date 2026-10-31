use crate::proto::{self, PKT};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const HID_ID: &str = "HID_ID=0003:000037D7:00002401";
/// Vendor config interface, the one with `5a a5` commands.
const CONFIG_IFACE: &str = "input1";

pub fn find_hidraw() -> Option<PathBuf> {
    let mut found: Vec<_> = fs::read_dir("/sys/class/hidraw").ok()?.flatten().collect();
    found.sort_by_key(|e| e.file_name());
    found.into_iter().find_map(|e| {
        let uevent = fs::read_to_string(e.path().join("device/uevent")).ok()?;
        let phys = uevent.lines().find_map(|l| l.strip_prefix("HID_PHYS="))?;
        (uevent.lines().any(|l| l == HID_ID) && phys.ends_with(CONFIG_IFACE))
            .then(|| PathBuf::from("/dev").join(e.file_name()))
    })
}

pub struct Device {
    pub path: PathBuf,
    writer: File,
    replies: Receiver<[u8; PKT]>,
    pub input: Arc<Mutex<Option<proto::Input>>>,
    alive: Arc<AtomicBool>,
}

impl Device {
    pub fn open() -> Result<Device, String> {
        let path = find_hidraw().ok_or("Геймпад Vader 5 Pro не найден (подключи его кабелем)")?;
        let writer = OpenOptions::new().read(true).write(true).open(&path).map_err(|e| {
            format!("Нет доступа к {}: {e}. Установи udev-правило из README.", path.display())
        })?;
        let mut reader = writer.try_clone().map_err(|e| e.to_string())?;
        let (tx, replies) = mpsc::channel();
        let input = Arc::new(Mutex::new(None));
        let alive = Arc::new(AtomicBool::new(true));
        let (input2, alive2) = (input.clone(), alive.clone());
        thread::spawn(move || {
            let mut buf = [0u8; 64];
            while let Ok(n) = reader.read(&mut buf) {
                if let Some(i) = proto::parse_input(&buf[..n]) {
                    *input2.lock().unwrap() = Some(i);
                } else if n >= PKT && tx.send(buf[..PKT].try_into().unwrap()).is_err() {
                    break;
                }
            }
            alive2.store(false, Ordering::Relaxed);
        });
        Ok(Device { path, writer, replies, input, alive })
    }

    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Relaxed)
    }

    /// Sends a request and waits for the reply with the same command id.
    pub fn call(&mut self, pkt: [u8; PKT]) -> Option<[u8; PKT]> {
        while self.replies.try_recv().is_ok() {}
        // hidraw needs a leading report id, this interface has none so it is 0.
        let mut out = [0u8; PKT + 1];
        out[1..].copy_from_slice(&pkt);
        for _ in 0..3 {
            self.writer.write_all(&out).ok()?;
            let deadline = Instant::now() + Duration::from_millis(500);
            while let Some(left) = deadline.checked_duration_since(Instant::now()) {
                match self.replies.recv_timeout(left) {
                    Ok(r) if proto::is_reply(&r, pkt[2]) => return Some(r),
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
        }
        None
    }
}
