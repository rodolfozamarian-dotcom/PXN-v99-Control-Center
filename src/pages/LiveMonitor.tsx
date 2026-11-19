import { ButtonGrid, HatIndicator, PedalBar } from "../components/Inputs";
import { Shifter } from "../components/Shifter";
import { Indicator, Note, PageHeader, Panel, Readout, Segmented } from "../components/ui";
import { WheelGauge } from "../components/WheelGauge";
import { useT, type Key } from "../lib/i18n";
import { axisFraction, settingsStore, statusStore, steeringNorm, steerRateStore, telemetryStore, useStore, type PedalKey } from "../lib/store";

const RANGES = [270, 360, 540, 900, 1080] as const;
const PEDALS: { key: PedalKey; label: Key; color: string }[] = [
  { key: "clutch", label: "pedal.clutch", color: "var(--color-steel)" },
  { key: "brake", label: "pedal.brake", color: "var(--color-bad)" },
  { key: "throttle", label: "pedal.throttle", color: "var(--color-ok)" },
];
// Embreagens de alavanca atrás do volante, abaixo das borboletas de marcha.
const HAND_CLUTCHES: { key: PedalKey; label: Key; color: string }[] = [
  { key: "clutchLeft", label: "pedal.clutchLeft", color: "var(--color-accent)" },
  { key: "clutchRight", label: "pedal.clutchRight", color: "var(--color-accent)" },
];

