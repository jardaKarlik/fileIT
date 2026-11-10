// src-tauri/src/classifier/formats.rs
//
// Readers for additional document formats:
//   • legacy Office (.doc, .xls) and Outlook .msg — OLE / compound files
//   • ISDOC e-invoices (.isdoc, .isdocx)
//   • Datová schránka messages (.zfo) — envelope + attachments
//   • XPS / OXPS documents
// Archives (.zip) and .zfo attachments are unpacked by the classifier (it
// needs the OCR context for PDFs and images inside); this module only
// returns their entries.

use anyhow::{Context, Result};
use base64::Engine;
use std::io::Read;
use std::path::Path;

/// Read at most this much of a stream / XML body.
const MAX_READ: usize = 4 * 1024 * 1024;

// ─────────────────────────────────────────────────────────────────────────────
// Legacy Office (.doc / .xls) — printable text runs from the main stream
// ─────────────────────────────────────────────────────────────────────────────

/// Text from a legacy Word (.doc) or Excel (.xls) file. The binary formats
/// store text as UTF-16 or 8-bit (windows-1250 for Czech) runs inside the
/// "WordDocument" / "Workbook" stream; collecting those runs is enough for
/// classification (names, IČO, keywords) without a full format parser.
pub fn extract_legacy_office(path: &Path) -> Result<String> {
    let mut cf = cfb::open(path).with_context(|| format!("Opening OLE file: {:?}", path))?;
    let candidates = ["/WordDocument", "/Workbook", "/Book", "/PowerPoint Document"];
    for stream in candidates {
        if cf.is_stream(stream) {
            let mut buf = Vec::new();
            cf.open_stream(stream)?.take(MAX_READ as u64).read_to_end(&mut buf)?;
            return Ok(binary_text_runs(&buf));
        }
    }
    Ok(String::new())
}

/// Outlook .msg: subject, sender and body from their MAPI property streams
/// (`…001F` = UTF-16, `…001E` = 8-bit).
pub fn extract_msg(path: &Path) -> Result<String> {
    let mut cf = cfb::open(path).with_context(|| format!("Opening .msg: {:?}", path))?;
    let mut prop = |id: &str| -> Option<String> {
        let utf16 = format!("/__substg1.0_{}001F", id);
        let ansi = format!("/__substg1.0_{}001E", id);
        if cf.is_stream(&utf16) {
            let mut b = Vec::new();
            cf.open_stream(&utf16).ok()?.take(MAX_READ as u64).read_to_end(&mut b).ok()?;
            let units: Vec<u16> = b.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
            return Some(String::from_utf16_lossy(&units).trim_end_matches('\0').to_string());
        }
        if cf.is_stream(&ansi) {
            let mut b = Vec::new();
            cf.open_stream(&ansi).ok()?.take(MAX_READ as u64).read_to_end(&mut b).ok()?;
            return Some(super::extractor::decode_text(&b).trim_end_matches('\0').to_string());
        }
        None
    };
    let subject = prop("0037").unwrap_or_default();
    let sender = prop("0C1A").or_else(|| prop("0042")).unwrap_or_default();
    let sender_mail = prop("0C1F").or_else(|| prop("5D01")).unwrap_or_default();
    let body = prop("1000").unwrap_or_default();
    Ok(format!("Předmět: {}\nOd: {} {}\n\n{}", subject, sender, sender_mail, body))
}

