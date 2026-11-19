// Comandos PID "Device Control" e teste de vibração, para diagnóstico.
import { useState } from "react";
import { api } from "../lib/api";
import { useT, type Key } from "../lib/i18n";
import { statusStore, useStore } from "../lib/store";
import { Button, Note } from "./ui";

const CONTROLS: { cmd: "enable" | "disable" | "stop_all" | "reset"; label: string; note: Key }[] = [
  { cmd: "enable", label: "Enable Actuators", note: "dc.enable" },
  { cmd: "disable", label: "Disable Actuators", note: "dc.disable" },
  { cmd: "stop_all", label: "Stop All Effects", note: "dc.stopAll" },
  { cmd: "reset", label: "Device Reset", note: "dc.reset" },
];

export function DeviceControlPanel() {
  const t = useT();
  const status = useStore(statusStore);
  const available = status.connected && status.mode === "DirectInput";
  const [last, setLast] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const run = async (cmd: (typeof CONTROLS)[number]["cmd"], label: string) => {
    try {
      await api.deviceControl(cmd);
      setLast(`${label}: ${t("dc.sent")}`);
    } catch (e) {
      setLast(`${label}: ${t("common.failure")} (${String(e)})`);
    }
  };

  const vibrate = () => {
    if (!window.confirm(t("dc.vibrationConfirm"))) return;
    setBusy(true);
    api
      .vibrationTest()
      .then(() => setLast(`${t("dc.vibration")}: ${t("dc.sent")}`))
      .catch((e) => setLast(`${t("dc.vibration")}: ${t("common.failure")} (${String(e)})`))
      .finally(() => setBusy(false));
  };

  return (
    <div>
      <div className="grid grid-cols-2 gap-x-4 gap-y-3">
        {CONTROLS.map((c) => (
          <div key={c.cmd}>
            <Button disabled={!available} onClick={() => void run(c.cmd, c.label)}>
              {c.label}
            </Button>
            <p className="mt-1.5 text-xs text-dim">{t(c.note)}</p>
          </div>
        ))}
        <div className="col-span-2 border-t border-line pt-3">
          <Button disabled={!available || busy} onClick={vibrate}>
            {t("dc.vibration")}
          </Button>
          <p className="mt-1.5 text-xs text-dim">{t("dc.vibrationNote")}</p>
        </div>
      </div>
      <p className="selectable mono mt-3 text-xs text-steel">{last ?? t("dc.none")}</p>
      <Note>{t("dc.footer")}</Note>
    </div>
  );
}
