// src/components/Home/SunburstGlyph.tsx
// Decorative mini-sunburst for the "Nová vizualizace" entry point — a static
// echo of the Graph screen chart: dark disc center, thin rings, wedges
// coloured by the same cyclic hue wheel, file leaves as pale spikes.
// Deterministic (seeded) so it never changes between renders.

const WHEEL: [number, number, number][] = [
  [0x39, 0x6b, 0xff], [0x8b, 0x4d, 0xff], [0xd9, 0x46, 0xef], [0xff, 0x4f, 0x9a],
  [0xff, 0x70, 0x5a], [0xff, 0xb3, 0x47], [0xff, 0xe4, 0x5c], [0xb8, 0xf0, 0x4a],
  [0x4c, 0xe0, 0x6a], [0x2f, 0xd8, 0xc0], [0x3a, 0xc8, 0xff],
];

const lerp = (a: number, b: number, t: number): number => a + (b - a) * t;

const wheelAt = (t: number): string => {
  const n = WHEEL.length;
  const x = (((t % 1) + 1) % 1) * n;
  const i = Math.floor(x) % n;
  const j = (i + 1) % n;
  const f = x - Math.floor(x);
  const r = Math.round(lerp(WHEEL[i][0], WHEEL[j][0], f));
  const g = Math.round(lerp(WHEEL[i][1], WHEEL[j][1], f));
  const b = Math.round(lerp(WHEEL[i][2], WHEEL[j][2], f));
  return `rgb(${r},${g},${b})`;
};

// Seeded LCG — stable decoration, no re-render flicker.
const rng = (seed: number): (() => number) => {
  let s = seed >>> 0;
  return () => {
    s = (s * 1664525 + 1013904223) >>> 0;
    return s / 4294967296;
  };
};

interface Ring {
  r0: number;
  r1: number;
  slices: number;
  seed: number;
  fileRing?: boolean;
}

const RINGS: Ring[] = [
  { r0: 0.38, r1: 0.55, slices: 5, seed: 11 },
  { r0: 0.55, r1: 0.68, slices: 9, seed: 23 },
  { r0: 0.68, r1: 0.80, slices: 16, seed: 37 },
  { r0: 0.80, r1: 0.90, slices: 26, seed: 51 },
  { r0: 0.90, r1: 0.98, slices: 40, seed: 67, fileRing: true },
];

export function SunburstGlyph({ size = 56 }: { size?: number }) {
  const c = size / 2;
  const paths: JSX.Element[] = [];

  for (const ring of RINGS) {
    const rand = rng(ring.seed);
    // Angular gaps mimic free space; wedges sorted desc within the ring.
    const weights: number[] = [];
    for (let i = 0; i < ring.slices; i += 1) {
      weights.push((1 - i / ring.slices) * (0.4 + rand() * 0.6) + 0.04);
    }
    const total = weights.reduce((s, w) => s + w, 0);
    const fill = ring.fileRing ? 0.82 : 0.94; // leaves leave more gaps
    let a = 0;
    for (let i = 0; i < ring.slices; i += 1) {
      const span = (weights[i] / total) * Math.PI * 2 * fill;
      if (span <= 0.004) { a += span; continue; }
      const mid = (a + span / 2) / (Math.PI * 2);
      const col = wheelAt(mid);
      const p = (r: number, ang: number): string =>
        `${(c + r * Math.sin(ang)).toFixed(2)} ${(c - r * Math.cos(ang)).toFixed(2)}`;
      const large = span > Math.PI ? 1 : 0;
      const d = [
        `M ${p(ring.r1 * c, a)}`,
        `A ${(ring.r1 * c).toFixed(2)} ${(ring.r1 * c).toFixed(2)} 0 ${large} 1 ${p(ring.r1 * c, a + span)}`,
        `L ${p(ring.r0 * c, a + span)}`,
        `A ${(ring.r0 * c).toFixed(2)} ${(ring.r0 * c).toFixed(2)} 0 ${large} 0 ${p(ring.r0 * c, a)}`,
        'Z',
      ].join(' ');
      paths.push(
        <path
          key={`${ring.seed}-${i}`}
          d={d}
          fill={col}
          opacity={ring.fileRing ? 0.95 : 1}
        />,
      );
      a += span;
    }
  }

  return (
    <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} aria-hidden="true">
      <circle cx={c} cy={c} r={c} fill="#141021" />
      {paths}
      <circle cx={c} cy={c} r={size * 0.155} fill="#141021" />
    </svg>
  );
}