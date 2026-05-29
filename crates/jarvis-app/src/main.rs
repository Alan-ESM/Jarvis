#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use eframe::egui::{
    self, Align, Color32, ComboBox, FontId, Id, Layout, Pos2, Rect, RichText, Sense, Stroke,
    TextureHandle, Vec2,
};
use image::imageops::FilterType;
use std::{
    env, fs,
    path::PathBuf,
    process::Command,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

const LOGO_BYTES: &[u8] = include_bytes!("../../../assets/Jarvis.png");
const INPUT_ID: &str = "jarvis-composer-input";
const MODELS: [&str; 5] = ["Flash", "X", "Ultra", "GPT-5", "OpenAI compatible"];
const ACCESS_LEVELS: [&str; 3] = ["Intermediaire", "Illimite", "Desactive"];
const MAX_UPLOAD_BYTES: u64 = 100 * 1024 * 1024;
const WINDOW_SIZE: [f32; 2] = [1440.0, 900.0];
const MIN_WINDOW_SIZE: [f32; 2] = [1280.0, 800.0];
const SIDEBAR_WIDTH: f32 = 320.0;
const TOPBAR_HEIGHT: f32 = 64.0;
const COMPOSER_HEIGHT: f32 = 92.0;
const COMPOSER_ATTACHMENTS_HEIGHT: f32 = 140.0;
const CONTENT_MAX_WIDTH: f32 = 1120.0;
const GLOBAL_PADDING: f32 = 24.0;

mod theme {
    use eframe::egui::Color32;

    pub const BG: Color32 = Color32::from_rgb(10, 11, 13);
    pub const PANEL: Color32 = Color32::from_rgb(17, 20, 24);
    pub const SURFACE: Color32 = Color32::from_rgb(22, 26, 31);
    pub const ACTIVE: Color32 = Color32::from_rgb(28, 33, 40);
    pub const KHAKI_DARK: Color32 = Color32::from_rgb(94, 102, 82);
    pub const KHAKI_LIGHT: Color32 = Color32::from_rgb(125, 138, 106);
    pub const GOLD: Color32 = Color32::from_rgb(184, 154, 90);
    pub const GOLD_HOVER: Color32 = Color32::from_rgb(210, 180, 111);
    pub const TEXT: Color32 = Color32::from_rgb(234, 229, 216);
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(168, 163, 154);
    pub const TEXT_FAINT: Color32 = Color32::from_rgb(111, 116, 109);
    pub const ONLINE: Color32 = Color32::from_rgb(142, 174, 112);
    pub const OFFLINE: Color32 = Color32::from_rgb(172, 104, 86);

    pub fn border() -> Color32 {
        Color32::from_rgba_unmultiplied(255, 255, 255, 20)
    }

    pub fn border_active() -> Color32 {
        Color32::from_rgba_unmultiplied(210, 180, 111, 130)
    }
}

fn main() -> Result<(), eframe::Error> {
    tracing_subscriber::fmt()
        .with_env_filter("jarvis=info,warn")
        .init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(WINDOW_SIZE)
            .with_min_inner_size(MIN_WINDOW_SIZE)
            .with_title("Jarvis")
            .with_icon(load_icon_data()),
        ..Default::default()
    };

    eframe::run_native(
        "Jarvis",
        options,
        Box::new(|cc| Ok(Box::new(JarvisUi::new(cc)))),
    )
}

#[derive(Debug, Clone)]
struct ChatMessage {
    role: MessageRole,
    content: String,
}

#[derive(Debug, Clone, Copy)]
enum MessageRole {
    User,
    Jarvis,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Panel {
    Chat,
    Search,
    Extensions,
    Automations,
    Memory,
    Files,
    Logs,
    Settings,
}

#[derive(Debug, Clone)]
struct UploadedFile {
    path: PathBuf,
    size: u64,
    status: FileStatus,
}

#[derive(Debug, Clone)]
enum FileStatus {
    Ready,
    Rejected(String),
}

#[derive(Debug, Clone)]
enum NetworkStatus {
    Checking,
    Online { mbps: f32, latency_ms: u128 },
    Offline,
}

#[derive(Debug, Clone)]
struct SearchReply {
    query: String,
    summary: String,
}

#[derive(Debug, Clone)]
struct VoiceReply {
    path: PathBuf,
    summary: String,
}

#[derive(Default)]
struct VoiceRecorder {
    active: Option<RecordingHandle>,
    last_recording: Option<PathBuf>,
    error: Option<String>,
}

struct RecordingHandle {
    stream: cpal::Stream,
    samples: Arc<Mutex<Vec<i16>>>,
    sample_rate: u32,
    channels: u16,
}

struct JarvisUi {
    logo: TextureHandle,
    messages: Vec<ChatMessage>,
    uploaded_files: Vec<UploadedFile>,
    input: String,
    active_panel: Panel,
    active_model: String,
    access_level: String,
    loading: bool,
    loading_started: Instant,
    show_portal: bool,
    portal_started: Instant,
    app_started: Instant,
    network: NetworkStatus,
    network_tx: Sender<NetworkStatus>,
    network_rx: Receiver<NetworkStatus>,
    network_probe_running: bool,
    last_network_probe: Instant,
    search_tx: Sender<SearchReply>,
    search_rx: Receiver<SearchReply>,
    search_running: bool,
    voice_tx: Sender<VoiceReply>,
    voice_rx: Receiver<VoiceReply>,
    voice_transcription_running: bool,
    voice: VoiceRecorder,
}

impl JarvisUi {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure_style(&cc.egui_ctx);
        let logo = load_logo(&cc.egui_ctx);
        let (network_tx, network_rx) = mpsc::channel();
        let (search_tx, search_rx) = mpsc::channel();
        let (voice_tx, voice_rx) = mpsc::channel();
        let mut app = Self {
            logo,
            messages: vec![ChatMessage {
                role: MessageRole::System,
                content: "Bienvenue. Ecris une demande: Jarvis peut rechercher sur Google, garder des fichiers joints, ouvrir un terminal et enregistrer le micro.".to_string(),
            }],
            uploaded_files: Vec::new(),
            input: String::new(),
            active_panel: Panel::Chat,
            active_model: "Flash".to_string(),
            access_level: "Intermediaire".to_string(),
            loading: false,
            loading_started: Instant::now(),
            show_portal: true,
            portal_started: Instant::now(),
            app_started: Instant::now(),
            network: NetworkStatus::Checking,
            network_tx,
            network_rx,
            network_probe_running: false,
            last_network_probe: Instant::now() - Duration::from_secs(30),
            search_tx,
            search_rx,
            search_running: false,
            voice_tx,
            voice_rx,
            voice_transcription_running: false,
            voice: VoiceRecorder::default(),
        };
        app.spawn_network_probe();
        app
    }

    fn send_current_message(&mut self) {
        let content = self.input.trim().to_string();
        if content.is_empty() {
            return;
        }

        self.messages.push(ChatMessage {
            role: MessageRole::User,
            content: content.clone(),
        });
        self.input.clear();
        self.loading = true;
        self.loading_started = Instant::now();
        self.spawn_google_search(content);
    }

    fn new_chat(&mut self) {
        self.messages.clear();
        self.messages.push(ChatMessage {
            role: MessageRole::System,
            content: "Nouvelle discussion Jarvis.".to_string(),
        });
        self.input.clear();
        self.uploaded_files.clear();
        self.active_panel = Panel::Chat;
    }

    fn open_upload_dialog(&mut self) {
        if let Some(paths) = rfd::FileDialog::new()
            .set_title("Ajouter des fichiers au prompt Jarvis")
            .pick_files()
        {
            self.add_uploaded_files(paths);
        }
    }

