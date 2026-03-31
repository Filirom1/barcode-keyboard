#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui;
use n0_future::StreamExt;

use barcode_keyboard::node::{AcceptEvent, EchoNode};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // --terminal [base-url]  → skip GUI entirely
    if args.get(1).map(|s| s.as_str()) == Some("--terminal") {
        let base_url = args.get(2).cloned().unwrap_or_default();
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(run_terminal(base_url));
        return;
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([380.0, 600.0])
            .with_min_inner_size([320.0, 400.0])
            .with_title("Barcode Keyboard"),
        ..Default::default()
    };
    if let Err(e) = eframe::run_native(
        "Barcode Keyboard",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    ) {
        eprintln!("GUI failed to start: {e}");
        eprintln!();
        eprintln!("Use terminal mode instead:");
        eprintln!("  keyboard --terminal [https://your-site.netlify.app]");
        std::process::exit(1);
    }
}

// ── Terminal mode ─────────────────────────────────────────────────────────────

async fn run_terminal(base_url: String) {
    println!("Starting iroh node…");
    let node = match EchoNode::spawn().await {
        Ok(n) => n,
        Err(e) => {
            eprintln!("iroh error: {e}");
            return;
        }
    };

    let id = node.endpoint().id().to_string();

    if !base_url.is_empty() {
        let url = format!(
            "{}/scanner.html?endpoint={}",
            base_url.trim_end_matches('/'),
            id
        );
        println!("Scanner URL: {url}");
        println!();
        if let Ok(qr) = qrcode::QrCode::new(url.as_bytes()) {
            use qrcode::render::unicode;
            let image = qr
                .render::<unicode::Dense1x2>()
                .dark_color(unicode::Dense1x2::Dark)
                .light_color(unicode::Dense1x2::Light)
                .build();
            println!("{image}");
        }
    } else {
        println!("Endpoint ID: {id}");
        println!();
        println!("Tip: pass your site URL to get the scanner QR code:");
        println!("  keyboard --terminal https://your-site.netlify.app");
        println!();
    }

    // Try xdotool subprocess first (works in nix-shell without dynamic lib issues),
    // then fall back to enigo, then to print-only.
    let keyboard_mode = detect_keyboard_mode();
    println!("Keyboard mode: {keyboard_mode}");
    println!("Waiting for barcode scans…");

    let mut last_scan: Option<(String, Instant)> = None;
    let mut enigo = if keyboard_mode == "enigo" {
        enigo::Enigo::new(&enigo::Settings::default()).ok()
    } else {
        None
    };
    let start = Instant::now();

    let mut events = node.accept_events();
    while let Some(event) = events.next().await {
        if let AcceptEvent::Received { content, .. } = event {
            let now = Instant::now();
            let is_dup = last_scan.as_ref().is_some_and(|(prev, t)| {
                prev == &content && now.duration_since(*t) < Duration::from_secs(10)
            });
            if is_dup {
                println!("(dup ignored: {content})");
                continue;
            }

            let s = start.elapsed().as_secs();
            println!("[{:02}:{:02}:{:02}] {content}", s / 3600, (s / 60) % 60, s % 60);

            match keyboard_mode.as_str() {
                "powershell" => {
                    type_powershell(&content);
                }
                "xdotool" => {
                    type_xdotool(&content);
                }
                "enigo" => {
                    if let Some(e) = &mut enigo {
                        use enigo::{Direction, Key, Keyboard};
                        let _ = e.text(&content);
                        let _ = e.key(Key::Return, Direction::Click);
                    }
                }
                _ => {}
            }

            last_scan = Some((content, now));
        }
    }
}

fn detect_keyboard_mode() -> String {
    // WSL2: inject via powershell.exe SendKeys (types into focused Windows window)
    if is_wsl() {
        return "powershell".to_string();
    }
    // Native Linux: prefer xdotool subprocess (avoids dynamic lib issues in nix-shell)
    if std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok() {
        if which_xdotool() {
            return "xdotool".to_string();
        }
        return "enigo".to_string();
    }
    "print-only".to_string()
}

fn is_wsl() -> bool {
    std::env::var("WSL_DISTRO_NAME").is_ok()
}

fn which_xdotool() -> bool {
    std::process::Command::new("xdotool")
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn type_xdotool(text: &str) {
    let _ = std::process::Command::new("xdotool")
        .args(["type", "--clearmodifiers", "--delay", "0", "--", text])
        .status();
    let _ = std::process::Command::new("xdotool")
        .args(["key", "Return"])
        .status();
}

fn type_powershell(text: &str) {
    // SendKeys special chars: + ^ % ~ ( ) [ ] { } must be wrapped in {}
    let escaped: String = text
        .chars()
        .flat_map(|c| match c {
            '+' | '^' | '%' | '~' | '(' | ')' | '[' | ']' | '{' | '}' => {
                vec!['{', c, '}']
            }
            c => vec![c],
        })
        .collect();
    let cmd = format!(
        "Add-Type -AssemblyName System.Windows.Forms; \
         [System.Windows.Forms.SendKeys]::SendWait('{escaped}'); \
         [System.Windows.Forms.SendKeys]::SendWait('{{ENTER}}')"
    );
    let _ = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &cmd])
        .status();
}

// ── GUI mode ──────────────────────────────────────────────────────────────────

#[derive(Default)]
struct Shared {
    endpoint_id: Option<String>,
    pending: Vec<String>,
}

