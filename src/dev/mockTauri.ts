// Simulador do núcleo nativo, só para desenvolvimento no navegador
// (http://localhost:1430/?mock). Gera volante, pedais e telemetria do jogo
// falsos e permite simular a desconexão do jogo com ?gameOffAfter=SEGUNDOS.
// Nunca entra no instalador: é carregado apenas com import.meta.env.DEV.

type Handler = (msg: { event: string; id: number; payload: unknown }) => void;

const callbacks = new Map<number, Handler>();
const listeners = new Map<string, number[]>();
let nextId = 1;
const t0 = performance.now();
const params = new URLSearchParams(location.search);
const gameOffAfter = Number(params.get("gameOffAfter") ?? "0") * 1000;

const axes = [0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36].map((usage, i) => ({
  usage,
  name: ["X", "Y", "Z", "Rx", "Ry", "Rz", "Slider"][i],
  bits: 16,
  min: usage === 0x30 || usage === 0x31 ? -32767 : 0,
  max: usage === 0x30 || usage === 0x31 ? 32767 : 65535,
}));

const mockStatus = {
  connected: true,
  mode: "DirectInput",
  info: {
    product: "PXN-V99 (simulado)",
    manufacturer: "PXN",
    serial: "000000000000",
    vid: 0x11ff,
    pid: 0x3245,
    revision: 0x38,
    path: "mock",
    input_len: 19,
    output_len: 18,
    feature_len: 5,
    axes,
    button_count: 24,
    has_hat: true,
    has_pid_state: true,
    has_device_gain: true,
    pid_pool: { ram_pool_size: 600, simultaneous_effects_max: 10 },
  },
  report_hz: 250,
  total_reports: 0,
  error_count: 0,
  reconnects: 0,
  last_error: null,
  applied_gain_percent: null,
  centering_active: true,
};

let seq = 0;
function telemetry() {
  const s = (performance.now() - t0) / 1000;
  seq++;
  return {
    seq,
    t_ms: performance.now() - t0,
    axes: [Math.round(Math.sin(s * 0.7) * 9000), 0, Math.round((Math.sin(s) * 0.5 + 0.5) * 65535), 0, 0, 20000, 0],
    hat: null,
    buttons: s % 4 < 2 ? [17] : [18],
    pid: { effect_playing: true, actuators_enabled: true, paused: false, safety_switch: false, actuator_power: true, effect_block_index: 1 },
  };
}

let lastFrame: Record<string, unknown> | null = null;
function game() {
  const ms = performance.now() - t0;
  const connected = !gameOffAfter || ms < gameOffAfter;
  if (connected) {
    const s = ms / 1000;
    lastFrame = {
      v: 1, t: ms, pz: false, in: true, on: true, rpm: 1100 + Math.sin(s) * 300, rmin: 850, rmax: 2200, ld: 0.5, thr: 0.4, clt: 0, gr: 3,
      spd: 12, wn: 4, wc: 4, gRd: 0, gHd: 0, gFd: 4, gSf: 0, dep: 0.03, slp: 0.05, csl: 0, stl: false, trq: 0.4, sus: 0.2, wat: 0,
      sfl: Math.sin(s * 3) * 0.2, sfr: Math.cos(s * 3) * 0.2,
    };
  }
  return { connected, hz: connected ? 100 : 0, frames: Math.round(ms / 10), bad_lines: 0, last: lastFrame, last_message: null };
}

function levels() {
  const g = game();
  return g.connected
    ? { active: true, reason: "active", engine: 0.3, engine_hz: 37, clutch: 0, terrain: 0.25, impacts: 0, suspension: 0.2 }
    : { active: false, reason: "no_data", engine: 0, engine_hz: 30, clutch: 0, terrain: 0, impacts: 0, suspension: 0 };
}

function emit(event: string, payload: unknown) {
  for (const id of listeners.get(event) ?? []) callbacks.get(id)?.({ event, id, payload });
}

const commands: Record<string, (args: Record<string, unknown>) => unknown> = {
  get_status: () => mockStatus,
  get_telemetry: telemetry,
  get_log: () => [],
  get_game: game,
  game_record: () => "C:\\mock\\telemetria.jsonl",
  effects_update: () => null,
  centering_start: () => 1,
  centering_update: () => null,
  centering_stop: () => null,
  device_control: () => null,
  vibration_test: () => null,
  ui_heartbeat: () => null,
  ui_error: (a) => console.warn("[mock ui_error]", a.message),
  tray_labels: () => null,
};

(window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {
  transformCallback(cb: Handler) {
    const id = nextId++;
    callbacks.set(id, cb);
    return id;
  },
  async invoke(cmd: string, args: Record<string, unknown> = {}) {
    if (cmd === "plugin:event|listen") {
      const list = listeners.get(args.event as string) ?? [];
      list.push(args.handler as number);
      listeners.set(args.event as string, list);
      return args.handler;
    }
    if (cmd === "plugin:event|unlisten") return null;
    const fn = commands[cmd];
    if (!fn) throw new Error(`mock: comando desconhecido ${cmd}`);
    return fn(args);
  },
};
(window as unknown as Record<string, unknown>).__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => undefined };

setInterval(() => emit("telemetry", telemetry()), 16);
setInterval(() => emit("game", game()), 33);
setInterval(() => emit("effects", levels()), 66);
setInterval(() => emit("status", mockStatus), 500);
console.info("[mock] núcleo simulado ativo", { gameOffAfter });
export {};
