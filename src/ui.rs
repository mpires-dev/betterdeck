use crate::device::DeviceCommand;
use crate::i18n::Language;
use crate::model::{Action, COLUMNS, KeyConfig, Page, Profile};
use crate::{BetterDeckApp, edit_action};
use eframe::egui::{
    self, Align, Align2, Color32, FontId, Frame, Layout, Margin, RichText, Sense, Stroke,
    StrokeKind, Vec2,
};
use std::time::{Duration, Instant};

const BG: Color32 = Color32::from_rgb(23, 22, 24);
const PANEL: Color32 = Color32::from_rgb(35, 34, 36);
const CENTER: Color32 = Color32::from_rgb(33, 32, 34);
const TILE: Color32 = Color32::from_rgb(43, 42, 44);
const FIELD: Color32 = Color32::from_rgb(26, 25, 27);
const CODE_BG: Color32 = Color32::from_rgb(20, 19, 22);
const OUTLINE: Color32 = Color32::from_rgb(67, 62, 70);
pub(crate) const ACCENT: Color32 = Color32::from_rgb(216, 139, 227);
const TEXT: Color32 = Color32::from_rgb(246, 245, 247);
const MUTED: Color32 = Color32::from_rgb(163, 162, 164);
const KEY_MOVE_DURATION: Duration = Duration::from_millis(260);

pub(crate) struct KeyDrag {
    index: usize,
    profile: usize,
    page: usize,
    grab_offset: Vec2,
    size: Vec2,
    hover: Option<usize>,
}

pub(crate) struct KeyMoveAnimation {
    profile: usize,
    page: usize,
    started: Instant,
    from: Vec<(usize, egui::Rect)>,
}

fn title(text: impl Into<String>, size: f32) -> RichText {
    RichText::new(text.into()).size(size).color(TEXT)
}

fn section(text: &str) -> RichText {
    RichText::new(text).size(10.0).strong().color(MUTED)
}

fn muted(text: impl Into<String>, size: f32) -> RichText {
    RichText::new(text.into()).size(size).color(MUTED)
}

fn panel(fill: Color32, inset: i8) -> Frame {
    Frame::new()
        .fill(fill)
        .corner_radius(12)
        .inner_margin(Margin::same(inset))
}

#[derive(Clone, Copy)]
enum Icon {
    Grid,
    Radio,
    Layers,
    Plus,
    Power,
    ChevronLeft,
    ChevronRight,
    Image,
    Trash,
    Info,
    Play,
    Plug,
}

