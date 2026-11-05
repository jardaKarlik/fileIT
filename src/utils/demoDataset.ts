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

// Document types per institution group, with weights (how often each
// appears in a typical advisor's archive for that institution).
type InstGroup = { names: string[]; docs: [string, number][] };

const INSTITUTIONS: InstGroup[] = [
  { names: ['Česká spořitelna', 'ČSOB', 'Komerční banka', 'Raiffeisenbank', 'MONETA Money Bank', 'Air Bank', 'Fio banka'],
    docs: [['Výpis z účtu', 30], ['Smlouva', 16], ['Úvěr a hypotéka', 14], ['Dopis z banky', 12], ['Souhlas', 10], ['Dodatek ke smlouvě', 8], ['Žádost', 6], ['Výpověď', 4]] },
  { names: ['Kooperativa', 'Allianz', 'Generali Česká pojišťovna', 'ČPP', 'UNIQA'],
    docs: [['Pojistná smlouva', 34], ['Dodatek ke smlouvě', 14], ['Souhlas', 12], ['Platba a upomínka', 10], ['Žádost', 8], ['Plná moc', 6], ['Výpověď', 6]] },
  { names: ['ČEZ', 'E.ON', 'Pražská plynárenská', 'innogy'],
    docs: [['Vyúčtování energií', 45], ['Smlouva', 20], ['Faktura', 25], ['Výpověď', 5]] },
  { names: ['O2', 'T-Mobile', 'Vodafone'],
    docs: [['Faktura', 60], ['Smlouva', 30], ['Výpověď', 10]] },
  { names: ['Finanční úřad'],
    docs: [['Daňové přiznání', 45], ['Rozhodnutí úřadu', 30], ['Platba a upomínka', 25]] },
  { names: ['ČSSZ', 'VZP', 'OZP'],
    docs: [['Rozhodnutí úřadu', 40], ['Žádost', 35], ['Platba a upomínka', 25]] },
  { names: ['Katastrální úřad'],
    docs: [['List vlastnictví (LV)', 75], ['Žádost', 25]] },
  { names: ['Úřad práce'],
    docs: [['Žádost', 60], ['Rozhodnutí úřadu', 40]] },
  { names: ['Notářství Mgr. Malá', 'Advokátní kancelář Fiala'],
    docs: [['Plná moc', 40], ['Právní dokument', 30], ['Smlouva', 30]] },
  { names: ['Magistrát města Brna', 'Městský úřad Kolín'],
    docs: [['Žádost', 45], ['Rozhodnutí úřadu', 35], ['Osobní doklady', 20]] },
];

// Group weights: banks and insurers dominate an advisor's archive; the land
// registry gets enough weight that LV folders show up clearly in the demo.
const GROUP_WEIGHTS = [30, 24, 10, 7, 7, 7, 6, 3, 3, 3];

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
    const doc = group.docs[pickWeighted(r, group.docs.map(d => d[1]))][0];
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
