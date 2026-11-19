// Câmbio em H: mostra a marcha engatada, a chave Low/High e o botão P a partir
// dos botões reais. Clique em uma posição e acione-a no câmbio para ensinar
// qual botão ela usa.
import { useEffect, useRef, useState } from "react";
import { useT } from "../lib/i18n";
import { settingsStore, telemetryStore, useStore, type GearKey } from "../lib/store";

const W = 300;
const H = 268;
const LANES = [50, 117, 184, 251];
const TOP = 38;
const BOTTOM = 182;
const MID = (TOP + BOTTOM) / 2;
const RANGE_Y = 236;

const GEARS: { gear: GearKey; x: number; y: number }[] = [
  { gear: "1", x: LANES[0], y: TOP },
  { gear: "2", x: LANES[0], y: BOTTOM },
  { gear: "3", x: LANES[1], y: TOP },
  { gear: "4", x: LANES[1], y: BOTTOM },
  { gear: "5", x: LANES[2], y: TOP },
  { gear: "6", x: LANES[2], y: BOTTOM },
  { gear: "R", x: LANES[3], y: BOTTOM },
];
const LEGEND: { key: GearKey; label: string }[] = [
  ...GEARS.map((g) => ({ key: g.gear, label: g.gear as string })),
  { key: "LH", label: "L/H" },
  { key: "P", label: "P" },
];