fn draw_icon(p: &egui::Painter, rect: egui::Rect, icon: Icon, color: Color32) {
    let center = rect.center();
    let unit = rect.width().min(rect.height()) / 16.0;
    let point = |x: f32, y: f32| center + egui::vec2(x * unit, y * unit);
    let stroke = Stroke::new((1.6 * unit).max(1.2), color);
    match icon {
        Icon::Grid => {
            for y in [-4.0, 3.0] {
                for x in [-4.0, 3.0] {
                    p.rect_stroke(
                        egui::Rect::from_min_size(point(x, y), Vec2::splat(5.0 * unit)),
                        1.0 * unit,
                        stroke,
                        StrokeKind::Inside,
                    );
                }
            }
        }
        Icon::Radio => {
            p.circle_filled(center, 1.5 * unit, color);
            p.circle_stroke(center, 4.3 * unit, stroke);
            p.circle_stroke(center, 7.0 * unit, stroke);
        }
        Icon::Layers => {
            for y in [-4.5, 0.0, 4.5] {
                p.line_segment([point(-6.0, y), point(0.0, y + 3.0)], stroke);
                p.line_segment([point(0.0, y + 3.0), point(6.0, y)], stroke);
            }
        }
        Icon::Plus => {
            p.line_segment([point(-5.0, 0.0), point(5.0, 0.0)], stroke);
            p.line_segment([point(0.0, -5.0), point(0.0, 5.0)], stroke);
        }
        Icon::Power => {
            p.circle_stroke(point(0.0, 1.0), 5.8 * unit, stroke);
            p.line_segment(
                [point(0.0, -7.0), point(0.0, 1.0)],
                Stroke::new(stroke.width + 0.8, color),
            );
        }
        Icon::ChevronLeft => {
            p.line_segment([point(2.0, -5.0), point(-3.0, 0.0)], stroke);
            p.line_segment([point(-3.0, 0.0), point(2.0, 5.0)], stroke);
        }
        Icon::ChevronRight => {
            p.line_segment([point(-2.0, -5.0), point(3.0, 0.0)], stroke);
            p.line_segment([point(3.0, 0.0), point(-2.0, 5.0)], stroke);
        }
        Icon::Image => {
            p.rect_stroke(
                egui::Rect::from_min_max(point(-7.0, -5.0), point(4.0, 6.0)),
                1.5 * unit,
                stroke,
                StrokeKind::Inside,
            );
            p.circle_filled(point(-3.5, -1.5), 1.3 * unit, color);
            p.line_segment([point(-6.0, 4.0), point(-2.0, 1.0)], stroke);
            p.line_segment([point(-2.0, 1.0), point(1.0, 4.0)], stroke);
            p.line_segment([point(5.0, -7.0), point(5.0, -1.0)], stroke);
            p.line_segment([point(2.0, -4.0), point(8.0, -4.0)], stroke);
        }
        Icon::Trash => {
            p.rect_stroke(
                egui::Rect::from_min_max(point(-5.0, -3.0), point(5.0, 7.0)),
                1.0 * unit,
                stroke,
                StrokeKind::Inside,
            );
            p.line_segment([point(-7.0, -5.0), point(7.0, -5.0)], stroke);
            p.line_segment([point(-2.0, -7.0), point(2.0, -7.0)], stroke);
            p.line_segment([point(-2.0, 0.0), point(-2.0, 4.0)], stroke);
            p.line_segment([point(2.0, 0.0), point(2.0, 4.0)], stroke);
        }
        Icon::Info => {
            p.circle_stroke(center, 6.5 * unit, stroke);
            p.circle_filled(point(0.0, -3.0), 0.9 * unit, color);
            p.line_segment([point(0.0, -0.5), point(0.0, 4.0)], stroke);
        }
        Icon::Play => {
            p.add(egui::Shape::convex_polygon(
                vec![point(-4.0, -6.0), point(6.0, 0.0), point(-4.0, 6.0)],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Plug => {
            p.line_segment([point(-3.0, -7.0), point(-3.0, -2.0)], stroke);
            p.line_segment([point(3.0, -7.0), point(3.0, -2.0)], stroke);
            p.rect_stroke(
                egui::Rect::from_min_max(point(-5.0, -2.0), point(5.0, 3.0)),
                1.0 * unit,
                stroke,
                StrokeKind::Inside,
            );
            p.line_segment([point(0.0, 3.0), point(0.0, 7.0)], stroke);
        }
    }
}

fn row_button(
    ui: &mut egui::Ui,
    icon: Icon,
    name: &str,
    active: bool,
    height: f32,
) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::click());
    if active || response.hovered() {
        ui.painter().rect_filled(
            rect,
            10.0,
            if active {
                Color32::from_rgb(58, 45, 63)
            } else {
                Color32::from_rgb(50, 49, 51)
            },
        );
    }
    draw_icon(
        ui.painter(),
        egui::Rect::from_center_size(
            rect.left_center() + egui::vec2(20.0, 0.0),
            Vec2::splat(16.0),
        ),
        icon,
        if active { ACCENT } else { MUTED },
    );
    ui.painter().text(
        rect.left_center() + egui::vec2(37.0, 0.0),
        Align2::LEFT_CENTER,
        name,
        FontId::proportional(12.0),
        if active {
            TEXT
        } else {
            Color32::from_rgb(184, 183, 185)
        },
    );
    response
}

fn icon_button(ui: &mut egui::Ui, icon: Icon, size: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    ui.painter().rect_filled(
        rect,
        8.0,
        if response.hovered() {
            Color32::from_rgb(65, 60, 67)
        } else {
            Color32::from_rgb(53, 52, 54)
        },
    );
    draw_icon(
        ui.painter(),
        rect.shrink(6.0),
        icon,
        Color32::from_rgb(200, 199, 201),
    );
    response
}

fn image_picker(language: Language) -> Option<String> {
    let script = if language == Language::English {
        "POSIX path of (choose file with prompt \"Choose a key image\" of type {\"public.image\"})"
    } else {
        "POSIX path of (choose file with prompt \"Escolher imagem da tecla\" of type {\"public.image\"})"
    };
    let output = std::process::Command::new("osascript")
        .args(["-e", script])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn mcp_code_card(
    ui: &mut egui::Ui,
    language_setting: Language,
    heading: &str,
    description: &str,
    format: (&str, &str),
    code: &str,
    copy_id: &'static str,
) {
    let (filename, language) = format;
    Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, OUTLINE))
        .corner_radius(12)
        .inner_margin(Margin::same(16))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(title(heading, 15.0).strong());
            ui.add_space(3.0);
            ui.label(muted(description, 11.0));
            ui.add_space(12.0);

            Frame::new()
                .fill(CODE_BG)
                .stroke(Stroke::new(1.0, Color32::from_rgb(54, 51, 57)))
                .corner_radius(8)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    Frame::new()
                        .inner_margin(Margin {
                            left: 12,
                            right: 8,
                            top: 7,
                            bottom: 7,
                        })
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(filename).monospace().size(10.0).color(MUTED),
                                );
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    let id = ui.make_persistent_id(copy_id);
                                    let copied = ui.ctx().data(|data| {
                                        data.get_temp::<Instant>(id)
                                            .is_some_and(|time| time.elapsed().as_secs_f32() < 2.0)
                                    });
                                    let label = if copied {
                                        language_setting.tr("✓ Copiado")
                                    } else {
                                        language_setting.tr("Copiar código")
                                    };
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new(label).size(10.0).color(if copied {
                                                    ACCENT
                                                } else {
                                                    TEXT
                                                }),
                                            )
                                            .fill(if copied {
                                                Color32::from_rgb(58, 45, 63)
                                            } else {
                                                TILE
                                            })
                                            .stroke(Stroke::new(1.0, OUTLINE))
                                            .corner_radius(6),
                                        )
                                        .clicked()
                                    {
                                        ui.ctx().copy_text(code.to_owned());
                                        ui.ctx()
                                            .data_mut(|data| data.insert_temp(id, Instant::now()));
                                    }
                                    if copied {
                                        ui.ctx().request_repaint_after(Duration::from_millis(200));
                                    }
                                    ui.label(muted(language, 10.0));
                                });
                            });
                        });
                    ui.separator();
                    Frame::new()
                        .inner_margin(Margin {
                            left: 12,
                            right: 12,
                            top: 9,
                            bottom: 11,
                        })
                        .show(ui, |ui| {
                            egui::ScrollArea::horizontal()
                                .id_salt(copy_id)
                                .auto_shrink([false, true])
                                .show(ui, |ui| {
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(code)
                                                .monospace()
                                                .size(11.0)
                                                .color(Color32::from_rgb(217, 211, 221)),
                                        )
                                        .wrap_mode(egui::TextWrapMode::Extend),
                                    );
                                });
                        });
                });
        });
}

