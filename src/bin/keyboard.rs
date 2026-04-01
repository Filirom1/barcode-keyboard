#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui;
use n0_future::StreamExt;

use barcode_keyboard::node::{AcceptEvent, EchoNode};

// ── i18n ──────────────────────────────────────────────────────────────────────

struct Lang {
    title: &'static str,
    site_url: &'static str,
    url_hint: &'static str,
    starting_node: &'static str,
    ready: &'static str,
    enter_url_hint: &'static str,
    received: &'static str,
    waiting: &'static str,
    copy_all: &'static str,
    clear: &'static str,
    preferences: &'static str,
    formats_label: &'static str,
}

const EN: Lang = Lang {
    title: "Barcode Keyboard",
    site_url: "Site URL:",
    url_hint: "https://filirom1.github.io/barcode-keyboard",
    starting_node: "Starting iroh node…",
    ready: "● Ready",
    enter_url_hint: "Enter your site URL above to get the QR code",
    received: "Received",
    waiting: "Waiting for scans…",
    copy_all: "Copy all",
    clear: "Clear",
    preferences: "Preferences",
    formats_label: "Barcode formats:",
};

const FR: Lang = Lang {
    title: "Clavier Code-barres",
    site_url: "URL du site :",
    url_hint: "https://filirom1.github.io/barcode-keyboard",
    starting_node: "Démarrage du nœud iroh…",
    ready: "● Prêt",
    enter_url_hint: "Entrez l'URL du site ci-dessus pour obtenir le QR code",
    received: "Reçus",
    waiting: "En attente de scans…",
    copy_all: "Tout copier",
    clear: "Effacer",
    preferences: "Préférences",
    formats_label: "Formats de codes-barres :",
};

fn detect_lang() -> &'static Lang {
    let locale = sys_locale::get_locale().unwrap_or_default();
    if locale.starts_with("fr") { &FR } else { &EN }
}

// ── Preference enums ──────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum Suffix { #[default] Enter, Tab, None }

#[derive(Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum DedupMode { #[default] Consecutive, Any, Off }

#[derive(Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum Transform { #[default] None, Upper, Lower, Trim }

#[derive(Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum Camera { #[default] Rear, Front }

// ── Formats ───────────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq)]
struct Formats {
    ean13: bool, ean8: bool, upca: bool, upce: bool,
    code39: bool, code128: bool, qrcode: bool, pdf417: bool,
    itf: bool, codabar: bool, code93: bool, databar: bool,
}

impl Default for Formats {
    fn default() -> Self {
        Self {
            ean13: true, ean8: true, upca: true, upce: false,
            code39: true, code128: true, qrcode: false, pdf417: false,
            itf: false, codabar: false, code93: false, databar: false,
        }
    }
}

impl Formats {
    fn is_default(&self) -> bool { *self == Self::default() }

    fn to_param(&self) -> String {
        let mut v: Vec<&str> = vec![];
        if self.ean13   { v.push("ean13"); }
        if self.ean8    { v.push("ean8"); }
        if self.upca    { v.push("upca"); }
        if self.upce    { v.push("upce"); }
        if self.code39  { v.push("code39"); }
        if self.code128 { v.push("code128"); }
        if self.qrcode  { v.push("qrcode"); }
        if self.pdf417  { v.push("pdf417"); }
        if self.itf     { v.push("itf"); }
        if self.codabar { v.push("codabar"); }
        if self.code93  { v.push("code93"); }
        if self.databar { v.push("databar"); }
        v.join(",")
    }

    fn from_str(s: &str) -> Self {
        let parts: HashSet<&str> = s.split(',').map(str::trim).collect();
        Self {
            ean13:   parts.contains("ean13"),
            ean8:    parts.contains("ean8"),
            upca:    parts.contains("upca"),
            upce:    parts.contains("upce"),
            code39:  parts.contains("code39"),
            code128: parts.contains("code128"),
            qrcode:  parts.contains("qrcode"),
            pdf417:  parts.contains("pdf417"),
            itf:     parts.contains("itf"),
            codabar: parts.contains("codabar"),
            code93:  parts.contains("code93"),
            databar: parts.contains("databar"),
        }
    }
}

