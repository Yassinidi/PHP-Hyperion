<?php
declare(strict_types=1);

require_once __DIR__ . '/../src/Database.php';
require_once __DIR__ . '/../src/AuthService.php';

use AuthRbac\AuthService;

$uri = parse_url($_SERVER['REQUEST_URI'] ?? '/', PHP_URL_PATH);
$method = $_SERVER['REQUEST_METHOD'] ?? 'GET';

// 1. Health Probe
if ($uri === '/health') {
    header('Content-Type: application/json; charset=utf-8');
    echo json_encode([
        'status' => 'ok',
        'project' => 'auth-rbac',
        'timestamp' => microtime(true),
        'engine' => defined('HYPERION_PERSISTENT') ? 'hyperion-core-2.0' : 'standard-php'
    ]);
    return;
}

// 2. Web Security Dashboard
if ($uri === '/' && $method === 'GET') {
    $roles = AuthService::getRoles();
    $users = AuthService::getUsers();
    require __DIR__ . '/../src/Views/dashboard.php';
    return;
}

// -------------------------------------------------------------
// REST API
// -------------------------------------------------------------
header('Content-Type: application/json; charset=utf-8');

// 3. GET /api/rbac/roles
if ($uri === '/api/rbac/roles' && $method === 'GET') {
    echo json_encode([
        'success' => true,
        'data' => AuthService::getRoles()
    ]);
    return;
}

// 4. POST /api/auth/token (Mint JWT)
if ($uri === '/api/auth/token' && $method === 'POST') {
    $raw = file_get_contents('php://input');
    $payload = json_decode($raw, true);
    if (!is_array($payload)) {
        $payload = $_POST;
    }

    $username = (string)($payload['username'] ?? 'alice_admin');
    $password = (string)($payload['password'] ?? 'HyperionSec2026!');

    $result = AuthService::issueToken($username, $password);
    if ($result['success'] ?? false) {
        http_response_code(200);
    } else {
        http_response_code(401);
    }
    echo json_encode($result);
    return;
}

// 5. POST /api/auth/authorize (Validate & RBAC Check)
if ($uri === '/api/auth/authorize' && $method === 'POST') {
    $raw = file_get_contents('php://input');
    $payload = json_decode($raw, true);
    if (!is_array($payload)) {
        $payload = $_POST;
    }

    $token = (string)($payload['token'] ?? '');
    $required = (int)($payload['required_perm'] ?? AuthService::PERM_READ_PUBLIC);

    $result = AuthService::authorize($token, $required);
    if ($result['success'] ?? false) {
        http_response_code(200);
    } else {
        http_response_code(403);
    }
    echo json_encode($result);
    return;
}

// 404 Fallback
http_response_code(404);
echo json_encode(['success' => false, 'error' => 'Route not found: ' . $uri]);
