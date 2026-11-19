import { useState } from "react";
import { DeviceControlPanel } from "../components/DeviceControl";
import { Button, Indicator, Note, PageHeader, Panel } from "../components/ui";
import { hex4 } from "../lib/api";
import { useT } from "../lib/i18n";
import { logStore, statusStore, useStore } from "../lib/store";
import { APP_VERSION } from "../lib/version";

const levelColor = { info: "var(--color-steel)", warn: "var(--color-warn)", error: "var(--color-bad)" } as const;

export function Diagnostics() {
  const t = useT();
  const status = useStore(statusStore);
  const log = useStore(logStore);
  const [copied, setCopied] = useState<string | null>(null);
  const info = status.info;

  // Relatório técnico: apenas dados do dispositivo e eventos; nenhum dado pessoal.
  const copyReport = async () => {
    const report = {
      generated_at: new Date().toISOString(),
      app: `PXN V99 Control Center ${APP_VERSION}`,
      connected: status.connected,
      mode: status.mode,
      device: info && {
        model: info.product,
        manufacturer: info.manufacturer,
        usb_vid: hex4(info.vid),
        usb_pid: hex4(info.pid),
        bcd_device: hex4(info.revision),
        driver: "HidUsb + pid.dll (Microsoft HID PID)",
        reports: { input: info.input_len, output: info.output_len, feature: info.feature_len },
        axes: info.axes,
        buttons: info.button_count,
        hat: info.has_hat,
        features: { pid_state: info.has_pid_state, device_gain: info.has_device_gain, pid_pool: info.pid_pool },
      },
      counters: { errors: status.error_count, reconnects: status.reconnects, reports: status.total_reports },
      last_error: status.last_error,
      events: log.map((l) => ({ at: new Date(l.ts).toISOString(), level: l.level, category: l.category, message: l.message })),
    };
    try {
      await navigator.clipboard.writeText(JSON.stringify(report, null, 2));
      setCopied(t("diag.copied"));
    } catch (e) {
      setCopied(`${t("diag.copyFailed")}: ${String(e)}`);
    }
    setTimeout(() => setCopied(null), 4000);
  };

  return (
    <div>
      <PageHeader
        code="06"
        title={t("nav.diagnostics")}
        subtitle={t("diag.subtitle")}
        right={
          <div className="flex items-center gap-3">
            {copied && <span className="text-steel">{copied}</span>}
            <Button onClick={() => void copyReport()}>{t("diag.copyReport")}</Button>
          </div>
        }
      />

      <div className="grid grid-cols-12 gap-4">
        <Panel code="A" title={t("diag.hardware")} className="col-span-5">
          <Indicator label={t("dash.model")} value={info?.product ?? null} tone="accent" />
          <Indicator label={t("diag.manufacturer")} value={info?.manufacturer ?? null} tone="accent" />
          <Indicator label="USB VID:PID" value={info ? `${hex4(info.vid)}:${hex4(info.pid)}` : null} tone="accent" />
          <Indicator label={t("diag.revision")} value={info ? hex4(info.revision) : null} tone="accent" />
          <Indicator label="Driver" value={info ? "HidUsb + pid.dll" : null} tone="ok" />
          <Indicator label={t("diag.axesButtons")} value={info ? `${info.axes.length} / ${info.button_count}` : null} tone="accent" />
          <Indicator label={t("diag.reportSizes")} value={info ? `${info.input_len} / ${info.output_len} / ${info.feature_len} B` : null} tone="accent" />
          <Indicator label="PID State" value={info ? (info.has_pid_state ? t("diag.supported") : t("diag.absent")) : null} tone={info?.has_pid_state ? "ok" : "warn"} />
          <Indicator label={t("diag.lastError")} value={status.last_error} tone="bad" />
          <Note>{t("diag.firmwareNote")}</Note>
        </Panel>

        <Panel code="B" title={t("diag.events", { n: log.length })} className="col-span-7">
          <div className="selectable mono h-[430px] overflow-y-auto pr-1 text-[11.5px] leading-relaxed">
            {log.length === 0 && <p className="text-dim">{t("diag.noEvents")}</p>}
            {[...log].reverse().map((l) => (
              <div key={l.id} className="flex gap-3 border-b border-line/50 py-1.5">
                <span className="shrink-0 text-dim">{new Date(l.ts).toLocaleTimeString()}</span>
                <span className="w-11 shrink-0 uppercase" style={{ color: levelColor[l.level] }}>
                  {l.level}
                </span>
                <span className="w-9 shrink-0 text-accent">{l.category}</span>
                <span className="text-text">{l.message}</span>
              </div>
            ))}
          </div>
          <Note>{t("diag.logNote")}</Note>
        </Panel>

        <Panel code="C" title={t("diag.deviceControl")} className="col-span-7">
          <DeviceControlPanel />
        </Panel>
      </div>
    </div>
  );
}
