<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Codex — High-Performance Markdown Knowledge Engine</title>
    <link rel="stylesheet" href="/style.css">
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;600;700&family=JetBrains+Mono:wght@400;600&display=swap" rel="stylesheet">
</head>
<body>
    <div class="portal-container">
        <header>
            <div class="brand">
                <h1>📚 Codex Documentation Hub</h1>
                <div style="font-size: 0.85rem; color: var(--text-secondary); margin-top: 0.25rem;">
                    Pure PHP In-Memory Markdown Compiler & Full-Text Search
                </div>
            </div>
            <div>
                <span class="badge">Engine: <?= defined('HYPERION_PERSISTENT') ? 'Hyperion Core (Persistent)' : 'Standard PHP 8.4' ?></span>
            </div>
        </header>

        <section class="article-grid">
            <?php foreach ($articles as $a): ?>
            <article class="article-card">
                <div>
                    <div style="font-size: 0.8rem; color: var(--accent-orange); font-weight: 600; text-transform: uppercase; margin-bottom: 0.25rem;">
                        <?= htmlspecialchars($a['category']) ?>
                    </div>
                    <h2 class="article-title" style="margin-top: 0;">
                        <a href="/article/<?= htmlspecialchars($a['slug']) ?>" style="color: inherit; text-decoration: none;">
                            <?= htmlspecialchars($a['title']) ?>
                        </a>
                    </h2>
                    <p class="article-excerpt"><?= htmlspecialchars($a['excerpt']) ?>...</p>
                </div>
                <div>
                    <div class="tags" style="margin-bottom: 0.75rem;">
                        <?php foreach (explode(',', $a['tags']) as $t): ?>
                        <span class="tag">#<?= htmlspecialchars(trim($t)) ?></span>
                        <?php endforeach; ?>
                    </div>
                    <div style="font-size: 0.8rem; color: var(--text-secondary); display: flex; justify-content: space-between;">
                        <span>👁️ <?= $a['views_count'] ?> views</span>
                        <span><?= date('M d, Y', (int)$a['created_at']) ?></span>
                    </div>
                </div>
            </article>
            <?php endforeach; ?>
        </section>
    </div>
</body>
</html>
