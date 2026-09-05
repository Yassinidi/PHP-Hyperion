<?php
declare(strict_types=1);

/**
 * ⚡ Dual-Engine Verification Battery for Complex Features:
 * Filesystem/Deletion, Laravel Reverb Broadcasting, Complex Queries & Modern Syntax.
 *
 * Runs seamlessly under both Zend PHP 8.4 and PHP-Hyperion.
 */

require_once __DIR__ . '/../vendor/autoload.php';

$app = require_once __DIR__ . '/../bootstrap/app.php';
$kernel = $app->make(\Illuminate\Contracts\Console\Kernel::class);
$kernel->bootstrap();

$passed = 0;
$total = 0;

function runBattery(string $label, callable $fn): void {
    global $passed, $total;
    $total++;
    echo "[TEST {$total}] {$label}... ";
    try {
        $result = $fn();
        echo "✅ PASSED\n";
        if (is_array($result)) {
            foreach ($result as $k => $v) {
                $valStr = is_bool($v) ? ($v ? 'true' : 'false') : (is_scalar($v) ? (string)$v : json_encode($v));
                echo "      └─ {$k}: {$valStr}\n";
            }
        }
        $passed++;
    } catch (\Throwable $e) {
        echo "❌ FAILED: " . $e->getMessage() . " in " . $e->getFile() . ":" . $e->getLine() . "\n";
        echo "Stack trace:\n" . $e->getTraceAsString() . "\n";
    }
}

echo "\n=================================================================\n";
echo "⚡ LARAVEL COMPLEX FEATURES BATTERY: PHP 8.4 & HYPERION ENGINE\n";
echo "=================================================================\n\n";

$fileService = $app->make(\App\Services\FileManagerService::class);
$queryService = $app->make(\App\Services\AdvancedQueryService::class);

// -----------------------------------------------------------------------------
// SECTION 1: File Storage, Hashing, Native Copy & Deletion Operations
// -----------------------------------------------------------------------------

runBattery("1. Single File Storage, Hash Check & Soft Delete", function () use ($fileService) {
    $content = "Payload for single file test: " . bin2hex(random_bytes(32));
    $att = $fileService->storeFile('local', 'tests/battery', 'sample.txt', $content);

    if (!\Illuminate\Support\Facades\Storage::disk('local')->exists($att->path)) {
        throw new \Exception("File was not written to storage");
    }

    $read = $fileService->readFile($att->uuid);
    if ($read !== $content) {
        throw new \Exception("Read content mismatch");
    }

    if ($att->sha256_hash !== hash('sha256', $content)) {
        throw new \Exception("SHA256 hash mismatch");
    }

    // Soft delete
    $del = $fileService->deleteFile($att->uuid, false);

    // Verify physical file still exists on disk
    if (!\Illuminate\Support\Facades\Storage::disk('local')->exists($att->path)) {
        throw new \Exception("Physical file was prematurely deleted on soft delete");
    }

    // Verify database soft deleted
    if (\App\Models\Attachment::where('uuid', $att->uuid)->exists()) {
        throw new \Exception("Attachment still returned in active query");
    }

    // Restore
    $restored = $fileService->restoreFile($att->uuid);

    // Force delete
    $forceDel = $fileService->deleteFile($att->uuid, true);

    // Verify physical file is now removed from disk!
    if (\Illuminate\Support\Facades\Storage::disk('local')->exists($att->path)) {
        throw new \Exception("Physical file was NOT removed on force delete");
    }

    return [
        'uuid' => $att->uuid,
        'size' => $att->size_bytes,
        'hash' => substr($att->sha256_hash, 0, 16) . '...',
        'restored' => $restored->deleted_at === null,
        'physically_cleaned' => $forceDel['physical_deleted'],
    ];
});

runBattery("2. Batch File Storage & Bulk Deletion", function () use ($fileService) {
    $uuids = [];
    for ($i = 1; $i <= 3; $i++) {
        $att = $fileService->storeFile('local', 'tests/batch', "batch_{$i}.txt", "Content {$i}");
        $uuids[] = $att->uuid;
    }

    $batchRes = $fileService->batchDeleteFiles($uuids, true);
    if ($batchRes['deleted_count'] !== 3) {
        throw new \Exception("Expected 3 files deleted in batch, got {$batchRes['deleted_count']}");
    }

    return ['deleted_files' => $batchRes['deleted_count']];
});

runBattery("3. Recursive Directory Creation & Deletion", function () use ($fileService) {
    $dir = 'tests/nested_dir_' . uniqid();
    \Illuminate\Support\Facades\Storage::disk('local')->put("{$dir}/sub1/file1.txt", "file 1");
    \Illuminate\Support\Facades\Storage::disk('local')->put("{$dir}/sub2/file2.txt", "file 2");

    if (!\Illuminate\Support\Facades\Storage::disk('local')->exists("{$dir}/sub1/file1.txt")) {
        throw new \Exception("Failed to write nested directory files");
    }

    $deleted = $fileService->deleteDirectory('local', $dir);
    $existsAfter = \Illuminate\Support\Facades\Storage::disk('local')->exists("{$dir}/sub1/file1.txt");

    if (!$deleted || $existsAfter) {
        throw new \Exception("deleteDirectory failed to remove folder recursively");
    }

    return ['directory' => $dir, 'recursive_cleaned' => true];
});

runBattery("4. Native copy() and unlink() Engine Verification", function () use ($fileService) {
    $payload = "Native engine filesystem parity check " . microtime(true);
    $res = $fileService->testTempAndNativeCopy($payload);

    if (!$res['copy_success'] || !$res['content_integrity'] || !$res['unlinked_source'] || !$res['unlinked_dest']) {
        throw new \Exception("Native copy or unlink failed: " . json_encode($res));
    }

    return $res;
});

