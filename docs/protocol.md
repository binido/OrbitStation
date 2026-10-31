# Vader 5 Pro protocol

Recovered from Flydigi Space Station 4.2.0.9 (`SpaceStationService.exe`, assembly
`Flydigi.ControllerSdk`, classes `*CommandNewXInput`) and checked on a wired Vader 5 Pro
(VID `37d7`, PID `2401`, device id 130, firmware 7.1.5.0).

## USB interfaces

| Interface | Linux node | Purpose |
|---|---|---|
| 0 | xpad | XInput gamepad |
| 1 | hidraw, usage page `0xffa0`, 32-byte reports, no report id | config commands |
| 2 | hidraw | mouse |
| 3 | hidraw, usage page `0xffef`, report id 5 | keyboard mode |

## Packets (interface 1)

Request: `5a a5 CMD LEN PAYLOAD.. SUM`, `LEN = 2 + len(PAYLOAD)`, `SUM = (CMD + LEN + PAYLOAD) & 0xff`.
On hidraw the write is prefixed with report id `00`.

Reply: `5a a5 CMD TOTAL INDEX DATA..`. Data starts at byte 5, or at byte 4 in the last packet of a multi-packet reply.

| CMD | Payload | Meaning |
|---|---|---|
| `01` | - | heartbeat: device id, connection, MAC, battery, versions |
| `02` | - | read nickname |
| `03` | - | read hardware switches and settings |
| `04` | - | read UID |
| `11` | `ff EN ff ff ff` | extended input stream (`ef` reports) on/off |
| `12` | `L R 00 00` | vibration, motors 0..255 |
| `13` | `SUB EN` | switch, see below |
| `14` | rate | report rate: 1=1000, 2=500, 3=250, 4=125 Hz |
| `15` | precision | 1=8, 4=9, 2=10, 5=11, 3=12, 6=14, 7=16 bit |
| `16` | sensitivity | 14 (highest) .. 20 (lowest), 17 = middle |
| `17` | minutes | sleep time, 0 = never |
| `18` | UTF-8 | write nickname; the pad stores the payload with the checksum, so end the name with `00`, all `ff` = no name |
| `1f` | chip | firmware upgrade mode (do not send) |
| `a1` | - | profile info, byte 5 = active profile |
| `a2` | id | apply profile 0..3 |
| `a3` | id, size | read mapping config |
| `af` | id | **reset all four profiles** to factory, whatever id is sent (measured), 10 s |
| `a5` | n, data | write mapping config packet |
| `a6` | version lo, hi | save mapping config |
| `a7` | id, size | read RGB config |
| `a9` | n, data | write RGB config packet |
| `ac` | id, size | read macro config |
| `f0`-`f7` | | factory tests |

`13` sub-commands: 1 quick profile switch, 2 Home as Xbox Guide, 3 gyro debounce, 4 mapping switch,
5 stick debounce, 6 stick auto-calibration, 7 stick rebound, 8 screen status bar, 9 screen off, 10 audio.

### `03` reply

| Byte | Meaning |
|---|---|
| 5 | supported switches, bit `SUB-1` |
| 6 | enabled switches, bit `SUB-1` |
| 7, 8 | audio supported / enabled (bit 0) |
| 9 | sleep time |
| 10 | report rate, 0 = not supported |
| 11 | stick precision, 0 = not supported |
| 12 | center sensitivity, 0 = not supported |

### `01` reply (data offsets)

0 device id, 1 connection type, 2-5 MAC, 6 battery (high nibble 1 = charging, low nibble = level),
7 chip, 8 motion chip, 10-11 firmware, 12-13 dongle, 14-15 switch, 16-17 trigger, 18-19 screen,
20-21 ADC, 22-23 NearLink. Each version is two bytes printed as four nibbles.

### `ef` extended input

3-10 sticks LX LY RX RY (i16 LE), 11-14 buttons, 15-16 triggers, 17-22 gyro, 23-28 accel (4096 = 1 g).
Button bits: `↑ ↓ ← → A B Select X | Y Start LB RB - - L3 R3 | C Z M1 M2 M3 M4 LM RM | O - - Home`.

## Profile mapping blob

840 bytes, read with `a3 [profile, 20]` as 42 packets (`total`, `index`, `profile`, 20 data bytes).
Reading switches the pad to that profile. Writing: `a4 [profile, first, count, 20]`, then
`a5 [n, 20 bytes]` for each packet of the span (`n` counts from 0), then `a6 [version lo, hi]`
commits the running profile to flash (up to 10 s). Verified on hardware.

