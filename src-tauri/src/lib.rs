//! PXN V99 Control Center: núcleo nativo.
//!
//! A interface (WebView) só fala com o hardware através dos comandos abaixo,
//! que validam cada pedido antes de repassá-lo à camada `hw`.

mod core;
mod game;
mod shell;
mod hw;
mod model;

use crate::core::Core;
use crate::hw::device::DeviceControl;
use crate::model::{DeviceStatus, LogEntry, Telemetry};
use std::sync::Arc;
use tauri::{Manager, State};

type CoreState<'a> = State<'a, Arc<Core>>;

#[tauri::command(async)]
fn get_status(core: CoreState) -> DeviceStatus {
    core.status_snapshot()
}

#[tauri::command(async)]
fn get_telemetry(core: CoreState) -> Telemetry {
    core.telemetry.lock().map(|t| t.clone()).unwrap_or_default()
}

#[tauri::command(async)]
fn get_game(core: CoreState) -> game::GameStatus {
    core.game.lock().map(|g| g.clone()).unwrap_or_default()
}

/// Atualiza a configuração dos efeitos de jogo. Vale na hora para a malha.
#[tauri::command(async)]
fn effects_update(core: CoreState, params: hw::effects::EffectsParams, commit: bool) -> Result<(), String> {
    let params = params.validated()?;
    let previous = core.effects.lock().map(|e| *e).unwrap_or_default();
    if let Ok(mut e) = core.effects.lock() {
        *e = params;
    }
    if commit || previous.enabled != params.enabled {
        core.log(
            "info",
            "game",
            format!(
                "Efeitos de jogo {}: geral {:.0}%, motor {:.0}%, embreagem {:.0}%, terreno {:.0}%, impactos {:.0}%, {} cilindros",
                if params.enabled { "ligados" } else { "desligados" },
                params.master,
                params.engine,
                params.clutch,
                params.terrain,
                params.impacts,
                params.cylinders
            ),
        );
    }
    Ok(())
}

/// Grava os próximos 30 s da telemetria crua do jogo em arquivo, para
/// diagnóstico. Devolve o caminho do arquivo.
#[tauri::command(async)]
fn game_record(core: CoreState) -> Result<String, String> {
    game::start_recording(&core, 30)
}

#[tauri::command(async)]
fn get_log(core: CoreState) -> Vec<LogEntry> {
    core.log_snapshot()
}

/// Envia o Device Gain PID (ganho global de Force Feedback), 0..=100 %.
/// `commit` indica o fim de um ajuste (soltar o slider) e gera registro no log.
#[tauri::command(async)]
fn set_device_gain(core: CoreState, percent: u8, commit: bool) -> Result<u8, String> {
    if percent > 100 {
        return Err("Valor fora do intervalo 0..100".into());
    }
    let dev = core.current_device().ok_or("Volante não conectado em DirectInput")?;
    match dev.set_device_gain(percent) {
        Ok(()) => {
            if let Ok(mut s) = core.status.lock() {
                s.applied_gain_percent = Some(percent);
            }
            if commit {
                core.log("info", "ffb", format!("Device Gain enviado: {percent}%"));
                core.emit_status();
            }
            Ok(percent)
        }
        Err(e) => {
            if let Ok(mut s) = core.status.lock() {
                s.error_count += 1;
                s.last_error = Some(e.clone());
            }
            core.log("error", "ffb", format!("Falha ao enviar Device Gain {percent}%: {e}"));
            core.emit_status();
            Err(e)
        }
    }
}

fn record_error(core: &Core, what: &str, e: &str) {
    if let Ok(mut s) = core.status.lock() {
        s.error_count += 1;
        s.last_error = Some(e.to_string());
    }
    core.log("error", "ffb", format!("{what}: {e}"));
    core.emit_status();
}

/// Inicia a centralização controlada pelo app (malha de força constante).
/// Sem `polarity`, o app calibra o sentido do motor com dois pulsos fracos;
/// a interface pede confirmação antes. Devolve a polaridade em uso.
#[tauri::command(async)]
fn centering_start(core: CoreState, params: hw::centering::Params, polarity: Option<i8>) -> Result<i8, String> {
    let dev = core.current_device().ok_or("Volante não conectado em DirectInput")?;
    if let Ok(mut c) = core.centering.lock() {
        if let Some(h) = c.take() {
            h.stop();
        }
    }
    let calibrated = polarity.is_none();
    match hw::centering::start(core.inner().clone(), dev, params, polarity) {
        Ok((polarity, handle)) => {
            if let Ok(mut c) = core.centering.lock() {
                *c = Some(handle);
            }
            if let Ok(mut s) = core.status.lock() {
                s.centering_active = true;
            }
            core.log(
                "info",
                "ffb",
                format!(
                    "Centralização do app iniciada: força {:.0}%, força máxima a {:.0}°, amortecimento {:.0}%, zona morta {:.1}°, polaridade {polarity}{}",
                    params.strength,
                    params.full_angle,
                    params.damping,
                    params.deadzone,
                    if calibrated { " (calibrada agora)" } else { "" }
                ),
            );
            core.emit_status();
            Ok(polarity)
        }
        Err(e) => {
            record_error(&core, "Falha ao iniciar a centralização", &e);
            Err(e)
        }
    }
}

