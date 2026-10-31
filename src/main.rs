#[macro_use]
mod i18n;
mod device;
mod proto;
mod theme;
mod ui;

use device::{Device, Profile};
use eframe::egui::{self, Color32, Pos2, RichText, Stroke, Vec2};
use proto::macros::{self, Macro, Step};
use proto::{DeviceInfo, HwStatus, Toggle, cmd, gyro, map, rgb, stick, trigger, vibration};
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use ui::icon;

#[derive(PartialEq, Clone, Copy)]
enum Tab {
    Device,
    Test,
    Remap,
    Sticks,
    Gyro,
    Vibration,
    Lighting,
    Settings,
}

impl Tab {
    fn icon(self) -> &'static str {
        match self {
            Tab::Device => icon::GAMEPAD,
            Tab::Test => icon::HAND_TAP,
            Tab::Remap => icon::SWAP,
            Tab::Sticks => icon::JOYSTICK,
            Tab::Gyro => icon::COMPASS,
            Tab::Vibration => icon::VIBRATE,
            Tab::Lighting => icon::LIGHTBULB,
            Tab::Settings => icon::GEAR,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Tab::Device => t!("Device"),
            Tab::Test => t!("Button test"),
            Tab::Remap => t!("Remapping"),
            Tab::Sticks => t!("Sticks"),
            Tab::Gyro => t!("Gyro"),
            Tab::Vibration => t!("Vibration"),
            Tab::Lighting => t!("Lighting"),
            Tab::Settings => t!("Settings"),
        }
    }

    /// Tabs that edit the profile stored on the pad.
    fn edits_profile(self) -> bool {
        matches!(self, Tab::Remap | Tab::Sticks | Tab::Gyro | Tab::Vibration | Tab::Lighting)
    }
}

/// Profile as read from the pad and as edited in the window.
struct Mapping {
    cfg: u8,
    device: Profile,
    map: Vec<u8>,
    macros: Vec<Macro>,
    rgb: Vec<u8>,
    /// Colours the lighting effect is generated from; the pad stores only frames.
    colors: Vec<rgb::Rgb>,
    /// Set by the vibration editor, the app runs the motors for a second.
    test_vibration: bool,
}

impl Mapping {
    fn new(cfg: u8, device: Profile) -> Mapping {
        Mapping {
            cfg,
            map: device.map.clone(),
            macros: macros::parse(&device.macros),
            rgb: device.rgb.clone(),
            colors: vec![[0, 116, 255]],
            test_vibration: false,
            device,
        }
    }

    /// Puts a profile from a file or the factory template into the editor, the device copy stays.
    fn replace(&mut self, p: &Profile) -> Result<(), String> {
        if (p.map.len(), p.macros.len(), p.rgb.len()) != (self.device.map.len(), self.device.macros.len(), self.device.rgb.len()) {
            return Err(t!("the file is from another gamepad or damaged").into());
        }
        self.map = p.map.clone();
        self.macros = macros::parse(&p.macros);
        self.rgb = p.rgb.clone();
        Ok(())
    }

    /// The profile to write: keys with a macro point at the macro store, keys that lost one are reset,
    /// otherwise the pad would go on playing an orphaned macro under the new binding.
    fn build(&self) -> Result<Profile, String> {
        let mut map = self.map.clone();
        for (id, name) in map::KEYS {
            let count = self.macros.iter().filter(|m| m.key == id).count();
            if count > 1 {
                return Err(t!("button {} has more than one macro", name));
            }
            if count == 1 {
                map::set_key(&mut map, id, map::Key { target: map::MACRO, turbo: 0, hz: 0 });
            } else if map::key(&map, id).target == map::MACRO {
                map::set_key(&mut map, id, map::Key { target: map::IDENTITY, turbo: 0, hz: 0 });
            }
        }
        Ok(Profile { map, macros: macros::encode(&self.device.macros, &self.macros)?, rgb: self.rgb.clone() })
    }
}

/// Interface scale steps, on top of the system scale.
const ZOOMS: [f32; 8] = [0.75, 0.9, 1.0, 1.1, 1.25, 1.5, 1.75, 2.0];

struct App {
    dev: Option<Device>,
    /// The zoom last saved, Ctrl +/- change it too.
    zoom: f32,
    colors: theme::Colors,
    watcher: theme::Watcher,
    error: String,
    last_poll: Instant,
    info: Option<DeviceInfo>,
    status: Option<HwStatus>,
    profile: u8,
    tab: Tab,
    test_mode: bool,
    /// Grip left/right and trigger left/right motors on the test tab.
    vibration: [u8; 4],
    vibration_stop: Option<Instant>,
    mapping: Option<Mapping>,
    nickname: String,
    import_pick: Option<PathBuf>,
    confirm_reset_all: bool,
    note: String,
}

impl App {
    fn new(ctx: &egui::Context) -> App {
        // egui's text is smaller than a desktop's at the same scale (12.5 px body against ~14.5 px),
        // so 100% here matches other apps.
        ctx.all_styles_mut(|s| s.text_styles.values_mut().for_each(|f| f.size *= 1.16));
        // Space Grotesk has no Cyrillic: Manrope (Regular, Cyrillic only) draws it, egui's fonts the rest.
        let mut fonts = egui::FontDefinitions::default();
        for (i, (name, data)) in [
            ("SpaceGrotesk", &include_bytes!("../assets/fonts/SpaceGrotesk-Regular.ttf")[..]),
            ("Manrope", &include_bytes!("../assets/fonts/Manrope-Regular-Cyrillic.ttf")[..]),
            // before egui's own icon font, which uses the same private-use codepoints
            ("Phosphor", ui::FONT),
        ]
        .into_iter()
        .enumerate()
        {
            fonts.font_data.insert(name.into(), std::sync::Arc::new(egui::FontData::from_static(data)));
            fonts.families.entry(egui::FontFamily::Proportional).or_default().insert(i, name.into());
        }
        ctx.set_fonts(fonts);
        let zoom = read_config("zoom").and_then(|z| z.parse().ok()).filter(|z| (0.5..=3.0).contains(z)).unwrap_or(1.0);
        ctx.set_zoom_factor(zoom);
        let mut app = App {
            zoom,
            colors: theme::apply(ctx),
            watcher: theme::Watcher::new(ctx),
            dev: None,
            error: String::new(),
            last_poll: Instant::now(),
            info: None,
            status: None,
            profile: 0,
            tab: Tab::Device,
            test_mode: false,
            vibration: [0; 4],
            vibration_stop: None,
            mapping: None,
            nickname: String::new(),
            import_pick: None,
            confirm_reset_all: false,
            note: String::new(),
        };
        app.connect();
        app
    }

