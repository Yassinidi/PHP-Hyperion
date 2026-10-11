<?php
declare(strict_types=1);

require_once __DIR__ . '/../src/Database.php';
require_once __DIR__ . '/../src/BankService.php';

use BankNative\BankService;

$uri = parse_url($_SERVER['REQUEST_URI'] ?? '/', PHP_URL_PATH);
$method = $_SERVER['REQUEST_METHOD'] ?? 'GET';

// 1. Health Probe
if ($uri === '/health') {
    header('Content-Type: application/json; charset=utf-8');
    echo json_encode([
        'status' => 'ok',
        'project' => 'bank-native',
        'timestamp' => microtime(true),
        'engine' => defined('HYPERION_PERSISTENT') ? 'hyperion-core-2.0' : 'standard-php'
    ]);
    return;
}

// 2. Web Banking Dashboard
if ($uri === '/' && $method === 'GET') {
    $accounts = BankService::getAccounts();
    $transfers = BankService::getTransfers(20);
    $stats = BankService::getStats();
    require __DIR__ . '/../src/Views/dashboard.php';
    return;
}

// -------------------------------------------------------------
// JSON REST API
// -------------------------------------------------------------
header('Content-Type: application/json; charset=utf-8');

// 3. GET /api/accounts
if ($uri === '/api/accounts' && $method === 'GET') {
    echo json_encode([
        'success' => true,
        'data' => BankService::getAccounts()
    ]);
    return;
}

// 4. GET /api/accounts/{id}
if (preg_match('#^/api/accounts/(\d+)$#', $uri, $matches) && $method === 'GET') {
    $acc = BankService::getAccount((int)$matches[1]);
    if ($acc) {
        echo json_encode(['success' => true, 'data' => $acc]);
    } else {
        http_response_code(404);
        echo json_encode(['success' => false, 'error' => 'Account not found']);
    }
    return;
}

// 5. POST /api/transfer & POST /api/transfer/batch (ACID Money Transfer)
if (($uri === '/api/transfer' || $uri === '/api/transfer/batch') && $method === 'POST') {
    $rawInput = file_get_contents('php://input');
    $payload = json_decode($rawInput, true);
    if (!is_array($payload)) {
        $payload = $_POST;
    }

    try {
        if ($uri === '/api/transfer/batch' || isset($payload['transfers']) || (isset($payload[0]) && is_array($payload[0]))) {
            $transfers = $payload['transfers'] ?? $payload;
            $result = BankService::transferBatch((array)$transfers);
            http_response_code(201);
            echo json_encode($result);
        } else {
            $fromId = (int)($payload['from_account_id'] ?? 0);
            $toId = (int)($payload['to_account_id'] ?? 0);
            $amount = (float)($payload['amount'] ?? 0.0);
            $note = (string)($payload['note'] ?? 'Transfer');

            $result = BankService::transfer($fromId, $toId, $amount, $note);
            http_response_code(201);
            echo json_encode($result);
        }
    } catch (\Throwable $e) {
        http_response_code(400);
        echo json_encode([
            'success' => false,
            'error' => $e->getMessage()
        ]);
    }
    return;
}

// 6. GET /api/transfers
if ($uri === '/api/transfers' && $method === 'GET') {
    $limit = isset($_GET['limit']) ? max(1, min(100, (int)$_GET['limit'])) : 50;
    echo json_encode([
        'success' => true,
        'data' => BankService::getTransfers($limit)
    ]);
    return;
}

// 7. GET /api/stats
if ($uri === '/api/stats' && $method === 'GET') {
    echo json_encode([
        'success' => true,
        'data' => BankService::getStats()
    ]);
    return;
}

// 404 Route Fallback
http_response_code(404);
echo json_encode(['success' => false, 'error' => 'Endpoint not found', 'path' => $uri]);
