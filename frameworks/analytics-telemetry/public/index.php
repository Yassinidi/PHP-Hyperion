<?php
declare(strict_types=1);

require_once __DIR__ . '/../src/Database.php';
require_once __DIR__ . '/../src/TelemetryService.php';

use AnalyticsTelemetry\TelemetryService;

$uri = parse_url($_SERVER['REQUEST_URI'] ?? '/', PHP_URL_PATH);
$method = $_SERVER['REQUEST_METHOD'] ?? 'GET';

// 1. Health Probe
if ($uri === '/health') {
    header('Content-Type: application/json; charset=utf-8');
    echo json_encode([
        'status' => 'ok',
        'project' => 'analytics-telemetry',
        'timestamp' => microtime(true),
        'engine' => defined('HYPERION_PERSISTENT') ? 'hyperion-core-2.0' : 'standard-php'
    ]);
    return;
}

// 2. Web Telemetry Dashboard
if ($uri === '/' && $method === 'GET') {
    $summary = TelemetryService::getSummary(3600);
    $recentMetrics = TelemetryService::getRecentMetrics(20);
    $alerts = TelemetryService::evaluateAlerts();
    require __DIR__ . '/../src/Views/dashboard.php';
    return;
}

// -------------------------------------------------------------
// REST API
// -------------------------------------------------------------
header('Content-Type: application/json; charset=utf-8');

// 3. GET /api/metrics/summary
if ($uri === '/api/metrics/summary' && $method === 'GET') {
    $window = (int)($_GET['window'] ?? 3600);
    echo json_encode([
        'success' => true,
        'data' => TelemetryService::getSummary($window)
    ]);
    return;
}

// 4. GET /api/metrics/recent
if ($uri === '/api/metrics/recent' && $method === 'GET') {
    $limit = (int)($_GET['limit'] ?? 20);
    echo json_encode([
        'success' => true,
        'data' => TelemetryService::getRecentMetrics($limit)
    ]);
    return;
}

// 5. GET /api/alerts/evaluate
if ($uri === '/api/alerts/evaluate' && $method === 'GET') {
    echo json_encode([
        'success' => true,
        'data' => TelemetryService::evaluateAlerts()
    ]);
    return;
}

// 6. POST /api/telemetry/ingest
if ($uri === '/api/telemetry/ingest' && $method === 'POST') {
    $raw = file_get_contents('php://input');
    $payload = json_decode($raw, true);
    if (!is_array($payload)) {
        $payload = $_POST;
    }

    if (isset($payload['batch']) && is_array($payload['batch'])) {
        $res = TelemetryService::ingestBatch($payload['batch']);
    } elseif (isset($payload[0]) && is_array($payload[0])) {
        $res = TelemetryService::ingestBatch($payload);
    } else {
        $res = TelemetryService::ingest($payload);
    }

    http_response_code(201);
    echo json_encode($res);
    return;
}

// 404 Fallback
http_response_code(404);
echo json_encode(['success' => false, 'error' => 'Route not found: ' . $uri]);
