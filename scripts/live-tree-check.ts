// scripts/live-tree-check.ts — verify the live scan tree (dev only).
import { buildLivePathTree } from '../src/utils/vizTree.ts';

const paths = [
  'C:\\Users\\jaros\\Documents\\Docs\\a.pdf',
  'C:\\Users\\jaros\\Documents\\Docs\\b.pdf',
  'C:\\Users\\jaros\\Documents\\Photos\\c.jpg',
  'C:\\Vault\\x.pdf',
];

// Mixed roots: no common prefix → generic 'Sken' root with top-level children.
const mixed = buildLivePathTree(paths);
console.log('mixed root', mixed.name, '| top:', mixed.children.map(c => `${c.name}=${c.count}`).join(', '));
const mixedOk =
  mixed.count === 4 &&
  mixed.name === 'Sken' &&
  mixed.children.some(c => c.name === 'Users/jaros/Documents' && c.count === 3) &&
  mixed.children.some(c => c.name === 'Vault');

// Single scan root: common prefix is stripped, root renamed to the scan root,
// ring 1 shows the branching folders directly.
const single = buildLivePathTree(paths.slice(0, 3));
console.log('single root', single.name, '| top:', single.children.map(c => `${c.name}=${c.count}`).join(', '));
const singleOk =
  single.count === 3 &&
  single.name === 'Documents' &&
  single.children.find(c => c.name === 'Docs')?.count === 2 &&
  single.children.find(c => c.name === 'Photos')?.count === 1;

console.log(mixedOk && singleOk ? 'LIVE TREE OK' : 'LIVE TREE BROKEN');
process.exit(mixedOk && singleOk ? 0 : 1);