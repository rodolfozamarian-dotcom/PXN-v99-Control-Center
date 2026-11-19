import { SpringControl } from "../components/SpringControl";
import { Badge, Indicator, Note, PageHeader, Panel } from "../components/ui";
import { useT } from "../lib/i18n";
import { statusStore, telemetryStore, useStore } from "../lib/store";

export function ForceFeedback() {
  const t = useT();
  const status = useStore(statusStore);
  const tel = useStore(telemetryStore);
  const pool = status.info?.pid_pool;
  const yesNo = (v: boolean) => (v ? t("common.yes") : t("common.no"));

  return (
    <div>
      <PageHeader
        code="02"
        title={t("nav.ffb")}
        subtitle={t("ffb.subtitle")}
        right={<Badge tone={status.mode === "DirectInput" ? "ok" : "off"}>{status.mode === "DirectInput" ? t("ffb.channelOpen") : t("ffb.noChannel")}</Badge>}
      />

      <div className="grid grid-cols-12 gap-4">
        <Panel code="A" title={t("ffb.centering")} className="col-span-8">
          <SpringControl />
        </Panel>

        <Panel code="B" title={t("ffb.reported")} className="col-span-4">
          <Indicator
            label={t("ffb.actuators")}
            value={tel.pid ? (tel.pid.actuators_enabled ? t("ffb.enabled") : t("ffb.disabled")) : null}
            tone={tel.pid?.actuators_enabled ? "ok" : "warn"}
          />
          <Indicator label={t("ffb.effectPlaying")} value={tel.pid ? yesNo(tel.pid.effect_playing) : null} tone={tel.pid?.effect_playing ? "ok" : "off"} />
          <Indicator label={t("ffb.paused")} value={tel.pid ? yesNo(tel.pid.paused) : null} tone={tel.pid?.paused ? "warn" : "ok"} />
          <Indicator label={t("ffb.maxEffects")} value={pool ? `${pool.simultaneous_effects_max}` : null} tone="accent" />
          <Indicator label={t("ffb.effectMemory")} value={pool ? `${pool.ram_pool_size} B` : null} tone="accent" />
          <Note>{t("ffb.reportedNote")}</Note>
        </Panel>
      </div>
    </div>
  );
}
