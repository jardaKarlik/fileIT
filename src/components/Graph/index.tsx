// src/components/Graph/index.tsx
// Dedicated exploration screen for the squirreldisk-style visualisation.
// Reached from the Restructure screen ("Zobrazit graf"), so the scan result
// is already in context. The user picks the hierarchy — an ordered sequence
// of FileIT dimensions (Rok → Instituce → Klient → …), one of the presets or
// freely reordered — plus the wedge metric (Velikost / Počet) and the chart
// type (Výseč / Dlaždice).

import { useEffect, useMemo, useState } from 'react';
import { useStore } from '../../store';
import {
  buildHierarchyTree,
  DIMENSION_ICONS,
  DIMENSION_LABELS,
  HIERARCHY_PRESETS,
  METRIC_LABELS,
  presetLabel,
  VizChart,
  VizDimension,
  VizMetric,
  VizNode,
} from '../../utils/vizTree';
import { Sunburst } from '../Dashboard/Sunburst';
import { Treemap } from '../Dashboard/Treemap';
import './Graph.css';

const ALL_DIMS: VizDimension[] = ['year', 'inst', 'client', 'doc', 'filetype'];

export function Graph() {
  const { setScreen, scanResult } = useStore();
  const [dims, setDims] = useState<VizDimension[]>(HIERARCHY_PRESETS[0].dims);
  const [metric, setMetric] = useState<VizMetric>('bytes');
  const [chart, setChart] = useState<VizChart>('sunburst');
  const [focus, setFocus] = useState<VizNode[]>([]);

  const tree = useMemo(
    () => buildHierarchyTree(scanResult?.files ?? [], dims, metric),
    [scanResult, dims, metric],
  );

  // Any hierarchy/metric change invalidates zoom focus.
  useEffect(() => setFocus([]), [dims, metric]);

  const move = (idx: number, delta: -1 | 1): void => {
    setDims(prev => {
      const next = [...prev];
      const target = idx + delta;
      if (target < 0 || target >= next.length) return prev;
      [next[idx], next[target]] = [next[target], next[idx]];
      return next;
    });
  };

  const remove = (idx: number): void => {
    setDims(prev => (prev.length <= 1 ? prev : prev.filter((_, i) => i !== idx)));
  };

  const add = (dim: VizDimension): void => {
    setDims(prev => (prev.includes(dim) ? prev : [...prev, dim]));
  };

  if (!scanResult || scanResult.files.length === 0) {
    return (
      <div className="graph-wrap">
        <div className="graph-empty">
          <p>Nejsou k dispozici žádná data — spusťte nejdřív analýzu.</p>
          <button className="btn-ghost" onClick={() => setScreen('dashboard')}>← Zpět na přehled</button>
        </div>
      </div>
    );
  }

  return (
    <div className="graph-wrap">
      <div className="graph-head">
        <div>
          <h2 className="graph-title">
            Prozkoumejte své soubory
            <span className="new-badge">nové</span>
          </h2>
          <p className="graph-sub">
            {scanResult.total_files} souborů · {((scanResult.total_size_bytes || 0) / 1e9).toFixed(1).replace('.', ',')} GB ·
            {' '}klikněte na výseč a projděte hierarchii do hloubky
          </p>
        </div>
        <button className="btn-ghost" onClick={() => setScreen('restructure')}>← Zpět na uspořádání</button>
      </div>

      {/* Hierarchy picker */}
      <div className="graph-controls">
        <div className="graph-block">
          <div className="graph-block-label">Hierarchie</div>
          <div className="preset-pills">
            {HIERARCHY_PRESETS.map(p => (
              <button
                key={p.id}
                className={`preset-pill${p.dims.join() === dims.join() ? ' active' : ''}`}
                onClick={() => setDims(p.dims)}
              >
                {presetLabel(p.dims)}
              </button>
            ))}
          </div>
          <div className="dim-editor">
            <span className="dim-editor-label">Pořadí úrovní:</span>
            {dims.map((d, idx) => (
              <span key={d} className="dim-chip">
                <span className="dim-chip-icon">{DIMENSION_ICONS[d]}</span>
                {DIMENSION_LABELS[d]}
                <button className="chip-btn" title="Vlevo" onClick={() => move(idx, -1)} disabled={idx === 0}>◀</button>
                <button className="chip-btn" title="Vpravo" onClick={() => move(idx, 1)} disabled={idx === dims.length - 1}>▶</button>
                <button className="chip-btn" title="Odebrat" onClick={() => remove(idx)} disabled={dims.length === 1}>✕</button>
              </span>
            ))}
            {ALL_DIMS.filter(d => !dims.includes(d)).length > 0 && (
              <span className="dim-add">
                {ALL_DIMS.filter(d => !dims.includes(d)).map(d => (
                  <button key={d} className="chip-add" onClick={() => add(d)}>+ {DIMENSION_LABELS[d]}</button>
                ))}
              </span>
            )}
          </div>
        </div>

        <div className="graph-block">
          <div className="graph-block-label">Metoda & graf</div>
          <div className="graph-toggles">
            <div className="seg-toggle">
              {(['bytes', 'count'] as VizMetric[]).map(m => (
                <button
                  key={m}
                  className={`seg-btn${metric === m ? ' active' : ''}`}
                  onClick={() => setMetric(m)}
                >
                  {METRIC_LABELS[m]}
                </button>
              ))}
            </div>
            <div className="seg-toggle">
              {(['sunburst', 'treemap'] as VizChart[]).map(c => (
                <button
                  key={c}
                  className={`seg-btn${chart === c ? ' active' : ''}`}
                  onClick={() => setChart(c)}
                >
                  {c === 'sunburst' ? '◔ Výseč' : '▦ Dlaždice'}
                </button>
              ))}
            </div>
          </div>
        </div>
      </div>

      {/* Chart */}
      <div className="graph-canvas">
        {chart === 'sunburst' ? (
          <Sunburst root={tree} metric={metric} focus={focus} onFocusChange={setFocus} />
        ) : (
          <Treemap root={tree} metric={metric} focus={focus} onFocusChange={setFocus} />
        )}
      </div>
    </div>
  );
}
