// src/utils/folderTree.ts
// Shared folder-structure logic for the Restructure screen and the
// "Jak to funguje" demo builder. Mirrors ClassifiedFile::group_key in Rust:
// the same four dimensions and the same folder names.

import type { ClassifiedFile, GroupDimension } from '../types';

export const DIMENSIONS: Record<GroupDimension, { name: string; icon: string; iconClass: string; example: string }> = {
  customer: { name: 'Jméno klienta', icon: '👤', iconClass: 'i-customer', example: 'Novák Pavel' },
  date:     { name: 'Měsíc a rok',   icon: '📅', iconClass: 'i-date',     example: '2025-03' },
  inst:     { name: 'Instituce',     icon: '🏦', iconClass: 'i-inst',     example: 'Česká spořitelna' },
  doc:      { name: 'Typ dokumentu', icon: '📄', iconClass: 'i-doc',      example: 'Smlouva' },
};

/** Folder used when a file has no value for the dimension (same as Rust). */
export const UNKNOWN: Record<GroupDimension, string> = {
  customer: 'Neznámý klient', date: 'Neurčeno', inst: 'Bez instituce', doc: 'Neurčený typ',
};

/** Czech folder names for document types (Rust DocumentType::label_cs). */
export const DOC_TYPE_LABELS: Record<string, string> = {
  contract: 'Smlouva',
  bank_letter: 'Dopis z banky',
  invoice: 'Faktura',
  payment_demand: 'Platba a upomínka',
  tax_return: 'Daňové přiznání',
  utility_bill: 'Vyúčtování energií',
  insurance_contract: 'Pojistná smlouva',
  bank_statement: 'Výpis z účtu',
  admin_decision: 'Rozhodnutí úřadu',
  identity: 'Osobní doklady',
  legal_letter: 'Právní dokument',
  property_record: 'List vlastnictví (LV)',
  application: 'Žádost',
  consent: 'Souhlas',
  power_of_attorney: 'Plná moc',
  termination: 'Výpověď',
  amendment: 'Dodatek ke smlouvě',
  loan_contract: 'Úvěr a hypotéka',
};

/** One file reduced to the four folder dimensions (null = unknown). */
export interface FileObj {
  customer: string | null;
  /** "YYYY-MM" */
  date: string | null;
  inst: string | null;
  doc: string | null;
  /** Optional example file name (demo data). */
  name?: string;
}

export type TreeNode = { count: number; children: Record<string, TreeNode> | null; isUnknown: boolean };

/** Group files into a nested folder tree, largest folders first, unknown last. */
export function groupBy(files: FileObj[], dims: GroupDimension[]): Record<string, TreeNode> {
  if (!dims.length) return {};
  const [dim, ...rest] = dims;
  const groups: Record<string, FileObj[]> = {};
  files.forEach(f => {
    const v = f[dim] ?? UNKNOWN[dim];
    (groups[v] ??= []).push(f);
  });
  const result: Record<string, TreeNode> = {};
  Object.keys(groups)
    .sort((a, b) => a === UNKNOWN[dim] ? 1 : b === UNKNOWN[dim] ? -1 : groups[b].length - groups[a].length)
    .forEach(k => {
      result[k] = {
        count: groups[k].length,
        children: rest.length ? groupBy(groups[k], rest) : null,
        isUnknown: k === UNKNOWN[dim],
      };
    });
  return result;
}

/** Convert scan results to FileObj, with Czech document-type folder names. */
export function fromClassifiedFiles(files: ClassifiedFile[]): FileObj[] {
  return files.map(f => {
    const docKey = (f.document_type as string | undefined)?.toLowerCase();
    return {
      customer: f.customer?.name ?? null,
      date: f.document_date ? new Date(f.document_date).toISOString().slice(0, 7) : null,
      inst: f.institution?.canonical_name ?? null,
      doc: docKey && docKey !== 'unknown' ? (DOC_TYPE_LABELS[docKey] ?? docKey) : null,
      name: f.record?.name,
    };
  });
}

/** Number of folders at each depth of a tree. */
export function countFolders(tree: Record<string, TreeNode>): { top: number; sub: number } {
  let sub = 0;
  const walk = (t: Record<string, TreeNode>) => {
    for (const n of Object.values(t)) {
      if (n.children) {
        sub += Object.keys(n.children).length;
        walk(n.children);
      }
    }
  };
  walk(tree);
  return { top: Object.keys(tree).length, sub };
}