/// Printable UTF-16LE and 8-bit (windows-1250) runs of at least a few
/// characters, in file order.
fn binary_text_runs(bytes: &[u8]) -> String {
    let mut out = String::new();

    // UTF-16LE runs
    let mut run: Vec<u16> = Vec::new();
    let flush16 = |run: &mut Vec<u16>, out: &mut String| {
        if run.len() >= 4 {
            out.push_str(&String::from_utf16_lossy(run));
            out.push(' ');
        }
        run.clear();
    };
    for pair in bytes.chunks_exact(2) {
        let u = u16::from_le_bytes([pair[0], pair[1]]);
        let printable = (0x20..0x7F).contains(&u) || (0xA0..=0x17F).contains(&u) || u == 0x2013 || u == 0x201E || u == 0x201C;
        if printable { run.push(u) } else { flush16(&mut run, &mut out) }
    }
    flush16(&mut run, &mut out);

    // 8-bit runs (windows-1250)
    let mut run8: Vec<u8> = Vec::new();
    let mut ascii_runs = String::new();
    for &b in bytes {
        if (0x20..0x7F).contains(&b) || b >= 0xA0 || b == 0x8A || b == 0x8E || b == 0x9A || b == 0x9E {
            run8.push(b);
        } else {
            if run8.len() >= 5 {
                ascii_runs.push_str(&encoding_rs::WINDOWS_1250.decode_without_bom_handling(&run8).0);
                ascii_runs.push(' ');
            }
            run8.clear();
        }
    }
    if run8.len() >= 5 {
        ascii_runs.push_str(&encoding_rs::WINDOWS_1250.decode_without_bom_handling(&run8).0);
    }
    // 8-bit text is the common case for Czech .doc; keep whichever found more.
    if ascii_runs.len() > out.len() { ascii_runs } else { out }
}

// ─────────────────────────────────────────────────────────────────────────────
// XML helpers (namespace-agnostic, no full parser needed)
// ─────────────────────────────────────────────────────────────────────────────

/// Inner XML of the first `<tag …>…</tag>` (any namespace prefix).
fn element<'a>(xml: &'a str, tag: &str) -> Option<&'a str> {
    let mut search = 0;
    while let Some(rel) = xml[search..].find('<') {
        let start = search + rel;
        let rest = &xml[start + 1..];
        let name_end = rest.find(|c: char| c == '>' || c == ' ' || c == '/' || c == '\n' || c == '\r' || c == '\t')?;
        let name = &rest[..name_end];
        let local = name.rsplit(':').next().unwrap_or(name);
        if local == tag && !name.starts_with('/') {
            let open_end = start + 1 + rest.find('>')?;
            if xml[..open_end].ends_with('/') {
                return Some("");
            }
            let close = format!("</{}>", name);
            let body_start = open_end + 1;
            let end = xml[body_start..].find(&close)? + body_start;
            return Some(&xml[body_start..end]);
        }
        search = start + 1;
    }
    None
}

/// Text of the first `<tag>` (tags inside stripped, entities decoded).
fn text_of(xml: &str, tag: &str) -> Option<String> {
    element(xml, tag).map(|inner| xml_unescape(strip_tags(inner).trim())).filter(|s| !s.is_empty())
}

/// Value of `attr="…"` in the opening tag of the first `<tag …>`.
fn attr_of(xml: &str, tag: &str, attr: &str) -> Option<String> {
    let mut search = 0;
    while let Some(rel) = xml[search..].find('<') {
        let start = search + rel;
        let open_end = start + xml[start..].find('>')?;
        let open = &xml[start + 1..open_end];
        let name = open.split_whitespace().next().unwrap_or("");
        if name.rsplit(':').next() == Some(tag) {
            let key = format!("{}=\"", attr);
            let i = open.find(&key)? + key.len();
            let j = open[i..].find('"')? + i;
            return Some(xml_unescape(&open[i..j]));
        }
        search = open_end;
    }
    None
}

fn strip_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => { in_tag = false; out.push(' '); }
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn xml_unescape(s: &str) -> String {
    let mut out = s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'");
    // Numeric entities (&#382; / &#x17E;)
    while let Some(i) = out.find("&#") {
        let Some(j) = out[i..].find(';').map(|j| i + j) else { break };
        let code = &out[i + 2..j];
        let ch = if let Some(hex) = code.strip_prefix('x').or_else(|| code.strip_prefix('X')) {
            u32::from_str_radix(hex, 16).ok()
        } else {
            code.parse().ok()
        }
        .and_then(char::from_u32);
        match ch {
            Some(c) => out.replace_range(i..=j, &c.to_string()),
            None => break,
        }
    }
    out.replace("&amp;", "&")
}