    fn add_uploaded_files(&mut self, paths: Vec<PathBuf>) {
        let mut added = 0usize;
        for path in paths {
            let size = fs::metadata(&path)
                .map(|metadata| metadata.len())
                .unwrap_or(0);
            if self.uploaded_files.iter().any(|file| file.path == path) {
                continue;
            }
            let status = if size > MAX_UPLOAD_BYTES {
                FileStatus::Rejected("trop volumineux pour ce prototype".to_string())
            } else {
                FileStatus::Ready
            };
            self.uploaded_files
                .push(UploadedFile { path, size, status });
            added += 1;
        }
        if added > 0 {
            self.messages.push(ChatMessage {
                role: MessageRole::System,
                content: format!("{added} fichier(s) ajoute(s) au prompt. Ils ne sont pas envoyes automatiquement."),
            });
        }
    }

    fn apply_shortcuts(&mut self, ctx: &egui::Context) {
        let mut send = false;
        let mut upload = false;
        let mut focus_input = false;
        let mut terminal = false;
        let mut mic = false;
        ctx.input(|input| {
            if input.key_pressed(egui::Key::Enter) && self.show_portal {
                self.show_portal = false;
            }
            if input.key_pressed(egui::Key::Escape) && self.show_portal {
                self.show_portal = false;
            }
            if input.modifiers.ctrl && input.key_pressed(egui::Key::Enter) {
                send = true;
            }
            if input.modifiers.ctrl && input.key_pressed(egui::Key::O) {
                upload = true;
            }
            if input.modifiers.ctrl && input.key_pressed(egui::Key::N) {
                self.new_chat();
            }
            if input.modifiers.ctrl && input.key_pressed(egui::Key::K) {
                self.active_panel = Panel::Search;
            }
            if input.modifiers.ctrl && input.key_pressed(egui::Key::L) {
                focus_input = true;
            }
            if input.modifiers.ctrl && input.key_pressed(egui::Key::T) {
                terminal = true;
            }
            if input.modifiers.ctrl && input.key_pressed(egui::Key::M) {
                mic = true;
            }
        });

        if send {
            self.send_current_message();
        }
        if upload {
            self.open_upload_dialog();
        }
        if focus_input {
            ctx.memory_mut(|memory| memory.request_focus(Id::new(INPUT_ID)));
        }
        if terminal {
            self.open_terminal();
        }
        if mic {
            self.toggle_microphone();
        }
    }

    fn poll_async_events(&mut self) {
        while let Ok(status) = self.network_rx.try_recv() {
            self.network = status;
            self.network_probe_running = false;
            self.last_network_probe = Instant::now();
        }
        while let Ok(reply) = self.search_rx.try_recv() {
            self.loading = false;
            self.search_running = false;
            self.active_panel = Panel::Chat;
            self.messages.push(ChatMessage {
                role: MessageRole::Jarvis,
                content: format!(
                    "🔎 Recherche Google pour: {}\n\n{}",
                    reply.query, reply.summary
                ),
            });
        }
        while let Ok(reply) = self.voice_rx.try_recv() {
            self.voice_transcription_running = false;
            self.messages.push(ChatMessage {
                role: MessageRole::Jarvis,
                content: format!(
                    "Transcription audio: {}\n\n{}",
                    reply.path.display(),
                    reply.summary
                ),
            });
        }
        if !self.network_probe_running
            && self.last_network_probe.elapsed() > Duration::from_secs(18)
        {
            self.spawn_network_probe();
        }
    }

    fn spawn_network_probe(&mut self) {
        self.network_probe_running = true;
        let tx = self.network_tx.clone();
        thread::spawn(move || {
            let _ = tx.send(measure_network());
        });
    }

    fn spawn_google_search(&mut self, query: String) {
        if self.search_running {
            return;
        }
        self.search_running = true;
        let tx = self.search_tx.clone();
        thread::spawn(move || {
            let summary = google_search_summary(&query);
            let _ = tx.send(SearchReply { query, summary });
        });
    }

    fn spawn_voice_transcription(&mut self, path: PathBuf) {
        if self.voice_transcription_running {
            self.messages.push(ChatMessage {
                role: MessageRole::System,
                content: "Une transcription vocale est deja en cours.".to_string(),
            });
            return;
        }
        self.voice_transcription_running = true;
        self.messages.push(ChatMessage {
            role: MessageRole::System,
            content: "Transcription vocale en cours, avec extraction type NotebookLM.".to_string(),
        });
        let tx = self.voice_tx.clone();
        thread::spawn(move || {
            let summary = transcribe_recording(&path);
            let _ = tx.send(VoiceReply { path, summary });
        });
    }

    fn open_terminal(&mut self) {
        let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let command = format!(
            "Set-Location -LiteralPath '{}'",
            cwd.display().to_string().replace('\'', "''")
        );
        let result = Command::new("cmd")
            .args([
                "/C",
                "start",
                "Jarvis Terminal",
                "powershell",
                "-NoExit",
                "-NoLogo",
                "-Command",
                &command,
            ])
            .spawn();
        if let Err(error) = result {
            self.messages.push(ChatMessage {
                role: MessageRole::System,
                content: format!("Impossible d'ouvrir le terminal: {error}"),
            });
        }
    }

    fn toggle_microphone(&mut self) {
        if self.voice.active.is_some() {
            match self.voice.stop() {
                Ok(path) => {
                    self.messages.push(ChatMessage {
                        role: MessageRole::System,
                        content: format!("Enregistrement sauvegarde: {}", path.display()),
                    });
                    self.spawn_voice_transcription(path);
                }
                Err(error) => self.messages.push(ChatMessage {
                    role: MessageRole::System,
                    content: format!("Erreur micro: {error}"),
                }),
            }
            return;
        }

        match self.voice.start() {
            Ok(()) => self.messages.push(ChatMessage {
                role: MessageRole::System,
                content: "Enregistrement micro en cours. Clique de nouveau pour arreter."
                    .to_string(),
            }),
            Err(error) => self.messages.push(ChatMessage {
                role: MessageRole::System,
                content: format!("Impossible d'activer le micro: {error}"),
            }),
        }
    }
}

impl eframe::App for JarvisUi {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(16));
        self.poll_async_events();
        self.apply_shortcuts(ctx);

        if self.show_portal {
            egui::CentralPanel::default().show(ctx, |ui| draw_portal(ui, self));
            return;
        }

        egui::SidePanel::left("sidebar")
            .resizable(false)
            .default_width(SIDEBAR_WIDTH)
            .show(ctx, |ui| draw_sidebar(ui, self));

        egui::TopBottomPanel::top("topbar")
            .resizable(false)
            .exact_height(TOPBAR_HEIGHT)
            .show(ctx, |ui| draw_topbar(ui, self));

        egui::TopBottomPanel::bottom("composer")
            .resizable(false)
            .exact_height(if self.uploaded_files.is_empty() {
                COMPOSER_HEIGHT
            } else {
                COMPOSER_ATTACHMENTS_HEIGHT
            })
            .show(ctx, |ui| draw_composer_spec(ui, self));

        egui::CentralPanel::default().show(ctx, |ui| draw_workspace(ui, self));
    }
}

impl VoiceRecorder {
    fn is_recording(&self) -> bool {
        self.active.is_some()
    }

    fn start(&mut self) -> anyhow::Result<()> {
        if self.active.is_some() {
            return Ok(());
        }

        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| anyhow::anyhow!("aucun micro par defaut trouve"))?;
        let supported = device.default_input_config()?;
        let sample_rate = supported.sample_rate().0;
        let channels = supported.channels();
        let config = supported.config();
        let samples = Arc::new(Mutex::new(Vec::<i16>::new()));
        let writer_samples = Arc::clone(&samples);
        let err_fn = |error| eprintln!("Jarvis microphone stream error: {error}");

