<?php
declare(strict_types=1);

require_once __DIR__ . '/../src/Database.php';
require_once __DIR__ . '/../src/Auth.php';
require_once __DIR__ . '/../src/Controllers/StoreController.php';
require_once __DIR__ . '/../src/Controllers/AdminController.php';

use EcommerceNative\Controllers\StoreController;
use EcommerceNative\Controllers\AdminController;
use EcommerceNative\Auth;

$uri = parse_url($_SERVER['REQUEST_URI'] ?? '/', PHP_URL_PATH);
$method = $_SERVER['REQUEST_METHOD'] ?? 'GET';

// 1. Storefront Web Page
if ($uri === '/' && $method === 'GET') {
    $catId = isset($_GET['category_id']) ? (int)$_GET['category_id'] : null;
    $banners = StoreController::getBanners();
    $categories = StoreController::getCategories();
    $products = StoreController::getProducts($catId);
    require __DIR__ . '/../src/Views/home.php';
    return;
}

// 2. Admin Web Page
if ($uri === '/admin' && $method === 'GET') {
    $dashboard = AdminController::getDashboard();
    $users = AdminController::getUsers();
    require __DIR__ . '/../src/Views/admin.php';
    return;
}

// JSON API Handlers
header('Content-Type: application/json; charset=utf-8');

// 3. API: Banners
if ($uri === '/api/banners' && $method === 'GET') {
    echo json_encode(['data' => StoreController::getBanners()]);
    return;
}

// 4. API: Categories
if ($uri === '/api/categories' && $method === 'GET') {
    echo json_encode(['data' => StoreController::getCategories()]);
    return;
}

// 5. API: Products List
if ($uri === '/api/products' && $method === 'GET') {
    $catId = isset($_GET['category_id']) ? (int)$_GET['category_id'] : null;
    echo json_encode(['data' => StoreController::getProducts($catId)]);
    return;
}

// 6. API: Single Product Detail
if (preg_match('#^/api/products/(\d+)$#', $uri, $matches) && $method === 'GET') {
    $prod = StoreController::getProduct((int)$matches[1]);
    if ($prod) {
        echo json_encode(['data' => $prod]);
    } else {
        http_response_code(404);
        echo json_encode(['error' => 'Product not found']);
    }
    return;
}

// 7. API: Cart Checkout
if ($uri === '/api/cart/checkout' && $method === 'POST') {
    $raw = $GLOBALS['HTTP_RAW_POST_DATA'] ?? $_SERVER['RAW_BODY'] ?? file_get_contents('php://input');
    $data = (!empty($raw) ? json_decode($raw, true) : null) ?: $_POST;
    $result = StoreController::checkout($data ?: []);
    if (!empty($result['success'])) {
        http_response_code(201);
        echo json_encode($result);
    } else {
        http_response_code(400);
        echo json_encode($result);
    }
    return;
}

// 8. API: Admin Dashboard Metrics
if ($uri === '/api/admin/dashboard' && $method === 'GET') {
    echo json_encode(['data' => AdminController::getDashboard()]);
    return;
}

// 9. API: Admin Users List
if ($uri === '/api/admin/users' && $method === 'GET') {
    echo json_encode(['data' => AdminController::getUsers()]);
    return;
}

// 10. API: Auth Login
if ($uri === '/api/auth/login' && $method === 'POST') {
    $raw = $GLOBALS['HTTP_RAW_POST_DATA'] ?? $_SERVER['RAW_BODY'] ?? file_get_contents('php://input');
    $data = (!empty($raw) ? json_decode($raw, true) : null) ?: $_POST;
    $email = (string)($data['email'] ?? '');
    $pass = (string)($data['password'] ?? '');
    $user = Auth::attempt($email, $pass);
    if ($user) {
        echo json_encode(['success' => true, 'user' => $user]);
    } else {
        http_response_code(401);
        echo json_encode(['success' => false, 'error' => 'Invalid email or password']);
    }
    return;
}

// 11. Health Check
if ($uri === '/health') {
    echo json_encode(['status' => 'ok', 'project' => 'ecommerce-native', 'timestamp' => microtime(true)]);
    return;
}

// 404 Fallback
http_response_code(404);
echo json_encode(['error' => 'Not Found', 'path' => $uri]);
