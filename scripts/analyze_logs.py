#!/usr/bin/env python3
"""Analyze FileIT classification logs for success ratio."""
import re
import os

# Try multiple possible log locations
log_paths = [
    r"C:\Users\jaros\AppData\Local\cz.fileit.app\logs\fileit.log",
    r"C:\Users\jaros\AppData\Roaming\fileit\logs\fileit.log",
    r"C:\Users\jaros\AppData\Local\Temp\cline\proceed-while-running-*.log",
]

log_content = None
for path in log_paths:
    if '*' in path:
        import glob
        files = glob.glob(path)
        if files:
            path = files[0]
    if os.path.exists(path):
        print(f"Found log at: {path}")
        with open(path, encoding='utf-8') as f:
            log_content = f.read()
        break

if not log_content:
    print("ERROR: Could not find log file")
    exit(1)

# Count classifications
classified = re.findall(r'Classified "(.*?)": customer=(.*?), inst=(.*?), type=(.*?), conf=([\d.]+)', log_content)

print(f"\n=== Classification Analysis ===")
print(f"Total classified: {len(classified)}")

# Count with customer identified
with_customer = [c for c in classified if c[1] != 'None']
print(f"With customer: {len(with_customer)}")

# Count with institution identified
with_inst = [c for c in classified if c[2] != 'None']
print(f"With institution: {len(with_inst)}")

# Count with confidence > 0.5
high_conf = [c for c in classified if float(c[4]) > 0.5]
print(f"High confidence (>0.5): {len(high_conf)}")

# Count with confidence > 0.3
med_conf = [c for c in classified if float(c[4]) > 0.3]
print(f"Medium confidence (>0.3): {len(med_conf)}")

# Count with confidence > 0
any_conf = [c for c in classified if float(c[4]) > 0]
print(f"Any confidence (>0): {len(any_conf)}")

# Count with confidence == 0
zero_conf = [c for c in classified if float(c[4]) == 0]
print(f"Zero confidence: {len(zero_conf)}")

# Calculate success ratio
if len(classified) > 0:
    success_ratio = len(any_conf) / len(classified) * 100
    print(f"\nSuccess ratio (any conf > 0): {success_ratio:.1f}%")
    
    high_ratio = len(high_conf) / len(classified) * 100
    print(f"High confidence ratio (>0.5): {high_ratio:.1f}%")
    
    # Show sample of low-confidence classifications
    low_conf = [c for c in classified if float(c[4]) <= 0.3 and c[1] != 'None']
    print(f"\nSample low-confidence classifications:")
    for c in low_conf[:5]:
        print(f"  - {c[0]}: customer={c[1]}, type={c[3]}, conf={c[4]}")