        let stream = match supported.sample_format() {
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config,
                move |data: &[i16], _| push_i16_samples(&writer_samples, data.iter().copied()),
                err_fn,
                None,
            )?,
            cpal::SampleFormat::U16 => device.build_input_stream(
                &config,
                move |data: &[u16], _| {
                    push_i16_samples(
                        &writer_samples,
                        data.iter().map(|sample| (*sample as i32 - 32768) as i16),
                    )
                },
                err_fn,
                None,
            )?,
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config,
                move |data: &[f32], _| {
                    push_i16_samples(
                        &writer_samples,
                        data.iter()
                            .map(|sample| (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16),
                    )
                },
                err_fn,
                None,
            )?,
            sample_format => anyhow::bail!("format micro non supporte: {sample_format:?}"),
        };

        stream.play()?;
        self.error = None;
        self.active = Some(RecordingHandle {
            stream,
            samples,
            sample_rate,
            channels,
        });
        Ok(())
    }

    fn stop(&mut self) -> anyhow::Result<PathBuf> {
        let handle = self
            .active
            .take()
            .ok_or_else(|| anyhow::anyhow!("aucun enregistrement actif"))?;
        drop(handle.stream);

        let dir = env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Jarvis")
            .join("recordings");
        fs::create_dir_all(&dir)?;
        let file_name = format!("jarvis-mic-{}.wav", chrono_like_timestamp());
        let path = dir.join(file_name);
        let spec = hound::WavSpec {
            channels: handle.channels,
            sample_rate: handle.sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let samples = handle
            .samples
            .lock()
            .map_err(|_| anyhow::anyhow!("micro buffer verrouille"))?;
        let mut writer = hound::WavWriter::create(&path, spec)?;
        for sample in samples.iter() {
            writer.write_sample(*sample)?;
        }
        writer.finalize()?;
        self.last_recording = Some(path.clone());
        Ok(path)
    }
}

fn push_i16_samples<I>(samples: &Arc<Mutex<Vec<i16>>>, data: I)
where
    I: IntoIterator<Item = i16>,
{
    if let Ok(mut buffer) = samples.lock() {
        buffer.extend(data);
    }
}

fn load_logo(ctx: &egui::Context) -> TextureHandle {
    let rgba = circular_logo_rgba(256);
    let color_image = egui::ColorImage::from_rgba_unmultiplied([256, 256], rgba.as_slice());
    ctx.load_texture("jarvis-logo", color_image, egui::TextureOptions::LINEAR)
}

fn load_icon_data() -> egui::IconData {
    let rgba = circular_logo_rgba(256);
    egui::IconData {
        rgba,
        width: 256,
        height: 256,
    }
}

fn circular_logo_rgba(size: u32) -> Vec<u8> {
    let image = image::load_from_memory(LOGO_BYTES)
        .expect("Jarvis logo asset must be valid")
        .resize_to_fill(size, size, FilterType::Lanczos3)
        .to_rgba8();
    let mut rgba = image.into_raw();
    let center = (size as f32 - 1.0) * 0.5;
    let radius = center - 1.0;
    let feather = 2.5_f32;
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let distance = (dx * dx + dy * dy).sqrt();
            let alpha_index = ((y * size + x) * 4 + 3) as usize;
            if distance > radius {
                rgba[alpha_index] = 0;
            } else if distance > radius - feather {
                let fade = ((radius - distance) / feather).clamp(0.0, 1.0);
                rgba[alpha_index] = ((rgba[alpha_index] as f32) * fade) as u8;
            }
        }
    }
    rgba
}

fn configure_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals.dark_mode = true;
    style.visuals.window_fill = theme::BG;
    style.visuals.panel_fill = theme::BG;
    style.visuals.extreme_bg_color = theme::BG;
    style.visuals.faint_bg_color = theme::SURFACE;
    style.visuals.widgets.inactive.bg_fill = theme::SURFACE;
    style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(33, 38, 44);
    style.visuals.widgets.active.bg_fill = theme::ACTIVE;
    style.visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, theme::TEXT);
    style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, theme::TEXT);
    style.visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, theme::GOLD_HOVER);
    style.visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(184, 154, 90, 80);
    style.visuals.selection.stroke = Stroke::new(1.0, theme::GOLD_HOVER);
    style.visuals.override_text_color = Some(theme::TEXT);
    style.spacing.item_spacing = Vec2::new(10.0, 10.0);
    style.spacing.button_padding = Vec2::new(16.0, 10.0);
    style.spacing.menu_margin = egui::Margin::symmetric(12.0, 10.0);
    style
        .text_styles
        .insert(egui::TextStyle::Heading, FontId::proportional(20.0));
    style
        .text_styles
        .insert(egui::TextStyle::Body, FontId::proportional(16.0));
    style
        .text_styles
        .insert(egui::TextStyle::Button, FontId::proportional(16.0));
    style
        .text_styles
        .insert(egui::TextStyle::Small, FontId::proportional(12.0));
    ctx.set_style(style);
}

fn draw_portal(ui: &mut egui::Ui, app: &mut JarvisUi) {
    let rect = ui.max_rect();
    let painter = ui.painter_at(rect);
    let elapsed = app.portal_started.elapsed().as_secs_f32();
    painter.rect_filled(rect, 0.0, theme::BG);
    draw_dark_aurora(&painter, rect, elapsed);
    draw_geometric_field(&painter, rect, elapsed);

    let center = rect.center() + Vec2::new(0.0, -32.0);
    let progress = (elapsed / 3.0).clamp(0.0, 1.0);
    let rotation = (1.0 - progress) * std::f32::consts::TAU * 0.85;
    draw_infinity_ribbon(&painter, center, progress, rotation);

    let gold = (elapsed - 1.6).clamp(0.0, 1.0);
    let radius = 16.0 + gold * 58.0 + (elapsed * 3.0).sin().abs() * 4.0;
    painter.circle_filled(
        center,
        radius,
        Color32::from_rgba_unmultiplied(210, 180, 111, (46.0 + 82.0 * gold) as u8),
    );
    painter.circle_filled(center, 10.0 + gold * 8.0, theme::GOLD_HOVER);

    painter.text(
        Pos2::new(center.x, rect.bottom() - 142.0),
        egui::Align2::CENTER_CENTER,
        "JARVIS",
        FontId::proportional(40.0),
        theme::TEXT,
    );

    let button_rect = Rect::from_center_size(
        Pos2::new(center.x, rect.bottom() - 84.0),
        Vec2::new(248.0, 48.0),
    );
    let enter = ui
        .put(
            button_rect,
            egui::Button::new(
                RichText::new("✨ Entrer dans Jarvis")
                    .strong()
                    .color(theme::BG),
            )
            .fill(theme::GOLD)
            .stroke(Stroke::new(1.0, theme::GOLD_HOVER))
            .rounding(egui::Rounding::same(24.0)),
        )
        .on_hover_text("Ouvrir l'interface Jarvis");
    if enter.clicked() {
        app.show_portal = false;
    }
}

