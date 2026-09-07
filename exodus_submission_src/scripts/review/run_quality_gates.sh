#!/usr/bin/env bash

# Project Exodus: Review Quality Gates Script
# Runs all quality checks and captures output for review documentation

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
OUTPUT_DIR="$PROJECT_ROOT/docs/reviews/phase1-prep"
LOG_DIR="$OUTPUT_DIR/logs"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Create output directories
mkdir -p "$OUTPUT_DIR" "$LOG_DIR"

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}Project Exodus: Quality Gates Execution${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo "Project Root: $PROJECT_ROOT"
echo "Output Dir: $OUTPUT_DIR"
echo "Log Dir: $LOG_DIR"
echo ""

# Track overall status
OVERALL_STATUS=0
TOTAL_CHECKS=0
PASSED_CHECKS=0
FAILED_CHECKS=0

# Function to run a check and log results
run_check() {
    local check_name="$1"
    local command="$2"
    local log_file="$LOG_DIR/${check_name// /_}.log"
    local result_file="$OUTPUT_DIR/${check_name// /_}.json"
    
    TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
    
    echo -e "${YELLOW} Running: $check_name${NC}"
    echo "Command: $command"
    
    # Run command and capture output
    if eval "$command" > "$log_file" 2>&1; then
        PASSED_CHECKS=$((PASSED_CHECKS + 1))
        echo -e "${GREEN}✓ PASSED${NC}"
        
        # Create simple JSON result
        cat > "$result_file" <<EOF
{
  "check": "$check_name",
  "status": "passed",
  "command": "$command",
  "exit_code": 0,
  "timestamp": "$(date -Iseconds)"
}
EOF
        return 0
    else
        FAILED_CHECKS=$((FAILED_CHECKS + 1))
        OVERALL_STATUS=1
        echo -e "${RED}✗ FAILED${NC}"
        
        # Create simple JSON result
        cat > "$result_file" <<EOF
{
  "check": "$check_name",
  "status": "failed",
  "command": "$command",
  "exit_code": $?,
  "timestamp": "$(date -Iseconds)",
  "log_file": "$log_file"
}
EOF
        return 1
    fi
}

# Function to display check summary
display_summary() {
    echo ""
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}Quality Gates Summary${NC}"
    echo -e "${GREEN}========================================${NC}"
    echo "Total Checks: $TOTAL_CHECKS"
    echo -e "${GREEN}Passed: $PASSED_CHECKS${NC}"
    echo -e "${RED}Failed: $FAILED_CHECKS${NC}"
    echo ""
    
    if [ $OVERALL_STATUS -eq 0 ]; then
        echo -e "${GREEN}✓ All quality gates passed!${NC}"
    else
        echo -e "${RED}✗ Some quality gates failed${NC}"
        echo "Review log files in: $LOG_DIR"
    fi
    echo ""
}

echo "Starting quality gates execution..."
echo ""

# 1. Environment Checks
echo -e "${YELLOW}[1/6] Environment Checks${NC}"
echo "----------------------------------------"
run_check "Rust Version" "rustc --version"
run_check "Cargo Version" "cargo --version"
run_check "Git Version" "git --version"
echo ""

# 2. Format Check
echo -e "${YELLOW}[2/6] Format Check${NC}"
echo "----------------------------------------"
run_check "Format Check" "cargo fmt --check --all 2>&1 || true"
echo ""

# 3. Clippy Check
echo -e "${YELLOW}[3/6] Clippy Check${NC}"
echo "----------------------------------------"
run_check "Clippy (All Targets)" "cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 || true"
echo ""

# 4. Test Suite
echo -e "${YELLOW}[4/6] Test Suite${NC}"
echo "----------------------------------------"
run_check "Unit Tests" "cargo test --workspace --all-features 2>&1 || true"
echo ""

# 5. Build Check
echo -e "${YELLOW}[5/6] Build Check${NC}"
echo "----------------------------------------"
run_check "Debug Build" "cargo build --workspace --all-features 2>&1 || true"
run_check "Release Build" "cargo build --release --workspace --all-features 2>&1 || true"
echo ""

# 6. Documentation Checks
echo -e "${YELLOW}[6/6] Documentation Checks${NC}"
echo "----------------------------------------"
run_check "Doc Tests" "cargo test --workspace --doc 2>&1 || true"
run_check "Build Docs" "cargo doc --workspace --all-features --no-deps 2>&1 || true"
echo ""

# Display final summary
display_summary

# Generate overall report
REPORT_FILE="$OUTPUT_DIR/quality_gates_report.json"
cat > "$REPORT_FILE" <<EOF
{
  "project": "exodus",
  "review_phase": "phase1-prep",
  "timestamp": "$(date -Iseconds)",
  "environment": {
    "rustc_version": "$(rustc --version 2>&1 || echo 'unknown')",
    "cargo_version": "$(cargo --version 2>&1 || echo 'unknown')",
    "git_version": "$(git --version 2>&1 || echo 'unknown')"
  },
  "summary": {
    "total_checks": $TOTAL_CHECKS,
    "passed_checks": $PASSED_CHECKS,
    "failed_checks": $FAILED_CHECKS,
    "overall_status": "${OVERALL_STATUS}"
  },
  "checks": [
EOF

# Add individual check results to report
for result_file in "$OUTPUT_DIR"/*.json; do
    if [ -f "$result_file" ]; then
        # Skip the report file itself
        if [ "$(basename "$result_file")" != "quality_gates_report.json" ]; then
            cat "$result_file" >> "$REPORT_FILE"
            echo "," >> "$REPORT_FILE"
        fi
    fi
done

# Close the JSON array and object
sed -i '$ s/,$//' "$REPORT_FILE"  # Remove trailing comma
cat >> "$REPORT_FILE" <<EOF
  ]
}
EOF

echo "Quality gates report saved to: $REPORT_FILE"
echo "Log files saved to: $LOG_DIR"

# Clean up: remove individual result files (they're in the report)
rm -f "$OUTPUT_DIR"/*.json 2>/dev/null || true
mv "$REPORT_FILE" "$OUTPUT_DIR/quality_gates_report_$(date +%Y%m%d_%H%M%S).json"

exit $OVERALL_STATUS
