// src/components/Dashboard/Sunburst.tsx
// Faithful port of squirreldisk's sunburst wiring (src/ui/sunburst.rs +
// src/ui/theme.rs):
// - angle 0 at 12 o'clock, increasing clockwise; children sorted descending
// - colour = cyclic hue wheel sampled at the segment's angular midpoint,
//   lightened per ring depth; files lighter + desaturated
// - "smaller items" aggregates are neutral, folded when their arc < 4 px
// - thin rings (r0 = 20 % of radius, remaining radius split across rings)
// - no labels on the chart; the side legend carries names and sizes

import { useMemo, useState } from 'react';
import { VizNode, VizMetric, fmtBytes } from '../../utils/vizTree';

const RINGS = 8;   // center disc + 7 data rings
const MIN_PX = 4;  // fold slices whose arc is under 4 px at mid-ring

interface Props {
  root: VizNode;
  metric: VizMetric;
  focus: VizNode[];
  onFocusChange: (path: VizNode[]) => void;
}

// Hazelnut wheel from theme.rs — the cyclic gradient from the screenshots.
const WHEEL: [number, number, number][] = [
  [0x39, 0x6b, 0xff], [0x8b, 0x4d, 0xff], [0xd9, 0x46, 0xef], [0xff, 0x4f, 0x9a],
  [0xff, 0x70, 0x5a], [0xff, 0xb3, 0x47], [0xff, 0xe4, 0x5c], [0xb8, 0xf0, 0x4a],
  [0x4c, 0xe0, 0x6a], [0x2f, 0xd8, 0xc0], [0x3a, 0xc8, 0xff],
];

const lerp = (a: number, b: number, t: number): number => a + (b - a) * t;

const wheelAt = (t: number): [number, number, number] => {
  const n = WHEEL.length;
  const x = (((t % 1) + 1) % 1) * n;
  const i = Math.floor(x) % n;
  const j = (i + 1) % n;
  const f = x - Math.floor(x);
  return [
    lerp(WHEEL[i][0], WHEEL[j][0], f),
    lerp(WHEEL[i][1], WHEEL[j][1], f),
    lerp(WHEEL[i][2], WHEEL[j][2], f),
  ];
};

// theme.rs to_hsl / from_hsl
const toHsl = (r: number, g: number, b: number): [number, number, number] => {
  const rn = r / 255; const gn = g / 255; const bn = b / 255;
  const max = Math.max(rn, gn, bn);
  const min = Math.min(rn, gn, bn);
  const l = (max + min) / 2;
  if (max - min < 1e-6) return [0, 0, l];
  const d = max - min;
  const s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
  let h: number;
  if (max === rn) h = (gn - bn) / d + (gn < bn ? 6 : 0);
  else if (max === gn) h = (bn - rn) / d + 2;
  else h = (rn - gn) / d + 4;
  return [h / 6, s, l];
};

const fromHsl = (h: number, s: number, l: number): string => {
  const hue = (p: number, q: number, t: number): number => {
    if (t < 0) t += 1;
    if (t > 1) t -= 1;
    if (t < 1 / 6) return p + (q - p) * 6 * t;
    if (t < 0.5) return q;
    if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6;
    return p;
  };
  s = Math.min(Math.max(s, 0), 1);
  l = Math.min(Math.max(l, 0), 1);
  if (s === 0) {
    const v = Math.round(l * 255);
    return `rgb(${v},${v},${v})`;
  }
  const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
  const p = 2 * l - q;
  const r = Math.round(hue(p, q, h + 1 / 3) * 255);
  const g = Math.round(hue(p, q, h) * 255);
  const b = Math.round(hue(p, q, h - 1 / 3) * 255);
  return `rgb(${r},${g},${b})`;
};

const DEPTH_LIGHT = 0.03;

const segmentColor = (midFraction: number, depth: number, isFile: boolean): string => {
  const [r, g, b] = wheelAt(midFraction);
  const [h, s, l] = toHsl(r, g, b);
  const d = Math.max(depth - 1, 0);
  let l2 = l + d * DEPTH_LIGHT;
  let s2 = s * (1 - d * 0.025);
  if (isFile) {
    l2 += 0.04;
    s2 *= 0.85;
  }
  return fromHsl(h, s2, Math.min(Math.max(l2, 0.12), 0.88));
};

