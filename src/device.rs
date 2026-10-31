use crate::proto::{self, PKT, cmd, map};
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
        self.call_wait(pkt, Duration::from_millis(500))
    }

    fn send(&mut self, pkt: &[u8; PKT]) -> Option<()> {
        while self.replies.try_recv().is_ok() {}
        // hidraw needs a leading report id, this interface has none so it is 0.
        let mut out = [0u8; PKT + 1];
        out[1..].copy_from_slice(pkt);
        self.writer.write_all(&out).ok()
    }

    fn reply(&self, cmd: u8, wait: Duration) -> Option<[u8; PKT]> {
        let deadline = Instant::now() + wait;
        while let Some(left) = deadline.checked_duration_since(Instant::now()) {
            match self.replies.recv_timeout(left) {
                Ok(r) if proto::is_reply(&r, cmd) => return Some(r),
                Ok(_) => {}
                Err(_) => break,
            }
        }
        None
    }

    /// A bad packet is answered with silence, so a missing reply is retried.
    fn call_wait(&mut self, pkt: [u8; PKT], wait: Duration) -> Option<[u8; PKT]> {
        (0..3).find_map(|_| {
            self.send(&pkt)?;
            self.reply(pkt[2], wait)
        })
    }

    /// Reads a profile's mapping blob. The pad switches to that profile while answering.
    pub fn read_profile(&mut self, cfg: u8) -> Option<Vec<u8>> {
        (0..3).find_map(|_| {
            self.send(&proto::request(cmd::MAP_READ, &[cfg, map::PKG as u8]))?;
            let mut blob = vec![0u8; map::SIZE];
            let mut got = 0;
            while let Some(r) = self.reply(cmd::MAP_READ, Duration::from_millis(500)) {
                let (total, idx) = (r[3] as usize, r[4] as usize);
                if total * map::PKG != map::SIZE || idx >= total {
                    return None;
                }
                blob[idx * map::PKG..(idx + 1) * map::PKG].copy_from_slice(&r[6..6 + map::PKG]);
                got += 1;
                if got == total {
                    return Some(blob);
                }
            }
            None
        })
    }

    /// Sends the packets that differ from `old` and commits the running profile to flash.
    pub fn write_profile(&mut self, cfg: u8, old: &[u8], new: &[u8], version: u16) -> Result<(), String> {
        let Some((start, count)) = map::changed_span(old, new) else { return Ok(()) };
        let head = [cfg, start as u8, count as u8, map::PKG as u8];
        self.call(proto::request(cmd::MAP_WRITE_START, &head)).ok_or("геймпад не принял начало записи")?;
        for i in 0..count {
            let mut pack = [0u8; map::PKG + 1];
            pack[0] = i as u8;
            pack[1..].copy_from_slice(&new[(start + i) * map::PKG..(start + i + 1) * map::PKG]);
            self.call(proto::request(cmd::MAP_WRITE_PACK, &pack))
                .ok_or_else(|| format!("геймпад не принял пакет {}", start + i))?;
        }
        // The SDK gives the flash write 10 s where every other command gets 500 ms.
        self.call_wait(proto::request(cmd::MAP_SAVE, &version.to_le_bytes()), Duration::from_secs(10))
            .ok_or("геймпад не подтвердил сохранение")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Needs a wired Vader 5 Pro: `cargo test -- --ignored`. Profile 4 is changed and restored.
    #[test]
    #[ignore]
    fn remap_roundtrip_on_hardware() {
        let mut d = Device::open().unwrap();
        let active = proto::parse_profile(&d.call(proto::request(cmd::PROFILE_INFO, &[])).unwrap());
        let orig = d.read_profile(3).unwrap();
        let mut new = orig.clone();
        map::set_target(&mut new, 18, if map::target(&orig, 18) == 4 { 5 } else { 4 });
        map::set_version(&mut new, 0x1234);
        let result = d.write_profile(3, &orig, &new, 0x1234).and_then(|_| {
            (d.read_profile(3).as_ref() == Some(&new)).then_some(()).ok_or("readback differs".into())
        });
        d.write_profile(3, &new, &orig, map::version(&orig)).unwrap();
        assert_eq!(d.read_profile(3), Some(orig));
        d.call(proto::request(cmd::APPLY_PROFILE, &[active])).unwrap();
        result.unwrap();
    }
}
