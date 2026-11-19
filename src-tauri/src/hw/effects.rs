//! Motor de vibração a partir da telemetria do Farming Simulator 25.
//!
//! Roda dentro da malha de força do app (250 Hz) e devolve, a cada passo:
//! - uma força a somar ao efeito de força constante (embreagem, terreno,
//!   impactos: conteúdo de baixa frequência, até ~30 Hz);
//! - a amplitude e a frequência da vibração do motor, executada pelo próprio
//!   firmware como efeito senoidal PID (mais limpa em 15–90 Hz).
//!
//! Modelo, em resumo:
//! - Motor: frequência de explosão = rpm/60 x cilindros/2. Amplitude alta em
//!   baixa rotação e sob carga ("motor puxado"), quase nula em alta rotação.
//! - Embreagem: ao soltar o pedal com marcha engatada, a queda de rotação por
//!   segundo define a trepidação. Soltar de uma vez derruba a rotação e
//!   trepida forte; soltar devagar dá só um toque de "engatou". Motor
//!   afogando trepida forte.
//! - Terreno: ruído filtrado proporcional à velocidade e ao tipo de solo sob
//!   as rodas, mais a atividade real da suspensão; sulcos periódicos em lavoura.
//! - Impactos: pancadas curtas em picos de suspensão e ao aterrissar.
//! - Suspensão: a diferença de movimento entre as rodas dianteiras esquerda e
//!   direita dá trancos no volante (uma roda sobe num torrão, a outra não), e o
//!   movimento vertical comum dá uma trepidação leve.
//!
//! Segurança: a soma dos efeitos nunca passa de `EFFECTS_HARD_CAP` x volume
//! geral; tudo some suavemente sem telemetria recente, com o jogo pausado,
//! fora do veículo ou com o motor desligado.

use crate::game::GameFrame;
use serde::{Deserialize, Serialize};
use std::f32::consts::TAU;

/// Fração máxima da força do volante que os efeitos podem usar (com volume geral em 100%).
pub const EFFECTS_HARD_CAP: f32 = 1.0;
/// Telemetria mais antiga que isso é considerada perdida.
pub const STALE_MS: u128 = 300;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EffectsParams {
    pub enabled: bool,
    /// Volume geral 0..100 %.
    pub master: f32,
    pub engine: f32,
    pub clutch: f32,
    pub terrain: f32,
    pub impacts: f32,
    /// Trancos da suspensão dianteira 0..100 %.
    #[serde(default = "default_suspension")]
    pub suspension: f32,
    /// Cilindros do motor (define a frequência de explosão).
    pub cylinders: u8,
}

impl Default for EffectsParams {
    fn default() -> Self {
        Self { enabled: false, master: 70.0, engine: 60.0, clutch: 80.0, terrain: 60.0, impacts: 70.0, suspension: 60.0, cylinders: 4 }
    }
}

fn default_suspension() -> f32 {
    60.0
}

impl EffectsParams {
    pub fn validated(self) -> Result<Self, String> {
        let pct = |v: f32| v.is_finite() && (0.0..=100.0).contains(&v);
        if pct(self.master) && pct(self.engine) && pct(self.clutch) && pct(self.terrain) && pct(self.impacts) && pct(self.suspension) && (3..=8).contains(&self.cylinders) {
            Ok(self)
        } else {
            Err("Parâmetros de efeitos fora do intervalo permitido".into())
        }
    }
}

/// Intensidade de cada canal (0..1, já com volume aplicado), para a interface.
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct EffectLevels {
    pub active: bool,
    /// Por que os efeitos estão em silêncio, ou "active":
    /// disabled | no_data | not_in_vehicle | paused | motor_off | active.
    pub reason: &'static str,
    pub engine: f32,
    pub engine_hz: f32,
    pub clutch: f32,
    pub terrain: f32,
    pub impacts: f32,
    pub suspension: f32,
}

pub struct Output {
    /// Força a somar à força constante, em fração da força máxima (com sinal).
    pub force: f32,
    /// Amplitude do senoide do motor, 0..1 da força máxima.
    pub engine_mag: f32,
    pub engine_hz: f32,
    pub levels: EffectLevels,
}

/// Motor ligado. Se o mod não souber dizer (`on` ausente), vale a rotação.
fn motor_running(f: &GameFrame) -> bool {
    match f.on {
        Some(on) => on,
        None => f.rpm.map(|r| r > 200.0).unwrap_or(false),
    }
}

