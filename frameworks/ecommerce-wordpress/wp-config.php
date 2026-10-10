<?php
declare(strict_types=1);

define('DB_NAME', 'ecommerce_wp');
define('DB_USER', 'root');
define('DB_PASSWORD', '');
define('DB_HOST', 'localhost');
define('DB_CHARSET', 'utf8mb4');
define('DB_COLLATE', '');

if (!defined('ABSPATH')) {
    define('ABSPATH', __DIR__ . '/');
}
if (!defined('WPINC')) {
    define('WPINC', 'wp-includes');
}

require_once ABSPATH . WPINC . '/wp-db.php';

global $wpdb;
$wpdb = new wpdb(ABSPATH . 'wp-content/database/ecommerce_wp.sqlite');

$table_prefix = 'wp_';

define('WP_DEBUG', false);
