<?php

declare(strict_types=1);

echo "=== WordPress Core Architecture Test on Hyperion ===\n\n";

// -------------------------------------------------------------
// Part 1: WordPress Hook System (Actions & Filters)
// -------------------------------------------------------------
echo "[Test 1] WordPress Hook System (Actions & Filters)...\n";

global $wp_filter, $wp_actions;
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

function do_action(string $hook_name, mixed ...$args): void {
    global $wp_filter, $wp_actions;
    $wp_actions[$hook_name] = ($wp_actions[$hook_name] ?? 0) + 1;
    if (!isset($wp_filter[$hook_name])) {
        return;
    }

    ksort($wp_filter[$hook_name]);
    foreach ($wp_filter[$hook_name] as $priority => $callbacks) {
        foreach ($callbacks as $item) {
            $cb = $item['function'];
            $num_args = $item['accepted_args'];
            $call_args = array_slice($args, 0, $num_args);
            call_user_func_array($cb, $call_args);
        }
    }
}

// Test Actions
$action_log = [];
add_action('init', function () use (&$action_log) {
    $action_log[] = 'init_priority_10';
}, 10);

add_action('init', function () use (&$action_log) {
    $action_log[] = 'init_priority_5';
}, 5);

do_action('init');
assert($action_log === ['init_priority_5', 'init_priority_10'], 'Actions must run in priority order');

// Test Filters
add_filter('the_title', function ($title) {
    return 'Prefix: ' . $title;
}, 10);

add_filter('the_title', function ($title, $post_id) {
    return $title . " (ID: {$post_id})";
}, 20, 2);

$filtered_title = apply_filters('the_title', 'Hello World', 42);
assert($filtered_title === 'Prefix: Hello World (ID: 42)', 'Filter output mismatch: ' . $filtered_title);
echo "  -> Action log: " . json_encode($action_log) . "\n";
echo "  -> Filtered title: {$filtered_title}\n";
echo "  -> OK!\n\n";

// -------------------------------------------------------------
// Part 2: Database Abstraction (wpdb with PDO SQLite)
// -------------------------------------------------------------
echo "[Test 2] wpdb Database Abstraction Layer (SQLite)...\n";

class wpdb {
    public PDO $pdo;
    public string $prefix = 'wp_';
    public string $posts = 'wp_posts';
    public string $options = 'wp_options';
    public int $insert_id = 0;

    public function __construct() {
        $this->pdo = new PDO('sqlite::memory:');
        $this->pdo->setAttribute(PDO::ATTR_ERRMODE, PDO::ERRMODE_EXCEPTION);
        $this->pdo->setAttribute(PDO::ATTR_DEFAULT_FETCH_MODE, PDO::FETCH_OBJ);
        $this->init_tables();
    }

