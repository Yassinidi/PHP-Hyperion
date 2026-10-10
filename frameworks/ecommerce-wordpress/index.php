<?php
declare(strict_types=1);

define('WP_USE_THEMES', true);

require __DIR__ . '/wp-load.php';

if (!defined('HYPERION_PERSISTENT')) {
    wp();
    require ABSPATH . WPINC . '/template-loader.php';
}
