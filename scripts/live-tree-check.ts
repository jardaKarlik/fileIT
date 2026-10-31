// scripts/live-tree-check.ts — verify the live scan tree (dev only).
import { buildLivePathTree } from '../src/utils/vizTree.ts';

const paths = [
  'C:\\Users\\jaros\\Documents\\Docs\\a.pdf',
  'C:\\Users\\jaros\\Documents\\Docs\\b.pdf',
  'C:\\Users\\jaros\\Documents\\Photos\\c.jpg',
  'C:\\Vault\\x.pdf',
];

const t = buildLivePathTree(paths);
console.log('root count', t.count, '(expect 4)');
console.log('top (compressed):', t.children.map(c => `${c.name}=${c.count}`).join(', '));
// Users/jaros/Documents is a single-child chain until Docs/Photos split it,
// so compression must produce one merged node with two children.
const merged = t.children[0];
const ok =
  t.count === 4 &&
  merged.name === 'Users/jaros/Documents' &&
  merged.children.length === 2 &&
  merged.count === 3;
console.log(ok ? 'LIVE TREE OK' : 'LIVE TREE BROKEN');
process.exit(ok ? 0 : 1);