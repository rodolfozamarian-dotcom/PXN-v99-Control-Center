// Efeitos de jogo: vibração no volante a partir da telemetria do Farming
// Simulator 25 (mod FS25_PXNV99Telemetry). O modelo físico roda no núcleo
// (src-tauri/src/hw/effects.rs); aqui ficam os controles e os medidores.
import { useEffect, useRef, useState } from "react";
import { SpringControl } from "../components/SpringControl";
import { Badge, Button, Fader, Indicator, Note, PageHeader, Panel, Readout, Segmented } from "../components/ui";
import { api, type EffectsParams, type GameFrame } from "../lib/api";
import { useT, type Key } from "../lib/i18n";
import { effectLevelsStore, gameStore, settingsStore, statusStore, useStore } from "../lib/store";

type EffectKey = "engine" | "clutch" | "terrain" | "suspension" | "impacts";

const EFFECTS: { key: EffectKey; label: Key; hint: Key }[] = [
  { key: "engine", label: "fx.engine", hint: "fx.engineHint" },
  { key: "clutch", label: "fx.clutch", hint: "fx.clutchHint" },
  { key: "terrain", label: "fx.terrain", hint: "fx.terrainHint" },
  { key: "suspension", label: "fx.suspension", hint: "fx.suspensionHint" },
  { key: "impacts", label: "fx.impacts", hint: "fx.impactsHint" },
];

const CYLINDERS = [3, 4, 6] as const;
const SEND_INTERVAL_MS = 50;

/** Grava 30 s da telemetria crua do jogo em arquivo, para diagnóstico. */
function RecordButton({ disabled }: { disabled: boolean }) {
  const t = useT();
  const [state, setState] = useState<{ until: number; path: string } | { error: string } | null>(null);
  const [, tick] = useState(0);
  const recording = state !== null && "until" in state && Date.now() < state.until;

  useEffect(() => {
    if (!recording) return;
    const id = window.setInterval(() => tick((n) => n + 1), 500);
    return () => clearInterval(id);
  }, [recording]);

  const start = () =>
    api
      .gameRecord()
      .then((path) => setState({ until: Date.now() + 30_000, path }))
      .catch((e) => setState({ error: String(e) }));

  return (
    <div className="mt-4 border-t border-line pt-3">
      <Button disabled={disabled || recording} onClick={() => void start()}>
        {recording ? t("fx.recording", { s: Math.ceil(((state as { until: number }).until - Date.now()) / 1000) }) : t("fx.record")}
      </Button>
      {state && "path" in state && <p className="selectable mono mt-2 break-all text-[11px] text-dim">{state.path}</p>}
      {state && "error" in state && <p className="selectable mt-2 text-xs text-bad">{state.error}</p>}
    </div>
  );
}

function EffectsPanel() {
  const t = useT();
  const cfg = useStore(settingsStore);
  const levels = useStore(effectLevelsStore);
  const status = useStore(statusStore);
  const g = useStore(gameStore);
  const fx = cfg.effects;
  const lastSent = useRef(0);
  const trailing = useRef<number | null>(null);

  const push = (next: EffectsParams, commit: boolean) => {
    const send = () => {
      lastSent.current = performance.now();
      api.effectsUpdate(next, commit).catch(() => undefined);
    };
    if (trailing.current !== null) clearTimeout(trailing.current);
    trailing.current = null;
    const wait = SEND_INTERVAL_MS - (performance.now() - lastSent.current);
    if (commit || wait <= 0) send();
    else trailing.current = window.setTimeout(send, wait);
  };

  const update = (patch: Partial<EffectsParams>, commit: boolean) => {
    const next = { ...fx, ...patch };
    settingsStore.update((s) => ({ ...s, effects: next }));
    push(next, commit);
  };

  // Medidor: nível atual relativo ao teto (volume geral x força máxima).
  const ceiling = Math.max(0.0001, fx.master / 100);

  const silence: Partial<Record<string, Key>> = {
    no_data: "fx.silentNoData",
    not_in_vehicle: "fx.silentNotInVehicle",
    paused: "fx.silentPaused",
    motor_off: "fx.silentMotorOff",
  };
  let state: { tone: "ok" | "warn" | "off"; text: string };
  if (!fx.enabled) state = { tone: "off", text: t("fx.off") };
  else if (!status.centering_active) state = { tone: "warn", text: t("fx.needEngine") };
  else if (!g.connected) state = { tone: "warn", text: t("fx.waitingGame") };
  else if (!levels.active) state = { tone: "warn", text: t(silence[levels.reason] ?? "fx.silent") };
  else state = { tone: "ok", text: t("fx.vibrating", { hz: levels.engine_hz.toFixed(0) }) };

  return (
    <div>
      <div className="mb-5 flex flex-wrap items-center gap-3">
        <label className="flex cursor-pointer items-center gap-2.5">
          <input type="checkbox" checked={fx.enabled} onChange={(e) => update({ enabled: e.target.checked }, true)} className="h-4 w-4 accent-[var(--color-accent)]" />
          <span className="font-display text-[13px] uppercase tracking-[0.1em]">{t("fx.enable")}</span>
        </label>
        <Badge tone={state.tone}>{state.text}</Badge>
      </div>
      <div className="grid grid-cols-3 gap-x-7 gap-y-6">
        <Fader label={t("fx.master")} value={fx.master} hint={t("fx.masterHint")} onChange={(v, c) => update({ master: v }, c)} />
        {EFFECTS.map((e) => (
          <Fader key={e.key} label={t(e.label)} value={fx[e.key]} level={levels[e.key] / ceiling} hint={t(e.hint)} onChange={(v, c) => update({ [e.key]: v } as Partial<EffectsParams>, c)} />
        ))}
      </div>
      <div className="mt-6 flex items-center gap-4 border-t border-line pt-4">
        <span className="label">{t("fx.cylinders")}</span>
        <Segmented options={CYLINDERS} value={fx.cylinders as (typeof CYLINDERS)[number]} onChange={(c) => update({ cylinders: c }, true)} />
        <span className="text-xs text-dim">{t("fx.cylindersHint")}</span>
      </div>
    </div>
  );
}