// -----------------------------------------------------------------------------
// SECTION 2: Laravel Reverb WebSockets & Realtime Event Broadcasting
// -----------------------------------------------------------------------------

runBattery("5. Reverb Event Broadcasting & Serialization (RealtimeOrderStatusChanged)", function () {
    $order = \App\Models\Order::latest()->first();
    if (!$order) {
        throw new \Exception("No orders found");
    }

    $event = new \App\Events\RealtimeOrderStatusChanged(
        order: $order,
        previousStatus: 'pending',
        newStatus: 'shipped',
        reason: 'Carrier pickup confirmed'
    );

    // Broadcast event
    broadcast($event);

    $channels = $event->broadcastOn();
    $channelName = $channels[0]->name;
    $eventName = $event->broadcastAs();
    $payload = $event->broadcastWith();

    if ($channelName !== 'private-orders.' . $order->id) {
        throw new \Exception("Unexpected broadcast channel: {$channelName}");
    }

    if ($eventName !== 'order.status.updated') {
        throw new \Exception("Unexpected broadcastAs name: {$eventName}");
    }

    if ($payload['new_status'] !== 'shipped' || $payload['order_id'] !== $order->id) {
        throw new \Exception("Payload data mismatch");
    }

    return [
        'broadcaster' => config('broadcasting.default'),
        'channel' => $channelName,
        'event_name' => $eventName,
        'order_number' => $payload['order_number'],
    ];
});

runBattery("6. Broadcast Events (FileDeletedBroadcast & InventoryThresholdBreached)", function () {
    $uuid = (string) \Illuminate\Support\Str::uuid();
    $fileEvent = new \App\Events\FileDeletedBroadcast($uuid, 'invoice.pdf', true);
    broadcast($fileEvent);

    $product = \App\Models\Product::first();
    $invEvent = new \App\Events\InventoryThresholdBreached($product, 5, 15);
    broadcast($invEvent);

    return [
        'file_channel' => $fileEvent->broadcastOn()[0]->name,
        'file_event' => $fileEvent->broadcastAs(),
        'inv_channel' => $invEvent->broadcastOn()[0]->name,
        'inv_event' => $invEvent->broadcastAs(),
        'requires_reorder' => $invEvent->broadcastWith()['requires_reorder'],
    ];
});

// -----------------------------------------------------------------------------
// SECTION 3: Complex Queries, Window Functions, CTEs, Joins & Transactions
// -----------------------------------------------------------------------------

runBattery("7. Window Functions (ROW_NUMBER & AVG OVER PARTITION BY)", function () use ($queryService) {
    $rankings = $queryService->getCategoryProductRankings();

    if ($rankings->isEmpty()) {
        throw new \Exception("Window ranking returned empty dataset");
    }

    $first = $rankings->first();
    if (!isset($first->rank_in_category) || !isset($first->category_avg_price)) {
        throw new \Exception("Window calculation columns missing from result");
    }

    return [
        'total_ranked' => $rankings->count(),
        'top_product' => $first->product_name,
        'category' => $first->category_name,
        'rank' => $first->rank_in_category,
        'category_avg' => '$' . $first->category_avg_price,
    ];
});

runBattery("8. Subquery Joins (joinSub) with Group By Aggregations", function () use ($queryService) {
    $stats = $queryService->getProductPerformanceViaJoinSub();

    return [
        'products_with_sales' => $stats->count(),
        'top_performer' => $stats->first()->product_name ?? 'N/A',
        'units_sold' => $stats->first()->total_units_sold ?? 0,
        'revenue' => '$' . ($stats->first()->gross_revenue ?? 0),
    ];
});

runBattery("9. Correlated Subqueries via addSelect & Conditional Cohort Aggregates", function () use ($queryService) {
    $customers = $queryService->getCustomerMetricsCorrelated();
    $cohorts = $queryService->getCustomerCohortSpending();

    return [
        'customers_analyzed' => $customers->count(),
        'first_customer_orders' => $customers->first()->order_count ?? 0,
        'cohorts_count' => $cohorts->count(),
    ];
});

runBattery("10. Nested Transactions with Savepoint Isolation", function () use ($queryService) {
    $res = $queryService->testNestedTransactionWithSavepoint();

    if (!$res['isolation_passed']) {
        throw new \Exception("Savepoint transaction isolation failed: " . json_encode($res));
    }

    return $res;
});

runBattery("11. Soft Deletes Full Lifecycle (Create, Hide, Trash, Restore, Force)", function () use ($queryService) {
    $res = $queryService->testSoftDeleteLifecycle();

    if (!$res['lifecycle_passed']) {
        throw new \Exception("Soft delete lifecycle failed: " . json_encode($res));
    }

    return $res;
});

// -----------------------------------------------------------------------------
// SECTION 4: Modern PHP 8.4 Syntax
// -----------------------------------------------------------------------------

runBattery("12. Modern PHP 8.4 Syntax (Match Guards, Callables, Nullsafe, Destructuring)", function () use ($queryService) {
    $customer = \App\Models\Customer::first();
    $res = $queryService->testModernPhpSyntax(3450.00, $customer);

    if ($res['tier'] !== 'Business Premium' || empty($res['formatted']) || empty($res['customer_name'])) {
        throw new \Exception("Syntax evaluation mismatch: " . json_encode($res));
    }

    return $res;
});

$totalCount = $GLOBALS['total'] ?? $total;
$passedCount = $GLOBALS['passed'] ?? $passed;

echo "\n=================================================================\n";
echo "RESULTS: {$passedCount}/{$totalCount} BATTERY TESTS PASSED\n";
echo "=================================================================\n\n";

if ($passedCount !== $totalCount || $totalCount === 0) {
    exit(1);
}

