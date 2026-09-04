<?php
require __DIR__ . '/../../../example-app/vendor/autoload.php';

use Illuminate\Foundation\Application;

$basePath = realpath(__DIR__ . '/../../../example-app');
$app = Application::configure(basePath: $basePath)->create();

$baseConfigDir = __DIR__ . '/../../../example-app/vendor/laravel/framework/config';
$files = glob($baseConfigDir . '/*.php');
foreach ($files as $file) {
    $name = basename($file);
    echo "Loading framework config: $name ... ";
    $cfg = require $file;
    echo "OK (keys: " . (is_array($cfg) ? count($cfg) : 0) . ")\n";
}
echo "All framework config files loaded successfully!\n";
