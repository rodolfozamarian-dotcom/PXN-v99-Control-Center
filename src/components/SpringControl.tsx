// Centralização controlada pelo app: o núcleo lê a posição do volante e comanda
// um efeito PID de força constante. Nada é enviado antes de clicar em "Iniciar".
import { useEffect, useRef, useState } from "react";
import { api, type CenteringParams } from "../lib/api";
import { useT } from "../lib/i18n";
import { settingsStore, statusStore, useStore } from "../lib/store";
import { Badge, Button, Fader, Note } from "./ui";

const SEND_INTERVAL_MS = 50;

type Field = "strength" | "fullAngle" | "damping" | "deadzone";

export function SpringControl({ compact = false }: { compact?: boolean }) {
  const t = useT();
  const status = useStore(statusStore);
  const cfg = useStore(settingsStore);
  const c = cfg.centering;
  const active = status.centering_active;
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const lastSent = useRef(0);
  const trailing = useRef<number | null>(null);

  const available = status.connected && status.mode === "DirectInput";

  useEffect(() => () => void (trailing.current !== null && clearTimeout(trailing.current)), []);

  const toParams = (v: typeof c): CenteringParams => ({
    strength: v.strength,
    full_angle: v.fullAngle,
    damping: v.damping,
    deadzone: v.deadzone,
    half_range: settingsStore.get().steeringRange / 2,
  });

  const push = (v: typeof c, commit: boolean) => {
    if (!active) return; // parado: os controles só preparam os valores
    const send = () => {
      lastSent.current = performance.now();
      api
        .centeringUpdate(toParams(v), commit)
        .then(() => setError(null))
        .catch((e) => setError(String(e)));
    };
    if (trailing.current !== null) clearTimeout(trailing.current);
    trailing.current = null;
    const wait = SEND_INTERVAL_MS - (performance.now() - lastSent.current);
    if (commit || wait <= 0) send();
    else trailing.current = window.setTimeout(send, wait);
  };

  const change = (key: Field, raw: number, min: number, max: number, commit: boolean) => {
    const value = Math.max(min, Math.min(max, Number.isFinite(raw) ? raw : min));
    const next = { ...c, [key]: value };
    settingsStore.update((s) => ({ ...s, centering: next }));
    push(next, commit);
  };

  const start = async () => {
    const polarity = cfg.motorPolarity;
    if (polarity === null && !window.confirm(t("centering.calibrateConfirm"))) return;
    setBusy(true);
    try {
      const used = await api.centeringStart(toParams(c), polarity);
      settingsStore.update((s) => ({ ...s, motorPolarity: used }));
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const stop = () => {
    api.centeringStop().catch((e) => setError(String(e)));
  };

  const stepBtn =
    "mono h-8 w-8 border border-line-strong text-base text-text hover:border-accent hover:text-accent disabled:cursor-not-allowed disabled:opacity-35";

  return (
    <div>
      <div className="mb-4 flex flex-wrap items-center gap-2">
        {!available ? (
          <Badge tone="off">{status.mode === "XInput" ? t("common.unavailableXInput") : t("common.wheelDisconnected")}</Badge>
        ) : active ? (
          <Badge tone="ok">{t("centering.active")}</Badge>
        ) : (
          <Badge tone="off">{t("centering.stopped")}</Badge>
        )}
        {cfg.motorPolarity === null && <Badge tone="accent">{t("centering.notCalibrated")}</Badge>}
      </div>

      <div className="label mb-2">{t("centering.strength")}</div>
      <div className="flex items-center gap-2">
        <button className={stepBtn} disabled={!available} onClick={() => change("strength", c.strength - 1, 0, 100, true)} aria-label="-1%">
          −
        </button>
        <input
          type="range"
          className="slider mx-2 flex-1"
          min={0}
          max={100}
          step={1}
          value={c.strength}
          disabled={!available}
          style={{ ["--fill" as string]: `${c.strength}%` }}
          onChange={(e) => change("strength", Number(e.target.value), 0, 100, false)}
          onPointerUp={(e) => change("strength", Number((e.target as HTMLInputElement).value), 0, 100, true)}
          onKeyUp={(e) => change("strength", Number((e.target as HTMLInputElement).value), 0, 100, true)}
          aria-label={t("centering.strength")}
        />
        <button className={stepBtn} disabled={!available} onClick={() => change("strength", c.strength + 1, 0, 100, true)} aria-label="+1%">
          +
        </button>
        <div className="ml-1 flex h-10 items-baseline border border-line-strong bg-bg px-2 pt-1.5">
          <input
            type="number"
            min={0}
            max={100}
            value={c.strength}
            disabled={!available}
            onChange={(e) => change("strength", Math.round(Number(e.target.value)), 0, 100, true)}
            className={`readout w-14 bg-transparent text-right font-semibold text-text outline-none disabled:text-dim ${compact ? "text-xl" : "text-2xl"}`}
            aria-label={t("centering.strength")}
          />
          <span className="ml-1 text-steel">%</span>
        </div>
      </div>

      {!compact && (
        <div className="mt-6 grid grid-cols-3 gap-6">
          <Fader
            label={t("centering.fullAngle")}
            unit="°"
            min={3}
            max={180}
            value={c.fullAngle}
            disabled={!available}
            hint={t("centering.fullAngleHint")}
            onChange={(v, commit) => change("fullAngle", v, 3, 180, commit)}
          />
          <Fader
            label={t("centering.damping")}
            value={c.damping}
            disabled={!available}
            hint={t("centering.dampingHint")}
            onChange={(v, commit) => change("damping", v, 0, 100, commit)}
          />
          <Fader
            label={t("centering.deadzone")}
            unit="°"
            min={0}
            max={10}
            step={0.5}
            value={c.deadzone}
            disabled={!available}
            hint={t("centering.deadzoneHint")}
            onChange={(v, commit) => change("deadzone", v, 0, 10, commit)}
          />
        </div>
      )}

      <div className="mt-5 flex flex-wrap items-center gap-3">
        {active ? (
          <Button variant="danger" onClick={stop}>
            {t("centering.stop")}
          </Button>
        ) : (
          <Button variant="primary" disabled={!available || busy} onClick={() => void start()}>
            {busy ? t("centering.starting") : t("centering.start")}
          </Button>
        )}
        {!compact && cfg.motorPolarity !== null && (
          <Button disabled={active} onClick={() => settingsStore.update((s) => ({ ...s, motorPolarity: null }))} title={t("centering.recalibrateHint")}>
            {t("centering.recalibrate")}
          </Button>
        )}
      </div>

      {error && (
        <p className="selectable mt-3 text-bad">
          {t("common.failure")}: {error}
        </p>
      )}
      {!compact && <Note>{t("centering.explain")}</Note>}
    </div>
  );
}
