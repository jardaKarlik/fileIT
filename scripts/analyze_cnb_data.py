import xml.etree.ElementTree as ET
from pathlib import Path
from collections import defaultdict, Counter

XML_DIR = Path(r"C:\_temp\xml")
files = sorted(XML_DIR.glob("*.xml"))

# Re-simulate the build script logic exactly
raw_rows = 0
missing_ico_name = 0
header_skipped = 0
by_ico = {}
name_by_ico = defaultdict(set)   # ico -> set of distinct names (to detect collisions)
counts_per_file = {}

def is_header_row(ico, name):
    if not ico or not name:
        return True
    if not ico.isdigit():
        return True
    if name.lower() in ['název subjektu','název','name','identifikace','ico']:
        return True
    if 'stav k' in name.lower() or 'celkem' in name.lower():
        return True
    return False

for fp in files:
    tree = ET.parse(fp)
    root = tree.getroot()
    fc = 0
    for drec in root.findall('D-Rec'):
        raw_rows += 1
        fc += 1
        ico_e = drec.find('E-Icn')
        name_e = drec.find('G-Ident')
        if ico_e is None or name_e is None:
            missing_ico_name += 1
            continue
        ico = ico_e.text.strip() if ico_e.text else ''
        name = name_e.text.strip() if name_e.text else ''
        if is_header_row(ico, name):
            header_skipped += 1
            continue
        if ico:
            by_ico[ico] = name
            name_by_ico[ico].add(name)
    counts_per_file[fp.name] = fc

top = Counter(counts_per_file).most_common(15)
print("FILES:", len(files))
print("RAW D-Rec rows (with missing & headers):", raw_rows)
print("Rows missing ICO or name elem:", missing_ico_name)
print("Rows skipped as header/empty:", header_skipped)
print("Candidate rows kept:", raw_rows - missing_ico_name - header_skipped)
print("UNIQUE ICOs (final registry size):", len(by_ico))
print("ICO with multiple distinct names (collisions):", sum(1 for s in name_by_ico.values() if len(s) > 1))
print("\nTOP 15 FILES BY RAW RECORDS:")
for n,c in top:
    print(f"  {c:6d}  {n}")
