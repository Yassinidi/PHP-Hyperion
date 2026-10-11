<?php
declare(strict_types=1);

require_once __DIR__ . '/../src/Database.php';
require_once __DIR__ . '/../src/QueueService.php';

use QueueWorkflow\QueueService;

$uri = parse_url($_SERVER['REQUEST_URI'] ?? '/', PHP_URL_PATH);
$method = $_SERVER['REQUEST_METHOD'] ?? 'GET';

// 1. Health Probe
if ($uri === '/health') {
    header('Content-Type: application/json; charset=utf-8');
    echo json_encode([
        'status' => 'ok',
        'project' => 'queue-workflow',
        'timestamp' => microtime(true),
        'engine' => defined('HYPERION_PERSISTENT') ? 'hyperion-core-2.0' : 'standard-php'
    ]);
    return;
}

// 2. Web Queue Dashboard
if ($uri === '/' && $method === 'GET') {
    $stats = QueueService::getStats();
    $recentJobs = QueueService::getRecentJobs(20);
    require __DIR__ . '/../src/Views/dashboard.php';
    return;
}

// -------------------------------------------------------------
// REST API
// -------------------------------------------------------------
header('Content-Type: application/json; charset=utf-8');

// 3. GET /api/queue/stats
if ($uri === '/api/queue/stats' && $method === 'GET') {
    echo json_encode([
        'success' => true,
        'data' => QueueService::getStats()
    ]);
    return;
}

// 4. POST /api/queue/push & POST /api/queue/push/batch
if (($uri === '/api/queue/push' || $uri === '/api/queue/push/batch') && $method === 'POST') {
    $raw = file_get_contents('php://input');
    $payload = json_decode($raw, true);
    if (!is_array($payload)) {
        $payload = $_POST;
    }

    $queue = (string)($payload['queue'] ?? 'default');
    $priority = (int)($payload['priority'] ?? 10);
    $delay = (int)($payload['delay'] ?? 0);

    if (isset($payload['tasks']) && is_array($payload['tasks'])) {
        $res = QueueService::pushBatch($queue, $payload['tasks'], $priority, $delay);
    } else {
        $data = (array)($payload['data'] ?? $payload);
        $res = QueueService::push($queue, $data, $priority, $delay);
    }

    http_response_code(201);
    echo json_encode($res);
    return;
}

// 5. POST /api/queue/claim
if ($uri === '/api/queue/claim' && $method === 'POST') {
    $raw = file_get_contents('php://input');
    $payload = json_decode($raw, true);
    if (!is_array($payload)) {
        $payload = $_POST;
    }

    $queue = (string)($payload['queue'] ?? 'default');
    $workerId = (string)($payload['worker_id'] ?? ('worker_' . getmypid()));
    $timeout = (int)($payload['lease_timeout'] ?? 30);

    $job = QueueService::claim($queue, $workerId, $timeout);
    if ($job) {
        http_response_code(200);
        echo json_encode(['success' => true, 'job' => $job]);
    } else {
        http_response_code(204); // No content (empty queue)
    }
    return;
}

// 6. POST /api/queue/complete
if ($uri === '/api/queue/complete' && $method === 'POST') {
    $raw = file_get_contents('php://input');
    $payload = json_decode($raw, true);
    if (!is_array($payload)) {
        $payload = $_POST;
    }

    $jobId = (int)($payload['job_id'] ?? 0);
    $result = (array)($payload['result'] ?? ['status' => 'done']);

    $res = QueueService::complete($jobId, $result);
    echo json_encode($res);
    return;
}

// 404 Fallback
http_response_code(404);
echo json_encode(['success' => false, 'error' => 'Route not found: ' . $uri]);