#[tauri::command(async)]
fn centering_update(core: CoreState, params: hw::centering::Params, commit: bool) -> Result<(), String> {
    let params = params.validated()?;
    let guard = core.centering.lock().map_err(|_| "estado interno corrompido")?;
    let handle = guard.as_ref().ok_or("Centralização não está ativa")?;
    handle.update(params);
    drop(guard);
    if commit {
        core.log(
            "info",
            "ffb",
            format!(
                "Centralização ajustada: força {:.0}%, força máxima a {:.0}°, amortecimento {:.0}%, zona morta {:.1}°",
                params.strength, params.full_angle, params.damping, params.deadzone
            ),
        );
    }
    Ok(())
}

#[tauri::command(async)]
fn centering_stop(core: CoreState) -> Result<(), String> {
    if let Ok(c) = core.centering.lock() {
        if let Some(h) = c.as_ref() {
            h.stop();
        }
    }
    Ok(())
}

/// Vibração senoidal fraca e curta para verificar se o firmware executa
/// efeitos PID enviados diretamente. Limites validados na camada de hardware.
#[tauri::command(async)]
fn vibration_test(core: CoreState) -> Result<(), String> {
    const MAGNITUDE: i32 = 25;
    const DURATION_MS: i32 = 1000;
    let dev = core.current_device().ok_or("Volante não conectado em DirectInput")?;
    match dev.vibration_start(MAGNITUDE, DURATION_MS) {
        Ok(block) => {
            core.log("info", "ffb", format!("Teste de vibração iniciado no bloco {block}: {MAGNITUDE}% por {DURATION_MS} ms"));
            std::thread::sleep(std::time::Duration::from_millis(DURATION_MS as u64 + 300));
            let _ = dev.release_block(block);
            core.log("info", "ffb", "Teste de vibração encerrado");
            Ok(())
        }
        Err(e) => {
            record_error(&core, "Falha no teste de vibração", &e);
            Err(e)
        }
    }
}

/// Envia um comando PID Device Control: enable | disable | stop_all | reset.
#[tauri::command(async)]
fn device_control(core: CoreState, command: String) -> Result<(), String> {
    let cmd = match command.as_str() {
        "enable" => DeviceControl::EnableActuators,
        "disable" => DeviceControl::DisableActuators,
        "stop_all" => DeviceControl::StopAllEffects,
        "reset" => DeviceControl::Reset,
        _ => return Err("Comando desconhecido".into()),
    };
    let dev = core.current_device().ok_or("Volante não conectado em DirectInput")?;
    match dev.device_control(cmd) {
        Ok(()) => {
            core.log("info", "ffb", format!("Device Control enviado: {cmd:?}"));
            core.emit_status();
            Ok(())
        }
        Err(e) => {
            record_error(&core, "Falha no Device Control", &e);
            Err(e)
        }
    }
}

/// Sinal de vida da interface (a cada segundo).
#[tauri::command]
fn ui_heartbeat(core: CoreState) {
    if let Ok(mut h) = core.ui_watch.last_heartbeat.lock() {
        *h = Some(std::time::Instant::now());
    }
}

/// Erro de interface capturado pela tela: vai para o log em arquivo.
#[tauri::command]
fn ui_error(core: CoreState, message: String) {
    let message: String = message.chars().take(2000).collect();
    core.log("error", "ui", message);
}

/// Textos do menu do ícone da bandeja no idioma da interface.
#[tauri::command]
fn tray_labels(app: tauri::AppHandle, open: String, quit: String) -> Result<(), String> {
    shell::set_tray_labels(&app, &open, &quit).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let core = Arc::new(Core::new(app.handle().clone()));
            app.manage(core.clone());
            hw::worker::spawn(core.clone());
            game::spawn(core.clone());
            shell::create_tray(app.handle())?;
            shell::spawn_ui_watchdog(app.handle().clone(), core);
            Ok(())
        })
        // Minimizar vai para a bandeja; fechar devolve o volante ao firmware.
        .on_window_event(shell::on_window_event)
        .invoke_handler(tauri::generate_handler![
            get_status,
            get_telemetry,
            get_log,
            get_game,
            effects_update,
            game_record,
            set_device_gain,
            vibration_test,
            centering_start,
            centering_update,
            centering_stop,
            device_control,
            ui_heartbeat,
            ui_error,
            tray_labels
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o PXN V99 Control Center");
}
