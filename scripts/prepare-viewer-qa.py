#!/usr/bin/env python3
"""Generate offline manual-QA artifacts without opening a browser."""
import json
import subprocess
from pathlib import Path

root = Path(__file__).resolve().parents[1]
binary = root / 'target/release/quatopsy'
output = root / 'target/viewer-qa'
output.mkdir(parents=True, exist_ok=True)
for name in ['sign_alternating', 'norm_drift', 'clean_slew', 'time_decreasing', 'near_pi', 'exact_epoch', 'dense_queue', 'irregular_time']:
    case = output / name.replace("_", "-")
    case.mkdir(exist_ok=True)
    source = root / 'fixtures/conformance' / name
    if name == 'irregular_time':
        source = root / 'fixtures/viewer/irregular_time'
    if name in ['exact_epoch', 'dense_queue']:
        source = case
        manifest = json.loads((root / 'fixtures/conformance/sign_alternating/manifest.json').read_text())
        manifest['time_unit'] = 'ns'
        (case / 'manifest.json').write_text(json.dumps(manifest))
        count = 2205 if name == 'dense_queue' else 4
        (case / 'input.csv').write_text('t,qw,qx,qy,qz\n' + ''.join(f'{1700000000000000001+i},{-1 if i%2 else 1},0,0,0\n' for i in range(count)))
    common = ['--input', str(source / 'input.csv'), '--manifest', str(source / 'manifest.json'), '--report', str(case / 'report.json')]
    result = subprocess.run([str(binary), 'analyze', *common, '--overwrite'], capture_output=True, text=True)
    if result.returncode not in [0, 1, 2]:
        raise SystemExit(result.stderr)
    subprocess.run([str(binary), 'view', *common, '--output', str(case / 'viewer'), '--overwrite'], check=True, capture_output=True)
    print(case / 'viewer/index.html')
print('Manual QA instructions:', root / 'docs/INVESTIGATION_FIDELITY.md')
