<?php
declare(strict_types=1);

namespace QueueWorkflow;

use PDO;

final class Database {
    public const SHARD_COUNT = 4;
    /** @var array<int, PDO> */
    private static array $shardPool = [];

    public static function getPartitionConnection(int $shardId): PDO {
        $shardId = abs($shardId) % self::SHARD_COUNT;
        if (isset(self::$shardPool[$shardId])) {
            return self::$shardPool[$shardId];
        }

        $dbDir = __DIR__ . '/../storage';
        if (!is_dir($dbDir)) {
            mkdir($dbDir, 0777, true);
        }

        $dbPath = $dbDir . "/queue_{$shardId}.sqlite";
        $isNew = !file_exists($dbPath);

        $pdo = new PDO('sqlite:' . $dbPath, null, null, [
            PDO::ATTR_ERRMODE            => PDO::ERRMODE_EXCEPTION,
            PDO::ATTR_DEFAULT_FETCH_MODE => PDO::FETCH_ASSOC,
            PDO::ATTR_PERSISTENT         => false,
        ]);

        $pdo->exec("PRAGMA journal_mode = WAL;");
        $pdo->exec("PRAGMA synchronous = NORMAL;");
        $pdo->exec("PRAGMA busy_timeout = 5000;");
        $pdo->exec("PRAGMA temp_store = MEMORY;");
        $pdo->exec("PRAGMA mmap_size = 67108864;");

        if ($isNew) {
            self::initSchema($pdo);
            self::seedInitialData($pdo, $shardId);
        }

        self::$shardPool[$shardId] = $pdo;
        return $pdo;
    }

    public static function getConnection(): PDO {
        return self::getPartitionConnection(0);
    }

    public static function getAllPartitionConnections(): array {
        $connections = [];
        for ($i = 0; $i < self::SHARD_COUNT; $i++) {
            $connections[$i] = self::getPartitionConnection($i);
        }
        return $connections;
    }

    private static function initSchema(PDO $pdo): void {
        $pdo->exec("
            CREATE TABLE IF NOT EXISTS jobs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                queue_name TEXT NOT NULL,
                payload TEXT NOT NULL,
                priority INTEGER NOT NULL DEFAULT 10,
                status TEXT NOT NULL DEFAULT 'pending',
                attempts INTEGER NOT NULL DEFAULT 0,
                max_attempts INTEGER NOT NULL DEFAULT 3,
                worker_id TEXT NULL,
                claimed_at INTEGER NULL,
                completed_at INTEGER NULL,
                result_payload TEXT NULL,
                available_at INTEGER NOT NULL,
                created_at INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_jobs_claim 
                ON jobs (queue_name, status, available_at, priority DESC, id ASC);

            CREATE INDEX IF NOT EXISTS idx_jobs_status
                ON jobs (status);
        ");
    }

    private static function seedInitialData(PDO $pdo, int $shardId): void {
        $now = time();
        $stmt = $pdo->prepare("
            INSERT INTO jobs (queue_name, payload, priority, status, attempts, available_at, created_at)
            VALUES (?, ?, ?, 'pending', 0, ?, ?)
        ");

        $queues = ['default', 'notifications', 'webhooks', 'payments'];
        $pdo->beginTransaction();
        for ($i = 0; $i < 125; $i++) {
            $q = $queues[$i % count($queues)];
            $payload = json_encode([
                'task' => 'task_' . ($shardId * 125 + $i),
                'user_id' => mt_rand(100, 999),
                'amount' => mt_rand(10, 500),
            ]);
            $priority = ($i % 10 === 0) ? 50 : 10;
            $stmt->execute([$q, $payload, $priority, $now, $now]);
        }
        $pdo->commit();
    }
}
