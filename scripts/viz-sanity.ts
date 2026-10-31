// scripts/viz-sanity.ts — quick runtime sanity check for vizTree (dev only).
import { buildVizTree, foldSmallChildren, fmtBytes } from '../src/utils/vizTree.ts';
import type { ClassifiedFile } from '../src/types/index.ts';

const mk = (path: string, size: number, inst: string | null, cust: string | null, dt: string): ClassifiedFile => ({
  record: { path, name: path.split('/').pop() || path, extension: 'pdf', size_bytes: size, modified: '2023-01-01', sha256: null },
  category: 'pdf',
  customer: cust ? { name: cust, rodne_cislo: null, ico: null, confidence: 0.9 } : null,
  all_customers: [],
  institution: inst ? { canonical_name: inst, category: 'bank', confidence: 0.9 } : null,
  document_type: dt as ClassifiedFile['document_type'],
  document_date: null,
  confidence: 0.9,
  ocr_used: false,
  text_preview: null,
  is_duplicate: false,
  duplicate_of: null,
  pdf_producer: null,
  pdf_creator: null,
});

const files = [
  mk('C:/vault/Ceska-sporitelna/2023/a.pdf', 4000, 'Česká spořitelna', 'Jan Novák', 'BankStatement'),
  mk('C:/vault/Ceska-sporitelna/2023/b.pdf', 2000, 'Česká spořitelna', 'Jan Novák', 'Contract'),
  mk('C:/vault/CEZ/2024/c.pdf', 1000, 'ČEZ', null, 'Invoice'),
  mk('C:/vault/Neidentifikovano/d.pdf', 500, null, null, 'Unknown'),
];

let failed = 0;
const check = (label: string, cond: boolean): void => {
  console.log(`${cond ? '✓' : '✗'} ${label}`);
  if (!cond) failed += 1;
};

const inst = buildVizTree(files, 'institution');
check('institution groups 3 top nodes', inst.children.length === 3);
check('institution total bytes 7500', inst.bytes === 7500);
check('institution sorted desc', inst.children[0].name === 'Česká spořitelna');
const cs = inst.children[0];
check('institution sub-splits doc types', cs.children.map(c => c.name).join(',') === 'BankStatement,Contract');

const clients = buildVizTree(files, 'clients');
check('clients groups 2 (Jan Novák + Bez klienta)', clients.children.length === 2);
check('clients Jan Novák count 2', clients.children[0].count === 2);

const types = buildVizTree(files, 'fileTypes');
check('fileTypes single PDF group', types.children.length === 1 && types.children[0].name === 'PDF');
check('fileTypes sub-groups institutions', types.children[0].children.length === 3);

const count = buildVizTree(files, 'fileCount');
check('fileCount values are counts (3/1/…)', count.children.map(c => c.value).join(',') === '2,1,1');

const folders = buildVizTree(files, 'folders');
check('folders rooted at scan root (common prefix stripped)', folders.children.length === 3);
check('folders root named after scan root', folders.name === 'vault');
check('folder sizes roll up', folders.bytes === 7500);

const folded = foldSmallChildren(inst, inst.value, 0.2);
check('foldSmallChildren keeps aggregate', folded.some(c => c.isAggregate === true));

check('fmtBytes', fmtBytes(7500) === '7.3 kB');

console.log(failed === 0 ? 'ALL PASSED' : `${failed} FAILED`);
process.exit(failed === 0 ? 0 : 1);