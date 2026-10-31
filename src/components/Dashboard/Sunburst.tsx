// src/components/Dashboard/Sunburst.tsx
// SVG sunburst in the spirit of squirreldisk's src/ui/sunburst.rs:
// - angle proportional to the metric value, children sorted descending
// - slices below a minimum arc fold into a "Další položky" aggregate wedge
// - click a ring segment to zoom it into the center (breadcrumb to go back)
// - hover highlight with a tooltip (name, bytes, file count)

import { useMemo, useState } from 'react';
import { VizNode, VizMetric, foldSmallChildren, fmtBytes } from '../../utils/vizTree';

const PALETTE = [
  '#E5484D', '#F76B15', '#FFB224', '#46A758', '#00A2C7',
  '#3E63DD', '#8E4EC6', '#E93D82', '#6E56CF', '#29A383',
];

const RINGS = 3;               // center disc + 2 data rings
const MIN_ARC_FRACTION = 0.012; // slices below ~1.2% fold into "smaller items"

interface Props {
  root: VizNode;
  metric: VizMetric;
  focus: VizNode[];
  onFocusChange: (path: VizNode[]) => void;
}

const colorFor = (key: string): string => {
  let h = 0;
  for (let i = 0; i < key.length; i += 1) h = (h * 31 + key.charCodeAt(i)) >>> 0;
  return PALETTE[h % PALETTE.length];
};

const shade = (hex: string, depth: number): string => {
  const n = parseInt(hex.slice(1), 16);
  const f = depth === 1 ? 1 : depth === 2 ? 0.72 : 0.5;
  const r = Math.round(((n >> 16) & 255) * f);
  const g = Math.round(((n >> 8) & 255) * f);
  const b = Math.round((n & 255) * f);
  return `rgb(${r},${g},${b})`;
};

const describeArc = (cx: number, cy: number, r0: number, r1: number, a0: number, a1: number): string => {
  const p = (r: number, a: number): [number, number] =>
    [cx + r * Math.cos(a), cy + r * Math.sin(a)];
  const large = a1 - a0 > Math.PI ? 1 : 0;
  const [x0, y0] = p(r1, a0);
  const [x1, y1] = p(r1, a1);
  const [x2, y2] = p(r0, a1);
  const [x3, y3] = p(r0, a0);
  return [
    `M ${x0} ${y0}`,
    `A ${r1} ${r1} 0 ${large} 1 ${x1} ${y1}`,
    `L ${x2} ${y2}`,
    `A ${r0} ${r0} 0 ${large} 0 ${x3} ${y3}`,
    'Z',
  ].join(' ');
};

interface Seg { node: VizNode; a0: number; a1: number; depth: number; color: string }

function buildSegments(view: VizNode, rings: number): { segs: Seg[]; total: number } {
  const segs: Seg[] = [];
  const total = view.value || 1;

  const walk = (node: VizNode, a0: number, a1: number, depth: number): void => {
    if (depth >= rings) return;
    const kids = foldSmallChildren(node, node.value || total, MIN_ARC_FRACTION);
    let cursor = a0;
    const span = a1 - a0;
    for (const kid of kids) {
      const kidSpan = span * (kid.value / (node.value || total));
      segs.push({
        node: kid,
        a0: cursor,
        a1: cursor + kidSpan,
        depth,
        color: shade(colorFor(kid.key), depth),
      });
      walk(kid, cursor, cursor + kidSpan, depth + 1);
      cursor += kidSpan;
    }
  };

  walk(view, 0, Math.PI * 2, 1);
  return { segs, total };
}

