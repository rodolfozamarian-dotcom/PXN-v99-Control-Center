// Volante visto de frente + escala semicircular de -curso/2 a +curso/2.
// `norm` é a posição real do eixo (-1..1); null = sem dados.

const CX = 200;
const CY = 200;
const SWEEP = 120; // a escala ocupa de -120° a +120° no mostrador

function polar(r: number, deg: number): [number, number] {
  const a = (deg * Math.PI) / 180;
  return [CX + r * Math.sin(a), CY - r * Math.cos(a)];
}

function arc(r: number, from: number, to: number) {
  const [x1, y1] = polar(r, from);
  const [x2, y2] = polar(r, to);
  const large = Math.abs(to - from) > 180 ? 1 : 0;
  const sweep = to > from ? 1 : 0;
  return `M ${x1.toFixed(2)} ${y1.toFixed(2)} A ${r} ${r} 0 ${large} ${sweep} ${x2.toFixed(2)} ${y2.toFixed(2)}`;
}

export function WheelGauge({ norm, range, className = "" }: { norm: number | null; range: number; className?: string }) {
  const half = range / 2;
  const angle = norm === null ? 0 : norm * half;
  const gaugeDeg = norm === null ? 0 : norm * SWEEP;
  const live = norm !== null;
  const ticks = Array.from({ length: 25 }, (_, i) => i);

  return (
    <svg viewBox="0 0 400 400" className={className} role="img" aria-label={`${angle.toFixed(1)}°`}>
      <defs>
        <radialGradient id="hub" cx="50%" cy="40%" r="70%">
          <stop offset="0%" stopColor="#2a303a" />
          <stop offset="100%" stopColor="#12151a" />
        </radialGradient>
        <linearGradient id="rim" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor="#3a424f" />
          <stop offset="100%" stopColor="#1a1e25" />
        </linearGradient>
      </defs>

      {/* Escala */}
      <path d={arc(184, -SWEEP, SWEEP)} fill="none" stroke="var(--color-line-strong)" strokeWidth="2" />
      {ticks.map((i) => {
        const deg = -SWEEP + (i * 2 * SWEEP) / 24;
        const major = i % 6 === 0;
        const [x1, y1] = polar(184, deg);
        const [x2, y2] = polar(major ? 170 : 177, deg);
        return (
          <line
            key={i}
            x1={x1}
            y1={y1}
            x2={x2}
            y2={y2}
            stroke={major ? "var(--color-steel)" : "var(--color-line-strong)"}
            strokeWidth={major ? 2 : 1}
          />
        );
      })}
      {[-1, -0.5, 0, 0.5, 1].map((f) => {
        const [x, y] = polar(157, f * SWEEP);
        return (
          <text key={f} x={x} y={y + 4} textAnchor="middle" fontSize="11" fill="var(--color-steel)" fontFamily="var(--font-display)">
            {f === 0 ? "0" : `${f > 0 ? "+" : "−"}${Math.abs(f * half).toFixed(0)}°`}
          </text>
        );
      })}

      {/* Deflexão atual a partir do centro */}
      {live && Math.abs(gaugeDeg) > 0.3 && (
        <path d={arc(184, 0, gaugeDeg)} fill="none" stroke="var(--color-accent)" strokeWidth="5" strokeLinecap="round" />
      )}
      {live && (
        <g transform={`rotate(${gaugeDeg} ${CX} ${CY})`}>
          <path d={`M ${CX} ${CY - 196} l -7 -11 h 14 z`} fill="var(--color-accent)" />
        </g>
      )}

      {/* Volante */}
      <g transform={`rotate(${angle} ${CX} ${CY})`} opacity={live ? 1 : 0.35}>
        <circle cx={CX} cy={CY} r="120" fill="none" stroke="url(#rim)" strokeWidth="22" />
        <circle cx={CX} cy={CY} r="131" fill="none" stroke="#454e5c" strokeWidth="1" />
        <circle cx={CX} cy={CY} r="109" fill="none" stroke="#0c0e12" strokeWidth="1" />
        {/* Raios */}
        <path d={`M ${CX - 110} ${CY - 8} L ${CX - 40} ${CY - 14} L ${CX - 40} ${CY + 18} L ${CX - 108} ${CY + 22} Z`} fill="#222831" stroke="#39414d" />
        <path d={`M ${CX + 110} ${CY - 8} L ${CX + 40} ${CY - 14} L ${CX + 40} ${CY + 18} L ${CX + 108} ${CY + 22} Z`} fill="#222831" stroke="#39414d" />
        <path d={`M ${CX - 16} ${CY + 36} L ${CX + 16} ${CY + 36} L ${CX + 12} ${CY + 110} L ${CX - 12} ${CY + 110} Z`} fill="#222831" stroke="#39414d" />
        <circle cx={CX} cy={CY} r="46" fill="url(#hub)" stroke="#3d4552" strokeWidth="1.5" />
        <text x={CX} y={CY + 5} textAnchor="middle" fontSize="15" fontWeight="600" letterSpacing="2" fill="var(--color-steel)" fontFamily="var(--font-display)">
          PXN
        </text>
        {/* Marca das 12 horas */}
        <rect x={CX - 4} y={CY - 132} width="8" height="24" rx="2" fill={live ? "var(--color-accent)" : "var(--color-dim)"} />
      </g>
    </svg>
  );
}
