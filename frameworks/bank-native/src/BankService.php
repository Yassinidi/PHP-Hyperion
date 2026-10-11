<?php
declare(strict_types=1);

namespace BankNative;

use PDO;
use Exception;

class BankService {
    public static function getAccounts(): array {
        $db = Database::getConnection();
        $stmt = $db->query("
            SELECT a.id, a.account_number, a.holder_name, a.email, a.balance, a.currency, a.status,
                   COUNT(t.id) as total_transfers
            FROM accounts a
            LEFT JOIN transfers t ON (a.id = t.from_account_id OR a.id = t.to_account_id)
            GROUP BY a.id
            ORDER BY a.id ASC
        ");
        return $stmt->fetchAll();
    }

    public static function getAccount(int $id): ?array {
        $db = Database::getConnection();
        $stmt = $db->prepare("SELECT * FROM accounts WHERE id = ?");
        $stmt->execute([$id]);
        $acc = $stmt->fetch();
        if (!$acc) {
            return null;
        }

        // Recent transfers
        $tStmt = $db->prepare("
            SELECT t.*, fa.holder_name as from_holder, ta.holder_name as to_holder
            FROM transfers t
            JOIN accounts fa ON t.from_account_id = fa.id
            JOIN accounts ta ON t.to_account_id = ta.id
            WHERE t.from_account_id = ? OR t.to_account_id = ?
            ORDER BY t.id DESC
            LIMIT 15
        ");
        $tStmt->execute([$id, $id]);
        $acc['recent_transfers'] = $tStmt->fetchAll();

        return $acc;
    }

    private static ?\PDOStatement $accountStmt = null;
    private static ?\PDOStatement $debitStmt = null;
    private static ?\PDOStatement $creditStmt = null;
    private static ?\PDOStatement $ledgerStmt = null;
    private static int $txnCounter = 0;

    private static function generateReference(): string {
        self::$txnCounter++;
        return 'TXN-' . strtoupper(dechex((int)(microtime(true) * 1000000)) . str_pad((string)(self::$txnCounter % 10000), 4, '0', STR_PAD_LEFT));
    }

    public static function transfer(int $fromId, int $toId, float $amount, string $note = ''): array {
        if ($fromId === $toId) {
            throw new Exception("Source and destination accounts must be different.");
        }
        if ($amount <= 0) {
            throw new Exception("Transfer amount must be strictly greater than zero.");
        }

        $db = Database::getConnection();
        if (self::$accountStmt === null) {
            self::$accountStmt = $db->prepare("SELECT id, balance, status, holder_name, account_number FROM accounts WHERE id = ?");
            self::$debitStmt = $db->prepare("UPDATE accounts SET balance = balance - ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?");
            self::$creditStmt = $db->prepare("UPDATE accounts SET balance = balance + ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?");
            self::$ledgerStmt = $db->prepare("INSERT INTO transfers (reference, from_account_id, to_account_id, amount, note, status) VALUES (?, ?, ?, ?, ?, 'completed')");
        }

        $db->beginTransaction();

        try {
            // 1. Lock and inspect sender account
            self::$accountStmt->execute([$fromId]);
            $sender = self::$accountStmt->fetch();
            self::$accountStmt->closeCursor();

            if (!$sender) {
                throw new Exception("Sender account ID #{$fromId} not found.");
            }
            if ($sender['status'] !== 'active') {
                throw new Exception("Sender account #{$sender['account_number']} is frozen or inactive.");
            }
            if ((float)$sender['balance'] < $amount) {
                throw new Exception(sprintf(
                    "Insufficient funds. Account balance ($%.2f) is less than transfer amount ($%.2f).",
                    (float)$sender['balance'],
                    $amount
                ));
            }

            // 2. Lock and inspect recipient account
            self::$accountStmt->execute([$toId]);
            $recipient = self::$accountStmt->fetch();
            self::$accountStmt->closeCursor();

            if (!$recipient) {
                throw new Exception("Recipient account ID #{$toId} not found.");
            }
            if ($recipient['status'] !== 'active') {
                throw new Exception("Recipient account #{$recipient['account_number']} is frozen or inactive.");
            }

            // 3. Atomically debit sender
            self::$debitStmt->execute([$amount, $fromId]);

            // 4. Atomically credit recipient
            self::$creditStmt->execute([$amount, $toId]);

            // 5. Append immutable transfer ledger record
            $ref = self::generateReference();
            self::$ledgerStmt->execute([$ref, $fromId, $toId, $amount, $note]);
            $txnId = (int)$db->lastInsertId();

            $db->commit();

            return [
                'success' => true,
                'transfer_id' => $txnId,
                'reference' => $ref,
                'from_account' => $sender['account_number'],
                'from_holder' => $sender['holder_name'],
                'to_account' => $recipient['account_number'],
                'to_holder' => $recipient['holder_name'],
                'amount' => round($amount, 2),
                'sender_remaining_balance' => round((float)$sender['balance'] - $amount, 2),
                'recipient_new_balance' => round((float)$recipient['balance'] + $amount, 2),
                'timestamp' => microtime(true),
            ];
        } catch (\Throwable $e) {
            if ($db->inTransaction()) {
                $db->rollBack();
            }
            throw $e;
        }
    }

    public static function transferBatch(array $transfers): array {
        if (empty($transfers)) {
            return ['success' => true, 'count' => 0, 'results' => []];
        }

        $db = Database::getConnection();
        if (self::$accountStmt === null) {
            self::$accountStmt = $db->prepare("SELECT id, balance, status, holder_name, account_number FROM accounts WHERE id = ?");
            self::$debitStmt = $db->prepare("UPDATE accounts SET balance = balance - ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?");
            self::$creditStmt = $db->prepare("UPDATE accounts SET balance = balance + ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?");
            self::$ledgerStmt = $db->prepare("INSERT INTO transfers (reference, from_account_id, to_account_id, amount, note, status) VALUES (?, ?, ?, ?, ?, 'completed')");
        }

        $db->beginTransaction();
        $results = [];

        try {
            foreach ($transfers as $t) {
                $fromId = (int)($t['from_account_id'] ?? 0);
                $toId = (int)($t['to_account_id'] ?? 0);
                $amount = (float)($t['amount'] ?? 0.0);
                $note = (string)($t['note'] ?? 'Batch transfer');

                if ($fromId === $toId || $amount <= 0) {
                    continue;
                }

                self::$accountStmt->execute([$fromId]);
                $sender = self::$accountStmt->fetch();
                self::$accountStmt->closeCursor();

                if (!$sender || $sender['status'] !== 'active' || (float)$sender['balance'] < $amount) {
                    continue;
                }

                self::$accountStmt->execute([$toId]);
                $recipient = self::$accountStmt->fetch();
                self::$accountStmt->closeCursor();

                if (!$recipient || $recipient['status'] !== 'active') {
                    continue;
                }

                self::$debitStmt->execute([$amount, $fromId]);
                self::$creditStmt->execute([$amount, $toId]);

                $ref = self::generateReference();
                self::$ledgerStmt->execute([$ref, $fromId, $toId, $amount, $note]);
                $txnId = (int)$db->lastInsertId();

                $results[] = [
                    'transfer_id' => $txnId,
                    'reference' => $ref,
                    'amount' => $amount
                ];
            }

            $db->commit();
            return [
                'success' => true,
                'count' => count($results),
                'results' => $results,
                'timestamp' => microtime(true)
            ];
        } catch (\Throwable $e) {
            if ($db->inTransaction()) {
                $db->rollBack();
            }
            throw $e;
        }
    }

    public static function getTransfers(int $limit = 50): array {
        $db = Database::getConnection();
        $stmt = $db->prepare("
            SELECT t.id, t.reference, t.amount, t.note, t.status, t.created_at,
                   fa.account_number as from_account, fa.holder_name as from_holder,
                   ta.account_number as to_account, ta.holder_name as to_holder
            FROM transfers t
            JOIN accounts fa ON t.from_account_id = fa.id
            JOIN accounts ta ON t.to_account_id = ta.id
            ORDER BY t.id DESC
            LIMIT ?
        ");
        $stmt->bindValue(1, $limit, PDO::PARAM_INT);
        $stmt->execute();
        return $stmt->fetchAll();
    }

    public static function getStats(): array {
        $db = Database::getConnection();
        $accStats = $db->query("
            SELECT COUNT(*) as account_count,
                   COALESCE(SUM(balance), 0) as total_liquidity,
                   COALESCE(AVG(balance), 0) as avg_balance
            FROM accounts
        ")->fetch();

        $txnStats = $db->query("
            SELECT COUNT(*) as total_transfers,
                   COALESCE(SUM(amount), 0) as total_volume
            FROM transfers
            WHERE status = 'completed'
        ")->fetch();

        return [
            'total_accounts' => (int)($accStats['account_count'] ?? 0),
            'total_liquidity' => round((float)($accStats['total_liquidity'] ?? 0), 2),
            'avg_balance' => round((float)($accStats['avg_balance'] ?? 0), 2),
            'total_transfers' => (int)($txnStats['total_transfers'] ?? 0),
            'total_volume_transferred' => round((float)($txnStats['total_volume'] ?? 0), 2),
            'currency' => 'USD',
            'engine' => defined('HYPERION_PERSISTENT') ? 'PHP-Hyperion Core 2.0 (Persistent IPC)' : 'Standard PHP 8.4 (Zend Server)',
        ];
    }
}
