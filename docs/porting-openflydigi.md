# Porting openflydigi

What of [openflydigi](https://github.com/mkaliaha/openflydigi) is in OrbitStation, and what is left,
in the order it is worth doing. Only the Vader 5 Pro has been on this desk; everything else is built
from openflydigi's measurements and marked as untested here.

## Done

| Part | Notes |
|---|---|
| Vader 5 Pro, everything | Tested on hardware |
| Apex 5 configuration | Profiles, remapping, macros (3.1 page), sticks, triggers' travel, gyro, grip vibration, lighting, settings, live test |
| Apex 6 / 6 Pro | Identity only |
| CD2 dock | Info, name, four switches, seated controller's charge, eight computed lighting effects |

## Left, in order

| # | Part | Size | Risk | Notes |
|---|---|---|---|---|
| 1 | Apex 5 force triggers stored in the profile (offset 185) | medium | low | Profile data like the rest; a "Force triggers" page |
| 2 | Live force-trigger effects (commands 81, 82) | small | low | Race, Sniper, Recoil, Lock, Vibration, and rumble routed into the triggers |
| 3 | Apex 5 screen settings | small | low | Ordinary commands, PROTOCOL.md §8c |
| 4 | Dock picture and GIF | medium | low | Needs an image decoder (a new dependency) and a crop view |
| 5 | Per-game routes | large | medium | A background service: game detection, Flydigi's public trigger presets, Steam wrapper, Forza Data Out, DSX listener |
| 6 | Virtual DualSense | large | medium | uhid relay; the USB/IP path and haptic audio need root, kernel modules and PipeWire config |
| 7 | DualSense audio haptics to grip motors | medium | medium | Apex-specific upstream |
| 8 | Apex 6 dialect | large | medium | Upstream itself is emulator-tested only |

## Not planned

| Part | Why |
|---|---|
| Apex 5 picture upload to the screen | It goes through the display chip's bootloader over serial; a wireless or interrupted upload can leave the chip in bootloader mode. Same reasoning as calibration |
| Game-memory route | Reads and writes another process's memory; anti-cheat systems treat that as cheating |
| Firmware updates | Not implemented upstream either |
