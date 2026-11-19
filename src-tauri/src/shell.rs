//! Janela e bandeja do sistema:
//! - minimizar esconde a janela e deixa só o ícone na bandeja (perto do relógio);
//! - clique no ícone reabre; o menu do ícone tem "Abrir" e "Sair";
//! - vigia da tela: a interface manda um sinal de vida por segundo. Se a
//!   janela está em foco e o sinal some, a tela é recarregada e o fato vai
//!   para o log. O núcleo (centralização e efeitos) não é afetado.

use crate::core::Core;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Runtime, WebviewWindow};

const TRAY_ID: &str = "main";
/// A janela precisa estar em foco há este tempo antes de o vigia agir.
const FOCUS_GRACE: Duration = Duration::from_secs(3);
/// Sem sinal de vida por este tempo, com a janela em foco: recarregar.
const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(5);
/// Intervalo mínimo entre dois recarregamentos automáticos.
const RELOAD_COOLDOWN: Duration = Duration::from_secs(20);

/// Estado do vigia da tela.
pub struct UiWatch {
    pub last_heartbeat: Mutex<Option<Instant>>,
    pub focused_since: Mutex<Option<Instant>>,
    last_reload: Mutex<Option<Instant>>,
}

impl UiWatch {
    pub fn new() -> Self {
        Self { last_heartbeat: Mutex::new(None), focused_since: Mutex::new(None), last_reload: Mutex::new(None) }
    }
}

fn main_window<R: Runtime>(app: &AppHandle<R>) -> Option<WebviewWindow<R>> {
    app.get_webview_window("main")
}

pub fn show_main<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = main_window(app) {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// Para a centralização e os efeitos, devolvendo o volante ao firmware.
pub fn release_wheel(core: &Core) {
    if let Ok(c) = core.centering.lock() {
        if let Some(h) = c.as_ref() {
            h.stop();
        }
    }
    if let Some(dev) = core.current_device() {
        let _ = dev.constant_stop();
        let _ = dev.periodic_stop();
    }
}

fn build_menu<R: Runtime>(app: &AppHandle<R>, open: &str, quit: &str) -> tauri::Result<Menu<R>> {
    let open = MenuItem::with_id(app, "open", open, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", quit, true, None::<&str>)?;
    Menu::with_items(app, &[&open, &quit])
}

pub fn create_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let menu = build_menu(app, "Abrir", "Sair")?;
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("PXN V99 Control Center")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main(app),
            "quit" => {
                if let Some(core) = app.try_state::<Arc<Core>>() {
                    release_wheel(&core);
                    // Dá tempo à malha de força de parar antes de encerrar.
                    thread::sleep(Duration::from_millis(150));
                }
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

/// Traduz o menu do ícone para o idioma escolhido na interface.
pub fn set_tray_labels<R: Runtime>(app: &AppHandle<R>, open: &str, quit: &str) -> tauri::Result<()> {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        tray.set_menu(Some(build_menu(app, open, quit)?))?;
    }
    Ok(())
}

/// Chamado em cada evento de janela.
pub fn on_window_event<R: Runtime>(window: &tauri::Window<R>, event: &tauri::WindowEvent) {
    let Some(core) = window.try_state::<Arc<Core>>() else { return };
    match event {
        tauri::WindowEvent::Focused(focused) => {
            core.ui_focused.store(*focused, Ordering::Relaxed);
            if let Ok(mut f) = core.ui_watch.focused_since.lock() {
                *f = if *focused { Some(Instant::now()) } else { None };
            }
            if *focused {
                // Força um redesenho completo ao voltar para a janela.
                let _ = window.emit("ui-wake", ());
            }
        }
        tauri::WindowEvent::Resized(_) => {
            if window.is_minimized().unwrap_or(false) {
                let _ = window.hide();
            }
        }
        tauri::WindowEvent::CloseRequested { .. } => release_wheel(&core),
        _ => {}
    }
}

/// Vigia da tela (thread de fundo).
pub fn spawn_ui_watchdog(app: AppHandle, core: Arc<Core>) {
    let _ = thread::Builder::new().name("pxn-ui-watch".into()).spawn(move || loop {
        thread::sleep(Duration::from_secs(1));
        let Some(w) = main_window(&app) else { continue };
        let visible = w.is_visible().unwrap_or(false) && !w.is_minimized().unwrap_or(true);
        let focused_long = core
            .ui_watch
            .focused_since
            .lock()
            .ok()
            .and_then(|f| *f)
            .map(|t| t.elapsed() >= FOCUS_GRACE)
            .unwrap_or(false);
        if !visible || !focused_long {
            continue;
        }
        let stale = core
            .ui_watch
            .last_heartbeat
            .lock()
            .ok()
            .and_then(|h| *h)
            .map(|t| t.elapsed())
            .filter(|e| *e >= HEARTBEAT_TIMEOUT);
        let cooled = core.ui_watch.last_reload.lock().ok().and_then(|r| *r).map(|t| t.elapsed() >= RELOAD_COOLDOWN).unwrap_or(true);
        if let (Some(silence), true) = (stale, cooled) {
            core.log("warn", "ui", format!("Tela sem resposta há {:.0} s com a janela em foco; recarregando", silence.as_secs_f32()));
            if let Ok(mut r) = core.ui_watch.last_reload.lock() {
                *r = Some(Instant::now());
            }
            if let Ok(mut h) = core.ui_watch.last_heartbeat.lock() {
                *h = Some(Instant::now());
            }
            if let Err(e) = w.reload() {
                core.log("error", "ui", format!("Falha ao recarregar a tela: {e}"));
            }
        }
    });
}
