mod device;
mod proto;

use device::Device;
use eframe::egui::{self, Color32, Pos2, Stroke, Vec2};
use proto::{DeviceInfo, HwStatus, Toggle, cmd};
use std::time::{Duration, Instant};

#[derive(PartialEq, Clone, Copy)]
enum Tab {
    Device,
    Test,
    Settings,
}

struct App {
    dev: Option<Device>,
    error: String,
    last_poll: Instant,
    info: Option<DeviceInfo>,
    status: Option<HwStatus>,
    profile: u8,
    tab: Tab,
    test_mode: bool,
    vibration: [u8; 2],
    note: String,
}

impl App {
    fn new() -> App {
        let mut app = App {
            dev: None,
            error: String::new(),
            last_poll: Instant::now(),
            info: None,
            status: None,
            profile: 0,
            tab: Tab::Device,
            test_mode: false,
            vibration: [0, 0],
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
            self.note = format!("Геймпад не ответил на команду 0x{:02x}", pkt[2]);
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
            Some(_) => format!("Сохранено: {what}"),
            None => format!("Не удалось применить: {what}"),
        };
    }

    fn device_tab(&mut self, ui: &mut egui::Ui) {
        let Some(info) = self.info.clone() else {
            ui.label("Нет данных от геймпада.");
            return;
        };
        egui::Grid::new("info").num_columns(2).spacing([24.0, 6.0]).show(ui, |ui| {
            ui.label("Модель");
            ui.strong(proto::model_name(info.device_id));
            ui.end_row();
            ui.label("ID устройства");
            ui.label(info.device_id.to_string());
            ui.end_row();
            ui.label("Прошивка");
            ui.label(&info.firmware);
            ui.end_row();
            ui.label("Радиомодуль");
            ui.label(&info.nearlink);
            ui.end_row();
            ui.label("Батарея");
            ui.label(if info.charging { "Заряжается".to_string() } else { format!("{} / 5", info.battery) });
            ui.end_row();
            ui.label("Узел");
            ui.monospace(self.dev.as_ref().map(|d| d.path.display().to_string()).unwrap_or_default());
            ui.end_row();
        });
        ui.add_space(16.0);
        ui.heading("Профиль");
        ui.horizontal(|ui| {
            for p in 0..4u8 {
                if ui.selectable_label(self.profile == p, format!("Профиль {}", p + 1)).clicked() && self.profile != p {
                    self.profile = p;
                    self.apply(proto::request(cmd::APPLY_PROFILE, &[p]), &format!("профиль {}", p + 1));
                }
            }
        });
        ui.add_space(8.0);
        if ui.button("Обновить").clicked() {
            self.refresh();
        }
    }

    fn test_tab(&mut self, ui: &mut egui::Ui) {
        let input = self.dev.as_ref().and_then(|d| *d.input.lock().unwrap()).unwrap_or_default();
        ui.ctx().request_repaint_after(Duration::from_millis(16));
        ui.horizontal(|ui| {
            stick(ui, "Левый стик", input.lx, input.ly, input.buttons & 1 << 14 != 0);
            stick(ui, "Правый стик", input.rx, input.ry, input.buttons & 1 << 15 != 0);
            ui.vertical(|ui| {
                ui.label("Триггеры");
                ui.add(egui::ProgressBar::new(input.lt as f32 / 255.0).text(format!("LT {}", input.lt)).desired_width(160.0));
                ui.add(egui::ProgressBar::new(input.rt as f32 / 255.0).text(format!("RT {}", input.rt)).desired_width(160.0));
            });
        });
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            for (i, name) in proto::BUTTONS.iter().enumerate().filter(|(_, n)| !n.is_empty()) {
                let on = input.buttons & 1 << i != 0;
                let text = egui::RichText::new(*name).monospace();
                ui.add(egui::Button::new(if on { text.color(Color32::BLACK) } else { text })
                    .fill(if on { Color32::from_rgb(120, 200, 120) } else { ui.visuals().faint_bg_color })
                    .min_size(Vec2::new(48.0, 28.0)));
            }
        });
        ui.add_space(8.0);
        ui.monospace(format!("Гироскоп      {:>7} {:>7} {:>7}", input.gyro[0], input.gyro[1], input.gyro[2]));
        ui.monospace(format!("Акселерометр  {:>7} {:>7} {:>7}", input.accel[0], input.accel[1], input.accel[2]));
        ui.add_space(16.0);
        ui.heading("Вибрация");
        let old = self.vibration;
        ui.add(egui::Slider::new(&mut self.vibration[0], 0..=255).text("левый мотор"));
        ui.add(egui::Slider::new(&mut self.vibration[1], 0..=255).text("правый мотор"));
        if old != self.vibration {
            let [l, r] = self.vibration;
            self.call(proto::request(cmd::VIBRATION, &[l, r, 0, 0]));
        }
    }

    fn settings_tab(&mut self, ui: &mut egui::Ui) {
        let Some(mut st) = self.status.clone() else {
            ui.label("Нет данных о настройках.");
            return;
        };
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
        ui.add_space(8.0);
        let choice = |ui: &mut egui::Ui, title: &str, value: &mut u8, opts: &[(u8, &str)], c: u8, app: &mut App| {
            ui.label(title);
            ui.horizontal_wrapped(|ui| {
                for (v, name) in opts {
                    if ui.radio_value(value, *v, *name).clicked() {
                        app.apply(proto::request(c, &[*v]), title);
                    }
                }
            });
            ui.add_space(6.0);
        };
        if st.report_rate != 0 {
            choice(ui, "Частота опроса стиков", &mut st.report_rate, &proto::REPORT_RATES, cmd::REPORT_RATE, self);
        }
        if st.precision != 0 {
            choice(ui, "Точность стиков", &mut st.precision, &proto::PRECISIONS, cmd::PRECISION, self);
        }
        if st.sensitivity != 0 {
            choice(ui, "Чувствительность в центре стика", &mut st.sensitivity, &proto::SENSITIVITIES, cmd::SENSITIVITY, self);
        }
        choice(ui, "Время до сна", &mut st.sleep_time, &proto::SLEEP_TIMES, cmd::SLEEP_TIME, self);
        self.status = Some(st);
    }
}

