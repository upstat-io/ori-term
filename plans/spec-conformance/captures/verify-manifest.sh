#!/usr/bin/env bash
#
# Spec Conformance — Capture Manifest Verifier
#
# Walks every `[[capture]]` entry in `captures/manifest.toml`,
# recomputes the sha256 of its `.cap` transcript, counts the
# unique `(category, intermediates, final_byte)` tuples the
# transcript produces, and asserts:
#
#   1. The `.cap` file exists.
#   2. The recomputed sha256 matches `sha256 = "..."`. If the
#      manifest says `sha256 = "PENDING"`, the entry is flagged as
#      pending (not failing) — pending entries are allowed until
#      Section 01.5 lands the actual capture bytes.
#   3. The unique tuple count is ≥ `unique_tuples_expected_min`.
#   4. `unique_tuples_expected_min` is ≥ the global
#      `idle_reject_threshold` (hard gate — a manifest that sets
#      the per-capture minimum BELOW the idle threshold would let
#      broken scripts land silently).
#
# Exit codes:
#   0  — all captures verified clean
#   1  — at least one capture mismatched / idle / missing
#   2  — manifest parse error
#
# Dependencies:
#   - `python3` with `tomllib` (Python 3.11+) — used for the TOML
#     parse. The repo already requires Python 3 for the capture
#     runner, so this is not a new dep.
#   - `sha256sum` — GNU coreutils.
#   - `cargo` + `oriterm_test_support` — the tuple count shells
#     out to `catalog_coverage_check extract-capture-tuples` which
#     is Section 01.3's SSOT for tuple extraction.

set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname "${BASH_SOURCE[0]}")" &> /dev/null && pwd)"
REPO_ROOT="$(cd -- "$SCRIPT_DIR/../../.." && pwd)"
MANIFEST="$SCRIPT_DIR/manifest.toml"

if [[ ! -f "$MANIFEST" ]]; then
    echo "verify-manifest: $MANIFEST not found" >&2
    exit 2
fi

python3 - "$MANIFEST" "$SCRIPT_DIR" "$REPO_ROOT" <<'PY'
import hashlib
import subprocess
import sys
import tomllib
from pathlib import Path

manifest_path = Path(sys.argv[1])
captures_dir = Path(sys.argv[2])
repo_root = Path(sys.argv[3])

with manifest_path.open("rb") as f:
    manifest = tomllib.load(f)

idle_threshold = int(manifest.get("idle_reject_threshold", 20))
entries = manifest.get("capture", [])

failures: list[str] = []
pending: list[str] = []
clean: list[str] = []

for entry in entries:
    app = entry.get("app", "<unknown>")
    script_rel = entry.get("script", "")
    transcript_rel = entry.get("transcript", "")
    expected_sha = entry.get("sha256", "")
    expected_min = int(entry.get("unique_tuples_expected_min", 0))

    transcript_path = (repo_root / "plans/spec-conformance" / transcript_rel).resolve()

    # Gate 4 — per-entry minimum must respect the idle threshold.
    if expected_min < idle_threshold:
        failures.append(
            f"{app}: unique_tuples_expected_min={expected_min} is "
            f"below idle_reject_threshold={idle_threshold} "
            f"(entry script={script_rel})"
        )
        continue

    # Gate 1 — transcript exists?
    if not transcript_path.exists():
        if expected_sha == "PENDING":
            pending.append(
                f"{app}: transcript {transcript_rel} not present yet "
                f"(manifest sha256 = PENDING — run 01.5 to produce it)"
            )
            continue
        failures.append(f"{app}: transcript missing: {transcript_path}")
        continue

    # Gate 2 — sha256 match.
    digest = hashlib.sha256(transcript_path.read_bytes()).hexdigest()
    if expected_sha == "PENDING":
        pending.append(
            f"{app}: transcript exists but manifest sha256 still "
            f"PENDING (actual = {digest[:16]}…); update manifest to "
            f"sha256 = \"{digest}\""
        )
        continue
    if digest != expected_sha:
        failures.append(
            f"{app}: sha256 mismatch — expected {expected_sha[:16]}…, "
            f"got {digest[:16]}…"
        )
        continue

    # Gate 3 — unique tuple count.
    try:
        result = subprocess.run(
            [
                "cargo",
                "run",
                "--quiet",
                "-p",
                "oriterm_test_support",
                "--bin",
                "catalog_coverage_check",
                "--",
                "extract-capture-tuples",
                str(transcript_path),
            ],
            cwd=repo_root,
            capture_output=True,
            text=True,
            check=True,
        )
    except subprocess.CalledProcessError as e:
        failures.append(
            f"{app}: catalog_coverage_check extract failed: "
            f"{e.stderr.strip()}"
        )
        continue

    tuple_lines = [l for l in result.stdout.splitlines() if l.strip()]
    unique_tuple_count = len(tuple_lines)

    if unique_tuple_count < expected_min:
        failures.append(
            f"{app}: unique tuple count {unique_tuple_count} is "
            f"below expected minimum {expected_min} — script likely "
            f"idle or broken (see captures/scripts/README.md §Idle "
            f"rejection)"
        )
        continue

    clean.append(
        f"{app}: OK — sha256 {digest[:16]}…, {unique_tuple_count} "
        f"unique tuples"
    )

for line in clean:
    print(f"  ✓ {line}")
for line in pending:
    print(f"  … {line}")
for line in failures:
    print(f"  ✗ {line}")

print()
print(
    f"verify-manifest: {len(clean)} clean, "
    f"{len(pending)} pending, {len(failures)} failed"
)

if failures:
    sys.exit(1)
sys.exit(0)
PY