struct App {
    base_url: String,
    base_url_confirmed: String,
    shared: Arc<Mutex<Shared>>,
    enigo: Option<enigo::Enigo>,
    last_scan: Option<(String, Instant)>,
    history: Vec<(String, String)>,
    start: Instant,
    qr_texture: Option<egui::TextureHandle>,
    qr_for_url: String,
}

impl App {
    fn new(cc: &eframe::CreationContext) -> Self {
        let shared: Arc<Mutex<Shared>> = Arc::default();
        let shared_bg = shared.clone();
        let ctx = cc.egui_ctx.clone();

        std::thread::spawn(move || {
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(async move {
                    let node = match EchoNode::spawn().await {
                        Ok(n) => n,
                        Err(e) => {
                            eprintln!("iroh error: {e}");
                            return;
                        }
                    };
                    shared_bg.lock().unwrap().endpoint_id =
                        Some(node.endpoint().id().to_string());
                    ctx.request_repaint();

                    let mut events = node.accept_events();
                    while let Some(event) = events.next().await {
                        if let AcceptEvent::Received { content, .. } = event {
                            shared_bg.lock().unwrap().pending.push(content);
                            ctx.request_repaint();
                        }
                    }
                });
        });

        Self {
            base_url: String::new(),
            base_url_confirmed: String::new(),
            shared,
            enigo: enigo::Enigo::new(&enigo::Settings::default()).ok(),
            last_scan: None,
            history: Vec::new(),
            start: Instant::now(),
            qr_texture: None,
            qr_for_url: String::new(),
        }
    }

    fn scanner_url(&self) -> Option<String> {
        let id = self.shared.lock().unwrap().endpoint_id.clone()?;
        if self.base_url_confirmed.is_empty() {
            return None;
        }
        Some(format!(
            "{}/scanner.html?endpoint={}",
            self.base_url_confirmed.trim_end_matches('/'),
            id
        ))
    }

    fn rebuild_qr_if_needed(&mut self, url: &str, ctx: &egui::Context) {
        if self.qr_for_url == url {
            return;
        }
        let Ok(qr) = qrcode::QrCode::new(url.as_bytes()) else {
            return;
        };
        let modules = qr.to_colors();
        let w = qr.width();
        let pad = 3usize;
        let size = w + pad * 2;
        let mut rgba = vec![255u8; size * size * 4];
        for (i, c) in modules.iter().enumerate() {
            let row = i / w + pad;
            let col = i % w + pad;
            let b = (row * size + col) * 4;
            let v = if *c == qrcode::Color::Dark { 0u8 } else { 255u8 };
            rgba[b] = v;
            rgba[b + 1] = v;
            rgba[b + 2] = v;
            rgba[b + 3] = 255;
        }
        self.qr_texture = Some(ctx.load_texture(
            "qr",
            egui::ColorImage::from_rgba_unmultiplied([size, size], &rgba),
            egui::TextureOptions::NEAREST,
        ));
        self.qr_for_url = url.to_string();
    }

    fn elapsed_str(&self) -> String {
        let s = self.start.elapsed().as_secs();
        format!("{:02}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let pending: Vec<String> = std::mem::take(&mut self.shared.lock().unwrap().pending);

        for code in pending {
            let now = Instant::now();
            let is_dup = self.last_scan.as_ref().is_some_and(|(prev, t)| {
                prev == &code && now.duration_since(*t) < Duration::from_secs(10)
            });
            if is_dup {
                continue;
            }

            if let Some(e) = &mut self.enigo {
                use enigo::{Direction, Key, Keyboard};
                let _ = e.text(&code);
                let _ = e.key(Key::Return, Direction::Click);
            }

            let timestamp = self.elapsed_str();
            self.history.insert(0, (code.clone(), timestamp));
            self.last_scan = Some((code, now));
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Barcode Keyboard");
            ui.add_space(8.0);

            // URL input
            ui.horizontal(|ui| {
                ui.label("Site URL:");
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.base_url)
                        .hint_text("https://xxx.netlify.app")
                        .desired_width(f32::INFINITY),
                );
                let pressed_enter =
                    resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if ui.button("OK").clicked() || pressed_enter {
                    if !self.base_url.is_empty() {
                        self.base_url_confirmed = self.base_url.clone();
                    }
                }
            });
            ui.add_space(6.0);

            // iroh status
            let endpoint_id = self.shared.lock().unwrap().endpoint_id.clone();
            if endpoint_id.is_none() {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Starting iroh node…");
                });
            } else {
                ui.colored_label(egui::Color32::from_rgb(74, 222, 128), "● Ready");
            }
            ui.add_space(8.0);

            // QR code
            if let Some(url) = self.scanner_url() {
                self.rebuild_qr_if_needed(&url, ctx);
                ui.vertical_centered(|ui| {
                    if let Some(tex) = &self.qr_texture {
                        ui.image((tex.id(), egui::Vec2::splat(220.0)));
                    }
                    ui.add_space(4.0);
                    ui.add(egui::Label::new(egui::RichText::new(&url).small().weak()).wrap());
                });
            } else if endpoint_id.is_some() {
                ui.vertical_centered(|ui| {
                    ui.colored_label(
                        egui::Color32::GRAY,
                        "Enter your site URL above to get the QR code",
                    );
                });
            }

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);

            // History
            ui.label(format!("Received: {}", self.history.len()));
            ui.add_space(4.0);

            egui::ScrollArea::vertical().show(ui, |ui| {
                if self.history.is_empty() {
                    ui.colored_label(egui::Color32::GRAY, "Waiting for scans…");
                }
                for (code, time) in &self.history {
                    ui.horizontal(|ui| {
                        ui.monospace(code);
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.weak(time);
                            },
                        );
                    });
                }
            });
        });
    }
}
