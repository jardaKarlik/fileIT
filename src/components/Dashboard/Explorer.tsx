// src/components/Dashboard/Explorer.tsx
// squirreldisk-style explorer: pick the metric (institution by default,
// switchable to clients / file types / file count / folders), pick the chart
// (sunburst or treemap), click to zoom, breadcrumb to go back.

import { useEffect, useMemo, useState } from 'react';
import { useStore } from '../../store';
import {
  buildVizTree, METRIC_LABELS, VizChart, VizMetric, VizNode,
} from '../../utils/vizTree';
import { Sunburst } from './Sunburst';
import { Treemap } from './Treemap';

const METRIC_ORDER: VizMetric[] = [
  'institution', 'clients', 'fileTypes', 'fileCount', 'folders',
];

export function Explorer() {
  const scanResult = useStore(s => s.scanResult);
  const [metric, setMetric] = useState<VizMetric>('institution');
  const [chart, setChart] = useState<VizChart>('sunburst');
  const [focus, setFocus] = useState<VizNode[]>([]);

  const tree = useMemo(
    () => buildVizTree(scanResult?.files ?? [], metric),
    [scanResult, metric],
  );

  // Metric switches change the hierarchy — zoom focus can't survive it.
  useEffect(() => setFocus([]), [metric]);

  if (!scanResult || scanResult.files.length === 0) {
    return (
      <div className="chart-card explorer-card">
        <div className="chart-title">Prozkoumat</div>
        <div className="explorer-empty">Po analýze zde uvidíte vizualizaci souborů.</div>
      </div>
    );
  }

  return (
    <div className="chart-card explorer-card">
      <div className="chart-title">
        Prozkoumat
        <span className="chart-tag">
          {METRIC_LABELS[metric]} · {scanResult.total_files} souborů
        </span>
      </div>

      <div className="explorer-controls">
        <div className="metric-pills" role="tablist">
          {METRIC_ORDER.map(m => (
            <button
              key={m}
              role="tab"
              aria-selected={m === metric}
              className={`metric-pill${m === metric ? ' active' : ''}`}
              onClick={() => setMetric(m)}
            >
              {METRIC_LABELS[m]}
            </button>
          ))}
        </div>
        <div className="chart-toggle">
          <button
            className={`toggle-btn${chart === 'sunburst' ? ' active' : ''}`}
            onClick={() => setChart('sunburst')}
            title="Výsečový graf"
          >
            ◔ Výseč
          </button>
          <button
            className={`toggle-btn${chart === 'treemap' ? ' active' : ''}`}
            onClick={() => setChart('treemap')}
            title="Treemap"
          >
            ▦ Dlaždice
          </button>
        </div>
      </div>

      <div className="explorer-canvas">
        {chart === 'sunburst' ? (
          <Sunburst root={tree} metric={metric} focus={focus} onFocusChange={setFocus} />
        ) : (
          <Treemap root={tree} metric={metric} focus={focus} onFocusChange={setFocus} />
        )}
      </div>
    </div>
  );
}