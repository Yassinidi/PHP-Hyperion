<?php
declare(strict_types=1);

namespace EcommerceNative\Controllers;

use EcommerceNative\Database;
use EcommerceNative\Auth;
use PDO;

class AdminController {
    public static function getDashboard(): array {
        $db = Database::getConnection();

        $revenue = (float)$db->query("SELECT COALESCE(SUM(total_amount), 0) FROM orders WHERE status = 'completed'")->fetchColumn();
        $ordersCount = (int)$db->query("SELECT COUNT(*) FROM orders")->fetchColumn();
        $usersCount = (int)$db->query("SELECT COUNT(*) FROM users")->fetchColumn();
        $clientsCount = (int)$db->query("SELECT COUNT(*) FROM users WHERE role = 'client'")->fetchColumn();
        $productsCount = (int)$db->query("SELECT COUNT(*) FROM products WHERE is_active = 1")->fetchColumn();
        $categoriesCount = (int)$db->query("SELECT COUNT(*) FROM categories WHERE is_active = 1")->fetchColumn();
        $bannersCount = (int)$db->query("SELECT COUNT(*) FROM banners WHERE is_active = 1")->fetchColumn();

        $recentOrders = $db->query("
            SELECT o.id, o.total_amount, o.status, o.created_at, u.name as customer_name, u.email as customer_email
            FROM orders o
            JOIN users u ON u.id = o.user_id
            ORDER BY o.id DESC LIMIT 5
        ")->fetchAll();

        $lowStock = $db->query("
            SELECT id, title, sku, price, stock 
            FROM products 
            WHERE stock < 30 AND is_active = 1 
            ORDER BY stock ASC LIMIT 5
        ")->fetchAll();

        return [
            'metrics' => [
                'total_revenue' => round($revenue, 2),
                'total_orders' => $ordersCount,
                'total_users' => $usersCount,
                'total_clients' => $clientsCount,
                'total_products' => $productsCount,
                'total_categories' => $categoriesCount,
                'total_banners' => $bannersCount,
            ],
            'recent_orders' => $recentOrders,
            'low_stock_alerts' => $lowStock,
        ];
    }

    public static function getUsers(): array {
        return Auth::getAllUsers();
    }
}