fn draw_sidebar(ui: &mut egui::Ui, app: &mut JarvisUi) {
    draw_aurora_background(ui, app.app_started.elapsed().as_secs_f32(), theme::PANEL);
    ui.add_space(20.0);
    ui.horizontal(|ui| {
        draw_logo_mark(ui, &app.logo, 62.0, app.app_started.elapsed().as_secs_f32());
        ui.vertical(|ui| {
            ui.label(RichText::new("Jarvis").size(28.0).strong());
            ui.label(
                RichText::new("Agent desktop Windows")
                    .size(14.0)
                    .color(mut_text()),
            );
        });
    });
    ui.add_space(24.0);

    sidebar_action(ui, "📝 Nouveau clavardage", "Ctrl+N", || app.new_chat());
    if nav_button(
        ui,
        app.active_panel == Panel::Search,
        "🔎 Recherche",
        "Ctrl+K",
    )
    .clicked()
    {
        app.active_panel = Panel::Search;
    }
    if nav_button(ui, app.active_panel == Panel::Extensions, "🧩 Modules", "").clicked() {
        app.active_panel = Panel::Extensions;
    }
    if nav_button(
        ui,
        app.active_panel == Panel::Automations,
        "⏱ Automatisations",
        "",
    )
    .clicked()
    {
        app.active_panel = Panel::Automations;
    }
    if nav_button(
        ui,
        app.active_panel == Panel::Files,
        "📎 Fichiers joints",
        "Ctrl+O",
    )
    .clicked()
    {
        app.active_panel = Panel::Files;
    }
    if nav_button(ui, app.active_panel == Panel::Logs, "📜 Logs", "").clicked() {
        app.active_panel = Panel::Logs;
    }

    ui.add_space(24.0);
    ui.label(
        RichText::new("Discussions IA")
            .size(14.0)
            .strong()
            .color(theme::TEXT_FAINT),
    );
    discussion_button(ui, app, "Jarvis UI futuriste", "En cours", true);
    discussion_button(ui, app, "Recherche Google locale", "A brancher", false);
    discussion_button(ui, app, "Micro et fichiers", "Prototype", false);
    discussion_button(ui, app, "Securite sandbox", "Masque UI", false);

    ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
        ui.add_space(12.0);
        if nav_button(ui, app.active_panel == Panel::Settings, "⚙ Parametres", "").clicked() {
            app.active_panel = Panel::Settings;
        }
        ui.add_space(8.0);
        draw_network_badge(ui, &app.network);
    });
}

fn draw_logo_mark(ui: &mut egui::Ui, texture: &TextureHandle, size: f32, time: f32) {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let painter = ui.painter_at(rect.expand(7.0));
    let pulse = 0.45 + 0.55 * (time * 2.2).sin().abs();
    painter.circle_filled(
        rect.center(),
        size * 0.56,
        Color32::from_rgba_unmultiplied(17, 20, 24, 235),
    );
    painter.circle_stroke(
        rect.center(),
        size * 0.55 + pulse * 2.0,
        Stroke::new(1.5, Color32::from_rgba_unmultiplied(210, 180, 111, 150)),
    );
    painter.circle_stroke(
        rect.center(),
        size * 0.42,
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(125, 138, 106, 130)),
    );
    painter.image(
        texture.id(),
        rect.shrink(3.0),
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
        Color32::WHITE,
    );
    if response.hovered() {
        painter.circle_stroke(
            rect.center(),
            size * 0.62,
            Stroke::new(2.0, Color32::from_rgba_unmultiplied(210, 180, 111, 190)),
        );
    }
}

fn draw_topbar(ui: &mut egui::Ui, app: &mut JarvisUi) {
    draw_aurora_background(
        ui,
        app.app_started.elapsed().as_secs_f32() + 2.0,
        theme::PANEL,
    );
    ui.horizontal_centered(|ui| {
        ui.add_space(20.0);
        ui.label(
            RichText::new(panel_title(app.active_panel))
                .size(21.0)
                .strong(),
        );
        ui.label(RichText::new("• interface locale native").color(mut_text()));

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui
                .add_sized(
                    [44.0, 38.0],
                    egui::Button::new("🖥").rounding(egui::Rounding::same(18.0)),
                )
                .on_hover_text("Ouvrir le terminal - Ctrl+T")
                .clicked()
            {
                app.open_terminal();
            }
            draw_model_dropdown(ui, app);
            draw_access_dropdown(ui, app);
            draw_network_badge(ui, &app.network);
        });
    });
}

fn draw_workspace(ui: &mut egui::Ui, app: &mut JarvisUi) {
    draw_aurora_background(ui, app.app_started.elapsed().as_secs_f32(), theme::BG);
    match app.active_panel {
        Panel::Chat => draw_chat(ui, app),
        Panel::Search => draw_simple_panel(
            ui,
            "🔎 Recherche",
            "Chaque message envoye lance une recherche Google si GOOGLE_SEARCH_API_KEY et GOOGLE_SEARCH_ENGINE_ID sont configures.",
        ),
        Panel::Extensions => draw_simple_panel(
            ui,
            "🧩 Modules",
            "Les plugins seront exposes ici par capacite, niveau de permission et journalisation.",
        ),
        Panel::Automations => draw_simple_panel(
            ui,
            "⏱ Automatisations",
            "Les routines locales, rappels et workflows durables seront pilotes depuis ce panneau.",
        ),
        Panel::Memory => draw_simple_panel(
            ui,
            "🧠 Memoire",
            "La memoire persistante restera locale, auditable et separable par discussion.",
        ),
        Panel::Files => draw_files_panel(ui, app),
        Panel::Logs => draw_simple_panel(
            ui,
            "📜 Logs",
            "Les logs Windows et applicatifs seront analyses ici avec resume, anomalies et actions recommandees.",
        ),
        Panel::Settings => draw_settings_panel(ui, app),
    }
}

fn draw_chat(ui: &mut egui::Ui, app: &mut JarvisUi) {
    ui.add_space(GLOBAL_PADDING);
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .show(ui, |ui| {
            let max_width = ui.available_width().min(CONTENT_MAX_WIDTH);
            ui.with_layout(Layout::top_down(Align::Center), |ui| {
                ui.set_width(max_width);
                for message in &app.messages {
                    draw_message_aligned(ui, message);
                    ui.add_space(12.0);
                }
                if app.loading {
                    draw_loading(ui, app.loading_started.elapsed().as_secs_f32());
                }
            });
        });
}

fn draw_message_aligned(ui: &mut egui::Ui, message: &ChatMessage) {
    let (label, accent, fill, width_ratio, align_right) = match message.role {
        MessageRole::User => (
            "Vous",
            theme::GOLD_HOVER,
            Color32::from_rgb(33, 34, 31),
            0.68,
            true,
        ),
        MessageRole::Jarvis => ("Jarvis", theme::GOLD, theme::PANEL, 0.72, false),
        MessageRole::System => ("Systeme", theme::TEXT_MUTED, theme::SURFACE, 0.72, false),
    };

    let row_width = ui.available_width();
    let bubble_width = (row_width * width_ratio).min(CONTENT_MAX_WIDTH * width_ratio);
    let side_gap = (row_width - bubble_width - 24.0).max(0.0);

    ui.horizontal(|ui| {
        if align_right {
            ui.add_space(side_gap);
        }
        ui.vertical(|ui| {
            ui.set_max_width(bubble_width);
            egui::Frame::none()
                .fill(fill)
                .stroke(Stroke::new(
                    1.0,
                    if align_right {
                        theme::border_active()
                    } else {
                        theme::border()
                    },
                ))
                .rounding(egui::Rounding::same(14.0))
                .inner_margin(egui::Margin::symmetric(16.0, 14.0))
                .show(ui, |ui| {
                    ui.set_max_width(bubble_width);
                    ui.label(RichText::new(label).size(13.0).strong().color(accent));
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(&message.content)
                            .size(16.0)
                            .color(theme::TEXT),
                    );
                });
        });
        if !align_right {
            ui.add_space(side_gap);
        }
    });
}

