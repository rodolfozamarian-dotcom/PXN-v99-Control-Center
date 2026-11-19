import { useEffect, useState, type ReactNode } from "react";
import { Dot, Segmented, type Tone } from "./components/ui";
import { api } from "./lib/api";
import { useT, type Key } from "./lib/i18n";
import { gameStore, settingsStore, startBridge, statusStore, useStore, type Lang } from "./lib/store";
import { About } from "./pages/About";
import { Calibration } from "./pages/Calibration";
import { Dashboard } from "./pages/Dashboard";
import { Diagnostics } from "./pages/Diagnostics";
import { ForceFeedback } from "./pages/ForceFeedback";
import { GameEffects } from "./pages/GameEffects";
import { LiveMonitor } from "./pages/LiveMonitor";

export type PageId = "dashboard" | "ffb" | "game" | "monitor" | "calibration" | "diagnostics" | "about";

const NAV: { id: PageId; label: Key; icon: ReactNode }[] = [
  { id: "dashboard", label: "nav.dashboard", icon: <path d="M3 3h6v6H3zM11 3h6v4h-6zM11 9h6v8h-6zM3 11h6v6H3z" /> },
  { id: "ffb", label: "nav.ffb", icon: <path d="M10 3v14M5 7l5-4 5 4M5 13l5 4 5-4" /> },
  { id: "game", label: "nav.game", icon: <path d="M2 10h3l2-5 3 10 3-8 2 3h3" /> },
  { id: "monitor", label: "nav.monitor", icon: <path d="M10 3a7 7 0 1 0 0 14 7 7 0 0 0 0-14zM10 3v4M3 10h4M13 10h4" /> },
  { id: "calibration", label: "nav.calibration", icon: <path d="M10 2v4M10 14v4M2 10h4M14 10h4M10 8a2 2 0 1 0 0 4 2 2 0 0 0 0-4z" /> },
  { id: "diagnostics", label: "nav.diagnostics", icon: <path d="M3 4h14M3 8h10M3 12h14M3 16h7" /> },
  { id: "about", label: "nav.about", icon: <path d="M10 3a7 7 0 1 0 0 14 7 7 0 0 0 0-14zM10 9v5M10 6v.5" /> },
];

const LANGS: readonly Lang[] = ["pt", "en", "es"];

function StatusCell({ label, value, tone }: { label: string; value: string; tone: Tone }) {
  return (
    <div className="flex h-full items-center gap-2.5 border-l border-line px-4">
      <Dot tone={tone} />
      <span className="label text-dim">{label}</span>
      <span className="mono text-[11px] uppercase text-text">{value}</span>
    </div>
  );
}

export default function App() {
  const t = useT();
  // A página fica no endereço (#/game etc.): um recarregamento volta à mesma tela.
  const [page, setPageState] = useState<PageId>(() => {
    const h = location.hash.replace("#/", "") as PageId;
    return NAV.some((n) => n.id === h) ? h : "dashboard";
  });
  const setPage = (p: PageId) => {
    setPageState(p);
    history.replaceState(null, "", `#/${p}`);
  };
  const status = useStore(statusStore);
  const game = useStore(gameStore);
  const cfg = useStore(settingsStore);
  const dinput = status.connected && status.mode === "DirectInput";

  useEffect(() => {
    startBridge().catch((e) => console.error("startBridge", e));
  }, []);

  useEffect(() => {
    document.documentElement.lang = cfg.lang === "pt" ? "pt-BR" : cfg.lang;
    api.trayLabels(t("tray.open"), t("tray.quit")).catch(() => undefined);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [cfg.lang]);

  // Ao voltar para a janela, força um redesenho completo da tela.
  useEffect(() => {
    let off: (() => void) | undefined;
    api
      .onUiWake(() => {
        const root = document.documentElement;
        root.style.opacity = "0.999";
        requestAnimationFrame(() => requestAnimationFrame(() => (root.style.opacity = "")));
      })
      .then((u) => (off = u))
      .catch(() => undefined);
    return () => off?.();
  }, []);

  return (
    <div className="flex h-full">
      <aside className="flex w-60 shrink-0 flex-col border-r border-line bg-panel">
        <div className="flex h-16 flex-col justify-center border-b border-line px-5">
          <div className="font-display text-[21px] font-bold leading-none tracking-[0.08em]">
            PXN <span className="text-accent">V99</span>
          </div>
          <div className="label mt-1.5 text-[9.5px]">Control Center</div>
        </div>

        <nav className="flex-1 py-2">
          {NAV.map((n, i) => {
            const active = page === n.id;
            return (
              <button
                key={n.id}
                onClick={() => setPage(n.id)}
                className={`relative flex h-10 w-full items-center gap-3 px-5 text-left transition-colors ${
                  active ? "bg-raised text-text" : "text-steel hover:bg-raised/50 hover:text-text"
                }`}
              >
                {active && <span className="absolute left-0 top-0 h-full w-[2px] bg-accent" />}
                <span className={`mono w-5 text-[10px] ${active ? "text-accent" : "text-dim"}`}>{String(i + 1).padStart(2, "0")}</span>
                <svg viewBox="0 0 20 20" className="h-4 w-4 shrink-0" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="square">
                  {n.icon}
                </svg>
                <span className="font-display text-[12.5px] uppercase tracking-[0.09em]">{t(n.label)}</span>
              </button>
            );
          })}
        </nav>

        <div className="border-t border-line px-5 py-4">
          <div className="label mb-2 text-dim">{t("app.language")}</div>
          <Segmented options={LANGS} value={cfg.lang} format={(l) => l.toUpperCase()} onChange={(lang) => settingsStore.update((s) => ({ ...s, lang }))} />
        </div>
      </aside>

      <div className="flex min-w-0 flex-1 flex-col">
        <div className="flex h-10 shrink-0 items-center justify-between border-b border-line bg-panel">
          <div className="mono flex h-full items-center px-5 text-[11px] text-dim">
            {status.info ? `${status.info.product} · SN ${status.info.serial || "—"}` : t("app.searching")}
          </div>
          <div className="flex h-full">
            <StatusCell
              label="USB"
              value={dinput ? `${status.report_hz.toFixed(0)} Hz` : status.connected ? "XInput" : "OFF"}
              tone={dinput ? "ok" : status.connected ? "warn" : "bad"}
            />
            <StatusCell label="FFB" value={status.centering_active ? "ON" : "OFF"} tone={status.centering_active ? "ok" : "off"} />
            <StatusCell
              label="FS25"
              value={game.connected ? `${game.hz.toFixed(0)} Hz` : "OFF"}
              tone={game.connected ? "ok" : "off"}
            />
            <StatusCell label="FX" value={cfg.effects.enabled ? "ON" : "OFF"} tone={cfg.effects.enabled ? "accent" : "off"} />
          </div>
        </div>

        <main className="workspace flex-1 overflow-y-auto px-7 py-6">
          <div className="mx-auto max-w-[1500px]">
            {page === "dashboard" && <Dashboard go={setPage} />}
            {page === "ffb" && <ForceFeedback />}
            {page === "game" && <GameEffects />}
            {page === "monitor" && <LiveMonitor />}
            {page === "calibration" && <Calibration />}
            {page === "diagnostics" && <Diagnostics />}
            {page === "about" && <About />}
          </div>
        </main>
      </div>
    </div>
  );
}
