<?php
declare(strict_types=1);

namespace EcommerceNative;

use PDO;

class Auth {
    public static function attempt(string $email, string $password): ?array {
        $db = Database::getConnection();
        $stmt = $db->prepare("SELECT id, name, email, password_hash, role FROM users WHERE email = :email");
        $stmt->execute(['email' => $email]);
        $user = $stmt->fetch();

        if ($user && password_verify($password, $user['password_hash'])) {
            unset($user['password_hash']);
            return $user;
        }
        return null;
    }

    public static function getUserById(int $id): ?array {
        $db = Database::getConnection();
        $stmt = $db->prepare("SELECT id, name, email, role, created_at FROM users WHERE id = :id");
        $stmt->execute(['id' => $id]);
        $user = $stmt->fetch();
        return $user ?: null;
    }

    public static function getAllUsers(): array {
        $db = Database::getConnection();
        $stmt = $db->query("SELECT id, name, email, role, created_at FROM users ORDER BY id ASC");
        return $stmt->fetchAll();
    }
}
