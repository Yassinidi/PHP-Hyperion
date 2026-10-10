<?php
declare(strict_types=1);

namespace App\Controller;

use App\Database;
use App\Auth;
use Symfony\Component\HttpFoundation\Request;
use Symfony\Component\HttpFoundation\Response;
use Symfony\Component\HttpFoundation\JsonResponse;

class AdminController {
    public function admin(): Response {
        $dashboard = self::getDashboard();
        $users = self::getUsers();

        ob_start();
        require __DIR__ . '/../Views/admin.php';
        $html = ob_get_clean();

        return new Response($html, 200, ['Content-Type' => 'text/html; charset=utf-8']);
    }

    public function dashboard(): JsonResponse {
        return new JsonResponse(['data' => self::getDashboard()]);
    }

    public function users(): JsonResponse {
        return new JsonResponse(['data' => self::getUsers()]);
    }

    public function login(Request $request): JsonResponse {
        $data = json_decode($request->getContent(), true) ?: $request->request->all();
        $email = (string)($data['email'] ?? '');
        $pass = (string)($data['password'] ?? '');
        $user = Auth::attempt($email, $pass);
        if ($user) {
            return new JsonResponse(['success' => true, 'user' => $user]);
        }
        return new JsonResponse(['success' => false, 'error' => 'Invalid email or password'], 401);
    }

    public static function getDashboard(): array {
        $pdo = Database::get();

        $revenue = (float)$pdo->query("SELECT COALESCE(SUM(total), 0) FROM orders WHERE status = 'completed'")->fetchColumn();
        $totalOrders = (int)$pdo->query("SELECT COUNT(*) FROM orders")->fetchColumn();
        $totalProducts = (int)$pdo->query("SELECT COUNT(*) FROM products")->fetchColumn();
        $totalClients = (int)$pdo->query("SELECT COUNT(*) FROM users WHERE role = 'client'")->fetchColumn();
        $lowStock = $pdo->query("SELECT id, name, stock FROM products WHERE stock < 20 ORDER BY stock ASC")->fetchAll();
        $recentOrders = $pdo->query("
            SELECT o.id, o.total, o.status, o.created_at, u.name as client_name, u.email as client_email
            FROM orders o
            JOIN users u ON o.user_id = u.id
            ORDER BY o.id DESC
            LIMIT 5
        ")->fetchAll();

        return [
            'metrics' => [
                'total_revenue'   => round($revenue, 2),
                'total_orders'    => $totalOrders,
                'total_products'  => $totalProducts,
                'total_clients'   => $totalClients,
                'low_stock_count' => count($lowStock)
            ],
            'low_stock_items' => $lowStock,
            'recent_orders'   => $recentOrders
        ];
    }

    public static function getUsers(): array {
        return Auth::allUsers();
    }
}
