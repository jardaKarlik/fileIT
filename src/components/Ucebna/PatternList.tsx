import { useState } from 'react';
import { UnclassifiedPattern } from '../../types';
import './PatternList.css';

interface PatternListProps {
  patterns: UnclassifiedPattern[];
  taught: Set<string>;
  onTeach: (patternId: string) => void;
}

export function PatternList({ patterns, taught, onTeach }: PatternListProps) {
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [expandedId, setExpandedId] = useState<string | null>(null);

  function toggleSelect(id: string) {
    setSelected(prev => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  function toggleExpand(id: string) {
    setExpandedId(prev => (prev === id ? null : id));
  }

  function confidenceBadge(pattern: UnclassifiedPattern) {
    const strongCount = pattern.signals.filter(s => s.strength === 'strong').length;
    const total = pattern.signals.length;
    if (total === 0) return { pct: 0, cls: 'conf-red' };
    const pct = Math.round((strongCount / total) * 100);
    if (pct < 60) return { pct, cls: 'conf-red' };
    if (pct <= 80) return { pct, cls: 'conf-yellow' };
    return { pct, cls: 'conf-green' };
  }

  function suggestion(pattern: UnclassifiedPattern): string {
    const inst = pattern.signals.find(s => s.label.includes('produc'));
    if (inst) return inst.label;
    if (pattern.pdf_producer) return pattern.pdf_producer;
    return '—';
  }

  return (
    <div className="pl-wrap">
      <table className="pl-table">
        <thead>
          <tr>
            <th className="pl-th-id">ID</th>
            <th className="pl-th-name">Položka v Učebně</th>
            <th className="pl-th-count"># Shodných</th>
            <th className="pl-th-conf">Jistota</th>
            <th className="pl-th-suggest">Náš Návrh</th>
            <th className="pl-th-check">Označit</th>
            <th className="pl-th-expand" />
          </tr>
        </thead>
          {patterns.map(pat => {
            const conf = confidenceBadge(pat);
            const isExpanded = expandedId === pat.pattern_id;
            const isTaught = taught.has(pat.pattern_id);

            return (
              <tbody key={pat.pattern_id} className={isTaught ? 'pl-taught' : ''}>
                <tr className={`pl-row ${isExpanded ? 'expanded' : ''}`}>
                  <td className="pl-cell-id">
                    <code>{pat.pattern_id.slice(0, 8)}</code>
                  </td>
                  <td className="pl-cell-name">
                    <div className="pl-signals-inline">
                      {pat.signals.slice(0, 2).map((s, i) => (
                        <span key={i} className={`pl-signal pl-signal-${s.strength}`}>
                          {s.label}
                        </span>
                      ))}
                      {pat.signals.length > 2 && (
                        <span className="pl-signal-more">+{pat.signals.length - 2}</span>
                      )}
                    </div>
                  </td>
                  <td className="pl-cell-count">{pat.file_count}</td>
                  <td className="pl-cell-conf">
                    <span className={`pl-conf-badge ${conf.cls}`}>
                      {conf.pct}%
                    </span>
                  </td>
                  <td className="pl-cell-suggest">{suggestion(pat)}</td>
                  <td className="pl-cell-check">
                    <input
                      type="checkbox"
                      checked={selected.has(pat.pattern_id)}
                      onChange={() => toggleSelect(pat.pattern_id)}
                    />
                  </td>
                  <td className="pl-cell-expand">
                    <button
                      className={`pl-expand-btn ${isExpanded ? 'open' : ''}`}
                      onClick={() => toggleExpand(pat.pattern_id)}
                      title={isExpanded ? 'Skrýt detaily' : 'Zobrazit detaily'}
                    >
                      ▾
                    </button>
                  </td>
                </tr>
                {isExpanded && (
                  <tr className="pl-detail-row">
                    <td colSpan={7}>
                      <div className="pl-detail">
                        <div className="pl-detail-grid">
                          <div className="pl-detail-item">
                            <span className="pl-detail-label">Plné ID</span>
                            <code className="pl-detail-value">{pat.pattern_id}</code>
                          </div>
                          <div className="pl-detail-item">
                            <span className="pl-detail-label">PDF Producer</span>
                            <span className="pl-detail-value">{pat.pdf_producer ?? '—'}</span>
                          </div>
                          <div className="pl-detail-item">
                            <span className="pl-detail-label">PDF Creator</span>
                            <span className="pl-detail-value">{pat.pdf_creator ?? '—'}</span>
                          </div>
                          <div className="pl-detail-item">
                            <span className="pl-detail-label">Velikostní skupina</span>
                            <span className="pl-detail-value">{pat.size_bucket ?? '—'}</span>
                          </div>
                        </div>

                        {pat.thumbnail_colors.length > 0 && (
                          <div className="pl-detail-item">
                            <span className="pl-detail-label">Barevné vzorky</span>
                            <div className="pl-swatches">
                              {pat.thumbnail_colors.map((c, i) => (
                                <span
                                  key={i}
                                  className="pl-swatch"
                                  style={{ background: colorBucketToHex(c) }}
                                  title={c}
                                />
                              ))}
                            </div>
                          </div>
                        )}

                        <div className="pl-detail-signals">
                          <span className="pl-detail-label">Všechny signály</span>
                          <div className="pl-signals-full">
                            {pat.signals.map((s, i) => (
                              <span key={i} className={`pl-signal pl-signal-${s.strength}`}>
                                {s.label}
                              </span>
                            ))}
                          </div>
                        </div>

                        <div className="pl-detail-actions">
                          <button
                            className="pl-btn-teach"
                            onClick={() => onTeach(pat.pattern_id)}
                            disabled={isTaught}
                          >
                            {isTaught ? '✓ Naučeno' : 'Rozpoznat tento vzor'}
                          </button>
                        </div>
                      </div>
                    </td>
                  </tr>
                )}
              </tbody>
            );
          })}
      </table>

      {selected.size > 0 && (
        <div className="pl-selection-bar">
          Vybráno: {selected.size} vzorů
        </div>
      )}
    </div>
  );
}

function colorBucketToHex(bucket: string): string {
  const hMatch = bucket.match(/H(\d+)-(\d+)/);
  if (!hMatch) return '#cccccc';
  const hue = (parseInt(hMatch[1]) + parseInt(hMatch[2])) / 2;
  return `hsl(${hue}, 60%, 55%)`;
}
