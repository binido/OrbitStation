# Porting openflydigi

What of [openflydigi](https://github.com/mkaliaha/openflydigi) is in OrbitStation, and what is left,
in the order it is worth doing. Only the Vader 5 Pro has been on this desk; everything else is built
from openflydigi's measurements and marked as untested here.

## Done

| Part | Notes |
|---|---|
| Vader 5 Pro, everything | Tested on hardware |
| Apex 5 configuration | Profiles, remapping, macros (3.1 page), sticks, triggers' travel, gyro, grip vibration, lighting, settings, live test |
| Apex 5 screen settings | Status bar always on, keep the picture on the screen; both hidden on models without a screen |
| Apex 5 force triggers | Effects stored in the profile, started after every write and profile switch; "Try on the controller" sends them live without storing |
| Apex 6 / 6 Pro | Identity only |
| CD2 dock | Info, name, four switches, seated controller's charge, eight computed lighting effects, a picture or GIF (up to 200 frames, filled and cropped like Space Station's first framing) |

## Left, in order

| # | Part | Size | Risk | Notes |
|---|---|---|---|---|
| 1 | Per-game routes | large | medium | A background service: game detection, Flydigi's public trigger presets, Steam wrapper, Forza Data Out, DSX listener |
| 2 | Virtual DualSense | large | medium | uhid relay; the USB/IP path and haptic audio need root, kernel modules and PipeWire config |
| 3 | DualSense audio haptics to grip motors | medium | medium | Apex-specific upstream |
| 4 | Apex 6 dialect | large | medium | Upstream itself is emulator-tested only |

## Not planned

| Part | Why |
|---|---|
| Apex 5 picture upload to the screen | It goes through the display chip's bootloader over serial; a wireless or interrupted upload can leave the chip in bootloader mode. Same reasoning as calibration |
| Game-memory route | Reads and writes another process's memory; anti-cheat systems treat that as cheating |
| Firmware updates | Not implemented upstream either |
| Screen and LED colour test (command 242) | Floods the screen and the LEDs and stays until the controller is switched off and on |
