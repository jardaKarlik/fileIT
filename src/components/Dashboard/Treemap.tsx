// src/components/Dashboard/Treemap.tsx
// Squarified treemap (Bruls, Huizing, van Wijk) — the same algorithm
// squirreldisk uses in src/ui/treemap.rs. Nested rectangles with header
// strips for group nodes; tiny tiles fold into a "Další položky" tile;
// click to zoom, breadcrumb to go back.

import { useMemo, useState } from 'react';
import { VizNode, VizMetric, fmtBytes } from '../../utils/vizTree';
import { AGGREGATE_COLOR, segmentColor } from './vizColors';

const MIN_TILE_AREA = 240; // px² — below this a tile is unreadable and folds

interface Props {
  root: VizNode;
  metric: VizMetric;
  focus: VizNode[];
  onFocusChange: (path: VizNode[]) => void;
}

interface Rect { x: number; y: number; w: number; h: number }
interface Tile { node: VizNode; rect: Rect; nested: boolean; color: string }

/**
 * Treemap tile colour — same theme.rs wheel system as the sunburst: hue from
 * the tile's ordinal position among its (descending) siblings, deeper levels
 * lighten, file leaves lighten + desaturate, aggregates neutral.
 */
const tileColor = (midFraction: number, depth: number, isFile: boolean, isAggregate: boolean): string =>
  isAggregate ? AGGREGATE_COLOR : segmentColor(midFraction, depth, isFile);

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

function layout(view: VizNode): Tile[] {
  const tiles: Tile[] = [];

  const walk = (node: VizNode, rect: Rect, depth: number): void => {
    if (rect.w < 8 || rect.h < 8) return;
    // squirreldisk-style px folding (treemap.rs MIN_AREA): a child whose tile
    // would be smaller than MIN_TILE_AREA px² folds into one neutral
    // aggregate — the threshold is absolute, not a fraction of the total.
    const total = node.value || 1;
    const sorted = [...node.children].sort((a, b) => b.value - a.value);
    const visible: VizNode[] = [];
    const small: VizNode[] = [];
    for (const kid of sorted) {
      const area = (kid.value / total) * rect.w * rect.h;
      (area < MIN_TILE_AREA ? small : visible).push(kid);
    }
    let kids: VizNode[];
    if (small.length === 0) {
      kids = sorted;
    } else if (visible.length === 0) {
      kids = small.slice(0, 4); // everything tiny — show the largest few
    } else {
      kids = [
        ...visible,
        {
          key: `${node.key}/agg`,
          name: `Další (${small.length})`,
          value: small.reduce((s, c) => s + c.value, 0),
          bytes: small.reduce((s, c) => s + c.bytes, 0),
          count: small.reduce((s, c) => s + c.count, 0),
          depth: (visible[0]?.depth ?? node.depth + 1),
          children: small,
          isAggregate: true,
        },
      ];
    }
    if (kids.length === 0) return;
    const areas = kids.map(k => Math.max((k.value / total) * rect.w * rect.h, 1e-3));
    const rects = squarify(areas, rect);

    rects.forEach((r, idx) => {
      const kid = kids[idx];
      if (!kid || r.w <= 0 || r.h <= 0) return;
      const nested = !kid.isAggregate && kid.children.length > 0 && r.w >= 56 && r.h >= 44 && depth < 3;
      const isFile = kid.children.length === 0;
      tiles.push({
        node: kid,
        rect: r,
        nested,
        color: tileColor((idx + 0.5) / kids.length, depth, isFile, kid.isAggregate === true),
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
  metric === 'count' ? `${node.count} souborů` : fmtBytes(node.bytes);

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
        {tiles.map((tile, tIdx) => {
          const { rect: r } = tile;
          const isHover = hover === tile;
          const cid = `tmclip-${tIdx}`;
          return (
            <g
              key={tile.node.key}
              onClick={() => enter(tile)}
              onMouseEnter={() => setHover(tile)}
              style={{ cursor: !tile.node.isAggregate && tile.node.children.length > 0 ? 'pointer' : 'default' }}
            >
              <defs>
                <clipPath id={cid}>
                  <rect x={r.x} y={r.y} width={Math.max(r.w - 1, 0)} height={Math.max(r.h - 1, 0)} />
                </clipPath>
              </defs>
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
              <g clipPath={`url(#${cid})`}>
              {tile.nested && r.w > 46 && (
                <>
                  <text x={r.x + 5} y={r.y + HEADER / 2 + 4} className="tm-name">
                    {tile.node.name.length > Math.floor(r.w / 7)
                      ? `${tile.node.name.slice(0, Math.max(Math.floor(r.w / 7) - 1, 1))}…`
                      : tile.node.name}
                  </text>
                  <text x={r.x + r.w - 5} y={r.y + HEADER / 2 + 4} textAnchor="end" className="tm-val">
                    {metric === 'count' ? `${tile.node.count}` : fmtBytes(tile.node.bytes)}
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
