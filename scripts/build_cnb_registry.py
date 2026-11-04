#!/usr/bin/env python3
"""CNB Mini-Database Builder — with per-file timeouts."""
import json
import sys
import xml.etree.ElementTree as ET
from pathlib import Path
from collections import defaultdict
import time
import signal

XML_DIR = Path(r"C:\_temp\xml")
OUTPUT_PATH = Path(__file__).parent.parent / "src-tauri" / "resources" / "cnb_registry.json"

# Timeout per file in seconds
FILE_TIMEOUT = 30

class TimeoutError(Exception):
    pass

def timeout_handler(signum, frame):
    raise TimeoutError("File processing timeout")

# Set up timeout handler (Unix only, Windows needs different approach)
if hasattr(signal, 'SIGALRM'):
    signal.signal(signal.SIGALRM, timeout_handler)

def log(msg):
    """Log message with Unicode-safe encoding."""
    # Replace problematic Unicode characters with ASCII equivalents
    safe_msg = msg.replace('✓', '[OK]').replace('✗', '[FAIL]')
    print(safe_msg, flush=True)

def normalize_name(raw):
    """Clean an institution name from the CNB XML.

    The CNB JERRS exports wrap G-Ident values in literal double quotes and
    use inconsistent spacing (e.g. `G-Ident>" CITY Most s.r.o. "</G-Ident>`).
    Strip surrounding quotes, collapse interior whitespace, and re-trim.
    """
    name = (raw or '').strip()
    if name.startswith('"') and name.endswith('"'):
        name = name[1:-1]
    name = ' '.join(name.split())
    return name

def is_header_row(ico, name):
    """Check if this is a header/metadata row, not actual institution data."""
    # Skip empty rows
    if not ico or not name:
        return True
    # Skip rows where ICO is not numeric
    if not ico.isdigit():
        return True
    # Skip rows where name looks like a column header
    if name.lower() in ['název subjektu', 'název', 'name', 'identifikace', 'ico']:
        return True
    # Skip rows that look like status messages
    if 'stav k' in name.lower() or 'celkem' in name.lower():
        return True
    return False

def process_file(xml_path, institutions_by_ico):
    """Process a single XML file. Returns (new_count, skipped_count, total_records)."""
    tree = ET.parse(xml_path)
    root = tree.getroot()
    
    new_count = 0
    skipped_count = 0
    total_records = 0
    
    for drec in root.findall('D-Rec'):
        ico_elem = drec.find('E-Icn')
        name_elem = drec.find('G-Ident')
        
        if ico_elem is None or name_elem is None:
            total_records += 1
            continue
            
        ico = ico_elem.text.strip() if ico_elem.text else ''
        name = normalize_name(name_elem.text if name_elem.text is not None else '')
        
        total_records += 1
        
        if is_header_row(ico, name):
            skipped_count += 1
            continue
        
        if ico not in institutions_by_ico:
            institutions_by_ico[ico] = name
            new_count += 1
    
    return new_count, skipped_count, total_records

def main():
    log(f"Scanning {XML_DIR} for XML files...")
    
    xml_files = sorted(XML_DIR.glob("*.xml"))
    log(f"Found {len(xml_files)} XML files")
    
    if not xml_files:
        log("ERROR: No XML files found!")
        return 1
    
    log(f"\n{'='*60}")
    log("Processing files individually (with timeouts)")
    log(f"{'='*60}\n")
    
    institutions_by_ico = {}
    stats = {
        'files_processed': 0,
        'files_timeout': 0,
        'files_error': 0,
        'total_records': 0,
        'total_skipped': 0,
    }
    
    for i, xml_path in enumerate(xml_files, 1):
        log(f"[{i}/{len(xml_files)}] {xml_path.name}")
        
        start_time = time.time()
        
        try:
            # Set timeout (Unix only)
            if hasattr(signal, 'SIGALRM'):
                signal.alarm(FILE_TIMEOUT)
            
            new_count, skipped_count, total_records = process_file(xml_path, institutions_by_ico)
            
            # Cancel timeout
            if hasattr(signal, 'SIGALRM'):
                signal.alarm(0)
            
            elapsed = time.time() - start_time
            stats['files_processed'] += 1
            stats['total_records'] += total_records
            stats['total_skipped'] += skipped_count
            
            log(f"  OK: {total_records} records, {skipped_count} skipped, +{new_count} new")
            log(f"  OK: Total: {len(institutions_by_ico)} institutions ({elapsed:.1f}s)")
            
        except TimeoutError:
            log(f"  FAIL: TIMEOUT (>{FILE_TIMEOUT}s)")
            stats['files_timeout'] += 1
            
        except Exception as e:
            log(f"  FAIL: ERROR: {e}")
            stats['files_error'] += 1
    
    # Write output
    log(f"\n{'='*60}")
    log("Writing output...")
    log(f"{'='*60}\n")
    
    institutions = [{'ico': ico, 'name': name} for ico, name in institutions_by_ico.items()]
    institutions.sort(key=lambda x: x['name'])
    
    output = {
        'version': '1.0.0',
        'generated': time.strftime('%Y-%m-%d %H:%M:%S'),
        'source': 'CNB JERRS Registry',
        'schema_version': 2,
        'institution_count': len(institutions),
        'stats': stats,
        'institutions': institutions,
    }
    
    OUTPUT_PATH.parent.mkdir(parents=True, exist_ok=True)
    with open(OUTPUT_PATH, 'w', encoding='utf-8') as f:
        json.dump(output, f, ensure_ascii=False, indent=2)
    
    log(f"OK: Wrote {len(institutions)} institutions to {OUTPUT_PATH}")
    log(f"\n{'='*60}")
    log("Statistics:")
    log(f"  Files processed: {stats['files_processed']}")
    log(f"  Files timeout: {stats['files_timeout']}")
    log(f"  Files error: {stats['files_error']}")
    log(f"  Total records: {stats['total_records']}")
    log(f"  Total skipped: {stats['total_skipped']}")
    log(f"  Unique institutions: {len(institutions)}")
    log(f"{'='*60}")
    
    return 0

if __name__ == "__main__":
    sys.exit(main())