    fn connect(&mut self) {
        self.last_poll = Instant::now();
        self.test_mode = false;
        match Device::open() {
            Ok(d) => {
                self.dev = Some(d);
                self.error.clear();
                self.refresh();
            }
            Err(e) => self.error = e,
        }
    }

    fn call(&mut self, pkt: [u8; proto::PKT]) -> Option<[u8; proto::PKT]> {
        let r = self.dev.as_mut()?.call(pkt);
        if r.is_none() {
            self.note = t!("The gamepad did not answer command 0x{}", format!("{:02x}", pkt[2]));
        }
        r
    }

    fn refresh(&mut self) {
        if let Some(r) = self.call(proto::request(cmd::HEARTBEAT, &[])) {
            self.info = Some(proto::parse_heartbeat(&r));
        }
        if let Some(r) = self.call(proto::request(cmd::HW_STATUS, &[])) {
            self.status = Some(proto::parse_hw_status(&r));
        }
        if let Some(r) = self.call(proto::request(cmd::PROFILE_INFO, &[])) {
            self.profile = proto::parse_profile(&r);
        }
        if let Some(r) = self.call(proto::request(cmd::NICKNAME, &[])) {
            self.nickname = proto::parse_nickname(&r);
        }
    }

    fn set_test_mode(&mut self, on: bool) {
        if self.test_mode != on {
            self.test_mode = on;
            self.call(proto::request(cmd::TEST_MODE, &[0xff, on as u8, 0xff, 0xff, 0xff]));
        }
    }

    /// Sends a setting and reports the outcome in the status line.
    fn apply(&mut self, pkt: [u8; proto::PKT], what: &str) {
        self.note = match self.call(pkt) {
            Some(_) => t!("Saved: {}", what),
            None => t!("Could not apply: {}", what),
        };
    }

