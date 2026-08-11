#!/usr/bin/env python3
"""Apply scan-manifest + CNB-registry changes to commands/mod.rs with exact indentation."""
import sys

P = r"C:\_dev\fileit\src-tauri\src\commands\mod.rs"
s = open(P, encoding="utf-8").read()

edits = []

# 1. Add import
old = "\nuse crate::metadata_reader;\nuse crate::scanner;\nuse crate::restructure;\n"
new = "\nuse crate::metadata_reader;\nuse crate::scan_manifest;\nuse crate::scanner;\nuse crate::restructure;\n"
assert old in s, "import anchor missing"
s = s.replace(old, new, 1)

# 2. Add cnb_registry field to AppState
old = "    pub cm_registry: RwLock<CmRegistry>,\n}"
new = "    pub cm_registry: RwLock<CmRegistry>,\n    pub cnb_registry: crate::cnb_registry::CnbRegistry,\n}"
assert old in s, "AppState anchor missing"
s = s.replace(old, new, 1)

# 3. Manifest load + diff after scan_directories
old = (
    "    let records = scanner::scan_directories(&app, scan_roots, &categories, &[])\n"
    "        .await\n"
    "        .map_err(|e| e.to_string())?;\n"
    "\n"
    "    let mut classified: Vec<ClassifiedFile> = Vec::new();\n"
)
new = (
    "    let records = scanner::scan_directories(&app, scan_roots, &categories, &[])\n"
    "        .await\n"
    "        .map_err(|e| e.to_string())?;\n"
    "\n"
    "    // Load previous scan manifest for incremental classification\n"
    "    let prev_manifest = scan_manifest::ScanManifest::load_latest(&state.app_data_dir);\n"
    "\n"
    "    // Build lookup map of unchanged files from previous scan\n"
    "    let mut unchanged_lookup: std::collections::HashMap<String, scan_manifest::ManifestEntry> =\n"
    "        std::collections::HashMap::new();\n"
    "\n"
    "    let mut reused_count = 0usize;\n"
    "    if let Some(ref manifest) = prev_manifest {\n"
    "        let diff = manifest.diff(&records);\n"
    "        reused_count = diff.unchanged_files.len();\n"
    "        for entry in diff.unchanged_files {\n"
    "            unchanged_lookup.insert(entry.path.clone(), entry);\n"
    "        }\n"
    "        info!(\n"
    "            \"Re-scan diff: {} unchanged, {} modified, {} new, {} deleted\",\n"
    "            reused_count,\n"
    "            diff.modified_files.len(),\n"
    "            diff.new_files.len(),\n"
    "            diff.deleted_files.len()\n"
    "        );\n"
    "    }\n"
    "\n"
    "    let mut classified: Vec<ClassifiedFile> = Vec::new();\n"
)
assert old in s, "scan anchor 3 missing"
s = s.replace(old, new, 1)

# 4. Reuse classification logic before Stage 1
old = (
    "        let _ = app.emit(\n"
    "            \"classify_progress\",\n"
    "            serde_json::json!({\n"
    "                \"done\": i,\n"
    "                \"total\": total,\n"
    "                \"current\": record.name\n"
    "            }),\n"
    "        );\n"
    "\n"
    "        // \u2500\u2500 Stage 1: Format-specific metadata fast-read + path analysis \u2500\u2500\u2500\u2500\u2500\n"
)
new = (
    "        let _ = app.emit(\n"
    "            \"classify_progress\",\n"
    "            serde_json::json!({\n"
    "                \"done\": i,\n"
    "                \"total\": total,\n"
    "                \"current\": record.name\n"
    "            }),\n"
    "        );\n"
    "\n"
    "        // Reuse classification from previous scan if file is unchanged\n"
    "        let path_key = record.path.to_string_lossy().to_string();\n"
    "        if let Some(prev_entry) = unchanged_lookup.get(&path_key) {\n"
    "            if let Some(ref clf_json) = prev_entry.classification {\n"
    "                if let Ok(prev_cf) = serde_json::from_str::<ClassifiedFile>(clf_json) {\n"
    "                    log::debug!(\"Reusing previous classification for: {}\", record.name);\n"
    "                    let sha = record.sha256.clone();\n"
    "                    if let Some(ref hash) = sha {\n"
    "                        if let Some(first_path) = seen_hashes.get(hash) {\n"
    "                            let mut prev_cf = prev_cf;\n"
    "                            prev_cf.is_duplicate = true;\n"
    "                            prev_cf.duplicate_of = Some(first_path.clone());\n"
    "                            classified.push(prev_cf);\n"
    "                        } else {\n"
    "                            seen_hashes.insert(hash.clone(), record.path.clone());\n"
    "                            classified.push(prev_cf);\n"
    "                        }\n"
    "                    } else {\n"
    "                        classified.push(prev_cf);\n"
    "                    }\n"
    "                    continue;\n"
    "                }\n"
    "            }\n"
    "        }\n"
    "\n"
    "        // \u2500\u2500 Stage 1: Format-specific metadata fast-read + path analysis \u2500\u2500\u2500\u2500\u2500\n"
)
assert old in s, "scan anchor 4 missing"
s = s.replace(old, new, 1)

open(P, "w", encoding="utf-8").write(s)
print("Part A applied successfully")
