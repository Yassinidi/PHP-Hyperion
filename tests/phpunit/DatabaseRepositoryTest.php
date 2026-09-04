<?php
declare(strict_types=1);

namespace Tests\PHPUnit;

use PDO;
use PHPUnit\Framework\TestCase;

class UserRepository
{
    public function __construct(private PDO $pdo) {}

    public function createTable(): void
    {
        $this->pdo->exec("
            CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                email TEXT NOT NULL UNIQUE,
                role TEXT NOT NULL DEFAULT 'user',
                created_at TEXT NOT NULL
            )
        ");
    }

    public function insert(string $name, string $email, string $role = 'user'): int
    {
        $stmt = $this->pdo->prepare("
            INSERT INTO users (name, email, role, created_at)
            VALUES (:name, :email, :role, datetime('now'))
        ");
        $stmt->execute([
            ':name' => $name,
            ':email' => $email,
            ':role' => $role,
        ]);
        return (int) $this->pdo->lastInsertId();
    }

    public function findById(int $id): ?array
    {
        $stmt = $this->pdo->prepare("SELECT * FROM users WHERE id = ?");
        $stmt->execute([$id]);
        $row = $stmt->fetch(PDO::FETCH_ASSOC);
        return $row ?: null;
    }

    public function findByEmail(string $email): ?array
    {
        $stmt = $this->pdo->prepare("SELECT * FROM users WHERE email = ?");
        $stmt->execute([$email]);
        $row = $stmt->fetch(PDO::FETCH_ASSOC);
        return $row ?: null;
    }

    public function findByRole(string $role): array
    {
        $stmt = $this->pdo->prepare("SELECT * FROM users WHERE role = ? ORDER BY id ASC");
        $stmt->execute([$role]);
        return $stmt->fetchAll(PDO::FETCH_ASSOC);
    }

    public function updateRole(int $id, string $newRole): bool
    {
        $stmt = $this->pdo->prepare("UPDATE users SET role = ? WHERE id = ?");
        return $stmt->execute([$newRole, $id]);
    }

    public function delete(int $id): bool
    {
        $stmt = $this->pdo->prepare("DELETE FROM users WHERE id = ?");
        return $stmt->execute([$id]);
    }

    public function count(): int
    {
        $stmt = $this->pdo->query("SELECT COUNT(*) FROM users");
        return (int) $stmt->fetchColumn();
    }
}

class OrderRepository
{
    public function __construct(private PDO $pdo) {}

