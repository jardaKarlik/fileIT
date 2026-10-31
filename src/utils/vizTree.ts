// src/utils/vizTree.ts
// Hierarchy builder for the squirreldisk-style graph screen.
// The sunburst/treemap are hierarchical, so instead of one flat grouping the
// caller picks an ordered sequence of dimensions, e.g.
//   Rok → Instituce → Klient → Typ dokumentu → Typ souboru
// Every leaf is one scanned file; parent values roll up bytes and counts.

import type { ClassifiedFile } from '../types';

/** Metric: wedge size = file bytes, or 1 per file (count). */
export type VizMetric = 'bytes' | 'count';
export type VizChart = 'sunburst' | 'treemap';

/** One level of the visual hierarchy. */
export type VizDimension = 'year' | 'inst' | 'client' | 'doc' | 'filetype' | 'folders';

export interface VizNode {
  /** Stable identity across re-renders — drives zoom morph keys. */
  key: string;
  name: string;
  /** Aggregated weight: bytes for 'bytes', file count for 'count'. */
  value: number;
  /** Total bytes, always tracked for tooltips regardless of metric. */
  bytes: number;
  /** Number of files under this node. */
  count: number;
  depth: number;
  children: VizNode[];
  /** Set when this node is the folded "smaller items" aggregate. */
  isAggregate?: boolean;
  file?: ClassifiedFile;
}

export const METRIC_LABELS: Record<VizMetric, string> = {
  bytes: 'Velikost',
  count: 'Počet souborů',
};

export const DIMENSION_LABELS: Record<VizDimension, string> = {
  year: 'Rok',
  inst: 'Instituce',
  client: 'Klient',
  doc: 'Typ dokumentu',
  filetype: 'Typ souboru',
  folders: 'Složky',
};

export const DIMENSION_ICONS: Record<VizDimension, string> = {
  year: '📅',
  inst: '🏦',
  client: '👤',
  doc: '📄',
  filetype: '🏷️',
  folders: '📁',
};

export interface HierarchyPreset {
  id: string;
  dims: VizDimension[];
}

/** Hierarchies with multiple meaningful levels, as requested. */
export const HIERARCHY_PRESETS: HierarchyPreset[] = [
  { id: 'year-inst-client-doc-filetype', dims: ['year', 'inst', 'client', 'doc', 'filetype'] },
  { id: 'client-year-doc-inst-filetype', dims: ['client', 'year', 'doc', 'inst', 'filetype'] },
  { id: 'inst-doc-client-year-filetype', dims: ['inst', 'doc', 'client', 'year', 'filetype'] },
  { id: 'folders', dims: ['folders'] },
];

export const presetLabel = (dims: VizDimension[]): string =>
  dims.map(d => DIMENSION_LABELS[d]).join(' → ');

const UNKNOWN = {
  year: 'Neznámý rok',
  inst: 'Neurčeno',
  client: 'Bez klienta',
  doc: 'Neurčený typ',
  filetype: 'Jiné',
};

const normalizeUnknown = (value: string): string =>
  value.trim().toLowerCase() === 'unknown' ? 'unknown' : value.trim().toLowerCase();

const dimOf = (dim: VizDimension, f: ClassifiedFile): { key: string; name: string } => {
  switch (dim) {
    case 'year': {
      const src = f.document_date || f.record.modified;
      const year = src ? new Date(src).getFullYear() : NaN;
      const label = Number.isNaN(year) ? UNKNOWN.year : String(year);
      return { key: `y-${label}`, name: label };
    }
    case 'inst': {
      const raw = f.institution?.canonical_name?.trim();
      const isUnknown = !raw || raw.toLowerCase() === 'unknown';
      const name = isUnknown ? UNKNOWN.inst : raw!;
      return { key: `i-${name}`, name };
    }
    case 'client': {
      const raw = f.customer?.name?.trim();
      const name = !raw || normalizeUnknown(raw) === 'unknown' ? UNKNOWN.client : raw!;
      return { key: `c-${name}`, name };
    }
    case 'doc': {
      const raw = (f.document_type || '').trim();
      const name = !raw || normalizeUnknown(raw) === 'unknown' ? UNKNOWN.doc : raw;
      return { key: `d-${name}`, name };
    }
    case 'filetype': {
      const ext = f.record.extension ? f.record.extension.toUpperCase() : UNKNOWN.filetype;
      return { key: `f-${ext}`, name: ext };
    }
    case 'folders':
      return { key: 'root', name: 'Soubory' };
  }
};

const makeNode = (key: string, name: string, depth: number): VizNode => ({
  key, name, value: 0, bytes: 0, count: 0, depth, children: [],
});

const sortChildren = (node: VizNode): void => {
  node.children.sort((a, b) => b.value - a.value);
  for (const child of node.children) sortChildren(child);
};

const rollUp = (node: VizNode): void => {
  for (const child of node.children) rollUp(child);
  if (node.children.length > 0) {
    node.bytes = node.children.reduce((s, c) => s + c.bytes, 0);
    node.count = node.children.reduce((s, c) => s + c.count, 0);
    node.value = node.children.reduce((s, c) => s + c.value, 0);
  }
};

/**
 * Build the hierarchy root for the selected dimension sequence.
 * dims = ['folders'] builds the real directory tree instead.
 */
