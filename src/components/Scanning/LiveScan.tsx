// src/components/Scanning/LiveScan.tsx
// Live-growing sunburst during the scan — squirreldisk-style progress.
// Each discovered file adds to its folder wedge (weighted by file count,
// since sizes are not known until classification finishes).

import { useMemo, useState } from 'react';
import { buildLivePathTree, VizNode } from '../../utils/vizTree';
import { Sunburst } from '../Dashboard/Sunburst';

export function LiveScan({ paths, done }: { paths: string[]; done: boolean }) {
  const [focus, setFocus] = useState<VizNode[]>([]);
  const tree = useMemo(() => buildLivePathTree(paths), [paths]);
  // While the scan is still running the tree is rebuilt continuously, so
  // zoom focus cannot be kept stable — zoom unlocks once the scan is done.
  const noop = useMemo(() => () => {}, []);
  const shownFocus = done ? focus : EMPTY_FOCUS;
  const onFocusChange = done ? setFocus : noop;

  if (paths.length === 0) return null;

  return (
    <div className="scan-viz">
      <div className="scan-viz-label">
        {done ? 'Struktura nalezených souborů' : 'Struktura roste v reálném čase'}
      </div>
      <Sunburst
        root={tree}
        metric="fileCount"
        focus={shownFocus}
        onFocusChange={onFocusChange}
      />
    </div>
  );
}

const EMPTY_FOCUS: VizNode[] = [];