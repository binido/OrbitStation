<img src="assets/icons/orbitstation.svg" width="96" alt="OrbitStation icon">

# OrbitStation
An open-source Linux application for configuring and managing Flydigi controllers and peripherals. A community-driven alternative to Flydigi Space Station, bringing device customization and control to Linux.

Native Rust + egui, no Electron. Supported: **Flydigi Vader 5 Pro** over USB cable.

## Supported devices

Several devices can be connected at once; each gets the pages it supports.

| Device | Status | What works |
|---|---|---|
| Flydigi Vader 5 Pro | Tested on hardware | Everything below |
| Flydigi Apex 5 | Not tested here, from openflydigi's measurements | Info, name, profiles, buttons, sticks, triggers, gyro, grip vibration, lighting, settings, live test, macros (up to five, inside the profile, 10 ms steps). Force triggers and the screen are not driven, no factory profile |
| Flydigi Apex 6 / 6 Pro | Not tested | Info only: it speaks a newer dialect for the rest |
| Flydigi CD2 charging dock | Not tested here, from openflydigi's measurements | Info, name, the four switches, whether a controller is seated and its charge. Dock lighting is not driven yet |

Older Flydigi controllers (Vader 3 and 4, Apex 3 and 4, Direwolf) show up on Linux as a plain
Xbox controller without Flydigi's vendor interface, so they cannot be configured from here.

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
[openflydigi](https://github.com/mkaliaha/openflydigi) for the profile blob research and the Apex 5, Apex 6
and CD2 dock facts.

## Adding a device

The code is split so a new device is mostly data:

- `src/station` knows nothing about Flydigi: the `Device` trait with optional capabilities
  (`ProfileStore`, `HardwareSettings`, `LiveInput`), the registry of connected devices and HID discovery.
- `src/flydigi` speaks Space Station's protocol and implements `Device` for every model of that family.
- `src/models` describes each model: the id it reports, its buttons and which profile parts it has.
- `src/app` is the window; a device gets the pages its capabilities and features allow.

A new model that speaks the same protocol is a new file in `src/models` and a line in `models::PADS`
or `models::DOCKS`.
A device with a protocol of its own implements `station::Device` and adds a `Family` to `station::FAMILIES`.

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
