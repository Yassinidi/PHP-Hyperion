#!/bin/bash
set -e

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
HYPERION_BIN="${DIR}/../../php-hyperion/target/release/hyperion-cli"
PHP_BIN="php"

echo "========================================================================"
echo "⚡ DUAL-ENGINE VERIFICATION: COMPLEX FEATURES, REVERB & ADVANCED QUERIES"
echo "========================================================================"
echo "Workload: frameworks/laravel-complex"
echo "Engines:  Standard Zend PHP 8.4 vs Next-Gen PHP-Hyperion"
echo "========================================================================"
echo ""

echo "[1/3] Running Dual-Engine Verification Battery with Standard PHP 8.4..."
echo "------------------------------------------------------------------------"
$PHP_BIN "${DIR}/tests/verify_complex_engine.php"
echo ">>> PHP 8.4 verification completed successfully! ✅"
echo ""

echo "[2/3] Running Dual-Engine Verification Battery with PHP-Hyperion..."
echo "------------------------------------------------------------------------"
$HYPERION_BIN "${DIR}/tests/verify_complex_engine.php"
echo ">>> PHP-Hyperion verification completed successfully! ✅"
echo ""

echo "[3/3] Testing Laravel Artisan Commands on Hyperion..."
echo "------------------------------------------------------------------------"
echo "  -> Running 'artisan store:audit-inventory' (Symfony Console Table + Models)..."
$HYPERION_BIN "${DIR}/artisan" store:audit-inventory
echo ""
echo "  -> Running 'artisan route:list' (Routing Subsystem + Middlewares)..."
$HYPERION_BIN "${DIR}/artisan" route:list | grep -E "files|broadcasting|queries" | head -n 8
echo ""

echo "========================================================================"
echo "🎉 ALL ADVANCED LARAVEL 12 FEATURES & SYNTAX VERIFICATIONS PASSED 100%!"
echo "========================================================================"
