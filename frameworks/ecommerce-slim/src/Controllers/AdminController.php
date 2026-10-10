<?php
declare(strict_types=1);

namespace EcommerceSlim\Controllers;

use EcommerceSlim\Database;
use EcommerceSlim\Auth;

class AdminController {
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
