<?php

declare(strict_types=1);

// Mock WordPress Minimal Runtime for HTTP Serving
global $wp_filter, $wp_actions, $wpdb;
$wp_filter = [];
$wp_actions = [];

function add_filter(string $hook_name, callable $callback, int $priority = 10, int $accepted_args = 1): bool {
    global $wp_filter;
    $wp_filter[$hook_name][$priority][] = [
        'function' => $callback,
        'accepted_args' => $accepted_args,
    ];
    return true;
}

function add_action(string $hook_name, callable $callback, int $priority = 10, int $accepted_args = 1): bool {
    return add_filter($hook_name, $callback, $priority, $accepted_args);
}

function apply_filters(string $hook_name, mixed $value, mixed ...$args): mixed {
    global $wp_filter;
    if (!isset($wp_filter[$hook_name])) {
        return $value;
    }
    ksort($wp_filter[$hook_name]);
    foreach ($wp_filter[$hook_name] as $priority => $callbacks) {
        foreach ($callbacks as $item) {
            $cb = $item['function'];
            $num_args = $item['accepted_args'];
            $call_args = array_slice(array_merge([$value], $args), 0, $num_args);
            $value = call_user_func_array($cb, $call_args);
        }
    }
    return $value;
}

class wpdb {
    public PDO $pdo;
    public string $prefix = 'wp_';
    public string $posts = 'wp_posts';

    public function __construct() {
        $this->pdo = new PDO('sqlite::memory:');
        $this->pdo->setAttribute(PDO::ATTR_DEFAULT_FETCH_MODE, PDO::FETCH_OBJ);
        $this->init();
    }

    private function init(): void {
        $this->pdo->exec("
            CREATE TABLE wp_posts (
                ID INTEGER PRIMARY KEY AUTOINCREMENT,
                post_title TEXT NOT NULL,
                post_content TEXT NOT NULL,
                post_status TEXT DEFAULT 'publish'
            );
            INSERT INTO wp_posts (post_title, post_content) VALUES
                ('Hello Hyperion', 'Welcome to WordPress on PHP-Hyperion!'),
                ('Blazing Fast Execution', 'Microsecond response times on modern hardware.');
        ");
    }

    public function get_results(string $query): array {
        return $this->pdo->query($query)->fetchAll();
    }
}

$wpdb = new wpdb();

add_filter('the_title', function ($title) {
    return "WP &bull; " . $title;
});

// Render WordPress Posts
$rows = $wpdb->get_results("SELECT * FROM wp_posts WHERE post_status = 'publish'");
$posts = [];
foreach ($rows as $r) {
    $posts[] = [
        'id' => $r->ID,
        'title' => apply_filters('the_title', $r->post_title),
        'content' => $r->post_content,
    ];
}

header('Content-Type: application/json');
echo json_encode([
    'platform' => 'WordPress 6.x Simulation',
    'engine' => 'PHP-Hyperion 8.4',
    'post_count' => count($posts),
    'posts' => $posts,
]);