#[allow(dead_code)]
fn draw_message(ui: &mut egui::Ui, message: &ChatMessage) {
    let (label, accent, fill) = match message.role {
        MessageRole::User => (
            "👤 Vous",
            Color32::from_rgb(215, 234, 244),
            Color32::from_rgb(31, 35, 45),
        ),
        MessageRole::Jarvis => (
            "✨ Jarvis",
            Color32::from_rgb(92, 222, 255),
            Color32::from_rgb(18, 31, 42),
        ),
        MessageRole::System => (
            "🛡 Systeme",
            Color32::from_rgb(162, 178, 192),
            Color32::from_rgb(24, 27, 35),
        ),
    };

    egui::Frame::none()
        .fill(fill)
        .stroke(Stroke::new(1.0, Color32::from_rgb(46, 58, 72)))
        .rounding(egui::Rounding::same(14.0))
        .inner_margin(egui::Margin::symmetric(16.0, 12.0))
        .show(ui, |ui| {
            ui.label(RichText::new(label).strong().color(accent));
            ui.add_space(5.0);
            ui.label(RichText::new(&message.content).size(15.0));
        });
}

fn draw_files_panel(ui: &mut egui::Ui, app: &mut JarvisUi) {
    ui.set_max_width(ui.available_width().min(CONTENT_MAX_WIDTH));
    ui.add_space(GLOBAL_PADDING);
    ui.horizontal(|ui| {
        ui.heading(RichText::new("📎 Fichiers joints").size(20.0));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui
                .add_sized(
                    [150.0, 38.0],
                    egui::Button::new("📎 Uploader").rounding(egui::Rounding::same(14.0)),
                )
                .on_hover_text("Ajouter des fichiers au prompt")
                .clicked()
            {
                app.open_upload_dialog();
            }
        });
    });
    ui.add_space(10.0);
    ui.label(
        RichText::new(
            "Les fichiers restent attaches au prompt. Rien n'est envoye automatiquement.",
        )
        .color(mut_text()),
    );
    ui.add_space(18.0);

    if app.uploaded_files.is_empty() {
        draw_empty_state(ui, "Aucun fichier joint.");
        return;
    }

    for file in &app.uploaded_files {
        file_chip(ui, file, true);
        ui.add_space(8.0);
    }
}

fn draw_settings_panel(ui: &mut egui::Ui, app: &mut JarvisUi) {
    ui.set_max_width(ui.available_width().min(CONTENT_MAX_WIDTH));
    ui.add_space(GLOBAL_PADDING);
    ui.heading(RichText::new("⚙ Parametres").size(20.0));
    ui.add_space(14.0);
    setting_row(ui, "Modele actif", app.active_model.as_str());
    setting_row(ui, "Acces fichiers", app.access_level.as_str());
    setting_row(
        ui,
        "Micro",
        if app.voice.is_recording() {
            "Enregistrement"
        } else {
            "Pret"
        },
    );
    setting_row(ui, "Internet", network_text(&app.network).as_str());
    if let Some(path) = &app.voice.last_recording {
        setting_row(ui, "Dernier audio", path.display().to_string().as_str());
    }
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        if ui.button("🧠 Memoire").clicked() {
            app.active_panel = Panel::Memory;
        }
        if ui.button("📜 Logs").clicked() {
            app.active_panel = Panel::Logs;
        }
        if ui.button("🖥 Terminal").clicked() {
            app.open_terminal();
        }
    });
}

fn draw_simple_panel(ui: &mut egui::Ui, title: &str, body: &str) {
    ui.set_max_width(ui.available_width().min(CONTENT_MAX_WIDTH));
    ui.add_space(GLOBAL_PADDING);
    ui.heading(RichText::new(title).size(20.0));
    ui.add_space(12.0);
    ui.label(RichText::new(body).color(mut_text()));
}

fn draw_empty_state(ui: &mut egui::Ui, text: &str) {
    egui::Frame::none()
        .fill(theme::SURFACE)
        .stroke(Stroke::new(1.0, theme::border()))
        .rounding(egui::Rounding::same(14.0))
        .inner_margin(egui::Margin::symmetric(18.0, 18.0))
        .show(ui, |ui| {
            ui.label(RichText::new(text).color(mut_text()));
        });
}

fn draw_composer_spec(ui: &mut egui::Ui, app: &mut JarvisUi) {
    draw_aurora_background(ui, app.app_started.elapsed().as_secs_f32() + 5.0, theme::BG);
    let max_width = ui.available_width().min(CONTENT_MAX_WIDTH);
    ui.add_space(if app.uploaded_files.is_empty() {
        14.0
    } else {
        10.0
    });
    ui.with_layout(Layout::top_down(Align::Center), |ui| {
        ui.set_width(max_width);
        if !app.uploaded_files.is_empty() {
            egui::ScrollArea::horizontal()
                .max_height(44.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for file in &app.uploaded_files {
                            file_chip(ui, file, false);
                        }
                    });
                });
            ui.add_space(8.0);
        }

        egui::Frame::none()
            .fill(Color32::from_rgba_unmultiplied(22, 26, 31, 248))
            .stroke(Stroke::new(1.0, theme::border()))
            .rounding(egui::Rounding::same(16.0))
            .inner_margin(egui::Margin::symmetric(16.0, 8.0))
            .show(ui, |ui| {
                ui.set_height(56.0);
                ui.horizontal_centered(|ui| {
                    if ui
                        .add_sized(
                            [40.0, 40.0],
                            egui::Button::new(RichText::new("+").size(20.0).strong())
                                .rounding(egui::Rounding::same(20.0)),
                        )
                        .on_hover_text("Uploader un fichier - Ctrl+O")
                        .clicked()
                    {
                        app.open_upload_dialog();
                    }

                    let edit = egui::TextEdit::multiline(&mut app.input)
                        .id(Id::new(INPUT_ID))
                        .hint_text("Demande quelque chose a Jarvis...")
                        .desired_rows(1)
                        .desired_width((ui.available_width() - 108.0).max(240.0));
                    ui.add(edit);

                    let mic_label = if app.voice.is_recording() {
                        "REC"
                    } else {
                        "MIC"
                    };
                    if ui
                        .add_sized(
                            [40.0, 40.0],
                            egui::Button::new(RichText::new(mic_label).size(12.0).strong())
                                .rounding(egui::Rounding::same(20.0)),
                        )
                        .on_hover_text("Enregistrer le micro - Ctrl+M")
                        .clicked()
                    {
                        app.toggle_microphone();
                    }
                    if ui
                        .add_sized(
                            [40.0, 40.0],
                            egui::Button::new(RichText::new("->").size(16.0).strong())
                                .fill(theme::GOLD)
                                .stroke(Stroke::new(1.0, theme::GOLD_HOVER))
                                .rounding(egui::Rounding::same(20.0)),
                        )
                        .on_hover_text("Envoyer - Ctrl+Enter")
                        .clicked()
                    {
                        app.send_current_message();
                    }
                });
            });
    });
}

