mod actions;
mod device;
mod i18n;
mod importer;
mod mcp;
mod model;
mod obs;
#[cfg(target_os = "macos")]
mod status_bar;
mod ui;

use anyhow::{Context, Result};
use device::{DeviceCommand, DeviceEvent};
use eframe::egui;
use i18n::Language;
use model::{Action, DeckConfig, KEYS, MediaKey};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant},
};

fn config_path() -> Result<PathBuf> {
    let dirs = directories::ProjectDirs::from("dev", "betterdeck", "BetterDeck")
        .context("localizar diretório de configuração")?;
    Ok(dirs.config_dir().join("deck.json"))
}

fn app_icon() -> Result<egui::IconData> {
    let source = image::load_from_memory(include_bytes!("../assets/betterdeck-app-icon.png"))
        .context("carregar ícone do BetterDeck")?
        .to_rgba8();
    let side = source.width().min(source.height());
    let square = image::imageops::crop_imm(
        &source,
        (source.width() - side) / 2,
        (source.height() - side) / 2,
        side,
        side,
    )
    .to_image();
    let rgba = image::imageops::resize(&square, 512, 512, image::imageops::FilterType::Lanczos3);
    Ok(egui::IconData {
        rgba: rgba.into_raw(),
        width: 512,
        height: 512,
    })
}

fn load_config(path: &PathBuf) -> DeckConfig {
    let saved = std::fs::read(path)
        .ok()
        .and_then(|data| serde_json::from_slice::<DeckConfig>(&data).ok());
    let mut config = saved.unwrap_or_else(|| {
        let mut config = DeckConfig::default();
        if let Ok(profiles) = importer::import_profiles()
            && !profiles.is_empty()
        {
            config.profiles = profiles;
        }
        config
    });
    config.normalize();
    config
}

fn save_config(path: &PathBuf, config: &DeckConfig) -> Result<()> {
    let parent = path.parent().context("diretório inválido")?;
    std::fs::create_dir_all(parent)?;
    let data = serde_json::to_vec_pretty(config)?;
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, data)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&temporary, std::fs::Permissions::from_mode(0o600))?;
    }
    std::fs::rename(temporary, path)?;
    Ok(())
}

struct BetterDeckApp {
    config: DeckConfig,
    path: PathBuf,
    selected: usize,
    status: String,
    saved_state: String,
    device_tx: Sender<DeviceCommand>,
    device_rx: Receiver<DeviceEvent>,
    action_tx: Sender<String>,
    action_rx: Receiver<String>,
    navigation_tx: Sender<Action>,
    navigation_rx: Receiver<Action>,
    asleep: bool,
    obs_open: bool,
    textures: HashMap<String, egui::TextureHandle>,
    logo_texture: Option<egui::TextureHandle>,
    key_drag: Option<ui::KeyDrag>,
    key_move_animation: Option<ui::KeyMoveAnimation>,
    key_rects: [Option<egui::Rect>; KEYS],
    mcp_setup_open: bool,
    pending_apply_at: Option<Instant>,
    mcp_bridge: Option<mcp::Bridge>,
    #[cfg(target_os = "macos")]
    status_bar: Option<status_bar::StatusBar>,
    #[cfg(target_os = "macos")]
    quit_requested: bool,
}

impl BetterDeckApp {
    fn new(path: PathBuf) -> Self {
        let mcp_bridge = mcp::Bridge::start(&path).ok();
        let config = load_config(&path);
        if !path.exists() {
            let _ = save_config(&path, &config);
        }
        let saved_state = serde_json::to_string(&config).unwrap_or_default();
        let (device_tx, device_commands) = mpsc::channel();
        let (device_events, device_rx) = mpsc::channel();
        device::spawn(device_commands, device_events);
        let _ = device_tx.send(DeviceCommand::Apply(config.clone()));
        let (action_tx, action_rx) = mpsc::channel();
        let (navigation_tx, navigation_rx) = mpsc::channel();
        Self {
            config,
            path,
            selected: 0,
            status: "Procurando D6…".into(),
            saved_state,
            device_tx,
            device_rx,
            action_tx,
            action_rx,
            navigation_tx,
            navigation_rx,
            asleep: false,
            obs_open: false,
            textures: HashMap::new(),
            logo_texture: None,
            key_drag: None,
            key_move_animation: None,
            key_rects: [None; KEYS],
            mcp_setup_open: false,
            pending_apply_at: None,
            mcp_bridge,
            #[cfg(target_os = "macos")]
            status_bar: None,
            #[cfg(target_os = "macos")]
            quit_requested: false,
        }
    }

