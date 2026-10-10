<?php
declare(strict_types=1);

$uri = parse_url($_SERVER['REQUEST_URI'] ?? '/', PHP_URL_PATH);
$method = $_SERVER['REQUEST_METHOD'] ?? 'GET';

// 1. Health Check
if ($uri === '/health') {
    wp_send_json(['status' => 'ok', 'project' => 'ecommerce-wordpress', 'timestamp' => microtime(true)]);
    return;
}

// 2. API: Banners
if ($uri === '/api/banners' && $method === 'GET') {
    wp_send_json(['data' => wp_get_banners()]);
    return;
}

// 3. API: Categories
if ($uri === '/api/categories' && $method === 'GET') {
    wp_send_json(['data' => wp_get_categories()]);
    return;
}

// 4. API: Products List
if ($uri === '/api/products' && $method === 'GET') {
    $catId = isset($_GET['category_id']) ? (int)$_GET['category_id'] : null;
    wp_send_json(['data' => wp_get_products($catId)]);
    return;
}

// 5. API: Product Detail
if (preg_match('#^/api/products/(\d+)$#', $uri, $matches) && $method === 'GET') {
    $prod = wp_get_product((int)$matches[1]);
    if ($prod) {
        wp_send_json(['data' => $prod]);
    } else {
        http_response_code(404);
        wp_send_json(['error' => 'Product not found']);
    }
    return;
}

// 6. API: Cart Checkout
if ($uri === '/api/cart/checkout' && $method === 'POST') {
    $raw = $GLOBALS['HTTP_RAW_POST_DATA'] ?? $_SERVER['RAW_BODY'] ?? file_get_contents('php://input');
    $data = (!empty($raw) ? json_decode($raw, true) : null) ?: $_POST;
    $result = wc_checkout($data ?: []);
    if (!empty($result['success'])) {
        http_response_code(201);
        wp_send_json($result);
    } else {
        http_response_code(400);
        wp_send_json($result);
    }
    return;
}

// 7. API: Admin Dashboard
if ($uri === '/api/admin/dashboard' && $method === 'GET') {
    wp_send_json(['data' => wp_get_admin_dashboard()]);
    return;
}

// 8. API: Admin Users
if ($uri === '/api/admin/users' && $method === 'GET') {
    wp_send_json(['data' => wp_get_users()]);
    return;
}

// 9. API: Auth Login
if ($uri === '/api/auth/login' && $method === 'POST') {
    $raw = $GLOBALS['HTTP_RAW_POST_DATA'] ?? $_SERVER['RAW_BODY'] ?? file_get_contents('php://input');
    $data = (!empty($raw) ? json_decode($raw, true) : null) ?: $_POST;
    $email = (string)($data['email'] ?? '');
    $pass = (string)($data['password'] ?? '');
    $user = wp_authenticate_user($email, $pass);
    if ($user) {
        wp_send_json(['success' => true, 'user' => $user]);
    } else {
        http_response_code(401);
        wp_send_json(['success' => false, 'error' => 'Invalid email or password']);
    }
    return;
}

// 10. Storefront Web Page
if ($uri === '/' && $method === 'GET') {
    $catId = isset($_GET['category_id']) ? (int)$_GET['category_id'] : null;
    $banners = wp_get_banners();
    $categories = wp_get_categories();
    $products = wp_get_products($catId);
    require ABSPATH . 'wp-content/themes/ecommerce-theme/index.php';
    return;
}

// 11. Admin Web Portal
if ($uri === '/admin' && $method === 'GET') {
    $dashboard = wp_get_admin_dashboard();
    $users = wp_get_users();
    require ABSPATH . 'wp-content/themes/ecommerce-theme/admin.php';
    return;
}

// 404 Fallback
http_response_code(404);
header('Content-Type: application/json; charset=utf-8');
echo json_encode(['error' => 'Not Found', 'path' => $uri]);