#[allow(dead_code)]
fn draw_composer(ui: &mut egui::Ui, app: &mut JarvisUi) {
    draw_aurora_background(
        ui,
        app.app_started.elapsed().as_secs_f32() + 5.0,
        Color32::from_rgb(8, 10, 15),
    );
    ui.add_space(10.0);
    let max_width = ui.available_width().min(920.0);
    ui.with_layout(Layout::top_down(Align::Center), |ui| {
        ui.set_width(max_width);
        egui::Frame::none()
            .fill(Color32::from_rgba_unmultiplied(36, 41, 52, 238))
            .stroke(Stroke::new(1.0, Color32::from_rgb(68, 82, 98)))
            .rounding(egui::Rounding::same(20.0))
            .inner_margin(egui::Margin::symmetric(14.0, 9.0))
            .show(ui, |ui| {
            if !app.uploaded_files.is_empty() {
                egui::ScrollArea::horizontal().max_height(42.0).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for file in &app.uploaded_files {
                            file_chip(ui, file, false);
                        }
                    });
                });
                ui.add_space(6.0);
            }
            let edit = egui::TextEdit::multiline(&mut app.input)
                .id(Id::new(INPUT_ID))
                .hint_text("Demande quelque chose a Jarvis... il cherchera sur Google si la recherche est configuree.")
                .desired_rows(2)
                .desired_width(ui.available_width());
            ui.add(edit);
            ui.add_space(9.0);
            ui.horizontal(|ui| {
                if ui
                    .add_sized([42.0, 38.0], egui::Button::new("📎").rounding(egui::Rounding::same(18.0)))
                    .on_hover_text("Uploader un fichier - Ctrl+O")
                    .clicked()
                {
                    app.open_upload_dialog();
                }
                draw_access_dropdown(ui, app);
                ui.label(RichText::new(format!("{} fichier(s)", app.uploaded_files.len())).color(mut_text()));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add_sized([52.0, 42.0], egui::Button::new("➤").rounding(egui::Rounding::same(21.0)))
                        .on_hover_text("Envoyer - Ctrl+Enter")
                        .clicked()
                    {
                        app.send_current_message();
                    }
                    let mic_text = if app.voice.is_recording() { "●" } else { "🎙" };
                    if ui
                        .add_sized([46.0, 42.0], egui::Button::new(mic_text).rounding(egui::Rounding::same(21.0)))
                        .on_hover_text("Enregistrer le micro - Ctrl+M")
                        .clicked()
                    {
                        app.toggle_microphone();
                    }
                    draw_model_dropdown(ui, app);
                });
            });
        });
    });
}

fn draw_loading(ui: &mut egui::Ui, elapsed: f32) {
    let rect = ui
        .allocate_exact_size(Vec2::new(ui.available_width(), 46.0), Sense::hover())
        .0;
    let painter = ui.painter_at(rect);
    let width = rect.width();
    let x = rect.left() + ((elapsed * 220.0) % width.max(1.0));
    painter.line_segment(
        [
            Pos2::new(rect.left(), rect.center().y),
            Pos2::new(rect.right(), rect.center().y),
        ],
        Stroke::new(1.0, Color32::from_rgb(42, 62, 76)),
    );
    painter.circle_filled(
        Pos2::new(x, rect.center().y),
        6.0,
        Color32::from_rgb(92, 232, 255),
    );
    painter.text(
        rect.left_top() + Vec2::new(0.0, 2.0),
        egui::Align2::LEFT_TOP,
        "Recherche et synthese en cours",
        FontId::proportional(14.0),
        Color32::from_rgb(132, 235, 255),
    );
}

fn sidebar_action(ui: &mut egui::Ui, label: &str, shortcut: &str, mut action: impl FnMut()) {
    let display = if shortcut.is_empty() {
        label.to_string()
    } else {
        format!("{label}   {shortcut}")
    };
    if nav_button(ui, false, display.as_str(), shortcut).clicked() {
        action();
    }
}

fn nav_button(ui: &mut egui::Ui, selected: bool, label: &str, hover: &str) -> egui::Response {
    let fill = if selected {
        theme::ACTIVE
    } else {
        theme::SURFACE
    };
    let text = if selected {
        RichText::new(label).size(16.0).strong().color(theme::TEXT)
    } else {
        RichText::new(label).size(16.0).color(theme::TEXT_MUTED)
    };
    let response = ui
        .add_sized(
            [ui.available_width(), 52.0],
            egui::Button::new(text)
                .fill(fill)
                .stroke(Stroke::new(
                    1.0,
                    if selected {
                        theme::border_active()
                    } else {
                        theme::border()
                    },
                ))
                .rounding(egui::Rounding::same(16.0)),
        )
        .on_hover_text(if hover.is_empty() { label } else { hover });
    if response.hovered() {
        ui.painter().rect_stroke(
            response.rect.expand(1.0),
            egui::Rounding::same(16.0),
            Stroke::new(1.0, theme::GOLD_HOVER),
        );
    }
    response
}

fn discussion_button(
    ui: &mut egui::Ui,
    app: &mut JarvisUi,
    name: &str,
    status: &str,
    selected: bool,
) {
    let label = format!("💬 {name}\n   {status}");
    if nav_button(ui, selected, &label, "Ouvrir la discussion").clicked() {
        app.active_panel = Panel::Chat;
    }
}

fn draw_model_dropdown(ui: &mut egui::Ui, app: &mut JarvisUi) {
    ComboBox::from_id_salt("model-dropdown")
        .selected_text(format!("🤖 {}", app.active_model))
        .width(152.0)
        .show_ui(ui, |ui| {
            for model in MODELS {
                ui.selectable_value(&mut app.active_model, model.to_string(), model);
            }
        });
}

fn draw_access_dropdown(ui: &mut egui::Ui, app: &mut JarvisUi) {
    ComboBox::from_id_salt("access-dropdown")
        .selected_text(format!("🛡 {}", app.access_level))
        .width(144.0)
        .show_ui(ui, |ui| {
            for access in ACCESS_LEVELS {
                ui.selectable_value(&mut app.access_level, access.to_string(), access);
            }
        });
}

fn draw_network_badge(ui: &mut egui::Ui, status: &NetworkStatus) {
    let (text, color) = match status {
        NetworkStatus::Checking => ("NET ...".to_string(), theme::TEXT_MUTED),
        NetworkStatus::Online { mbps, latency_ms } => (
            format!("NET {:.1} Mbps / {} ms", mbps, latency_ms),
            theme::ONLINE,
        ),
        NetworkStatus::Offline => ("NET OFF".to_string(), theme::OFFLINE),
    };
    ui.label(RichText::new(text).size(13.0).strong().color(color));
}

fn setting_row(ui: &mut egui::Ui, name: &str, value: &str) {
    egui::Frame::none()
        .fill(theme::SURFACE)
        .stroke(Stroke::new(1.0, theme::border()))
        .rounding(egui::Rounding::same(12.0))
        .inner_margin(egui::Margin::symmetric(14.0, 10.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(name).color(mut_text()));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(value).strong());
                });
            });
        });
    ui.add_space(8.0);
}

fn file_chip(ui: &mut egui::Ui, file: &UploadedFile, full: bool) {
    let name = file
        .path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("fichier");
    let (status, color) = match &file.status {
        FileStatus::Ready => ("pret", theme::ONLINE),
        FileStatus::Rejected(reason) => (reason.as_str(), theme::OFFLINE),
    };
    egui::Frame::none()
        .fill(theme::SURFACE)
        .stroke(Stroke::new(1.0, theme::border()))
        .rounding(egui::Rounding::same(12.0))
        .inner_margin(egui::Margin::symmetric(14.0, 8.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("📄").size(18.0));
                ui.vertical(|ui| {
                    ui.label(RichText::new(name).strong());
                    let detail = if full {
                        format!(
                            "{} - {} - {}",
                            format_size(file.size),
                            status,
                            file.path.display()
                        )
                    } else {
                        format!("{} - {}", format_size(file.size), status)
                    };
                    ui.label(RichText::new(detail).size(12.0).color(color));
                });
            });
        });
}

