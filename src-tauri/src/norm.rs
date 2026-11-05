// src-tauri/src/norm.rs
//
// Unicode normalisation for file names and paths.
//
// macOS (HFS+ always, APFS often — Finder, Safari downloads, zip tools)
// stores accented names in decomposed form (NFD): "č" is "c" + U+030C.
// Windows and nearly all document text use the composed form (NFC). Two
// strings that look identical then compare unequal, so name matching,
// institution / keyword search, scan-manifest keys and collision checks
// would silently miss Czech names.
//
// Rule: the real on-disk path (`FileRecord.path`) is never rewritten — it is
// what we open and move. Every *comparison* or *lookup key* goes through
// these helpers instead.

use std::path::Path;
use unicode_normalization::{is_nfc_quick, IsNormalized, UnicodeNormalization};

/// NFC form of `s`. Cheap when `s` is already NFC (the common case).
pub fn nfc(s: &str) -> String {
    match is_nfc_quick(s.chars()) {
        IsNormalized::Yes => s.to_string(),
        _ => s.nfc().collect(),
    }
}

/// NFC form of a path, as a string. Use for map keys and comparisons.
pub fn path_key(path: &Path) -> String {
    nfc(&path.to_string_lossy())
}

/// Key for "is this the same name on disk": NFC + lower-case. NTFS and the
/// default APFS / HFS+ volumes are case-insensitive, and APFS / HFS+ are
/// also normalisation-insensitive, so `Faktura.pdf`, `faktura.pdf` and an
/// NFD-spelled `faktura.pdf` all name the same file.
pub fn collision_key(path: &Path) -> String {
    path_key(path).to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // "Dvořák smlouva.pdf" spelled decomposed, as macOS stores it.
    const NFD_NAME: &str = "Dvor\u{030C}a\u{0301}k smlouva.pdf";
    const NFC_NAME: &str = "Dvořák smlouva.pdf";

    #[test]
    fn nfd_czech_name_becomes_nfc() {
        assert_ne!(NFD_NAME, NFC_NAME, "test data must really differ");
        assert_eq!(nfc(NFD_NAME), NFC_NAME);
        assert_eq!(nfc(NFC_NAME), NFC_NAME);
        // All Czech diacritics
        let nfd_all = "e\u{030C}s\u{030C}c\u{030C}r\u{030C}z\u{030C}y\u{0301}a\u{0301}i\u{0301}e\u{0301}u\u{030A}u\u{0301}d\u{030C}t\u{030C}n\u{030C}";
        assert_eq!(nfc(nfd_all), "ěščřžýáíéůúďťň");
    }

    #[test]
    fn path_keys_match_across_forms() {
        let a = PathBuf::from("/Users/jana/Klienti").join(NFD_NAME);
        let b = PathBuf::from("/Users/jana/Klienti").join(NFC_NAME);
        assert_eq!(path_key(&a), path_key(&b));
    }

    #[test]
    fn collision_key_ignores_case_and_form() {
        let a = PathBuf::from("/x/Faktura Řehoř.pdf");
        let b = PathBuf::from("/x/faktura R\u{030C}ehor\u{030C}.PDF");
        assert_eq!(collision_key(&a), collision_key(&b));
        assert_ne!(collision_key(&a), collision_key(Path::new("/x/faktura2.pdf")));
    }
}
