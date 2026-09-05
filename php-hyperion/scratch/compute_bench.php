<?php

// 1. Recursive Fibonacci (CPU recursion benchmark)
function fib($n) {
    if ($n <= 1) return $n;
    return fib($n - 1) + fib($n - 2);
}

$t0 = microtime(true);
$fib_res = fib(28);
$t_fib = (microtime(true) - $t0) * 1000;

// 2. Array allocation & functional pipeline (Memory & GC benchmark)
$t0 = microtime(true);
$arr = [];
for ($i = 0; $i < 20000; $i++) {
    $arr[] = $i;
}
$mapped = array_map(function($x) { return $x * 2 + 1; }, $arr);
$filtered = array_filter($mapped, function($x) { return $x % 3 == 0; });
$sum = array_sum($filtered);
$t_arr = (microtime(true) - $t0) * 1000;

// 3. String hashing & manipulation
$t0 = microtime(true);
$str = "Hyperion-High-Performance-PHP-Engine-2026";
$hashes = [];
for ($i = 0; $i < 5000; $i++) {
    $hashes[] = hash('sha256', $str . '-' . $i);
}
$t_str = (microtime(true) - $t0) * 1000;

// 4. JSON Encode / Decode Benchmark
$t0 = microtime(true);
$payload = [
    'status' => 'success',
    'items' => array_slice($hashes, 0, 500),
    'metadata' => ['engine' => 'hyperion', 'fib' => $fib_res, 'sum' => $sum]
];
for ($i = 0; $i < 500; $i++) {
    $encoded = json_encode($payload);
    $decoded = json_decode($encoded, true);
}
$t_json = (microtime(true) - $t0) * 1000;

$total_time = $t_fib + $t_arr + $t_str + $t_json;
$peak_mem = memory_get_peak_usage(true) / 1024 / 1024;

echo json_encode([
    'fib_time_ms' => round($t_fib, 2),
    'array_time_ms' => round($t_arr, 2),
    'string_hash_time_ms' => round($t_str, 2),
    'json_time_ms' => round($t_json, 2),
    'total_compute_time_ms' => round($total_time, 2),
    'peak_memory_mb' => round($peak_mem, 2),
    'fib_result' => $fib_res,
    'sum_result' => $sum,
], JSON_PRETTY_PRINT) . "\n";