    fn device_tab(&mut self, ui: &mut egui::Ui) {
        let Some(info) = self.info.clone() else {
            ui.label(t!("No data from the gamepad."));
            return;
        };
        ui::card(ui, icon::INFO, t!("Gamepad"), |ui| {
            ui::grid(ui, "info", |ui| {
                ui::key(ui, t!("Model"));
                ui.strong(proto::model_name(info.device_id));
                ui.end_row();
                ui::key(ui, t!("Gamepad name"));
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.nickname).hint_text(t!("no name")).desired_width(200.0));
                    if ui.button(ui::label(icon::FLOPPY, t!("Save name"))).clicked() {
                        let name = self.nickname.clone();
                        self.apply(proto::nickname_packet(&name), t!("gamepad name"));
                    }
                });
                ui.end_row();
                ui::key(ui, t!("Battery"));
                ui.label(if info.charging { t!("Charging").to_string() } else { format!("{} / 5", info.battery) });
                ui.end_row();
                ui::key(ui, t!("Firmware"));
                ui.label(&info.firmware);
                ui.end_row();
                ui::key(ui, t!("Radio module"));
                ui.label(&info.nearlink);
                ui.end_row();
                ui::key(ui, t!("Device ID"));
                ui.label(info.device_id.to_string());
                ui.end_row();
                ui::key(ui, t!("Device node"));
                ui.monospace(self.dev.as_ref().map(|d| d.path.display().to_string()).unwrap_or_default());
                ui.end_row();
            });
        });
        ui::card(ui, icon::SLIDERS, t!("Active profile"), |ui| {
            ui.horizontal(|ui| {
                for p in 0..4u8 {
                    if ui.selectable_label(self.profile == p, t!("Profile {}", p + 1)).clicked() && self.profile != p {
                        self.profile = p;
                        self.mapping = None;
                        self.apply(proto::request(cmd::APPLY_PROFILE, &[p]), &t!("profile {}", p + 1));
                    }
                }
                ui.add_space(12.0);
                if ui.button(ui::label(icon::REFRESH, t!("Refresh"))).clicked() {
                    self.refresh();
                }
            });
        });
        ui::card(ui, icon::WARNING, t!("Factory reset of all profiles"), |ui| {
            ui.label(t!("Like 'Restore default' in Space Station: all 4 profiles go back to factory settings. General settings stay as they are."));
            ui.label(t!("Before the reset the app saves copies of all profiles, you can load them back from a file."));
            ui.checkbox(&mut self.confirm_reset_all, t!("I understand that all 4 profiles will be erased"));
            let button = egui::Button::new(RichText::new(ui::label(icon::TRASH, t!("Reset all profiles"))).color(ui.visuals().error_fg_color));
            if ui.add_enabled(self.confirm_reset_all, button).clicked() {
                self.confirm_reset_all = false;
                self.reset_all();
            }
        });
    }

    fn reset_all(&mut self) {
        let Some(dev) = self.dev.as_mut() else { return };
        // Reading a profile switches to it, so every slot is read and the active one comes back after.
        for cfg in 0..4 {
            match dev.read_profile(cfg).map(|p| backup_profile(cfg, &p)) {
                Some(Ok(_)) => {}
                _ => {
                    self.note = t!("Could not back up profile {}, the reset is cancelled", cfg + 1);
                    dev.call(proto::request(cmd::APPLY_PROFILE, &[self.profile]));
                    return;
                }
            }
        }
        self.note = match dev.reset_all_profiles() {
            Ok(()) => t!("All profiles reset, copies are in {}", data_dir().join("backups").display()),
            Err(e) => e,
        };
        dev.call(proto::request(cmd::APPLY_PROFILE, &[self.profile]));
        self.mapping = None;
        self.refresh();
    }

    fn test_tab(&mut self, ui: &mut egui::Ui) {
        let input = self.dev.as_ref().and_then(|d| *d.input.lock().unwrap()).unwrap_or_default();
        ui.ctx().request_repaint_after(Duration::from_millis(16));
        ui::card(ui, icon::JOYSTICK, t!("Sticks and triggers"), |ui| {
            ui.horizontal(|ui| {
                stick(ui, t!("Left stick"), input.lx, input.ly, input.buttons & 1 << 14 != 0);
                ui.add_space(16.0);
                stick(ui, t!("Right stick"), input.rx, input.ry, input.buttons & 1 << 15 != 0);
                ui.add_space(16.0);
                ui.vertical(|ui| {
                    ui::key(ui, t!("Triggers"));
                    for (name, v) in [("LT", input.lt), ("RT", input.rt)] {
                        ui.horizontal(|ui| {
                            ui.monospace(name);
                            ui.add(egui::ProgressBar::new(v as f32 / 255.0).desired_width(180.0));
                            ui.monospace(format!("{v:>3}"));
                        });
                    }
                });
            });
        });
        ui::card(ui, icon::CURSOR, t!("Buttons"), |ui| {
            ui.horizontal_wrapped(|ui| {
                for (i, name) in proto::BUTTONS.iter().enumerate().filter(|(_, n)| !n.is_empty()) {
                    let on = input.buttons & 1 << i != 0;
                    let text = RichText::new(*name).monospace();
                    let sel = ui.visuals().selection;
                    ui.add(egui::Button::new(if on { text.color(sel.stroke.color) } else { text })
                        .fill(if on { sel.bg_fill } else { ui.visuals().widgets.inactive.bg_fill })
                        .min_size(Vec2::new(52.0, 32.0)));
                }
            });
        });
        ui::card(ui, icon::COMPASS, t!("Motion sensors"), |ui| {
            ui.monospace(format!("{:<14}{:>7} {:>7} {:>7}", t!("Gyro"), input.gyro[0], input.gyro[1], input.gyro[2]));
            ui.monospace(format!("{:<14}{:>7} {:>7} {:>7}", t!("Accelerometer"), input.accel[0], input.accel[1], input.accel[2]));
        });
        ui::card(ui, icon::VIBRATE, t!("Vibration"), |ui| {
            let old = self.vibration;
            for (v, name) in self.vibration.iter_mut().zip([t!("left grip"), t!("right grip"), t!("LT motor"), t!("RT motor")]) {
                ui.add(egui::Slider::new(v, 0..=255).text(name));
            }
            if old != self.vibration {
                self.vibration_stop = None;
                self.call(proto::request(cmd::VIBRATION, &self.vibration));
            }
        });
    }

    fn load_mapping(&mut self) {
        let cfg = self.profile;
        match self.dev.as_mut().and_then(|d| d.read_profile(cfg)) {
            Some(p) => {
                self.mapping = Some(Mapping::new(cfg, p));
                self.note = t!("Profile {} loaded", cfg + 1);
            }
            None => self.note = t!("Could not read the profile").into(),
        }
    }

    fn save_mapping(&mut self) {
        let (Some(dev), Some(m)) = (self.dev.as_mut(), self.mapping.as_mut()) else { return };
        let mut new = match m.build() {
            Ok(p) => p,
            Err(e) => {
                self.note = e;
                return;
            }
        };
        // Lighting has no tag of its own, a lighting-only save keeps the profile's tag.
        if new.map != m.device.map || new.macros != m.device.macros {
            map::set_version(&mut new.map, new_version(&m.device.map));
        }
        let backup = backup_profile(m.cfg, &m.device);
        self.note = match dev.write_profile(m.cfg, &m.device, &new) {
            Err(e) => t!("Write failed: {}", e),
            Ok(()) if dev.read_profile(m.cfg).as_ref() != Some(&new) => {
                t!("Written, but the check read did not match, load the profile again").into()
            }
            Ok(()) => {
                *m = Mapping::new(m.cfg, new);
                match backup {
                    Ok(path) => t!("Profile saved, previous copy: {}", path.display()),
                    Err(e) => t!("Profile saved, but the copy could not be made: {}", e),
                }
            }
        };
    }

    fn profile_loaded(&self) -> bool {
        self.mapping.as_ref().is_some_and(|m| m.cfg == self.profile)
    }

    /// Shows an editor of the loaded profile; the write controls sit in their own bar under the page.
    fn profile_tab(&mut self, ui: &mut egui::Ui, editor: fn(&mut egui::Ui, &mut Mapping)) {
        if !self.profile_loaded() {
            ui::card(ui, icon::DOWNLOAD, &t!("Profile {}", self.profile + 1), |ui| {
                ui.label(t!("Profile {} is not loaded yet.", self.profile + 1));
                if ui.button(ui::label(icon::DOWNLOAD, t!("Load from gamepad"))).clicked() {
                    self.load_mapping();
                }
            });
            return;
        }
        self.profile_files(ui);
        let m = self.mapping.as_mut().unwrap();
        editor(ui, m);
        if std::mem::take(&mut m.test_vibration) {
            self.call(proto::request(cmd::VIBRATION, &[160, 160, 160, 160]));
            self.vibration_stop = Some(Instant::now() + Duration::from_secs(1));
        }
    }

    /// Write, discard and reload, always in view however long the page is.
    fn profile_actions(&mut self, ui: &mut egui::Ui) {
        let m = self.mapping.as_mut().unwrap();
        let built = m.build();
        let dirty = built.as_ref().is_ok_and(|p| *p != m.device);
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let write = egui::Button::new(RichText::new(ui::label(icon::FLOPPY, t!("Write to gamepad"))).color(ui.visuals().selection.stroke.color))
                .fill(ui.visuals().selection.bg_fill);
            if ui.add_enabled(dirty, write).clicked() {
                self.save_mapping();
            }
            let m = self.mapping.as_mut().unwrap();
            if ui.add_enabled(dirty, egui::Button::new(ui::label(icon::UNDO, t!("Discard changes")))).clicked() {
                *m = Mapping::new(m.cfg, m.device.clone());
            }
            if ui.button(ui::label(icon::REFRESH, t!("Reload"))).clicked() {
                self.load_mapping();
            }
            match &built {
                Err(e) => {
                    ui.colored_label(ui.visuals().warn_fg_color, ui::label(icon::WARNING, e));
                }
                Ok(_) if dirty => {
                    ui.colored_label(ui.visuals().warn_fg_color, t!("Unsaved changes"));
                }
                Ok(_) => {}
            }
        });
        ui.add_space(4.0);
    }

    /// Profile name, files and the factory template, shared by every profile tab.
    fn profile_files(&mut self, ui: &mut egui::Ui) {
        let cfg = self.mapping.as_ref().unwrap().cfg;
        ui::card(ui, icon::SLIDERS, &t!("Profile {}", cfg + 1), |ui| {
            ui::grid(ui, "profile", |ui| {
                ui::key(ui, t!("Name"));
                ui.horizontal(|ui| {
                    let m = self.mapping.as_mut().unwrap();
                    let mut title = map::title(&m.map);
                    let edit = egui::TextEdit::singleline(&mut title).char_limit(map::TITLE_CHARS).desired_width(160.0);
                    if ui.add(edit).on_hover_text(t!("Profile name, up to 10 characters")).changed() {
                        map::set_title(&mut m.map, &title);
                    }
                    let factory = ui.button(ui::label(icon::FACTORY, t!("Factory profile")));
                    if factory.on_hover_text(t!("Only this profile, the others stay untouched")).clicked() {
                        let factory = Profile::factory(m.cfg);
                        self.note = m.replace(&factory).map_or_else(|e| e, |_| t!("Factory values filled in, press 'Write to gamepad'").into());
                    }
                });
                ui.end_row();
                ui::key(ui, t!("Files"));
                ui.horizontal_wrapped(|ui| {
                    if ui.button(ui::label(icon::FILE_SAVE, t!("Save to file"))).clicked() {
                        let m = self.mapping.as_ref().unwrap();
                        let name = map::title(&m.map).chars().filter(|c| c.is_alphanumeric() || " -_".contains(*c)).collect::<String>();
                        let name = if name.trim().is_empty() { format!("profile{}", m.cfg + 1) } else { name.trim().to_string() };
                        let path = data_dir().join("profiles").join(format!("{name}.osp"));
                        self.note = match m.build().and_then(|p| write_file(&path, &p.to_file()).map_err(|e| e.to_string())) {
                            Ok(()) => t!("Saved to {}", path.display()),
                            Err(e) => t!("Could not save: {}", e),
                        };
                    }
                    let files = profile_files();
                    let label = self.import_pick.as_ref().map_or(t!("choose a file").to_string(), |p| file_label(p));
                    egui::ComboBox::from_id_salt("import").selected_text(label).width(240.0).show_ui(ui, |ui| {
                        for f in files {
                            let text = file_label(&f);
                            ui.selectable_value(&mut self.import_pick, Some(f), text);
                        }
                    });
                    if ui.add_enabled(self.import_pick.is_some(), egui::Button::new(ui::label(icon::FILE_LOAD, t!("Load from file")))).clicked() {
                        let path = self.import_pick.clone().unwrap();
                        let loaded = std::fs::read(&path).ok().and_then(|d| Profile::from_file(&d)).ok_or(t!("could not read the file").to_string());
                        let m = self.mapping.as_mut().unwrap();
                        self.note = match loaded.and_then(|p| m.replace(&p)) {
                            Ok(()) => t!("Loaded from {}, press 'Write to gamepad'", file_label(&path)),
                            Err(e) => e,
                        };
                    }
                    if ui.button(ui::label(icon::FOLDER_OPEN, t!("Open folder"))).clicked() {
                        let dir = data_dir();
                        let _ = std::fs::create_dir_all(dir.join("profiles"));
                        let _ = std::process::Command::new("xdg-open").arg(dir).spawn();
                    }
                });
                ui.end_row();
            });
        });
    }

    fn settings_tab(&mut self, ui: &mut egui::Ui) {
        ui::card(ui, icon::TRANSLATE, t!("Application"), |ui| {
            ui::grid(ui, "app", |ui| {
                ui::key(ui, t!("Language"));
                let mut lang = i18n::current();
                let name = |l| i18n::LANGS.iter().find(|x| x.0 == l).unwrap().2;
                egui::ComboBox::from_id_salt("language").selected_text(name(lang)).width(180.0).show_ui(ui, |ui| {
                    for (l, _, n) in i18n::LANGS {
                        ui.selectable_value(&mut lang, l, n);
                    }
                });
                if lang != i18n::current() {
                    i18n::set(lang);
                    if let Err(e) = i18n::save(lang) {
                        self.note = t!("Could not save: {}", e);
                    }
                }
                ui.end_row();
                ui::key(ui, t!("Interface scale"));
                let mut zoom = ui.ctx().zoom_factor();
                egui::ComboBox::from_id_salt("zoom")
                    .selected_text(format!("{:.0}%", zoom * 100.0))
                    .width(180.0)
                    .show_ui(ui, |ui| {
                        for z in ZOOMS {
                            ui.selectable_value(&mut zoom, z, format!("{:.0}%", z * 100.0));
                        }
                    })
                    .response
                    .on_hover_text(t!("100% is the system scale. Ctrl + and Ctrl - work too, Ctrl 0 goes back to 100%."));
                if zoom != ui.ctx().zoom_factor() {
                    ui.ctx().set_zoom_factor(zoom);
                }
                ui.end_row();
            });
        });
        if self.dev.is_none() {
            return;
        }
        let Some(mut st) = self.status.clone() else {
            ui.label(t!("No settings data."));
            return;
        };
        ui::card(ui, icon::GEAR, t!("General"), |ui| {
            for t in Toggle::ALL {
                if !st.usable(t) {
                    continue;
                }
                let mut on = st.enabled(t);
                if ui.checkbox(&mut on, t.label()).changed() {
                    st.set(t, on);
                    self.apply(t.packet(on), t.label());
                }
            }
        });
        let choice = |ui: &mut egui::Ui, title: &str, value: &mut u8, opts: &[(u8, &str)], c: u8, app: &mut App| {
            ui::key(ui, title);
            ui.horizontal_wrapped(|ui| {
                for (v, name) in opts {
                    if ui.radio_value(value, *v, i18n::tr(name)).clicked() {
                        app.apply(proto::request(c, &[*v]), title);
                    }
                }
            });
            ui.end_row();
        };
        if st.report_rate != 0 || st.precision != 0 || st.sensitivity != 0 {
            ui::card(ui, icon::JOYSTICK, t!("Sticks"), |ui| {
                ui::grid(ui, "sticks", |ui| {
                    if st.report_rate != 0 {
                        choice(ui, t!("Stick polling rate"), &mut st.report_rate, &proto::REPORT_RATES, cmd::REPORT_RATE, self);
                    }
                    if st.precision != 0 {
                        choice(ui, t!("Stick precision"), &mut st.precision, &proto::PRECISIONS, cmd::PRECISION, self);
                    }
                    if st.sensitivity != 0 {
                        choice(ui, t!("Sensitivity near the stick center"), &mut st.sensitivity, &proto::SENSITIVITIES, cmd::SENSITIVITY, self);
                    }
                });
            });
        }
        ui::card(ui, icon::BATTERY, t!("Power"), |ui| {
            ui::grid(ui, "power", |ui| {
                choice(ui, t!("Time until sleep"), &mut st.sleep_time, &proto::SLEEP_TIMES, cmd::SLEEP_TIME, self);
            });
        });
        self.status = Some(st);
    }
}