fn stick(ui: &mut egui::Ui, title: &str, x: i16, y: i16, pressed: bool) {
    ui.vertical(|ui| {
        ui.label(title);
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(120.0), egui::Sense::hover());
        let p = ui.painter();
        let r = rect.width() / 2.0 - 4.0;
        p.circle_stroke(rect.center(), r, Stroke::new(1.5, ui.visuals().weak_text_color()));
        let dot = rect.center() + Vec2::new(x as f32, -(y as f32)) / 32768.0 * r;
        let color = if pressed { Color32::from_rgb(120, 200, 120) } else { ui.visuals().strong_text_color() };
        p.circle_filled(Pos2::new(dot.x, dot.y), 7.0, color);
        ui.monospace(format!("{x:>6} {y:>6}"));
    });
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.dev.as_ref().is_some_and(|d| !d.is_alive()) {
            self.dev = None;
            self.error = "Геймпад отключён".into();
        }
        if self.last_poll.elapsed() > Duration::from_secs(if self.dev.is_some() { 10 } else { 2 }) {
            if self.dev.is_some() {
                self.last_poll = Instant::now();
                if let Some(r) = self.call(proto::request(cmd::HEARTBEAT, &[])) {
                    self.info = Some(proto::parse_heartbeat(&r));
                }
            } else {
                self.connect();
            }
        }
        ui.ctx().request_repaint_after(Duration::from_secs(1));
        if self.dev.is_some() {
            self.set_test_mode(self.tab == Tab::Test);
        }

        egui::Panel::left("nav").resizable(false).exact_size(170.0).show(ui, |ui| {
            ui.add_space(8.0);
            ui.heading("OrbitStation");
            ui.add_space(8.0);
            for (t, name) in [(Tab::Device, "Устройство"), (Tab::Test, "Тест кнопок"), (Tab::Settings, "Настройки")] {
                ui.selectable_value(&mut self.tab, t, name);
            }
        });
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.label(if self.dev.is_some() { &self.note } else { &self.error });
        });
        egui::CentralPanel::default_margins().show(ui, |ui| {
            if self.dev.is_none() {
                ui.heading("Ожидание геймпада…");
                ui.label(&self.error);
                return;
            }
            egui::ScrollArea::vertical().show(ui, |ui| match self.tab {
                Tab::Device => self.device_tab(ui),
                Tab::Test => self.test_tab(ui),
                Tab::Settings => self.settings_tab(ui),
            });
        });
    }

    fn on_exit(&mut self) {
        if self.dev.is_some() {
            self.call(proto::request(cmd::VIBRATION, &[0, 0, 0, 0]));
            self.set_test_mode(false);
        }
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([760.0, 560.0]).with_title("OrbitStation"),
        ..Default::default()
    };
    eframe::run_native("OrbitStation", options, Box::new(|_| Ok(Box::new(App::new()))))
}
