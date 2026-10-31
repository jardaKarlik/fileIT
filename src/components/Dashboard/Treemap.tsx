// src/components/Dashboard/Treemap.tsx
// Squarified treemap (Bruls, Huizing, van Wijk) — the same algorithm
// squirreldisk uses in src/ui/treemap.rs. Nested rectangles with header
// strips for group nodes; tiny tiles fold into a "Další položky" tile;
// click to zoom, breadcrumb to go back.

import { useMemo, useState } from 'react';
import { VizNode, VizMetric, foldSmallChildren, fmtBytes } from '../../utils/vizTree';

const MIN_TILE_FRACTION = 0.02;

interface Props {
  root: VizNode;
  metric: VizMetric;
  focus: VizNode[];
  onFocusChange: (path: VizNode[]) => void;
}

interface Rect { x: number; y: number; w: number; h: number }
interface Tile { node: VizNode; rect: Rect; nested: boolean; color: string }

const PALETTE = [
  '#E5484D', '#F76B15', '#FFB224', '#46A758', '#00A2C7',
  '#3E63DD', '#8E4EC6', '#E93D82', '#6E56CF', '#29A383',
];

const colorFor = (key: string): string => {
  let h = 0;
  for (let i = 0; i < key.length; i += 1) h = (h * 31 + key.charCodeAt(i)) >>> 0;
  return PALETTE[h % PALETTE.length];
};

const lighten = (hex: string, f: number): string => {
  const n = parseInt(hex.slice(1), 16);
  const mix = (c: number) => Math.round(c + (255 - c) * f);
  return `rgb(${mix((n >> 16) & 255)},${mix((n >> 8) & 255)},${mix(n & 255)})`;
};

/**
 * Squarified layout: `areas` must be sorted descending and sum to `rect`'s
 * area; returns one rect per area. Direct port of squirreldisk's squarify().
 */
function squarify(areas: number[], rect: Rect): Rect[] {
  const out: Rect[] = [];
  let r = { ...rect };
  let i = 0;
  while (i < areas.length) {
    const short = Math.max(Math.min(r.w, r.h), 1e-3);
    const worst = (row: number[]): number => {
      const s = row.reduce((a, b) => a + b, 0);
      let mx = 0;
      let mn = Infinity;
      for (const v of row) { mx = Math.max(mx, v); mn = Math.min(mn, v); }
      return Math.max((short * short * mx) / (s * s), (s * s) / (short * short * Math.max(mn, 1e-6)));
    };
    let j = i + 1;
    while (j < areas.length && worst(areas.slice(i, j + 1)) <= worst(areas.slice(i, j))) j += 1;
    const row = areas.slice(i, j);
    const s = row.reduce((a, b) => a + b, 0);
    if (r.w >= r.h) {
      const w = Math.min(s / Math.max(r.h, 1e-3), r.w);
      let y = r.y;
      for (const a of row) {
        const h = a / Math.max(w, 1e-3);
        out.push({ x: r.x, y, w, h });
        y += h;
      }
      r = { x: r.x + w, y: r.y, w: r.w - w, h: r.h };
    } else {
      const h = Math.min(s / Math.max(r.w, 1e-3), r.h);
      let x = r.x;
      for (const a of row) {
        const w = a / Math.max(h, 1e-3);
        out.push({ x, y: r.y, w, h });
        x += w;
      }
      r = { x: r.x, y: r.y + h, w: r.w, h: r.h - h };
    }
    i = j;
  }
  return out;
}

const W = 560;
const H = 420;
const HEADER = 18;
const PAD = 3;
const MIN_AREA = 90; // tiles smaller than this (px²) fold away