export function Sunburst({ root, metric, focus, onFocusChange }: Props) {
  const size = 420;
  const cx = size / 2;
  const cy = size / 2;
  const radius = size / 2 - 8;
  const r0 = radius * 0.2;
  const ring = (radius - r0) / (RINGS - 1);

  const view = focus.length > 0 ? focus[focus.length - 1] : root;
  const { segs } = useMemo(() => buildSegments(view, RINGS), [view]);
  const [hover, setHover] = useState<Seg | null>(null);

  const radii = (depth: number): [number, number] =>
    depth === 0 ? [0, r0] : [r0 + (depth - 1) * ring, r0 + depth * ring];

  const enter = (seg: Seg): void => {
    if (!seg.node.isAggregate && seg.node.children.length > 0) {
      onFocusChange([...focus, seg.node]);
    }
  };

  const crumbs = [root, ...focus];

  return (
    <div className="sunburst-wrap">
      <svg
        width={size}
        height={size}
        viewBox={`0 0 ${size} ${size}`}
        className="sunburst-svg"
        onMouseLeave={() => setHover(null)}
      >
        {/* center disc = current view; click to zoom back out */}
        <circle
          cx={cx}
          cy={cy}
          r={r0}
          fill={focus.length > 0 ? shade(colorFor(view.key), 1) : 'var(--ink)'}
          className="sunburst-center"
          onClick={() => focus.length > 0 && onFocusChange(focus.slice(0, -1))}
          style={{ cursor: focus.length > 0 ? 'pointer' : 'default' }}
        />
        <text x={cx} y={cy - 4} textAnchor="middle" className="sunburst-center-name">
          {view.name.length > 14 ? `${view.name.slice(0, 13)}…` : view.name}
        </text>
        <text x={cx} y={cy + 12} textAnchor="middle" className="sunburst-center-val">
          {metric === 'fileCount' ? `${view.count}` : fmtBytes(view.bytes)}
        </text>

        {segs.map(seg => {
          const [ir, or] = radii(seg.depth);
          const d = describeArc(cx, cy, ir, or, seg.a0, seg.a1);
          const isHover = hover === seg;
          return (
            <path
              key={seg.node.key}
              d={d}
              fill={seg.color}
              stroke="var(--bg)"
              strokeWidth={1}
              opacity={hover && !isHover ? 0.45 : 1}
              className="sunburst-seg"
              onMouseEnter={() => setHover(seg)}
              onClick={() => enter(seg)}
              style={{
                cursor: !seg.node.isAggregate && seg.node.children.length > 0 ? 'pointer' : 'default',
                transition: 'opacity 120ms',
              }}
            >
              <title>
                {seg.node.name} ·{' '}
                {metric === 'fileCount' ? `${seg.node.count} souborů` : fmtBytes(seg.node.bytes)}
                {' · '}
                {Math.round(((seg.a1 - seg.a0) / (Math.PI * 2)) * 100)} %
              </title>
            </path>
          );
        })}

        {/* labels on the outermost ring, only for wide slices */}
        {segs.filter(s => s.depth === 2 && (s.a1 - s.a0) > 0.22).map(seg => {
          const [, or] = radii(seg.depth);
          const mid = (seg.a0 + seg.a1) / 2;
          const lx = cx + (or - 14) * Math.cos(mid);
          const ly = cy + (or - 14) * Math.sin(mid);
          return (
            <text
              key={`lbl-${seg.node.key}`}
              x={lx}
              y={ly}
              textAnchor="middle"
              className="sunburst-label"
              transform={`rotate(${(mid * 180) / Math.PI + (mid > Math.PI ? 90 : -90)} ${lx} ${ly})`}
            >
              {seg.node.name.length > 12 ? `${seg.node.name.slice(0, 11)}…` : seg.node.name}
            </text>
          );
        })}
      </svg>

      {hover && (
        <div className="sunburst-tooltip">
          <div className="st-name">{hover.node.name}</div>
          <div className="st-row">
            {metric === 'fileCount'
              ? `${hover.node.count} souborů`
              : `${fmtBytes(hover.node.bytes)} · ${hover.node.count} souborů`}
            {' · '}
            {Math.round(((hover.a1 - hover.a0) / (Math.PI * 2)) * 100)} %
          </div>
        </div>
      )}

      <div className="sunburst-crumbs">
        {crumbs.map((n, i) => (
          <button
            key={n.key}
            className="crumb"
            onClick={() => onFocusChange(focus.slice(0, i))}
            disabled={i === crumbs.length - 1}
          >
            {n.name}
            {i < crumbs.length - 1 && <span className="crumb-sep">›</span>}
          </button>
        ))}
        <span className="crumb-total">
          {metric === 'fileCount' ? `${view.count} souborů` : fmtBytes(view.bytes)}
        </span>
      </div>
    </div>
  );
}