    fn texture_for(&mut self, ctx: &egui::Context, path: &str) -> Option<egui::TextureId> {
        if let Some(texture) = self.textures.get(path) {
            return Some(texture.id());
        }
        let image = image::open(path)
            .ok()?
            .resize_to_fill(112, 112, image::imageops::FilterType::Triangle)
            .to_rgba8();
        let pixels = egui::ColorImage::from_rgba_unmultiplied([112, 112], image.as_raw());
        let texture = ctx.load_texture(path, pixels, egui::TextureOptions::LINEAR);
        let id = texture.id();
        self.textures.insert(path.to_owned(), texture);
        Some(id)
    }

    fn press(&mut self, index: usize) {
        let action = self.config.current_page().keys[index].action.clone();
        self.dispatch_action(action);
    }

    fn dispatch_action(&mut self, action: Action) {
        match action {
            Action::NextPage => {
                let profile = self.config.current_profile_mut();
                profile.active_page = (profile.active_page + 1) % profile.pages.len();
            }
            Action::PreviousPage => {
                let profile = self.config.current_profile_mut();
                profile.active_page =
                    (profile.active_page + profile.pages.len() - 1) % profile.pages.len();
            }
            Action::GoToPage(index) => {
                let profile = self.config.current_profile_mut();
                if index < profile.pages.len() {
                    profile.active_page = index;
                }
            }
            Action::SwitchProfile(index) => {
                if index < self.config.profiles.len() {
                    self.config.active_profile = index;
                }
            }
            Action::Brightness(value) => {
                self.config.brightness = value.min(100);
            }
            Action::Sleep => {
                self.asleep = !self.asleep;
                let _ = self.device_tx.send(DeviceCommand::Sleep(self.asleep));
            }
            action => {
                let tx = self.action_tx.clone();
                let navigation = self.navigation_tx.clone();
                let connection = self.config.obs.clone();
                thread::spawn(move || {
                    if let Err(error) = actions::execute(&action, &connection, &navigation) {
                        let _ = tx.send(error.to_string());
                    }
                });
            }
        }
    }
}

fn action_text(ui: &mut egui::Ui, label: &str, value: &mut String, multiline: bool) {
    ui.add_space(11.0);
    ui.label(
        egui::RichText::new(label)
            .size(12.0)
            .color(egui::Color32::from_rgb(211, 210, 212)),
    );
    ui.add_space(4.0);
    let edit = if multiline {
        egui::TextEdit::multiline(value)
    } else {
        egui::TextEdit::singleline(value)
    };
    ui.add_sized(
        [ui.available_width(), if multiline { 72.0 } else { 36.0 }],
        edit.background_color(egui::Color32::from_rgb(26, 25, 27))
            .desired_width(ui.available_width()),
    );
}

