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
console.log('top:', t.children.map(c => `${c.name}=${c.count}`).join(', '));
const users = t.children.find(c => c.name === 'Users');
const docs = users?.children.find(c => c.name === 'jaros')?.children.find(c => c.name === 'Documents');
console.log('Documents count (incl. Photos child)', docs?.count, '(expect 3)');
const ok = docs?.count === 3 && t.count === 4;
console.log(ok ? 'LIVE TREE OK' : 'LIVE TREE BROKEN');
process.exit(ok ? 0 : 1);