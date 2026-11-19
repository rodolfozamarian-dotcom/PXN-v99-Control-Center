//! Centralização controlada pelo app.
//!
//! Uma mola PID do firmware é proporcional ao ângulo sobre o curso inteiro, e
//! por isso fica fraca perto do centro. Aqui o próprio app fecha a malha: lê a
//! posição do volante e comanda um efeito PID de força constante, o que
//! permite escolher a força máxima, o ângulo em que ela é atingida, o
//! amortecimento e a zona morta.
//!
//! Segurança: a força nunca passa de `strength`; a malha para sozinha se o
//! volante desconectar, se a escrita falhar repetidamente ou quando o app fecha.

use super::device::Device;
use super::ffi::{timeBeginPeriod, timeEndPeriod};
use super::effects::{Synth, STALE_MS};
use crate::core::Core;
use serde::Deserialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tauri::Emitter;

const LOOP_INTERVAL: Duration = Duration::from_millis(4);
/// O efeito constante tem duração de 30 s e é reiniciado com folga.
const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(12);
/// Variação mínima (em 1/10000 da força) para reenviar o comando.
const MIN_DELTA: i32 = 40;
const MAX_WRITE_ERRORS: u32 = 5;
/// Atualização da senoide do motor (amplitude e frequência).
const ENGINE_UPDATE_INTERVAL: Duration = Duration::from_millis(100);
/// Cada Set Periodic pode reiniciar a fase da senoide no firmware: trocar o
/// período só quando a rotação muda de verdade, e não a cada oscilação.
const ENGINE_PERIOD_HOLD: Duration = Duration::from_millis(400);
const LEVELS_EMIT_INTERVAL: Duration = Duration::from_millis(66);
/// Velocidade (°/s) em que o amortecimento de 100 % gera força total.
const DAMPING_REF_SPEED: f32 = 720.0;
const USAGE_X: u16 = 0x30;

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Params {
    /// Força máxima, 0..=100 %.
    pub strength: f32,
    /// Ângulo (graus a partir do centro) em que a força máxima é atingida.
    pub full_angle: f32,
    /// Amortecimento, 0..=100 %.
    pub damping: f32,
    /// Zona morta em graus em torno do centro.
    pub deadzone: f32,
    /// Metade do curso do volante em graus (450 para 900°).
    pub half_range: f32,
}

impl Params {
    pub fn validated(self) -> Result<Self, String> {
        let ok = self.strength.is_finite()
            && (0.0..=100.0).contains(&self.strength)
            && (1.0..=450.0).contains(&self.full_angle)
            && (0.0..=100.0).contains(&self.damping)
            && (0.0..=30.0).contains(&self.deadzone)
            && (45.0..=1080.0).contains(&self.half_range);
        if ok {
            Ok(self)
        } else {
            Err("Parâmetros de centralização fora do intervalo permitido".into())
        }
    }
}

pub struct Handle {
    stop: Arc<AtomicBool>,
    params: Arc<Mutex<Params>>,
}

