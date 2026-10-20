#!/usr/bin/env bash
set -e

# Rey Standard Library Master Test Runner (28 Modules + Edge Cases)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
STD_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
REYC_BIN="${STD_DIR}/../rey-lang/compiler/v1/target/release/rey-v0"

if [ ! -f "${REYC_BIN}" ]; then
    echo "Compiler binary not found at ${REYC_BIN}"
    echo "Building compiler..."
    (cd "${STD_DIR}/../rey-lang/compiler/v1" && cargo build --release)
fi

echo "=========================================="
echo " Running Rey Standard Library Test Suite"
echo " Compiler: ${REYC_BIN}"
echo " Total Modules: 28 + Edge Cases"
echo "=========================================="

TESTS=(
    # Tier 1: Core Runtime & Essentials
    "tests/core/core_test.rey"
    "tests/mem/mem_test.rey"
    "tests/string/string_test.rey"
    "tests/fmt/fmt_test.rey"
    "tests/collections/collections_test.rey"
    "tests/io/io_test.rey"
    "tests/fs/fs_test.rey"
    "tests/process/process_test.rey"

    # Tier 2: Systems, OS & Networking
    "tests/net/net_test.rey"
    "tests/http/http_test.rey"
    "tests/time/time_test.rey"
    "tests/env/env_test.rey"
    "tests/sync/sync_test.rey"
    "tests/crypto/crypto_test.rey"
    "tests/os/os_test.rey"

    # Tier 3: Data Formats & Productivity
    "tests/json/json_test.rey"
    "tests/toml/toml_test.rey"
    "tests/csv/csv_test.rey"
    "tests/regex/regex_test.rey"
    "tests/cli/cli_test.rey"
    "tests/log/log_test.rey"
    "tests/test/test_test.rey"
    "tests/binary/binary_test.rey"

    # Tier 4: High-Performance Domains
    "tests/math/math_test.rey"
    "tests/linalg/linalg_test.rey"
    "tests/compress/compress_test.rey"
    "tests/image/image_test.rey"
    "tests/sql/sql_test.rey"

    # Comprehensive Edge Cases & Boundary Conditions
    "tests/edge_cases/edge_cases_test.rey"
)

PASSED=0
FAILED=0

for t in "${TESTS[@]}"; do
    echo "--- Running $t ---"
    if "${REYC_BIN}" run "${STD_DIR}/$t"; then
        echo ">>> OK: $t"
        PASSED=$((PASSED + 1))
    else
        echo ">>> FAILED: $t"
        FAILED=$((FAILED + 1))
        exit 1
    fi
    echo ""
done

echo "=========================================="
echo " ALL TESTS PASSED!"
echo " Total: ${PASSED} / ${#TESTS[@]} test suites passed, ${FAILED} failed."
echo "=========================================="