fn silence_reason(p: &EffectsParams, f: Option<&GameFrame>) -> &'static str {
    if !p.enabled {
        return "disabled";
    }
    let Some(f) = f else { return "no_data" };
    if f.in_vehicle != Some(true) {
        "not_in_vehicle"
    } else if f.pz == Some(true) {
        "paused"
    } else if !motor_running(f) {
        "motor_off"
    } else {
        "active"
    }
}

/// Filtro passa-baixa de 1 polo.
#[derive(Default)]
struct Lp(f32);
impl Lp {
    fn step(&mut self, x: f32, cutoff_hz: f32, dt: f32) -> f32 {
        let a = 1.0 - (-TAU * cutoff_hz * dt).exp();
        self.0 += a * (x - self.0);
        self.0
    }
}

pub struct Synth {
    rng: u32,
    fade: f32,
    last_frame_t: Option<f64>,
    prev_rpm: Option<f32>,
    prev_clutch: Option<f32>,
    rpm_rate: f32,
    clutch_env: f32,
    clutch_phase: f32,
    noise_a: Lp,
    noise_b: Lp,
    ridge_phase: f32,
    sus_avg: f32,
    was_airborne: bool,
    impact_env: f32,
    impact_phase: f32,
    impact_sign: f32,
    engine_amp: Lp,
    terrain_amp: Lp,
    susp_lp: Lp,
    susp_level: f32,
}

impl Default for Synth {
    fn default() -> Self {
        Self {
            rng: 0x9E37_79B9,
            fade: 0.0,
            last_frame_t: None,
            prev_rpm: None,
            prev_clutch: None,
            rpm_rate: 0.0,
            clutch_env: 0.0,
            clutch_phase: 0.0,
            noise_a: Lp::default(),
            noise_b: Lp::default(),
            ridge_phase: 0.0,
            sus_avg: 0.0,
            was_airborne: false,
            impact_env: 0.0,
            impact_phase: 0.0,
            impact_sign: 1.0,
            engine_amp: Lp::default(),
            terrain_amp: Lp::default(),
            susp_lp: Lp::default(),
            susp_level: 0.0,
        }
    }
}