fn config_home() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn config_path(name: &str) -> PathBuf {
    config_home().join("orbitstation").join(name)
}

/// One small file per setting in ~/.config/orbitstation.
fn read_config(name: &str) -> Option<String> {
    std::fs::read_to_string(config_path(name)).ok().map(|s| s.trim().to_string())
}

fn write_config(name: &str, value: &str) -> std::io::Result<()> {
    write_file(&config_path(name), value.as_bytes())
}

fn data_dir() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("orbitstation")
}

fn write_file(path: &std::path::Path, data: &[u8]) -> std::io::Result<()> {
    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(path, data)
}

/// Saved profiles first, then backups, newest first.
fn profile_files() -> Vec<PathBuf> {
    let list = |dir: &str| {
        let mut v: Vec<(SystemTime, PathBuf)> = std::fs::read_dir(data_dir().join(dir))
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == "osp"))
            .map(|e| (e.metadata().and_then(|m| m.modified()).unwrap_or(UNIX_EPOCH), e.path()))
            .collect();
        v.sort_by_key(|e| std::cmp::Reverse(e.0));
        v.into_iter().map(|(_, p)| p)
    };
    list("profiles").chain(list("backups")).collect()
}

fn file_label(p: &std::path::Path) -> String {
    let name = p.file_stem().unwrap_or_default().to_string_lossy();
    if p.parent().is_some_and(|d| d.ends_with("backups")) { t!("backup: {}", name) } else { name.into_owned() }
}