/// "2024-03-15…" → "15.03.2024" (the Czech date form the classifier reads).
fn czech_date(iso: &str) -> Option<String> {
    let d = iso.get(..10)?;
    let p: Vec<&str> = d.split('-').collect();
    (p.len() == 3).then(|| format!("{}.{}.{}", p[2], p[1], p[0]))
}

// ─────────────────────────────────────────────────────────────────────────────
// ISDOC e-invoice
// ─────────────────────────────────────────────────────────────────────────────

/// ISDOC (.isdoc XML or .isdocx ZIP): the structured fields, written out as
/// Czech invoice text so the normal classifier recognises type, supplier,
/// IČO/DIČ, date, amount and variable symbol.
pub fn extract_isdoc(path: &Path) -> Result<String> {
    let is_zip = path.extension().map(|e| e.eq_ignore_ascii_case("isdocx")).unwrap_or(false);
    let xml = if is_zip {
        let mut zip = zip::ZipArchive::new(std::fs::File::open(path)?)?;
        let name = (0..zip.len())
            .filter_map(|i| zip.by_index(i).ok().map(|f| f.name().to_string()))
            .find(|n| n.to_lowercase().ends_with(".isdoc"))
            .context("No .isdoc inside .isdocx")?;
        let mut s = String::new();
        zip.by_name(&name)?.take(MAX_READ as u64).read_to_string(&mut s)?;
        s
    } else {
        let mut b = Vec::new();
        std::fs::File::open(path)?.take(MAX_READ as u64).read_to_end(&mut b)?;
        super::extractor::decode_text(&b)
    };
    Ok(isdoc_to_text(&xml))
}

