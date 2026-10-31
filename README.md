<img src="assets/icons/orbitstation.svg" width="96" alt="OrbitStation icon">

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
- Trigger travel per profile: where LT/RT start registering and where they read fully pressed
- Grip and trigger vibration: on/off, strength, trigger threshold and amplitude window
- RGB lighting per profile: solid, multi-colour, breath, flow, rainbow, wave, flash; brightness, speed
- Profile names, saving profiles to files and loading them back, factory reset of one profile
- Gamepad nickname
- Gyro aiming done by the pad itself: tilt drives the left or right stick while an enable key is held or toggled
- Live test of buttons, sticks, triggers, gyro and accelerometer, vibration test
- Settings: stick auto-calibration, debounce, rebound, precision, center sensitivity,
  Home as Xbox Guide, quick profile switch, sleep time and more
- English and Russian interface: follows the system language until one is picked in Settings
- Interface scale in Settings (75-200% of the system scale), Ctrl + / Ctrl - / Ctrl 0 work too
- GTK colours: the libadwaita palette, the GNOME accent colour and `@define-color` overrides from
  `~/.config/gtk-4.0/gtk.css` (themes from Noctalia, matugen and the like are picked up, live); light or dark follows the system
- Space Grotesk font with Manrope for Cyrillic, both shipped inside the binary (OFL, licences in `assets/fonts`)

Before every write the previous
profile is copied to `~/.local/share/orbitstation/backups` (`.osp` files, loadable from the profile tabs).
Saved profiles go to `~/.local/share/orbitstation/profiles`. "Reset all profiles" is the pad's own
command and wipes all four slots, so the app backs them all up first. The protocol is in [docs/protocol.md](docs/protocol.md). Thanks to
[openflydigi](https://github.com/mkaliaha/openflydigi) for the profile blob research.

## Install

Works on any distribution. Needs Rust 1.95 or newer ([rustup](https://rustup.rs)), a C compiler and git.

```bash
curl -fsSL https://raw.githubusercontent.com/binido/OrbitStation/main/install.sh | sh
```

The program goes to `~/.local/bin/orbitstation` with a menu entry, no root needed. Only the udev rule
that lets your user open the gamepad asks for a password; skip it with `sh -s -- --no-udev`.
Run the same command again to update. To remove:

```bash
curl -fsSL https://raw.githubusercontent.com/binido/OrbitStation/main/install.sh | sh -s -- --uninstall
```

From a checkout, `sh install.sh` builds that checkout instead of downloading.
