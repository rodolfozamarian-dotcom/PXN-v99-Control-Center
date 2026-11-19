// Indicadores de pedais, botões e hat, sempre a partir de dados reais.
import { useT } from "../lib/i18n";

export function PedalBar({
  label,
  value,
  color = "var(--color-accent)",
  height = 170,
}: {
  label: string;
  value: number | null;
  color?: string;
  height?: number;
}) {
  const pct = value === null ? 0 : Math.round(value * 100);
  return (
    <div className="flex flex-col items-center gap-2">
      <div className="readout text-lg font-semibold" style={{ color: value === null ? "var(--color-dim)" : "var(--color-text)" }}>
        {value === null ? "—" : `${pct}`}
        {value !== null && <span className="ml-0.5 text-[10px] text-steel">%</span>}
      </div>
      <div className="relative w-10 border border-line-strong bg-bg" style={{ height }}>
        {[25, 50, 75].map((m) => (
          <span key={m} className="absolute -left-1.5 z-10 h-px w-1.5 bg-line-strong" style={{ bottom: `${m}%` }} />
        ))}
        <div className="absolute inset-x-[3px] bottom-[3px] transition-[height] duration-75" style={{ height: `calc(${pct}% - 6px * ${pct / 100})`, background: color }} />
      </div>
      <div className="label text-center">{label}</div>
    </div>
  );
}

export function ButtonGrid({ count, pressed, columns = 8 }: { count: number; pressed: number[]; columns?: number }) {
  const t = useT();
  if (count === 0) return <p className="text-dim">{t("common.na")}</p>;
  return (
    <div className="grid gap-1" style={{ gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))` }}>
      {Array.from({ length: count }, (_, i) => i + 1).map((n) => {
        const on = pressed.includes(n);
        return (
          <div
            key={n}
            title={t(on ? "inputs.buttonPressed" : "inputs.buttonReleased", { n })}
            className={`mono flex h-8 items-center justify-center border text-xs transition-colors duration-75 ${
              on ? "border-accent bg-accent text-bg" : "border-line bg-raised text-steel"
            }`}
          >
            {String(n).padStart(2, "0")}
          </div>
        );
      })}
    </div>
  );
}

const HAT_KEYS = ["inputs.hatUp", "inputs.hatUpRight", "inputs.hatRight", "inputs.hatDownRight", "inputs.hatDown", "inputs.hatDownLeft", "inputs.hatLeft", "inputs.hatUpLeft"] as const;

export function HatIndicator({ hat, available }: { hat: number | null; available: boolean }) {
  const t = useT();
  if (!available) return <p className="text-dim">{t("common.na")}</p>;
  return (
    <div className="flex items-center gap-4">
      <svg viewBox="0 0 80 80" className="h-[72px] w-[72px]">
        <rect x="6" y="6" width="68" height="68" fill="var(--color-raised)" stroke="var(--color-line-strong)" />
        {HAT_KEYS.map((_, i) => {
          const a = (i * 45 * Math.PI) / 180;
          const on = hat === i;
          const s = on ? 9 : 5;
          return (
            <rect
              key={i}
              x={40 + 24 * Math.sin(a) - s / 2}
              y={40 - 24 * Math.cos(a) - s / 2}
              width={s}
              height={s}
              fill={on ? "var(--color-accent)" : "var(--color-line-strong)"}
            />
          );
        })}
        <rect x="37" y="37" width="6" height="6" fill={hat === null ? "var(--color-steel)" : "var(--color-line-strong)"} />
      </svg>
      <div>
        <div className="label">D-Pad</div>
        <div className="readout mt-1 text-base">{hat === null ? t("inputs.hatCenter") : t(HAT_KEYS[hat])}</div>
      </div>
    </div>
  );
}
