#!/usr/bin/env python3
"""Apply lib.rs patch for CNB registry and scan_manifest imports."""
import sys

P = r"C:\_dev\fileit\src-tauri\src\lib.rs"
s = open(P, encoding="utf-8").read()

# 1. Add cnb_registry and scan_manifest modules
old_import = "mod cm_registry;\nmod commands;"
new_import = "mod cm_registry;\nmod cnb_registry;\nmod scan_manifest;\nmod commands;"

assert old_import in s, "Import anchor missing"
s = s.replace(old_import, new_import, 1)

# 2. Add CNB registry loading with the session initialization
scan_managers_start = """
            // Create equation reader and scanner
             let equation_reader = {
                 let equation_buffer = pb_create_eq_buffer();
                 let mut equation_reader = EquationReader::new(equation_buffer);
                 equation_reader.extend_by_strings(selected_patterns.text.clone(), equation_reader);
                 EquationDatabase::fill_map_table(FILTERED, PRESERVED_PATTERNS_PROPERTIES);
                 equation_reader
             };
             values.insert("equation_reader".to_string(), equation_reader);
             EquationReader::prefix_reset();"""

load_position = s.find("pub fn run() {")
if load_position == -1:
    print("run() function not found")
    exit(1)

# Find where session is managed
session_manage_start = """
            app.manage(AppState {\n"""
session_manage_end = "\n             Ok(())\n         });\n\n         Ok(())"

# Find the position to insert after registering different readers
insert_position_end = s.find(session_manage_start) + len(session_manage_start)

# Load CNB registry and add to AppState
cnb_registry_code = """

            // Load CNB registry (Stage 0 path detection)
            let cnb_reg =\n
                cnb_registry::CnbRegistry::new(&app.handle()).expect(\"Failed to load CNB registry\");\n            log::info!(\"CNB registry: {} institutions loaded\", cnb_reg.count());"""

# Insert the CNB registry loading code at the right position
after_registering = s.find("register_tokenizer();\ntrim_tokenizers();")
if after_registering != -1:
    insert_at = after_registering + len("register_tokenizer();\n")
    s = s[:insert_at] + cnb_registry_code + s[insert_at:]

# Insert both into session management
session_start_pos = s.find("app.manage(AppState {\n")
session_end_pos = s.find(session_manage_end)
if session_start_pos != -1 and session_end_pos != -1:
    # Insert our CNB registry loading code between AppState setup and Ok(())
    insert_point = s.find(session_manage_end) - 4  # Before "Ok(()))"
    insert_code = f"""{cnb_registry_code}}

{session_manage_end}"""
    s = s.replace(insert_point, len(insert_point), insert_code)

open(P, "w", encoding="utf-8").write(s)
print("lib.rs patch applied successfully")