fn edit_action(
    ui: &mut egui::Ui,
    action: &mut Action,
    profiles: &[String],
    pages: &[String],
    depth: usize,
    language: Language,
) {
    let mut kind = action.kind();
    let popup_id = ui.make_persistent_id(("action-popup", depth, action as *const Action as usize));
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 40.0), egui::Sense::click());
    ui.painter().rect_filled(
        rect,
        8.0,
        if response.hovered() {
            egui::Color32::from_rgb(65, 59, 68)
        } else {
            egui::Color32::from_rgb(57, 56, 58)
        },
    );
    ui.painter().text(
        rect.left_center() + egui::vec2(12.0, 0.0),
        egui::Align2::LEFT_CENTER,
        language.tr(Action::KINDS[kind]),
        egui::FontId::proportional(12.0),
        egui::Color32::from_rgb(227, 226, 228),
    );
    let arrow = rect.right_center() - egui::vec2(18.0, 0.0);
    let stroke = egui::Stroke::new(1.5, egui::Color32::from_rgb(200, 199, 201));
    ui.painter()
        .line_segment([arrow + egui::vec2(-4.0, -2.0), arrow], stroke);
    ui.painter()
        .line_segment([arrow, arrow + egui::vec2(4.0, -2.0)], stroke);
    if response.clicked() {
        ui.memory_mut(|memory| memory.toggle_popup(popup_id));
    }
    ui.scope(|ui| {
        ui.visuals_mut().window_fill = egui::Color32::from_rgb(43, 42, 44);
        ui.visuals_mut().window_stroke =
            egui::Stroke::new(1.0, egui::Color32::from_rgb(77, 72, 79));
        ui.visuals_mut().menu_corner_radius = egui::CornerRadius::same(9);
        ui.spacing_mut().menu_margin = egui::Margin::same(6);
        egui::popup::popup_below_widget(
            ui,
            popup_id,
            &response,
            egui::popup::PopupCloseBehavior::CloseOnClickOutside,
            |ui| {
                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        for (index, name) in Action::KINDS.iter().enumerate() {
                            let (item, clicked) = ui.allocate_exact_size(
                                egui::vec2(ui.available_width(), 32.0),
                                egui::Sense::click(),
                            );
                            if kind == index || clicked.hovered() {
                                ui.painter().rect_filled(
                                    item,
                                    6.0,
                                    if kind == index {
                                        egui::Color32::from_rgb(66, 49, 73)
                                    } else {
                                        egui::Color32::from_rgb(55, 51, 58)
                                    },
                                );
                            }
                            ui.painter().text(
                                item.left_center() + egui::vec2(11.0, 0.0),
                                egui::Align2::LEFT_CENTER,
                                language.tr(name),
                                egui::FontId::proportional(12.0),
                                if kind == index {
                                    egui::Color32::from_rgb(242, 215, 245)
                                } else {
                                    egui::Color32::from_rgb(193, 192, 194)
                                },
                            );
                            if clicked.clicked() {
                                kind = index;
                                ui.memory_mut(|memory| memory.close_popup());
                            }
                        }
                    });
            },
        );
    });
    if kind != action.kind() {
        *action = Action::from_kind(kind);
    }
    match action {
        Action::Open(value) => {
            action_text(ui, language.tr("Nome de app, caminho ou URL"), value, false);
        }
        Action::Hotkey(value) => {
            action_text(ui, language.tr("Atalho (ex.: cmd+shift+4)"), value, false);
        }
        Action::Text(value) => {
            action_text(ui, language.tr("Texto a digitar"), value, true);
        }
        Action::Media(value) => {
            egui::ComboBox::from_id_salt(("media", depth))
                .selected_text(language.tr(value.label()))
                .show_ui(ui, |ui| {
                    for key in MediaKey::ALL {
                        ui.selectable_value(value, key, language.tr(key.label()));
                    }
                });
        }
        Action::Volume(value) => {
            ui.label(language.tr("Mudança de volume (0 = mudo)"));
            ui.add(egui::Slider::new(value, -100..=100));
        }
        Action::Brightness(value) => {
            ui.add(egui::Slider::new(value, 0..=100));
        }
        Action::GoToPage(index) => {
            egui::ComboBox::from_id_salt(("page", depth))
                .selected_text(
                    pages
                        .get(*index)
                        .map(String::as_str)
                        .unwrap_or(language.tr("Página")),
                )
                .show_ui(ui, |ui| {
                    for (i, name) in pages.iter().enumerate() {
                        ui.selectable_value(index, i, name);
                    }
                });
        }
        Action::SwitchProfile(index) => {
            egui::ComboBox::from_id_salt(("profile", depth))
                .selected_text(
                    profiles
                        .get(*index)
                        .map(String::as_str)
                        .unwrap_or(language.tr("Perfil")),
                )
                .show_ui(ui, |ui| {
                    for (i, name) in profiles.iter().enumerate() {
                        ui.selectable_value(index, i, name);
                    }
                });
        }
        Action::Shell(value) => {
            action_text(ui, language.tr("Comando zsh"), value, true);
        }
        Action::HttpGet(value) => {
            action_text(ui, "URL", value, false);
        }
        Action::Obs(value) => {
            let mut kind = value.kind();
            egui::ComboBox::from_id_salt(("obs", depth))
                .selected_text(language.tr(crate::model::ObsAction::KINDS[kind]))
                .show_ui(ui, |ui| {
                    for (i, name) in crate::model::ObsAction::KINDS.iter().enumerate() {
                        ui.selectable_value(&mut kind, i, language.tr(name));
                    }
                });
            if kind != value.kind() {
                *value = crate::model::ObsAction::from_kind(kind);
            }
            match value {
                crate::model::ObsAction::SwitchScene(name) => {
                    ui.label(language.tr("Nome da cena"));
                    ui.text_edit_singleline(name);
                }
                crate::model::ObsAction::ToggleInputMute(name) => {
                    ui.label(language.tr("Nome da entrada"));
                    ui.text_edit_singleline(name);
                }
                crate::model::ObsAction::ToggleSourceVisibility { scene, source } => {
                    ui.label(language.tr("Cena"));
                    ui.text_edit_singleline(scene);
                    ui.label(language.tr("Fonte"));
                    ui.text_edit_singleline(source);
                }
                _ => {}
            }
        }
        Action::Unsupported { uuid, .. } => {
            ui.label(format!(
                "{}: {uuid}",
                if language == Language::English {
                    "Plugin without an executor"
                } else {
                    "Plugin sem executor"
                }
            ));
        }
        Action::Wheesy(value) => {
            action_text(ui, language.tr("Comando do Wheesy"), value, false);
        }
        Action::WarpTab(value) => {
            action_text(ui, language.tr("Pasta da nova aba"), value, false);
        }
        Action::WarpSsh { host, name } => {
            action_text(ui, "Host SSH", host, false);
            action_text(ui, language.tr("Nome da configuração"), name, false);
        }
        Action::Delay(value) => {
            ui.label(language.tr("Atraso em milissegundos"));
            ui.add(egui::Slider::new(value, 0..=30_000));
        }
        Action::Sequence(steps) if depth < 2 => {
            let mut remove = None;
            for (index, step) in steps.iter_mut().enumerate() {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(format!(
                            "{} {}",
                            if language == Language::English {
                                "Step"
                            } else {
                                "Passo"
                            },
                            index + 1
                        ));
                        if ui.small_button("×").clicked() {
                            remove = Some(index);
                        }
                    });
                    edit_action(ui, step, profiles, pages, depth + index + 1, language);
                });
            }
            if let Some(index) = remove {
                steps.remove(index);
            }
            if steps.len() < 32 {
                if ui.button(language.tr("+ Ação")).clicked() {
                    steps.push(Action::None);
                }
                if ui.button(language.tr("+ Atraso")).clicked() {
                    steps.push(Action::Delay(500));
                }
            }
        }
        _ => {}
    }
}

