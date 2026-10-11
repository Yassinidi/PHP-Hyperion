<?php
declare(strict_types=1);

namespace QueueWorkflow;

use PDO;
use PDOStatement;

final class QueueService {
    /** @var array<int, PDOStatement> */
    private static array $stmtPushPool = [];
    /** @var array<int, PDOStatement> */
    private static array $stmtUpdateClaimPool = [];
    /** @var array<int, PDOStatement> */
    private static array $stmtCompletePool = [];
    private static int $rrCounter = 0;

    public static function getShardId(string $queue): int {
        self::$rrCounter = (self::$rrCounter + 1) % Database::SHARD_COUNT;
        return self::$rrCounter;
    }

    public static function push(string $queue, array $payload, int $priority = 10, int $delay = 0): array {
        $shardId = self::getShardId($queue);
        $db = Database::getPartitionConnection($shardId);

        if (!isset(self::$stmtPushPool[$shardId])) {
            self::$stmtPushPool[$shardId] = $db->prepare("
                INSERT INTO jobs (queue_name, payload, priority, status, attempts, available_at, created_at)
                VALUES (:queue, :payload, :priority, 'pending', 0, :avail, :now)
            ");
        }

        $now = time();
        $avail = $now + $delay;
        $json = json_encode($payload);

        self::$stmtPushPool[$shardId]->execute([
            ':queue'    => $queue,
            ':payload'  => $json,
            ':priority' => $priority,
            ':avail'    => $avail,
            ':now'      => $now
        ]);

        return [
            'success' => true,
            'shard'   => $shardId,
            'job_id'  => (int)$db->lastInsertId(),
            'queue'   => $queue,
            'status'  => 'pending'
        ];
    }

    public static function pushBatch(string $queue, array $tasks, int $priority = 10, int $delay = 0): array {
        $shardId = self::getShardId($queue);
        $db = Database::getPartitionConnection($shardId);

        if (!isset(self::$stmtPushPool[$shardId])) {
            self::$stmtPushPool[$shardId] = $db->prepare("
                INSERT INTO jobs (queue_name, payload, priority, status, attempts, available_at, created_at)
                VALUES (:queue, :payload, :priority, 'pending', 0, :avail, :now)
            ");
        }

        $now = time();
        $avail = $now + $delay;
        $stmt = self::$stmtPushPool[$shardId];
        $inserted = 0;

        $db->beginTransaction();
        try {
            foreach ($tasks as $task) {
                $payload = is_array($task) ? $task : ['task' => (string)$task];
                $stmt->execute([
                    ':queue'    => $queue,
                    ':payload'  => json_encode($payload),
                    ':priority' => $priority,
                    ':avail'    => $avail,
                    ':now'      => $now
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

        return [
            'success'  => true,
            'shard'    => $shardId,
            'count'    => $inserted,
            'queue'    => $queue,
            'status'   => 'pending'
        ];
    }

    public static function claim(string $queue, string $workerId, int $leaseTimeout = 30): ?array {
        $now = time();
        $staleThreshold = $now - $leaseTimeout;
        $connections = Database::getAllPartitionConnections();

        // Round-robin shard scan for work
        $startShard = mt_rand(0, Database::SHARD_COUNT - 1);
        for ($i = 0; $i < Database::SHARD_COUNT; $i++) {
            $shardId = ($startShard + $i) % Database::SHARD_COUNT;
            $db = $connections[$shardId];

            $db->beginTransaction();
            try {
                $stmt = $db->prepare("
                    SELECT id, payload, priority, attempts, max_attempts
                    FROM jobs
                    WHERE queue_name = :queue
                      AND available_at <= :now
                      AND (status = 'pending' OR (status = 'claimed' AND claimed_at < :stale))
                    ORDER BY priority DESC, id ASC
                    LIMIT 1
                ");
                $stmt->execute([
                    ':queue' => $queue,
                    ':now'   => $now,
                    ':stale' => $staleThreshold
                ]);
                $job = $stmt->fetch();

                if ($job) {
                    $jobId = (int)$job['id'];
                    if (!isset(self::$stmtUpdateClaimPool[$shardId])) {
                        self::$stmtUpdateClaimPool[$shardId] = $db->prepare("
                            UPDATE jobs
                            SET status = 'claimed',
                                worker_id = :worker_id,
                                claimed_at = :now,
                                attempts = attempts + 1
                            WHERE id = :id
                        ");
                    }

                    self::$stmtUpdateClaimPool[$shardId]->execute([
                        ':worker_id' => $workerId,
                        ':now'       => $now,
                        ':id'        => $jobId
                    ]);

                    $db->commit();

                    return [
                        'id'           => $jobId,
                        'shard'        => $shardId,
                        'queue'        => $queue,
                        'payload'      => json_decode($job['payload'], true),
                        'priority'     => (int)$job['priority'],
                        'attempt'      => (int)$job['attempts'] + 1,
                        'claimed_by'   => $workerId,
                        'claimed_at'   => $now
                    ];
                }
                $db->commit();
            } catch (\Throwable $e) {
                if ($db->inTransaction()) {
                    $db->rollBack();
                }
                throw $e;
            }
        }

        return null;
    }

    public static function complete(int $jobId, array $result = [], int $shardId = 0): array {
        $db = Database::getPartitionConnection($shardId);
        if (!isset(self::$stmtCompletePool[$shardId])) {
            self::$stmtCompletePool[$shardId] = $db->prepare("
                UPDATE jobs
                SET status = 'completed',
                    completed_at = :now,
                    result_payload = :res
                WHERE id = :id AND status = 'claimed'
            ");
        }

        $now = time();
        self::$stmtCompletePool[$shardId]->execute([
            ':now' => $now,
            ':res' => json_encode($result),
            ':id'  => $jobId
        ]);

        return [
            'success' => self::$stmtCompletePool[$shardId]->rowCount() > 0,
            'job_id'  => $jobId,
            'status'  => 'completed'
        ];
    }

    public static function getStats(): array {
        $connections = Database::getAllPartitionConnections();
        $byStatus = ['pending' => 0, 'claimed' => 0, 'completed' => 0, 'failed' => 0];
        $queueDepths = [];

        foreach ($connections as $db) {
            $stats = $db->query("
                SELECT status, count(*) AS count
                FROM jobs
                GROUP BY status
            ")->fetchAll();

            foreach ($stats as $s) {
                $status = $s['status'];
                if (isset($byStatus[$status])) {
                    $byStatus[$status] += (int)$s['count'];
                }
            }

            $queues = $db->query("
                SELECT queue_name, count(*) AS pending_count
                FROM jobs
                WHERE status = 'pending'
                GROUP BY queue_name
            ")->fetchAll();

            foreach ($queues as $q) {
                $name = $q['queue_name'];
                $queueDepths[$name] = ($queueDepths[$name] ?? 0) + (int)$q['pending_count'];
            }
        }

        $depthList = [];
        foreach ($queueDepths as $name => $count) {
            $depthList[] = ['queue_name' => $name, 'pending_count' => $count];
        }

        return [
            'status_breakdown' => $byStatus,
            'queue_depths'     => $depthList,
            'total_jobs'       => array_sum($byStatus),
        ];
    }

    public static function getRecentJobs(int $limit = 20): array {
        $db = Database::getConnection();
        $stmt = $db->prepare("
            SELECT id, queue_name, priority, status, attempts, worker_id, claimed_at, completed_at, created_at
            FROM jobs
            ORDER BY id DESC
            LIMIT ?
        ");
        $stmt->execute([$limit]);
        return $stmt->fetchAll();
    }
}
