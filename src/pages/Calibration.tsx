// Calibração de centro.
//
// O V99 calibra o centro sozinho ao ser ligado: gira até 450° para cada lado e
// volta ao centro (manual PXN). Não há comando USB documentado para disparar
// isso, então o app (1) mede o desvio do centro com o aro reto e (2) guia a
// recalibração pelo próprio volante, acompanhando cada etapa pelos dados reais.
import { useEffect, useRef, useState } from "react";
import { WheelGauge } from "../components/WheelGauge";
import { Badge, Button, Note, PageHeader, Panel, Readout, type Tone } from "../components/ui";
import { useT, type Key } from "../lib/i18n";
import { settingsStore, statusStore, steeringNorm, telemetryStore, useStore } from "../lib/store";

const CHECK_MS = 1500;
const SWEEP_DEG = 90; // excursão que indica a varredura automática
const SETTLED_DEG = 2;
const SETTLE_MS = 1500;
const GUIDE_TIMEOUT_MS = 120_000;

type CheckResult = { mean: number; spread: number } | null;
type Phase = "idle" | "waiting" | "off" | "sweeping" | "settling" | "done" | "timeout";

/** Ângulo atual em graus a partir do centro do volante, ou null. */
function useAngle() {
  const status = useStore(statusStore);
  const tel = useStore(telemetryStore);
  const cfg = useStore(settingsStore);
  const norm = steeringNorm(tel, status, cfg);
  return { norm, angle: norm === null ? null : norm * (cfg.steeringRange / 2), range: cfg.steeringRange, raw: tel.axes[0] ?? null };
}

function CenterCheck() {
  const t = useT();
  const status = useStore(statusStore);
  const { angle } = useAngle();
  const [running, setRunning] = useState(false);
  const [result, setResult] = useState<CheckResult>(null);
  const samples = useRef<number[]>([]);
  const angleRef = useRef(angle);
  angleRef.current = angle;

  const available = status.connected && status.mode === "DirectInput";
  const blocked = status.centering_active;

  const run = () => {
    samples.current = [];
    setResult(null);
    setRunning(true);
    const id = window.setInterval(() => {
      if (angleRef.current !== null) samples.current.push(angleRef.current);
    }, 20);
    window.setTimeout(() => {
      clearInterval(id);
      setRunning(false);
      const s = samples.current;
      if (s.length === 0) return setResult(null);
      const mean = s.reduce((a, b) => a + b, 0) / s.length;
      const spread = Math.max(...s) - Math.min(...s);
      setResult({ mean, spread });
    }, CHECK_MS);
  };

  let verdict: { tone: Tone; text: string } | null = null;
  if (result) {
    const dev = Math.abs(result.mean);
    const side = result.mean > 0 ? t("calib.toRight") : t("calib.toLeft");
    if (result.spread > 3) verdict = { tone: "warn", text: t("calib.moved") };
    else if (dev < 1) verdict = { tone: "ok", text: t("calib.centerOk", { dev: dev.toFixed(1) }) };
    else if (dev < 3) verdict = { tone: "warn", text: t("calib.centerSmall", { dev: dev.toFixed(1), side }) };
    else verdict = { tone: "bad", text: t("calib.centerOff", { dev: dev.toFixed(1), side }) };
  }

  return (
    <div>
      <ol className="space-y-2 text-steel">
        <li>
          <span className="mono mr-2 text-accent">1</span>
          {t("calib.check1")}
        </li>
        <li>
          <span className="mono mr-2 text-accent">2</span>
          {t("calib.check2")}
        </li>
      </ol>
      <div className="mt-4 flex flex-wrap items-center gap-3">
        <Button variant="primary" disabled={!available || blocked || running} onClick={run}>
          {running ? t("calib.measuring") : t("calib.check")}
        </Button>
        {verdict && <Badge tone={verdict.tone}>{verdict.text}</Badge>}
      </div>
      {blocked && <Note tone="warn">{t("calib.stopCentering")}</Note>}
      {result && (
        <div className="mt-4 grid grid-cols-2 gap-6">
          <Readout label={t("calib.deviation")} value={result.mean.toFixed(2)} unit="°" />
          <Readout label={t("calib.stability")} value={result.spread.toFixed(2)} unit="°" />
        </div>
      )}
    </div>
  );
}