impl eframe::App for BetterDeckApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(target_os = "macos")]
        {
            if self.status_bar.is_none() {
                match status_bar::StatusBar::new(ctx, self.config.language) {
                    Ok(status_bar) => self.status_bar = Some(status_bar),
                    Err(error) => self.status = format!("Barra de menus: {error}"),
                }
            }
            if let Some(status_bar) = &mut self.status_bar {
                status_bar.set_language(self.config.language);
                for command in status_bar.commands() {
                    match command {
                        status_bar::StatusCommand::Show => {
                            if let Err(error) = status_bar::set_dock_visible(true) {
                                self.status = error.to_string();
                            }
                            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                        }
                        status_bar::StatusCommand::Quit => {
                            self.quit_requested = true;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    }
                }
                if ctx.input(|input| input.viewport().close_requested()) && !self.quit_requested {
                    ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                    if let Err(error) = status_bar::set_dock_visible(false) {
                        self.status = error.to_string();
                    }
                }
            }
        }
        while let Ok(event) = self.device_rx.try_recv() {
            match event {
                DeviceEvent::Status(status) => {
                    eprintln!("{status}");
                    self.status = status;
                }
                DeviceEvent::Error(error) => {
                    eprintln!("{error}");
                    self.status = error;
                }
                DeviceEvent::Press(index) if index < KEYS => self.press(index),
                DeviceEvent::Press(_) => {}
            }
        }
        while let Ok(error) = self.action_rx.try_recv() {
            self.status = error;
        }
        while let Ok(action) = self.navigation_rx.try_recv() {
            self.dispatch_action(action);
        }
        if let Some(bridge) = &self.mcp_bridge {
            let requests: Vec<_> = bridge.requests.try_iter().collect();
            for request in requests {
                let result = mcp::apply(self, &request.name, &request.arguments);
                let _ = request.reply.send(result);
            }
        }
        self.toolbar(ctx);
        self.sidebar(ctx);
        self.inspector(ctx);
        self.grid(ctx);
        self.mcp_setup(ctx);
        if let Ok(snapshot) = serde_json::to_string(&self.config)
            && snapshot != self.saved_state
        {
            self.saved_state = snapshot;
            if let Err(error) = save_config(&self.path, &self.config) {
                self.status = format!("Erro ao salvar: {error}");
            }
            self.pending_apply_at = Some(Instant::now());
        }
        if self
            .pending_apply_at
            .is_some_and(|at| at.elapsed() >= Duration::from_millis(250))
        {
            let _ = self
                .device_tx
                .send(DeviceCommand::Apply(self.config.clone()));
            self.pending_apply_at = None;
        }
        ctx.request_repaint_after(Duration::from_millis(100));
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        let _ = self.device_tx.send(DeviceCommand::Stop);
    }
}

