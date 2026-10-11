<?php
declare(strict_types=1);

namespace AnalyticsTelemetry;

use PDO;

final class Database {
    public const SHARD_COUNT = 8;
    /** @var array<int, PDO> */
    private static array $shardPool = [];
    private static ?PDO $defaultPdo = null;

    public static function getPartitionConnection(int $shardId): PDO {
        $shardId = abs($shardId) % self::SHARD_COUNT;
        if (isset(self::$shardPool[$shardId])) {
            return self::$shardPool[$shardId];
        }

        $dbDir = __DIR__ . '/../storage';
        if (!is_dir($dbDir)) {
            mkdir($dbDir, 0777, true);
        }

        $dbPath = $dbDir . "/telemetry_{$shardId}.sqlite";
        $isNew = !file_exists($dbPath);

        $pdo = new PDO('sqlite:' . $dbPath, null, null, [
            PDO::ATTR_ERRMODE            => PDO::ERRMODE_EXCEPTION,
            PDO::ATTR_DEFAULT_FETCH_MODE => PDO::FETCH_ASSOC,
            PDO::ATTR_PERSISTENT         => false,
        ]);

        $pdo->exec("PRAGMA journal_mode = WAL;");
        $pdo->exec("PRAGMA synchronous = OFF;"); // Maximum throughput for telemetry time-series
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
            CREATE TABLE IF NOT EXISTS metrics (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                node_id TEXT NOT NULL,
                service_name TEXT NOT NULL,
                metric_name TEXT NOT NULL,
                val REAL NOT NULL,
                latency_ms REAL NOT NULL,
                status_code INTEGER NOT NULL,
                created_at INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_metrics_query 
                ON metrics (metric_name, created_at);

            CREATE INDEX IF NOT EXISTS idx_metrics_service 
                ON metrics (service_name, created_at);

            CREATE TABLE IF NOT EXISTS alert_rules (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                metric_name TEXT NOT NULL,
                threshold REAL NOT NULL,
                operator TEXT NOT NULL,
                severity TEXT NOT NULL
            );
        ");
    }

    private static function seedInitialData(PDO $pdo, int $shardId): void {
        if ($shardId === 0) {
            $rules = [
                ['High P99 Latency', 'request_latency', 50.0, 'gt', 'critical'],
                ['High CPU Utilization', 'cpu_percent', 85.0, 'gt', 'warning'],
                ['Error Rate Spike', 'error_count', 10.0, 'gt', 'critical'],
                ['Low Memory Headroom', 'memory_free_mb', 256.0, 'lt', 'warning'],
            ];

            $stmt = $pdo->prepare("INSERT INTO alert_rules (name, metric_name, threshold, operator, severity) VALUES (?, ?, ?, ?, ?)");
            foreach ($rules as $r) {
                $stmt->execute($r);
            }
        }

        // Seed 150 baseline points per shard (1,200 total)
        $services = ['api-gateway', 'payment-core', 'order-processor', 'inventory-service'];
        $now = time();
        $metricStmt = $pdo->prepare("
            INSERT INTO metrics (node_id, service_name, metric_name, val, latency_ms, status_code, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
        ");

        $pdo->beginTransaction();
        for ($i = 0; $i < 150; $i++) {
            $svc = $services[$i % count($services)];
            $latency = mt_rand(5, 40) + (mt_rand(0, 100) / 100.0);
            if ($i % 25 === 0) {
                $latency += mt_rand(40, 100);
            }
            $status = ($i % 40 === 0) ? 500 : 200;
            $ts = $now - mt_rand(0, 3600);
            $metricStmt->execute([
                'node-' . (($i % 8) + 1),
                $svc,
                'request_latency',
                $latency,
                $latency,
                $status,
                $ts
            ]);
        }
        $pdo->commit();
    }
}
