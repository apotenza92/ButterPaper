#!/usr/bin/env bash
# Test PDF Editor --list-annotations functionality
#
# This script tests the --list-annotations CLI functionality:
# - List annotations from a PDF with annotations
# - List annotations from a PDF without annotations
# - Test error handling for missing file
#
# Usage: ./scripts/test-list-annotations.sh

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

echo "=== PDF Editor List Annotations Test Suite ==="
echo ""

# Ensure binary exists
if [ ! -f "$BIN" ]; then
    info "Building release binary..."
    cargo build --release -p pdf-editor
fi

# Create test directory
mkdir -p "$TEST_DIR"

# Create a minimal PDF without annotations
create_no_annotations_pdf() {
    local path="$TEST_DIR/no-annotations.pdf"
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
<< /Length 60 >>
stream
BT
/F1 12 Tf
50 700 Td
(No annotations here.) Tj
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
0000000360 00000 n
trailer
<< /Size 6 /Root 1 0 R >>
startxref
438
%%EOF
PDFEOF
        info "Created no-annotations.pdf"
    fi
}

# Create a PDF with a highlight annotation
create_annotated_pdf() {
    local path="$TEST_DIR/with-annotations.pdf"
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
   /Contents 4 0 R
   /Resources << /Font << /F1 6 0 R >> >>
   /Annots [5 0 R] >>
endobj
4 0 obj
<< /Length 80 >>
stream
BT
/F1 12 Tf
50 700 Td
(This text has a highlight annotation.) Tj
ET
endstream
endobj
5 0 obj
<< /Type /Annot /Subtype /Highlight
   /Rect [50 695 200 715]
   /QuadPoints [50 715 200 715 50 695 200 695]
   /C [1 1 0]
   /T (Test Author)
   /Contents (Test annotation note) >>
endobj
6 0 obj
<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>
endobj
xref
0 7
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
0000000276 00000 n
0000000408 00000 n
0000000590 00000 n
trailer
<< /Size 7 /Root 1 0 R >>
startxref
668
%%EOF
PDFEOF
        info "Created with-annotations.pdf"
    fi
}

# Create the test PDFs
info "Preparing test PDFs..."
echo ""

create_no_annotations_pdf
create_annotated_pdf

# Test functions
test_list_annotations() {
    local pdf="$1"
    local expected_count="$2"  # Expected minimum count (for validation)
    local description="$3"

    # Run command and capture stdout and stderr separately
    STDOUT=$("$BIN" --list-annotations "$pdf" 2>/dev/null) || true
    STDERR=$("$BIN" --list-annotations "$pdf" 2>&1 >/dev/null) || true

    # Check if output is valid JSON array
    if echo "$STDOUT" | python3 -c "import sys, json; json.load(sys.stdin)" 2>/dev/null; then
        # Count elements in the JSON array
        COUNT=$(echo "$STDOUT" | python3 -c "import sys, json; print(len(json.load(sys.stdin)))")

        if [ "$expected_count" -eq 0 ] && [ "$COUNT" -eq 0 ]; then
            pass "$description (empty array as expected)"
        elif [ "$expected_count" -gt 0 ] && [ "$COUNT" -ge "$expected_count" ]; then
            pass "$description (found $COUNT annotations)"
        elif [ "$expected_count" -eq 0 ]; then
            fail "$description (expected 0 annotations, got $COUNT)"
        else
            fail "$description (expected at least $expected_count annotations, got $COUNT)"
        fi
    elif echo "$STDERR" | grep -q "^ANNOTATIONS: FAILED"; then
        ERROR=$(echo "$STDERR" | grep "^ANNOTATIONS: FAILED" | sed 's/ANNOTATIONS: FAILED error=//')
        fail "$description: $ERROR"
    else
        fail "$description: output is not valid JSON: $STDOUT"
    fi
}

test_list_annotations_error() {
    local args="$1"
    local expected_error="$2"
    local description="$3"

    OUTPUT=$("$BIN" $args 2>&1) || true

    if echo "$OUTPUT" | grep -q "$expected_error"; then
        pass "$description"
    else
        fail "$description: expected '$expected_error', got: $OUTPUT"
    fi
}

echo "--- List Annotations Tests ---"
echo ""

# Test 1: PDF without annotations - should return empty JSON array
test_list_annotations "$TEST_DIR/no-annotations.pdf" 0 "PDF without annotations returns empty array"

# Test 2: PDF with annotations - should return JSON array with annotations
test_list_annotations "$TEST_DIR/with-annotations.pdf" 1 "PDF with annotations returns JSON array"

# Test 3: Verify JSON structure has expected fields
echo ""
echo "--- JSON Structure Tests ---"
echo ""

PDF="$TEST_DIR/with-annotations.pdf"
STDOUT=$("$BIN" --list-annotations "$PDF" 2>/dev/null) || true

# Check for expected fields in the JSON output
check_json_field() {
    local field="$1"
    local description="$2"

    if echo "$STDOUT" | python3 -c "import sys, json; d=json.load(sys.stdin); assert any('$field' in str(x) for x in d)" 2>/dev/null; then
        pass "$description"
    else
        fail "$description"
    fi
}

# Verify essential fields exist in the annotation JSON
if [ -n "$STDOUT" ] && echo "$STDOUT" | python3 -c "import sys, json; d=json.load(sys.stdin); len(d) > 0" 2>/dev/null; then
    # Check for page_index field
    if echo "$STDOUT" | grep -q '"page_index"'; then
        pass "JSON contains page_index field"
    else
        fail "JSON missing page_index field"
    fi

    # Check for geometry field
    if echo "$STDOUT" | grep -q '"geometry"'; then
        pass "JSON contains geometry field"
    else
        fail "JSON missing geometry field"
    fi

    # Check for style field
    if echo "$STDOUT" | grep -q '"style"'; then
        pass "JSON contains style field"
    else
        fail "JSON missing style field"
    fi
else
    info "Skipping JSON structure tests (no annotations found)"
fi

# Test error cases
echo ""
echo "--- Error Handling Tests ---"
echo ""

# Test missing PDF argument
test_list_annotations_error "--list-annotations" "no PDF file specified" "Error when no PDF specified"

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
    echo -e "${GREEN}All list-annotations tests passed!${NC}"
    exit 0
fi
