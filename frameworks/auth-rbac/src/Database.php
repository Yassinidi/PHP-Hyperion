<?php
declare(strict_types=1);

namespace AuthRbac;

use PDO;

final class Database {
    private static ?PDO $pdo = null;

    public static function getConnection(): PDO {
        if (self::$pdo !== null) {
            return self::$pdo;
        }

        $dbDir = __DIR__ . '/../storage';
        if (!is_dir($dbDir)) {
            mkdir($dbDir, 0777, true);
        }

        $dbPath = $dbDir . '/auth.sqlite';
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
        $pdo->exec("PRAGMA mmap_size = 268435456;");

        if ($isNew) {
            self::initSchema($pdo);
            self::seedInitialData($pdo);
        }

        self::$pdo = $pdo;
        return self::$pdo;
    }

    private static function initSchema(PDO $pdo): void {
        $pdo->exec("
            CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT UNIQUE NOT NULL,
                password_hash TEXT NOT NULL,
                role_id INTEGER NOT NULL,
                status TEXT NOT NULL DEFAULT 'active'
            );

            CREATE TABLE IF NOT EXISTS roles (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT UNIQUE NOT NULL,
                parent_id INTEGER NULL,
                permission_mask INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS revocations (
                jti TEXT PRIMARY KEY,
                revoked_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS audit_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL,
                action TEXT NOT NULL,
                ip TEXT NOT NULL,
                created_at INTEGER NOT NULL
            );
        ");
    }

    private static function seedInitialData(PDO $pdo): void {
        // Permission Bits:
        // 1: read_public, 2: write_content, 4: delete_content, 8: manage_users, 16: system_admin
        $roles = [
            [1, 'guest', null, 1],
            [2, 'member', 1, 1 | 2],          // 3
            [3, 'moderator', 2, 1 | 2 | 4],   // 7
            [4, 'admin', 3, 1 | 2 | 4 | 8],   // 15
            [5, 'superadmin', 4, 31],         // all 5 flags
        ];

        $stmt = $pdo->prepare("INSERT INTO roles (id, name, parent_id, permission_mask) VALUES (?, ?, ?, ?)");
        foreach ($roles as $r) {
            $stmt->execute($r);
        }

        $passHash = password_hash('HyperionSec2026!', PASSWORD_BCRYPT);
        $users = [
            ['alice_admin', $passHash, 4],
            ['bob_dev', $passHash, 2],
            ['charlie_super', $passHash, 5],
            ['david_guest', $passHash, 1],
        ];

        $uStmt = $pdo->prepare("INSERT INTO users (username, password_hash, role_id) VALUES (?, ?, ?)");
        foreach ($users as $u) {
            $uStmt->execute($u);
        }
    }
}