function GuidedRecalibration() {
  const t = useT();
  const status = useStore(statusStore);
  const { angle } = useAngle();
  const [phase, setPhase] = useState<Phase>("idle");
  const [finalAngle, setFinalAngle] = useState<number | null>(null);
  const started = useRef(0);
  const settledSince = useRef<number | null>(null);

  // Avança as etapas a partir do estado real do volante.
  useEffect(() => {
    if (phase === "idle" || phase === "done" || phase === "timeout") return;
    if (Date.now() - started.current > GUIDE_TIMEOUT_MS) return setPhase("timeout");
    const a = angle ?? 0;
    if (phase === "waiting") {
      if (!status.connected) setPhase("off");
      else if (Math.abs(a) > SWEEP_DEG) setPhase("sweeping");
    } else if (phase === "off") {
      if (status.connected && angle !== null) setPhase(Math.abs(a) > SWEEP_DEG ? "sweeping" : "settling");
    } else if (phase === "sweeping") {
      if (Math.abs(a) < SETTLED_DEG) setPhase("settling");
    } else if (phase === "settling") {
      if (angle === null || Math.abs(a) >= SETTLED_DEG) {
        settledSince.current = null;
        if (Math.abs(a) > SWEEP_DEG) setPhase("sweeping");
      } else if (settledSince.current === null) {
        settledSince.current = Date.now();
      } else if (Date.now() - settledSince.current >= SETTLE_MS) {
        setFinalAngle(a);
        setPhase("done");
      }
    }
  }, [angle, status.connected, phase]);

  // Relógio para o tempo limite e para a etapa "settling" com o volante parado.
  const [, tick] = useState(0);
  useEffect(() => {
    if (phase === "idle" || phase === "done" || phase === "timeout") return;
    const id = window.setInterval(() => tick((n) => n + 1), 250);
    return () => clearInterval(id);
  }, [phase]);

  const start = () => {
    started.current = Date.now();
    settledSince.current = null;
    setFinalAngle(null);
    setPhase("waiting");
  };

  const steps: { key: Key; active: boolean; done: boolean }[] = [
    { key: "calib.step1", active: phase === "waiting", done: ["off", "sweeping", "settling", "done"].includes(phase) },
    { key: "calib.step2", active: phase === "off", done: ["sweeping", "settling", "done"].includes(phase) },
    { key: "calib.step3", active: phase === "sweeping" || phase === "settling", done: phase === "done" },
  ];

  return (
    <div>
      <p className="text-steel">{t("calib.guideIntro")}</p>
      <ol className="mt-4 space-y-2">
        {steps.map((s, i) => (
          <li key={s.key} className="flex items-start gap-3 border px-3 py-2.5" style={{ borderColor: s.active ? "var(--color-accent)" : "var(--color-line)" }}>
            <span className={`mono mt-px text-xs ${s.done ? "text-ok" : s.active ? "text-accent" : "text-dim"}`}>{s.done ? "OK" : String(i + 1).padStart(2, "0")}</span>
            <span className={s.active ? "text-text" : s.done ? "text-steel" : "text-dim"}>{t(s.key)}</span>
          </li>
        ))}
      </ol>
      <div className="mt-4 flex flex-wrap items-center gap-3">
        {phase === "idle" || phase === "done" || phase === "timeout" ? (
          <Button variant="primary" onClick={start}>
            {t("calib.startGuide")}
          </Button>
        ) : (
          <Button onClick={() => setPhase("idle")}>{t("calib.cancel")}</Button>
        )}
        {phase === "sweeping" && <Badge tone="warn">{t("calib.dontTouch")}</Badge>}
        {phase === "settling" && <Badge tone="accent">{t("calib.settling")}</Badge>}
        {phase === "done" && finalAngle !== null && <Badge tone={Math.abs(finalAngle) < 1 ? "ok" : "warn"}>{t("calib.done", { dev: finalAngle.toFixed(1) })}</Badge>}
        {phase === "timeout" && <Badge tone="bad">{t("calib.timeout")}</Badge>}
      </div>
      <Note>{t("calib.guideNote")}</Note>
    </div>
  );
}

export function Calibration() {
  const t = useT();
  const status = useStore(statusStore);
  const { norm, angle, range, raw } = useAngle();
  const centered = angle !== null && Math.abs(angle) < 0.5;

  return (
    <div>
      <PageHeader code="05" title={t("nav.calibration")} subtitle={t("calib.subtitle")} />
      <div className="grid grid-cols-12 gap-4">
        <Panel code="A" title={t("calib.live")} className="col-span-4">
          <WheelGauge norm={norm} range={range} className="mx-auto h-[260px] w-[260px]" />
          <div className="mt-3 grid grid-cols-2 gap-6">
            <Readout label={t("monitor.angle")} value={angle === null ? null : angle.toFixed(1)} unit="°" size="lg" tone={centered ? "ok" : "accent"} />
            <Readout label={t("calib.raw")} value={status.connected && raw !== null ? `${raw}` : null} />
          </div>
        </Panel>

        <Panel code="B" title={t("calib.checkTitle")} className="col-span-8">
          <CenterCheck />
        </Panel>

        <Panel code="C" title={t("calib.guideTitle")} className="col-span-12">
          <GuidedRecalibration />
        </Panel>
      </div>
    </div>
  );
}
