<?php

require_once __DIR__ . "/composer/ClassLoader.php";

$loader = new \Composer\Autoload\ClassLoader();
$map = require __DIR__ . "/composer/autoload_psr4.php";
foreach ($map as $namespace => $path) {
    $loader->addPsr4($namespace, $path);
}
$loader->register(true);

return $loader;
