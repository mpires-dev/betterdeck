use crate::i18n::Language;
use anyhow::{Context, Result};
use eframe::egui;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
use std::sync::mpsc::{self, Receiver};
use tray_icon::{
    Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuItem},
};

const SHOW_ID: &str = "betterdeck-show";
const QUIT_ID: &str = "betterdeck-quit";

#[derive(Clone, Copy)]
pub enum StatusCommand {
    Show,
    Quit,
}

pub struct StatusBar {
    _icon: TrayIcon,
    show_item: MenuItem,
    quit_item: MenuItem,
    language: Language,
    commands: Receiver<StatusCommand>,
}

impl StatusBar {
    pub fn new(ctx: &egui::Context, language: Language) -> Result<Self> {
        let menu = Menu::new();
        let show_item = MenuItem::with_id(SHOW_ID, language.tr("Mostrar BetterDeck"), true, None);
        let quit_item = MenuItem::with_id(
            QUIT_ID,
            if language == Language::English {
                "Quit"
            } else {
                "Sair"
            },
            true,
            None,
        );
        menu.append(&show_item)?;
        menu.append(&quit_item)?;

        let icon = TrayIconBuilder::new()
            .with_icon_templated(brand_icon()?)
            .with_tooltip("BetterDeck")
            .with_autosave_name("dev.betterdeck.app.status")
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .with_menu_on_right_click(true)
            .build()
            .context("criar ícone da barra de menus")?;

        let (tx, commands) = mpsc::channel();
        let menu_tx = tx.clone();
        let menu_ctx = ctx.clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            let command = match event.id.0.as_str() {
                SHOW_ID => Some(StatusCommand::Show),
                QUIT_ID => Some(StatusCommand::Quit),
                _ => None,
            };
            if let Some(command) = command {
                let _ = menu_tx.send(command);
                menu_ctx.request_repaint();
            }
        }));
        let tray_ctx = ctx.clone();
        TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                let _ = tx.send(StatusCommand::Show);
                tray_ctx.request_repaint();
            }
        }));

        Ok(Self {
            _icon: icon,
            show_item,
            quit_item,
            language,
            commands,
        })
    }

    pub fn set_language(&mut self, language: Language) {
        if self.language != language {
            self.show_item.set_text(language.tr("Mostrar BetterDeck"));
            self.quit_item.set_text(if language == Language::English {
                "Quit"
            } else {
                "Sair"
            });
            self.language = language;
        }
    }

    pub fn commands(&self) -> impl Iterator<Item = StatusCommand> + '_ {
        self.commands.try_iter()
    }
}

pub fn set_dock_visible(visible: bool) -> Result<()> {
    let mtm = MainThreadMarker::new().context("AppKit exige a thread principal")?;
    let app = NSApplication::sharedApplication(mtm);
    let policy = if visible {
        NSApplicationActivationPolicy::Regular
    } else {
        NSApplicationActivationPolicy::Accessory
    };
    if !app.setActivationPolicy(policy) {
        anyhow::bail!("macOS recusou alterar a presença no Dock");
    }
    if visible {
        app.activate();
    }
    Ok(())
}

fn brand_icon() -> Result<Icon> {
    let logo = image::load_from_memory(include_bytes!("../assets/betterdeck-logo.png"))?;
    let mark =
        logo.crop_imm(0, 0, 80, 80)
            .resize_exact(20, 20, image::imageops::FilterType::Lanczos3);
    Icon::from_rgba(mark.to_rgba8().into_raw(), 20, 20).context("gerar ícone da barra de menus")
}
