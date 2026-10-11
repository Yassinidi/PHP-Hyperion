<?php
declare(strict_types=1);

namespace CmsMarkdown;

final class MarkdownParser {
    public static function parse(string $markdown): array {
        $toc = [];
        $html = htmlspecialchars($markdown, ENT_NOQUOTES, 'UTF-8');

        // 1. Code blocks (```language ... ```)
        $html = preg_replace_callback('/```([a-zA-Z0-9_-]*)\n(.*?)\n```/s', function($matches) {
            $lang = htmlspecialchars($matches[1] ?: 'text');
            $code = $matches[2];
            return "<pre><code class=\"language-{$lang}\">{$code}</code></pre>";
        }, $html);

        // 2. Headings with TOC collection (#, ##, ###)
        $html = preg_replace_callback('/^(#{1,3})\s+(.+)$/m', function($matches) use (&$toc) {
            $level = strlen($matches[1]);
            $title = trim($matches[2]);
            $slug = strtolower(preg_replace('/[^a-zA-Z0-9]+/', '-', $title));
            $slug = trim($slug, '-');

            $toc[] = [
                'level' => $level,
                'title' => $title,
                'slug'  => $slug
            ];

            return "<h{$level} id=\"{$slug}\">{$title}</h{$level}>";
        }, $html);

        // 3. Blockquotes
        $html = preg_replace('/^&gt;\s+(.+)$/m', '<blockquote><p>$1</p></blockquote>', $html);

        // 4. Unordered Lists
        $html = preg_replace('/^\-\s+(.+)$/m', '<li>$1</li>', $html);
        $html = preg_replace('/(<li>.*<\/li>(\n)?)+/s', '<ul>$0</ul>', $html);

        // 5. Inline code
        $html = preg_replace('/`([^`]+)`/', '<code>$1</code>', $html);

        // 6. Bold & Italic
        $html = preg_replace('/\*\*([^*]+)\*\*/', '<strong>$1</strong>', $html);
        $html = preg_replace('/\*([^*]+)\*/', '<em>$1</em>', $html);

        // 7. Paragraphs
        $paragraphs = explode("\n\n", $html);
        $cleanParas = [];
        foreach ($paragraphs as $p) {
            $trimmed = trim($p);
            if ($trimmed === '') continue;
            if (preg_match('/^<(h[1-6]|ul|pre|blockquote)/', $trimmed)) {
                $cleanParas[] = $trimmed;
            } else {
                $cleanParas[] = "<p>{$trimmed}</p>";
            }
        }
        $finalHtml = implode("\n", $cleanParas);

        $wordCount = str_word_count(strip_tags($finalHtml));
        $readMinutes = max(1, (int)ceil($wordCount / 200));

        return [
            'html'         => $finalHtml,
            'toc'          => $toc,
            'word_count'   => $wordCount,
            'read_minutes' => $readMinutes
        ];
    }
}
