// src/utils/vizTree.ts
// Builds the visualization hierarchy from a ScanResult for the sunburst /
// treemap explorer. Inspired by squirreldisk's approach (src/ui/sunburst.rs):
// children sorted descending, small children fold into an aggregate wedge,
// and every node has a stable key so zoom animations can morph segments.

import type { ClassifiedFile } from '../types';

export type VizMetric =
  | 'institution'   // default: group by institution
  | 'clients'       // group by customer
  | 'fileTypes'     // group by category
  | 'fileCount'     // same shape as institution, weight = 1 per file
  | 'folders';      // real path-segment tree, weight = bytes

export type VizChart = 'sunburst' | 'treemap';

export interface VizNode {
  /** Stable identity across re-renders and metric switches within a file. */
  key: string;
  name: string;
  /** Aggregated weight: bytes for size metrics, file count for 'fileCount'. */
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

const UNKNOWN_INSTITUTION = 'Neurčeno';
const UNKNOWN_CLIENT = 'Bez klienta';
const UNKNOWN_TYPE = 'Jiné';

const CATEGORY_LABELS: Record<string, string> = {
  pdf: 'PDF',
  word: 'Word',
  excel: 'Excel',
  powerpoint: 'PowerPoint',
  csv: 'CSV',
  xml: 'XML',
  image: 'Obrázky',
  email: 'E-mail',
  text: 'Text',
  other: 'Jiné',
};

export const METRIC_LABELS: Record<VizMetric, string> = {
  institution: 'Instituce',
  clients: 'Klienti',
  fileTypes: 'Typy souborů',
  fileCount: 'Počet souborů',
  folders: 'Složky',
};

const leafValue = (metric: VizMetric, bytes: number, count: number): number =>
  metric === 'fileCount' ? count : bytes;

const makeNode = (key: string, name: string, depth: number): VizNode => ({
  key, name, value: 0, bytes: 0, count: 0, depth, children: [],
});

/**
 * Build the root node for the selected metric. The returned tree is always a
 * single root so both sunburst and treemap can start from the same node.
 */
export function buildVizTree(files: ClassifiedFile[], metric: VizMetric): VizNode {
  const rootName =
    metric === 'folders' ? 'Kořen'
    : metric === 'fileCount' ? 'Všechny soubory'
    : metric === 'clients' ? 'Klienti'
    : metric === 'fileTypes' ? 'Typy souborů'
    : 'Instituce';

  const root = makeNode('root', rootName, 0);

  if (metric === 'folders') {
    buildFolderTree(root, files);
    return root;
  }

  const groupOf = (f: ClassifiedFile): string => {
    switch (metric) {
      case 'institution':
      case 'fileCount':
        return f.institution?.canonical_name || UNKNOWN_INSTITUTION;
      case 'clients':
        return f.customer?.name || UNKNOWN_CLIENT;
      case 'fileTypes':
        return CATEGORY_LABELS[f.category] || UNKNOWN_TYPE;
    }
  };

  const subOf = (f: ClassifiedFile): { key: string; name: string } => {
    if (metric === 'fileTypes') {
      return {
        key: `inst-${f.institution?.canonical_name || UNKNOWN_INSTITUTION}`,
        name: f.institution?.canonical_name || UNKNOWN_INSTITUTION,
      };
    }
    const dt = f.document_type === 'Unknown' ? 'Neurčený typ' : f.document_type;
    return { key: `dt-${dt}`, name: dt };
  };

  const groups = new Map<string, VizNode>();

  for (const file of files) {
    const groupName = groupOf(file);
    let group = groups.get(groupName);
    if (!group) {
      group = makeNode(`grp-${groupName}`, groupName, 1);
      groups.set(groupName, group);
    }

    const sub = subOf(file);
    const subKey = `${group.key}/${sub.key}`;
    let subNode = group.children.find(c => c.key === subKey);
    if (!subNode) {
      subNode = makeNode(subKey, sub.name, 2);
      group.children.push(subNode);
    }

    const fileKey = `file-${file.record.path}`;
    let fileNode = subNode.children.find(c => c.key === fileKey);
    if (!fileNode) {
      fileNode = makeNode(fileKey, file.record.name, 3);
      fileNode.file = file;
      subNode.children.push(fileNode);
    }

    // Leaves carry their own metric value; parents are rolled up below.
    fileNode.bytes = file.record.size_bytes;
    fileNode.count = 1;
    fileNode.value = leafValue(metric, file.record.size_bytes, 1);
  }

  // Roll values up the chain (children are already sorted later).
  for (const group of groups.values()) {
    for (const sub of group.children) {
      sub.bytes = sub.children.reduce((s, c) => s + c.bytes, 0);
      sub.count = sub.children.reduce((s, c) => s + c.count, 0);
      sub.value = sub.children.reduce((s, c) => s + c.value, 0);
    }
    group.bytes = group.children.reduce((s, c) => s + c.bytes, 0);
    group.count = group.children.reduce((s, c) => s + c.count, 0);
    group.value = group.children.reduce((s, c) => s + c.value, 0);
  }
  root.children = [...groups.values()];
  root.bytes = root.children.reduce((s, c) => s + c.bytes, 0);
  root.count = root.children.reduce((s, c) => s + c.count, 0);
  root.value = leafValue(metric, root.bytes, root.count);

  sortChildren(root);
  return root;
}

/** Real directory tree from file paths — the squirreldisk-native view. */
function buildFolderTree(root: VizNode, files: ClassifiedFile[]): void {
  for (const file of files) {
    const parts = file.record.path.split(/[\\/]/).filter(Boolean);
    // Drop a leading Windows drive segment ("C:") — the scan root already
    // implies the volume, and a one-tile drive ring adds no information.
    const dirParts = parts.slice(0, -1).filter((p, i) => !(i === 0 && /^[A-Za-z]:$/.test(p)));
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
    fileNode.value = file.record.size_bytes;
    cursor.children.push(fileNode);
  }

  const roll = (node: VizNode): void => {
    if (node.children.length === 0) return;
    for (const child of node.children) {
      roll(child);
      node.bytes += child.bytes;
      node.count += child.count;
      node.value += child.value;
    }
  };
  roll(root);
  sortChildren(root);
}

function sortChildren(node: VizNode): void {
  node.children.sort((a, b) => b.value - a.value);
  for (const child of node.children) sortChildren(child);
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
 * "Další položky" node — squirreldisk's "smaller items" wedge (MIN_PX).
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