/// Saves the profile as it was on the pad before a write, the app has no other undo for flash.
fn backup_profile(cfg: u8, p: &Profile) -> std::io::Result<PathBuf> {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let path = data_dir().join("backups").join(format!("profile{}-{secs}.osp", cfg + 1));
    write_file(&path, &p.to_file())?;
    Ok(path)
}

/// Space Station picks a random tag that differs from the current one; 0xffff means "never written".
fn new_version(blob: &[u8]) -> u16 {
    let old = map::version(blob);
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos();
    let v = (nanos % 0xffff) as u16;
    if v == old { (v + 1) % 0xffff } else { v }
}

fn name_of(list: &[(u8, &str)], v: u8) -> String {
    list.iter().find(|t| t.0 == v).map_or_else(|| format!("#{v}"), |t| i18n::tr(t.1).to_string())
}

fn pick(ui: &mut egui::Ui, id: impl egui::AsIdSalt, value: &mut u8, list: &[(u8, &str)], width: f32) {
    egui::ComboBox::from_id_salt(id).selected_text(name_of(list, *value)).width(width).show_ui(ui, |ui| {
        for (v, n) in list {
            ui.selectable_value(value, *v, i18n::tr(n));
        }
    });
}

fn remap_editor(ui: &mut egui::Ui, m: &mut Mapping) {
    ui::card(ui, icon::SWAP, t!("Buttons"), |ui| {
        ui::key(ui, t!("The button on the left presses what is chosen on the right."));
        keys_grid(ui, m);
        ui.add_space(4.0);
        if ui.button(ui::label(icon::UNDO, t!("Default buttons"))).clicked() {
            for (id, _) in map::KEYS {
                map::set_key(&mut m.map, id, map::Key { target: map::IDENTITY, turbo: 0, hz: 0 });
            }
            m.macros.clear();
        }
    });
    ui::card(ui, icon::KEYBOARD, t!("Macros"), |ui| macros_editor(ui, m));
}

fn sticks_editor(ui: &mut egui::Ui, m: &mut Mapping) {
    ui.columns(2, |cols| {
        for (side, ui) in cols.iter_mut().enumerate() {
            let old = stick::get(&m.map, side);
            let mut s = old;
            ui::card(ui, icon::JOYSTICK, i18n::tr(stick::SIDES[side]), |ui| {
                curve_plot(ui, &stick::bank(&m.map, side));
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui::key(ui, t!("Curve"));
                    pick(ui, ("curve", side), &mut s.kind, &stick::PRESETS, 130.0);
                });
                // Preset points are applied on a change only, a stored curve stays byte for byte.
                if let Some((p1, p2)) = stick::preset(s.kind) {
                    if s.kind != old.kind {
                        (s.p1, s.p2) = (p1, p2);
                    }
                } else {
                    ui.add(egui::Slider::new(&mut s.p1.0, 0..=127).text(t!("point 1: travel")));
                    ui.add(egui::Slider::new(&mut s.p1.1, 0..=127).text(t!("point 1: output")));
                    ui.add(egui::Slider::new(&mut s.p2.0, 0..=127).text(t!("point 2: travel")));
                    ui.add(egui::Slider::new(&mut s.p2.1, 0..=127).text(t!("point 2: output")));
                }
                ui.add(egui::Slider::new(&mut s.center, 0..=100 - s.edge).text(t!("center dead zone, %")));
                ui.add(egui::Slider::new(&mut s.edge, 0..=100 - s.center).text(t!("outer dead zone, %")));
                ui.checkbox(&mut s.circular, t!("Circular travel area"));
            });
            if s != old {
                stick::set(&mut m.map, side, s);
            }
        }
    });
    ui::card(ui, icon::GAUGE, t!("Trigger travel"), |ui| {
        ui::key(ui, t!("The game gets a full press at the end of the chosen range and nothing before its start."));
        ui::grid(ui, "triggers", |ui| {
            for side in 0..2 {
                let (old_start, old_end) = trigger::get(&m.map, side);
                let pct = |b: u8| (b as u32 * 100 / 255) as u8;
                let (mut start, mut end) = (pct(old_start), pct(old_end));
                ui.strong(trigger::SIDES[side]);
                ui.horizontal(|ui| {
                    ui.add(egui::Slider::new(&mut start, 0..=end).text(t!("start, %")));
                    ui.add_space(12.0);
                    ui.add(egui::Slider::new(&mut end, start..=100).text(t!("full press, %")));
                });
                ui.end_row();
                // Untouched values keep their exact bytes, the percent view rounds them.
                if (start, end) != (pct(old_start), pct(old_end)) {
                    let byte = |p: u8| (p as u32 * 255 / 100) as u8;
                    trigger::set(&mut m.map, side, byte(start), byte(end));
                }
            }
        });
    });
}

