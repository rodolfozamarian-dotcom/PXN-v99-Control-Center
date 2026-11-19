import type { ReactNode } from "react";
import { useT } from "../lib/i18n";

export type Tone = "ok" | "warn" | "bad" | "accent" | "off";

export const toneColor: Record<Tone, string> = {
  ok: "var(--color-ok)",
  warn: "var(--color-warn)",
  bad: "var(--color-bad)",
  accent: "var(--color-accent)",
  off: "var(--color-dim)",
};

export function Panel({
  title,
  code,
  right,
  children,
  className = "",
}: {
  title?: string;
  /** Código curto do painel (ex.: "01"), mostrado antes do título. */
  code?: string;
  right?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  return (
    <section className={`panel ${className}`}>
      {title && (
        <header className="flex h-9 items-center justify-between gap-3 border-b border-line px-4">
          <h2 className="label flex items-center gap-2.5 text-text">
            {code && <span className="text-accent">{code}</span>}
            <span>{title}</span>
          </h2>
          {right}
        </header>
      )}
      <div className="p-4">{children}</div>
    </section>
  );
}

/** LED de estado. */
export function Dot({ tone, pulse = false }: { tone: Tone; pulse?: boolean }) {
  return (
    <span
      className={`inline-block h-[7px] w-[7px] shrink-0 ${pulse ? "animate-pulse" : ""}`}
      style={{
        background: toneColor[tone],
        boxShadow: tone === "off" ? "none" : `0 0 0 2px color-mix(in srgb, ${toneColor[tone]} 22%, transparent)`,
      }}
    />
  );
}

/** Linha de indicador: rótulo, valor real e LED. Sem dado → "Não disponível". */
export function Indicator({ label, value, tone }: { label: string; value: ReactNode | null; tone: Tone }) {
  const t = useT();
  return (
    <div className="flex min-h-9 items-center justify-between gap-3 border-b border-line/70 py-1.5 last:border-b-0">
      <span className="text-steel">{label}</span>
      <span className="flex items-center gap-2.5 text-right">
        <span className={`readout text-[13px] ${value === null ? "text-dim" : "text-text"}`}>{value === null ? t("common.na") : value}</span>
        <Dot tone={value === null ? "off" : tone} />
      </span>
    </div>
  );
}

export function Readout({
  label,
  value,
  unit,
  size = "md",
  tone,
}: {
  label: string;
  value: string | null;
  unit?: string;
  size?: "md" | "lg" | "xl";
  tone?: Tone;
}) {
  const t = useT();
  const cls = size === "xl" ? "text-6xl" : size === "lg" ? "text-4xl" : "text-2xl";
  return (
    <div>
      <div className="label mb-1.5">{label}</div>
      <div className={`readout ${cls} font-semibold leading-none`} style={tone && value !== null ? { color: toneColor[tone] } : undefined}>
        {value === null ? <span className="text-base font-normal text-dim">{t("common.na")}</span> : value}
        {value !== null && unit && <span className="ml-1 text-[0.42em] font-normal text-steel">{unit}</span>}
      </div>
    </div>
  );
}

export function Badge({ tone, children }: { tone: Tone; children: ReactNode }) {
  return (
    <span
      className="mono inline-flex h-6 items-center gap-2 border px-2 text-[10.5px] uppercase tracking-[0.1em]"
      style={{
        color: toneColor[tone],
        borderColor: `color-mix(in srgb, ${toneColor[tone]} 40%, transparent)`,
        background: `color-mix(in srgb, ${toneColor[tone]} 7%, transparent)`,
      }}
    >
      <Dot tone={tone} />
      {children}
    </span>
  );
}

export function Button({
  children,
  onClick,
  variant = "ghost",
  disabled,
  title,
}: {
  children: ReactNode;
  onClick?: () => void;
  variant?: "primary" | "ghost" | "danger";
  disabled?: boolean;
  title?: string;
}) {
  const look =
    variant === "primary"
      ? "border-accent bg-accent text-bg hover:bg-accent/85"
      : variant === "danger"
        ? "border-bad bg-bad text-bg hover:bg-bad/85"
        : "border-line-strong text-text hover:border-accent hover:text-accent";
  return (
    <button
      className={`mono h-8 border px-3.5 text-[11px] font-semibold uppercase tracking-[0.12em] transition-colors disabled:cursor-not-allowed disabled:opacity-35 ${look}`}
      onClick={onClick}
      disabled={disabled}
      title={title}
    >
      {children}
    </button>
  );
}

/** Botões segmentados (ex.: escala 270°/900°, cilindros, idioma). */
export function Segmented<T extends string | number>({
  options,
  value,
  onChange,
  format = (v) => String(v),
}: {
  options: readonly T[];
  value: T;
  onChange: (v: T) => void;
  format?: (v: T) => string;
}) {
  return (
    <div className="inline-flex border border-line-strong">
      {options.map((o, i) => (
        <button
          key={String(o)}
          onClick={() => onChange(o)}
          className={`mono h-7 px-2.5 text-[11px] transition-colors ${i > 0 ? "border-l border-line-strong" : ""} ${
            o === value ? "bg-accent text-bg" : "text-steel hover:text-text"
          }`}
        >
          {format(o)}
        </button>
      ))}
    </div>
  );
}

/** Slider com rótulo, valor e, opcionalmente, medidor do nível atual. */
export function Fader({
  label,
  value,
  unit = "%",
  min = 0,
  max = 100,
  step = 1,
  disabled,
  level,
  hint,
  onChange,
}: {
  label: string;
  value: number;
  unit?: string;
  min?: number;
  max?: number;
  step?: number;
  disabled?: boolean;
  /** Nível atual 0..1, mostrado abaixo do trilho. */
  level?: number;
  hint?: string;
  onChange: (v: number, commit: boolean) => void;
}) {
  const fill = ((value - min) / (max - min)) * 100;
  const read = (e: { target: EventTarget }) => Number((e.target as HTMLInputElement).value);
  return (
    <div>
      <div className="mb-2 flex items-baseline justify-between">
        <span className="label">{label}</span>
        <span className="readout text-base text-text">
          {value}
          <span className="ml-0.5 text-[11px] text-steel">{unit}</span>
        </span>
      </div>
      <input
        type="range"
        className="slider w-full"
        min={min}
        max={max}
        step={step}
        value={value}
        disabled={disabled}
        style={{ ["--fill" as string]: `${fill}%` }}
        onChange={(e) => onChange(read(e), false)}
        onPointerUp={(e) => onChange(read(e), true)}
        onKeyUp={(e) => onChange(read(e), true)}
        aria-label={label}
      />
      {level !== undefined && (
        <div className="mt-2 flex h-1 gap-px" aria-hidden>
          {Array.from({ length: 20 }, (_, i) => (
            <span
              key={i}
              className="flex-1 transition-colors duration-75"
              style={{ background: i < Math.round(Math.min(1, level) * 20) ? (i >= 16 ? "var(--color-bad)" : "var(--color-warn)") : "var(--color-line)" }}
            />
          ))}
        </div>
      )}
      {hint && <p className="mt-1.5 text-xs leading-snug text-dim">{hint}</p>}
    </div>
  );
}

export function PageHeader({ code, title, subtitle, right }: { code: string; title: string; subtitle?: string; right?: ReactNode }) {
  return (
    <div className="mb-5 flex items-end justify-between gap-4 border-b border-line pb-4">
      <div>
        <div className="label mb-1.5 text-dim">
          PXN V99 <span className="text-line-strong">/</span> <span className="text-accent">{code}</span>
        </div>
        <h1 className="font-display text-[26px] font-semibold uppercase leading-none tracking-[0.06em]">{title}</h1>
        {subtitle && <p className="mt-2 text-steel">{subtitle}</p>}
      </div>
      {right}
    </div>
  );
}

/** Texto explicativo de rodapé de painel. */
export function Note({ children, tone }: { children: ReactNode; tone?: Tone }) {
  return (
    <p className="mt-3 text-xs leading-relaxed" style={{ color: tone ? toneColor[tone] : "var(--color-dim)" }}>
      {children}
    </p>
  );
}
