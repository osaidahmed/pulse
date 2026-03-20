#!/bin/bash
set -euo pipefail

PGO_DIR="/tmp/pulse-pgo-data"
PROFILE_FILE="$PGO_DIR/merged.profdata"

echo "=== PGO Build for Pulse ==="

# Step 1: Clean previous PGO data
rm -rf "$PGO_DIR"
mkdir -p "$PGO_DIR"

# Step 2: Build instrumented binary
echo "[1/4] Building instrumented binary..."
RUSTFLAGS="-Cprofile-generate=$PGO_DIR" cargo build --release 2>&1 | tail -1

# Step 3: Run representative workload
echo "[2/4] Collecting profile data..."

# Generate a large test file
python3 -c "
for i in range(200):
    print(f'def fn_{i}(a, b, c, d, e, f):')
    for j in range(10):
        print(f'    if a == {j}:')
        print(f'        for x in range({j}):')
        print(f'            if b == {j}:')
        print(f'                pass')
    print(f'    return a')
    print()
" > /tmp/pgo_workload.py

# Run against fixture files and generated file
for f in tests/fixtures/python/*.py tests/fixtures/typescript/*.ts tests/fixtures/rust/*.rs tests/fixtures/java/*.java tests/fixtures/c/*.c tests/fixtures/cpp/*.cpp; do
    [ -f "$f" ] && ./target/release/pulse check "$f" > /dev/null 2>&1 || true
done

# Run against large file multiple times
for _ in $(seq 1 20); do
    ./target/release/pulse check /tmp/pgo_workload.py > /dev/null 2>&1
done

rm -f /tmp/pgo_workload.py

# Step 4: Merge profile data
echo "[3/4] Merging profiles..."
LLVM_PROFDATA=$(find "$(rustc --print sysroot)" -name "llvm-profdata" 2>/dev/null | head -1)

"$LLVM_PROFDATA" merge -o "$PROFILE_FILE" "$PGO_DIR"

# Step 5: Build optimized binary
echo "[4/4] Building PGO-optimized binary..."
RUSTFLAGS="-Cprofile-use=$PROFILE_FILE -Cllvm-args=-pgo-warn-missing-function" cargo build --release 2>&1 | tail -1

# Report
echo ""
echo "=== PGO build complete ==="
echo "Binary: target/release/pulse"
ls -lh target/release/pulse | awk '{print "Size:", $5}'

# Clean up
rm -rf "$PGO_DIR"