fn gyro_editor(ui: &mut egui::Ui, m: &mut Mapping) {
    let old = gyro::get(&m.map);
    let mut g = old;
    let mut keys = vec![(gyro::NO_KEY, t!("None"))];
    keys.extend(map::KEYS);
    ui::card(ui, icon::TARGET, t!("Aiming"), |ui| {
        ui::key(ui, t!("Tilting the gamepad moves the chosen stick. It works in any game with no software on the computer,"));
        ui::key(ui, t!("but only while the gyro is turned on with a button."));
        ui.add_space(6.0);
        ui::grid(ui, "gyro", |ui| {
            ui::key(ui, t!("Gyro controls"));
            pick(ui, "target", &mut g.target, &gyro::TARGETS, 220.0);
            ui.end_row();
            ui::key(ui, t!("Activation button"));
            pick(ui, "key", &mut g.key, &keys, 220.0);
            ui.end_row();
            ui::key(ui, t!("Second button"));
            pick(ui, "key2", &mut g.key2, &keys, 220.0);
            ui.end_row();
            ui::key(ui, t!("Activation mode"));
            pick(ui, "enable", &mut g.enable, &gyro::ENABLE, 220.0);
            ui.end_row();
            ui::key(ui, t!("Sensitivity"));
            ui.add(egui::Slider::new(&mut g.sensitivity, 0..=100));
            ui.end_row();
            ui::key(ui, t!("Dead zone"));
            ui.add(egui::Slider::new(&mut g.dead_zone, 0..=100));
            ui.end_row();
        });
        if g.target != 0 && g.key == gyro::NO_KEY && g.key2 == gyro::NO_KEY {
            ui.colored_label(ui.visuals().warn_fg_color, ui::label(icon::WARNING, t!("Without an activation button the gyro will not work.")));
        }
    });
    if g != old {
        gyro::set(&mut m.map, g);
    }
}

fn vibration_editor(ui: &mut egui::Ui, m: &mut Mapping) {
    let old = vibration::get(&m.map);
    let mut v = old;
    let pct = |b: u8| (b as u32 * 100 / 255) as u8;
    let byte = |p: u8| (p as u32 * 255 / 100) as u8;
    ui::card(ui, icon::VIBRATE, t!("Grips"), |ui| {
        ui.checkbox(&mut v.grip_on, t!("Grip vibration"));
        ui.add_enabled(v.grip_on, egui::Slider::new(&mut v.grip_strength, 1..=100).text(t!("strength, % (50 = like Xbox)")));
    });
    ui::card(ui, icon::WAVEFORM, t!("Triggers"), |ui| {
        ui.checkbox(&mut v.trig_on, t!("Trigger vibration"));
        ui.add_enabled_ui(v.trig_on, |ui| {
            ui.add(egui::Slider::new(&mut v.trig_strength, 1..=100).text(t!("strength, %")));
            ui.add(egui::Slider::new(&mut v.trig_threshold, 0..=100).text(t!("threshold: weaker signals stay silent")));
            ui::grid(ui, "windows", |ui| {
                for (side, name) in trigger::SIDES.iter().enumerate() {
                    let (lo, hi) = v.trig_window[side];
                    let (mut a, mut b) = (pct(lo), pct(hi));
                    ui.strong(*name);
                    ui.horizontal(|ui| {
                        ui.add(egui::Slider::new(&mut a, 0..=b).text(t!("min. amplitude, %")));
                        ui.add_space(12.0);
                        ui.add(egui::Slider::new(&mut b, a..=100).text(t!("max. amplitude, %")));
                    });
                    ui.end_row();
                    // Untouched values keep their exact bytes, the percent view rounds them.
                    if (a, b) != (pct(lo), pct(hi)) {
                        v.trig_window[side] = (byte(a), byte(b));
                    }
                }
            });
        });
    });
    if v != old {
        vibration::set(&mut m.map, v);
    }
    let test = ui.button(ui::label(icon::VIBRATE, t!("Test vibration (1 second)")));
    if test.on_hover_text(t!("Strength and ranges apply after writing to the gamepad")).clicked() {
        m.test_vibration = true;
    }
}

fn lighting_editor(ui: &mut egui::Ui, m: &mut Mapping) {
    ui::card(ui, icon::PALETTE, t!("Effect"), |ui| {
        let mut effect = m.rgb[rgb::MODE];
        let old_colors = m.colors.clone();
        ui::grid(ui, "effect", |ui| {
            ui::key(ui, t!("Effect"));
            pick(ui, "effect", &mut effect, &rgb::EFFECTS, 200.0);
            ui.end_row();
            if ![0, 6].contains(&effect) {
                ui::key(ui, t!("Colors"));
                ui.horizontal(|ui| {
                    let (mut drop, many) = (None, m.colors.len() > 1);
                    for (i, c) in m.colors.iter_mut().enumerate() {
                        ui.color_edit_button_srgb(c);
                        if many && ui.small_button(icon::X).clicked() {
                            drop = Some(i);
                        }
                    }
                    if let Some(i) = drop {
                        m.colors.remove(i);
                    }
                    if m.colors.len() < rgb::MAX_COLORS && ui.button(ui::label(icon::PLUS, t!("Add color"))).clicked() {
                        m.colors.push([255, 255, 255]);
                    }
                });
                ui.end_row();
            }
        });
        if effect != m.rgb[rgb::MODE] || m.colors != old_colors {
            rgb::apply(&mut m.rgb, effect, &m.colors);
        }
        ui.add(egui::Slider::new(&mut m.rgb[rgb::BRIGHTNESS], 0..=100).text(t!("brightness, %")));
        ui.add(egui::Slider::new(&mut m.rgb[rgb::PERIOD], 1..=30).text(t!("cycle time (higher = slower)")));
        let mut sync = m.rgb[rgb::GRIP_SYNC] != 0;
        if ui.checkbox(&mut sync, t!("Lighting reacts to vibration")).changed() {
            m.rgb[rgb::GRIP_SYNC] = sync as u8;
        }
    });
    ui::card(ui, icon::SPARKLE, t!("Animation preview"), |ui| {
        ui::key(ui, t!("Animation frames the gamepad plays (top to bottom):"));
        let (leds, frames) = (rgb::leds(&m.rgb), rgb::frames(&m.rgb));
        let cell = 18.0;
        let (rect, _) = ui.allocate_exact_size(Vec2::new(cell * leds as f32, cell * frames as f32), egui::Sense::hover());
        for f in 0..frames {
            for l in 0..leds {
                let [r, g, b] = rgb::led(&m.rgb, f, l);
                let min = rect.min + Vec2::new(l as f32 * cell, f as f32 * cell);
                ui.painter().rect_filled(egui::Rect::from_min_size(min, Vec2::splat(cell - 3.0)), 4.0, Color32::from_rgb(r, g, b));
            }
        }
    });
}

