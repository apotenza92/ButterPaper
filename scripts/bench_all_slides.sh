#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
PDF_PATH="${1:-$ROOT_DIR/samples/All Slides + Cases.pdf}"
OUT_PATH="${2:-/tmp/butterpaper-benchmark.json}"

echo "Running benchmark on: $PDF_PATH"
cargo run -p butterpaper -- \
  --benchmark-scroll \
  --benchmark-file "$PDF_PATH" \
  --benchmark-seconds 45 \
  --benchmark-output "$OUT_PATH"

python3 - "$OUT_PATH" <<'PY'
import json
import sys
from pathlib import Path

path = Path(sys.argv[1])
if not path.exists():
    print(f"benchmark output not found: {path}", file=sys.stderr)
    sys.exit(2)

data = json.loads(path.read_text())
print(f"Benchmark pass: {data.get('pass')}")
if not data.get("pass"):
    reasons = data.get("fail_reasons", [])
    if reasons:
        print("Fail reasons:")
        for reason in reasons:
            print(f"- {reason}")
    sys.exit(2)
PY

echo "Benchmark output: $OUT_PATH"