fn isdoc_to_text(xml: &str) -> String {
    let party = |section: &str| -> (String, String, String) {
        let Some(sec) = element(xml, section) else { return Default::default() };
        let name = text_of(sec, "Name").unwrap_or_default();
        let ico = element(sec, "PartyIdentification").and_then(|p| text_of(p, "ID")).unwrap_or_default();
        let dic = element(sec, "PartyTaxScheme").and_then(|p| text_of(p, "CompanyID")).unwrap_or_default();
        (name, ico, dic)
    };
    let (s_name, s_ico, s_dic) = party("AccountingSupplierParty");
    let (c_name, c_ico, c_dic) = party("AccountingCustomerParty");
    // The invoice's own ID is the first <ID> directly in the document
    // header, before the parties.
    let header_end = xml.find("AccountingSupplierParty").unwrap_or(xml.len());
    let number = text_of(&xml[..header_end], "ID").unwrap_or_default();
    let issue = text_of(xml, "IssueDate").and_then(|d| czech_date(&d)).unwrap_or_default();
    let due = text_of(xml, "PaymentDueDate").and_then(|d| czech_date(&d)).unwrap_or_default();
    let vs = text_of(xml, "VariableSymbol").unwrap_or_default();
    let iban = text_of(xml, "IBAN").unwrap_or_default();
    let total = element(xml, "LegalMonetaryTotal")
        .and_then(|t| text_of(t, "PayableAmount").or_else(|| text_of(t, "TaxInclusiveAmount")))
        .unwrap_or_default()
        .replace('.', ",");
    let note = text_of(xml, "Note").unwrap_or_default();
    format!(
        "FAKTURA – daňový doklad (ISDOC) č. {number}\n\
         Dodavatel: {s_name}, IČO: {s_ico}, DIČ: {s_dic}\n\
         Odběratel: {c_name}, IČO: {c_ico}, DIČ: {c_dic}\n\
         Datum vystavení: {issue}\nDatum splatnosti: {due}\n\
         Variabilní symbol: {vs}\nIBAN: {iban}\n\
         Celkem k úhradě: {total} Kč\n{note}\n"
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Datová schránka (.zfo)
// ─────────────────────────────────────────────────────────────────────────────

/// A file packed inside a .zfo message or a .zip archive.
pub struct InnerFile {
    pub name: String,
    pub bytes: Vec<u8>,
}

/// .zfo = signed (PKCS#7) XML data message. The XML sits unencoded inside
/// the signature container, so it is located by its root tag. Returns the
/// envelope as Czech text (sender, recipient, subject, delivery date,
/// attachment names) and the decoded attachments.
pub fn extract_zfo(path: &Path) -> Result<(String, Vec<InnerFile>)> {
    let mut raw = Vec::new();
    std::fs::File::open(path)?.take(64 * 1024 * 1024).read_to_end(&mut raw)?;
    let text = String::from_utf8_lossy(&raw);
    let start = ["dmReturnedMessage", "MessageDownloadResponse", "dmDelivery", "dmMessage"]
        .iter()
        .filter_map(|root| text.find(root).and_then(|i| text[..i].rfind('<')))
        .min()
        .context("No data-message XML found in .zfo")?;
    let xml = &text[start..];

    let sender = text_of(xml, "dmSender").unwrap_or_default();
    let sender_addr = text_of(xml, "dmSenderAddress").unwrap_or_default();
    let recipient = text_of(xml, "dmRecipient").unwrap_or_default();
    let subject = text_of(xml, "dmAnnotation").unwrap_or_default();
    let reference = text_of(xml, "dmSenderRefNumber").or_else(|| text_of(xml, "dmSenderIdent")).unwrap_or_default();
    let delivered = text_of(xml, "dmDeliveryTime")
        .or_else(|| text_of(xml, "dmAcceptanceTime"))
        .and_then(|d| czech_date(&d))
        .unwrap_or_default();

    let mut files = Vec::new();
    let mut names = Vec::new();
    let mut rest = xml;
    while let Some(i) = rest.find("dmFile ") {
        // Start at the '<' of this element, whatever its namespace prefix.
        let tag_start = rest[..i].rfind('<').unwrap_or(i);
        let chunk = &rest[tag_start..];
        let name = attr_of(chunk, "dmFile", "dmFileDescr").unwrap_or_else(|| format!("priloha{}", files.len() + 1));
        let skip = (i - tag_start) + 7;
        let next = chunk[skip..].find("dmFile ").map(|j| j + skip).unwrap_or(chunk.len());
        let body = &chunk[..next];
        if let Some(b64) = element(body, "dmEncodedContent") {
            let clean: String = b64.chars().filter(|c| !c.is_whitespace()).collect();
            if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(clean.as_bytes()) {
                files.push(InnerFile { name: name.clone(), bytes });
            }
        }
        names.push(name);
        rest = &chunk[next.min(chunk.len())..];
        if next >= chunk.len() {
            break;
        }
    }

    let envelope = format!(
        "Datová zpráva (datová schránka)\nOdesílatel: {sender}, {sender_addr}\nAdresát: {recipient}\n\
         Věc: {subject}\nČ.j.: {reference}\nDodáno: {delivered}\nPřílohy: {}\n",
        names.join(", ")
    );
    Ok((envelope, files))
}

// ─────────────────────────────────────────────────────────────────────────────
// XPS / OXPS
// ─────────────────────────────────────────────────────────────────────────────

/// XPS / OXPS: a ZIP of FixedPage XML; text is in `UnicodeString` attributes
/// of <Glyphs>. Reads the first pages.
pub fn extract_xps(path: &Path) -> Result<String> {
    let mut zip = zip::ZipArchive::new(std::fs::File::open(path)?).context("Parsing XPS as ZIP")?;
    let mut pages: Vec<String> = (0..zip.len())
        .filter_map(|i| zip.by_index(i).ok().map(|f| f.name().to_string()))
        .filter(|n| n.to_lowercase().ends_with(".fpage"))
        .collect();
    pages.sort_by_key(|n| {
        n.rsplit('/').next().and_then(|f| f.split('.').next()).and_then(|d| d.parse::<u32>().ok()).unwrap_or(u32::MAX)
    });
    let mut out = String::new();
    for name in pages.into_iter().take(5) {
        let mut xml = String::new();
        zip.by_name(&name)?.take(MAX_READ as u64).read_to_string(&mut xml)?;
        let mut rest = xml.as_str();
        while let Some(i) = rest.find("UnicodeString=\"") {
            let s = &rest[i + 15..];
            let Some(j) = s.find('"') else { break };
            out.push_str(&xml_unescape(&s[..j]));
            out.push(' ');
            rest = &s[j..];
        }
        out.push('\n');
    }
    Ok(out)
}

// ─────────────────────────────────────────────────────────────────────────────
// ZIP archives
// ─────────────────────────────────────────────────────────────────────────────

/// Limits that keep archive scanning fast and safe (zip bombs).
pub const ZIP_MAX_ENTRIES: usize = 25;
pub const ZIP_MAX_ENTRY_BYTES: u64 = 25 * 1024 * 1024;
pub const ZIP_MAX_TOTAL_BYTES: u64 = 100 * 1024 * 1024;

/// Document entries of a ZIP (supported types only, nested archives and
/// encrypted entries skipped), within the limits above.
pub fn zip_document_entries(path: &Path) -> Result<Vec<InnerFile>> {
    let mut zip = zip::ZipArchive::new(std::fs::File::open(path)?).context("Parsing ZIP")?;
    let mut out = Vec::new();
    let mut total = 0u64;
    for i in 0..zip.len() {
        if out.len() >= ZIP_MAX_ENTRIES || total >= ZIP_MAX_TOTAL_BYTES {
            break;
        }
        let Ok(mut entry) = zip.by_index(i) else { continue };
        if entry.is_dir() || entry.encrypted() {
            continue;
        }
        let name = entry.name().replace('\\', "/");
        let file_name = name.rsplit('/').next().unwrap_or(&name).to_string();
        if file_name.starts_with("~$") || name.contains("__MACOSX/") {
            continue;
        }
        let ext = file_name.rsplit('.').next().unwrap_or("").to_lowercase();
        let cat = crate::types::FileCategory::from_extension(&ext);
        if matches!(cat, crate::types::FileCategory::Other | crate::types::FileCategory::Archive) {
            continue;
        }
        let size = entry.size();
        if size > ZIP_MAX_ENTRY_BYTES || total + size > ZIP_MAX_TOTAL_BYTES {
            continue;
        }
        let mut bytes = Vec::with_capacity(size as usize);
        // `take` guards against entries that lie about their size.
        if (&mut entry).take(ZIP_MAX_ENTRY_BYTES).read_to_end(&mut bytes).is_err() {
            continue;
        }
        total += bytes.len() as u64;
        out.push(InnerFile { name: file_name, bytes });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isdoc_fields_become_invoice_text() {
        let xml = r#"<?xml version="1.0"?><Invoice xmlns="http://isdoc.cz/namespace/2013">
          <DocumentType>1</DocumentType><ID>FV2024001</ID><IssueDate>2024-03-15</IssueDate>
          <AccountingSupplierParty><Party><PartyIdentification><ID>45274649</ID></PartyIdentification>
            <PartyName><Name>ČEZ Prodej, a.s.</Name></PartyName>
            <PartyTaxScheme><CompanyID>CZ45274649</CompanyID></PartyTaxScheme></Party></AccountingSupplierParty>
          <AccountingCustomerParty><Party><PartyIdentification><ID>27082440</ID></PartyIdentification>
            <PartyName><Name>Firma s.r.o.</Name></PartyName></Party></AccountingCustomerParty>
          <LegalMonetaryTotal><PayableAmount>12500.00</PayableAmount></LegalMonetaryTotal>
          <PaymentMeans><Payment><Details><PaymentDueDate>2024-03-29</PaymentDueDate>
            <VariableSymbol>2024001</VariableSymbol><IBAN>CZ6508000000192000145399</IBAN></Details></Payment></PaymentMeans>
        </Invoice>"#;
        let t = isdoc_to_text(xml);
        assert!(t.contains("č. FV2024001"), "{}", t);
        assert!(t.contains("Dodavatel: ČEZ Prodej, a.s., IČO: 45274649, DIČ: CZ45274649"), "{}", t);
        assert!(t.contains("Odběratel: Firma s.r.o., IČO: 27082440"), "{}", t);
        assert!(t.contains("Datum vystavení: 15.03.2024"), "{}", t);
        assert!(t.contains("Variabilní symbol: 2024001"), "{}", t);
        assert!(t.contains("Celkem k úhradě: 12500,00 Kč"), "{}", t);
    }

    #[test]
    fn zfo_envelope_and_attachment() {
        let pdf = base64::engine::general_purpose::STANDARD.encode(b"%PDF-1.4 test");
        let xml = format!(
            "\u{30}\u{82}garbage<?xml version=\"1.0\"?><q:MessageDownloadResponse xmlns:q=\"x\"><q:dmReturnedMessage>\
             <p:dmDm xmlns:p=\"y\"><p:dmSender>Finanční úřad pro Jihomoravský kraj</p:dmSender>\
             <p:dmSenderAddress>Brno</p:dmSenderAddress><p:dmRecipient>Jan Novák</p:dmRecipient>\
             <p:dmAnnotation>Platební výměr na daň z nemovitých věcí</p:dmAnnotation>\
             <p:dmFiles><p:dmFile dmMimeType=\"application/pdf\" dmFileMetaType=\"main\" dmFileDescr=\"vymer.pdf\">\
             <p:dmEncodedContent>{pdf}</p:dmEncodedContent></p:dmFile></p:dmFiles></p:dmDm>\
             <q:dmDeliveryTime>2024-05-02T10:11:12</q:dmDeliveryTime></q:dmReturnedMessage></q:MessageDownloadResponse>sig"
        );
        let dir = std::env::temp_dir().join(format!("fileit-zfo-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("zprava.zfo");
        std::fs::write(&p, xml.as_bytes()).unwrap();
        let (env, files) = extract_zfo(&p).unwrap();
        assert!(env.contains("Odesílatel: Finanční úřad pro Jihomoravský kraj"), "{}", env);
        assert!(env.contains("Věc: Platební výměr"), "{}", env);
        assert!(env.contains("Dodáno: 02.05.2024"), "{}", env);
        assert!(env.contains("Přílohy: vymer.pdf"), "{}", env);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].bytes, b"%PDF-1.4 test");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn xps_glyph_text() {
        use std::io::Write;
        let dir = std::env::temp_dir().join(format!("fileit-xps-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("doc.xps");
        let mut z = zip::ZipWriter::new(std::fs::File::create(&p).unwrap());
        let o: zip::write::SimpleFileOptions = Default::default();
        z.start_file("Documents/1/Pages/1.fpage", o).unwrap();
        z.write_all(r#"<FixedPage><Glyphs UnicodeString="Smlouva o úvěru" /><Glyphs UnicodeString="Dvo&#345;ák &amp; syn" /></FixedPage>"#.as_bytes()).unwrap();
        z.finish().unwrap();
        let t = extract_xps(&p).unwrap();
        assert!(t.contains("Smlouva o úvěru") && t.contains("Dvořák & syn"), "{}", t);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn binary_runs_find_czech_text() {
        let mut bytes = vec![0u8, 1, 2, 3];
        bytes.extend(encoding_rs::WINDOWS_1250.encode("Smlouva pro Jana Dvořáka").0.iter());
        bytes.extend([0u8, 0, 7, 8]);
        assert!(binary_text_runs(&bytes).contains("Smlouva pro Jana Dvořáka"));
    }

    #[test]
    fn zip_entries_respect_limits() {
        use std::io::Write;
        let dir = std::env::temp_dir().join(format!("fileit-zip-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("a.zip");
        let mut z = zip::ZipWriter::new(std::fs::File::create(&p).unwrap());
        let o: zip::write::SimpleFileOptions = Default::default();
        for (n, c) in [("smlouva.txt", "Smlouva"), ("app.exe", "MZ"), ("inner.zip", "PK"), ("__MACOSX/x.txt", "x")] {
            z.start_file(n, o).unwrap();
            z.write_all(c.as_bytes()).unwrap();
        }
        z.finish().unwrap();
        let e = zip_document_entries(&p).unwrap();
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].name, "smlouva.txt");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
