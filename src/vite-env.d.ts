/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** Dev only: scan this single folder instead of Documents/Desktop/Downloads. */
  readonly VITE_FILEIT_SCAN_ROOT?: string;
}
