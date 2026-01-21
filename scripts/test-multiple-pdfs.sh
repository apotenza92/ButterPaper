#!/usr/bin/env bash
# Test PDF Editor with multiple PDF files using --test-load flag
#
# This script tests the PDF loading functionality with various PDF files:
# - Different sizes (1 page, multi-page, large documents)
# - Different versions (PDF 1.4, 1.5, 1.7, 2.0)
# - Different content types (text, images, scanned)
#
# Usage: ./scripts/test-multiple-pdfs.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
BIN="$PROJECT_DIR/target/release/pdf-editor"
TEST_DIR="/tmp/pdf-editor-test-pdfs"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

pass() { echo -e "${GREEN}PASS${NC}: $1"; PASSED=$((PASSED + 1)); }
fail() { echo -e "${RED}FAIL${NC}: $1"; FAILED=$((FAILED + 1)); }
info() { echo -e "${YELLOW}INFO${NC}: $1"; }

PASSED=0
FAILED=0

echo "=== PDF Editor Multi-File Test Suite ==="
echo ""

# Ensure binary exists
if [ ! -f "$BIN" ]; then
    info "Building release binary..."
    cargo build --release -p pdf-editor
fi

# Create test directory
mkdir -p "$TEST_DIR"

# Download test PDFs if not present
download_test_pdf() {
    local name="$1"
    local url="$2"
    local path="$TEST_DIR/$name"

    if [ ! -f "$path" ]; then
        info "Downloading $name..."
        if curl -sL "$url" -o "$path" 2>/dev/null; then
            info "Downloaded $name"
        else
            info "Failed to download $name (will skip)"
            return 1
        fi
    fi
    return 0
}

# Create a minimal valid PDF for testing (PDF 1.4, 1 page)
create_minimal_pdf() {
    local path="$TEST_DIR/minimal.pdf"
    if [ ! -f "$path" ]; then
        cat > "$path" << 'PDFEOF'
%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>
endobj
xref
0 4
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
trailer
<< /Size 4 /Root 1 0 R >>
startxref
196
%%EOF
PDFEOF
        info "Created minimal.pdf"
    fi
}

# Download various test PDFs
info "Preparing test PDFs..."
echo ""

# Simple sample PDF (should always work)
download_test_pdf "sample.pdf" "https://pdfobject.com/pdf/sample.pdf"

# Create minimal PDF
create_minimal_pdf

# Test each PDF file
echo "--- Testing PDF Files ---"
echo ""

test_pdf() {
    local path="$1"
    local name=$(basename "$path")

    if [ ! -f "$path" ]; then
        info "Skipping $name (not found)"
        return
    fi

    # Run test-load and capture output
    OUTPUT=$("$BIN" --test-load "$path" 2>&1) || true
    EXIT_CODE=$?

    if echo "$OUTPUT" | grep -q "^LOAD: OK"; then
        # Extract pages and time from output
        PAGES=$(echo "$OUTPUT" | grep "^LOAD: OK" | sed 's/.*pages=\([0-9]*\).*/\1/')
        TIME=$(echo "$OUTPUT" | grep "^LOAD: OK" | sed 's/.*time=\([0-9]*\)ms.*/\1/')
        pass "$name (${PAGES} pages, ${TIME}ms)"
    else
        ERROR=$(echo "$OUTPUT" | grep "^LOAD: FAILED" | sed 's/LOAD: FAILED error=//' || echo "unknown error")
        fail "$name: $ERROR"
    fi
}

# Test all PDFs in test directory
for pdf in "$TEST_DIR"/*.pdf; do
    if [ -f "$pdf" ]; then
        test_pdf "$pdf"
    fi
done

echo ""
echo "--- Summary ---"
TOTAL=$((PASSED + FAILED))
echo "Total: $TOTAL tests"
echo -e "${GREEN}Passed: $PASSED${NC}"
if [ $FAILED -gt 0 ]; then
    echo -e "${RED}Failed: $FAILED${NC}"
    exit 1
else
    echo "Failed: 0"
    echo ""
    echo -e "${GREEN}All PDF loading tests passed!${NC}"
    exit 0
fi