// ── Config ────────────────────────────────────────────────────────────────────

fn default_dedup_secs() -> u64 { 10 }
fn default_true() -> bool { true }

#[derive(serde::Serialize, serde::Deserialize)]
struct Config {
    // ── Connection ──────────────────────────────────────────────────────────
    #[serde(default)]
    url: String,

    // ── Phone-side scanner (encoded into QR URL) ────────────────────────────
    /// Comma-separated format names; None means use scanner defaults.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    formats: Option<String>,
    #[serde(default = "default_true")]
    vibrate: bool,
    #[serde(default)]
    camera: Camera,
    #[serde(default)]
    torch: bool,

    // ── PC-side behaviour ───────────────────────────────────────────────────
    #[serde(default)]
    suffix: Suffix,
    #[serde(default = "default_dedup_secs")]
    dedup_secs: u64,
    #[serde(default)]
    dedup_mode: DedupMode,
    #[serde(default)]
    prefix: String,
    #[serde(default)]
    transform: Transform,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ignore_pattern: Option<String>,
    #[serde(default)]
    copy_only: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            url: String::new(),
            formats: None,
            vibrate: true,
            camera: Camera::default(),
            torch: false,
            suffix: Suffix::default(),
            dedup_secs: default_dedup_secs(),
            dedup_mode: DedupMode::default(),
            prefix: String::new(),
            transform: Transform::default(),
            ignore_pattern: None,
            copy_only: false,
        }
    }
}

// ── Config persistence ────────────────────────────────────────────────────────

fn config_dir() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("barcode-keyboard")
}

fn config_file() -> std::path::PathBuf { config_dir().join("config.json") }

fn load_config() -> Config {
    std::fs::read_to_string(config_file())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_config(cfg: &Config) {
    let _ = std::fs::create_dir_all(config_dir());
    if let Ok(json) = serde_json::to_string_pretty(cfg) {
        let _ = std::fs::write(config_file(), json);
    }
}

// ── Scanner URL ───────────────────────────────────────────────────────────────

fn build_scanner_url(base_url: &str, endpoint_id: &str, cfg: &Config) -> String {
    let mut url = format!(
        "{}/scanner.html?endpoint={}",
        base_url.trim_end_matches('/'),
        endpoint_id
    );
    if let Some(fmts) = &cfg.formats {
        url.push_str("&formats=");
        url.push_str(fmts);
    }
    if !cfg.vibrate  { url.push_str("&vibrate=0"); }
    if cfg.camera == Camera::Front { url.push_str("&camera=front"); }
    if cfg.torch     { url.push_str("&torch=1"); }
    url
}

// ── Keyboard injection ────────────────────────────────────────────────────────

#[derive(PartialEq)]
enum KeyboardMode { PowerShell, XDotool, Enigo, PrintOnly }

fn detect_keyboard_mode() -> KeyboardMode {
    if is_wsl() { return KeyboardMode::PowerShell; }
    if std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok() {
        if which_xdotool() { return KeyboardMode::XDotool; }
        return KeyboardMode::Enigo;
    }
    KeyboardMode::PrintOnly
}

fn is_wsl() -> bool { std::env::var("WSL_DISTRO_NAME").is_ok() }

fn which_xdotool() -> bool {
    std::process::Command::new("xdotool").arg("version")
        .output().map(|o| o.status.success()).unwrap_or(false)
}

fn apply_transform(text: &str, transform: Transform) -> String {
    match transform {
        Transform::None  => text.to_string(),
        Transform::Upper => text.to_uppercase(),
        Transform::Lower => text.to_lowercase(),
        Transform::Trim  => text.trim().to_string(),
    }
}

fn inject(mode: &KeyboardMode, enigo: &mut Option<enigo::Enigo>, raw: &str, cfg: &Config) {
    let content = apply_transform(raw, cfg.transform);
    let text = format!("{}{}", cfg.prefix, content);
    match mode {
        KeyboardMode::PowerShell => type_powershell(&text, cfg.suffix),
        KeyboardMode::XDotool    => type_xdotool(&text, cfg.suffix),
        KeyboardMode::Enigo      => { if let Some(e) = enigo { type_enigo(e, &text, cfg.suffix); } }
        KeyboardMode::PrintOnly  => {}
    }
}

fn type_xdotool(text: &str, suffix: Suffix) {
    let _ = std::process::Command::new("xdotool")
        .args(["type", "--clearmodifiers", "--delay", "0", "--", text]).status();
    match suffix {
        Suffix::Enter => { let _ = std::process::Command::new("xdotool").args(["key", "Return"]).status(); }
        Suffix::Tab   => { let _ = std::process::Command::new("xdotool").args(["key", "Tab"]).status(); }
        Suffix::None  => {}
    }
}

fn type_powershell(text: &str, suffix: Suffix) {
    let escaped: String = text.chars().flat_map(|c| match c {
        '+' | '^' | '%' | '~' | '(' | ')' | '[' | ']' | '{' | '}' => vec!['{', c, '}'],
        c => vec![c],
    }).collect();
    let suffix_keys = match suffix {
        Suffix::Enter => "{ENTER}",
        Suffix::Tab   => "{TAB}",
        Suffix::None  => "",
    };
    let cmd = if suffix_keys.is_empty() {
        format!(
            "Add-Type -AssemblyName System.Windows.Forms; \
             [System.Windows.Forms.SendKeys]::SendWait('{escaped}')"
        )
    } else {
        format!(
            "Add-Type -AssemblyName System.Windows.Forms; \
             [System.Windows.Forms.SendKeys]::SendWait('{escaped}'); \
             [System.Windows.Forms.SendKeys]::SendWait('{suffix_keys}')"
        )
    };
    let _ = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &cmd]).status();
}