// Aggregates: a neutral shade (theme.rs aggregate_color) — reads as "the rest".
const AGGREGATE_COLOR = '#a9a1b8';

interface Seg {
  node: VizNode;
  a0: number;
  a1: number;
  depth: number;
  color: string;
  isFile: boolean;
}

/** squirreldisk polar mapping: a=0 at 12 o'clock, increasing clockwise. */
const point = (cx: number, cy: number, r: number, a: number): [number, number] =>
  [cx + r * Math.sin(a), cy - r * Math.cos(a)];

const describeArc = (cx: number, cy: number, r0: number, r1: number, a0: number, a1: number): string => {
  if (a1 - a0 >= Math.PI * 2 - 1e-6) {
    const [ox, oy] = point(cx, cy, r1, a0);
    const [omx, omy] = point(cx, cy, r1, a0 + Math.PI);
    const [ix, iy] = point(cx, cy, r0, a0);
    const [imx, imy] = point(cx, cy, r0, a0 + Math.PI);
    return [
      `M ${ox} ${oy}`,
      `A ${r1} ${r1} 0 1 1 ${omx} ${omy}`,
      `A ${r1} ${r1} 0 1 1 ${ox} ${oy}`,
      `L ${ix} ${iy}`,
      `A ${r0} ${r0} 0 1 0 ${imx} ${imy}`,
      `A ${r0} ${r0} 0 1 0 ${ix} ${iy}`,
      'Z',
    ].join(' ');
  }
  const large = a1 - a0 > Math.PI ? 1 : 0;
  const [x0, y0] = point(cx, cy, r1, a0);
  const [x1, y1] = point(cx, cy, r1, a1);
  const [x2, y2] = point(cx, cy, r0, a1);
  const [x3, y3] = point(cx, cy, r0, a0);
  return [
    `M ${x0} ${y0}`,
    `A ${r1} ${r1} 0 ${large} 1 ${x1} ${y1}`,
    `L ${x2} ${y2}`,
    `A ${r0} ${r0} 0 ${large} 0 ${x3} ${y3}`,
    'Z',
  ].join(' ');
};

function buildSegments(view: VizNode, r0: number, ring: number): Seg[] {
  const segs: Seg[] = [];

  const walk = (node: VizNode, a0: number, a1: number, depth: number): void => {
    if (depth >= RINGS || node.children.length === 0) return;
    // px-based folding: arc length at this ring's mid radius (sunburst.rs MIN_PX)
    const midR = r0 + (depth - 0.5) * ring;
    const minSpan = MIN_PX / Math.max(midR, 1);
    const span = a1 - a0;
    const kids = [...node.children].sort((x, y) => y.value - x.value);
    let cursor = a0;
    const kept: Array<{ kid: VizNode; span: number }> = [];
    const folded: VizNode[] = [];
    for (const kid of kids) {
      const kidSpan = span * (kid.value / (node.value || 1));
      if (kidSpan < minSpan && kept.length > 0) {
        folded.push(kid);
      } else {
        kept.push({ kid, span: kidSpan });
      }
    }
    // If everything fit, no aggregate; otherwise append the folded aggregate.
    const entries: Array<{ node: VizNode; span: number; agg: boolean }> = kept.map(k => ({
      node: k.kid,
      span: k.span,
      agg: false,
    }));
    if (folded.length > 0) {
      entries.push({
        node: {
          key: `${node.key}/agg`,
          name: `Další (${folded.length})`,
          value: folded.reduce((s, c) => s + c.value, 0),
          bytes: folded.reduce((s, c) => s + c.bytes, 0),
          count: folded.reduce((s, c) => s + c.count, 0),
          depth: depth + 1,
          children: folded,
          isAggregate: true,
        },
        span: span - kept.reduce((s, k) => s + k.span, 0),
        agg: true,
      });
    }

    for (const e of entries) {
      if (e.span <= 0) continue;
      const mid = cursor + e.span / 2;
      const midFraction = mid / (Math.PI * 2);
      const isFile = !e.agg && e.node.children.length === 0;
      segs.push({
        node: e.node,
        a0: cursor,
        a1: cursor + e.span,
        depth,
        color: e.agg ? AGGREGATE_COLOR : segmentColor(midFraction, depth, isFile),
        isFile,
      });
      walk(e.node, cursor, cursor + e.span, depth + 1);
      cursor += e.span;
    }
  };

  walk(view, 0, Math.PI * 2, 1);
  return segs;
}

