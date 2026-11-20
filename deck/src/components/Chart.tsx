import type { ChartData } from "../types";

const W = 1000;
const H = 420;

function fmt(n: number, unit: string) {
  const s = Math.abs(n) >= 1000 ? n.toLocaleString("en-US", { maximumFractionDigits: 1 }) : String(+n.toFixed(2));
  if (!unit) return s;
  return unit === "$" ? `$${s}` : unit.startsWith("$") ? `$${s}${unit.slice(1)}` : `${s}${unit}`;
}

/** Plain SVG charts that render identically in the editor, presenter and HTML export. */
export function Chart({ data }: { data: ChartData }) {
  const n = Math.min(data.labels.length, data.values.length);
  if (!n) return <div className="s-chart-empty">Add chart data in the panel on the right</div>;
  const labels = data.labels.slice(0, n);
  const values = data.values.slice(0, n);

  if (data.kind === "donut") return <Donut labels={labels} values={values} unit={data.unit} />;

  const max = Math.max(...values, 0);
  const min = Math.min(...values, 0);
  const span = max - min || 1;
  const padL = 20;
  const padB = 50;
  const padT = 64;
  const plotH = H - padB - padT;
  const y = (v: number) => padT + plotH - ((v - min) / span) * plotH;
  const slot = (W - padL * 2) / n;

  return (
    <svg className="s-chart" viewBox={`0 0 ${W} ${H}`} role="img" aria-label="Chart">
      <line x1={padL} x2={W - padL} y1={y(0)} y2={y(0)} stroke="var(--s-muted)" strokeOpacity={0.4} strokeWidth={2} />
      {data.kind === "bar" &&
        values.map((v, i) => {
          const bw = Math.min(slot * 0.6, 140);
          const x = padL + slot * i + (slot - bw) / 2;
          const top = Math.min(y(v), y(0));
          const h = Math.max(Math.abs(y(v) - y(0)), 2);
          return (
            <g key={i}>
              <rect x={x} y={top} width={bw} height={h} rx={6} fill={i === n - 1 ? "var(--s-primary)" : "var(--s-accent)"} opacity={i === n - 1 ? 1 : 0.75} />
              <text x={x + bw / 2} y={top - 12} textAnchor="middle" className="s-chart-value">
                {fmt(v, data.unit)}
              </text>
            </g>
          );
        })}
      {data.kind === "line" && (
        <>
          <polyline
            fill="none"
            stroke="var(--s-primary)"
            strokeWidth={6}
            strokeLinejoin="round"
            strokeLinecap="round"
            points={values.map((v, i) => `${padL + slot * i + slot / 2},${y(v)}`).join(" ")}
          />
          {values.map((v, i) => (
            <g key={i}>
              <circle cx={padL + slot * i + slot / 2} cy={y(v)} r={10} fill="var(--s-bg)" stroke="var(--s-primary)" strokeWidth={5} />
              <text x={padL + slot * i + slot / 2} y={y(v) - 22} textAnchor="middle" className="s-chart-value">
                {fmt(v, data.unit)}
              </text>
            </g>
          ))}
        </>
      )}
      {labels.map((l, i) => (
        <text key={i} x={padL + slot * i + slot / 2} y={H - 14} textAnchor="middle" className="s-chart-label">
          {l}
        </text>
      ))}
    </svg>
  );
}

function Donut({ labels, values, unit }: { labels: string[]; values: number[]; unit: string }) {
  const total = values.reduce((a, b) => a + Math.max(b, 0), 0) || 1;
  const r = 150;
  const cx = 220;
  const cy = H / 2;
  const C = 2 * Math.PI * r;
  let offset = 0;
  const palette = ["var(--s-primary)", "var(--s-accent)", "var(--s-muted)", "var(--s-text)"];
  return (
    <svg className="s-chart" viewBox={`0 0 ${W} ${H}`} role="img" aria-label="Donut chart">
      <g transform={`rotate(-90 ${cx} ${cy})`}>
        {values.map((v, i) => {
          const len = (Math.max(v, 0) / total) * C;
          const el = (
            <circle
              key={i}
              cx={cx}
              cy={cy}
              r={r}
              fill="none"
              stroke={palette[i % palette.length]}
              strokeOpacity={i >= palette.length ? 0.5 : 1}
              strokeWidth={70}
              strokeDasharray={`${len} ${C - len}`}
              strokeDashoffset={-offset}
            />
          );
          offset += len;
          return el;
        })}
      </g>
      {labels.map((l, i) => (
        <g key={i} transform={`translate(470 ${70 + i * 56})`}>
          <rect width={28} height={28} rx={6} fill={palette[i % palette.length]} opacity={i >= palette.length ? 0.5 : 1} />
          <text x={44} y={22} className="s-chart-legend">
            {l} <tspan className="s-chart-legend-v">{unit ? fmt(values[i], unit) : `${Math.round((Math.max(values[i], 0) / total) * 100)}%`}</tspan>
          </text>
        </g>
      ))}
    </svg>
  );
}
