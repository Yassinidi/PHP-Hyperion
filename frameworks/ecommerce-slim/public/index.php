<?php
declare(strict_types=1);

require __DIR__ . '/../vendor/autoload.php';

use Slim\Factory\AppFactory;
use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use EcommerceSlim\Controllers\StoreController;
use EcommerceSlim\Controllers\AdminController;
use EcommerceSlim\Auth;

$app = AppFactory::create();
$app->addRoutingMiddleware();
$app->addBodyParsingMiddleware();
$app->addErrorMiddleware(false, false, false);

// 1. Storefront Home Page
$app->get('/', function (Request $request, Response $response): Response {
    $params = $request->getQueryParams();
    $catId = isset($params['category_id']) ? (int)$params['category_id'] : null;
    $banners = StoreController::getBanners();
    $categories = StoreController::getCategories();
    $products = StoreController::getProducts($catId);

    ob_start();
    require __DIR__ . '/../src/Views/home.php';
    $html = ob_get_clean();

    $response->getBody()->write($html);
    return $response->withHeader('Content-Type', 'text/html; charset=utf-8');
});

// 2. Admin Web Portal
$app->get('/admin', function (Request $request, Response $response): Response {
    $dashboard = AdminController::getDashboard();
    $users = AdminController::getUsers();

    ob_start();
    require __DIR__ . '/../src/Views/admin.php';
    $html = ob_get_clean();

    $response->getBody()->write($html);
    return $response->withHeader('Content-Type', 'text/html; charset=utf-8');
});

// 3. API: Banners
$app->get('/api/banners', function (Request $request, Response $response): Response {
    $payload = json_encode(['data' => StoreController::getBanners()]);
    $response->getBody()->write($payload);
    return $response->withHeader('Content-Type', 'application/json');
});

// 4. API: Categories
$app->get('/api/categories', function (Request $request, Response $response): Response {
    $payload = json_encode(['data' => StoreController::getCategories()]);
    $response->getBody()->write($payload);
    return $response->withHeader('Content-Type', 'application/json');
});

// 5. API: Products List
$app->get('/api/products', function (Request $request, Response $response): Response {
    $params = $request->getQueryParams();
    $catId = isset($params['category_id']) ? (int)$params['category_id'] : null;
    $payload = json_encode(['data' => StoreController::getProducts($catId)]);
    $response->getBody()->write($payload);
    return $response->withHeader('Content-Type', 'application/json');
});

// 6. API: Product Detail
$app->get('/api/products/{id:[0-9]+}', function (Request $request, Response $response, array $args): Response {
    $id = (int)$args['id'];
    $prod = StoreController::getProduct($id);
    if ($prod) {
        $response->getBody()->write(json_encode(['data' => $prod]));
        return $response->withHeader('Content-Type', 'application/json');
    }
    $response->getBody()->write(json_encode(['error' => 'Product not found']));
    return $response->withStatus(404)->withHeader('Content-Type', 'application/json');
});

// 7. API: Cart Checkout
$app->post('/api/cart/checkout', function (Request $request, Response $response): Response {
    $parsed = $request->getParsedBody();
    if (!is_array($parsed)) {
        $raw = (string)$request->getBody();
        $parsed = json_decode($raw, true) ?: [];
    }
    $result = StoreController::checkout($parsed);
    $status = !empty($result['success']) ? 201 : 400;
    $response->getBody()->write(json_encode($result));
    return $response->withStatus($status)->withHeader('Content-Type', 'application/json');
});

// 8. API: Admin Dashboard
$app->get('/api/admin/dashboard', function (Request $request, Response $response): Response {
    $payload = json_encode(['data' => AdminController::getDashboard()]);
    $response->getBody()->write($payload);
    return $response->withHeader('Content-Type', 'application/json');
});

// 9. API: Admin Users List
$app->get('/api/admin/users', function (Request $request, Response $response): Response {
    $payload = json_encode(['data' => AdminController::getUsers()]);
    $response->getBody()->write($payload);
    return $response->withHeader('Content-Type', 'application/json');
});

// 10. API: Auth Login
$app->post('/api/auth/login', function (Request $request, Response $response): Response {
    $parsed = $request->getParsedBody();
    if (!is_array($parsed)) {
        $raw = (string)$request->getBody();
        $parsed = json_decode($raw, true) ?: [];
    }
    $email = (string)($parsed['email'] ?? '');
    $pass = (string)($parsed['password'] ?? '');
    $user = Auth::attempt($email, $pass);
    if ($user) {
        $response->getBody()->write(json_encode(['success' => true, 'user' => $user]));
        return $response->withHeader('Content-Type', 'application/json');
    }
    $response->getBody()->write(json_encode(['success' => false, 'error' => 'Invalid email or password']));
    return $response->withStatus(401)->withHeader('Content-Type', 'application/json');
});

// 11. Health Check
$app->get('/health', function (Request $request, Response $response): Response {
    $payload = json_encode(['status' => 'ok', 'project' => 'ecommerce-slim', 'timestamp' => microtime(true)]);
    $response->getBody()->write($payload);
    return $response->withHeader('Content-Type', 'application/json');
});

// Run if directly executed outside persistent daemon
if (!defined('HYPERION_PERSISTENT')) {
    $app->run();
}

return $app;
