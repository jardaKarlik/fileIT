// scripts/viz-sanity.ts — quick runtime sanity check for the hierarchy
// builder (dev only). Run: node --experimental-strip-types scripts/viz-sanity.ts
import { buildHierarchyTree, foldSmallChildren, fmtBytes, HIERARCHY_PRESETS } from '../src/utils/vizTree.ts';
import type { ClassifiedFile } from '../src/types/index.ts';

const mk = (path: string, size: number, inst: string | null, cust: string | null, dt: string, date: string): ClassifiedFile => ({
  record: { path, name: path.split('/').pop() || path, extension: 'pdf', size_bytes: size, modified: date, sha256: null },
  category: 'pdf',
  customer: cust ? { name: cust, rodne_cislo: null, ico: null, confidence: 0.9 } : null,
  all_customers: [],
  institution: inst ? { canonical_name: inst, category: 'bank', confidence: 0.9 } : null,
  document_type: dt as ClassifiedFile['document_type'],
  document_date: date,
  confidence: 0.9,
  ocr_used: false,
  text_preview: null,
  is_duplicate: false,
  duplicate_of: null,
  pdf_producer: null,
  pdf_creator: null,
});

const files = [
  mk('C:/vault/Ceska-sporitelna/2023/a.pdf', 4000, 'Česká spořitelna', 'Jan Novák', 'BankStatement', '2023-01-05'),
  mk('C:/vault/Ceska-sporitelna/2023/b.pdf', 2000, 'Česká spořitelna', 'Jan Novák', 'Contract', '2023-02-05'),
  mk('C:/vault/CEZ/2024/c.pdf', 1000, 'ČEZ', null, 'Invoice', '2024-03-05'),
  mk('C:/vault/Neidentifikovano/d.pdf', 500, null, null, 'Unknown', '2022-06-05'),
];

let failed = 0;
const check = (label: string, cond: boolean): void => {
  console.log(`${cond ? '✓' : '✗'} ${label}`);
  if (!cond) failed += 1;
};

// Preset 1: Rok → Instituce → Klient → Typ → Typ souboru
const p1 = buildHierarchyTree(files, HIERARCHY_PRESETS[0].dims, 'bytes');
check('preset1 groups by year (3 at ring 1)', p1.children.length === 3);
check('preset1 sorted desc (2023 first)', p1.children[0].name === '2023');
const y2023 = p1.children.find(c => c.name === '2023')!;
check('preset1 year rolls bytes', y2023.bytes === 6000);
check('preset1 second level institutions', y2023.children[0].name === 'Česká spořitelna');
const inst = y2023.children[0];
check('preset1 third level clients', inst.children[0].name === 'Jan Novák');
check('preset1 leaves are files with sizes', inst.children[0].children.map(dt => dt.children[0].bytes).join(',') === '4000,2000');

// Preset 2: Klient first
const p2 = buildHierarchyTree(files, HIERARCHY_PRESETS[1].dims, 'bytes');
check('preset2 client-first grouping', p2.children.length === 2 && p2.children[0].name === 'Jan Novák');

// Preset 3: Instituce → Typ dokumentu → …
const p3 = buildHierarchyTree(files, HIERARCHY_PRESETS[2].dims, 'count');
check('preset3 institution-first', p3.children.length === 3);
const cs3 = p3.children.find(c => c.name === 'Česká spořitelna')!;
check('preset3 count metric rolls counts', cs3.count === 2 && cs3.children.length === 2);

// Folders preset: real path tree rooted at the scan root
const pf = buildHierarchyTree(files, HIERARCHY_PRESETS[3].dims, 'bytes');
check('folders preset roots at scan root', pf.name === 'vault');
check('folders preset rolls bytes', pf.bytes === 7500);

check('foldSmallChildren keeps aggregate', foldSmallChildren(p1, p1.value, 0.2).some(c => c.isAggregate === true));
check('fmtBytes', fmtBytes(7500) === '7.3 kB');

console.log(failed === 0 ? 'ALL PASSED' : `${failed} FAILED`);
process.exit(failed === 0 ? 0 : 1);