export function LiveMonitor() {
  const t = useT();
  const status = useStore(statusStore);
  const tel = useStore(telemetryStore);
  const cfg = useStore(settingsStore);
  const rate = useStore(steerRateStore);

  const info = status.info;
  const dinput = status.connected && status.mode === "DirectInput";
  const norm = steeringNorm(tel, status, cfg);
  const half = cfg.steeringRange / 2;
  const angle = norm === null ? null : norm * half;
  const degPerSec = rate * half;
  const centered = norm !== null && Math.abs(norm) < 0.004;
  const moving = Math.abs(degPerSec) > 2;
  const direction = norm === null ? null : !moving ? t("monitor.still") : degPerSec < 0 ? t("monitor.toLeft") : t("monitor.toRight");

  const setPedal = (key: PedalKey, patch: Partial<{ usage: number | null; invert: boolean }>) =>
    settingsStore.update((s) => ({ ...s, pedals: { ...s.pedals, [key]: { ...s.pedals[key], ...patch } } }));

  return (
    <div>
      <PageHeader code="04" title={t("nav.monitor")} subtitle={t("monitor.subtitle")} />

      <div className="grid grid-cols-12 gap-4">
        <Panel
          code="A"
          title={t("monitor.steering")}
          className="col-span-8"
          right={
            <div className="flex items-center gap-3">
              <span className="label text-dim">{t("monitor.scale")}</span>
              <Segmented options={RANGES} value={cfg.steeringRange as (typeof RANGES)[number]} format={(r) => `${r}°`} onChange={(r) => settingsStore.update((s) => ({ ...s, steeringRange: r }))} />
            </div>
          }
        >
          <div className="flex items-center gap-6">
            <WheelGauge norm={norm} range={cfg.steeringRange} className="h-[390px] w-[390px] shrink-0" />
            <div className="grid flex-1 grid-cols-2 gap-x-6 gap-y-7">
              <div className="col-span-2">
                <Readout label={t("monitor.angle")} value={angle === null ? null : angle.toFixed(1)} unit="°" size="xl" tone="accent" />
              </div>
              <Readout label={t("monitor.left")} value={angle === null ? null : (angle < 0 ? -angle : 0).toFixed(1)} unit="°" />
              <Readout label={t("monitor.right")} value={angle === null ? null : (angle > 0 ? angle : 0).toFixed(1)} unit="°" />
              <Readout label={t("monitor.turnPct")} value={norm === null ? null : (norm * 100).toFixed(1)} unit="%" />
              <Readout label={t("monitor.angularSpeed")} value={norm === null ? null : degPerSec.toFixed(0)} unit="°/s" />
              <Readout label={t("monitor.motion")} value={direction} />
              <Readout label={t("monitor.centering")} value={norm === null ? null : centered ? t("monitor.atCenter") : t("monitor.offCenter")} tone={centered ? "ok" : undefined} />
            </div>
          </div>
          <Note>{t("monitor.scaleNote")}</Note>
        </Panel>

        <Panel code="B" title={t("monitor.connection")} className="col-span-4">
          <Indicator label="USB" value={status.connected ? t("common.connected") : t("common.disconnected")} tone={status.connected ? "ok" : "bad"} />
          <Indicator label="HID" value={dinput ? t("monitor.hidShared") : status.connected ? t("monitor.hidNoXInput") : null} tone={dinput ? "ok" : "warn"} />
          <Indicator label={t("dash.mode")} value={status.mode} tone={status.mode === "DirectInput" ? "ok" : "warn"} />
          <Indicator
            label="Force Feedback"
            value={tel.pid ? (tel.pid.actuators_enabled ? (tel.pid.effect_playing ? t("monitor.ffbEffect") : t("ffb.enabled")) : t("ffb.disabled")) : null}
            tone={tel.pid?.actuators_enabled ? "ok" : "warn"}
          />
          <Indicator label={t("monitor.updateRate")} value={dinput ? `${status.report_hz.toFixed(0)} Hz` : null} tone={status.report_hz >= 60 ? "ok" : "accent"} />
          <Indicator label={t("monitor.reports")} value={dinput ? status.total_reports.toLocaleString() : null} tone="accent" />
          <Indicator label={t("monitor.errors")} value={`${status.error_count}`} tone={status.error_count ? "bad" : "ok"} />
          <Indicator label={t("monitor.reconnects")} value={`${status.reconnects}`} tone={status.reconnects ? "warn" : "ok"} />
          <Note>{t("monitor.rateNote")}</Note>
        </Panel>

        <Panel code="C" title={t("monitor.pedalsAndClutches")} className="col-span-5">
          <div className="flex justify-around">
            {[...PEDALS, ...HAND_CLUTCHES].map(({ key, label, color }, i) => (
              <div key={key} className={`flex flex-col items-center gap-2 ${i === PEDALS.length ? "border-l border-line pl-4" : ""}`}>
                <PedalBar label={t(label)} value={axisFraction(tel, status, cfg.pedals[key].usage, cfg.pedals[key].invert)} color={color} />
                <select
                  value={cfg.pedals[key].usage ?? ""}
                  onChange={(e) => setPedal(key, { usage: e.target.value === "" ? null : Number(e.target.value) })}
                  className="mono h-7 w-[84px] border border-line-strong bg-raised px-1 text-[11px] text-text outline-none"
                  aria-label={t(label)}
                >
                  <option value="">{t("monitor.none")}</option>
                  {(info?.axes ?? []).map((a) => (
                    <option key={a.usage} value={a.usage}>
                      {t("monitor.axis")} {a.name}
                    </option>
                  ))}
                </select>
                <label className="flex cursor-pointer items-center gap-1.5 text-xs text-steel">
                  <input type="checkbox" checked={cfg.pedals[key].invert} onChange={(e) => setPedal(key, { invert: e.target.checked })} className="accent-[var(--color-accent)]" />
                  {t("monitor.invert")}
                </label>
              </div>
            ))}
          </div>
          <Note>{t("monitor.pedalsNote")}</Note>
        </Panel>

        <Panel code="D" title={t("monitor.shifter")} className="col-span-7">
          <Shifter />
        </Panel>

        <Panel code="E" title={t("monitor.rawAxes")} className="col-span-3">
          {info?.axes.length ? (
            <div className="space-y-3">
              {info.axes.map((a, i) => {
                const raw = tel.axes[i];
                const f = raw === undefined ? 0 : (raw - a.min) / (a.max - a.min);
                return (
                  <div key={a.usage}>
                    <div className="flex justify-between text-xs">
                      <span className="mono text-steel">{a.name}</span>
                      <span className="mono text-text">{raw ?? "—"}</span>
                    </div>
                    <div className="mt-1 h-1 bg-bg">
                      <div className="h-full bg-accent" style={{ width: `${Math.max(0, Math.min(1, f)) * 100}%` }} />
                    </div>
                  </div>
                );
              })}
            </div>
          ) : (
            <p className="text-dim">{t("common.na")}</p>
          )}
        </Panel>

        <Panel code="F" title={t("monitor.buttons", { n: info?.button_count ?? 0 })} className="col-span-9">
          <div className="flex items-start gap-8">
            <div className="flex-1">
              <ButtonGrid count={info?.button_count ?? 0} pressed={tel.buttons} columns={8} />
            </div>
            <HatIndicator hat={tel.hat} available={!!info?.has_hat} />
            <div className="w-40">
              <div className="label">{t("monitor.pressed")}</div>
              <div className="mono mt-1 text-sm">{tel.buttons.length ? tel.buttons.join(" · ") : t("monitor.none")}</div>
            </div>
          </div>
          <Note>{t("monitor.buttonsNote")}</Note>
        </Panel>
      </div>
    </div>
  );
}
