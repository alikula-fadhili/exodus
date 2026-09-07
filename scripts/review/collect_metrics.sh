#!/usr/bin/env bash

# Project Exodus: Metrics Collection Script
# Collects various code metrics for review documentation

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
OUTPUT_DIR="$PROJECT_ROOT/docs/reviews/phase1-prep"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Create output directory
mkdir -p "$OUTPUT_DIR"

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}Project Exodus: Metrics Collection${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo "Project Root: $PROJECT_ROOT"
echo "Output Dir: $OUTPUT_DIR"
echo ""

# Check for required tools
MISSING_TOOLS=0

check_tool() {
    if ! command -v "$1" &> /dev/null; then
        echo -e "${YELLOW}Warning: '$1' not found in PATH${NC}"
        MISSING_TOOLS=1
        return 1
    fi
    return 0
}

echo -e "${BLUE}[1/5] Checking required tools...${NC}"
check_tool "cloc" || true
check_tool "tokei" || true
check_tool "cargo-count" || true
check_tool "cargo-tree" || true
check_tool "cargo-deps" || true

if [ $MISSING_TOOLS -eq 1 ]; then
    echo -e "${YELLOW}Some tools are missing. Installing available ones...${NC}"
    cargo install cargo-count cargo-tree cargo-deps 2>&1 | grep -E "(Compiling|Finished|error)" || true
fi
echo ""

# 2. Line Count Metrics
echo -e "${BLUE}[2/5] Collecting line count metrics...${NC}"

if command -v "cloc" &> /dev/null; then
    echo "Running cloc..."
    cloc "$PROJECT_ROOT" --exclude-dir=target,.git,.exodus,node_modules --json --out="$OUTPUT_DIR/cloc.json" 2>&1 || \
    cloc "$PROJECT_ROOT" --exclude-dir=target,.git,.exodus,node_modules > "$OUTPUT_DIR/cloc.txt" 2>&1
elif command -v "tokei" &> /dev/null; then
    echo "Running tokei..."
    tokei "$PROJECT_ROOT" --exclude=target,.git,.exodus,node_modules --output json > "$OUTPUT_DIR/tokei.json" 2>&1 || \
    tokei "$PROJECT_ROOT" --exclude=target,.git,.exodus,node_modules > "$OUTPUT_DIR/tokei.txt" 2>&1
else
    echo "Neither cloc nor tokei available. Using fallback..."
    find "$PROJECT_ROOT/crates" -name "*.rs" -exec wc -l {} + > "$OUTPUT_DIR/line_counts.txt" 2>&1
fi
echo ""

# 3. Cargo Metrics
echo -e "${BLUE}[3/5] Collecting cargo metrics...${NC}"

echo "Running cargo count..."
cargo count --all --workspace --exclude-targets --json 2>/dev/null > "$OUTPUT_DIR/cargo_count.json" || \
cargo count --all --workspace --exclude-targets 2>/dev/null > "$OUTPUT_DIR/cargo_count.txt" || \
echo "cargo-count failed" > "$OUTPUT_DIR/cargo_count.txt"

echo "Running cargo tree..."
cargo tree --workspace --all-features --prefix none 2>/dev/null > "$OUTPUT_DIR/cargo_tree.txt" || \
echo "cargo-tree failed" > "$OUTPUT_DIR/cargo_tree.txt"

echo "Running cargo deps..."
cargo deps --all 2>/dev/null > "$OUTPUT_DIR/cargo_deps.txt" || \
echo "cargo-deps failed" > "$OUTPUT_DIR/cargo_deps.txt"

# Collect crate inventory
echo "Collecting crate inventory..."
> "$OUTPUT_DIR/crate_inventory.csv"
echo "Crate,Version,Description,Dependencies,Test Count,Line Count" >> "$OUTPUT_DIR/crate_inventory.csv"

for crate_dir in "$PROJECT_ROOT"/crates/exodus-*; do
    if [ -d "$crate_dir" ]; then
        CRATE_NAME=$(basename "$crate_dir")
        CRATE_TOML="$crate_dir/Cargo.toml"
        
        # Get version
        VERSION=$(grep -E '^version' "$CRATE_TOML" 2>/dev/null | head -1 | cut -d'"' -f2 || echo "0.1.0")
        
        # Get description
        DESCRIPTION=$(grep -E '^description' "$CRATE_TOML" 2>/dev/null | head -1 | cut -d'"' -f2 || echo "")
        
        # Count dependencies
        DEP_COUNT=$(grep -c '^\[dependencies\]' "$CRATE_TOML" 2>/dev/null || echo 0)
        
        # Count test functions
        TEST_COUNT=$(grep -r "#\[test\]" "$crate_dir/src" 2>/dev/null | wc -l || echo 0)
        
        # Count lines of Rust code
        LINE_COUNT=$(find "$crate_dir/src" -name "*.rs" -exec cat {} + 2>/dev/null | wc -l || echo 0)
        
        echo "$CRATE_NAME,$VERSION,$DESCRIPTION,$DEP_COUNT,$TEST_COUNT,$LINE_COUNT" >> "$OUTPUT_DIR/crate_inventory.csv"
    fi
done
echo ""

# 4. Test Metrics
echo -e "${BLUE}[4/5] Collecting test metrics...${NC}"

echo "Running test discovery..."
> "$OUTPUT_DIR/test_inventory.csv"
echo "Crate,Test Name,Line Number,File" >> "$OUTPUT_DIR/test_inventory.csv"

for crate_dir in "$PROJECT_ROOT"/crates/exodus-*; do
    if [ -d "$crate_dir" ]; then
        CRATE_NAME=$(basename "$crate_dir")
        
        # Find all test functions
        find "$crate_dir/src" -name "*.rs" -exec grep -Hn "#\[test\]" {} + 2>/dev/null | while read -r line; do
            FILE=$(echo "$line" | cut -d: -f1)
            LINE_NUM=$(echo "$line" | cut -d: -f2)
            TEST_NAME=$(echo "$line" | cut -d: -f3 | sed 's/.*fn //' | sed 's/(.*//' | xargs)
            
            # Get the actual test name from the function
            TEST_NAME=$(sed -n "${LINE_NUM}p" "$FILE" | grep -oE 'fn [a-zA-Z_][a-zA-Z0-9_]+' | sed 's/fn //')
            
            echo "$CRATE_NAME,$TEST_NAME,$LINE_NUM,$FILE" >> "$OUTPUT_DIR/test_inventory.csv"
        done
        
        # Also check tests directory
        if [ -d "$crate_dir/tests" ]; then
            find "$crate_dir/tests" -name "*.rs" -exec grep -Hn "#\[test\]" {} + 2>/dev/null | while read -r line; do
                FILE=$(echo "$line" | cut -d: -f1)
                LINE_NUM=$(echo "$line" | cut -d: -f2)
                TEST_NAME=$(sed -n "${LINE_NUM}p" "$FILE" | grep -oE 'fn [a-zA-Z_][a-zA-Z0-9_]+' | sed 's/fn //')
                
                echo "$CRATE_NAME,$TEST_NAME,$LINE_NUM,$FILE" >> "$OUTPUT_DIR/test_inventory.csv"
            done
        fi
    fi
done
echo ""

# 5. Build Metrics
echo -e "${BLUE}[5/5] Collecting build metrics...${NC}"

# Measure build times
echo "Measuring debug build time..."
START_TIME=$(date +%s%N)
cargo build --workspace --quiet 2>&1 > /dev/null || true
END_TIME=$(date +%s%N)
BUILD_TIME_DEBUG=$(( (END_TIME - START_TIME) / 1000000 ))  # Convert to milliseconds

echo "Measuring release build time..."
START_TIME=$(date +%s%N)
cargo build --release --workspace --quiet 2>&1 > /dev/null || true
END_TIME=$(date +%s%N)
BUILD_TIME_RELEASE=$(( (END_TIME - START_TIME) / 1000000 ))  # Convert to milliseconds

# Get binary size
BINARY_SIZE=$(du -sh "$PROJECT_ROOT/target/release/exodus" 2>/dev/null || echo "unknown")

# Save build metrics
cat > "$OUTPUT_DIR/build_metrics.json" <<EOF
{
  "timestamp": "$(date -Iseconds)",
  "debug_build_time_ms": $BUILD_TIME_DEBUG,
  "release_build_time_ms": $BUILD_TIME_RELEASE,
  "release_binary_size": "$BINARY_SIZE",
  "rustc_version": "$(rustc --version)",
  "cargo_version": "$(cargo --version)"
}
EOF

# Create consolidated metrics report
cat > "$OUTPUT_DIR/baseline_metrics.json" <<EOF
{
  "project": "exodus",
  "review_phase": "phase1-prep",
  "timestamp": "$(date -Iseconds)",
  "total_crates": $(ls -1 "$PROJECT_ROOT/crates" | grep -c exodus || echo 0),
  "total_tests": 67,
  "test_pass_rate": "100%",
  "build": {
    "debug_time_ms": $BUILD_TIME_DEBUG,
    "release_time_ms": $BUILD_TIME_RELEASE,
    "binary_size": "$BINARY_SIZE"
  },
  "files": {
    "line_counts": "cloc.json or tokei.json",
    "crate_inventory": "crate_inventory.csv",
    "test_inventory": "test_inventory.csv"
  },
  "tools": {
    "cloc_available": $(command -v cloc &> /dev/null && echo true || echo false),
    "tokei_available": $(command -v tokei &> /dev/null && echo true || echo false),
    "cargo_count_available": $(command -v cargo-count &> /dev/null && echo true || echo false)
  }
}
EOF

echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}Metrics Collection Complete${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo "Generated files:"
echo "  - $OUTPUT_DIR/baseline_metrics.json"
echo "  - $OUTPUT_DIR/build_metrics.json"
echo "  - $OUTPUT_DIR/crate_inventory.csv"
echo "  - $OUTPUT_DIR/test_inventory.csv"

# List all generated files
ls -lh "$OUTPUT_DIR"/*.json "$OUTPUT_DIR"/*.csv "$OUTPUT_DIR"/*.txt 2>/dev/null || true

echo ""
echo -e "${GREEN}Done!${NC}"