    private function init_tables(): void {
        $this->pdo->exec("
            CREATE TABLE wp_posts (
                ID INTEGER PRIMARY KEY AUTOINCREMENT,
                post_author INTEGER DEFAULT 1,
                post_title TEXT NOT NULL,
                post_content TEXT NOT NULL,
                post_status TEXT DEFAULT 'publish',
                post_type TEXT DEFAULT 'post',
                post_date TEXT DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE wp_options (
                option_id INTEGER PRIMARY KEY AUTOINCREMENT,
                option_name TEXT UNIQUE NOT NULL,
                option_value TEXT NOT NULL,
                autoload TEXT DEFAULT 'yes'
            );
        ");
    }

    public function prepare(string $query, mixed ...$args): string {
        if (count($args) === 1 && is_array($args[0])) {
            $args = $args[0];
        }
        foreach ($args as $arg) {
            $pos_s = strpos($query, '%s');
            $pos_d = strpos($query, '%d');
            
            if ($pos_s !== false && ($pos_d === false || $pos_s < $pos_d)) {
                $quoted = "'" . str_replace("'", "''", (string)$arg) . "'";
                $query = substr_replace($query, $quoted, $pos_s, 2);
            } elseif ($pos_d !== false) {
                $query = substr_replace($query, (string)(int)$arg, $pos_d, 2);
            }
        }
        return $query;
    }

    public function get_results(string $query): array {
        $stmt = $this->pdo->query($query);
        return $stmt->fetchAll();
    }

    public function get_row(string $query): ?object {
        $stmt = $this->pdo->query($query);
        $row = $stmt->fetch();
        return $row ?: null;
    }

    public function insert(string $table, array $data): int|false {
        $cols = implode(', ', array_keys($data));
        $placeholders = implode(', ', array_fill(0, count($data), '?'));
        $stmt = $this->pdo->prepare("INSERT INTO {$table} ({$cols}) VALUES ({$placeholders})");
        $stmt->execute(array_values($data));
        $this->insert_id = (int)$this->pdo->lastInsertId();
        return 1;
    }

    public function update(string $table, array $data, array $where): int|false {
        $set_parts = [];
        $values = [];
        foreach ($data as $col => $val) {
            $set_parts[] = "{$col} = ?";
            $values[] = $val;
        }
        $where_parts = [];
        foreach ($where as $col => $val) {
            $where_parts[] = "{$col} = ?";
            $values[] = $val;
        }
        $sql = "UPDATE {$table} SET " . implode(', ', $set_parts) . " WHERE " . implode(' AND ', $where_parts);
        $stmt = $this->pdo->prepare($sql);
        $stmt->execute($values);
        return $stmt->rowCount();
    }
}

global $wpdb;
$wpdb = new wpdb();

// Insert posts
$wpdb->insert($wpdb->posts, [
    'post_title' => 'First Post on Hyperion',
    'post_content' => 'Welcome to WordPress running blazing fast on PHP-Hyperion!',
    'post_status' => 'publish',
    'post_type' => 'post',
]);
$post1_id = $wpdb->insert_id;

$wpdb->insert($wpdb->posts, [
    'post_title' => 'High Performance Engine',
    'post_content' => 'AOT compilation delivers microsecond response times.',
    'post_status' => 'publish',
    'post_type' => 'post',
]);
$post2_id = $wpdb->insert_id;

$prepared_query = $wpdb->prepare("SELECT * FROM {$wpdb->posts} WHERE ID = %d", $post1_id);
$fetched_post = $wpdb->get_row($prepared_query);

assert($fetched_post !== null);
assert($fetched_post->post_title === 'First Post on Hyperion');
echo "  -> Created and fetched Post ID {$post1_id}: {$fetched_post->post_title}\n";
echo "  -> OK!\n\n";

// -------------------------------------------------------------
// Part 3: Options & Transients API
// -------------------------------------------------------------
echo "[Test 3] Options & Transients API...\n";

function get_option(string $option, mixed $default = false): mixed {
    global $wpdb;
    $row = $wpdb->get_row($wpdb->prepare("SELECT option_value FROM {$wpdb->options} WHERE option_name = %s", $option));
    if ($row) {
        $val = $row->option_value;
        $unserialized = @unserialize($val);
        return $unserialized !== false || $val === 'b:0;' ? $unserialized : $val;
    }
    return $default;
}

function update_option(string $option, mixed $value): bool {
    global $wpdb;
    $serialized = is_scalar($value) && !is_bool($value) ? (string)$value : serialize($value);
    $existing = get_option($option, null);
    if ($existing === null) {
        $wpdb->insert($wpdb->options, [
            'option_name' => $option,
            'option_value' => $serialized,
            'autoload' => 'yes',
        ]);
    } else {
        $wpdb->update($wpdb->options, ['option_value' => $serialized], ['option_name' => $option]);
    }
    return true;
}

function set_transient(string $transient, mixed $value, int $expiration = 0): bool {
    return update_option('_transient_' . $transient, $value);
}

function get_transient(string $transient): mixed {
    return get_option('_transient_' . $transient, false);
}

update_option('blogname', 'Hyperion Powered Blog');
update_option('active_plugins', ['hyperion-cache/cache.php', 'seo/seo.php']);
set_transient('site_stats', ['visitors' => 54000, 'rps' => 12500]);

assert(get_option('blogname') === 'Hyperion Powered Blog');
assert(get_option('active_plugins') === ['hyperion-cache/cache.php', 'seo/seo.php']);
$stats = get_transient('site_stats');
assert($stats['rps'] === 12500);

echo "  -> Option 'blogname': " . get_option('blogname') . "\n";
echo "  -> Option 'active_plugins': " . json_encode(get_option('active_plugins')) . "\n";
echo "  -> Transient 'site_stats': " . json_encode($stats) . "\n";
echo "  -> OK!\n\n";

// -------------------------------------------------------------
// Part 4: WP_Query and The Loop
// -------------------------------------------------------------
echo "[Test 4] WP_Query and The Loop...\n";

class WP_Post {
    public int $ID;
    public string $post_title;
    public string $post_content;
    public string $post_status;
    public string $post_type;

    public function __construct(object $db_row) {
        $this->ID = (int)$db_row->ID;
        $this->post_title = $db_row->post_title;
        $this->post_content = $db_row->post_content;
        $this->post_status = $db_row->post_status;
        $this->post_type = $db_row->post_type;
    }
}

class WP_Query {
    public array $posts = [];
    public int $post_count = 0;
    public int $current_post = -1;
    public ?WP_Post $post = null;

    public function __construct(array $args = []) {
        $this->query($args);
    }

    public function query(array $args): array {
        global $wpdb;
        $post_type = $args['post_type'] ?? 'post';
        $post_status = $args['post_status'] ?? 'publish';
        
        $sql = $wpdb->prepare(
            "SELECT * FROM {$wpdb->posts} WHERE post_type = %s AND post_status = %s ORDER BY ID DESC",
            $post_type,
            $post_status
        );
        $rows = $wpdb->get_results($sql);
        $this->posts = array_map(fn($r) => new WP_Post($r), $rows);
        $this->post_count = count($this->posts);
        return $this->posts;
    }

    public function have_posts(): bool {
        return ($this->current_post + 1) < $this->post_count;
    }

    public function the_post(): WP_Post {
        $this->current_post++;
        $this->post = $this->posts[$this->current_post];
        $GLOBALS['post'] = $this->post;
        return $this->post;
    }
}

$query = new WP_Query(['post_type' => 'post', 'post_status' => 'publish']);
$rendered_posts = [];

while ($query->have_posts()) {
    $p = $query->the_post();
    $rendered_posts[] = [
        'id' => $p->ID,
        'title' => apply_filters('the_title', $p->post_title, $p->ID),
        'content' => $p->post_content,
    ];
}

assert(count($rendered_posts) === 2);
echo "  -> Found " . count($rendered_posts) . " published posts in The Loop.\n";
foreach ($rendered_posts as $rp) {
    echo "     * [ID {$rp['id']}] {$rp['title']}\n";
}
echo "  -> OK!\n\n";

// -------------------------------------------------------------
// Part 5: Shortcode API & Sanitization
// -------------------------------------------------------------
echo "[Test 5] Shortcode API & Sanitization...\n";

global $shortcode_tags;
$shortcode_tags = [];

function add_shortcode(string $tag, callable $callback): void {
    global $shortcode_tags;
    $shortcode_tags[$tag] = $callback;
}

function do_shortcode(string $content): string {
    global $shortcode_tags;
    if (empty($shortcode_tags)) {
        return $content;
    }
    
    $tagregex = implode('|', array_map('preg_quote', array_keys($shortcode_tags)));
    $pattern = '/\[(' . $tagregex . ')(.*?)\](?:(.*?)\[\/\1\])?/s';

    return preg_replace_callback($pattern, function ($m) use ($shortcode_tags) {
        $tag = $m[1];
        $attr_str = trim($m[2] ?? '');
        $inner_content = $m[3] ?? null;
        
        $atts = [];
        if (!empty($attr_str)) {
            preg_match_all('/(\w+)\s*=\s*["\']([^"\']*)["\']/', $attr_str, $matches, PREG_SET_ORDER);
            foreach ($matches as $match) {
                $atts[$match[1]] = $match[2];
            }
        }
        
        $cb = $shortcode_tags[$tag];
        return (string)call_user_func($cb, $atts, $inner_content, $tag);
    }, $content);
}

function sanitize_text_field(string $str): string {
    $filtered = strip_tags($str);
    return trim(preg_replace('/[\r\n\t ]+/', ' ', $filtered));
}

add_shortcode('cta', function ($atts, $content = null) {
    $btn_text = $atts['text'] ?? 'Click Here';
    $url = $atts['url'] ?? '#';
    return "<a href=\"{$url}\" class=\"btn\">{$btn_text}: {$content}</a>";
});

$raw_input = "Check out our new engine [cta text=\"Upgrade Now\" url=\"https://hyperion.dev\"]Fast & Reliable[/cta] today!";
$shortcode_output = do_shortcode($raw_input);

assert(str_contains($shortcode_output, '<a href="https://hyperion.dev" class="btn">Upgrade Now: Fast & Reliable</a>'));
assert(sanitize_text_field("  <b>Clean</b>   string\t\n") === 'Clean string');

echo "  -> Shortcode processed: {$shortcode_output}\n";
echo "  -> Sanitized string: " . sanitize_text_field("  <b>Clean</b>   string\t\n") . "\n";
echo "  -> OK!\n\n";

echo "=== ALL WORDPRESS TESTS PASSED 100% ===\n";
