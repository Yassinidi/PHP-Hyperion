<?php
declare(strict_types=1);

namespace CmsMarkdown;

use PDO;
use PDOStatement;

final class CmsService {
    private static ?PDOStatement $stmtBySlug = null;
    private static ?PDOStatement $stmtIncViews = null;
    private static ?PDOStatement $stmtSearch = null;

    private static function db(): PDO {
        return Database::getConnection();
    }

    public static function getArticles(int $limit = 20, int $offset = 0): array {
        $db = self::db();
        $stmt = $db->prepare("
            SELECT id, title, slug, category, tags, views_count, created_at,
                   substr(content_markdown, 1, 180) AS excerpt
            FROM articles
            ORDER BY id ASC
            LIMIT :limit OFFSET :offset
        ");
        $stmt->bindValue(':limit', $limit, PDO::PARAM_INT);
        $stmt->bindValue(':offset', $offset, PDO::PARAM_INT);
        $stmt->execute();
        return $stmt->fetchAll();
    }

    public static function getArticle(string $slug): ?array {
        $db = self::db();
        if (self::$stmtBySlug === null) {
            self::$stmtBySlug = $db->prepare("
                SELECT id, title, slug, category, tags, content_markdown, views_count, created_at
                FROM articles
                WHERE slug = ?
            ");
        }
        self::$stmtBySlug->execute([$slug]);
        $row = self::$stmtBySlug->fetch();

        if (!$row) {
            return null;
        }

        // Increment view count
        if (self::$stmtIncViews === null) {
            self::$stmtIncViews = $db->prepare("UPDATE articles SET views_count = views_count + 1 WHERE id = ?");
        }
        self::$stmtIncViews->execute([$row['id']]);

        // Parse markdown to HTML and extract TOC
        $parsed = MarkdownParser::parse($row['content_markdown']);

        return array_merge($row, [
            'views_count'  => $row['views_count'] + 1,
            'html'         => $parsed['html'],
            'toc'          => $parsed['toc'],
            'word_count'   => $parsed['word_count'],
            'read_minutes' => $parsed['read_minutes']
        ]);
    }

    public static function search(string $query, int $limit = 10): array {
        $db = self::db();
        if (self::$stmtSearch === null) {
            self::$stmtSearch = $db->prepare("
                SELECT id, title, slug, category, tags,
                       substr(content_markdown, 1, 160) AS snippet
                FROM articles
                WHERE title LIKE :q OR content_markdown LIKE :q OR tags LIKE :q
                LIMIT :limit
            ");
        }

        $term = '%' . trim($query) . '%';
        self::$stmtSearch->bindValue(':q', $term, PDO::PARAM_STR);
        self::$stmtSearch->bindValue(':limit', $limit, PDO::PARAM_INT);
        self::$stmtSearch->execute();
        return self::$stmtSearch->fetchAll();
    }
}