fn type_enigo(e: &mut enigo::Enigo, text: &str, suffix: Suffix) {
    use enigo::{Direction, Key, Keyboard};
    let _ = e.text(text);
    match suffix {
        Suffix::Enter => { let _ = e.key(Key::Return, Direction::Click); }
        Suffix::Tab   => { let _ = e.key(Key::Tab, Direction::Click); }
        Suffix::None  => {}
    }
}

// ── Dedup ─────────────────────────────────────────────────────────────────────

struct DedupFilter {
    mode: DedupMode,
    secs: u64,
    last: Option<(String, Instant)>,
    seen: HashSet<String>,
}

impl DedupFilter {
    fn from_cfg(cfg: &Config) -> Self {
        Self { mode: cfg.dedup_mode, secs: cfg.dedup_secs, last: None, seen: HashSet::new() }
    }

    fn is_dup(&self, code: &str) -> bool {
        if self.mode == DedupMode::Off || self.secs == 0 { return false; }
        match self.mode {
            DedupMode::Consecutive => self.last.as_ref().is_some_and(|(prev, t)| {
                prev == code && t.elapsed() < Duration::from_secs(self.secs)
            }),
            DedupMode::Any => self.seen.contains(code),
            DedupMode::Off => false,
        }
    }

    fn record(&mut self, code: String) {
        if self.mode == DedupMode::Any { self.seen.insert(code.clone()); }
        self.last = Some((code, Instant::now()));
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.get(1).map(|s| s.as_str()) == Some("--terminal") {
        let cfg = load_config();
        let base_url = args.get(2).cloned().unwrap_or_else(|| cfg.url.clone());
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(run_terminal(base_url, cfg));
        return;
    }

    let lang = detect_lang();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([380.0, 660.0])
            .with_min_inner_size([320.0, 400.0])
            .with_title(lang.title),
        ..Default::default()
    };
    if let Err(e) = eframe::run_native(
        lang.title,
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc, lang)))),
    ) {
        eprintln!("GUI failed to start: {e}");
        eprintln!("Use terminal mode: keyboard --terminal [URL]");
        std::process::exit(1);
    }
}

