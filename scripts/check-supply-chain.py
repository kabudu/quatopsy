#!/usr/bin/env python3
"""Allowlist licenses for locked Cargo metadata. Workspace crates must declare Apache-2.0."""
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

root = Path(__file__).resolve().parents[1]
allowed = {
    "Apache-2.0",
    "MIT",
    "MIT OR Apache-2.0",
    "Apache-2.0 OR MIT",
    "MIT/Apache-2.0",
    "Apache-2.0 OR BSL-1.0",
    "(MIT OR Apache-2.0) AND Unicode-3.0",
    "Unlicense OR MIT",
    "Unlicense/MIT",
    "Zlib OR Apache-2.0 OR MIT",
}
workspace = {"quatopsy-cli", "quatopsy-core", "quatopsy-oracle", "quatopsy-schema"}
proc = subprocess.run(
    ["cargo", "metadata", "--format-version", "1", "--locked", "--offline", "--manifest-path", str(root / "Cargo.toml")],
    check=True,
    capture_output=True,
    text=True,
)
meta = json.loads(proc.stdout)
resolved = {node["id"] for node in meta["resolve"]["nodes"]}
errors = []
for package in meta["packages"]:
    if package["id"] not in resolved:
        continue
    license_id = package.get("license") or "(none)"
    if package["name"] in workspace:
        if license_id != "Apache-2.0":
            errors.append(f"{package['name']}@{package['version']}: expected Apache-2.0, got {license_id}")
        continue
    if license_id not in allowed:
        errors.append(f"{package['name']}@{package['version']}: disallowed license {license_id}")
if errors:
    print("supply-chain license errors:", file=sys.stderr)
    for item in errors:
        print(f"  {item}", file=sys.stderr)
    sys.exit(1)
lock = root / "Cargo.lock"
digest_proc = subprocess.run(
    ["python3", "-c", "import hashlib,pathlib,sys; print(hashlib.sha256(pathlib.Path(sys.argv[1]).read_bytes()).hexdigest())", str(lock)],
    check=True,
    capture_output=True,
    text=True,
)
print(f"supply-chain: {len(resolved)} packages")
print(f"Cargo.lock sha256: {digest_proc.stdout.strip()}")