/// Draws the 9-point table the pad plays: travel to the right, output up, the diagonal for reference.
fn curve_plot(ui: &mut egui::Ui, bank: &[u8; 9]) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(180.0), egui::Sense::hover());
    let p = ui.painter();
    p.rect_stroke(rect, 2.0, Stroke::new(1.0, ui.visuals().weak_text_color()), egui::StrokeKind::Inside);
    let at = |x: f32, y: f32| Pos2::new(rect.left() + x * rect.width(), rect.bottom() - y.clamp(0.0, 1.0) * rect.height());
    p.line_segment([at(0.0, 0.0), at(1.0, 1.0)], Stroke::new(1.0, ui.visuals().weak_text_color()));
    let pts: Vec<Pos2> = bank.iter().enumerate().map(|(i, v)| at(i as f32 / 8.0, (*v as f32 - 50.0) / 100.0)).collect();
    p.add(egui::Shape::line(pts, Stroke::new(2.0, ui.visuals().selection.bg_fill)));
}

fn keys_grid(ui: &mut egui::Ui, m: &mut Mapping) {
    egui::Grid::new("remap").num_columns(4).spacing([20.0, 8.0]).striped(true).show(ui, |ui| {
        ui::key(ui, t!("Button"));
        ui::key(ui, t!("Presses"));
        ui::key(ui, t!("Turbo"));
        ui::key(ui, t!("Presses per second"));
        ui.end_row();
        for (id, name) in map::KEYS {
            let mut k = map::key(&m.map, id);
            let changed = k != map::key(&m.device.map, id);
            ui.label(if changed { RichText::new(name).strong() } else { RichText::new(name) });
            if m.macros.iter().any(|mc| mc.key == id) {
                ui.label(t!("Macro"));
                ui.end_row();
                continue;
            }
            pick(ui, ("target", id), &mut k.target, &map::TARGETS, 140.0);
            pick(ui, ("turbo", id), &mut k.turbo, &map::TURBO, 170.0);
            if k.turbo > 0 {
                k.hz = k.hz.max(1);
                ui.add(egui::Slider::new(&mut k.hz, 1..=map::TURBO_MAX_HZ));
            }
            map::set_key(&mut m.map, id, k);
            ui.end_row();
        }
    });
}

fn macros_editor(ui: &mut egui::Ui, m: &mut Mapping) {
    let mut remove = None;
    for (i, mc) in m.macros.iter_mut().enumerate() {
        ui.push_id(i, |ui| {
            egui::Frame::new()
                .fill(ui.visuals().text_color().gamma_multiply(0.04))
                .corner_radius(8)
                .inner_margin(12)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal_wrapped(|ui| {
                        ui::key(ui, t!("Button"));
                        pick(ui, "key", &mut mc.key, &map::KEYS, 80.0);
                        ui.add_space(8.0);
                        ui::key(ui, t!("Mode"));
                        pick(ui, "kind", &mut mc.kind, &macros::KINDS, 170.0);
                        if mc.kind != 1 {
                            ui.add_space(8.0);
                            ui::key(ui, t!("Pause between repeats, ms"));
                            ui.add(egui::DragValue::new(&mut mc.interval_ms).range(0..=10_000));
                        }
                        ui.add_space(8.0);
                        let delete = RichText::new(ui::label(icon::TRASH, t!("Delete macro"))).color(ui.visuals().error_fg_color);
                        if ui.button(delete).clicked() {
                            remove = Some(i);
                        }
                    });
                    ui.add_space(4.0);
                    let mut drop = None;
                    egui::Grid::new("steps").num_columns(4).spacing([12.0, 6.0]).show(ui, |ui| {
                        for (j, st) in mc.steps.iter_mut().enumerate() {
                            ui.push_id(j, |ui| {
                                ui.add(egui::DragValue::new(&mut st.delay_ms).range(0..=10_000).suffix(t!(" ms")));
                                let was_stick = macros::is_stick(st.event);
                                pick(ui, "ev", &mut st.event, &macros::EVENTS, 120.0);
                                if was_stick != macros::is_stick(st.event) {
                                    st.key = if macros::is_stick(st.event) { 161 } else { 4 };
                                }
                                if macros::is_stick(st.event) {
                                    pick(ui, "k", &mut st.key, &macros::DIRECTIONS, 90.0);
                                } else {
                                    pick(ui, "k", &mut st.key, &map::TARGETS[1..], 90.0);
                                }
                                if ui.small_button(icon::X).clicked() {
                                    drop = Some(j);
                                }
                            });
                            ui.end_row();
                        }
                    });
                    if let Some(j) = drop {
                        mc.steps.remove(j);
                    }
                    if ui.button(ui::label(icon::PLUS, t!("Add a button press"))).clicked() {
                        mc.steps.push(Step { delay_ms: 30, key: 4, event: 1 });
                        mc.steps.push(Step { delay_ms: 50, key: 4, event: 0 });
                    }
                });
        });
        ui.add_space(8.0);
    }
    if let Some(i) = remove {
        m.macros.remove(i);
    }
    let free = map::KEYS.iter().rev().map(|k| k.0).find(|k| m.macros.iter().all(|mc| mc.key != *k));
    if m.macros.len() < macros::MAX && ui.button(ui::label(icon::PLUS, t!("Add macro"))).clicked() {
        let steps = vec![Step { delay_ms: 0, key: 4, event: 1 }, Step { delay_ms: 50, key: 4, event: 0 }];
        m.macros.push(Macro { key: free.unwrap_or(18), kind: 1, interval_ms: 0, steps });
    }
}