    public function createTables(): void
    {
        $this->pdo->exec("
            CREATE TABLE IF NOT EXISTS orders (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                order_number TEXT NOT NULL UNIQUE,
                user_id INTEGER NOT NULL,
                total_cents INTEGER NOT NULL,
                currency TEXT NOT NULL DEFAULT 'USD',
                status TEXT NOT NULL DEFAULT 'pending',
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS order_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                order_id INTEGER NOT NULL,
                product_name TEXT NOT NULL,
                unit_price_cents INTEGER NOT NULL,
                quantity INTEGER NOT NULL,
                FOREIGN KEY (order_id) REFERENCES orders(id) ON DELETE CASCADE
            );
        ");
    }

    public function createOrderWithItems(string $orderNumber, int $userId, int $totalCents, string $currency, array $items): int
    {
        $this->pdo->beginTransaction();
        try {
            $stmt = $this->pdo->prepare("
                INSERT INTO orders (order_number, user_id, total_cents, currency, status, created_at)
                VALUES (?, ?, ?, ?, 'pending', datetime('now'))
            ");
            $stmt->execute([$orderNumber, $userId, $totalCents, $currency]);
            $orderId = (int) $this->pdo->lastInsertId();

            $itemStmt = $this->pdo->prepare("
                INSERT INTO order_items (order_id, product_name, unit_price_cents, quantity)
                VALUES (?, ?, ?, ?)
            ");
            foreach ($items as $item) {
                $itemStmt->execute([
                    $orderId,
                    $item['name'],
                    $item['unit_price_cents'],
                    $item['quantity'],
                ]);
            }

            $this->pdo->commit();
            return $orderId;
        } catch (\Throwable $e) {
            $this->pdo->rollBack();
            throw $e;
        }
    }

    public function findOrderWithItems(int $orderId): ?array
    {
        $stmt = $this->pdo->prepare("SELECT * FROM orders WHERE id = ?");
        $stmt->execute([$orderId]);
        $order = $stmt->fetch(PDO::FETCH_ASSOC);
        if (!$order) {
            return null;
        }

        $itemsStmt = $this->pdo->prepare("SELECT * FROM order_items WHERE order_id = ? ORDER BY id ASC");
        $itemsStmt->execute([$orderId]);
        $order['items'] = $itemsStmt->fetchAll(PDO::FETCH_ASSOC);

        return $order;
    }

    public function findOrdersByStatus(string $status): array
    {
        $stmt = $this->pdo->prepare("
            SELECT o.*, u.name as user_name, u.email as user_email
            FROM orders o
            JOIN users u ON u.id = o.user_id
            WHERE o.status = ?
            ORDER BY o.id DESC
        ");
        $stmt->execute([$status]);
        return $stmt->fetchAll(PDO::FETCH_ASSOC);
    }
}

final class DatabaseRepositoryTest extends TestCase
{
    private PDO $pdo;
    private UserRepository $userRepo;
    private OrderRepository $orderRepo;

    protected function setUp(): void
    {
        $this->pdo = new PDO('sqlite::memory:');
        $this->pdo->setAttribute(PDO::ATTR_ERRMODE, PDO::ERRMODE_EXCEPTION);

        $this->userRepo = new UserRepository($this->pdo);
        $this->userRepo->createTable();

        $this->orderRepo = new OrderRepository($this->pdo);
        $this->orderRepo->createTables();
    }

    public function testUserCrudOperations(): void
    {
        $this->assertSame(0, $this->userRepo->count());

        $userId = $this->userRepo->insert('Alice Johnson', 'alice@hyperion.io', 'admin');
        $this->assertGreaterThan(0, $userId);
        $this->assertSame(1, $this->userRepo->count());

        $user = $this->userRepo->findById($userId);
        $this->assertNotNull($user);
        $this->assertSame('Alice Johnson', $user['name']);
        $this->assertSame('alice@hyperion.io', $user['email']);
        $this->assertSame('admin', $user['role']);

        $byEmail = $this->userRepo->findByEmail('alice@hyperion.io');
        $this->assertSame($userId, (int) $byEmail['id']);

        $updated = $this->userRepo->updateRole($userId, 'superadmin');
        $this->assertTrue($updated);
        $refetched = $this->userRepo->findById($userId);
        $this->assertSame('superadmin', $refetched['role']);

        $deleted = $this->userRepo->delete($userId);
        $this->assertTrue($deleted);
        $this->assertSame(0, $this->userRepo->count());
        $this->assertNull($this->userRepo->findById($userId));
    }

    public function testOrderTransactionalCreationWithItems(): void
    {
        $userId = $this->userRepo->insert('Bob Smith', 'bob@hyperion.io');

        $items = [
            ['name' => 'High-Speed PHP Compiler', 'unit_price_cents' => 19900, 'quantity' => 1],
            ['name' => 'JIT Optimization License', 'unit_price_cents' => 9900, 'quantity' => 2],
        ];

        $orderId = $this->orderRepo->createOrderWithItems('ORD-2026-001', $userId, 39700, 'USD', $items);
        $this->assertGreaterThan(0, $orderId);

        $order = $this->orderRepo->findOrderWithItems($orderId);
        $this->assertNotNull($order);
        $this->assertSame('ORD-2026-001', $order['order_number']);
        $this->assertSame(39700, (int) $order['total_cents']);
        $this->assertSame('pending', $order['status']);
        $this->assertCount(2, $order['items']);
        $this->assertSame('High-Speed PHP Compiler', $order['items'][0]['product_name']);
        $this->assertSame(1, (int) $order['items'][0]['quantity']);
        $this->assertSame('JIT Optimization License', $order['items'][1]['product_name']);
        $this->assertSame(2, (int) $order['items'][1]['quantity']);
    }

    public function testOrderTransactionRollbackOnError(): void
    {
        $userId = $this->userRepo->insert('Charlie Brown', 'charlie@hyperion.io');

        $initialOrders = $this->pdo->query("SELECT COUNT(*) FROM orders")->fetchColumn();
        $initialItems = $this->pdo->query("SELECT COUNT(*) FROM order_items")->fetchColumn();

        $invalidItems = [
            ['name' => 'Valid Item', 'unit_price_cents' => 1000, 'quantity' => 1],
            // Missing required field 'quantity' to cause an SQL error
            ['name' => 'Broken Item', 'unit_price_cents' => 2000, 'quantity' => null],
        ];

        try {
            $this->orderRepo->createOrderWithItems('ORD-FAIL-01', $userId, 3000, 'USD', $invalidItems);
            $this->fail('Expected PDOException was not thrown');
        } catch (\Throwable) {
            // Success: Exception caught, check rollback
        }

        $afterOrders = $this->pdo->query("SELECT COUNT(*) FROM orders")->fetchColumn();
        $afterItems = $this->pdo->query("SELECT COUNT(*) FROM order_items")->fetchColumn();

        $this->assertSame($initialOrders, $afterOrders);
        $this->assertSame($initialItems, $afterItems);
    }

    public function testJoinQueryFilterByStatus(): void
    {
        $u1 = $this->userRepo->insert('Dev Alpha', 'alpha@hyperion.io');
        $u2 = $this->userRepo->insert('Dev Beta', 'beta@hyperion.io');

        $this->orderRepo->createOrderWithItems('ORD-101', $u1, 5000, 'USD', [
            ['name' => 'RAM Upgrade', 'unit_price_cents' => 5000, 'quantity' => 1]
        ]);
        $this->orderRepo->createOrderWithItems('ORD-102', $u2, 8000, 'USD', [
            ['name' => 'NVMe SSD', 'unit_price_cents' => 8000, 'quantity' => 1]
        ]);

        $pending = $this->orderRepo->findOrdersByStatus('pending');
        $this->assertCount(2, $pending);
        $this->assertSame('Dev Beta', $pending[0]['user_name']);
        $this->assertSame('beta@hyperion.io', $pending[0]['user_email']);
        $this->assertSame('Dev Alpha', $pending[1]['user_name']);
        $this->assertSame('alpha@hyperion.io', $pending[1]['user_email']);

        $shipped = $this->orderRepo->findOrdersByStatus('shipped');
        $this->assertCount(0, $shipped);
    }
}
