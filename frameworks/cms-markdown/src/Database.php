<?php
declare(strict_types=1);

namespace CmsMarkdown;

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

        $dbPath = $dbDir . '/cms.sqlite';
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
            CREATE TABLE IF NOT EXISTS articles (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                slug TEXT UNIQUE NOT NULL,
                category TEXT NOT NULL,
                tags TEXT NOT NULL,
                content_markdown TEXT NOT NULL,
                views_count INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_articles_slug ON articles (slug);
            CREATE INDEX IF NOT EXISTS idx_articles_cat ON articles (category);
        ");
    }

    private static function seedInitialData(PDO $pdo): void {
        $articles = [
            [
                'Hyperion Architectural Deep Dive',
                'hyperion-architectural-deep-dive',
                'Architecture',
                'php,hyperion,performance,rust',
                "# Hyperion Architectural Deep Dive\n\nWelcome to the next generation of PHP application runtime.\n\n## 1. Zero-Bootstrapping Overhead\nTraditional PHP environments re-parse configuration files and autoload scripts on every request.\nHyperion pre-warms the application memory state.\n\n> \"High performance is achieved not by faster execution alone, but by eliminating unnecessary work.\"\n\n### Core Advantages:\n- Persistent OPcache state\n- Asynchronous event loops via Rust\n- Sub-millisecond latency guarantees\n\n```rust\nfn dispatch_request(req: Request) -> Response {\n    hyperion_runtime::process(req)\n}\n```\n\n## 2. Memory Isolation Guarantees\nHyperion enforces strict garbage collection and superglobal resets.\nEvery connection remains safe from cross-request leaks."
            ],
            [
                'Scaling High-Concurrency Microservices with PHP',
                'scaling-high-concurrency-microservices',
                'Engineering',
                'microservices,concurrency,scalability',
                "# Scaling High-Concurrency Microservices with PHP\n\nModern web services demand predictable throughput.\n\n## The Concurrency Bottleneck\nWhen thread counts escalate, database locking and context switching dominate.\n\n### Essential Techniques:\n- Connection pool amortization\n- Batching writes with group commits\n- Utilizing SQLite WAL mode for read parallelism\n\n```php\n\$pdo->exec('PRAGMA journal_mode = WAL;');\n```\n\n## Monitoring Latency Percentiles\nAverage latency is misleading. Optimize for P99 to ensure fair user experience."
            ],
            [
                'Database Concurrency and ACID Transaction Safety',
                'database-concurrency-acid-safety',
                'Database',
                'database,acid,transactions,sqlite',
                "# Database Concurrency and ACID Transaction Safety\n\nACID transactions ensure data consistency across failures.\n\n## Row Locking vs Table Locking\nIn financial ledgers, preventing double-spend anomalies requires row-level isolation.\n\n> Always verify balances inside active transactions before committing state changes.\n\n### Checklist:\n- Validate accounts exist\n- Ensure non-negative balances\n- Rollback immediately upon error"
            ],
            [
                'Modern Cryptography in Web Applications',
                'modern-cryptography-web-applications',
                'Security',
                'security,crypto,jwt,hmac',
                "# Modern Cryptography in Web Applications\n\nSecurity is paramount in distributed systems.\n\n## Timing Attack Defense\nWhen verifying tokens, avoid standard string equality operators.\nUse `hash_equals()` to guarantee constant-time execution.\n\n```php\nif (!hash_equals(\$signature, \$expected)) {\n    throw new SecurityException('Tampering detected');\n}\n```\n\n## Stateless Authorization\nJWTs eliminate central session store bottlenecks when combined with bitmask permissions."
            ]
        ];

        $stmt = $pdo->prepare("
            INSERT INTO articles (title, slug, category, tags, content_markdown, views_count, created_at)
            VALUES (?, ?, ?, ?, ?, 120, ?)
        ");

        $now = time();
        foreach ($articles as $a) {
            $stmt->execute([$a[0], $a[1], $a[2], $a[3], $a[4], $now]);
        }
    }
}
