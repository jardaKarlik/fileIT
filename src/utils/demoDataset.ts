// src/utils/demoDataset.ts
// 600 fictional documents for the folder-builder demo ("Jak to funguje")
// and the Restructure screen's sample-data mode. Client names are invented;
// institutions are real Czech institutions; document types and folder names
// are exactly the ones the classifier produces. Deterministic (seeded) so
// the demo looks the same every time.

import type { FileObj } from './folderTree';

const CLIENTS = [
  'Novák Pavel', 'Horáková Jana', 'Dvořák Martin', 'Svobodová Lucie', 'Černý Tomáš',
  'Procházková Eva', 'Kučera Jiří', 'Veselá Hana', 'Krejčí Petr', 'Marková Kateřina',
  'Pokorný Jan', 'Benešová Alena', 'Fiala Ondřej', 'Sedláčková Martina', 'Doležal Michal',
  'Zemanová Petra', 'Kolář David', 'Navrátilová Lenka', 'Čermák Lukáš', 'Urbanová Veronika',
  'Vaněk Roman', 'Blažková Monika', 'Kříž Josef', 'Kovářová Ivana', 'Bartoš Filip',
  'Vlčková Simona', 'Růžička Karel', 'Šimková Zuzana', 'Kopecký Daniel', 'Holubová Tereza',
  'Hájek Milan', 'Musilová Jitka', 'Matoušek Vojtěch', 'Kadlecová Barbora', 'Šťastný Radek',
  'Říhová Klára', 'Kratochvíl Adam', 'Pospíšilová Markéta', 'Moravec Stanislav', 'Tichá Pavla',
];

type InstGroup = { names: string[]; docs: string[] };

// Institution groups and the document types each one realistically issues.
const INSTITUTIONS: InstGroup[] = [
  { names: ['Česká spořitelna', 'ČSOB', 'Komerční banka', 'Raiffeisenbank', 'MONETA Money Bank', 'Air Bank', 'Fio banka'],
    docs: ['Výpis z účtu', 'Smlouva', 'Úvěr a hypotéka', 'Dopis z banky', 'Dodatek ke smlouvě', 'Souhlas', 'Výpověď'] },
  { names: ['Kooperativa', 'Allianz', 'Generali Česká pojišťovna', 'ČPP', 'UNIQA'],
    docs: ['Pojistná smlouva', 'Dodatek ke smlouvě', 'Výpověď', 'Souhlas', 'Plná moc', 'Platba a upomínka'] },
  { names: ['ČEZ', 'E.ON', 'Pražská plynárenská', 'innogy'],
    docs: ['Vyúčtování energií', 'Smlouva', 'Faktura', 'Výpověď'] },
  { names: ['O2', 'T-Mobile', 'Vodafone'],
    docs: ['Faktura', 'Smlouva', 'Výpověď'] },
  { names: ['Finanční úřad'],
    docs: ['Daňové přiznání', 'Rozhodnutí úřadu', 'Platba a upomínka'] },
  { names: ['ČSSZ', 'VZP', 'OZP'],
    docs: ['Rozhodnutí úřadu', 'Žádost', 'Platba a upomínka'] },
  { names: ['Katastrální úřad'],
    docs: ['List vlastnictví (LV)', 'Žádost'] },
  { names: ['Úřad práce'],
    docs: ['Žádost', 'Rozhodnutí úřadu'] },
  { names: ['Notářství Mgr. Malá', 'Advokátní kancelář Fiala'],
    docs: ['Plná moc', 'Právní dokument', 'Smlouva'] },
  { names: ['Magistrát města Brna', 'Městský úřad Kolín'],
    docs: ['Osobní doklady', 'Žádost', 'Rozhodnutí úřadu'] },
];

// Group weights: banks and insurers dominate an advisor's archive.
const GROUP_WEIGHTS = [30, 22, 10, 8, 8, 7, 5, 3, 4, 3];

const FILE_STEM: Record<string, string> = {
  'Výpis z účtu': 'vypis', 'Smlouva': 'smlouva', 'Úvěr a hypotéka': 'uverova-smlouva',
  'Dopis z banky': 'dopis', 'Dodatek ke smlouvě': 'dodatek', 'Souhlas': 'souhlas-gdpr',
  'Výpověď': 'vypoved', 'Pojistná smlouva': 'pojistna-smlouva', 'Plná moc': 'plna-moc',
  'Platba a upomínka': 'upominka', 'Vyúčtování energií': 'vyuctovani', 'Faktura': 'faktura',
  'Daňové přiznání': 'danove-priznani', 'Rozhodnutí úřadu': 'rozhodnuti', 'Žádost': 'zadost',
  'List vlastnictví (LV)': 'LV', 'Právní dokument': 'pravni-dokument', 'Osobní doklady': 'doklad',
};

/** Small seeded PRNG (mulberry32) so the demo is reproducible. */
function rng(seed: number) {
  return () => {
    seed |= 0; seed = (seed + 0x6D2B79F5) | 0;
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function pickWeighted(r: () => number, weights: number[]): number {
  const total = weights.reduce((a, b) => a + b, 0);
  let x = r() * total;
  for (let i = 0; i < weights.length; i++) {
    x -= weights[i];
    if (x < 0) return i;
  }
  return weights.length - 1;
}

export const DEMO_FILE_COUNT = 600;

export const DEMO_FILES: FileObj[] = (() => {
  const r = rng(20260601);
  const files: FileObj[] = [];
  // Uneven client sizes, like a real archive (a few big clients, many small).
  const clientWeights = CLIENTS.map((_, i) => Math.max(2, 26 - i * 0.6));
  for (let i = 0; i < DEMO_FILE_COUNT; i++) {
    const group = INSTITUTIONS[pickWeighted(r, GROUP_WEIGHTS)];
    const inst = group.names[Math.floor(r() * group.names.length)];
    const doc = group.docs[Math.floor(r() * group.docs.length)];
    const client = CLIENTS[pickWeighted(r, clientWeights)];
    const year = 2023 + Math.floor(r() * 4);              // 2023–2026
    const month = 1 + Math.floor(r() * (year === 2026 ? 10 : 12));
    const date = `${year}-${String(month).padStart(2, '0')}`;
    // A small share stays unrecognised so the "Neznámý…" folders appear.
    const u = r();
    files.push({
      customer: u < 0.03 ? null : client,
      date: u > 0.985 ? null : date,
      inst: u > 0.04 && u < 0.06 ? null : inst,
      doc: u > 0.06 && u < 0.075 ? null : doc,
      name: `${FILE_STEM[doc] ?? 'dokument'}-${date}.pdf`,
    });
  }
  return files;
})();
