<p align="center">
  <img src="assets/icons/orbitstation.svg" width="128" height="128" alt="OrbitStation icon">
</p>

<h1 align="center">OrbitStation</h1>

<p align="center">
  <b>Flydigi controllers on Linux; an open-source, community-driven alternative to Flydigi Space Station.</b><br>
  Profiles, remapping, macros, sticks, triggers, gyro, vibration, lighting and the CD2 dock,<br>
  in a native app written in Rust. No Electron, no Windows, no account.
</p>

<p align="center">
  <img alt="Rust" src="https://img.shields.io/badge/100%25-Rust-b7410e?style=flat-square&logo=rust">
  <img alt="Linux" src="https://img.shields.io/badge/Linux-native-2f7bf5?style=flat-square&logo=linux&logoColor=white">
  <img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-3a3a3a?style=flat-square">
  <img alt="Status: early alpha" src="https://img.shields.io/badge/status-early%20alpha-d69e2e?style=flat-square">
</p>

<p align="center">
  <a href="#supported-devices">Devices</a> ·
  <a href="#what-is-supported">What is supported</a> ·
  <a href="#features">Features</a> ·
  <a href="#under-the-hood">Under the hood</a> ·
  <a href="#install">Install</a> ·
  <a href="#contributing">Contributing</a>
</p>

<br>

<table>
  <tr>
    <td width="25%" valign="top">
      <h3>Space Station's protocol</h3>
      Recovered from Space Station itself and checked on a real Vader 5 Pro. The pad stores everything, so settings keep working on any machine.
    </td>
    <td width="25%" valign="top">
      <h3>Native and small</h3>
      Rust and egui, one binary, fonts inside it. Starts instantly and lives in <code>~/.local</code>, no root needed.
    </td>
    <td width="25%" valign="top">
      <h3>Safe to experiment</h3>
      Every profile is backed up before each write, and any backup loads back in one click.
    </td>
    <td width="25%" valign="top">
      <h3>Fits your desktop</h3>
      GTK and libadwaita colours, the GNOME accent, live theme changes, English and Russian.
    </td>
  </tr>
</table>

<br>

## Supported devices

Several devices can be connected at once; each gets the pages it supports.

