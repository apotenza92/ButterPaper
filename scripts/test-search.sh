#!/usr/bin/env bash
# Test PDF Editor search functionality using --search flag
#
# This script tests the --search CLI functionality:
# - Search for text that exists in the PDF
# - Search for text that doesn't exist
# - Test case-insensitive search
# - Test empty search results
#
# Usage: ./scripts/test-search.sh

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

echo "=== PDF Editor Search Test Suite ==="
echo ""

# Ensure binary exists
if [ ! -f "$BIN" ]; then
    info "Building release binary..."
    cargo build --release -p pdf-editor
fi

# Create test directory
mkdir -p "$TEST_DIR"

# Create a PDF with searchable text content
create_text_pdf() {
    local path="$TEST_DIR/searchable.pdf"
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
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792]
   /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>
endobj
4 0 obj
<< /Length 200 >>
stream
BT
/F1 12 Tf
50 700 Td
(Hello World) Tj
0 -20 Td
(This is a test PDF document.) Tj
0 -20 Td
(It contains searchable text content.) Tj
0 -20 Td
(Apple Banana Cherry) Tj
0 -20 Td
(apple banana cherry) Tj
ET
endstream
endobj
5 0 obj
<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>
endobj
xref
0 6
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
0000000248 00000 n
0000000500 00000 n
trailer
<< /Size 6 /Root 1 0 R >>
startxref
578
%%EOF
PDFEOF
        info "Created searchable.pdf"
    fi
}

# Create the test PDFs
info "Preparing test PDFs..."
echo ""

create_text_pdf

# Test functions
test_search() {
    local pdf="$1"
    local query="$2"
    local expected_found="$3"  # "yes" or "no"
    local description="$4"

    OUTPUT=$("$BIN" --search "$query" "$pdf" 2>&1) || true

    if echo "$OUTPUT" | grep -q "^FOUND:"; then
        # Extract count from output
        COUNT=$(echo "$OUTPUT" | grep "^FOUND:" | sed 's/.*count=\([0-9]*\).*/\1/')

        if [ "$expected_found" = "yes" ] && [ "$COUNT" -gt 0 ]; then
            pass "$description (found $COUNT matches)"
        elif [ "$expected_found" = "no" ] && [ "$COUNT" -eq 0 ]; then
            pass "$description (no matches as expected)"
        elif [ "$expected_found" = "yes" ]; then
            fail "$description (expected matches, got 0)"
        else
            fail "$description (expected no matches, got $COUNT)"
        fi
    elif echo "$OUTPUT" | grep -q "^SEARCH: FAILED"; then
        ERROR=$(echo "$OUTPUT" | grep "^SEARCH: FAILED" | sed 's/SEARCH: FAILED error=//')
        fail "$description: $ERROR"
    else
        fail "$description: unexpected output: $OUTPUT"
    fi
}

test_search_error() {
    local args="$1"
    local expected_error="$2"
    local description="$3"

    OUTPUT=$("$BIN" $args 2>&1) || true
    EXIT_CODE=$?

    if echo "$OUTPUT" | grep -q "$expected_error"; then
        pass "$description"
    else
        fail "$description: expected '$expected_error', got: $OUTPUT"
    fi
}

echo "--- Search Tests ---"
echo ""

PDF="$TEST_DIR/searchable.pdf"

# Test 1: Search for existing text (exact case)
test_search "$PDF" "Hello" "yes" "Search for 'Hello' (exists)"

# Test 2: Search for existing text (different case - should still find due to case-insensitive)
test_search "$PDF" "hello" "yes" "Search for 'hello' (case-insensitive)"

# Test 3: Search for text that doesn't exist
test_search "$PDF" "nonexistent12345" "no" "Search for nonexistent text"

# Test 4: Search for word in the text
test_search "$PDF" "PDF" "yes" "Search for 'PDF'"

# Test 5: Search for another word
test_search "$PDF" "document" "yes" "Search for 'document'"

# Test 6: Search for fruit names (appears twice - once uppercase, once lowercase)
test_search "$PDF" "apple" "yes" "Search for 'apple' (appears in both cases)"

# Test 7: Search for 'World'
test_search "$PDF" "World" "yes" "Search for 'World'"

# Test error cases
echo ""
echo "--- Error Handling Tests ---"
echo ""

# Test missing PDF argument
test_search_error "--search test" "no PDF file specified" "Error when no PDF specified"

# Test missing search term
test_search_error "--search" "no search term specified" "Error when no search term specified"

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
    echo -e "${GREEN}All search tests passed!${NC}"
    exit 0
fi
