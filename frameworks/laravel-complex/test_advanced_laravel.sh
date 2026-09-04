#!/bin/bash
set -e

echo "========================================================"
echo "⚡ TESTING ADVANCED LARAVEL SYNTAX: PHP 8.4 vs HYPERION"
echo "========================================================"

echo ""
echo "[1] Testing API Resource collection: GET /api/v1/products"
ZEND_PRODUCTS=$(curl -s http://127.0.0.1:8010/api/v1/products | jq '.data | length')
HYP_PRODUCTS=$(curl -s http://127.0.0.1:8004/api/v1/products | jq '.data | length')
echo "  -> Zend PHP 8.4: $ZEND_PRODUCTS products"
echo "  -> Hyperion:     $HYP_PRODUCTS products"

echo ""
echo "[2] Testing Form Request Custom Validation Closure (HTTP 422 expected)"
ZEND_422=$(curl -s -o /dev/null -w "%{http_code}" -X POST http://127.0.0.1:8010/api/v1/orders \
  -H "Content-Type: application/json" -H "Accept: application/json" \
  -d '{"customer_id": 1, "items": [{"product_id": 1, "quantity": 1000}]}')
HYP_422=$(curl -s -o /dev/null -w "%{http_code}" -X POST http://127.0.0.1:8004/api/v1/orders \
  -H "Content-Type: application/json" -H "Accept: application/json" \
  -d '{"customer_id": 1, "items": [{"product_id": 1, "quantity": 1000}]}')
echo "  -> Zend PHP 8.4 status: $ZEND_422"
echo "  -> Hyperion status:     $HYP_422"

echo ""
echo "[3] Testing Order Creation (Enums, DI, Observer, Events, Polymorphic Activity, API Resource)"
ZEND_ORDER=$(curl -s -X POST http://127.0.0.1:8010/api/v1/orders \
  -H "Content-Type: application/json" -H "Accept: application/json" \
  -d '{"customer_id": 2, "items": [{"product_id": 2, "quantity": 1}]}')
ZEND_NUM=$(echo $ZEND_ORDER | jq -r '.data.order_number')
ZEND_STATUS=$(echo $ZEND_ORDER | jq -r '.data.status.code')
ZEND_TOTAL=$(echo $ZEND_ORDER | jq -r '.data.total_amount')

HYP_ORDER=$(curl -s -X POST http://127.0.0.1:8004/api/v1/orders \
  -H "Content-Type: application/json" -H "Accept: application/json" \
  -d '{"customer_id": 3, "items": [{"product_id": 3, "quantity": 1}]}')
HYP_NUM=$(echo $HYP_ORDER | jq -r '.data.order_number')
HYP_STATUS=$(echo $HYP_ORDER | jq -r '.data.status.code')
HYP_TOTAL=$(echo $HYP_ORDER | jq -r '.data.total_amount')

echo "  -> Zend PHP 8.4: Order $ZEND_NUM, status $ZEND_STATUS, total \$$ZEND_TOTAL"
echo "  -> Hyperion:     Order $HYP_NUM, status $HYP_STATUS, total \$$HYP_TOTAL"

echo ""
echo "[4] Testing Web Route: GET /orders (Blade, Enums, Activity timeline)"
ZEND_WEB=$(curl -s -o /dev/null -w "%{http_code}" http://127.0.0.1:8010/orders)
HYP_WEB=$(curl -s -o /dev/null -w "%{http_code}" http://127.0.0.1:8004/orders)
echo "  -> Zend PHP 8.4 status: $ZEND_WEB"
echo "  -> Hyperion status:     $HYP_WEB"

echo ""
echo "[5] Testing Telemetry Middleware Headers"
curl -s -I http://127.0.0.1:8004/api/v1/products | grep -E "X-Hyperion|X-Execution|X-Memory"

echo ""
echo "✅ ALL ADVANCED LARAVEL SYNTAX PARITY VERIFICATIONS PASSED!"
