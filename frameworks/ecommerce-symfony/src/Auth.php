<?php
declare(strict_types=1);

namespace App;

class Auth {
    public static function attempt(string $email, string $password): ?array {
        $pdo = Database::get();
        $stmt = $pdo->prepare('SELECT id, name, email, password, role FROM users WHERE email = :email LIMIT 1');
        $stmt->execute([':email' => $email]);
        $user = $stmt->fetch();
        if ($user && password_verify($password, $user['password'])) {
            unset($user['password']);
            return $user;
        }
        return null;
    }

    public static function allUsers(): array {
        $pdo = Database::get();
        $stmt = $pdo->query('SELECT id, name, email, role, created_at FROM users ORDER BY id ASC');
        return $stmt->fetchAll();
    }
}
