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

    /// Reads a stored blob of `total` packets, as the pad reports it.
    fn read_blob(&mut self, cmd: u8, cfg: u8) -> Option<Vec<u8>> {
        (0..3).find_map(|_| {
            self.send(&proto::request(cmd, &[cfg, map::PKG as u8]))?;
            let (mut blob, mut got) = (vec![], 0);
            while let Some(r) = self.reply(cmd, Duration::from_millis(500)) {
                let (total, idx) = (r[3] as usize, r[4] as usize);
                if idx >= total || (got > 0 && blob.len() != total * map::PKG) {
                    return None;
                }
                blob.resize(total * map::PKG, 0);
                blob[idx * map::PKG..][..map::PKG].copy_from_slice(&r[6..6 + map::PKG]);
                got += 1;
                if got == total {
                    return Some(blob);
                }
            }
            None
        })
    }

    /// Sends the packets that differ from `old`: a start command with the span, then numbered packets.
    fn write_blob(&mut self, start_cmd: u8, pack_cmd: u8, cfg: u8, old: &[u8], new: &[u8]) -> Result<(), String> {
        let Some((start, count)) = map::changed_span(old, new) else { return Ok(()) };
        let head = [cfg, start as u8, count as u8, map::PKG as u8];
        self.call(proto::request(start_cmd, &head)).ok_or("геймпад не принял начало записи")?;
        for i in 0..count {
            let mut pack = [0u8; map::PKG + 1];
            pack[0] = i as u8;
            pack[1..].copy_from_slice(&new[(start + i) * map::PKG..][..map::PKG]);
            self.call(proto::request(pack_cmd, &pack)).ok_or_else(|| format!("геймпад не принял пакет {}", start + i))?;
        }
        Ok(())
    }

    /// Reads mapping and macros of a profile. The pad switches to that profile while answering.
    pub fn read_profile(&mut self, cfg: u8) -> Option<Profile> {
        let map = self.read_blob(cmd::MAP_READ, cfg).filter(|b| b.len() == map::SIZE)?;
        let macros = self.read_blob(cmd::MACRO_READ, cfg)?;
        Some(Profile { map, macros })
    }

    /// Writes what changed in the same order as Space Station (mapping, then macros), commits the
    /// running profile to flash and applies it, since macros only start playing after an apply.
    pub fn write_profile(&mut self, cfg: u8, old: &Profile, new: &Profile) -> Result<(), String> {
        self.write_blob(cmd::MAP_WRITE_START, cmd::MAP_WRITE_PACK, cfg, &old.map, &new.map)?;
        self.write_blob(cmd::MACRO_WRITE_START, cmd::MACRO_WRITE_PACK, cfg, &old.macros, &new.macros)?;
        // The SDK gives the flash write 10 s where every other command gets 500 ms.
        let version = map::version(&new.map).to_le_bytes();
        self.call_wait(proto::request(cmd::MAP_SAVE, &version), Duration::from_secs(10))
            .ok_or("геймпад не подтвердил сохранение")?;
        self.call(proto::request(cmd::APPLY_PROFILE, &[cfg])).ok_or("геймпад не применил профиль")?;
        Ok(())
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Profile {
    pub map: Vec<u8>,
    pub macros: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::macros;

    /// Needs a wired Vader 5 Pro: `cargo test -- --ignored`. Profile 4 is changed and restored.
    #[test]
    #[ignore]
    fn profile_roundtrip_on_hardware() {
        let mut d = Device::open().unwrap();
        let active = proto::parse_profile(&d.call(proto::request(cmd::PROFILE_INFO, &[])).unwrap());
        let orig = d.read_profile(3).unwrap();
        let mut new = orig.clone();
        map::set_key(&mut new.map, 18, map::Key { target: map::MACRO, turbo: 0, hz: 0 });
        map::set_key(&mut new.map, 4, map::Key { target: map::IDENTITY, turbo: 1, hz: 20 });
        let left = proto::stick::get(&orig.map, 0);
        proto::stick::set(&mut new.map, 0, proto::stick::Stick { kind: 1, center: 20, edge: 10, p1: (64, 96), ..left });
        map::set_version(&mut new.map, 0x1234);
        let tap = vec![macros::Step { delay_ms: 0, key: 7, event: 1 }, macros::Step { delay_ms: 40, key: 7, event: 0 }];
        new.macros = macros::encode(&orig.macros, &[macros::Macro { key: 18, kind: 1, interval_ms: 0, steps: tap }]).unwrap();
        let check = |d: &mut Device, want: &Profile| -> Result<(), String> {
            let back = d.read_profile(3).ok_or("no readback")?;
            let diff = |a: &[u8], b: &[u8]| -> Vec<(usize, u8, u8)> {
                a.iter().zip(b).enumerate().filter(|(_, (x, y))| x != y).map(|(i, (x, y))| (i, *x, *y)).collect()
            };
            match (diff(&want.map, &back.map), diff(&want.macros, &back.macros)) {
                (m, k) if m.is_empty() && k.is_empty() && back.macros.len() == want.macros.len() => Ok(()),
                (m, k) => Err(format!("readback differs (index, wanted, read): map {m:?} macros {k:?} len {}", back.macros.len())),
            }
        };
        let written = d.write_profile(3, &orig, &new).and_then(|_| check(&mut d, &new));
        let restored = d.write_profile(3, &new, &orig).and_then(|_| check(&mut d, &orig));
        d.call(proto::request(cmd::APPLY_PROFILE, &[active])).unwrap();
        written.unwrap();
        restored.unwrap();
    }
}
