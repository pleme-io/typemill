#!/bin/bash

# Run tests sequentially to avoid race conditions
echo "Running tests sequentially..."

FAILED=0
PASSED=0
TOTAL=0

for file in tests/unit/*.test.ts; do
    echo -n "Testing $(basename $file)... "
    
    # Run test and capture output
    if output=$(bun test "$file" 2>&1); then
        # Extract pass/fail counts
        if echo "$output" | grep -q "0 fail"; then
            echo "✅ PASS"
            PASSED=$((PASSED + 1))
        else
            echo "❌ FAIL"
            FAILED=$((FAILED + 1))
            echo "$output" | grep "(fail)" | head -5
        fi
    else
        echo "❌ ERROR"
        FAILED=$((FAILED + 1))
    fi
    
    TOTAL=$((TOTAL + 1))
done

echo ""
echo "========================================="
echo "Test Summary: $PASSED/$TOTAL passed, $FAILED failed"
echo "========================================="

if [ $FAILED -eq 0 ]; then
    echo "✅ All tests passed!"
    exit 0
else
    echo "❌ Some tests failed"
    exit 1
fi