fn draw_aurora_background(ui: &mut egui::Ui, time: f32, base: Color32) {
    let rect = ui.max_rect();
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, base);
    draw_dark_aurora(&painter, rect, time);
    draw_geometric_field(&painter, rect, time);
}

fn draw_dark_aurora(painter: &egui::Painter, rect: Rect, time: f32) {
    let colors = [
        theme::KHAKI_LIGHT.gamma_multiply(0.35),
        Color32::from_rgba_unmultiplied(184, 154, 90, 34),
        theme::KHAKI_DARK.gamma_multiply(0.34),
        Color32::from_rgba_unmultiplied(210, 180, 111, 20),
    ];
    for (band, color) in colors.iter().enumerate() {
        for layer in 0..3 {
            let mut points = Vec::new();
            let y_base = rect.top() + rect.height() * (0.18 + band as f32 * 0.13);
            for index in 0..130 {
                let t = index as f32 / 129.0;
                let x = rect.left() + t * rect.width();
                let wave = ((t * 7.5 + time * (0.22 + band as f32 * 0.04)).sin() * 38.0)
                    + ((t * 19.0 - time * 0.16 + layer as f32).cos() * 15.0);
                points.push(Pos2::new(x, y_base + wave + layer as f32 * 18.0));
            }
            painter.add(egui::Shape::line(
                points,
                Stroke::new(28.0 - layer as f32 * 7.0, *color),
            ));
        }
    }

    for index in 0..34 {
        let t = index as f32 / 33.0;
        let x = rect.left() + rect.width() * t;
        let drift = (time * 0.45 + t * 12.0).sin();
        let top = rect.top() + rect.height() * (0.08 + 0.08 * drift.abs());
        let bottom = rect.top() + rect.height() * (0.56 + 0.10 * (time * 0.18 + t).cos());
        let alpha = (16.0 + 28.0 * drift.abs()) as u8;
        painter.line_segment(
            [Pos2::new(x, top), Pos2::new(x + drift * 18.0, bottom)],
            Stroke::new(
                1.2 + drift.abs() * 1.8,
                Color32::from_rgba_unmultiplied(210, 180, 111, alpha),
            ),
        );
    }
}

fn draw_geometric_field(painter: &egui::Painter, rect: Rect, time: f32) {
    let cols = 8;
    let rows = 5;
    for row in 0..rows {
        for col in 0..cols {
            let t = time * 0.06 + row as f32 * 0.37 + col as f32 * 0.19;
            let x = rect.left() + rect.width() * ((col as f32 + 0.5) / cols as f32);
            let y = rect.top() + rect.height() * ((row as f32 + 0.5) / rows as f32);
            let drift = Vec2::new(t.sin() * 10.0, t.cos() * 6.0);
            let center = Pos2::new(x, y) + drift;
            let radius = 18.0 + ((row + col) % 3) as f32 * 7.0;
            let alpha = 18 + (((t.sin() + 1.0) * 12.0) as u8);
            draw_hexagon(
                painter,
                center,
                radius,
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(210, 180, 111, alpha)),
            );
        }
    }

    for index in 0..12 {
        let t = index as f32 / 11.0;
        let y = rect.top() + rect.height() * (0.18 + t * 0.64);
        let x_offset = (time * 0.08 + t * 8.0).sin() * 36.0;
        painter.line_segment(
            [
                Pos2::new(rect.left() + rect.width() * 0.12 + x_offset, y),
                Pos2::new(
                    rect.right() - rect.width() * 0.14 + x_offset * 0.4,
                    y + 22.0,
                ),
            ],
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(125, 138, 106, 22)),
        );
    }
}

fn draw_hexagon(painter: &egui::Painter, center: Pos2, radius: f32, stroke: Stroke) {
    let mut points = Vec::with_capacity(7);
    for index in 0..=6 {
        let angle = std::f32::consts::TAU * index as f32 / 6.0;
        points.push(center + Vec2::new(angle.cos() * radius, angle.sin() * radius));
    }
    painter.add(egui::Shape::line(points, stroke));
}

#[allow(dead_code)]
fn draw_light_aurora(painter: &egui::Painter, rect: Rect, time: f32) {
    let colors = [
        Color32::from_rgba_unmultiplied(70, 168, 220, 42),
        Color32::from_rgba_unmultiplied(98, 222, 207, 34),
        Color32::from_rgba_unmultiplied(120, 118, 238, 26),
    ];
    for (band, color) in colors.iter().enumerate() {
        for layer in 0..3 {
            let mut points = Vec::new();
            let y_base = rect.top() + rect.height() * (0.22 + band as f32 * 0.14);
            for index in 0..110 {
                let t = index as f32 / 109.0;
                let x = rect.left() + t * rect.width();
                let wave = (t * 7.0 + time * 0.32 + band as f32).sin() * 34.0
                    + (t * 16.0 - time * 0.12).cos() * 9.0;
                points.push(Pos2::new(x, y_base + wave + layer as f32 * 15.0));
            }
            painter.add(egui::Shape::line(
                points,
                Stroke::new(30.0 - layer as f32 * 8.0, *color),
            ));
        }
    }
}

fn draw_infinity_ribbon(painter: &egui::Painter, center: Pos2, progress: f32, rotation: f32) {
    let total = 260;
    let draw_count = (total as f32 * progress.max(0.06)) as usize;
    let mut points = Vec::new();
    for index in 0..draw_count {
        let t = -std::f32::consts::PI + (index as f32 / (total - 1) as f32) * std::f32::consts::TAU;
        let depth = 0.9 + 0.1 * (t.cos() * 0.5 + 0.5);
        let x = 235.0 * t.sin() * depth;
        let y = 105.0 * (2.0 * t).sin() * depth;
        let rotated_x = x * rotation.cos() - y * rotation.sin();
        let rotated_y = x * rotation.sin() + y * rotation.cos();
        points.push(center + Vec2::new(rotated_x, rotated_y));
    }
    painter.add(egui::Shape::line(
        offset_points(&points, Vec2::new(0.0, 8.0)),
        Stroke::new(56.0, Color32::from_rgba_unmultiplied(0, 0, 0, 54)),
    ));
    painter.add(egui::Shape::line(
        points.clone(),
        Stroke::new(48.0, Color32::from_rgba_unmultiplied(94, 102, 82, 90)),
    ));
    painter.add(egui::Shape::line(
        points.clone(),
        Stroke::new(32.0, Color32::from_rgba_unmultiplied(184, 154, 90, 162)),
    ));
    painter.add(egui::Shape::line(
        offset_points(&points, Vec2::new(-3.0, -4.0)),
        Stroke::new(9.0, Color32::from_rgba_unmultiplied(234, 229, 216, 214)),
    ));
    painter.add(egui::Shape::line(
        offset_points(&points, Vec2::new(4.0, 4.0)),
        Stroke::new(7.0, Color32::from_rgba_unmultiplied(94, 102, 82, 150)),
    ));
}

fn offset_points(points: &[Pos2], offset: Vec2) -> Vec<Pos2> {
    points.iter().map(|point| *point + offset).collect()
}

fn panel_title(panel: Panel) -> &'static str {
    match panel {
        Panel::Chat => "Concevoir Jarvis desktop Windows",
        Panel::Search => "Recherche Google",
        Panel::Extensions => "Modules d'extension",
        Panel::Automations => "Automatisations",
        Panel::Memory => "Memoire",
        Panel::Files => "Fichiers et dossiers",
        Panel::Logs => "Logs",
        Panel::Settings => "Parametres",
    }
}