export function buildHierarchyTree(
  files: ClassifiedFile[],
  dims: VizDimension[],
  metric: VizMetric,
): VizNode {
  const leafValue = (f: ClassifiedFile): number =>
    metric === 'count' ? 1 : f.record.size_bytes;

  if (dims[0] === 'folders') {
    const root = makeNode('root', 'Soubory', 0);
    buildFolderTree(root, files, metric);
    return root;
  }

  const buildLevel = (level: number, fs: ClassifiedFile[], parentKey: string): VizNode[] => {
    const dim = dims[level];
    const groups = new Map<string, { node: VizNode; files: ClassifiedFile[] }>();
    for (const f of fs) {
      const { key, name } = dimOf(dim, f);
      const gk = `${parentKey}/${key}`;
      let g = groups.get(gk);
      if (!g) {
        g = { node: makeNode(gk, name, level + 1), files: [] };
        groups.set(gk, g);
      }
      g.files.push(f);
    }

    const out: VizNode[] = [];
    for (const g of groups.values()) {
      if (level === dims.length - 1) {
        for (const f of g.files) {
          const leaf = makeNode(`${g.node.key}/${f.record.path}`, f.record.name, g.node.depth + 1);
          leaf.file = f;
          leaf.bytes = f.record.size_bytes;
          leaf.count = 1;
          leaf.value = leafValue(f);
          g.node.children.push(leaf);
        }
      } else {
        g.node.children = buildLevel(level + 1, g.files, g.node.key);
      }
      g.node.bytes = g.node.children.reduce((s, c) => s + c.bytes, 0);
      g.node.count = g.node.children.reduce((s, c) => s + c.count, 0);
      g.node.value = g.node.children.reduce((s, c) => s + c.value, 0);
      out.push(g.node);
    }
    return out;
  };

  const root = makeNode('root', DIMENSION_LABELS[dims[0]], 0);
  root.children = buildLevel(0, files, 'root');
  root.bytes = root.children.reduce((s, c) => s + c.bytes, 0);
  root.count = root.children.reduce((s, c) => s + c.count, 0);
  root.value = root.children.reduce((s, c) => s + c.value, 0);
  sortChildren(root);
  return root;
}

/** Real directory tree from file paths. The common directory prefix (the
 *  scan root) is stripped so ring 1 is the first branching level; files are
 *  leaves carrying their bytes. */
function buildFolderTree(root: VizNode, files: ClassifiedFile[], metric: VizMetric): void {
  const partsList = files.map(f => dirPartsOf(f.record.path));
  const prefixLen = commonDirPrefix(partsList);
  if (prefixLen > 0) root.name = partsList[0][prefixLen - 1];

  files.forEach((file, idx) => {
    const dirParts = partsList[idx].slice(prefixLen);
    let cursor = root;
    let acc = '';
    for (const part of dirParts) {
      acc += (acc ? '/' : '') + part;
      let next = cursor.children.find(c => c.name === part);
      if (!next) {
        next = makeNode(`dir-${acc}`, part, cursor.depth + 1);
        cursor.children.push(next);
      }
      cursor = next;
    }

    const fileNode = makeNode(`file-${file.record.path}`, file.record.name, cursor.depth + 1);
    fileNode.file = file;
    fileNode.bytes = file.record.size_bytes;
    fileNode.count = 1;
    fileNode.value = metric === 'count' ? 1 : file.record.size_bytes;
    cursor.children.push(fileNode);
  });

  rollUp(root);
  sortChildren(root);
  compressChains(root);
}

/** Directory segments of a path with a leading Windows drive letter dropped. */
const dirPartsOf = (path: string): string[] =>
  path.split(/[\\/]/).filter(Boolean).slice(0, -1)
    .filter((seg, i) => !(i === 0 && /^[A-Za-z]:$/.test(seg)));

/** Longest common directory-prefix length (in segments) across all paths. */
function commonDirPrefix(partsList: string[][]): number {
  if (partsList.length === 0) return 0;
  let k = partsList[0].length;
  for (const parts of partsList) {
    let i = 0;
    while (i < k && i < parts.length && parts[i] === partsList[0][i]) i += 1;
    k = i;
    if (k === 0) break;
  }
  return k;
}

/**
 * Merge pass-through directories (single-child chains) so shallow trees
 * render meaningful wedges. File leaves are never merged away, and the root
 * keeps its name.
 */
function compressChains(node: VizNode): void {
  for (const child of node.children) compressChains(child);
  if (node.depth > 0) {
    while (node.children.length === 1) {
      const only = node.children[0];
      if (only.children.length === 0) break; // file leaf — keep it separate
      node.name = `${node.name}/${only.name}`;
      node.key = `${node.key}+${only.key}`;
      node.children = only.children;
    }
  }
  for (const child of node.children) child.depth = node.depth + 1;
}

/** Format bytes the way squirreldisk's fmt_size does. */
export function fmtBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} kB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

/**
 * Fold children below `minFraction` of the parent value into an aggregate
 * "Další položky" node — squirreldisk's "smaller items" wedge.
 */
export function foldSmallChildren(node: VizNode, total: number, minFraction: number): VizNode[] {
  if (node.children.length === 0) return [];
  const sorted = [...node.children].sort((a, b) => b.value - a.value);
  const visible: VizNode[] = [];
  const small: VizNode[] = [];
  for (const child of sorted) {
    (child.value / Math.max(total, 1) < minFraction ? small : visible).push(child);
  }
  if (small.length === 0) return visible;
  if (visible.length === 0) {
    // Everything is small — show the largest few rather than one lump.
    return small.slice(0, 3);
  }
  const aggregate: VizNode = {
    key: `${node.key}/aggregate`,
    name: `Další položky (${small.length})`,
    value: small.reduce((s, c) => s + c.value, 0),
    bytes: small.reduce((s, c) => s + c.bytes, 0),
    count: small.reduce((s, c) => s + c.count, 0),
    depth: visible[0]?.depth ?? node.depth + 1,
    children: small,
    isAggregate: true,
  };
  return [...visible, aggregate];
}
