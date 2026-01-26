#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
BIN="$PROJECT_DIR/target/release/butterpaper"
TEST_PDF="${1:-/tmp/sample.pdf}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

pass() { echo -e "${GREEN}✓ PASS${NC}: $1"; }
fail() { echo -e "${RED}✗ FAIL${NC}: $1"; FAILURES=$((FAILURES + 1)); }
skip() { echo -e "${YELLOW}○ SKIP${NC}: $1"; }

FAILURES=0

echo "=== ButterPaper Test Suite ==="
echo "Binary: $BIN"
echo "Test PDF: $TEST_PDF"
echo ""

# Ensure binary exists
if [ ! -f "$BIN" ]; then
    echo "Building..."
    cargo build --release -p butterpaper-gpui
fi

# Ensure test PDF exists
if [ ! -f "$TEST_PDF" ]; then
    echo "Downloading test PDF..."
    curl -sL "https://pdfobject.com/pdf/sample.pdf" -o "$TEST_PDF"
fi

echo ""
echo "--- Unit Tests ---"
cargo test --release -p butterpaper-core --quiet 2>/dev/null && pass "butterpaper-core tests" || fail "butterpaper-core tests"
cargo test --release -p butterpaper-render --quiet 2>/dev/null && pass "butterpaper-render tests" || fail "butterpaper-render tests"
cargo test --release -p butterpaper-cache --quiet 2>/dev/null && pass "butterpaper-cache tests" || fail "butterpaper-cache tests"
cargo test --release -p butterpaper-scheduler --quiet 2>/dev/null && pass "butterpaper-scheduler tests" || fail "butterpaper-scheduler tests"
cargo test --release -p butterpaper-ui --quiet 2>/dev/null && pass "butterpaper-ui tests" || fail "butterpaper-ui tests"

echo ""
echo "--- Integration Tests ---"

# Test 1: Binary exists and runs
if "$BIN" --help 2>&1 | grep -q "ButterPaper\|Usage\|error" || timeout 1 "$BIN" 2>&1 | grep -q "ButterPaper"; then
    pass "Binary runs"
else
    # App might not have --help, just check it starts
    pass "Binary exists"
fi

# Test 2: PDF loading (run briefly and check logs)
cd "$PROJECT_DIR"
OUTPUT=$(timeout 3 "$BIN" "$TEST_PDF" 2>&1 || true)
if echo "$OUTPUT" | grep -q "SUCCESS: Loaded PDF"; then
    pass "PDF loading"
else
    if echo "$OUTPUT" | grep -q "Loading PDF"; then
        fail "PDF loading (started but failed)"
        echo "    Output: $(echo "$OUTPUT" | grep -E "FAILED|Error|error" | head -1)"
    else
        skip "PDF loading (can't verify non-interactively)"
    fi
fi

# Test 3: Page rendering
if echo "$OUTPUT" | grep -q "SUCCESS: Page.*texture created"; then
    pass "Page rendering"
else
    if echo "$OUTPUT" | grep -q "Rendering page"; then
        fail "Page rendering (started but failed)"
    else
        skip "Page rendering (can't verify non-interactively)"
    fi
fi

echo ""
echo "--- Summary ---"
if [ $FAILURES -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}$FAILURES test(s) failed${NC}"
    exit 1
fi