fn network_text(status: &NetworkStatus) -> String {
    match status {
        NetworkStatus::Checking => "Verification".to_string(),
        NetworkStatus::Online { mbps, latency_ms } => {
            format!("{:.1} Mbps / {} ms", mbps, latency_ms)
        }
        NetworkStatus::Offline => "Off".to_string(),
    }
}

fn format_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let bytes = bytes as f64;
    if bytes >= GB {
        format!("{:.2} GB", bytes / GB)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes / MB)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes / KB)
    } else {
        format!("{bytes:.0} B")
    }
}

fn mut_text() -> Color32 {
    theme::TEXT_MUTED
}

fn chrono_like_timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    now.to_string()
}

fn measure_network() -> NetworkStatus {
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
    {
        Ok(client) => client,
        Err(_) => return NetworkStatus::Offline,
    };

    let started = Instant::now();
    let response = client
        .get("https://speed.cloudflare.com/__down?bytes=300000")
        .send();

    let Ok(response) = response else {
        return NetworkStatus::Offline;
    };
    if !response.status().is_success() {
        return NetworkStatus::Offline;
    }

    let bytes = match response.bytes() {
        Ok(bytes) => bytes,
        Err(_) => return NetworkStatus::Offline,
    };
    let elapsed = started.elapsed();
    if bytes.is_empty() || elapsed.as_secs_f32() <= 0.0 {
        return NetworkStatus::Offline;
    }

    NetworkStatus::Online {
        mbps: (bytes.len() as f32 * 8.0) / elapsed.as_secs_f32() / 1_000_000.0,
        latency_ms: elapsed.as_millis(),
    }
}

fn transcribe_recording(path: &PathBuf) -> String {
    let token = match env::var("HUGGINGFACE_API_TOKEN") {
        Ok(value) if !value.trim().is_empty() => value,
        _ => {
            return "Audio sauvegarde. Transcription desactivee: ajoute HUGGINGFACE_API_TOKEN dans l'environnement, puis relance Jarvis.".to_string();
        }
    };
    let model = env::var("HUGGINGFACE_TRANSCRIBE_MODEL")
        .unwrap_or_else(|_| "openai/whisper-large-v3-turbo".to_string());
    let audio = match fs::read(path) {
        Ok(audio) => audio,
        Err(error) => return format!("Impossible de lire l'audio: {error}"),
    };
    if audio.is_empty() {
        return "Le fichier audio est vide, rien a transcrire.".to_string();
    }

    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(90))
        .build()
    {
        Ok(client) => client,
        Err(error) => return format!("Client transcription indisponible: {error}"),
    };
    let url = format!("https://api-inference.huggingface.co/models/{model}");
    let response = client
        .post(url)
        .bearer_auth(token)
        .header(reqwest::header::CONTENT_TYPE, "audio/wav")
        .body(audio)
        .send();

    let Ok(response) = response else {
        return "Transcription impossible: verifier la connexion Internet.".to_string();
    };
    let status = response.status();
    let body = match response.text() {
        Ok(body) => body,
        Err(error) => return format!("Reponse transcription illisible: {error}"),
    };
    if !status.is_success() {
        return format!(
            "Hugging Face a refuse la transcription: HTTP {}. {}",
            status,
            preview_text(&body, 320)
        );
    }
    let transcript = parse_huggingface_transcript(&body).unwrap_or_else(|| body.trim().to_string());
    format_voice_brief(&transcript)
}

fn parse_huggingface_transcript(body: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    if let Some(text) = value.get("text").and_then(|value| value.as_str()) {
        return Some(text.trim().to_string());
    }
    if let Some(array) = value.as_array() {
        for item in array {
            if let Some(text) = item.get("text").and_then(|value| value.as_str()) {
                return Some(text.trim().to_string());
            }
        }
    }
    None
}

fn format_voice_brief(transcript: &str) -> String {
    let clean = transcript.split_whitespace().collect::<Vec<_>>().join(" ");
    if clean.is_empty() {
        return "Transcription vide: aucun contenu vocal exploitable.".to_string();
    }

    let sentences = split_sentences(&clean);
    let subject = sentences
        .first()
        .map(|sentence| preview_text(sentence, 140))
        .unwrap_or_else(|| preview_text(&clean, 140));
    let points = sentences
        .iter()
        .take(4)
        .map(|sentence| format!("- {}", preview_text(sentence, 180)))
        .collect::<Vec<_>>()
        .join("\n");
    let actions = sentences
        .iter()
        .filter(|sentence| looks_like_action(sentence))
        .take(4)
        .map(|sentence| format!("- {}", preview_text(sentence, 180)))
        .collect::<Vec<_>>();
    let action_block = if actions.is_empty() {
        "- Aucune action explicite detectee.".to_string()
    } else {
        actions.join("\n")
    };

    format!(
        "Transcription\n{}\n\nExtraction type NotebookLM\n- Sujet probable: {}\n- Points cles:\n{}\n- Actions detectees:\n{}",
        clean, subject, points, action_block
    )
}

fn split_sentences(text: &str) -> Vec<String> {
    text.split(['.', '!', '?', '\n'])
        .map(str::trim)
        .filter(|sentence| !sentence.is_empty())
        .map(str::to_string)
        .collect()
}

fn looks_like_action(sentence: &str) -> bool {
    let lower = sentence.to_lowercase();
    [
        "todo", "a faire", "il faut", "je dois", "on doit", "ajoute", "corrige", "modifie",
        "supprime", "cree", "lance", "push", "commit", "fix",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn preview_text(text: &str, max_chars: usize) -> String {
    let clean = text.trim();
    if clean.chars().count() <= max_chars {
        return clean.to_string();
    }
    let mut preview = clean.chars().take(max_chars).collect::<String>();
    preview.push_str("...");
    preview
}

fn google_search_summary(query: &str) -> String {
    let api_key = match env::var("GOOGLE_SEARCH_API_KEY") {
        Ok(value) if !value.trim().is_empty() => value,
        _ => {
            return "Google Search n'est pas configure. Ajoute GOOGLE_SEARCH_API_KEY et GOOGLE_SEARCH_ENGINE_ID dans l'environnement pour activer les resultats reels.".to_string();
        }
    };
    let engine_id = match env::var("GOOGLE_SEARCH_ENGINE_ID") {
        Ok(value) if !value.trim().is_empty() => value,
        _ => {
            return "Google Search n'est pas configure. Il manque GOOGLE_SEARCH_ENGINE_ID."
                .to_string();
        }
    };

    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(client) => client,
        Err(error) => return format!("Client recherche indisponible: {error}"),
    };

    let response = client
        .get("https://www.googleapis.com/customsearch/v1")
        .query(&[
            ("key", api_key.as_str()),
            ("cx", engine_id.as_str()),
            ("q", query),
            ("num", "5"),
        ])
        .send();

    let Ok(response) = response else {
        return "Recherche impossible: verifier la connexion Internet.".to_string();
    };
    if !response.status().is_success() {
        return format!("Google a refuse la recherche: HTTP {}", response.status());
    }
    let value: serde_json::Value = match response.json() {
        Ok(value) => value,
        Err(error) => return format!("Reponse Google illisible: {error}"),
    };
    let Some(items) = value.get("items").and_then(|items| items.as_array()) else {
        return "Aucun resultat Google exploitable.".to_string();
    };

    let mut lines = Vec::new();
    for (index, item) in items.iter().take(5).enumerate() {
        let title = item
            .get("title")
            .and_then(|value| value.as_str())
            .unwrap_or("Sans titre");
        let link = item
            .get("link")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let snippet = item
            .get("snippet")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        lines.push(format!("{}. {}\n{}\n{}", index + 1, title, snippet, link));
    }
    lines.join("\n\n")
}
