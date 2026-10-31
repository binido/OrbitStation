# OrbitStation
An open-source Linux application for configuring and managing Flydigi controllers and peripherals. A community-driven alternative to Flydigi Space Station, bringing device customization and control to Linux.

Native Rust + egui, no Electron. Supported: **Flydigi Vader 5 Pro** over USB cable.

## Features

- Device info: model, firmware, battery
- Switching between the 4 onboard profiles
- Button remapping per profile, including the back paddles M1-M4, LM/RM and C/Z
- Turbo per button (while held or toggled, 1-30 presses per second)
- Macros: up to 10 per profile, played once, while held or toggled
- Stick curves per profile: presets or custom points, inner and outer dead zones, circular range
- Live test of buttons, sticks, triggers, gyro and accelerometer, vibration test
- Settings: stick auto-calibration, debounce, rebound, precision, center sensitivity,
  Home as Xbox Guide, quick profile switch, sleep time and more

Trigger curves, gyro mapping and RGB are not done yet. Before every write the previous
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
