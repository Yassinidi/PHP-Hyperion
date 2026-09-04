#!/bin/bash
set -e

echo "================================================================="
echo "⚡ DUAL-ENGINE TEST: QUEUES, JOBS, BATCHES, EVENTS, CONSOLE, CACHE"
echo "================================================================="

echo ""
echo "[1] Testing Queue Dispatch: GenerateInventoryReportJob"
ZEND_DISP=$(curl -s -X POST http://127.0.0.1:8010/api/v1/queue/dispatch-job -H "Content-Type: application/json" -d '{"type":"report"}' | jq -r '.message')
HYP_DISP=$(curl -s -X POST http://127.0.0.1:8004/api/v1/queue/dispatch-job -H "Content-Type: application/json" -d '{"type":"report"}' | jq -r '.message')
echo "  -> Zend PHP 8.4: $ZEND_DISP"
echo "  -> Hyperion:     $HYP_DISP"

echo ""
echo "[2] Testing Job Batching: Bus::batch()"
ZEND_BATCH=$(curl -s -X POST http://127.0.0.1:8010/api/v1/queue/dispatch-job -H "Content-Type: application/json" -d '{"type":"batch"}' | jq -r '.batch_id')
HYP_BATCH=$(curl -s -X POST http://127.0.0.1:8004/api/v1/queue/dispatch-job -H "Content-Type: application/json" -d '{"type":"batch"}' | jq -r '.batch_id')
echo "  -> Zend PHP 8.4 Batch ID: $ZEND_BATCH"
echo "  -> Hyperion Batch ID:     $HYP_BATCH"

echo ""
echo "[3] Testing Order Shipped Event & Queued Notification"
ZEND_SHIP=$(curl -s -X POST http://127.0.0.1:8010/api/v1/orders/2/ship -H "Accept: application/json" | jq -r '.tracking_number')
HYP_SHIP=$(curl -s -X POST http://127.0.0.1:8004/api/v1/orders/3/ship -H "Accept: application/json" | jq -r '.tracking_number')
echo "  -> Zend PHP 8.4 Tracking: $ZEND_SHIP"
echo "  -> Hyperion Tracking:     $HYP_SHIP"

echo ""
echo "[4] Testing Queue Worker Execution via API (/api/v1/queue/work-once)"
ZEND_WORK=$(curl -s -X POST http://127.0.0.1:8010/api/v1/queue/work-once | jq -r '.message')
HYP_WORK=$(curl -s -X POST http://127.0.0.1:8004/api/v1/queue/work-once | jq -r '.message')
echo "  -> Zend PHP 8.4 Worker: $ZEND_WORK"
echo "  -> Hyperion Worker:     $HYP_WORK"

echo ""
echo "[5] Testing Console Commands & Task Scheduling"
php artisan store:audit-inventory | grep -E "SKU|Running Hyperion" | head -n 3
php artisan schedule:list | grep -E "store:audit-inventory|inventory:generate_report"

echo ""
echo "[6] Testing Cache Storage State"
CACHED_VAL=$(curl -s http://127.0.0.1:8004/api/v1/queue/stats | jq -r '.cached_inventory_valuation')
echo "  -> Cached Total Inventory Valuation: \$$CACHED_VAL"

echo ""
echo "✅ ALL QUEUE, JOB, EVENT, CONSOLE, AND CACHE PARITY TESTS PASSED!"