export function Shifter() {
  const t = useT();
  const tel = useStore(telemetryStore);
  const cfg = useStore(settingsStore);
  const [learning, setLearning] = useState<GearKey | null>(null);
  const baseline = useRef<number[]>([]);

  const pressed = (key: GearKey) => cfg.shifter[key] !== null && tel.buttons.includes(cfg.shifter[key] as number);
  const active = GEARS.find((s) => pressed(s.gear)) ?? null;
  const parked = pressed("P");
  const rangeMapped = cfg.shifter.LH !== null;
  const high = pressed("LH") !== cfg.rangeInverted;

  // Modo de aprendizado: o primeiro botão novo pressionado vira o botão da posição.
  useEffect(() => {
    if (!learning) return;
    const fresh = tel.buttons.find((b) => !baseline.current.includes(b));
    if (fresh !== undefined) {
      settingsStore.update((s) => ({ ...s, shifter: { ...s.shifter, [learning]: fresh } }));
      setLearning(null);
    }
  }, [tel.buttons, learning]);

  const startLearning = (key: GearKey) => {
    if (learning === key) return setLearning(null);
    baseline.current = [...tel.buttons];
    setLearning(key);
  };

  const ring = (key: GearKey, on: boolean, mapped: boolean, onColor: string) =>
    learning === key ? "var(--color-warn)" : on ? onColor : mapped ? "var(--color-line-strong)" : "var(--color-line)";
  const pulse = <animate attributeName="opacity" values="1;0.4;1" dur="0.9s" repeatCount="indefinite" />;
  const neutral = { x: (LANES[1] + LANES[2]) / 2, y: MID };

  return (
    <div>
      <div className="flex items-center gap-5">
        <svg viewBox={`0 0 ${W} ${H}`} className="h-[268px] w-[300px] shrink-0">
          <rect x="6" y="6" width={W - 12} height={H - 12} rx="2" fill="var(--color-raised)" stroke="var(--color-line-strong)" />
          {/* Trilhos do H */}
          <g stroke="#07090b" strokeWidth="16" strokeLinecap="round" fill="none">
            <line x1={LANES[0]} y1={MID} x2={LANES[3]} y2={MID} />
            {LANES.slice(0, 3).map((x) => (
              <line key={x} x1={x} y1={TOP} x2={x} y2={BOTTOM} />
            ))}
            <line x1={LANES[3]} y1={MID} x2={LANES[3]} y2={BOTTOM} />
          </g>

          {GEARS.map((s) => {
            const on = active?.gear === s.gear;
            const mapped = cfg.shifter[s.gear] !== null;
            return (
              <g key={s.gear} onClick={() => startLearning(s.gear)} className="cursor-pointer">
                <circle
                  cx={s.x}
                  cy={s.y}
                  r="17"
                  fill={on ? "var(--color-accent)" : "var(--color-panel)"}
                  stroke={ring(s.gear, on, mapped, "var(--color-accent)")}
                  strokeWidth={learning === s.gear ? 3 : 1.5}
                  strokeDasharray={mapped || learning === s.gear ? undefined : "3 3"}
                >
                  {learning === s.gear && pulse}
                </circle>
                <text
                  x={s.x}
                  y={s.y + 6}
                  textAnchor="middle"
                  fontSize="17"
                  fontWeight="600"
                  fontFamily="var(--font-display)"
                  fill={on ? "var(--color-bg)" : mapped ? "var(--color-text)" : "var(--color-dim)"}
                  pointerEvents="none"
                >
                  {s.gear}
                </text>
              </g>
            );
          })}

          {/* Botão P (freio de estacionamento), acima da ré */}
          <g onClick={() => startLearning("P")} className="cursor-pointer">
            <rect
              x={LANES[3] - 19}
              y={TOP - 17}
              width="38"
              height="34"
              rx="6"
              fill={parked ? "var(--color-bad)" : "var(--color-panel)"}
              stroke={ring("P", parked, cfg.shifter.P !== null, "var(--color-bad)")}
              strokeWidth={learning === "P" ? 3 : 1.5}
            >
              {learning === "P" && pulse}
            </rect>
            <text
              x={LANES[3]}
              y={TOP + 6}
              textAnchor="middle"
              fontSize="17"
              fontWeight="600"
              fontFamily="var(--font-display)"
              fill={parked ? "var(--color-bg)" : "var(--color-text)"}
              pointerEvents="none"
            >
              P
            </text>
          </g>

          {/* Chave Low / High */}
          <g onClick={() => startLearning("LH")} className="cursor-pointer">
            <rect
              x={W / 2 - 62}
              y={RANGE_Y - 15}
              width="124"
              height="30"
              rx="15"
              fill="var(--color-panel)"
              stroke={ring("LH", false, rangeMapped, "var(--color-accent)")}
              strokeWidth={learning === "LH" ? 3 : 1.5}
            >
              {learning === "LH" && pulse}
            </rect>
            {rangeMapped && (
              <rect x={high ? W / 2 + 2 : W / 2 - 58} y={RANGE_Y - 11} width="56" height="22" rx="11" fill="var(--color-accent)" pointerEvents="none" />
            )}
            {(["LOW", "HIGH"] as const).map((label, i) => {
              const on = rangeMapped && (i === 1) === high;
              return (
                <text
                  key={label}
                  x={W / 2 + (i === 0 ? -30 : 30)}
                  y={RANGE_Y + 4.5}
                  textAnchor="middle"
                  fontSize="12"
                  fontWeight="600"
                  letterSpacing="1.5"
                  fontFamily="var(--font-display)"
                  fill={on ? "var(--color-bg)" : "var(--color-steel)"}
                  pointerEvents="none"
                >
                  {label}
                </text>
              );
            })}
          </g>

          {/* Alavanca em ponto morto */}
          {!active && <circle cx={neutral.x} cy={neutral.y} r="11" fill="#2b323d" stroke="var(--color-steel)" strokeWidth="2" pointerEvents="none" />}
        </svg>

        <div className="flex-1">
          <div className="label mb-1">{t("shifter.gear")}</div>
          <div className="flex items-baseline gap-3">
            <span className="readout text-6xl font-semibold leading-none" style={{ color: active ? "var(--color-accent)" : "var(--color-steel)" }}>
              {active ? active.gear : "N"}
            </span>
            <span className="readout text-lg text-steel">{rangeMapped ? (high ? "HIGH" : "LOW") : ""}</span>
          </div>
          <div className="readout mt-1 h-5 text-sm" style={{ color: "var(--color-bad)" }}>
            {parked ? t("shifter.parked") : ""}
          </div>
          <div className="mt-3 grid grid-cols-3 gap-x-3 gap-y-1 text-xs">
            {LEGEND.map((s) => (
              <div key={s.key} className="flex justify-between">
                <span className="text-steel">{s.label}</span>
                <span className="readout">{cfg.shifter[s.key] === null ? "—" : `B${cfg.shifter[s.key]}`}</span>
              </div>
            ))}
          </div>
          <button
            onClick={() => settingsStore.update((s) => ({ ...s, rangeInverted: !s.rangeInverted }))}
            className="mono mt-3 text-[11px] uppercase tracking-[0.1em] text-steel underline decoration-dotted underline-offset-4 hover:text-accent"
          >
            {t("shifter.invertRange")}
          </button>
        </div>
      </div>
      <p className="mt-3 text-xs" style={{ color: learning ? "var(--color-warn)" : "var(--color-dim)" }}>
        {learning
          ? learning === "LH"
            ? t("shifter.learnRange")
            : learning === "P"
              ? t("shifter.learnPark")
              : t("shifter.learnGear", { gear: learning })
          : t("shifter.howToLearn")}
      </p>
    </div>
  );
}