impl BetterDeckApp {
    pub(super) fn mcp_setup(&mut self, ctx: &egui::Context) {
        if !self.mcp_setup_open {
            return;
        }
        let executable = std::env::current_exe()
            .unwrap_or_default()
            .display()
            .to_string();
        let claude = serde_json::to_string_pretty(&serde_json::json!({
            "mcpServers": {"betterdeck": {"command": executable, "args": ["--mcp"]}}
        }))
        .unwrap_or_default();
        let codex = format!(
            "[mcp_servers.betterdeck]\ncommand = {}\nargs = [\"--mcp\"]",
            serde_json::to_string(&executable).unwrap_or_default()
        );
        let mut open = self.mcp_setup_open;
        let mut close_clicked = false;
        let language = self.config.language;
        egui::Window::new(language.tr("Conectar agentes ao BetterDeck"))
            .open(&mut open)
            .title_bar(false)
            .collapsible(false)
            .resizable(true)
            .default_width(620.0)
            .min_width(420.0)
            .frame(
                Frame::new()
                    .fill(BG)
                    .stroke(Stroke::new(1.0, OUTLINE))
                    .corner_radius(16)
                    .inner_margin(Margin::same(20)),
            )
            .show(ctx, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(title(language.tr("Conectar agentes"), 21.0).strong());
                        ui.label(muted(language.tr("Use o BetterDeck com seus clientes MCP locais."), 11.0));
                    });
                    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                        if ui
                            .add(
                                egui::Button::new(RichText::new("×").size(19.0).color(MUTED))
                                    .fill(TILE)
                                    .stroke(Stroke::NONE)
                                    .corner_radius(8),
                            )
                            .on_hover_text(language.tr("Fechar"))
                            .clicked()
                        {
                            close_clicked = true;
                        }
                    });
                });
                ui.add_space(15.0);
                let connected = self.mcp_bridge.is_some();
                Frame::new()
                    .fill(if connected {
                        Color32::from_rgb(31, 48, 42)
                    } else {
                        Color32::from_rgb(58, 42, 42)
                    })
                    .corner_radius(8)
                    .inner_margin(Margin::symmetric(12, 9))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(if connected {
                                language.tr("●  Servidor MCP ativo · o BetterDeck pode ficar na barra de menus")
                            } else {
                                language.tr("●  Servidor MCP indisponível · feche outra instância e reinicie o BetterDeck")
                            })
                            .size(11.0)
                            .color(if connected {
                                Color32::from_rgb(163, 223, 189)
                            } else {
                                Color32::from_rgb(242, 178, 170)
                            }),
                        );
                    });
                ui.add_space(16.0);
                mcp_code_card(
                    ui,
                    language,
                    language.tr("Claude Code e outros clientes MCP"),
                    language.tr("Salve na raiz do projeto como .mcp.json."),
                    (".mcp.json", "JSON"),
                    &claude,
                    "mcp-copy-json",
                );
                ui.add_space(12.0);
                mcp_code_card(
                    ui,
                    language,
                    "Codex",
                    language.tr("Adicione ao arquivo de configuração do Codex."),
                    ("~/.codex/config.toml", "TOML"),
                    &codex,
                    "mcp-copy-toml",
                );
                ui.add_space(15.0);
                ui.label(muted(
                    language.tr("Acesso local a status, perfis, páginas, teclas, brilho, tela e OBS."),
                    10.0,
                ));
            });
        self.mcp_setup_open = open && !close_clicked;
    }

    pub(super) fn toolbar(&mut self, ctx: &egui::Context) {
        let language = self.config.language;
        if self.logo_texture.is_none()
            && let Ok(image) =
                image::load_from_memory(include_bytes!("../assets/betterdeck-logo.png"))
        {
            let rgba = image.to_rgba8();
            let pixels = egui::ColorImage::from_rgba_unmultiplied(
                [rgba.width() as usize, rgba.height() as usize],
                rgba.as_raw(),
            );
            self.logo_texture =
                Some(ctx.load_texture("betterdeck-logo", pixels, egui::TextureOptions::LINEAR));
        }
        egui::TopBottomPanel::top("toolbar")
            .exact_height(56.0)
            .show_separator_line(false)
            .frame(Frame::new().fill(BG).inner_margin(Margin {
                left: 16,
                right: 18,
                top: 9,
                bottom: 8,
            }))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if let Some(texture) = &self.logo_texture {
                        let (rect, _) =
                            ui.allocate_exact_size(Vec2::new(152.0, 27.0), Sense::hover());
                        ui.painter().image(
                            texture.id(),
                            rect,
                            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    }
                    ui.add_space(18.0);
                    let (line, _) = ui.allocate_exact_size(Vec2::new(1.0, 26.0), Sense::hover());
                    ui.painter()
                        .rect_filled(line, 0.0, Color32::from_rgb(67, 66, 68));
                    ui.add_space(18.0);
                    ui.vertical(|ui| {
                        ui.label(title("Stream Deck D6", 13.0));
                        ui.label(muted("USB · 3142:0060", 10.0));
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let connected = self.status.starts_with("D6 conectado");
                        Frame::new()
                            .fill(Color32::from_rgb(27, 41, 45))
                            .corner_radius(8)
                            .inner_margin(Margin::symmetric(12, 9))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let (dot, _) =
                                        ui.allocate_exact_size(Vec2::splat(7.0), Sense::hover());
                                    ui.painter().circle_filled(
                                        dot.center(),
                                        3.5,
                                        if connected {
                                            Color32::from_rgb(125, 216, 164)
                                        } else {
                                            MUTED
                                        },
                                    );
                                    ui.label(
                                        RichText::new(if connected {
                                            language.tr("D6 conectado")
                                        } else {
                                            language.tr("D6 desconectado")
                                        })
                                        .size(11.0)
                                        .color(
                                            if connected {
                                                Color32::from_rgb(188, 231, 208)
                                            } else {
                                                MUTED
                                            },
                                        ),
                                    )
                                    .on_hover_text(language.status(&self.status));
                                });
                            });
                        ui.add_space(8.0);
                        let (rect, response) =
                            ui.allocate_exact_size(Vec2::new(185.0, 34.0), Sense::click());
                        ui.painter().rect_filled(
                            rect,
                            8.0,
                            if response.hovered() {
                                Color32::from_rgb(75, 54, 84)
                            } else {
                                Color32::from_rgb(56, 42, 64)
                            },
                        );
                        draw_icon(
                            ui.painter(),
                            egui::Rect::from_center_size(
                                rect.left_center() + egui::vec2(20.0, 0.0),
                                Vec2::splat(16.0),
                            ),
                            Icon::Plug,
                            ACCENT,
                        );
                        ui.painter().text(
                            rect.left_center() + egui::vec2(37.0, 0.0),
                            Align2::LEFT_CENTER,
                            language.tr("Setup MCP Connection"),
                            FontId::proportional(11.0),
                            Color32::from_rgb(234, 233, 235),
                        );
                        if response.clicked() {
                            self.mcp_setup_open = true;
                        }
                        ui.add_space(8.0);
                        egui::ComboBox::from_id_salt("app-language")
                            .selected_text(self.config.language.label())
                            .width(82.0)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.config.language,
                                    Language::Portuguese,
                                    "Português",
                                );
                                ui.selectable_value(
                                    &mut self.config.language,
                                    Language::English,
                                    "English",
                                );
                            });
                    });
                });
            });
    }

    pub(super) fn sidebar(&mut self, ctx: &egui::Context) {
        let language = self.config.language;
        egui::SidePanel::left("sidebar")
            .exact_width(242.0)
            .resizable(false)
            .show_separator_line(false)
            .frame(Frame::new().fill(BG).inner_margin(Margin { left: 16, right: 0, top: 4, bottom: 0 }))
            .show(ctx, |ui| {
                panel(PANEL, 18).show(ui, |ui| {
                    ui.set_min_width(190.0);
                    ui.set_min_height(ui.available_height().max(530.0));
                    let obs = row_button(ui, Icon::Radio, "OBS WebSocket", true, 34.0);
                    draw_icon(ui.painter(), egui::Rect::from_center_size(obs.rect.right_center() - egui::vec2(14.0, 0.0), Vec2::splat(14.0)), if self.obs_open { Icon::ChevronLeft } else { Icon::ChevronRight }, MUTED);
                    if obs.clicked() { self.obs_open = !self.obs_open; }
                    if self.obs_open {
                        ui.add_space(8.0);
                        ui.label(muted(language.tr("Endereço"), 11.0));
                        ui.add_sized([ui.available_width(), 34.0], egui::TextEdit::singleline(&mut self.config.obs.address).background_color(FIELD));
                        ui.label(muted(language.tr("Senha"), 11.0));
                        ui.add_sized([ui.available_width(), 34.0], egui::TextEdit::singleline(&mut self.config.obs.password).password(true).background_color(FIELD));
                    }
                    ui.add_space(25.0);
                    ui.label(section(language.tr("PERFIS")));
                    ui.add_space(6.0);
                    let mut profile_target = None;
                    let mut profile_delete = None;
                    for (index, profile) in self.config.profiles.iter().enumerate() {
                        let response = row_button(ui, Icon::Layers, &profile.name, index == self.config.active_profile, 36.0);
                        if response.clicked() { profile_target = Some(index); }
                        if self.config.profiles.len() > 1 {
                            response.context_menu(|ui| {
                                if ui.button(language.tr("Excluir perfil")).clicked() { profile_delete = Some(index); ui.close_menu(); }
                            });
                        }
                    }
                    if let Some(index) = profile_target { self.config.active_profile = index; }
                    if let Some(index) = profile_delete {
                        self.config.profiles.remove(index);
                        self.config.active_profile = self.config.active_profile.min(self.config.profiles.len() - 1);
                    }
                    if row_button(ui, Icon::Plus, language.tr("Novo perfil"), false, 30.0).clicked() {
                        self.config.profiles.push(Profile::new(format!("{} {}", language.tr("Perfil"), self.config.profiles.len() + 1)));
                        self.config.active_profile = self.config.profiles.len() - 1;
                    }
                    ui.add_space(19.0);
                    ui.label(section(language.tr("PÁGINAS")));
                    ui.add_space(6.0);
                    let mut page_target = None;
                    let mut page_delete = None;
                    for (index, page) in self.config.current_profile().pages.iter().enumerate() {
                        let response = row_button(ui, Icon::Grid, &page.name, index == self.config.current_profile().active_page, 36.0);
                        if response.clicked() { page_target = Some(index); }
                        if self.config.current_profile().pages.len() > 1 {
                            response.context_menu(|ui| {
                                if ui.button(language.tr("Excluir página")).clicked() { page_delete = Some(index); ui.close_menu(); }
                            });
                        }
                    }
                    if let Some(index) = page_target { self.config.current_profile_mut().active_page = index; }
                    if let Some(index) = page_delete {
                        let profile = self.config.current_profile_mut();
                        profile.pages.remove(index);
                        profile.active_page = profile.active_page.min(profile.pages.len() - 1);
                    }
                    if row_button(ui, Icon::Plus, language.tr("Nova página"), false, 30.0).clicked() {
                        let profile = self.config.current_profile_mut();
                        profile.pages.push(Page::new(format!("{} {}", language.tr("Página"), profile.pages.len() + 1)));
                        profile.active_page = profile.pages.len() - 1;
                    }
                    ui.add_space(19.0);
                    ui.horizontal(|ui| {
                        ui.label(muted(language.tr("Brilho da tela"), 12.0));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| { ui.label(title(format!("{}%", self.config.brightness), 12.0)); });
                    });
                    ui.spacing_mut().slider_width = ui.available_width();
                    ui.add(egui::Slider::new(&mut self.config.brightness, 0..=100).show_value(false).trailing_fill(true));
                    ui.add_space(8.0);
                    if row_button(ui, Icon::Power, if self.asleep { language.tr("Acordar D6") } else { language.tr("Apagar D6") }, false, 30.0).clicked() {
                        self.asleep = !self.asleep;
                        let _ = self.device_tx.send(DeviceCommand::Sleep(self.asleep));
                    }
                    ui.add_space((ui.available_height() - 92.0).max(20.0));
                    Frame::new().fill(Color32::from_rgb(52, 51, 53)).corner_radius(11).inner_margin(13).show(ui, |ui| {
                        ui.set_min_width(164.0);
                        ui.label(RichText::new(language.tr("DICA RÁPIDA")).size(10.0).strong().color(ACCENT));
                        ui.add_space(5.0);
                        ui.label(RichText::new(language.tr("Clique para editar. Arraste uma tecla sobre outra para trocar de posição.")).size(11.0).color(Color32::from_rgb(198, 197, 199)));
                    });
                });
            });
    }

    pub(super) fn grid(&mut self, ctx: &egui::Context) {
        let language = self.config.language;
        let profile = self.config.active_profile;
        let page = self.config.current_profile().active_page;
        if self
            .key_drag
            .as_ref()
            .is_some_and(|drag| drag.profile != profile || drag.page != page)
        {
            self.key_drag = None;
        }
        if self.key_move_animation.as_ref().is_some_and(|animation| {
            animation.profile != profile
                || animation.page != page
                || animation.started.elapsed() >= KEY_MOVE_DURATION
        }) {
            self.key_move_animation = None;
        }
        if let Some(drag) = &mut self.key_drag {
            drag.hover = None;
        }
        if self.key_move_animation.is_some() {
            ctx.request_repaint_after(Duration::from_millis(16));
        }
        self.key_rects.fill(None);
        egui::CentralPanel::default()
            .frame(Frame::new().fill(BG).inner_margin(Margin {
                left: 16,
                right: 16,
                top: 4,
                bottom: 0,
            }))
            .show(ctx, |ui| {
                panel(CENTER, 20).show(ui, |ui| {
                    ui.set_min_width((ui.available_width() - 40.0).max(350.0));
                    ui.set_min_height(ui.available_height().max(540.0));
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(section(language.tr("CONFIGURAÇÃO DE TECLAS")));
                            ui.add_space(4.0);
                            ui.label(title(language.tr("Teclas do D6"), 25.0).strong());
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            Frame::new()
                                .fill(Color32::from_rgb(53, 52, 54))
                                .corner_radius(9)
                                .inner_margin(Margin::symmetric(10, 7))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(language.tr("15 teclas · 5 × 3"))
                                                .size(11.0)
                                                .color(Color32::from_rgb(213, 212, 214)),
                                        );
                                        let (rect, _) = ui
                                            .allocate_exact_size(Vec2::splat(14.0), Sense::hover());
                                        draw_icon(ui.painter(), rect, Icon::Grid, ACCENT);
                                    });
                                });
                        });
                    });
                    ui.add_space(42.0);
                    let grid_width = (ui.available_width() - 34.0).clamp(350.0, 708.0);
                    ui.vertical_centered(|ui| {
                        ui.set_width(grid_width);
                        ui.horizontal(|ui| {
                            ui.label(muted(&self.config.current_profile().name, 12.0));
                            ui.label(muted("›", 15.0));
                            ui.label(title(&self.config.current_page().name, 12.0));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let count = self.config.current_profile().pages.len();
                                if icon_button(ui, Icon::ChevronRight, 26.0).clicked() {
                                    let p = self.config.current_profile_mut();
                                    p.active_page = (p.active_page + 1) % count;
                                }
                                if icon_button(ui, Icon::ChevronLeft, 26.0).clicked() {
                                    let p = self.config.current_profile_mut();
                                    p.active_page = (p.active_page + count - 1) % count;
                                }
                            });
                        });
                        ui.add_space(12.0);
                        let card_width = ((grid_width - 56.0) / 5.0).max(58.0);
                        let card_height = (card_width * 1.07).min(156.0);
                        for row in 0..3 {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 14.0;
                                for col in 0..COLUMNS {
                                    let index = row * COLUMNS + col;
                                    self.key_card(
                                        ui,
                                        ctx,
                                        index,
                                        Vec2::new(card_width, card_height),
                                    );
                                }
                            });
                            if row < 2 {
                                ui.add_space(14.0);
                            }
                        }
                        self.complete_key_drag(ctx);
                        self.paint_key_drag(ctx);
                    });
                    ui.add_space((ui.available_height() - 84.0).max(20.0));
                    Frame::new()
                        .fill(Color32::from_rgb(44, 43, 45))
                        .corner_radius(12)
                        .inner_margin(Margin::symmetric(15, 12))
                        .show(ui, |ui| {
                            ui.set_min_width((ui.available_width() - 30.0).max(300.0));
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(title(language.tr("Teste a tecla selecionada"), 12.0));
                                    ui.label(muted(
                                        language.tr(
                                            "Pressione a tecla física no D6 ou use o teste abaixo.",
                                        ),
                                        11.0,
                                    ));
                                });
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.allocate_ui_with_layout(
                                        Vec2::new(122.0, 33.0),
                                        Layout::left_to_right(Align::Center),
                                        |ui| {
                                            if row_button(
                                                ui,
                                                Icon::Play,
                                                language.tr("Testar tecla"),
                                                true,
                                                33.0,
                                            )
                                            .clicked()
                                            {
                                                self.press(self.selected);
                                            }
                                        },
                                    );
                                });
                            });
                        });
                });
            });
    }

    fn key_card(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, index: usize, size: Vec2) {
        let key = self.config.current_page().keys[index].clone();
        let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        self.key_rects[index] = Some(rect);
        if response.drag_started_by(egui::PointerButton::Primary) {
            let pointer = response.interact_pointer_pos().unwrap_or(rect.center());
            self.selected = index;
            self.key_move_animation = None;
            self.key_drag = Some(KeyDrag {
                index,
                profile: self.config.active_profile,
                page: self.config.current_profile().active_page,
                grab_offset: pointer - rect.min,
                size,
                hover: None,
            });
        }
        let pointer = ctx.input(|input| input.pointer.latest_pos());
        if let Some(drag) = &mut self.key_drag
            && index != drag.index
            && pointer.is_some_and(|pos| rect.contains(pos))
        {
            drag.hover = Some(index);
        }
        let dragging_source = self
            .key_drag
            .as_ref()
            .is_some_and(|drag| drag.index == index);
        if dragging_source {
            ui.painter()
                .rect_filled(rect, 13.0, Color32::from_rgb(40, 36, 43));
            ui.painter().rect_stroke(
                rect,
                13.0,
                Stroke::new(1.0, Color32::from_rgb(88, 68, 94)),
                StrokeKind::Inside,
            );
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                self.config.language.tr("Solte sobre outra tecla"),
                FontId::proportional(10.0),
                MUTED,
            );
        } else {
            let paint_rect = self.animated_key_rect(index, rect);
            self.paint_key_visual(ctx, ui.painter(), paint_rect, index, &key);
        }
        if response.clicked() {
            self.selected = index;
        }
        if response.secondary_clicked() {
            self.press(index);
        }
        response
            .on_hover_cursor(egui::CursorIcon::Grab)
            .on_hover_text(
                self.config
                    .language
                    .tr("Clique para editar · arraste para trocar · botão direito para testar"),
            );
    }

    fn animated_key_rect(&self, index: usize, rect: egui::Rect) -> egui::Rect {
        let Some(animation) = &self.key_move_animation else {
            return rect;
        };
        let Some((_, from)) = animation.from.iter().find(|(slot, _)| *slot == index) else {
            return rect;
        };
        let progress = (animation.started.elapsed().as_secs_f32()
            / KEY_MOVE_DURATION.as_secs_f32())
        .clamp(0.0, 1.0);
        let eased = 1.0 - (1.0 - progress).powi(3);
        rect.translate((from.center() - rect.center()) * (1.0 - eased))
    }

    fn complete_key_drag(&mut self, ctx: &egui::Context) {
        if !ctx.input(|input| input.pointer.button_released(egui::PointerButton::Primary)) {
            return;
        }
        let Some(drag) = self.key_drag.take() else {
            return;
        };
        let Some(source_rect) = self.key_rects[drag.index] else {
            return;
        };
        let pointer = ctx
            .input(|input| input.pointer.latest_pos())
            .unwrap_or(source_rect.center());
        let floating_rect = egui::Rect::from_min_size(pointer - drag.grab_offset, drag.size);
        let mut from = vec![(drag.index, floating_rect)];
        if let Some(target) = drag.hover.filter(|target| *target != drag.index)
            && let Some(target_rect) = self.key_rects[target]
        {
            self.config.current_page_mut().keys.swap(drag.index, target);
            if self.selected == drag.index {
                self.selected = target;
            } else if self.selected == target {
                self.selected = drag.index;
            }
            from = vec![(drag.index, target_rect), (target, floating_rect)];
        }
        self.key_move_animation = Some(KeyMoveAnimation {
            profile: drag.profile,
            page: drag.page,
            started: Instant::now(),
            from,
        });
        ctx.request_repaint();
    }

    fn paint_key_drag(&mut self, ctx: &egui::Context) {
        let Some(drag) = &self.key_drag else { return };
        let Some(pointer) = ctx.input(|input| input.pointer.latest_pos()) else {
            return;
        };
        let index = drag.index;
        let rect = egui::Rect::from_min_size(pointer - drag.grab_offset, drag.size);
        let key = self.config.current_page().keys[index].clone();
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("dragged-key"),
        ));
        painter.rect_filled(
            rect.translate(egui::vec2(0.0, 8.0)),
            13.0,
            Color32::from_black_alpha(95),
        );
        self.paint_key_visual(ctx, &painter, rect, index, &key);
        ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
        ctx.request_repaint_after(Duration::from_millis(16));
    }

    fn paint_key_visual(
        &mut self,
        ctx: &egui::Context,
        painter: &egui::Painter,
        rect: egui::Rect,
        index: usize,
        key: &KeyConfig,
    ) {
        let selected = self.selected == index;
        let drop_target = self
            .key_drag
            .as_ref()
            .is_some_and(|drag| drag.hover == Some(index));
        painter.rect_filled(
            rect,
            13.0,
            if selected {
                Color32::from_rgb(66, 49, 73)
            } else {
                TILE
            },
        );
        if selected || drop_target {
            painter.rect_stroke(
                rect,
                13.0,
                Stroke::new(if drop_target { 2.5 } else { 1.5 }, ACCENT),
                StrokeKind::Inside,
            );
        }
        painter.text(
            rect.left_top() + egui::vec2(14.0, 14.0),
            Align2::LEFT_TOP,
            format!("{:02}", index + 1),
            FontId::proportional(11.0),
            if selected || drop_target {
                ACCENT
            } else {
                MUTED
            },
        );
        if selected {
            painter.circle_filled(rect.right_top() + egui::vec2(-17.0, 20.0), 3.0, ACCENT);
        }
        let icon_size = (rect.width() * 0.42).clamp(28.0, 54.0);
        let icon_rect = egui::Rect::from_center_size(
            rect.center() - egui::vec2(0.0, 6.0),
            Vec2::splat(icon_size),
        );
        painter.rect_filled(
            icon_rect,
            11.0,
            if selected {
                Color32::from_rgb(104, 71, 114)
            } else {
                Color32::from_rgb(57, 56, 58)
            },
        );
        if let Some(texture) = self.texture_for(ctx, &key.image) {
            painter.image(
                texture,
                icon_rect,
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        } else if key.title.is_empty() {
            painter.text(
                icon_rect.center(),
                Align2::CENTER_CENTER,
                "+",
                FontId::proportional(24.0),
                if selected { ACCENT } else { MUTED },
            );
        } else {
            painter.rect_filled(
                icon_rect.shrink(3.0),
                8.0,
                Color32::from_rgb(key.color[0], key.color[1], key.color[2]),
            );
        }
        let state = if !key.title.is_empty() {
            key.title.as_str()
        } else if matches!(key.action, Action::None) {
            if selected {
                self.config.language.tr("Selecionada")
            } else {
                self.config.language.tr("Sem ação")
            }
        } else {
            self.config.language.tr(Action::KINDS[key.action.kind()])
        };
        let label: String = state.chars().take(18).collect();
        painter.text(
            rect.center_bottom() - egui::vec2(0.0, 16.0),
            Align2::CENTER_BOTTOM,
            label,
            FontId::proportional(11.0),
            if selected { TEXT } else { MUTED },
        );
    }

    pub(super) fn inspector(&mut self, ctx: &egui::Context) {
        let language = self.config.language;
        let profiles: Vec<String> = self
            .config
            .profiles
            .iter()
            .map(|p| p.name.clone())
            .collect();
        let pages: Vec<String> = self
            .config
            .current_profile()
            .pages
            .iter()
            .map(|p| p.name.clone())
            .collect();
        let preview = self.config.current_page().keys[self.selected].clone();
        let texture = self.texture_for(ctx, &preview.image);
        egui::SidePanel::right("inspector")
            .exact_width(304.0)
            .resizable(false)
            .show_separator_line(false)
            .frame(Frame::new().fill(BG).inner_margin(Margin { left: 0, right: 16, top: 4, bottom: 0 }))
            .show(ctx, |ui| {
                panel(PANEL, 18).show(ui, |ui| {
                    ui.set_min_width(252.0);
                    ui.set_min_height(ui.available_height().max(530.0));
                    egui::ScrollArea::vertical().id_salt("inspector-scroll").show(ui, |ui| {
                        ui.label(section(language.tr("EDITOR DE TECLA")));
                        ui.add_space(5.0);
                        ui.label(title(format!("{} {:02}", if language == Language::English { "Key" } else { "Tecla" }, self.selected + 1), 23.0).strong());
                        ui.add_space(14.0);
                        Frame::new().fill(Color32::from_rgb(50, 42, 56)).corner_radius(12).inner_margin(11).show(ui, |ui| {
                            ui.set_min_width(240.0);
                            ui.horizontal(|ui| {
                                let (rect, _) = ui.allocate_exact_size(Vec2::splat(50.0), Sense::hover());
                                ui.painter().rect_filled(rect, 9.0, Color32::from_rgb(103, 72, 116));
                                if let Some(texture) = texture {
                                    ui.painter().image(texture, rect, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), Color32::WHITE);
                                } else {
                                    ui.painter().text(rect.center(), Align2::CENTER_CENTER, "+", FontId::proportional(24.0), ACCENT);
                                }
                                ui.add_space(6.0);
                                ui.vertical(|ui| {
                                    ui.label(title(if preview.title.is_empty() { language.tr("Pronta para configurar") } else { &preview.title }, 11.0));
                                    ui.label(muted(if matches!(preview.action, Action::None) { language.tr("Adicione uma ação abaixo") } else { language.tr(Action::KINDS[preview.action.kind()]) }, 10.0));
                                });
                            });
                        });
                        ui.add_space(16.0);
                        let key: &mut KeyConfig = &mut self.config.current_page_mut().keys[self.selected];
                        ui.label(title(language.tr("Título"), 12.0));
                        ui.add_sized([ui.available_width(), 38.0], egui::TextEdit::singleline(&mut key.title).hint_text(language.tr("Nome da tecla")).background_color(FIELD));
                        ui.add_space(11.0);
                        ui.label(title(language.tr("Cor da tecla"), 12.0));
                        Frame::new().fill(FIELD).corner_radius(8).inner_margin(Margin::symmetric(10, 6)).show(ui, |ui| {
                            ui.set_min_width(242.0);
                            ui.horizontal(|ui| {
                                ui.color_edit_button_srgb(&mut key.color);
                                ui.label(muted(format!("#{:02X}{:02X}{:02X}", key.color[0], key.color[1], key.color[2]), 11.0));
                            });
                        });
                        ui.add_space(11.0);
                        ui.label(title(language.tr("Imagem da tecla"), 12.0));
                        let (image_rect, image_response) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 78.0), Sense::click());
                        ui.painter().rect_filled(image_rect, 9.0, if image_response.hovered() { Color32::from_rgb(59, 55, 61) } else { Color32::from_rgb(44, 43, 45) });
                        let image_icon = egui::Rect::from_center_size(image_rect.center_top() + egui::vec2(0.0, 22.0), Vec2::splat(20.0));
                        draw_icon(ui.painter(), image_icon, Icon::Image, ACCENT);
                        ui.painter().text(image_rect.center_top() + egui::vec2(0.0, 42.0), Align2::CENTER_CENTER, if key.image.is_empty() { language.tr("Escolher imagem") } else { language.tr("Trocar imagem") }, FontId::proportional(11.0), Color32::from_rgb(231, 230, 232));
                        ui.painter().text(image_rect.center_top() + egui::vec2(0.0, 62.0), Align2::CENTER_CENTER, "PNG · JPEG · WebP", FontId::proportional(10.0), MUTED);
                        if image_response.clicked() && let Some(path) = image_picker(language) { key.image = path; }
                        if !key.image.is_empty() {
                            let name = std::path::Path::new(&key.image).file_name().and_then(|name| name.to_str()).unwrap_or(language.tr("Imagem selecionada"));
                            ui.label(muted(name, 10.0)).on_hover_text(&key.image);
                        }
                        ui.add_space(11.0);
                        ui.label(title(language.tr("Ação ao pressionar"), 12.0));
                        edit_action(ui, &mut key.action, &profiles, &pages, 0, language);
                        ui.add_space(14.0);
                        if row_button(ui, Icon::Trash, language.tr("Limpar tecla"), false, 28.0).clicked() { *key = KeyConfig::default(); }
                        ui.add_space((ui.available_height() - 110.0).max(25.0));
                        Frame::new().fill(Color32::from_rgb(46, 45, 47)).corner_radius(10).inner_margin(13).show(ui, |ui| {
                            ui.set_min_width(236.0);
                            ui.horizontal(|ui| {
                                let (rect, _) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
                                draw_icon(ui.painter(), rect, Icon::Info, ACCENT);
                                ui.label(RichText::new(language.tr("Permissão do macOS")).size(11.0).color(Color32::from_rgb(232, 231, 233)));
                            });
                            ui.add_space(4.0);
                            ui.label(RichText::new(language.tr("Atalhos e texto exigem Acessibilidade. Comandos e URLs só são executados ao pressionar uma tecla configurada.")).size(10.0).color(Color32::from_rgb(176, 175, 177)));
                        });
                    });
                });
            });
    }
}
