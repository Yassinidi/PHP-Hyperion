<?php
declare(strict_types=1);

require_once __DIR__ . '/../src/Database.php';
require_once __DIR__ . '/../src/MarkdownParser.php';
require_once __DIR__ . '/../src/CmsService.php';

use CmsMarkdown\CmsService;

$uri = parse_url($_SERVER['REQUEST_URI'] ?? '/', PHP_URL_PATH);
$method = $_SERVER['REQUEST_METHOD'] ?? 'GET';

// 1. Health Probe
if ($uri === '/health') {
    header('Content-Type: application/json; charset=utf-8');
    echo json_encode([
        'status' => 'ok',
        'project' => 'cms-markdown',
        'timestamp' => microtime(true),
        'engine' => defined('HYPERION_PERSISTENT') ? 'hyperion-core-2.0' : 'standard-php'
    ]);
    return;
}

// 2. Web Portal Home
if ($uri === '/' && $method === 'GET') {
    $articles = CmsService::getArticles(20);
    require __DIR__ . '/../src/Views/portal.php';
    return;
}

// 3. Render Article by Slug: /article/{slug}
if (preg_match('#^/article/([a-zA-Z0-9_-]+)$#', $uri, $matches) && $method === 'GET') {
    $article = CmsService::getArticle($matches[1]);
    if (!$article) {
        http_response_code(404);
        echo "<h1>404 Article Not Found</h1>";
        return;
    }

    header('Content-Type: text/html; charset=utf-8');
    ?>
    <!DOCTYPE html>
    <html lang="en">
    <head>
        <meta charset="UTF-8">
        <title><?= htmlspecialchars($article['title']) ?> — Codex</title>
        <link rel="stylesheet" href="/style.css">
    </head>
    <body>
        <div class="portal-container" style="max-width: 800px;">
            <a href="/" style="color: var(--accent-orange); text-decoration: none; font-size: 0.9rem;">← Back to Documentation</a>
            <div style="margin: 1.5rem 0;">
                <span class="badge"><?= htmlspecialchars($article['category']) ?></span>
                <span style="font-size: 0.85rem; color: var(--text-secondary); margin-left: 0.5rem;">⏱️ <?= $article['read_minutes'] ?> min read (<?= $article['word_count'] ?> words)</span>
            </div>
            
            <?php if (!empty($article['toc'])): ?>
            <div style="background: rgba(255,255,255,0.03); border: 1px solid var(--border); border-radius: 8px; padding: 1rem; margin-bottom: 2rem;">
                <div style="font-weight: 600; font-size: 0.9rem; margin-bottom: 0.5rem; color: var(--accent-amber);">Table of Contents</div>
                <ul style="padding-left: 1.25rem;">
                    <?php foreach ($article['toc'] as $item): ?>
                    <li><a href="#<?= $item['slug'] ?>" style="color: var(--text-primary); text-decoration: none;"><?= htmlspecialchars($item['title']) ?></a></li>
                    <?php endforeach; ?>
                </ul>
            </div>
            <?php endif; ?>

            <div class="article-body">
                <?= $article['html'] ?>
            </div>
        </div>
    </body>
    </html>
    <?php
    return;
}

// -------------------------------------------------------------
// REST API
// -------------------------------------------------------------
header('Content-Type: application/json; charset=utf-8');

// 4. GET /api/articles
if ($uri === '/api/articles' && $method === 'GET') {
    echo json_encode([
        'success' => true,
        'data' => CmsService::getArticles(20)
    ]);
    return;
}

// 5. GET /api/search?q={query}
if ($uri === '/api/search' && $method === 'GET') {
    $q = (string)($_GET['q'] ?? '');
    echo json_encode([
        'success' => true,
        'query' => $q,
        'data' => CmsService::search($q)
    ]);
    return;
}

// 404 Fallback
http_response_code(404);
echo json_encode(['success' => false, 'error' => 'Route not found: ' . $uri]);