| Offset | Size | Meaning |
|---|---|---|
| 0 | 2 | proto version, LE (`0x0302` on Vader 5 Pro) |
| 13 | 32 x 3 | key table, indexed by key id: target, turbo type, turbo frequency |
| 109 | 2 x 7 | stick curve source: type, inner dead zone %, p1 x, p1 y, p2 x, p2 y, end (0..127 scale) |
| 123 | 2 x 7 | trigger travel on 0..255: type, start, start, start, end, end, end (factory `0 0 0 0 255 255 255`) |
| 137 | 8 | gyro to stick: target (0 off, 1 left, 2 right, 3 mouse), enable key, enable (0 toggle, 1 hold), dead zone, sens x, sens y, use mode (1 racer for left), second key (255 none) |
| 145 | 9 | grip vibration: master, per side {switch, min, max, strength %}; switches 0 = on, ff = off |
| 154 | 29 | trigger motors: shared switch, per side 2 gears {type, min, max, threshold, min_start, strength %, min_time} |
| 225 | 2 | change tag, LE, `ffff` = never written |
| 770 | 20 | title, UTF-16LE, 10 chars, factory `配置N` |
| 790 | 2 x 12 | stick table the pad plays: type, 9 points (output % + 50 at 0, 12.5 .. 100 % travel), circular, outer dead zone % |

Stick type: 0 linear, 1 fast (p1 64,96), 2 slow (p1 64,32), 3 custom; p2 is 127,127 for presets.

Key target `ff` keeps the button as is, `20` runs a macro, `fe` keyboard/mouse.
Turbo: `[key itself or target, mode, presses per second]`, mode 1 = while held, 2 = toggled, rate 1..30;
a non-zero third byte is what marks the entry as turbo.
Key ids: 0 ↑, 1 →, 2 ↓, 3 ←, 4 A, 5 B, 6 Select, 7 X, 8 Y, 9 Start, 10 LB, 11 RB, 12 LT, 13 RT,
14 L3, 15 R3, 16 C, 17 Z, 18-21 M1-M4, 22 LM, 23 RM, 24 Fn, 25 Turbo, 27 Home.
Fuller layout: [openflydigi](https://github.com/mkaliaha/openflydigi) `docs/findings-profile-blob.md`.

## Macro store (protocol 3.2)

Read `ac [profile, 20]`, write `ad [profile, first, count, 20]` + `ae [n, 20 bytes]`, same framing as
the mapping blob, then `a6` save and `a2` apply (macros play only after an apply). Vader 5 Pro answers
83 packets (1660 bytes); Space Station writes only the first 81, the last 4 bytes stay `00` whatever
is written there.

| Offset | Size | Meaning |
|---|---|---|
| 0 | 2 | store version, LE (`0x0100`) |
| 2 | 2 | macro count, up to 10 |
| 4 | 10 x 2 | body offsets in 4-byte words after byte 24 |
| 24 | | bodies |

Body: key id, step count u16, kind (1 once, 2 while held, 3 toggled), repeat pause u16 ms,
6 x `ff`, 20-byte name, then per step: time since start u16 ms, key id, event (1 press, 0 release,
2/3 left/right stick with a direction as the key: 160 center, 161 up, 162 up-right .. 168 up-left,
clockwise; 5 hold is never written by Space Station, its "hold" is a press and a release). The key's table entry must be `20 00 00`; a body left on a remapped key keeps playing.
Verified on a Vader 5 Pro: a macro written this way plays from the button it is bound to.

## Lighting (per profile)

Read `a7 [profile, 20]`, write `a8 [profile, first, count, 20]` + `a9 [n, 20 bytes]`, then `a6` save.
Vader 5 Pro: 320 bytes = 16 packets, 10 LEDs x 10 frames. Header: version u16 (`0x0300`), click feedback
(freezes the shown frame, leave 0), loop start, loop end, cycle time (bigger = slower), brightness 0..100,
LED count, effect id (0 off, 1 flow, 3 breath, 4 solid, 5 multi, 6 rainbow, 7 wave, 8 flash), grip sync,
then 10 x `ff`. Frames follow at 20, 3 bytes per LED. The pad only plays frames, effects are generated
on the host. Factory: wave, brightness 20, period 4. Verified on a Vader 5 Pro, as are the vibration blocks above.

## Factory profile

`factory/` holds a slot as the pad ships it (mapping, macros, lighting), read after a reset; the slots
differ only in the title digit. Writing it back is how one profile is reset without command `af`.

## Reproducing

```bash
python3 tools/unbundle.py "SpaceStationService.exe" svc
rea inspect-managed-members svc/Flydigi.ControllerSdk.dll --format json > csdk.json
python3 tools/il.py csdk.json HeartBeatControllerCommandNewXInput
```
