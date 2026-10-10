<?php
declare(strict_types=1);

function wp(): void {
    // WordPress main query initialization
}

function wp_send_json(mixed $response, ?int $statusCode = null): void {
    if ($statusCode !== null) {
        http_response_code($statusCode);
    }
    header('Content-Type: application/json; charset=utf-8');
    echo json_encode($response);
}

function wp_send_json_success(mixed $data = null, int $statusCode = 200): void {
    wp_send_json(['success' => true, 'data' => $data], $statusCode);
}

function wp_send_json_error(mixed $data = null, int $statusCode = 400): void {
    wp_send_json(['success' => false, 'data' => $data], $statusCode);
}

function wp_get_banners(): array {
    global $wpdb;
    return $wpdb->get_results("SELECT * FROM wp_banners WHERE is_active = 1 ORDER BY id ASC");
}

function wp_get_categories(): array {
    global $wpdb;
    return $wpdb->get_results("SELECT * FROM wp_categories ORDER BY display_order ASC");
}

function wp_get_products(?int $categoryId = null): array {
    global $wpdb;
    if ($categoryId) {
        $stmt = $wpdb->prepare("SELECT p.*, c.name as category_name FROM wp_products p JOIN wp_categories c ON p.category_id = c.id WHERE p.category_id = :cid ORDER BY p.id ASC", [':cid' => $categoryId]);
        return $stmt->fetchAll();
    }
    return $wpdb->get_results("SELECT p.*, c.name as category_name FROM wp_products p JOIN wp_categories c ON p.category_id = c.id ORDER BY p.id ASC");
}

function wp_get_product(int $id): ?array {
    global $wpdb;
    $stmt = $wpdb->prepare("SELECT p.*, c.name as category_name FROM wp_products p JOIN wp_categories c ON p.category_id = c.id WHERE p.id = :id LIMIT 1", [':id' => $id]);
    $prod = $stmt->fetch();
    return $prod ?: null;
}

function wp_get_users(): array {
    global $wpdb;
    return $wpdb->get_results("SELECT id, name, email, role, created_at FROM wp_users ORDER BY id ASC");
}

function wp_authenticate_user(string $email, string $password): ?array {
    global $wpdb;
    $stmt = $wpdb->prepare("SELECT id, name, email, password, role FROM wp_users WHERE email = :email LIMIT 1", [':email' => $email]);
    $user = $stmt->fetch();
    if ($user && password_verify($password, $user['password'])) {
        unset($user['password']);
        return $user;
    }
    return null;
}

function wc_checkout(array $data): array {
    global $wpdb;
    $userId = (int)($data['user_id'] ?? 2);
    $items = $data['items'] ?? [];
    if (empty($items)) {
        return ['success' => false, 'error' => 'Cart is empty'];
    }

    $pdo = $wpdb->get_pdo();
    $pdo->beginTransaction();
    try {
        $total = 0.0;
        $validatedItems = [];
        foreach ($items as $item) {
            $pid = (int)($item['product_id'] ?? 0);
            $qty = max(1, (int)($item['quantity'] ?? 1));
            $stmt = $pdo->prepare('SELECT price, stock FROM wp_products WHERE id = :id');
            $stmt->execute([':id' => $pid]);
            $p = $stmt->fetch();
            if (!$p) {
                throw new Exception("Product ID {$pid} not found");
            }
            $price = (float)$p['price'];
            $total += $price * $qty;
            $validatedItems[] = ['product_id' => $pid, 'quantity' => $qty, 'unit_price' => $price];

            $upd = $pdo->prepare('UPDATE wp_products SET stock = MAX(0, stock - :qty) WHERE id = :id');
            $upd->execute([':qty' => $qty, ':id' => $pid]);
        }

        $orderStmt = $pdo->prepare('INSERT INTO wp_orders (user_id, total, status) VALUES (:uid, :total, :status)');
        $orderStmt->execute([':uid' => $userId, ':total' => $total, ':status' => 'completed']);
        $orderId = (int)$pdo->lastInsertId();

        $itemStmt = $pdo->prepare('INSERT INTO wp_order_items (order_id, product_id, quantity, unit_price) VALUES (:oid, :pid, :qty, :price)');
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
    } catch (Throwable $e) {
        if ($pdo->inTransaction()) {
            $pdo->rollBack();
        }
        return ['success' => false, 'error' => $e->getMessage()];
    }
}

function wp_get_admin_dashboard(): array {
    global $wpdb;
    $pdo = $wpdb->get_pdo();

    $revenue = (float)$pdo->query("SELECT COALESCE(SUM(total), 0) FROM wp_orders WHERE status = 'completed'")->fetchColumn();
    $totalOrders = (int)$pdo->query("SELECT COUNT(*) FROM wp_orders")->fetchColumn();
    $totalProducts = (int)$pdo->query("SELECT COUNT(*) FROM wp_products")->fetchColumn();
    $totalClients = (int)$pdo->query("SELECT COUNT(*) FROM wp_users WHERE role = 'client'")->fetchColumn();
    $lowStock = $pdo->query("SELECT id, name, stock FROM wp_products WHERE stock < 20 ORDER BY stock ASC")->fetchAll();
    $recentOrders = $pdo->query("
        SELECT o.id, o.total, o.status, o.created_at, u.name as client_name, u.email as client_email
        FROM wp_orders o
        JOIN wp_users u ON o.user_id = u.id
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
