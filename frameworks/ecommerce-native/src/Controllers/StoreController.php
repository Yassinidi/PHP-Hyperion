<?php
declare(strict_types=1);

namespace EcommerceNative\Controllers;

use EcommerceNative\Database;
use PDO;

class StoreController {
    public static function getBanners(): array {
        $db = Database::getConnection();
        $stmt = $db->query("SELECT id, title, subtitle, image_url, link_url, position FROM banners WHERE is_active = 1 ORDER BY id ASC");
        return $stmt->fetchAll();
    }

    public static function getCategories(): array {
        $db = Database::getConnection();
        $stmt = $db->query("
            SELECT c.*, COUNT(p.id) as product_count 
            FROM categories c
            LEFT JOIN products p ON p.category_id = c.id AND p.is_active = 1
            WHERE c.is_active = 1
            GROUP BY c.id
            ORDER BY c.id ASC
        ");
        return $stmt->fetchAll();
    }

    public static function getProducts(?int $categoryId = null): array {
        $db = Database::getConnection();
        if ($categoryId !== null) {
            $stmt = $db->prepare("
                SELECT p.*, c.name as category_name 
                FROM products p
                JOIN categories c ON c.id = p.category_id
                WHERE p.is_active = 1 AND p.category_id = :cat_id
                ORDER BY p.id ASC
            ");
            $stmt->execute(['cat_id' => $categoryId]);
        } else {
            $stmt = $db->query("
                SELECT p.*, c.name as category_name 
                FROM products p
                JOIN categories c ON c.id = p.category_id
                WHERE p.is_active = 1
                ORDER BY p.id ASC
            ");
        }
        return $stmt->fetchAll();
    }

    public static function getProduct(int $id): ?array {
        $db = Database::getConnection();
        $stmt = $db->prepare("
            SELECT p.*, c.name as category_name 
            FROM products p
            JOIN categories c ON c.id = p.category_id
            WHERE p.id = :id AND p.is_active = 1
        ");
        $stmt->execute(['id' => $id]);
        $res = $stmt->fetch();
        return $res ?: null;
    }

    public static function checkout(array $data): array {
        $db = Database::getConnection();
        $userId = (int)($data['user_id'] ?? 2); // default client user
        $items = $data['items'] ?? [];

        if (empty($items)) {
            return ['success' => false, 'error' => 'Cart is empty'];
        }

        $db->beginTransaction();
        try {
            $totalAmount = 0.0;
            $validatedItems = [];

            foreach ($items as $item) {
                $pid = (int)($item['product_id'] ?? 0);
                $qty = max(1, (int)($item['quantity'] ?? 1));

                $stmt = $db->prepare("SELECT id, price, stock, title FROM products WHERE id = :id AND is_active = 1");
                $stmt->execute(['id' => $pid]);
                $prod = $stmt->fetch();

                if (!$prod) {
                    throw new \Exception("Product ID {$pid} not found");
                }
                if ($prod['stock'] < $qty) {
                    throw new \Exception("Insufficient stock for {$prod['title']}");
                }

                $subtotal = $prod['price'] * $qty;
                $totalAmount += $subtotal;
                $validatedItems[] = [
                    'product_id' => $pid,
                    'quantity' => $qty,
                    'unit_price' => $prod['price'],
                ];

                // Deduct stock
                $upd = $db->prepare("UPDATE products SET stock = stock - :qty WHERE id = :id");
                $upd->execute(['qty' => $qty, 'id' => $pid]);
            }

            // Insert Order
            $orderStmt = $db->prepare("INSERT INTO orders (user_id, total_amount, status) VALUES (:uid, :total, 'completed')");
            $orderStmt->execute(['uid' => $userId, 'total' => round($totalAmount, 2)]);
            $orderId = (int)$db->lastInsertId();

            // Insert Order Items
            $itemStmt = $db->prepare("INSERT INTO order_items (order_id, product_id, quantity, unit_price) VALUES (:oid, :pid, :qty, :price)");
            foreach ($validatedItems as $v) {
                $itemStmt->execute([
                    'oid' => $orderId,
                    'pid' => $v['product_id'],
                    'qty' => $v['quantity'],
                    'price' => $v['unit_price'],
                ]);
            }

            $db->commit();
            return [
                'success' => true,
                'order_id' => $orderId,
                'total_amount' => round($totalAmount, 2),
                'items_count' => count($validatedItems),
                'status' => 'completed',
            ];
        } catch (\Throwable $e) {
            $db->rollBack();
            return ['success' => false, 'error' => $e->getMessage()];
        }
    }
}