// ── Terminal mode ─────────────────────────────────────────────────────────────

async fn run_terminal(base_url: String, cfg: Config) {
    let mode = detect_keyboard_mode();
    let mut enigo = if mode == KeyboardMode::Enigo {
        enigo::Enigo::new(&enigo::Settings::default()).ok()
    } else {
        None
    };
    let mut dedup = DedupFilter::from_cfg(&cfg);
    let start = Instant::now();

    println!("Starting iroh node…");
    let node = match EchoNode::spawn().await {
        Ok(n) => n,
        Err(e) => { eprintln!("iroh error: {e}"); return; }
    };
    let id = node.endpoint().id().to_string();

    if !base_url.is_empty() {
        let url = build_scanner_url(&base_url, &id, &cfg);
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
        println!("Tip: keyboard --terminal https://filirom1.github.io/barcode-keyboard");
        println!();
    }

    println!("Waiting for barcode scans…");

    let mut events = node.accept_events();
    while let Some(event) = events.next().await {
        if let AcceptEvent::Received { content, .. } = event {
            if dedup.is_dup(&content) {
                println!("(dup ignored: {content})");
                continue;
            }
            let s = start.elapsed().as_secs();
            println!("[{:02}:{:02}:{:02}] {content}", s / 3600, (s / 60) % 60, s % 60);
            inject(&mode, &mut enigo, &content, &cfg);
            dedup.record(content);
        }
    }
}

// ── GUI mode ──────────────────────────────────────────────────────────────────

#[derive(Default)]
struct Shared {
    endpoint_id: Option<String>,
    pending: Vec<String>,
}

struct App {
    lang: &'static Lang,
    url_edit: String,         // text-field buffer, confirmed into cfg.url on OK
    cfg: Config,              // source of truth for all settings
    formats: Formats,         // parsed from cfg.formats for checkbox UI
    shared: Arc<Mutex<Shared>>,
    keyboard_mode: KeyboardMode,
    enigo: Option<enigo::Enigo>,
    dedup: DedupFilter,
    history: Vec<String>,
    qr_texture: Option<egui::TextureHandle>,
    qr_cache_key: String,     // URL the current texture was built from
}

