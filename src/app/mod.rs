//! The window: a sidebar with the connected devices and their pages, the page of the selected
//! device, and the bars around it.

mod pages;
mod storage;
pub mod theme;
mod view;
pub mod widgets;

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use eframe::egui::{self, RichText, Vec2};

use pages::Tab;
use view::DeviceView;
use widgets::icon;

use crate::station::{Kind, Station};

pub use storage::{read_config, write_config};

/// How often the HID bus is searched for new devices.
const SCAN: Duration = Duration::from_secs(2);

/// The app, as eframe runs it.
pub struct App {
    station: Station,
    views: HashMap<PathBuf, DeviceView>,
    selected: Option<PathBuf>,
    tab: Tab,
    /// Why claimed nodes failed to open on the last scan, shown while no device is connected.
    scan_errors: Vec<String>,
    last_scan: Option<Instant>,
    /// Outcome of app-wide actions, for the status line.
    note: String,
    /// The zoom last saved, Ctrl +/- change it too.
    zoom: f32,
    colors: theme::Colors,
    watcher: theme::Watcher,
}

impl App {
    /// Styles the window and starts with no devices; the first frame scans for them.
    pub fn new(ctx: &egui::Context) -> App {
        widgets::install_fonts(ctx);
        let zoom = read_config("zoom").and_then(|z| z.parse().ok()).filter(|z| (0.5..=3.0).contains(z)).unwrap_or(1.0);
        ctx.set_zoom_factor(zoom);
        App {
            station: Station::default(),
            views: HashMap::new(),
            selected: None,
            tab: Tab::Device,
            scan_errors: vec![],
            last_scan: None,
            note: String::new(),
            zoom,
            colors: theme::apply(ctx),
            watcher: theme::Watcher::new(ctx),
        }
    }

    /// Opens new devices, forgets gone ones and keeps a device selected while there is one.
    fn scan(&mut self) {
        if self.last_scan.is_some_and(|t| t.elapsed() < SCAN) {
            return;
        }
        self.last_scan = Some(Instant::now());
        self.scan_errors = self.station.scan().iter().map(ToString::to_string).collect();
        let paths: Vec<PathBuf> = self.station.devices().map(|d| d.path().to_path_buf()).collect();
        self.views.retain(|p, _| paths.contains(p));
        for path in &paths {
            if !self.views.contains_key(path) {
                let mut view = DeviceView::default();
                if let Some(dev) = self.station.get(path) {
                    view.refresh(dev);
                }
                self.views.insert(path.clone(), view);
            }
        }
        if self.selected.as_ref().is_none_or(|s| !paths.contains(s)) {
            self.selected = paths.first().cloned();
        }
    }

    /// Checks in with every device, lets go of the ones that stopped answering and runs the timers.
    fn poll(&mut self) {
        let paths: Vec<PathBuf> = self.views.keys().cloned().collect();
        for path in paths {
            let (Some(dev), Some(view)) = (self.station.get(&path), self.views.get_mut(&path)) else {
                continue;
            };
            let testing = self.selected.as_ref() == Some(&path) && self.tab == Tab::Test;
            view.tick(dev, testing);
            if view.poll(dev).is_err() {
                // a fresh scan opens it again, picking whichever node answers
                self.note = t!("The gamepad stopped answering, reconnecting").into();
                self.station.drop_device(&path);
                self.views.remove(&path);
            }
        }
    }

