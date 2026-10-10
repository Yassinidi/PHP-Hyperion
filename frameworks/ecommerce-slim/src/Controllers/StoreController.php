<?php
declare(strict_types=1);

namespace EcommerceSlim\Controllers;

use EcommerceSlim\Database;
use PDO;

class StoreController {
    public static function getBanners(): array {
        $pdo = Database::get();
        return $pdo->query('SELECT * FROM banners WHERE is_active = 1 ORDER BY id ASC')->fetchAll();
    }

    public static function getCategories(): array {
        $pdo = Database::get();
        return $pdo->query('SELECT * FROM categories ORDER BY display_order ASC')->fetchAll();
    }

    public static function getProducts(?int $categoryId = null): array {
        $pdo = Database::get();
        if ($categoryId) {
            $stmt = $pdo->prepare('SELECT p.*, c.name as category_name FROM products p JOIN categories c ON p.category_id = c.id WHERE p.category_id = :cid ORDER BY p.id ASC');
            $stmt->execute([':cid' => $categoryId]);
            return $stmt->fetchAll();
        }
        return $pdo->query('SELECT p.*, c.name as category_name FROM products p JOIN categories c ON p.category_id = c.id ORDER BY p.id ASC')->fetchAll();
    }

    public static function getProduct(int $id): ?array {
        $pdo = Database::get();
        $stmt = $pdo->prepare('SELECT p.*, c.name as category_name FROM products p JOIN categories c ON p.category_id = c.id WHERE p.id = :id LIMIT 1');
        $stmt->execute([':id' => $id]);
        $prod = $stmt->fetch();
        return $prod ?: null;
    }

    public static function checkout(array $data): array {
        $userId = (int)($data['user_id'] ?? 2);
        $items = $data['items'] ?? [];
        if (empty($items)) {
            return ['success' => false, 'error' => 'Cart is empty'];
        }

        $pdo = Database::get();
        $pdo->beginTransaction();
        try {
            $total = 0.0;
            $validatedItems = [];
            foreach ($items as $item) {
                $pid = (int)($item['product_id'] ?? 0);
                $qty = max(1, (int)($item['quantity'] ?? 1));
                $stmt = $pdo->prepare('SELECT price, stock FROM products WHERE id = :id');
                $stmt->execute([':id' => $pid]);
                $p = $stmt->fetch();
                if (!$p) {
                    throw new \Exception("Product ID {$pid} not found");
                }
                $price = (float)$p['price'];
                $total += $price * $qty;
                $validatedItems[] = ['product_id' => $pid, 'quantity' => $qty, 'unit_price' => $price];

                // Deduct stock
                $upd = $pdo->prepare('UPDATE products SET stock = MAX(0, stock - :qty) WHERE id = :id');
                $upd->execute([':qty' => $qty, ':id' => $pid]);
            }

            $orderStmt = $pdo->prepare('INSERT INTO orders (user_id, total, status) VALUES (:uid, :total, :status)');
            $orderStmt->execute([':uid' => $userId, ':total' => $total, ':status' => 'completed']);
            $orderId = (int)$pdo->lastInsertId();

            $itemStmt = $pdo->prepare('INSERT INTO order_items (order_id, product_id, quantity, unit_price) VALUES (:oid, :pid, :qty, :price)');
            foreach ($validatedItems as $vi) {
                $itemStmt->execute([
                    ':oid'   => $orderId,
                    ':pid'   => $vi['product_id'],
                    ':qty'   => $vi['quantity'],
                    ':price' => $vi['unit_price']
                ]);
            }

            $pdo->commit();
            return [
                'success'  => true,
                'order_id' => $orderId,
                'total'    => round($total, 2),
                'items'    => count($validatedItems),
                'status'   => 'completed'
            ];
        } catch (\Throwable $e) {
            if ($pdo->inTransaction()) {
                $pdo->rollBack();
            }
            return ['success' => false, 'error' => $e->getMessage()];
        }
    }
}
