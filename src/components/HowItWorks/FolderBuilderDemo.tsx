// src/components/HowItWorks/FolderBuilderDemo.tsx
// Interactive folder-structure builder on 600 fictional documents.
// Uses the same grouping as the real Restructure screen (utils/folderTree).
// Demo data only — nothing here touches files.

import { useMemo, useState } from 'react';
import type { GroupDimension } from '../../types';
import { DIMENSIONS, UNKNOWN, groupBy, countFolders, type TreeNode } from '../../utils/folderTree';
import { DEMO_FILES, DEMO_FILE_COUNT } from '../../utils/demoDataset';

const PRESETS: { label: string; dims: GroupDimension[] }[] = [
  { label: 'Klient → Měsíc', dims: ['customer', 'date'] },
  { label: 'Klient → Instituce → Typ', dims: ['customer', 'inst', 'doc'] },
  { label: 'Instituce → Typ', dims: ['inst', 'doc'] },
  { label: 'Typ → Klient', dims: ['doc', 'customer'] },
];

const MAX_LEVELS = 3;
const TOP_SHOWN = 8;
const CHILD_SHOWN = 4;

export function FolderBuilderDemo() {
  const [dims, setDims] = useState<GroupDimension[]>(['customer', 'inst', 'doc']);
  const [open, setOpen] = useState<string | null>(null);

  const tree = useMemo(() => groupBy(DEMO_FILES, dims), [dims]);
  const counts = useMemo(() => countFolders(tree), [tree]);
  const topKeys = Object.keys(tree);
  const openKey = open && tree[open] ? open : topKeys[0] ?? null;

  // Example path: first file of the first folder at every level.
  const examplePath = useMemo(() => {
    const parts: string[] = [];
    let node: Record<string, TreeNode> | null = tree;
    while (node && Object.keys(node).length) {
      const k: string = Object.keys(node)[0];
      parts.push(k);
      node = node[k].children;
    }
    const match = DEMO_FILES.find(f => dims.every((d, i) => (f[d] ?? UNKNOWN[d]) === parts[i]));
    return parts.length ? `Organized\\${parts.join('\\')}\\${match?.name ?? 'dokument.pdf'}` : '';
  }, [tree, dims]);

  const add = (d: GroupDimension) => setDims(cur => (cur.includes(d) || cur.length >= MAX_LEVELS ? cur : [...cur, d]));
  const remove = (d: GroupDimension) => setDims(cur => cur.filter(x => x !== d));
  const move = (d: GroupDimension, delta: -1 | 1) => setDims(cur => {
    const i = cur.indexOf(d), j = i + delta;
    if (i < 0 || j < 0 || j >= cur.length) return cur;
    const next = [...cur];
    [next[i], next[j]] = [next[j], next[i]];
    return next;
  });

  return (
    <div className="fb-demo">
      <div className="fb-presets" role="group" aria-label="Rychlé předvolby">
        {PRESETS.map(p => {
          const active = p.dims.join() === dims.join();
          return (
            <button key={p.label} type="button" className={`fb-preset ${active ? 'active' : ''}`}
              aria-pressed={active} onClick={() => { setDims(p.dims); setOpen(null); }}>
              {p.label}
            </button>
          );
        })}
      </div>

      <div className="fb-grid">
        {/* Palette + stack */}
        <div className="fb-col">
          <div className="fb-label">Dostupné dimenze</div>
          <div className="fb-palette">
            {(Object.keys(DIMENSIONS) as GroupDimension[]).map(d => {
              const lvl = dims.indexOf(d);
              const used = lvl >= 0;
              const maxed = !used && dims.length >= MAX_LEVELS;
              return (
                <button key={d} type="button" className={`fb-chip ${used ? `used lvl${lvl + 1}` : ''}`}
                  disabled={maxed} onClick={() => (used ? remove(d) : add(d))}
                  aria-pressed={used} title={used ? 'Kliknutím odeberete' : 'Kliknutím přidáte'}>
                  <span className="fb-chip-icon">{DIMENSIONS[d].icon}</span>
                  <span className="fb-chip-text">
                    <span className="fb-chip-name">{DIMENSIONS[d].name}</span>
                    <span className="fb-chip-meta">např. {DIMENSIONS[d].example}</span>
                  </span>
                  <span className="fb-chip-state">{used ? `${lvl + 1}. úroveň` : maxed ? 'Max.' : '+ Přidat'}</span>
                </button>
              );
            })}
          </div>

          <div className="fb-label" style={{ marginTop: 16 }}>Vaše struktura</div>
          <ol className="fb-stack">
            {dims.map((d, i) => (
              <li key={d} className="fb-stack-row">
                <span className={`fb-stack-num lvl${i + 1}`}>{i + 1}</span>
                <span className="fb-stack-name">{DIMENSIONS[d].name}</span>
                <button type="button" className="fb-icon-btn" disabled={i === 0} onClick={() => move(d, -1)} aria-label={`Posunout ${DIMENSIONS[d].name} výš`}>▲</button>
                <button type="button" className="fb-icon-btn" disabled={i === dims.length - 1} onClick={() => move(d, 1)} aria-label={`Posunout ${DIMENSIONS[d].name} níž`}>▼</button>
                <button type="button" className="fb-icon-btn" onClick={() => remove(d)} aria-label={`Odebrat ${DIMENSIONS[d].name}`}>×</button>
              </li>
            ))}
            {dims.length === 0 && <li className="fb-stack-empty">Vyberte alespoň jednu dimenzi.</li>}
          </ol>
        </div>

        {/* Live tree */}
        <div className="fb-col fb-tree-col">
          <div className="fb-stats">
            <span><strong>{DEMO_FILE_COUNT}</strong> ukázkových dokumentů</span>
            <span><strong>{counts.top}</strong> složek</span>
            <span><strong>{counts.sub}</strong> podsložek</span>
          </div>
          {dims.length === 0 ? (
            <div className="fb-tree-empty">Zatím žádná struktura.</div>
          ) : (
            <div className="fb-tree" role="tree" aria-label="Náhled složek">
              <div className="fb-tree-root">📁 Organized</div>
              {topKeys.slice(0, TOP_SHOWN).map(k => {
                const node = tree[k];
                const isOpen = k === openKey;
                return (
                  <div key={k} role="treeitem" aria-expanded={node.children ? isOpen : undefined}>
                    <button type="button" className={`fb-node ${node.isUnknown ? 'unknown' : ''} ${isOpen ? 'open' : ''}`}
                      onClick={() => setOpen(isOpen ? '' : k)}>
                      <span className="fb-caret">{node.children ? (isOpen ? '▾' : '▸') : ' '}</span>
                      <span className="fb-node-name">📁 {k}</span>
                      <span className="fb-node-count">{node.count}</span>
                    </button>
                    {isOpen && node.children && (
                      <div className="fb-children">
                        {Object.keys(node.children).slice(0, CHILD_SHOWN).map(ck => {
                          const child = node.children![ck];
                          return (
                            <div key={ck}>
                              <div className={`fb-node child ${child.isUnknown ? 'unknown' : ''}`}>
                                <span className="fb-node-name">📁 {ck}</span>
                                <span className="fb-node-count">{child.count}</span>
                              </div>
                              {child.children && (
                                <div className="fb-grandchildren">
                                  {Object.keys(child.children).slice(0, 3).map(gk => (
                                    <div key={gk} className={`fb-node grandchild ${child.children![gk].isUnknown ? 'unknown' : ''}`}>
                                      <span className="fb-node-name">📁 {gk}</span>
                                      <span className="fb-node-count">{child.children![gk].count}</span>
                                    </div>
                                  ))}
                                  {Object.keys(child.children).length > 3 && (
                                    <div className="fb-more">+ {Object.keys(child.children).length - 3} dalších</div>
                                  )}
                                </div>
                              )}
                            </div>
                          );
                        })}
                        {Object.keys(node.children).length > CHILD_SHOWN && (
                          <div className="fb-more">+ {Object.keys(node.children).length - CHILD_SHOWN} dalších složek</div>
                        )}
                      </div>
                    )}
                  </div>
                );
              })}
              {topKeys.length > TOP_SHOWN && <div className="fb-more">+ {topKeys.length - TOP_SHOWN} dalších složek</div>}
            </div>
          )}
          {examplePath && (
            <div className="fb-example">
              <span className="fb-example-label">Příklad cesty</span>
              <code>{examplePath}</code>
            </div>
          )}
        </div>
      </div>

      <p className="fb-note">
        Ukázková data: smyšlení klienti, skutečné české instituce. Složky „Neznámý klient“, „Bez instituce“
        a „Neurčený typ“ ukazují, kam FileIT dá dokumenty, u kterých údaj nenašel. Před skutečným přesunem
        vždy uvidíte náhled a můžete vše vrátit zpět.
      </p>
    </div>
  );
}