impl App {
    fn new(cc: &eframe::CreationContext, lang: &'static Lang) -> Self {
        let shared: Arc<Mutex<Shared>> = Arc::default();
        let shared_bg = shared.clone();
        let ctx = cc.egui_ctx.clone();

        std::thread::spawn(move || {
            tokio::runtime::Runtime::new().unwrap().block_on(async move {
                let node = match EchoNode::spawn().await {
                    Ok(n) => n,
                    Err(e) => { eprintln!("iroh error: {e}"); return; }
                };
                shared_bg.lock().unwrap().endpoint_id = Some(node.endpoint().id().to_string());
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

        let cfg = load_config();
        let formats = cfg.formats.as_deref().map(Formats::from_str).unwrap_or_default();
        let dedup = DedupFilter::from_cfg(&cfg);
        let keyboard_mode = detect_keyboard_mode();
        let enigo = if keyboard_mode == KeyboardMode::Enigo {
            enigo::Enigo::new(&enigo::Settings::default()).ok()
        } else {
            None
        };

        Self {
            lang,
            url_edit: cfg.url.clone(),
            formats,
            dedup,
            keyboard_mode,
            enigo,
            cfg,
            shared,
            history: Vec::new(),
            qr_texture: None,
            qr_cache_key: String::new(),
        }
    }

    fn scanner_url(&self) -> Option<String> {
        let id = self.shared.lock().unwrap().endpoint_id.clone()?;
        if self.cfg.url.is_empty() { return None; }
        Some(build_scanner_url(&self.cfg.url, &id, &self.cfg))
    }

    fn rebuild_qr_if_needed(&mut self, url: &str, ctx: &egui::Context) {
        if self.qr_cache_key == url { return; }
        let Ok(qr) = qrcode::QrCode::new(url.as_bytes()) else { return; };
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
            rgba[b] = v; rgba[b+1] = v; rgba[b+2] = v; rgba[b+3] = 255;
        }
        self.qr_texture = Some(ctx.load_texture(
            "qr",
            egui::ColorImage::from_rgba_unmultiplied([size, size], &rgba),
            egui::TextureOptions::NEAREST,
        ));
        self.qr_cache_key = url.to_string();
    }

    fn show_preferences(&mut self, ui: &mut egui::Ui) {
        ui.label(self.lang.formats_label);
        let before = self.formats.clone();
        egui::Grid::new("fmts").num_columns(3).spacing([12.0, 4.0]).show(ui, |ui| {
            ui.checkbox(&mut self.formats.ean13,   "EAN-13");
            ui.checkbox(&mut self.formats.ean8,    "EAN-8");
            ui.checkbox(&mut self.formats.upca,    "UPC-A");
            ui.end_row();
            ui.checkbox(&mut self.formats.upce,    "UPC-E");
            ui.checkbox(&mut self.formats.code39,  "Code 39");
            ui.checkbox(&mut self.formats.code128, "Code 128");
            ui.end_row();
            ui.checkbox(&mut self.formats.qrcode,  "QR Code");
            ui.checkbox(&mut self.formats.pdf417,  "PDF417");
            ui.checkbox(&mut self.formats.itf,     "ITF");
            ui.end_row();
            ui.checkbox(&mut self.formats.codabar, "Codabar");
            ui.checkbox(&mut self.formats.code93,  "Code 93");
            ui.checkbox(&mut self.formats.databar, "DataBar");
            ui.end_row();
        });
        if self.formats != before {
            self.cfg.formats = if self.formats.is_default() { None } else { Some(self.formats.to_param()) };
            save_config(&self.cfg);
            self.qr_cache_key.clear();
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let lang = self.lang;
        let pending: Vec<String> = std::mem::take(&mut self.shared.lock().unwrap().pending);

        for code in pending {
            if self.dedup.is_dup(&code) { continue; }
            inject(&self.keyboard_mode, &mut self.enigo, &code, &self.cfg);
            self.history.insert(0, code.clone());
            self.dedup.record(code);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(lang.title);
            ui.add_space(8.0);

            // URL input
            ui.horizontal(|ui| {
                ui.label(lang.site_url);
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.url_edit)
                        .hint_text(lang.url_hint)
                        .desired_width(f32::INFINITY),
                );
                let pressed_enter =
                    resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if ui.button("OK").clicked() || pressed_enter {
                    if !self.url_edit.is_empty() {
                        self.cfg.url = self.url_edit.clone();
                        save_config(&self.cfg);
                    }
                }
            });
            ui.add_space(6.0);

            // iroh status
            let endpoint_id = self.shared.lock().unwrap().endpoint_id.clone();
            if endpoint_id.is_none() {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(lang.starting_node);
                });
            } else {
                ui.colored_label(egui::Color32::from_rgb(74, 222, 128), lang.ready);
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
                    ui.colored_label(egui::Color32::GRAY, lang.enter_url_hint);
                });
            }

            ui.add_space(6.0);

            // Preferences
            egui::CollapsingHeader::new(lang.preferences)
                .default_open(false)
                .show(ui, |ui| { self.show_preferences(ui); });

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            // History header
            ui.horizontal(|ui| {
                ui.label(format!("{}: {}", lang.received, self.history.len()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button(lang.clear).clicked() {
                        self.history.clear();
                    }
                    if ui.small_button(lang.copy_all).clicked() {
                        let all = self.history.iter().rev()
                            .cloned().collect::<Vec<_>>().join("\n");
                        ui.output_mut(|o| o.copied_text = all);
                    }
                });
            });
            ui.add_space(4.0);

            egui::ScrollArea::vertical().show(ui, |ui| {
                if self.history.is_empty() {
                    ui.colored_label(egui::Color32::GRAY, lang.waiting);
                }
                for code in &self.history {
                    ui.monospace(code);
                }
            });
        });
    }
}
