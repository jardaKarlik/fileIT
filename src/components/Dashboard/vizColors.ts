// src/components/Dashboard/vizColors.ts
// Shared squirreldisk theme.rs port: cyclic hue wheel + segment_color.
// Used by both the sunburst and the treemap so both charts read as one system.

// Hazelnut wheel from theme.rs — the cyclic gradient from the reference.
export const WHEEL: [number, number, number][] = [
  [0x39, 0x6b, 0xff], [0x8b, 0x4d, 0xff], [0xd9, 0x46, 0xef], [0xff, 0x4f, 0x9a],
  [0xff, 0x70, 0x5a], [0xff, 0xb3, 0x47], [0xff, 0xe4, 0x5c], [0xb8, 0xf0, 0x4a],
  [0x4c, 0xe0, 0x6a], [0x2f, 0xd8, 0xc0], [0x3a, 0xc8, 0xff],
];

const lerp = (a: number, b: number, t: number): number => a + (b - a) * t;

export const wheelAt = (t: number): [number, number, number] => {
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
export const toHsl = (r: number, g: number, b: number): [number, number, number] => {
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

export const fromHsl = (h: number, s: number, l: number): string => {
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

/** theme.rs segment_color: wheel at the angular/ordinal midpoint, deeper
 *  rings lighten and desaturate slightly, files lighten + desaturate more. */
export const segmentColor = (midFraction: number, depth: number, isFile: boolean): string => {
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

/** Aggregates ("smaller items"): neutral shade, reads as "the rest". */
export const AGGREGATE_COLOR = '#a9a1b8';
