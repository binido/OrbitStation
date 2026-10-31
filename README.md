# OrbitStation
An open-source Linux application for configuring and managing Flydigi controllers and peripherals. A community-driven alternative to Flydigi Space Station, bringing device customization and control to Linux.

Native Rust + egui, no Electron. Supported: **Flydigi Vader 5 Pro** over USB cable.

## Features

- Device info: model, firmware, battery
- Switching between the 4 onboard profiles
- Button remapping per profile, including the back paddles M1-M4, LM/RM and C/Z
- Live test of buttons, sticks, triggers, gyro and accelerometer, vibration test
- Settings: stick auto-calibration, debounce, rebound, precision, center sensitivity,
  Home as Xbox Guide, quick profile switch, sleep time and more

Macros, turbo, stick curves and RGB are not done yet. Before every remap write the previous
profile is copied to `~/.local/share/orbitstation/backups`. The protocol is in [docs/protocol.md](docs/protocol.md). Thanks to
[openflydigi](https://github.com/mkaliaha/openflydigi) for the profile blob research.

## Build and run

```bash
cargo run --release
```

## Permissions

The app talks to `/dev/hidraw*`. If it reports no access, install the udev rule and replug the gamepad:

```bash
sudo cp 70-orbitstation.rules /etc/udev/rules.d/ && sudo udevadm control --reload
```