fn main() -> Result<()> {
    if std::env::args().any(|arg| arg == "--mcp") {
        return mcp::run_stdio(&config_path()?);
    }
    if std::env::args().any(|arg| arg == "--import-preview") {
        for profile in importer::import_profiles()? {
            let assigned = profile.pages[0]
                .keys
                .iter()
                .filter(|key| !matches!(key.action, Action::None))
                .count();
            let unsupported = profile.pages[0]
                .keys
                .iter()
                .filter(|key| matches!(key.action, Action::Unsupported { .. }))
                .count();
            println!(
                "{}: {} teclas, {} plugins sem executor",
                profile.name, assigned, unsupported
            );
        }
        return Ok(());
    }
    if std::env::args().any(|arg| arg == "--probe") {
        println!("{}", device::probe()?);
        return Ok(());
    }
    if std::env::args().any(|arg| arg == "--hardware-test") {
        println!("{}", device::hardware_test()?);
        return Ok(());
    }
    let path = config_path()?;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1360.0, 800.0])
            .with_min_inner_size([1100.0, 720.0])
            .with_icon(app_icon()?),
        ..Default::default()
    };
    eframe::run_native(
        "BetterDeck",
        options,
        Box::new(move |cc| {
            let mut visuals = egui::Visuals::dark();
            visuals.selection.bg_fill = ui::ACCENT;
            cc.egui_ctx.set_visuals(visuals);
            Ok(Box::new(BetterDeckApp::new(path)))
        }),
    )
    .map_err(|error| anyhow::anyhow!(error.to_string()))
}