function layout(view: VizNode): Tile[] {
  const tiles: Tile[] = [];

  const walk = (node: VizNode, rect: Rect, depth: number): void => {
    if (rect.w < 8 || rect.h < 8) return;
    const kids = foldSmallChildren(node, node.value || 1, MIN_TILE_FRACTION)
      .filter(k => (k.value / (node.value || 1)) * rect.w * rect.h >= MIN_AREA);
    if (kids.length === 0) return;
    const areas = kids.map(k => Math.max((k.value / (node.value || 1)) * rect.w * rect.h, 1e-3));
    const rects = squarify(areas, rect);

    rects.forEach((r, idx) => {
      const kid = kids[idx];
      if (!kid || r.w <= 0 || r.h <= 0) return;
      const nested = kid.children.length > 0 && r.w >= 56 && r.h >= 44 && depth < 3;
      tiles.push({
        node: kid,
        rect: r,
        nested,
        color: lighten(colorFor(kid.key), depth === 1 ? 0 : 0.25),
      });
      if (nested) {
        walk(
          kid,
          { x: r.x + PAD, y: r.y + HEADER, w: r.w - PAD * 2, h: r.h - HEADER - PAD },
          depth + 1,
        );
      }
    });
  };

  walk(view, { x: 0, y: 0, w: W, h: H }, 1);
  return tiles;
}

const valueText = (metric: VizMetric, node: VizNode): string =>
  metric === 'fileCount' ? `${node.count} souborů` : fmtBytes(node.bytes);

export function Treemap({ root, metric, focus, onFocusChange }: Props) {
  const view = focus.length > 0 ? focus[focus.length - 1] : root;
  const tiles = useMemo(() => layout(view), [view, metric]);
  const [hover, setHover] = useState<Tile | null>(null);
  const crumbs = [root, ...focus];

  const enter = (tile: Tile): void => {
    if (!tile.node.isAggregate && tile.node.children.length > 0) {
      onFocusChange([...focus, tile.node]);
    }
  };

  return (
    <div className="treemap-wrap">
      <svg
        width={W}
        height={H}
        viewBox={`0 0 ${W} ${H}`}
        className="treemap-svg"
        onMouseLeave={() => setHover(null)}
      >
        {tiles.length === 0 && (
          <text x={W / 2} y={H / 2} textAnchor="middle" className="treemap-empty">
            Prázdné
          </text>
        )}
        {tiles.map(tile => {
          const { rect: r } = tile;
          const isHover = hover === tile;
          return (
            <g
              key={tile.node.key}
              onClick={() => enter(tile)}
              onMouseEnter={() => setHover(tile)}
              style={{ cursor: !tile.node.isAggregate && tile.node.children.length > 0 ? 'pointer' : 'default' }}
            >
              <rect
                x={r.x}
                y={r.y}
                width={r.w}
                height={r.h}
                fill={tile.color}
                stroke="var(--bg)"
                strokeWidth={PAD}
                opacity={hover && !isHover ? 0.55 : 1}
                style={{ transition: 'opacity 120ms' }}
              >
                <title>{`${tile.node.name} · ${valueText(metric, tile.node)}`}</title>
              </rect>
              {tile.nested && r.w > 46 && (
                <>
                  <text x={r.x + 5} y={r.y + HEADER / 2 + 4} className="tm-name">
                    {tile.node.name.length > Math.floor(r.w / 7)
                      ? `${tile.node.name.slice(0, Math.max(Math.floor(r.w / 7) - 1, 1))}…`
                      : tile.node.name}
                  </text>
                  <text x={r.x + r.w - 5} y={r.y + HEADER / 2 + 4} textAnchor="end" className="tm-val">
                    {metric === 'fileCount' ? `${tile.node.count}` : fmtBytes(tile.node.bytes)}
                  </text>
                </>
              )}
              {!tile.nested && r.w > 46 && r.h > 30 && (
                <>
                  <text x={r.x + 5} y={r.y + 14} className="tm-name">
                    {tile.node.name.length > Math.floor(r.w / 7)
                      ? `${tile.node.name.slice(0, Math.max(Math.floor(r.w / 7) - 1, 1))}…`
                      : tile.node.name}
                  </text>
                  {r.h > 44 && (
                    <text x={r.x + 5} y={r.y + 28} className="tm-val">
                      {valueText(metric, tile.node)}
                    </text>
                  )}
                </>
              )}
            </g>
          );
        })}
      </svg>

      {hover && (
        <div className="sunburst-tooltip">
          <div className="st-name">{hover.node.name}</div>
          <div className="st-row">
            {valueText(metric, hover.node)}
            {' · '}
            {Math.round((hover.node.value / Math.max(view.value, 1)) * 100)} %
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
        <span className="crumb-total">{valueText(metric, view)}</span>
      </div>
    </div>
  );
}
