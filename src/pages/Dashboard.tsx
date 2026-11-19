import { PedalBar } from "../components/Inputs";
import { SpringControl } from "../components/SpringControl";
import { Badge, Button, Indicator, PageHeader, Panel, Readout } from "../components/ui";
import { WheelGauge } from "../components/WheelGauge";
import { hex4 } from "../lib/api";
import { useT } from "../lib/i18n";
import { axisFraction, gameStore, settingsStore, statusStore, steeringNorm, telemetryStore, useStore } from "../lib/store";
import type { PageId } from "../App";

export function Dashboard({ go }: { go: (p: PageId) => void }) {
  const t = useT();
  const status = useStore(statusStore);
  const tel = useStore(telemetryStore);
  const cfg = useStore(settingsStore);
  const game = useStore(gameStore);
  const norm = steeringNorm(tel, status, cfg);
  const angle = norm === null ? null : norm * (cfg.steeringRange / 2);
  const p = cfg.pedals;
  const dinput = status.connected && status.mode === "DirectInput";

  return (
    <div>
      <PageHeader
        code="01"
        title={t("nav.dashboard")}
        subtitle={t("dash.subtitle")}
        right={
          <Badge tone={dinput ? "ok" : status.connected ? "warn" : "bad"}>
            {dinput ? t("dash.onlineDI") : status.connected ? t("dash.connectedXI") : t("common.disconnected")}
          </Badge>
        }
      />

      <div className="grid grid-cols-12 gap-4">
        <Panel code="A" title={t("dash.device")} className="col-span-3">
          <Indicator label={t("dash.connection")} value={status.connected ? t("common.connected") : t("common.disconnected")} tone={status.connected ? "ok" : "bad"} />
          <Indicator label={t("dash.mode")} value={status.mode} tone={status.mode === "DirectInput" ? "ok" : "warn"} />
          <Indicator label={t("dash.model")} value={status.info?.product ?? null} tone="accent" />
          <Indicator label="USB VID:PID" value={status.info ? `${hex4(status.info.vid)}:${hex4(status.info.pid)}` : null} tone="accent" />
          <Indicator label={t("dash.readRate")} value={dinput ? `${status.report_hz.toFixed(0)} Hz` : null} tone="ok" />
        </Panel>

        <Panel code="B" title={t("dash.wheel")} className="col-span-6">
          <div className="flex items-center gap-2">
            <WheelGauge norm={norm} range={cfg.steeringRange} className="mx-auto h-[330px] w-[330px]" />
            <div className="flex w-44 flex-col gap-6">
              <Readout label={t("monitor.angle")} value={angle === null ? null : angle.toFixed(1)} unit="°" size="lg" tone="accent" />
              <Readout label={t("dash.range")} value={`${cfg.steeringRange}`} unit="°" />
              <Readout
                label={t("dash.turn")}
                value={norm === null ? null : `${Math.abs(norm * 100).toFixed(0)}`}
                unit={norm === null ? undefined : norm < -0.005 ? t("dash.pctLeft") : norm > 0.005 ? t("dash.pctRight") : "%"}
              />
              <Button onClick={() => go("monitor")}>{t("dash.openMonitor")}</Button>
            </div>
          </div>
        </Panel>

        <Panel code="C" title={t("dash.pedals")} className="col-span-3">
          <div className="flex justify-around">
            <PedalBar label={t("pedal.clutch")} value={axisFraction(tel, status, p.clutch.usage, p.clutch.invert)} color="var(--color-steel)" />
            <PedalBar label={t("pedal.brake")} value={axisFraction(tel, status, p.brake.usage, p.brake.invert)} color="var(--color-bad)" />
            <PedalBar label={t("pedal.throttle")} value={axisFraction(tel, status, p.throttle.usage, p.throttle.invert)} color="var(--color-ok)" />
          </div>
        </Panel>

        <Panel code="D" title={t("dash.quickCentering")} className="col-span-8" right={<Button onClick={() => go("ffb")}>{t("dash.allSettings")}</Button>}>
          <SpringControl compact />
        </Panel>

        <Panel code="E" title={t("dash.forces")} className="col-span-4">
          <Indicator
            label={t("dash.appCentering")}
            value={status.centering_active ? `${t("common.active")} · ${cfg.centering.strength}%` : t("common.stopped")}
            tone={status.centering_active ? "ok" : "off"}
          />
          <Indicator
            label={t("nav.game")}
            value={cfg.effects.enabled ? `${t("common.on")} · ${cfg.effects.master}%` : t("common.off")}
            tone={cfg.effects.enabled ? "ok" : "off"}
          />
          <Indicator label="Farming Simulator 25" value={game.connected ? `${t("common.connected")} · ${game.hz.toFixed(0)} Hz` : t("common.waiting")} tone={game.connected ? "ok" : "off"} />
          <Button onClick={() => go("game")}>{t("dash.openEffects")}</Button>
        </Panel>
      </div>
    </div>
  );
}