fn stick(ui: &mut egui::Ui, title: &str, x: i16, y: i16, pressed: bool) {
    ui.vertical(|ui| {
        ui::key(ui, title);
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(120.0), egui::Sense::hover());
        let p = ui.painter();
        let r = rect.width() / 2.0 - 4.0;
        p.circle_stroke(rect.center(), r, Stroke::new(1.5, ui.visuals().weak_text_color()));
        let dot = rect.center() + Vec2::new(x as f32, -(y as f32)) / 32768.0 * r;
        let color = if pressed { ui.visuals().selection.bg_fill } else { ui.visuals().strong_text_color() };
        p.circle_filled(Pos2::new(dot.x, dot.y), 7.0, color);
        ui.monospace(format!("{x:>6} {y:>6}"));
    });
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.watcher.changed() {
            self.colors = theme::apply(ui.ctx());
        }
        let zoom = ui.ctx().zoom_factor();
        if zoom != self.zoom {
            self.zoom = zoom;
            if let Err(e) = write_config("zoom", &zoom.to_string()) {
                self.note = t!("Could not save: {}", e);
            }
        }
        if self.dev.as_ref().is_some_and(|d| !d.is_alive()) {
            self.dev = None;
            self.error = t!("Gamepad disconnected").into();
        }
        if self.last_poll.elapsed() > Duration::from_secs(if self.dev.is_some() { 10 } else { 2 }) {
            if self.dev.is_some() {
                self.last_poll = Instant::now();
                match self.call(proto::request(cmd::HEARTBEAT, &[])) {
                    Some(r) => self.info = Some(proto::parse_heartbeat(&r)),
                    None => {
                        // Reconnecting picks whichever pad answers, so a hung one is let go.
                        self.dev = None;
                        self.error = t!("The gamepad stopped answering, reconnecting").into();
                    }
                }
            } else {
                self.connect();
            }
        }
        ui.ctx().request_repaint_after(Duration::from_secs(1));
        if self.vibration_stop.is_some_and(|t| Instant::now() >= t) {
            self.vibration_stop = None;
            self.call(proto::request(cmd::VIBRATION, &[0; 4]));
        } else if self.vibration_stop.is_some() {
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        }
        if self.dev.is_some() {
            self.set_test_mode(self.tab == Tab::Test);
        }

        let sidebar = self.colors.get(ui.visuals().dark_mode).sidebar_bg;
        let nav = egui::Frame::side_top_panel(ui.style()).fill(sidebar).inner_margin(egui::Margin { left: 12, right: 12, top: 6, bottom: 16 });
        egui::Panel::left("nav").resizable(false).exact_size(230.0).frame(nav).show(ui, |ui| {
            let (rect, head) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 34.0), egui::Sense::click_and_drag());
            let font = egui::FontId::proportional(21.0);
            ui.painter().text(rect.left_center(), egui::Align2::LEFT_CENTER, "OrbitStation", font, ui.visuals().strong_text_color());
            ui::drag_window(ui.ctx(), &head);
            let sections: [(&str, &[Tab]); 3] = [
                (t!("Gamepad"), &[Tab::Device, Tab::Test]),
                (t!("Profile"), &[Tab::Remap, Tab::Sticks, Tab::Gyro, Tab::Vibration, Tab::Lighting]),
                (t!("Application"), &[Tab::Settings]),
            ];
            for (title, tabs) in sections {
                ui::nav_section(ui, title);
                ui.spacing_mut().item_spacing.y = 4.0;
                for &t in tabs {
                    if ui::nav_item(ui, self.tab == t, t.icon(), t.name()) {
                        self.tab = t;
                    }
                }
            }
        });
        let (page_icon, page) = if self.dev.is_none() && self.tab != Tab::Settings {
            (icon::PLUGS, t!("Waiting for the gamepad…"))
        } else {
            (self.tab.icon(), self.tab.name())
        };
        let bar = egui::Frame::side_top_panel(ui.style()).inner_margin(egui::Margin { left: 24, right: 8, top: 4, bottom: 4 });
        egui::Panel::top("titlebar").exact_size(46.0).frame(bar).show_separator_line(false).show(ui, |ui| ui::title_bar(ui, page_icon, page));
        let status = if self.dev.is_some() { &self.note } else { &self.error };
        let bar = egui::Frame::side_top_panel(ui.style()).inner_margin(egui::Margin::symmetric(16, 6));
        egui::Panel::bottom("status").frame(bar).show(ui, |ui| {
            ui.label(RichText::new(if status.is_empty() { String::new() } else { ui::label(icon::INFO, status) }).color(ui.visuals().weak_text_color()));
        });
        if self.dev.is_some() && self.tab.edits_profile() && self.profile_loaded() {
            let bar = egui::Frame::side_top_panel(ui.style()).inner_margin(egui::Margin::symmetric(16, 8));
            egui::Panel::bottom("actions").frame(bar).show(ui, |ui| self.profile_actions(ui));
        }
        let page = egui::Frame::central_panel(ui.style()).inner_margin(egui::Margin { left: 24, right: 24, top: 4, bottom: 16 });
        egui::CentralPanel::default().frame(page).show(ui, |ui| {
            if self.dev.is_none() && self.tab != Tab::Settings {
                ui::card(ui, icon::INFO, t!("Gamepad"), |ui| ui.label(&self.error));
                return;
            }
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                match self.tab {
                    Tab::Device => self.device_tab(ui),
                    Tab::Test => self.test_tab(ui),
                    Tab::Remap => self.profile_tab(ui, remap_editor),
                    Tab::Sticks => self.profile_tab(ui, sticks_editor),
                    Tab::Gyro => self.profile_tab(ui, gyro_editor),
                    Tab::Vibration => self.profile_tab(ui, vibration_editor),
                    Tab::Lighting => self.profile_tab(ui, lighting_editor),
                    Tab::Settings => self.settings_tab(ui),
                }
            });
        });
        ui::resize_edges(ui.ctx());
    }

    fn on_exit(&mut self) {
        if self.dev.is_some() {
            self.call(proto::request(cmd::VIBRATION, &[0, 0, 0, 0]));
            self.set_test_mode(false);
        }
    }
}

fn main() -> eframe::Result {
    i18n::set(i18n::load());
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 720.0])
            .with_min_inner_size([760.0, 520.0])
            .with_title("OrbitStation")
            // the app draws its own title bar, see ui::title_bar
            .with_decorations(false)
            // Matches orbitstation.desktop, so docks and task bars pick up the installed icon.
            .with_app_id("orbitstation")
            .with_icon(eframe::icon_data::from_png_bytes(include_bytes!("../assets/icons/orbitstation-128.png")).unwrap_or_default()),
        ..Default::default()
    };
    eframe::run_native("OrbitStation", options, Box::new(|cc| Ok(Box::new(App::new(&cc.egui_ctx)))))
}