impl Synth {
    /// Ruído uniforme em -1..1 (xorshift).
    fn noise(&mut self) -> f32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        (x as f32 / u32::MAX as f32) * 2.0 - 1.0
    }

    /// Um passo de `dt` segundos. `frame` é o último quadro do jogo e `fresh`
    /// diz se ele chegou há menos de `STALE_MS`.
    pub fn step(&mut self, p: &EffectsParams, frame: Option<&GameFrame>, fresh: bool, dt: f32) -> Output {
        let f = frame.filter(|_| fresh);
        let reason = silence_reason(p, f);
        let gate = reason == "active";
        // Entra e sai em ~150 ms, sem degrau.
        let target = if gate { 1.0 } else { 0.0 };
        let rate = dt / 0.15;
        self.fade += (target - self.fade).clamp(-rate, rate);

        let silent = Output {
            force: 0.0,
            engine_mag: 0.0,
            engine_hz: 30.0,
            levels: EffectLevels { reason, ..Default::default() },
        };
        let Some(f) = f else {
            self.prev_rpm = None;
            self.prev_clutch = None;
            self.last_frame_t = None;
            return silent;
        };
        if self.fade <= 0.0 && !gate {
            return silent;
        }

        let rmin = f.rmin.filter(|v| *v > 100.0).unwrap_or(800.0);
        let rmax = f.rmax.filter(|v| *v > rmin + 100.0).unwrap_or(rmin + 1400.0);
        let range = rmax - rmin;
        let rpm = f.rpm.unwrap_or(rmin).clamp(0.0, rmax * 1.3);
        let n = ((rpm - rmin) / range).clamp(0.0, 1.0);
        let load = f.ld.unwrap_or(0.0).clamp(0.0, 1.0);
        let speed = f.spd.unwrap_or(0.0).abs();
        let gear_engaged = f.gr.map(|g| g != 0).unwrap_or(true);

        // ---- Eventos calculados uma vez por quadro novo do jogo ----------
        let new_frame = f.t.is_some() && f.t != self.last_frame_t;
        if new_frame {
            let frame_dt = match (f.t, self.last_frame_t) {
                (Some(a), Some(b)) if a > b => ((a - b) / 1000.0) as f32,
                _ => 0.0,
            };
            self.last_frame_t = f.t;
            let clutch = f.clt.map(|c| c.clamp(0.0, 1.0));

            if frame_dt > 0.0 && frame_dt < 0.5 {
                if let Some(prev) = self.prev_rpm {
                    let inst = (rpm - prev) / frame_dt;
                    self.rpm_rate = self.rpm_rate * 0.5 + inst * 0.5;
                }
                let clutch_rate = match (clutch, self.prev_clutch) {
                    (Some(c), Some(pc)) => (c - pc) / frame_dt,
                    _ => 0.0,
                };
                // Pedal: 1 = pressionado. Soltando = valor caindo, ou na zona de atrito.
                let in_friction_zone = clutch.map(|c| c > 0.03 && c < 0.9).unwrap_or(false);
                let releasing = clutch_rate < -0.3 || in_friction_zone || f.csl.unwrap_or(0.0) > 0.0;
                let drop = (-self.rpm_rate).max(0.0) / range; // faixas de rotação por segundo
                // Queda brusca de rotação saindo devagar = embreagem engatando, mesmo
                // que o pedal não seja informado (câmbio automático, outro mod etc.).
                let hard_drop = drop > 0.35 && speed < 15.0 && load > 0.3;
                if gear_engaged && (releasing || hard_drop) && drop > 0.05 {
                    let starting = (1.0 - speed / 15.0).clamp(0.3, 1.0);
                    let severity = ((drop - 0.05) * 1.4).clamp(0.0, 1.0) * (0.5 + 0.5 * load) * starting;
                    self.clutch_env = self.clutch_env.max(severity);
                }
                // Toque de "engatou": pedal termina de subir com marcha engatada.
                if let (Some(c), Some(pc)) = (clutch, self.prev_clutch) {
                    if gear_engaged && pc >= 0.08 && c < 0.08 && speed < 20.0 {
                        self.clutch_env = self.clutch_env.max(0.18);
                    }
                }
                // Motor afogando.
                if f.stl == Some(true) || (rpm > 50.0 && rpm < rmin * 0.82) {
                    self.clutch_env = self.clutch_env.max(0.9);
                }

                // Impactos: pico de atividade da suspensão ou aterrissagem.
                let sus = f.sus.unwrap_or(0.0).max(0.0);
                if sus > 0.25 && sus > self.sus_avg * 2.5 {
                    self.trigger_impact(((sus - 0.25) / 0.8).clamp(0.2, 1.0));
                }
                self.sus_avg = self.sus_avg * 0.95 + sus * 0.05;
                let airborne = match (f.wn, f.wc) {
                    (Some(wn), Some(wc)) => wn > 0 && wc < wn,
                    _ => false,
                };
                if self.was_airborne && !airborne && speed > 3.0 {
                    self.trigger_impact(0.6);
                }
                self.was_airborne = airborne;
            }
            self.prev_rpm = Some(rpm);
            self.prev_clutch = clutch;
        }

        // ---- Motor (senoide do firmware) ---------------------------------
        let low = (1.0 - n).powf(1.6);
        let engine_raw = (0.60 + 0.40 * load) * low + 0.08 * load;
        let engine_level = self.engine_amp.step(engine_raw, 4.0, dt);
        let engine_hz = (rpm.max(rmin * 0.5) / 60.0 * p.cylinders as f32 / 2.0).clamp(10.0, 90.0);

        // A vibração do motor é só a senoide do firmware. Uma segunda componente
        // na força constante (testada na 0.3.2) batia com ela e virava pulsos.

        // ---- Embreagem (trepidação de baixa frequência) ------------------
        self.clutch_env *= (-dt / 0.25).exp();
        let judder_hz = (rpm / 60.0 * 0.5).clamp(6.0, 16.0);
        self.clutch_phase = (self.clutch_phase + TAU * judder_hz * dt) % TAU;
        let clutch_sig = self.clutch_env * self.clutch_phase.sin() * (0.85 + 0.15 * self.noise());

        // ---- Terreno ------------------------------------------------------
        let wc = f.wc.unwrap_or(0) as f32;
        let (road, hard, field, soft) = if wc > 0.0 {
            (
                f.g_road.unwrap_or(0) as f32 / wc,
                f.g_hard.unwrap_or(0) as f32 / wc,
                f.g_field.unwrap_or(0) as f32 / wc,
                f.g_soft.unwrap_or(0) as f32 / wc,
            )
        } else {
            (0.0, 0.0, 0.0, 0.0)
        };
        let roughness = 0.05 * road + 0.50 * hard + 0.80 * field + 0.65 * soft;
        let speed_factor = (speed / 25.0).clamp(0.0, 1.0).powf(0.6);
        let sus_activity = (f.sus.unwrap_or(0.0) / 0.4).clamp(0.0, 1.0);
        let terrain_raw = (speed_factor * roughness * (0.6 + 0.8 * sus_activity)).clamp(0.0, 1.0);
        let terrain_level = self.terrain_amp.step(terrain_raw, 3.0, dt);
        let cutoff = (6.0 + 0.8 * speed).min(35.0);
        let white = self.noise();
        let a = self.noise_a.step(white, cutoff, dt);
        // Dois polos derrubam a amplitude do ruído para ~1/10: o ganho 8 devolve
        // um sinal com picos perto de 1, como os demais canais.
        let colored = self.noise_b.step(a, cutoff, dt) * 8.0;
        let ridge_hz = ((speed / 3.6) / 0.5).min(30.0);
        self.ridge_phase = (self.ridge_phase + TAU * ridge_hz * dt) % TAU;
        let terrain_sig = terrain_level * (0.75 * colored.clamp(-1.0, 1.0) + 0.25 * field * self.ridge_phase.sin());

        // ---- Impactos ------------------------------------------------------
        self.impact_env *= (-dt / 0.12).exp();
        self.impact_phase += TAU * 14.0 * dt;
        let impact_sig = self.impact_env * self.impact_sign * self.impact_phase.sin();

        // ---- Suspensão dianteira --------------------------------------------
        // Diferença esquerda/direita = tranco lateral no volante (0,25 m/s já é
        // um tranco cheio); movimento comum = trepidação vertical leve.
        let (sfl, sfr) = (f.sfl.unwrap_or(0.0), f.sfr.unwrap_or(0.0));
        let kick = ((sfl - sfr) / 0.25).clamp(-1.0, 1.0);
        let vertical = (((sfl + sfr) * 0.5).abs() / 0.3).clamp(0.0, 1.0);
        let susp_raw = 0.8 * kick + 0.35 * vertical * self.noise();
        let susp_sig = self.susp_lp.step(susp_raw, 18.0, dt).clamp(-1.0, 1.0);
        self.susp_level = susp_sig.abs().max(self.susp_level * (-dt / 0.15).exp());

        // ---- Mistura --------------------------------------------------------
        let scale = p.master / 100.0 * EFFECTS_HARD_CAP * self.fade;
        let wc_ = p.clutch / 100.0;
        let wt = p.terrain / 100.0;
        let wi = p.impacts / 100.0;
        let ws = p.suspension / 100.0;
        let force = (scale * (wc_ * clutch_sig + wt * terrain_sig + wi * impact_sig + ws * susp_sig)).clamp(-scale, scale);
        let engine_mag = (scale * p.engine / 100.0 * engine_level).clamp(0.0, scale);

        Output {
            force,
            engine_mag,
            engine_hz,
            levels: EffectLevels {
                active: gate,
                reason,
                engine: engine_mag,
                engine_hz,
                clutch: scale * wc_ * self.clutch_env,
                terrain: scale * wt * terrain_level,
                impacts: scale * wi * self.impact_env,
                suspension: scale * ws * self.susp_level,
            },
        }
    }

    fn trigger_impact(&mut self, strength: f32) {
        if strength > self.impact_env {
            self.impact_env = strength;
            self.impact_phase = 0.0;
            self.impact_sign = if self.noise() >= 0.0 { 1.0 } else { -1.0 };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(t: f64, rpm: f32, load: f32, clt: f32, spd: f32) -> GameFrame {
        GameFrame {
            t: Some(t),
            in_vehicle: Some(true),
            on: Some(true),
            pz: Some(false),
            rpm: Some(rpm),
            rmin: Some(850.0),
            rmax: Some(2200.0),
            ld: Some(load),
            clt: Some(clt),
            gr: Some(1),
            spd: Some(spd),
            wn: Some(4),
            wc: Some(4),
            g_road: Some(4),
            ..Default::default()
        }
    }

    fn params() -> EffectsParams {
        EffectsParams { enabled: true, master: 100.0, ..Default::default() }
    }

    /// Roda `secs` segundos com o quadro dado e devolve a última saída.
    fn run(s: &mut Synth, p: &EffectsParams, f: &GameFrame, secs: f32) -> Output {
        let dt = 0.004;
        let mut out = s.step(p, Some(f), true, dt);
        for _ in 1..(secs / dt) as usize {
            out = s.step(p, Some(f), true, dt);
        }
        out
    }

    #[test]
    fn engine_vibrates_more_at_low_rpm_and_under_load() {
        let p = params();
        let idle = run(&mut Synth::default(), &p, &frame(1.0, 850.0, 0.1, 0.0, 0.0), 2.0);
        let lugging = run(&mut Synth::default(), &p, &frame(1.0, 950.0, 1.0, 0.0, 5.0), 2.0);
        let high = run(&mut Synth::default(), &p, &frame(1.0, 2150.0, 0.3, 0.0, 20.0), 2.0);
        assert!(lugging.engine_mag > idle.engine_mag, "motor puxado deve vibrar mais que em marcha lenta");
        assert!(idle.engine_mag > high.engine_mag * 3.0, "alta rotação deve vibrar bem menos");
        assert!((idle.engine_hz - 850.0 / 60.0 * 2.0).abs() < 0.5, "4 cilindros: rpm/60 x 2");
        assert!(lugging.engine_mag <= EFFECTS_HARD_CAP);
    }

    #[test]
    fn fast_clutch_release_judders_more_than_slow() {
        let p = params();
        let judder = |release_s: f32, rpm_end: f32| {
            let mut s = Synth::default();
            run(&mut s, &p, &frame(0.0, 1200.0, 0.2, 1.0, 0.0), 0.5);
            let frames = (release_s / 0.01) as usize;
            let mut peak: f32 = 0.0;
            for i in 1..=frames {
                let k = i as f32 / frames as f32;
                let fr = frame(10.0 * i as f64, 1200.0 - (1200.0 - rpm_end) * k, 0.8, 1.0 - k, 1.0 * k);
                for _ in 0..3 {
                    let o = s.step(&p, Some(&fr), true, 0.0033);
                    peak = peak.max(o.levels.clutch);
                }
            }
            peak
        };
        let fast = judder(0.15, 900.0);
        let slow = judder(2.0, 1150.0);
        assert!(fast > 0.3, "soltar de uma vez deve trepidar forte (pico {fast})");
        assert!(slow < fast * 0.5, "soltar devagar deve trepidar bem menos (lenta {slow}, rápida {fast})");
        assert!(slow > 0.0, "soltar devagar ainda dá o toque de engate");
    }

    #[test]
    fn silent_when_paused_or_stale_or_disabled() {
        let mut f = frame(1.0, 900.0, 1.0, 0.0, 10.0);
        let p = params();
        let mut s = Synth::default();
        run(&mut s, &p, &f, 1.0);
        f.pz = Some(true);
        let paused = run(&mut s, &p, &f, 0.5);
        assert_eq!(paused.engine_mag, 0.0);
        assert_eq!(paused.force, 0.0);

        let mut s = Synth::default();
        let stale = s.step(&p, Some(&frame(1.0, 900.0, 1.0, 0.0, 10.0)), false, 0.004);
        assert_eq!(stale.engine_mag, 0.0);

        let off = EffectsParams { enabled: false, ..p };
        let o = run(&mut Synth::default(), &off, &frame(1.0, 900.0, 1.0, 0.0, 10.0), 1.0);
        assert_eq!(o.engine_mag, 0.0);
        assert_eq!(o.force, 0.0);
    }

    /// Regressão da 0.3.2: uma componente do motor na força constante batia com
    /// a senoide do firmware e virava pulsos. Em asfalto liso, rotação estável e
    /// sem eventos, a força constante deve ficar praticamente parada.
    #[test]
    fn steady_engine_does_not_pulse_through_constant_force() {
        let p = params();
        let mut s = Synth::default();
        let mut f = frame(1.0, 900.0, 0.4, 0.0, 0.0);
        f.sus = Some(0.0);
        run(&mut s, &p, &f, 2.0);
        for _ in 0..1000 {
            let o = s.step(&p, Some(&f), true, 0.004);
            assert!(o.force.abs() < 0.01, "força constante oscilando: {}", o.force);
            assert!(o.engine_mag > 0.2, "a vibração do motor continua na senoide");
        }
    }

    /// Reproduz uma gravação real do jogo (botão "Gravar 30 s" na tela Game
    /// Effects) e imprime a linha do tempo dos efeitos. Uso:
    /// `PXN_REPLAY=caminho\\telemetria-xxx.jsonl cargo test replay -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn replay() {
        let path = std::env::var("PXN_REPLAY").expect("defina PXN_REPLAY com o caminho da gravação");
        let text = std::fs::read_to_string(path).expect("gravação ilegível");
        let p = params();
        let mut s = Synth::default();
        let mut prev_t: Option<f64> = None;
        for line in text.lines() {
            let Ok(f) = serde_json::from_str::<GameFrame>(line) else { continue };
            let Some(t) = f.t else { continue };
            let dt = prev_t.map(|pt| ((t - pt) / 1000.0) as f32).unwrap_or(0.01).clamp(0.001, 0.1);
            prev_t = Some(t);
            let o = s.step(&p, Some(&f), true, dt);
            println!(
                "t={:>8.0} rpm={:>6.0} clt={:>5} gr={:>3} spd={:>5.1} ld={:>4.2} csl={:>5} stl={:>5} sus={:>5} | {:<14} motor={:.2} embr={:.2} terreno={:.2} impacto={:.2}",
                t,
                f.rpm.unwrap_or(-1.0),
                f.clt.map(|v| format!("{v:.2}")).unwrap_or("-".into()),
                f.gr.map(|v| v.to_string()).unwrap_or("-".into()),
                f.spd.unwrap_or(0.0),
                f.ld.unwrap_or(0.0),
                f.csl.map(|v| format!("{v:.2}")).unwrap_or("-".into()),
                f.stl.map(|v| v.to_string()).unwrap_or("-".into()),
                f.sus.map(|v| format!("{v:.2}")).unwrap_or("-".into()),
                o.levels.reason,
                o.levels.engine,
                o.levels.clutch,
                o.levels.terrain,
                o.levels.impacts
            );
        }
    }

    #[test]
    fn front_suspension_kicks_the_wheel_only_when_sides_differ() {
        let p = params();
        let mut even = frame(1.0, 1500.0, 0.5, 0.0, 12.0);
        even.sfl = Some(0.2);
        even.sfr = Some(0.2);
        let mut bump = even.clone();
        bump.sfl = Some(0.3);
        bump.sfr = Some(-0.1);
        let o_even = run(&mut Synth::default(), &p, &even, 0.5);
        let o_bump = run(&mut Synth::default(), &p, &bump, 0.5);
        assert!(o_bump.levels.suspension > 0.5, "roda esquerda subindo deve dar tranco (nível {})", o_bump.levels.suspension);
        assert!(o_even.levels.suspension < o_bump.levels.suspension * 0.5, "movimento igual dos dois lados é só trepidação leve");
        assert!(o_bump.force.abs() <= EFFECTS_HARD_CAP + 1e-6);
    }

    #[test]
    fn reports_why_it_is_silent() {
        let p = params();
        let mut f = frame(1.0, 900.0, 0.5, 0.0, 5.0);
        assert_eq!(silence_reason(&p, Some(&f)), "active");
        f.on = Some(false);
        assert_eq!(silence_reason(&p, Some(&f)), "motor_off");
        // Mod antigo sem o campo: decide pela rotação.
        f.on = None;
        assert_eq!(silence_reason(&p, Some(&f)), "active");
        f.pz = Some(true);
        assert_eq!(silence_reason(&p, Some(&f)), "paused");
        f.in_vehicle = Some(false);
        assert_eq!(silence_reason(&p, Some(&f)), "not_in_vehicle");
        assert_eq!(silence_reason(&p, None), "no_data");
        assert_eq!(silence_reason(&EffectsParams { enabled: false, ..p }, Some(&f)), "disabled");
    }

    #[test]
    fn field_is_rougher_than_road_and_never_exceeds_cap() {
        let p = params();
        let mut road = frame(1.0, 1600.0, 0.5, 0.0, 20.0);
        road.sus = Some(0.1);
        let mut field = road.clone();
        field.g_road = Some(0);
        field.g_field = Some(4);
        let r = run(&mut Synth::default(), &p, &road, 2.0);
        let mut s = Synth::default();
        let fo = run(&mut s, &p, &field, 2.0);
        assert!(fo.levels.terrain > r.levels.terrain * 5.0);
        for _ in 0..2000 {
            let o = s.step(&p, Some(&field), true, 0.004);
            assert!(o.force.abs() <= EFFECTS_HARD_CAP + 1e-6);
        }
    }
}
