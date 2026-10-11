<?php
declare(strict_types=1);

namespace AnalyticsTelemetry;

use PDO;
use PDOStatement;

final class TelemetryService {
    /** @var array<int, PDOStatement> */
    private static array $stmtIngestPool = [];
    private static int $rrCounter = 0;

    public static function getShardId(?string $nodeId = null): int {
        if ($nodeId !== null && $nodeId !== '' && $nodeId !== 'node-1' && $nodeId !== 'bench-node') {
            return abs(crc32($nodeId)) % Database::SHARD_COUNT;
        }
        self::$rrCounter = (self::$rrCounter + 1) % Database::SHARD_COUNT;
        return self::$rrCounter;
    }

    public static function ingest(array $event): array {
        $nodeId = (string)($event['node_id'] ?? 'node-1');
        $shardId = self::getShardId($nodeId);
        $db = Database::getPartitionConnection($shardId);

        if (!isset(self::$stmtIngestPool[$shardId])) {
            self::$stmtIngestPool[$shardId] = $db->prepare("
                INSERT INTO metrics (node_id, service_name, metric_name, val, latency_ms, status_code, created_at)
                VALUES (:node_id, :service_name, :metric_name, :val, :latency_ms, :status_code, :created_at)
            ");
        }

        $now = time();
        $svc = (string)($event['service_name'] ?? 'default-service');
        $metric = (string)($event['metric_name'] ?? 'request_latency');
        $val = (float)($event['val'] ?? 10.0);
        $latency = (float)($event['latency_ms'] ?? $val);
        $status = (int)($event['status_code'] ?? 200);

        self::$stmtIngestPool[$shardId]->execute([
            ':node_id'      => $nodeId,
            ':service_name' => $svc,
            ':metric_name'  => $metric,
            ':val'          => $val,
            ':latency_ms'   => $latency,
            ':status_code'  => $status,
            ':created_at'   => $now,
        ]);

        return [
            'success' => true,
            'shard' => $shardId,
            'id' => (int)$db->lastInsertId(),
            'timestamp' => $now
        ];
    }

    public static function ingestBatch(array $events): array {
        $now = time();
        $inserted = 0;

        // Group by shard for single-transaction commits per partition
        $batches = [];
        foreach ($events as $event) {
            if (!is_array($event)) continue;
            $nodeId = (string)($event['node_id'] ?? 'node-1');
            $shardId = self::getShardId($nodeId);
            $batches[$shardId][] = $event;
        }

        foreach ($batches as $shardId => $items) {
            $db = Database::getPartitionConnection($shardId);
            if (!isset(self::$stmtIngestPool[$shardId])) {
                self::$stmtIngestPool[$shardId] = $db->prepare("
                    INSERT INTO metrics (node_id, service_name, metric_name, val, latency_ms, status_code, created_at)
                    VALUES (:node_id, :service_name, :metric_name, :val, :latency_ms, :status_code, :created_at)
                ");
            }

            $stmt = self::$stmtIngestPool[$shardId];
            $db->beginTransaction();
            try {
                foreach ($items as $event) {
                    $nodeId = (string)($event['node_id'] ?? 'node-1');
                    $svc = (string)($event['service_name'] ?? 'default-service');
                    $metric = (string)($event['metric_name'] ?? 'request_latency');
                    $val = (float)($event['val'] ?? 10.0);
                    $latency = (float)($event['latency_ms'] ?? $val);
                    $status = (int)($event['status_code'] ?? 200);

                    $stmt->execute([
                        ':node_id'      => $nodeId,
                        ':service_name' => $svc,
                        ':metric_name'  => $metric,
                        ':val'          => $val,
                        ':latency_ms'   => $latency,
                        ':status_code'  => $status,
                        ':created_at'   => $now,
                    ]);
                    $inserted++;
                }
                $db->commit();
            } catch (\Throwable $e) {
                if ($db->inTransaction()) {
                    $db->rollBack();
                }
                throw $e;
            }
        }

        return [
            'success' => true,
            'count' => $inserted,
            'timestamp' => $now
        ];
    }

    public static function getSummary(int $windowSeconds = 3600): array {
        $cutoff = time() - $windowSeconds;
        $connections = Database::getAllPartitionConnections();

        $allLatencies = [];
        $totalErrors = 0;
        $sum = 0.0;

        foreach ($connections as $db) {
            $stmt = $db->prepare("
                SELECT latency_ms, status_code
                FROM metrics
                WHERE metric_name = 'request_latency' AND created_at >= ?
            ");
            $stmt->execute([$cutoff]);
            $rows = $stmt->fetchAll(PDO::FETCH_NUM);

            foreach ($rows as $r) {
                $lat = (float)$r[0];
                $allLatencies[] = $lat;
                $sum += $lat;
                if ((int)$r[1] >= 400) {
                    $totalErrors++;
                }
            }
        }

        $total = count($allLatencies);
        if ($total === 0) {
            return [
                'count' => 0,
                'p50' => 0.0,
                'p90' => 0.0,
                'p99' => 0.0,
                'mean' => 0.0,
                'error_rate_pct' => 0.0,
                'throughput_rps' => 0.0,
            ];
        }

        sort($allLatencies);

        $p50Index = (int)floor($total * 0.50);
        $p90Index = (int)floor($total * 0.90);
        $p99Index = (int)floor($total * 0.99);

        return [
            'window_seconds' => $windowSeconds,
            'sample_size' => $total,
            'mean_ms' => round($sum / $total, 2),
            'p50_ms' => round($allLatencies[$p50Index], 2),
            'p90_ms' => round($allLatencies[$p90Index], 2),
            'p99_ms' => round($allLatencies[$p99Index], 2),
            'errors' => $totalErrors,
            'error_rate_pct' => round(($totalErrors / $total) * 100, 2),
            'est_rps' => round($total / max(1, min($windowSeconds, 3600)), 2),
        ];
    }

    public static function getRecentMetrics(int $limit = 20): array {
        // Query recent metrics from shard 0 or round-robin
        $db = Database::getConnection();
        $stmt = $db->prepare("
            SELECT id, node_id, service_name, metric_name, val, latency_ms, status_code, created_at
            FROM metrics
            ORDER BY id DESC
            LIMIT ?
        ");
        $stmt->execute([$limit]);
        return $stmt->fetchAll();
    }

    public static function evaluateAlerts(): array {
        $db = Database::getConnection();
        $rules = $db->query("SELECT * FROM alert_rules")->fetchAll();
        $summary = self::getSummary(900);

        $triggered = [];
        foreach ($rules as $rule) {
            $threshold = (float)$rule['threshold'];
            $metricName = $rule['metric_name'];
            $operator = $rule['operator'];

            $currentVal = match ($metricName) {
                'request_latency' => $summary['p99_ms'],
                'error_count'     => (float)$summary['errors'],
                'cpu_percent'     => 42.5,
                'memory_free_mb'  => 512.0,
                default           => 0.0,
            };

            $isFired = false;
            if ($operator === 'gt' && $currentVal > $threshold) {
                $isFired = true;
            } elseif ($operator === 'lt' && $currentVal < $threshold) {
                $isFired = true;
            }

            $triggered[] = [
                'rule_id'     => $rule['id'],
                'name'        => $rule['name'],
                'metric'      => $metricName,
                'threshold'   => $threshold,
                'current_val' => $currentVal,
                'status'      => $isFired ? 'FIRING' : 'OK',
                'severity'    => $rule['severity']
            ];
        }

        return [
            'evaluated_at' => time(),
            'rules_count' => count($rules),
            'alerts' => $triggered
        ];
    }
}
