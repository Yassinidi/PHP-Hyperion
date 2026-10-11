<?php
declare(strict_types=1);

namespace AuthRbac;

use PDO;
use PDOStatement;

final class AuthService {
    private const SECRET = 'hyperion_enterprise_jwt_signing_key_9999';

    public const PERM_READ_PUBLIC   = 1;
    public const PERM_WRITE_CONTENT = 2;
    public const PERM_DELETE_CONTENT = 4;
    public const PERM_MANAGE_USERS  = 8;
    public const PERM_SYSTEM_ADMIN  = 16;

    private static ?PDOStatement $stmtUser = null;
    private static ?PDOStatement $stmtRole = null;
    private static ?PDOStatement $stmtRevoked = null;

    private static function db(): PDO {
        return Database::getConnection();
    }

    private static function base64UrlEncode(string $data): string {
        return rtrim(strtr(base64_encode($data), '+/', '-_'), '=');
    }

    private static function base64UrlDecode(string $data): string {
        return base64_decode(strtr($data, '-_', '+/'));
    }

    public static function issueToken(string $username, string $password): array {
        $db = self::db();
        if (self::$stmtUser === null) {
            self::$stmtUser = $db->prepare("SELECT id, username, password_hash, role_id, status FROM users WHERE username = ?");
        }
        self::$stmtUser->execute([$username]);
        $user = self::$stmtUser->fetch();

        if (!$user || !password_verify($password, $user['password_hash'])) {
            return ['success' => false, 'error' => 'Invalid credentials'];
        }

        // Calculate inherited permission mask
        $mask = self::calculateRoleMask((int)$user['role_id']);
        $jti = bin2hex(random_bytes(16));
        $now = time();

        $header = self::base64UrlEncode((string)json_encode(['alg' => 'HS256', 'typ' => 'JWT']));
        $payload = self::base64UrlEncode((string)json_encode([
            'sub'   => (int)$user['id'],
            'name'  => $user['username'],
            'role'  => (int)$user['role_id'],
            'perms' => $mask,
            'jti'   => $jti,
            'iat'   => $now,
            'exp'   => $now + 3600,
        ]));

        $signature = self::base64UrlEncode(hash_hmac('sha256', "{$header}.{$payload}", self::SECRET, true));
        $jwt = "{$header}.{$payload}.{$signature}";

        return [
            'success' => true,
            'token' => $jwt,
            'user' => [
                'id' => $user['id'],
                'username' => $user['username'],
                'role_id' => $user['role_id'],
                'permissions' => $mask,
            ]
        ];
    }

    public static function authorize(string $token, int $requiredPerm): array {
        $parts = explode('.', $token);
        if (count($parts) !== 3) {
            return ['success' => false, 'error' => 'Malformed JWT'];
        }

        [$headerB64, $payloadB64, $signatureB64] = $parts;

        // 1. Timing-safe cryptographic HMAC verification
        $expectedSig = self::base64UrlEncode(hash_hmac('sha256', "{$headerB64}.{$payloadB64}", self::SECRET, true));
        if (!hash_equals($expectedSig, $signatureB64)) {
            return ['success' => false, 'error' => 'Invalid signature'];
        }

        $payload = json_decode(self::base64UrlDecode($payloadB64), true);
        if (!is_array($payload)) {
            return ['success' => false, 'error' => 'Corrupt claims'];
        }

        // 2. Expiration check
        if (($payload['exp'] ?? 0) < time()) {
            return ['success' => false, 'error' => 'Token expired'];
        }

        // 3. Revocation check
        $jti = $payload['jti'] ?? '';
        if (self::$stmtRevoked === null) {
            self::$stmtRevoked = self::db()->prepare("SELECT 1 FROM revocations WHERE jti = ?");
        }
        self::$stmtRevoked->execute([$jti]);
        if (self::$stmtRevoked->fetch()) {
            return ['success' => false, 'error' => 'Token has been revoked'];
        }

        // 4. Bitwise RBAC evaluation
        $userPerms = (int)($payload['perms'] ?? 0);
        $hasAccess = ($userPerms & $requiredPerm) === $requiredPerm;

        if (!$hasAccess) {
            return [
                'success' => false,
                'error' => 'Forbidden: Required permission bitmask not satisfied',
                'required' => $requiredPerm,
                'granted' => $userPerms
            ];
        }

        return [
            'success' => true,
            'user_id' => $payload['sub'],
            'username' => $payload['name'],
            'permissions' => $userPerms
        ];
    }

    public static function calculateRoleMask(int $roleId): int {
        $db = self::db();
        if (self::$stmtRole === null) {
            self::$stmtRole = $db->prepare("SELECT id, parent_id, permission_mask FROM roles WHERE id = ?");
        }

        $mask = 0;
        $currId = $roleId;
        $visited = [];

        while ($currId > 0 && !isset($visited[$currId])) {
            $visited[$currId] = true;
            self::$stmtRole->execute([$currId]);
            $role = self::$stmtRole->fetch();
            if (!$role) break;

            $mask |= (int)$role['permission_mask'];
            $currId = (int)($role['parent_id'] ?? 0);
        }

        return $mask;
    }

    public static function getRoles(): array {
        return self::db()->query("SELECT * FROM roles ORDER BY id ASC")->fetchAll();
    }

    public static function getUsers(): array {
        return self::db()->query("SELECT id, username, role_id, status FROM users ORDER BY id ASC")->fetchAll();
    }
}