    /// Names in the sidebar: the nickname, else the model; equal names get a number.
    fn device_names(&self) -> Vec<(PathBuf, Kind, String)> {
        let mut out: Vec<(PathBuf, Kind, String)> = self
            .station
            .devices()
            .map(|d| {
                let nick = self.views.get(d.path()).map(|v| v.nickname.trim()).unwrap_or_default();
                let name = if nick.is_empty() { crate::i18n::tr(d.model().name).to_string() } else { nick.to_string() };
                (d.path().to_path_buf(), d.model().kind, name)
            })
            .collect();
        let names: Vec<String> = out.iter().map(|e| e.2.clone()).collect();
        for (i, entry) in out.iter_mut().enumerate() {
            let same = names.iter().filter(|n| **n == names[i]).count();
            if same > 1 {
                let nth = names[..=i].iter().filter(|n| **n == names[i]).count();
                entry.2 = format!("{} {nth}", entry.2);
            }
        }
        out
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        let (rect, head) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 34.0), egui::Sense::click_and_drag());
        let font = egui::FontId::proportional(21.0);
        let color = ui.visuals().strong_text_color();
        ui.painter().text(rect.left_center(), egui::Align2::LEFT_CENTER, "OrbitStation", font, color);
        widgets::drag_window(ui.ctx(), &head);
        ui.spacing_mut().item_spacing.y = 4.0;

        let devices = self.device_names();
        if !devices.is_empty() {
            widgets::nav_section(ui, t!("Devices"));
        }
        for (path, kind, name) in devices {
            let kind_icon = if kind == Kind::Gamepad { icon::GAMEPAD } else { icon::PLUGS };
            if widgets::nav_item(ui, self.selected.as_ref() == Some(&path), kind_icon, &name) {
                self.selected = Some(path);
            }
        }
        if let Some(dev) = self.selected.as_ref().and_then(|p| self.station.get(p)) {
            for (title, tabs) in [(t!("Gamepad"), &Tab::DEVICE[..]), (t!("Profile"), &Tab::PROFILE[..])] {
                let shown: Vec<Tab> = tabs.iter().copied().filter(|t| t.fits(dev)).collect();
                if shown.is_empty() {
                    continue;
                }
                widgets::nav_section(ui, title);
                for t in shown {
                    if widgets::nav_item(ui, self.tab == t, t.icon(), t.name()) {
                        self.tab = t;
                    }
                }
            }
        }
        widgets::nav_section(ui, t!("Application"));
        if widgets::nav_item(ui, self.tab == Tab::Settings, Tab::Settings.icon(), Tab::Settings.name()) {
            self.tab = Tab::Settings;
        }
    }

    fn page(&mut self, ui: &mut egui::Ui) {
        let selected = self.selected.clone();
        let device = selected.as_ref().and_then(|p| Some((self.station.get(p)?, self.views.get_mut(p)?)));
        if self.tab == Tab::Settings {
            if let Some(e) = pages::app_settings(ui) {
                self.note = e;
            }
            if let Some((dev, view)) = device {
                Tab::Settings.show(ui, dev, view);
            }
            return;
        }
        let Some((dev, view)) = device else {
            widgets::card(ui, icon::INFO, t!("Gamepad"), |ui| {
                ui.label(t!("No Flydigi device found (connect it with a cable)"));
                for e in &self.scan_errors {
                    ui.colored_label(ui.visuals().warn_fg_color, e);
                }
            });
            return;
        };
        if !self.tab.fits(dev) {
            self.tab = Tab::Device;
        }
        self.tab.show(ui, dev, view);
    }

    fn page_title(&mut self) -> (&'static str, &'static str) {
        let connected = self.selected.as_ref().is_some_and(|p| self.views.contains_key(p));
        if connected || self.tab == Tab::Settings {
            (self.tab.icon(), self.tab.name())
        } else {
            (icon::PLUGS, t!("Waiting for the gamepad…"))
        }
    }

    fn status(&self) -> String {
        let view_note = self.selected.as_ref().and_then(|p| self.views.get(p)).map(|v| v.note.as_str());
        match view_note {
            Some(n) if !n.is_empty() => n.to_string(),
            _ => self.note.clone(),
        }
    }
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
        self.scan();
        self.poll();
        ui.ctx().request_repaint_after(Duration::from_secs(1));
        if self.views.values().any(|v| v.rumble_stop.is_some()) {
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        }

        let sidebar = self.colors.get(ui.visuals().dark_mode).sidebar_bg;
        let margin = egui::Margin { left: 12, right: 12, top: 6, bottom: 16 };
        let nav = egui::Frame::side_top_panel(ui.style()).fill(sidebar).inner_margin(margin);
        egui::Panel::left("nav").resizable(false).exact_size(230.0).frame(nav).show(ui, |ui| self.sidebar(ui));

        let (page_icon, page_name) = self.page_title();
        let bar = egui::Frame::side_top_panel(ui.style()).inner_margin(egui::Margin {
            left: 24,
            right: 8,
            top: 4,
            bottom: 4,
        });
        egui::Panel::top("titlebar")
            .exact_size(46.0)
            .frame(bar)
            .show_separator_line(false)
            .show(ui, |ui| widgets::title_bar(ui, page_icon, page_name));

        let status = self.status();
        let bar = egui::Frame::side_top_panel(ui.style()).inner_margin(egui::Margin::symmetric(16, 6));
        egui::Panel::bottom("status").frame(bar).show(ui, |ui| {
            let text = if status.is_empty() { String::new() } else { widgets::label(icon::INFO, &status) };
            ui.label(RichText::new(text).color(ui.visuals().weak_text_color()));
        });

        if self.tab.edits_profile() {
            let selected = self.selected.clone();
            let device = selected.as_ref().and_then(|p| Some((self.station.get(p)?, self.views.get_mut(p)?)));
            if let Some((dev, view)) = device.filter(|(_, v)| v.has_mapping()) {
                let bar = egui::Frame::side_top_panel(ui.style()).inner_margin(egui::Margin::symmetric(16, 8));
                egui::Panel::bottom("actions").frame(bar).show(ui, |ui| pages::profile_actions(ui, dev, view));
            }
        }

        let page = egui::Frame::central_panel(ui.style()).inner_margin(egui::Margin {
            left: 24,
            right: 24,
            top: 4,
            bottom: 16,
        });
        egui::CentralPanel::default().frame(page).show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| self.page(ui));
        });
        widgets::resize_edges(ui.ctx());
    }

    fn on_exit(&mut self) {
        for (path, view) in &mut self.views {
            if let Some(dev) = self.station.get(path) {
                view.shut_down(dev);
            }
        }
    }
}