| Device | Status | Connection |
|---|---|---|
| Flydigi Vader 5 Pro | Tested on hardware | USB cable |
| Flydigi Apex 5 | Built from [openflydigi](https://github.com/mkaliaha/openflydigi)'s measurements, not tested here | USB cable |
| Flydigi CD2 charging dock | Built from openflydigi's measurements, not tested here | USB cable |
| Flydigi Apex 6 / 6 Pro | Info only: it speaks a newer dialect for the rest | USB cable |

Older Flydigi controllers (Vader 3 and 4, Apex 3 and 4, Direwolf) show up on Linux as a plain
Xbox controller without Flydigi's vendor interface, so they cannot be configured from here.

Own an Apex 5 or a CD2? A report that it works, or doesn't, is the most useful contribution right now:
see [CONTRIBUTING.md](CONTRIBUTING.md#testing-on-hardware).

## What is supported

| | Vader 5 Pro | Apex 5 | CD2 dock | Apex 6 / 6 Pro |
|---|:-:|:-:|:-:|:-:|
| Device info: model, firmware, battery | Yes | Yes | Yes | Yes |
| Nickname | Yes | Yes | Yes | |
| 4 onboard profiles, switching, names, files | Yes | Yes | | |
| Factory reset of one profile | Yes | | | |
| Button remapping and turbo | Yes | Yes | | |
| Macros | Up to 10 | Up to 5, 10 ms steps | | |
| Stick curves and dead zones | Yes | Yes | | |
| Trigger travel | Yes | Yes | | |
| Gyro aiming on the pad | Yes | Yes | | |
| Grip vibration | Yes | Yes | | |
| Trigger motors | Yes | | | |
| Force-trigger effects | | Yes | | |
| RGB lighting | Yes | Yes | 8 effects, a picture or GIF | |
| Hardware settings | Yes | Yes | 4 switches | |
| Screen settings | | Yes | | |
| Live input test | Yes | Yes | | |
| Seated controller and its charge | | | Yes | |

Not supported yet, in the order it is planned: per-game profiles, a virtual DualSense,
DualSense audio haptics to the grip motors, the Apex 6 dialect. Details, and what is not planned
(Apex 5 screen pictures, firmware updates) and why: [docs/porting-openflydigi.md](docs/porting-openflydigi.md).

## Features

<table>
  <tr>
    <td width="50%" valign="top">
      <h3>Profiles</h3>
      Four onboard profiles, switched from the app. Name them, save them to <code>.osp</code> files and load them back.
      Before every write the previous profile is copied to <code>~/.local/share/orbitstation/backups</code>;
      "Reset all profiles" backs up all four slots first.
    </td>
    <td width="50%" valign="top">
      <h3>Buttons and macros</h3>
      Remap any button per profile, including the back paddles M1-M4, LM/RM and C/Z.
      Turbo while held or toggled, 1-30 presses per second. Macros played once, while held or toggled.
    </td>
  </tr>
  <tr>
    <td width="50%" valign="top">
      <h3>Sticks and triggers</h3>
      Stick curves from presets or custom points, inner and outer dead zones, circular range.
      Trigger travel: where LT/RT start registering and where they read fully pressed.
      On the Apex 5, force-trigger effects with a "Try on the controller" button.
    </td>
    <td width="50%" valign="top">
      <h3>Gyro and vibration</h3>
      Gyro aiming done by the pad itself: tilt drives a stick while an enable key is held or toggled.
      Grip and trigger vibration: on/off, strength, trigger threshold and amplitude window.
    </td>
  </tr>
  <tr>
    <td width="50%" valign="top">
      <h3>Lighting</h3>
      Solid, multi-colour, breath, flow, rainbow, wave and flash, with brightness and speed, per profile.
      The CD2 dock gets eight effects or your own picture or GIF, with a live preview.
    </td>
    <td width="50%" valign="top">
      <h3>Live test and settings</h3>
      Buttons, sticks, triggers, gyro and accelerometer live, plus a vibration test.
      Stick auto-calibration, debounce, rebound, precision, center sensitivity, Home as Xbox Guide,
      quick profile switch, sleep time and more.
    </td>
  </tr>
</table>

<br>

**Interface.** English and Russian, following the system language until one is picked in Settings.
Scale from 75 to 200% of the system scale (Ctrl + / Ctrl - / Ctrl 0). Colours come from the libadwaita
palette, the GNOME accent and `@define-color` overrides in `~/.config/gtk-4.0/gtk.css`, so themes from
Noctalia, matugen and the like are picked up live; light or dark follows the system. Space Grotesk with
Manrope for Cyrillic, both shipped inside the binary (OFL, licences in `assets/fonts`).

## Under the hood

- **`src/station`** knows nothing about Flydigi: the `Device` trait with optional capabilities
  (`ProfileStore`, `HardwareSettings`, `LiveInput`), the registry of connected devices and HID discovery.
- **`src/flydigi`** speaks Space Station's protocol and implements `Device` for every model of that family.
- **`src/models`** describes each model: the id it reports, its buttons and which profile parts it has.
- **`src/app`** is the window; a device gets the pages its capabilities and features allow.
- **Protocol** written down in [docs/protocol.md](docs/protocol.md).

A new model that speaks the same protocol is mostly data: a file in `src/models` and a line in
`models::PADS` or `models::DOCKS`. See [CONTRIBUTING.md](CONTRIBUTING.md#adding-a-device).

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

> [!IMPORTANT]
> **Status:** OrbitStation is in early alpha. Only the Vader 5 Pro has been tested on hardware;
> Apex 5 and CD2 support is written from openflydigi's measurements and may misbehave. Backups are
> taken before every write, but use it at your own risk.

## Contributing

Bug reports, hardware test reports, translations and code are all welcome.
Start with [CONTRIBUTING.md](CONTRIBUTING.md).

## Thanks

[openflydigi](https://github.com/mkaliaha/openflydigi) for the profile blob research and the Apex 5,
Apex 6 and CD2 dock facts.

## License

[MIT](LICENSE). Fonts are under the OFL, see `assets/fonts`.
