<?php
declare(strict_types=1);

namespace BankNative;

use PDO;

class Database {
    private static ?PDO $instance = null;

    public static function getConnection(): PDO {
        if (self::$instance === null) {
            $dbDir = dirname(__DIR__) . '/database';
            if (!is_dir($dbDir)) {
                mkdir($dbDir, 0777, true);
            }

            $dbPath = $dbDir . '/bank.sqlite';
            $isNew = !file_exists($dbPath) || filesize($dbPath) === 0;

            self::$instance = new PDO("sqlite:{$dbPath}", null, null, [
                PDO::ATTR_ERRMODE => PDO::ERRMODE_EXCEPTION,
                PDO::ATTR_DEFAULT_FETCH_MODE => PDO::FETCH_ASSOC,
            ]);

            // High-Performance SQLite WAL & Memory-Mapped Configuration
            self::$instance->exec("PRAGMA journal_mode = WAL;");
            self::$instance->exec("PRAGMA synchronous = NORMAL;");
            self::$instance->exec("PRAGMA busy_timeout = 5000;");
            self::$instance->exec("PRAGMA mmap_size = 268435456;");
            self::$instance->exec("PRAGMA cache_size = -64000;"); // 64MB cache
            self::$instance->exec("PRAGMA temp_store = MEMORY;");
            self::$instance->exec("PRAGMA wal_autocheckpoint = 1000;");

            if ($isNew) {
                self::initSchema(self::$instance);
            }
        }
        return self::$instance;
    }

    public static function resetConnection(): void {
        self::$instance = null;
    }

    private static function initSchema(PDO $db): void {
        $db->exec("
            CREATE TABLE IF NOT EXISTS accounts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                account_number TEXT UNIQUE NOT NULL,
                holder_name TEXT NOT NULL,
                email TEXT UNIQUE NOT NULL,
                balance REAL NOT NULL CHECK(balance >= 0),
                currency TEXT NOT NULL DEFAULT 'USD',
                status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active', 'frozen', 'closed')),
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS transfers (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                reference TEXT UNIQUE NOT NULL,
                from_account_id INTEGER NOT NULL,
                to_account_id INTEGER NOT NULL,
                amount REAL NOT NULL CHECK(amount > 0),
                note TEXT DEFAULT '',
                status TEXT NOT NULL DEFAULT 'completed' CHECK(status IN ('completed', 'failed', 'reversed')),
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (from_account_id) REFERENCES accounts(id),
                FOREIGN KEY (to_account_id) REFERENCES accounts(id)
            );

            CREATE INDEX IF NOT EXISTS idx_accounts_num ON accounts(account_number);
            CREATE INDEX IF NOT EXISTS idx_transfers_from ON transfers(from_account_id);
            CREATE INDEX IF NOT EXISTS idx_transfers_to ON transfers(to_account_id);
            CREATE INDEX IF NOT EXISTS idx_transfers_created ON transfers(created_at DESC);
        ");

        // Seed 5 Realistic Enterprise Bank Accounts
        $accounts = [
            ['ACC-1001', 'Alice Walker', 'alice.walker@mercury.bank', 25000.00, 'USD', 'active'],
            ['ACC-1002', 'Bob Martinez', 'bob.martinez@mercury.bank', 12500.00, 'USD', 'active'],
            ['ACC-1003', 'Charlie Dupont', 'charlie.dupont@mercury.bank', 8400.00, 'USD', 'active'],
            ['ACC-1004', 'Diana Prince', 'diana.prince@mercury.bank', 45000.00, 'USD', 'active'],
            ['ACC-1005', 'Evan Wright', 'evan.wright@mercury.bank', 3200.00, 'USD', 'active'],
        ];

        $stmt = $db->prepare("
            INSERT OR IGNORE INTO accounts (account_number, holder_name, email, balance, currency, status)
            VALUES (?, ?, ?, ?, ?, ?)
        ");
        foreach ($accounts as $acc) {
            $stmt->execute($acc);
        }

        // Seed Initial Ledger History
        $initialTransfers = [
            ['TXN-90001', 1, 2, 500.00, 'Invoice #1049 payment'],
            ['TXN-90002', 4, 3, 1200.00, 'Consulting retainer fee'],
            ['TXN-90003', 2, 5, 350.00, 'Reimbursement travel'],
        ];
        $tStmt = $db->prepare("
            INSERT OR IGNORE INTO transfers (reference, from_account_id, to_account_id, amount, note, status)
            VALUES (?, ?, ?, ?, ?, 'completed')
        ");
        foreach ($initialTransfers as $txn) {
            $tStmt->execute($txn);
        }
    }
}