impl Handle {
    pub fn update(&self, p: Params) {
        if let Ok(mut cur) = self.params.lock() {
            *cur = p;
        }
    }
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// Ângulo atual do volante em graus e o instante (ms) do último relatório.
fn read_angle(core: &Core, dev: &Device, half_range: f32) -> Option<(f32, f64)> {
    let idx = dev.info.axes.iter().position(|a| a.usage == USAGE_X)?;
    let axis = &dev.info.axes[idx];
    let t = core.telemetry.lock().ok()?;
    let raw = *t.axes.get(idx)?;
    let span = (axis.max - axis.min) as f32;
    if span <= 0.0 {
        return None;
    }
    let norm = ((raw - axis.min) as f32 / span) * 2.0 - 1.0;
    Some((norm * half_range, t.t_ms))
}

/// Descobre o sinal do motor com dois pulsos fracos e curtos: devolve +1 se
/// magnitude positiva aumenta o ângulo, -1 caso contrário. Move o volante
/// alguns graus; só é chamado após confirmação do usuário.
fn calibrate(core: &Core, dev: &Device, half_range: f32) -> Result<i8, String> {
    let (start, _) = read_angle(core, dev, half_range).ok_or("Sem leitura de posição: gire um pouco o volante e tente de novo")?;
    for magnitude in [1500, 3000] {
        dev.constant_force(magnitude)?;
        thread::sleep(Duration::from_millis(350));
        dev.constant_force(0)?;
        thread::sleep(Duration::from_millis(80));
        let (now, _) = read_angle(core, dev, half_range).ok_or("Leitura de posição perdida durante a calibração")?;
        let delta = now - start;
        if delta.abs() >= 1.5 {
            return Ok(if delta > 0.0 { 1 } else { -1 });
        }
    }
    Err("O volante não se moveu com o pulso de teste. Solte o volante e tente de novo; se persistir, o firmware não aceita força constante".into())
}

/// Inicia a malha. Se `polarity` for None, calibra o sentido primeiro.
/// Devolve a polaridade em uso.
pub fn start(core: Arc<Core>, dev: Arc<Device>, params: Params, polarity: Option<i8>) -> Result<(i8, Handle), String> {
    let params = params.validated()?;
    dev.constant_start()?;
    let polarity = match polarity {
        Some(p) if p == 1 || p == -1 => p,
        _ => match calibrate(&core, &dev, params.half_range) {
            Ok(p) => p,
            Err(e) => {
                let _ = dev.constant_stop();
                return Err(e);
            }
        },
    };
    if read_angle(&core, &dev, params.half_range).is_none() {
        let _ = dev.constant_stop();
        return Err("Sem leitura de posição: gire um pouco o volante e tente de novo".into());
    }

    let stop = Arc::new(AtomicBool::new(false));
    let shared = Arc::new(Mutex::new(params));
    let handle = Handle { stop: stop.clone(), params: shared.clone() };
    thread::Builder::new()
        .name("pxn-centering".into())
        .spawn(move || run(core, dev, shared, stop, polarity))
        .map_err(|e| format!("não foi possível iniciar a malha: {e}"))?;
    Ok((polarity, handle))
}

fn run(core: Arc<Core>, dev: Arc<Device>, params: Arc<Mutex<Params>>, stop: Arc<AtomicBool>, polarity: i8) {
    unsafe {
        timeBeginPeriod(1);
    }
    let mut last_sent = 0i32;
    let mut errors = 0u32;
    let mut prev: Option<(f32, f64)> = None;
    let mut velocity = 0.0f32;
    let mut last_motion = Instant::now();
    let mut last_keepalive = Instant::now();
    let mut synth = Synth::default();
    let mut last_tick = Instant::now();
    let mut engine_started = false;
    let mut engine_failed = false;
    let mut last_engine = (0i32, 0i32);
    let mut last_engine_send = Instant::now();
    let mut last_period_change = Instant::now();
    let mut last_levels_emit = Instant::now();
    let reason: &str;

    loop {
        if stop.load(Ordering::Relaxed) {
            reason = "parada pelo usuário";
            break;
        }
        match core.current_device() {
            Some(cur) if Arc::ptr_eq(&cur, &dev) => {}
            _ => {
                reason = "volante desconectado";
                break;
            }
        }
        let p = match params.lock() {
            Ok(p) => *p,
            Err(_) => {
                reason = "estado interno corrompido";
                break;
            }
        };

        if let Some((angle, t_ms)) = read_angle(&core, &dev, p.half_range) {
            match prev {
                Some((pa, pt)) if t_ms > pt => {
                    let inst = (angle - pa) / ((t_ms - pt) as f32 / 1000.0);
                    velocity = velocity * 0.7 + inst * 0.3;
                    last_motion = Instant::now();
                    prev = Some((angle, t_ms));
                }
                Some(_) => {
                    // Sem relatório novo o volante está parado: a velocidade decai.
                    if last_motion.elapsed() > Duration::from_millis(40) {
                        velocity *= 0.8;
                    }
                }
                None => prev = Some((angle, t_ms)),
            }

            // ---- Efeitos de jogo (vibração) ----
            let fx_params = core.effects.lock().map(|e| *e).unwrap_or_default();
            let (frame, fresh) = core
                .game
                .lock()
                .map(|g| (g.last.clone(), g.last_rx.map(|t| t.elapsed().as_millis() < STALE_MS).unwrap_or(false)))
                .unwrap_or((None, false));
            let dt = last_tick.elapsed().as_secs_f32().clamp(0.0005, 0.05);
            last_tick = Instant::now();
            let fx = synth.step(&fx_params, frame.as_ref(), fresh, dt);

            if fx_params.enabled && !engine_started && !engine_failed {
                match dev.periodic_start() {
                    Ok(block) => {
                        engine_started = true;
                        core.log("info", "ffb", format!("Vibração do motor pronta no bloco {block}"));
                    }
                    Err(e) => {
                        engine_failed = true;
                        core.log("error", "ffb", format!("Não foi possível criar a vibração do motor: {e}"));
                    }
                }
            } else if !fx_params.enabled && engine_started {
                let _ = dev.periodic_stop();
                engine_started = false;
                last_engine = (0, 0);
            }
            if engine_started && last_engine_send.elapsed() >= ENGINE_UPDATE_INTERVAL {
                let mag = (fx.engine_mag * 10000.0) as i32;
                let wanted = (1000.0 / fx.engine_hz).round() as i32;
                let period_moved = last_engine.1 == 0
                    || (wanted - last_engine.1).abs() >= 3
                    || (wanted != last_engine.1 && last_period_change.elapsed() >= ENGINE_PERIOD_HOLD);
                let period = if period_moved { wanted } else { last_engine.1 };
                let mag_moved = (mag - last_engine.0).abs() >= 300 || (mag == 0 && last_engine.0 != 0);
                if mag_moved || period != last_engine.1 {
                    if dev.periodic_set(mag, period).is_ok() {
                        if period != last_engine.1 {
                            last_period_change = Instant::now();
                        }
                        last_engine = (mag, period);
                    }
                    last_engine_send = Instant::now();
                }
            }
            if last_levels_emit.elapsed() >= core.ui_interval(LEVELS_EMIT_INTERVAL) {
                last_levels_emit = Instant::now();
                if let Ok(mut l) = core.effect_levels.lock() {
                    *l = fx.levels;
                }
                let _ = core.app.emit("effects", fx.levels);
            }

            let limit = p.strength / 100.0;
            let x = angle.abs() - p.deadzone;
            let spring = if x <= 0.0 { 0.0 } else { (x / p.full_angle).min(1.0) * limit };
            let damping = (p.damping / 100.0) * (velocity / DAMPING_REF_SPEED);
            let centering = (-angle.signum() * spring - damping).clamp(-limit, limit);
            let force = (centering + fx.force).clamp(-1.0, 1.0);
            let magnitude = (force * 10000.0) as i32 * polarity as i32;

            if (magnitude - last_sent).abs() >= MIN_DELTA || (magnitude == 0 && last_sent != 0) {
                match dev.constant_force(magnitude) {
                    Ok(()) => {
                        last_sent = magnitude;
                        errors = 0;
                    }
                    Err(_) => {
                        errors += 1;
                        if errors >= MAX_WRITE_ERRORS {
                            reason = "falhas repetidas de escrita";
                            break;
                        }
                    }
                }
            }
        }

        if last_keepalive.elapsed() >= KEEPALIVE_INTERVAL {
            last_keepalive = Instant::now();
            let _ = dev.constant_keepalive();
            if engine_started {
                let _ = dev.periodic_keepalive();
            }
        }
        thread::sleep(LOOP_INTERVAL);
    }

    let _ = dev.constant_stop();
    let _ = dev.periodic_stop();
    if let Ok(mut l) = core.effect_levels.lock() {
        *l = Default::default();
    }
    let _ = core.app.emit("effects", crate::hw::effects::EffectLevels::default());
    unsafe {
        timeEndPeriod(1);
    }
    if let Ok(mut c) = core.centering.lock() {
        *c = None;
    }
    if let Ok(mut s) = core.status.lock() {
        s.centering_active = false;
    }
    core.log("info", "ffb", format!("Centralização do app encerrada: {reason}"));
    core.emit_status();
}
