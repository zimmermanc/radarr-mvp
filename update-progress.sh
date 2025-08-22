#!/bin/bash

# Radarr MVP Progress Updater
# Run this after completing work to update 01-plan.md with current status

echo "=== Radarr MVP Progress Updater ==="
echo ""

# Get current directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR/unified-radarr" || exit 1

# Check compilation status
echo "Checking compilation status..."
if cargo build --workspace 2>&1 | grep -q "error\["; then
    ERROR_COUNT=$(cargo build --workspace 2>&1 | grep -c "error\[" || echo "unknown")
    COMPILE_STATUS="❌ BLOCKED - $ERROR_COUNT errors"
    COMPILE_BOOL=false
else
    COMPILE_STATUS="✅ WORKING"
    COMPILE_BOOL=true
fi

# Count warnings
WARNING_COUNT=$(cargo build --workspace 2>&1 | grep -c "warning:" || echo 0)

# Run tests if compilation works
if [ "$COMPILE_BOOL" = true ]; then
    echo "Running tests..."
    TEST_OUTPUT=$(cargo test --workspace 2>&1)
    TESTS_PASSED=$(echo "$TEST_OUTPUT" | grep -oP '\d+(?= passed)' | tail -1 || echo 0)
    TESTS_FAILED=$(echo "$TEST_OUTPUT" | grep -oP '\d+(?= failed)' | tail -1 || echo 0)
    TEST_STATUS="$TESTS_PASSED passed, $TESTS_FAILED failed"
else
    TEST_STATUS="Cannot run - compilation blocked"
    TESTS_PASSED=0
    TESTS_FAILED=0
fi

# Count features
WORKING_FEATURES=$(grep -c "✅" "$SCRIPT_DIR/.architecture/02-component-design.md" 2>/dev/null || echo 0)
BROKEN_FEATURES=$(grep -c "❌" "$SCRIPT_DIR/.architecture/02-component-design.md" 2>/dev/null || echo 0)
PARTIAL_FEATURES=$(grep -c "⚠️" "$SCRIPT_DIR/.architecture/02-component-design.md" 2>/dev/null || echo 0)

# Calculate rough completion percentage
TOTAL_COMPONENTS=10
COMPLETION=$((WORKING_FEATURES * 100 / TOTAL_COMPONENTS))

# Generate update text
UPDATE_DATE=$(date +"%Y-%m-%d %H:%M")

cat << EOF

=== CURRENT STATUS ===
Date: $UPDATE_DATE
Compilation: $COMPILE_STATUS
Warnings: $WARNING_COUNT
Tests: $TEST_STATUS
Features: $WORKING_FEATURES working, $PARTIAL_FEATURES partial, $BROKEN_FEATURES broken
Estimated Completion: ~$COMPLETION%

=== UPDATE TEMPLATE ===
Copy this into 01-plan.md under "Latest Progress Update":

## 📈 Latest Progress Update
**Last Updated**: $UPDATE_DATE
**Compilation Status**: $COMPILE_STATUS
**Tests Passing**: $TESTS_PASSED/$((TESTS_PASSED + TESTS_FAILED))
**MVP Completion**: ~$COMPLETION%

### Recent Achievements
- ✅ $(date +%Y-%m-%d) [Add your completed task here]
- 🔄 Currently working on: [Current task]

EOF

# Prompt for manual update
echo "=== MANUAL STEPS ==="
echo "1. Copy the update template above into 01-plan.md"
echo "2. Add specific tasks you completed"
echo "3. Update the component status table if needed"
echo "4. Commit your changes:"
echo ""
echo "   git add 01-plan.md"
echo "   git commit -m \"docs: progress update - [describe what you completed]\""
echo ""

# Check for critical milestones
if [ "$COMPILE_BOOL" = true ] && [ "$ERROR_COUNT" = "unknown" ]; then
    echo "🎉 MILESTONE: Project now compiles! This is a major achievement!"
    echo "   Previous status: 164 compilation errors"
    echo "   Current status: 0 compilation errors"
fi

if [ "$TESTS_PASSED" -gt 20 ] && [ "$TESTS_FAILED" -eq 0 ]; then
    echo "🎉 MILESTONE: All tests passing!"
fi

echo ""
echo "=== END PROGRESS UPDATE ==="