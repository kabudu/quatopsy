#!/usr/bin/env python3
"""Released-CLI workflow budgets, not browser or target-processor measurements."""
import json
import resource
import subprocess
import sys
import tempfile
import time
from pathlib import Path

binary = Path(sys.argv[1] if len(sys.argv) > 1 else 'target/release/quatopsy').resolve()
with tempfile.TemporaryDirectory(prefix='quatopsy-workflow-budget-') as tmp:
    root = Path(tmp)
    for name, count, dense, optional in [('nominal', 1_000_000, False, False), ('dense', 50_000, True, False), ('optional', 100_000, False, True)]:
        case = root / name
        case.mkdir()
        csv = case / 'input.csv'
        manifest = {'schema': 'quatopsy.manifest/1', 'component_order': 'wxyz', 'rotation_sense': 'active', 'frame_from': 'BODY', 'frame_to': 'J2000', 'time_unit': 'ns', 'columns': {'time': 't', 'quaternion': ['qw', 'qx', 'qy', 'qz']}}
        if optional:
            manifest['columns']['angular_velocity'] = ['wx', 'wy', 'wz']
            manifest['columns']['rotation_matrix'] = [f'r{i}' for i in range(9)]
        with csv.open('w') as handle:
            handle.write('t,qw,qx,qy,qz' + (',wx,wy,wz,' + ','.join(f'r{i}' for i in range(9)) if optional else '') + '\n')
            for i in range(count):
                sign = -1 if dense and i % 50 == 1 else 1
                handle.write(f'{i * 1000000},{sign},0,0,0' + (',0,0,0,1,0,0,0,1,0,0,0,1' if optional else '') + '\n')
        mf = case / 'manifest.json'
        mf.write_text(json.dumps(manifest))
        report = case / 'report.json'
        stages = [
            ('analyze', ['analyze', '--input', str(csv), '--manifest', str(mf), '--report', str(report)]),
            ('view', ['view', '--input', str(csv), '--manifest', str(mf), '--report', str(report), '--output', str(case / 'viewer')]),
        ]
        # The incident boundary intentionally limits findings to 1024 per rule.
        if not dense:
            stages.extend([
                ('investigate', ['investigate', '--case-id', name, '--input', str(csv), '--manifest', str(mf), '--output-dir', str(case / 'bundle')]),
                ('verify-evidence', ['verify-evidence', '--bundle', str(case / 'bundle')]),
            ])
        for stage, args in stages:
            started = time.monotonic()
            result = subprocess.run([str(binary), *args], capture_output=True, text=True, timeout=60)
            elapsed = time.monotonic() - started
            expected = 1 if dense and stage == 'analyze' else 0
            if result.returncode != expected:
                raise SystemExit(f'{name}/{stage}: exit {result.returncode}, expected {expected}: {result.stderr}')
            rss = resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss
            rss_bytes = rss if sys.platform == 'darwin' else rss * 1024
            if rss_bytes > 512 * 1024 * 1024:
                raise SystemExit(f'{name}/{stage}: cumulative child peak RSS {rss_bytes} exceeds 512 MiB')
            print(f'{name}/{stage}: {elapsed:.3f}s, cumulative child peak RSS {rss_bytes / 1024**2:.1f} MiB', flush=True)
        size = sum(p.stat().st_size for p in (case / 'viewer').iterdir())
        if size > 32 * 1024 * 1024:
            raise SystemExit(f'{name}: viewer payload exceeds 32 MiB: {size}')
        print(f'{name}/viewer payload: {size / 1024**2:.2f} MiB', flush=True)
