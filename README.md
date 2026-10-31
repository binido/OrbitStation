# OrbitStation
An open-source Linux application for configuring and managing Flydigi controllers and peripherals. A community-driven alternative to Flydigi Space Station, bringing device customization and control to Linux.

Native Rust + egui, no Electron. Supported: **Flydigi Vader 5 Pro** over USB cable.

## Features

- Device info: model, firmware, battery
- Switching between the 4 onboard profiles
- Live test of buttons, sticks, triggers, gyro and accelerometer, vibration test
- Settings: stick auto-calibration, debounce, rebound, precision, center sensitivity,
  Home as Xbox Guide, quick profile switch, sleep time and more

Button remapping, macros and RGB are not done yet. The protocol is in [docs/protocol.md](docs/protocol.md).

## Build and run

```bash
cargo run --release
```

## Permissions

The app talks to `/dev/hidraw*`. If it reports no access, install the udev rule and replug the gamepad:

```bash
sudo cp 70-orbitstation.rules /etc/udev/rules.d/ && sudo udevadm control --reload
```
