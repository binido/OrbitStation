# Contributing to OrbitStation

Thanks for helping. Bug reports, hardware test reports, translations and code are all welcome.

## Reporting a bug

Open an [issue](https://github.com/binido/OrbitStation/issues) with:

- the device and its firmware (both are on the device page);
- your distribution and desktop;
- what you did, what you expected, what happened;
- the terminal output of `orbitstation` if anything was printed.

If a write went wrong, the profile from before it is in `~/.local/share/orbitstation/backups`
and loads back from the profile tabs.

## Testing on hardware

Only the Vader 5 Pro has been on the maintainer's desk. The Apex 5 and the CD2 dock are written from
[openflydigi](https://github.com/mkaliaha/openflydigi)'s measurements, so a report from a real device
is worth a lot, even "everything works".

1. Install with the [install command](README.md#install), keeping the udev rule.
2. Connect the device by USB cable and go through its pages one by one.
3. Open an issue titled after the device, listing what worked and what did not.

Every write backs up the profile first, but be careful with "Reset all profiles": it wipes all four slots.

## Building from source

Needs Rust 1.95 or newer, a C compiler and git.

```bash
git clone https://github.com/binido/OrbitStation
cd OrbitStation
cargo run --release   # the app
cargo test            # the tests
```

Without the udev rule your user cannot open the device; `sh install.sh` installs it
(or see `70-orbitstation.rules`).

## Sending a change

1. Fork, branch from `main`, keep the change to one topic.
2. Run `cargo fmt`, `cargo clippy` and `cargo test` before pushing.
3. Open a pull request saying what changed and on which device it was checked.
   "Not tested on hardware" is fine, just say so.

Match the surrounding code: short doc comments that say what a thing is, no file headers beyond
the module's `//!` line, no new dependency where a few lines will do.

## Adding a device

- A model that speaks Space Station's protocol is mostly data: a file in `src/models` (the ids it reports,
  its buttons, which profile parts it has) and a line in `models::PADS` or `models::DOCKS`.
  `src/models/apex5.rs` is a good example.
- A device with a protocol of its own implements `station::Device` and adds a `Family` to
  `station::FAMILIES`.
- Protocol facts go into [docs/protocol.md](docs/protocol.md), with where they came from.
- What is left to port from openflydigi, and in which order: [docs/porting-openflydigi.md](docs/porting-openflydigi.md).

Do not add commands that can brick a device (firmware, bootloader, calibration) without a way back.

## Translations

UI strings are written in English with `t!("...")` and looked up in a table per language.
Russian lives in `src/i18n/ru.rs`. The test `every_string_has_russian` lists any string without
a translation, and `translations_keep_their_placeholders` checks that every `{}` survives.

A new language is a file next to `ru.rs`, a variant in `Lang` and a line in `LANGS` in `src/i18n.rs`.

## License

By contributing you agree that your work is released under the [MIT License](LICENSE).
