<?php
declare(strict_types=1);

/**
 * PHP-Hyperion Unit Test Suite Runner
 * Executes all primitive micro-benchmarks and functional tests against PHP CLI and Hyperion CLI.
 */

$testDir = __DIR__;
$files = glob($testDir . '/test_*.php');
sort($files);

$phpCli = PHP_BINARY;
$hyperionCli = realpath(__DIR__ . '/../../php-hyperion/target/release/hyperion-cli');

echo "====================================================================\n";
echo " 🧪 PHP-HYPERION PRIMITIVE & MICRO-UNIT TEST BATTERY\n";
echo "====================================================================\n";
echo "PHP CLI      : " . $phpCli . "\n";
echo "Hyperion CLI : " . ($hyperionCli ?: "Not built yet (optional)") . "\n";
echo "Found Tests  : " . count($files) . " test files\n";
echo "====================================================================\n\n";

$passedCount = 0;
$failedCount = 0;

printf("%-28s | %-12s | %-14s | %s\n", "Test File", "PHP CLI", "Hyperion CLI", "Status");
echo str_repeat("-", 68) . "\n";

foreach ($files as $file) {
    $basename = basename($file);
    if ($basename === 'run_unit_tests.php') {
        continue;
    }

    // 1. Run with standard PHP
    $t0 = microtime(true);
    $outputPhp = [];
    $retPhp = 0;
    exec(escapeshellcmd($phpCli) . ' ' . escapeshellarg($file) . ' 2>&1', $outputPhp, $retPhp);
    $durPhp = (microtime(true) - $t0) * 1000;

    // 2. Run with Hyperion CLI if available
    $durHypStr = "N/A";
    $retHyp = 0;
    if ($hyperionCli && file_exists($hyperionCli)) {
        $t0 = microtime(true);
        $outputHyp = [];
        exec(escapeshellcmd($hyperionCli) . ' ' . escapeshellarg($file) . ' 2>&1', $outputHyp, $retHyp);
        $durHyp = (microtime(true) - $t0) * 1000;
        $durHypStr = sprintf("%.1f ms", $durHyp);
    }

    $isPassed = ($retPhp === 0 && $retHyp === 0);
    $statusStr = $isPassed ? "\033[32mPASS ✓\033[0m" : "\033[31mFAIL ✗\033[0m";

    if ($isPassed) {
        $passedCount++;
    } else {
        $failedCount++;
    }

    printf(
        "%-28s | %-12s | %-14s | %s\n",
        $basename,
        sprintf("%.1f ms", $durPhp),
        $durHypStr,
        $statusStr
    );
}

echo str_repeat("=", 68) . "\n";
printf(" Summary: %d Passed, %d Failed (Total: %d)\n", $passedCount, $failedCount, $passedCount + $failedCount);
echo str_repeat("=", 68) . "\n\n";

exit($failedCount > 0 ? 1 : 0);
