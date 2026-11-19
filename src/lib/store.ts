// Stores mínimos (useSyncExternalStore) para status, telemetria, log e configurações.
import { useSyncExternalStore } from "react";
import { api, type DeviceStatus, type EffectLevels, type EffectsParams, type GameStatus, type LogEntry, type Telemetry } from "./api";

export interface Store<T> {
  get: () => T;
  set: (v: T) => void;
  update: (fn: (v: T) => T) => void;
  subscribe: (cb: () => void) => () => void;
}

export function createStore<T>(initial: T): Store<T> {
  let value = initial;
  const subs = new Set<() => void>();
  const set = (v: T) => {
    value = v;
    subs.forEach((cb) => cb());
  };
  return {
    get: () => value,
    set,
    update: (fn) => set(fn(value)),
    subscribe: (cb) => {
      subs.add(cb);
      return () => subs.delete(cb);
    },
  };
}

/** Agrupa atualizações frequentes em um único desenho por quadro de tela. */
function frameBatched<T>(store: Store<T>): (v: T) => void {
  let pending: T;
  let scheduled = false;
  return (v: T) => {
    pending = v;
    if (scheduled) return;
    scheduled = true;
    requestAnimationFrame(() => {
      scheduled = false;
      store.set(pending);
    });
  };
}

export function useStore<T>(store: Store<T>): T {
  return useSyncExternalStore(store.subscribe, store.get);
}

const emptyStatus: DeviceStatus = {
  connected: false,
  mode: null,
  info: null,
  report_hz: 0,
  total_reports: 0,
  error_count: 0,
  reconnects: 0,
  last_error: null,
  applied_gain_percent: null,
  centering_active: false,
};
const emptyTelemetry: Telemetry = { seq: 0, t_ms: 0, axes: [], hat: null, buttons: [], pid: null };

export const statusStore = createStore<DeviceStatus>(emptyStatus);
export const telemetryStore = createStore<Telemetry>(emptyTelemetry);
export const logStore = createStore<LogEntry[]>([]);
export const effectLevelsStore = createStore<EffectLevels>({ active: false, reason: "", engine: 0, engine_hz: 0, clutch: 0, terrain: 0, impacts: 0, suspension: 0 });
export const gameStore = createStore<GameStatus>({ connected: false, hz: 0, frames: 0, bad_lines: 0, last: null, last_message: null });

/** Velocidade do eixo de direção em fração do curso total por segundo (suavizada). */
export const steerRateStore = createStore(0);

// ---- Configurações locais (persistidas neste computador) -------------------

export type PedalKey = "throttle" | "brake" | "clutch" | "clutchLeft" | "clutchRight";
/** Posições do câmbio H, mais a chave Low/High ("LH") e o botão P (freio de estacionamento). */
export type GearKey = "1" | "2" | "3" | "4" | "5" | "6" | "R" | "LH" | "P";

const SHIFTER_LAYOUT_VERSION = 2;

export type Lang = "pt" | "en" | "es";

export interface Settings {
  /** Idioma da interface. */
  lang: Lang;
  /** Curso total configurado no volante, em graus. Usado apenas para a escala visual. */
  steeringRange: number;
  /** Usage HID do eixo de direção. */
  steeringUsage: number;
  pedals: Record<PedalKey, { usage: number | null; invert: boolean }>;
  /** Botão HID (1-based) que cada posição do câmbio H aciona. */
  shifter: Record<GearKey, number | null>;
  /** Versão do mapeamento padrão do câmbio; mapeamentos antigos são substituídos. */
  shifterVersion: number;
  /** false: botão da chave pressionado = High. true: pressionado = Low. */
  rangeInverted: boolean;
  /** Centralização controlada pelo app. */
  centering: { strength: number; fullAngle: number; damping: number; deadzone: number };
  /** Sentido do motor detectado na calibração (+1/-1), ou null se ainda não calibrado. */
  motorPolarity: number | null;
  /** Efeitos de jogo (vibração a partir da telemetria do FS25). */
  effects: EffectsParams;
}

const SETTINGS_KEY = "pxn-cc.settings.v2";

// Padrão conferido no volante real: acelerador = Z, freio = Rz, embreagem (pedal) = Slider.
// Rx/Ry são as duas embreagens de alavanca atrás do volante (lado a confirmar).
function systemLang(): Lang {
  const l = (typeof navigator !== "undefined" ? navigator.language : "pt").toLowerCase();
  return l.startsWith("es") ? "es" : l.startsWith("en") ? "en" : "pt";
}