function Bar({ label, value, color = "var(--color-accent)" }: { label: string; value: number | null; color?: string }) {
  const pct = value === null ? 0 : Math.max(0, Math.min(1, value)) * 100;
  return (
    <div>
      <div className="mb-1.5 flex justify-between">
        <span className="label">{label}</span>
        <span className="mono text-xs text-text">{value === null ? "—" : `${pct.toFixed(0)}%`}</span>
      </div>
      <div className="h-1.5 bg-bg">
        <div className="h-full transition-[width] duration-75" style={{ width: `${pct}%`, background: color }} />
      </div>
    </div>
  );
}

const GROUNDS: { key: keyof GameFrame; label: Key; color: string }[] = [
  { key: "gRd", label: "fx.groundRoad", color: "var(--color-steel)" },
  { key: "gHd", label: "fx.groundHard", color: "var(--color-warn)" },
  { key: "gFd", label: "fx.groundField", color: "var(--color-ok)" },
  { key: "gSf", label: "fx.groundSoft", color: "var(--color-bad)" },
];

export function GameEffects() {
  const t = useT();
  const g = useStore(gameStore);
  const f = g.last;
  const live = g.connected && f !== null;
  const inVehicle = live && f?.in === true;
  const rpmFrac = f?.rpm !== undefined && f.rmax ? (f.rpm - (f.rmin ?? 0)) / Math.max(1, f.rmax - (f.rmin ?? 0)) : null;
  const yesNo = (v: boolean) => (v ? t("common.yes") : t("common.no"));

  return (
    <div>
      <PageHeader
        code="03"
        title={t("nav.game")}
        subtitle={t("fx.subtitle")}
        right={
          <Badge tone={!g.connected ? "off" : f?.pz ? "warn" : "ok"}>
            {!g.connected ? t("fx.gameOff") : f?.pz ? t("fx.gamePaused") : t("fx.gameOn", { hz: g.hz.toFixed(0) })}
          </Badge>
        }
      />

      <div className="grid grid-cols-12 gap-4">
        <Panel code="A" title={t("fx.onWheel")} className="col-span-8">
          <EffectsPanel />
        </Panel>

        <Panel code="B" title={t("fx.forceEngine")} className="col-span-4">
          <SpringControl compact />
          <Note>{t("fx.forceEngineNote")}</Note>
        </Panel>

        <Panel code="C" title={t("fx.link")} className="col-span-4">
          <Indicator label="Mod PXN V99 Telemetry" value={g.connected ? t("common.connected") : t("common.waiting")} tone={g.connected ? "ok" : "warn"} />
          <Indicator label={t("fx.dataRate")} value={g.connected ? `${g.hz.toFixed(0)} Hz` : null} tone={g.hz >= 50 ? "ok" : "accent"} />
          <Indicator label={t("fx.frames")} value={g.connected ? g.frames.toLocaleString() : null} tone="accent" />
          <Indicator label={t("fx.badLines")} value={`${g.bad_lines}`} tone={g.bad_lines ? "warn" : "ok"} />
          <Indicator label={t("fx.inVehicle")} value={live ? yesNo(f?.in === true) : null} tone={f?.in ? "ok" : "off"} />
          <Indicator label={t("fx.motor")} value={inVehicle && f?.on !== undefined ? (f.on ? t("fx.motorOn") : t("fx.motorOff")) : null} tone={f?.on ? "ok" : "off"} />
          {g.last_message && <p className="selectable mt-3 text-xs text-warn">{g.last_message}</p>}
          <RecordButton disabled={!g.connected} />
          {!g.connected && <Note>{t("fx.howToConnect")}</Note>}
        </Panel>

        <Panel code="D" title={t("fx.engineTransmission")} className="col-span-8">
          <div className="grid grid-cols-4 gap-6">
            <Readout label={t("fx.rpm")} value={inVehicle && f?.rpm !== undefined ? f.rpm.toFixed(0) : null} unit="rpm" size="lg" tone="accent" />
            <Readout label={t("fx.rpmRange")} value={inVehicle && f?.rmax ? `${f.rmin?.toFixed(0) ?? "?"}–${f.rmax.toFixed(0)}` : null} unit="rpm" />
            <Readout label={t("shifter.gear")} value={inVehicle && f?.gr !== undefined ? `${f.gr}` : null} />
            <Readout label={t("fx.speed")} value={inVehicle && f?.spd !== undefined ? f.spd.toFixed(1) : null} unit="km/h" />
          </div>
          <div className="mt-6 grid grid-cols-2 gap-x-8 gap-y-4">
            <Bar label={t("fx.rpmInRange")} value={inVehicle ? rpmFrac : null} />
            <Bar label={t("fx.load")} value={inVehicle && f?.ld !== undefined ? f.ld : null} color="var(--color-warn)" />
            <Bar label={t("pedal.throttle")} value={inVehicle && f?.thr !== undefined ? Math.abs(f.thr) : null} color="var(--color-ok)" />
            <Bar label={t("fx.clutchPedal")} value={inVehicle && f?.clt !== undefined ? f.clt : null} color="var(--color-steel)" />
          </div>
        </Panel>

        <Panel code="E" title={t("fx.wheelsTerrain")} className="col-span-8">
          <div className="grid grid-cols-4 gap-3">
            {GROUNDS.map((gr) => {
              const n = inVehicle ? ((f?.[gr.key] as number | undefined) ?? 0) : null;
              const on = n !== null && n > 0;
              return (
                <div key={gr.key} className="border px-3 py-3" style={{ borderColor: on ? gr.color : "var(--color-line)", background: on ? `color-mix(in srgb, ${gr.color} 6%, transparent)` : undefined }}>
                  <div className="label" style={on ? { color: gr.color } : undefined}>
                    {t(gr.label)}
                  </div>
                  <div className="readout mt-1.5 text-2xl">{n === null ? "—" : `${n}`}</div>
                  <div className="text-xs text-dim">{t("fx.wheels")}</div>
                </div>
              );
            })}
          </div>
          <div className="mt-5 grid grid-cols-4 gap-6">
            <Readout label={t("fx.contact")} value={inVehicle && f?.wn !== undefined ? `${f.wc ?? 0}/${f.wn}` : null} />
            <Readout label={t("fx.sink")} value={inVehicle && f?.dep !== undefined ? (f.dep * 100).toFixed(1) : null} unit="cm" />
            <Readout label={t("fx.slip")} value={inVehicle && f?.slp !== undefined ? (f.slp * 100).toFixed(0) : null} unit="%" />
            <Readout
              label={t("fx.frontSusp")}
              value={inVehicle && f?.sfl !== undefined && f?.sfr !== undefined ? `${f.sfl.toFixed(2)} / ${f.sfr.toFixed(2)}` : null}
              unit="m/s"
            />
          </div>
        </Panel>

        <Panel code="F" title={t("fx.signals")} className="col-span-4">
          <Indicator label={t("fx.suspActivity")} value={inVehicle && f?.sus !== undefined ? `${f.sus.toFixed(2)} m/s` : null} tone="accent" />
          <Indicator label={t("fx.clutchSlip")} value={inVehicle && f?.csl !== undefined ? `${(f.csl * 100).toFixed(0)}%` : null} tone="accent" />
          <Indicator label={t("fx.torque")} value={inVehicle && f?.trq !== undefined ? `${(f.trq * 100).toFixed(0)}%` : null} tone="accent" />
          <Indicator label={t("fx.stalling")} value={inVehicle && f?.stl !== undefined ? yesNo(f.stl) : null} tone={f?.stl ? "bad" : "ok"} />
          <Indicator label={t("fx.inWater")} value={inVehicle && f?.wat !== undefined ? `${f.wat}` : null} tone="accent" />
          <Note>{t("fx.signalsNote")}</Note>
        </Panel>
      </div>
    </div>
  );
}
