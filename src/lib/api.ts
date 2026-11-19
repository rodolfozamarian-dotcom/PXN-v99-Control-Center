// Contrato com o núcleo nativo. A interface nunca acessa o hardware diretamente.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type Mode = "DirectInput" | "XInput";

export interface AxisInfo {
  usage: number;
  name: string;
  bits: number;
  min: number;
  max: number;
}

export interface DeviceInfo {
  product: string;
  manufacturer: string;
  serial: string;
  vid: number;
  pid: number;
  revision: number;
  path: string;
  input_len: number;
  output_len: number;
  feature_len: number;
  axes: AxisInfo[];
  button_count: number;
  has_hat: boolean;
  has_pid_state: boolean;
  has_device_gain: boolean;
  pid_pool: { ram_pool_size: number; simultaneous_effects_max: number } | null;
}

export interface DeviceStatus {
  connected: boolean;
  mode: Mode | null;
  info: DeviceInfo | null;
  report_hz: number;
  total_reports: number;
  error_count: number;
  reconnects: number;
  last_error: string | null;
  applied_gain_percent: number | null;
  centering_active: boolean;
}

export interface PidState {
  effect_playing: boolean;
  actuators_enabled: boolean;
  paused: boolean;
  safety_switch: boolean;
  actuator_power: boolean;
  effect_block_index: number;
}

export interface Telemetry {
  seq: number;
  t_ms: number;
  axes: number[];
  hat: number | null;
  buttons: number[];
  pid: PidState | null;
}

export interface LogEntry {
  id: number;
  ts: number;
  level: "info" | "warn" | "error";
  category: string;
  message: string;
}

export interface CenteringParams {
  strength: number;
  full_angle: number;
  damping: number;
  deadzone: number;
  half_range: number;
}


export interface GameFrame {
  v?: number;
  t?: number;
  pz?: boolean;
  in?: boolean;
  on?: boolean;
  rpm?: number;
  rmin?: number;
  rmax?: number;
  ld?: number;
  thr?: number;
  clt?: number;
  gr?: number;
  spd?: number;
  wn?: number;
  wc?: number;
  gRd?: number;
  gHd?: number;
  gFd?: number;
  gSf?: number;
  dep?: number;
  slp?: number;
  csl?: number;
  stl?: boolean;
  trq?: number;
  sus?: number;
  wat?: number;
  sfl?: number;
  sfr?: number;
}

export interface GameStatus {
  connected: boolean;
  hz: number;
  frames: number;
  bad_lines: number;
  last: GameFrame | null;
  last_message: string | null;
}

export interface EffectsParams {
  enabled: boolean;
  master: number;
  engine: number;
  clutch: number;
  terrain: number;
  impacts: number;
  suspension: number;
  cylinders: number;
}

export interface EffectLevels {
  active: boolean;
  reason: "disabled" | "no_data" | "not_in_vehicle" | "paused" | "motor_off" | "active" | "";
  engine: number;
  engine_hz: number;
  clutch: number;
  terrain: number;
  impacts: number;
  suspension: number;
}

export const api = {
  getStatus: () => invoke<DeviceStatus>("get_status"),
  getTelemetry: () => invoke<Telemetry>("get_telemetry"),
  getLog: () => invoke<LogEntry[]>("get_log"),
  getGame: () => invoke<GameStatus>("get_game"),
  gameRecord: () => invoke<string>("game_record"),
  uiHeartbeat: () => invoke<void>("ui_heartbeat"),
  uiError: (message: string) => invoke<void>("ui_error", { message }),
  trayLabels: (open: string, quit: string) => invoke<void>("tray_labels", { open, quit }),
  onUiWake: (cb: () => void) => listen<null>("ui-wake", () => cb()),
  effectsUpdate: (params: EffectsParams, commit: boolean) => invoke<void>("effects_update", { params, commit }),
  onEffects: (cb: (l: EffectLevels) => void) => listen<EffectLevels>("effects", (e) => cb(e.payload)),
  onGame: (cb: (g: GameStatus) => void) => listen<GameStatus>("game", (e) => cb(e.payload)),
  centeringStart: (params: CenteringParams, polarity: number | null) =>
    invoke<number>("centering_start", { params, polarity }),
  centeringUpdate: (params: CenteringParams, commit: boolean) => invoke<void>("centering_update", { params, commit }),
  centeringStop: () => invoke<void>("centering_stop"),
  vibrationTest: () => invoke<void>("vibration_test"),
  deviceControl: (command: "enable" | "disable" | "stop_all" | "reset") => invoke<void>("device_control", { command }),
  onStatus: (cb: (s: DeviceStatus) => void) => listen<DeviceStatus>("status", (e) => cb(e.payload)),
  onTelemetry: (cb: (t: Telemetry) => void) => listen<Telemetry>("telemetry", (e) => cb(e.payload)),
  onLog: (cb: (l: LogEntry) => void) => listen<LogEntry>("log", (e) => cb(e.payload)),
};

export const hex4 = (n: number) => n.toString(16).toUpperCase().padStart(4, "0");