const valueText = (metric: VizMetric, node: VizNode): string =>
  metric === 'fileCount' ? `${node.count} souborů` : fmtBytes(node.bytes);

export function Sunburst({ root, metric, focus, onFocusChange }: Props) {
  const size = 420;
  const cx = size / 2;
  const cy = size / 2;
  const radius = size / 2 - 8;
  const r0 = radius * 0.2; // theme.rs Geometry: center disc is 20 % of radius
  const ring = (radius - r0) / (RINGS - 1);

  const view = focus.length > 0 ? focus[focus.length - 1] : root;
  const segs = useMemo(
    () => buildSegments(view, r0, ring),
    [view, cx, cy, r0, ring],
  );
  const [hover, setHover] = useState<Seg | null>(null);

  const radii = (depth: number): [number, number] =>
    depth === 0 ? [0, r0] : [r0 + (depth - 1) * ring, r0 + depth * ring];

  const enter = (seg: Seg): void => {
    if (!seg.node.isAggregate && seg.node.children.length > 0) {
      onFocusChange([...focus, seg.node]);
    }
  };

  const crumbs = [root, ...focus];
  // Legend = current view's direct children in angular (descending) order.
  const legend = segs.filter(s => s.depth === 1);
  const aggEntry = legend.find(s => s.node.isAggregate);

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
          fill={focus.length > 0 ? segmentColor(0.62, 1, false) : 'var(--ink)'}
          className="sunburst-center"
          onClick={() => focus.length > 0 && onFocusChange(focus.slice(0, -1))}
          style={{ cursor: focus.length > 0 ? 'pointer' : 'default' }}
        />
        <text x={cx} y={cy - 4} textAnchor="middle" className="sunburst-center-name">
          {valueText(metric, view)}
        </text>
        <text x={cx} y={cy + 12} textAnchor="middle" className="sunburst-center-val">
          {view.name.length > 14 ? `${view.name.slice(0, 13)}…` : view.name}
        </text>

        {segs.map(seg => {
          const [ir, or] = radii(seg.depth);
          const d = describeArc(cx, cy, ir, or, seg.a0, seg.a1);
          const isHover = hover === seg;
          return (
            <path
              key={`${seg.node.key}@${seg.depth}`}
              d={d}
              fill={seg.color}
              stroke="var(--bg)"
              strokeWidth={0.6}
              opacity={hover && !isHover ? 0.55 : 1}
              className="sunburst-seg"
              onMouseEnter={() => setHover(seg)}
              onClick={() => enter(seg)}
              style={{
                cursor: !seg.node.isAggregate && seg.node.children.length > 0 ? 'pointer' : 'default',
                transition: 'opacity 120ms',
              }}
            >
              <title>{`${seg.node.name} · ${valueText(metric, seg.node)}`}</title>
            </path>
          );
        })}
      </svg>

      {/* squirreldisk-style side legend: children of the current view */}
      <div className="sunburst-legend">
        <div className="lg-head">
          <div className="lg-title">
            {view.name.length > 18 ? `${view.name.slice(0, 17)}…` : view.name}
          </div>
          <div className="lg-total">{valueText(metric, view)}</div>
        </div>
        {legend.map(seg => (
          <button
            key={`lg-${seg.node.key}`}
            className={`lg-row${hover === seg ? ' hover' : ''}`}
            onMouseEnter={() => setHover(seg)}
            onMouseLeave={() => setHover(null)}
            onClick={() => enter(seg)}
            disabled={seg.node.isAggregate || seg.node.children.length === 0}
          >
            <span className="lg-dot" style={{ background: seg.color }} />
            <span className="lg-name">
              {seg.node.name.length > 22 ? `${seg.node.name.slice(0, 21)}…` : seg.node.name}
            </span>
            <span className="lg-val">{valueText(metric, seg.node)}</span>
          </button>
        ))}
        {aggEntry && (
          <div className="lg-more">
            a {aggEntry.node.count} dalších položek
          </div>
        )}
      </div>

      {hover && (
        <div className="sunburst-tooltip">
          <div className="st-name">{hover.node.name}</div>
          <div className="st-row">
            {valueText(metric, hover.node)}
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
      </div>
    </div>
  );
}