const defaultSettings: Settings = {
  lang: systemLang(),
  steeringRange: 900,
  steeringUsage: 0x30,
  pedals: {
    throttle: { usage: 0x32, invert: false },
    brake: { usage: 0x35, invert: false },
    clutch: { usage: 0x36, invert: false },
    clutchLeft: { usage: 0x33, invert: false },
    clutchRight: { usage: 0x34, invert: false },
  },
  // Conferido no câmbio real: marchas 1 a 6 nos botões 17 a 22, ré no 23,
  // chave Low/High no 16 e P no 24.
  shifter: { "1": 17, "2": 18, "3": 19, "4": 20, "5": 21, "6": 22, R: 23, LH: 16, P: 24 },
  shifterVersion: SHIFTER_LAYOUT_VERSION,
  rangeInverted: false,
  centering: { strength: 30, fullAngle: 30, damping: 30, deadzone: 0 },
  motorPolarity: null,
  effects: { enabled: false, master: 70, engine: 60, clutch: 80, terrain: 60, impacts: 70, suspension: 60, cylinders: 4 },
};

function loadSettings(): Settings {
  try {
    const raw = localStorage.getItem(SETTINGS_KEY);
    if (raw) {
      const parsed = JSON.parse(raw) as Partial<Settings>;
      return { ...defaultSettings, ...parsed, pedals: { ...defaultSettings.pedals, ...parsed.pedals },
        // Um mapeamento salvo antes da correção do câmbio é descartado.
        shifter:
          parsed.shifterVersion === SHIFTER_LAYOUT_VERSION
            ? { ...defaultSettings.shifter, ...parsed.shifter }
            : defaultSettings.shifter,
        shifterVersion: SHIFTER_LAYOUT_VERSION,
        centering: { ...defaultSettings.centering, ...parsed.centering },
        effects: { ...defaultSettings.effects, ...parsed.effects },
      };
    }
  } catch {
    /* configuração ilegível: usar padrão */
  }
  return defaultSettings;
}

export const settingsStore = createStore<Settings>(loadSettings());
settingsStore.subscribe(() => {
  try {
    localStorage.setItem(SETTINGS_KEY, JSON.stringify(settingsStore.get()));
  } catch {
    /* sem armazenamento: seguir apenas em memória */
  }
});

// ---- Ligação com o núcleo --------------------------------------------------

let started = false;

export async function startBridge() {
  if (started) return;
  started = true;

  let prev: { norm: number; t: number } | null = null;
  let lastWall = 0;

  await api.onStatus((s) => statusStore.set(s));
  await api.onLog((l) => logStore.update((list) => [...list.slice(-499), l]));
  const setGame = frameBatched(gameStore);
  const setLevels = frameBatched(effectLevelsStore);
  const setTelemetry = frameBatched(telemetryStore);
  await api.onGame((g) => setGame(g));
  await api.onEffects((l) => setLevels(l));
  // O núcleo começa com os efeitos desligados: enviar a configuração salva.
  await api.effectsUpdate(settingsStore.get().effects, false).catch(() => undefined);
  await api.onTelemetry((t) => {
    setTelemetry(t);
    const norm = steeringNorm(t, statusStore.get(), settingsStore.get());
    if (norm !== null) {
      if (prev && t.t_ms > prev.t) {
        const inst = (norm - prev.norm) / ((t.t_ms - prev.t) / 1000);
        steerRateStore.set(steerRateStore.get() * 0.6 + inst * 0.4);
      }
      prev = { norm, t: t.t_ms };
      lastWall = performance.now();
    }
  });
  // Volante parado não envia relatórios: zerar a velocidade quando a leitura fica antiga.
  setInterval(() => {
    if (steerRateStore.get() !== 0 && performance.now() - lastWall > 120) steerRateStore.set(0);
  }, 60);

  statusStore.set(await api.getStatus());
  telemetryStore.set(await api.getTelemetry());
  logStore.set(await api.getLog());
  gameStore.set(await api.getGame());
}

// ---- Derivações ------------------------------------------------------------

/** Posição do eixo de direção em -1..1 (0 = centro lógico), ou null sem dados. */
export function steeringNorm(t: Telemetry, s: DeviceStatus, cfg: Settings): number | null {
  const axes = s.info?.axes;
  if (!axes || t.axes.length !== axes.length) return null;
  const i = axes.findIndex((a) => a.usage === cfg.steeringUsage);
  if (i < 0) return null;
  const { min, max } = axes[i];
  if (max <= min) return null;
  return Math.max(-1, Math.min(1, ((t.axes[i] - min) / (max - min)) * 2 - 1));
}

/** Posição de um eixo em 0..1, ou null se não mapeado ou sem dados. */
export function axisFraction(t: Telemetry, s: DeviceStatus, usage: number | null, invert = false): number | null {
  const axes = s.info?.axes;
  if (usage === null || !axes || t.axes.length !== axes.length) return null;
  const i = axes.findIndex((a) => a.usage === usage);
  if (i < 0) return null;
  const { min, max } = axes[i];
  if (max <= min) return null;
  const f = Math.max(0, Math.min(1, (t.axes[i] - min) / (max - min)));
  return invert ? 1 - f : f;
}
