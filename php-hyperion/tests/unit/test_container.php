<?php
require __DIR__ . '/../../../example-app/vendor/autoload.php';

use Illuminate\Foundation\Application;
use Illuminate\Foundation\MaintenanceModeManager;
use Illuminate\Contracts\Container\Container;
use Illuminate\Container\Util;

$app = new Application(dirname(dirname(__DIR__)) . '/example-app');

echo "Testing Reflection...\n";
$reflector = new ReflectionClass(MaintenanceModeManager::class);
$constructor = $reflector->getConstructor();
echo "Constructor is: " . ($constructor ? "found" : "NULL") . "\n";
if ($constructor) {
    $params = $constructor->getParameters();
    echo "Params count: " . count($params) . "\n";
    foreach ($params as $p) {
        echo "Param name: " . $p->getName() . "\n";
        $type = $p->getType();
        echo "Param type: " . ($type ? get_class($type) : "NULL") . "\n";
        if ($type) {
            echo "Type name: " . $type->getName() . "\n";
            echo "Is builtin: " . ($type->isBuiltin() ? "yes" : "no") . "\n";
        }
        echo "Declaring class: " . $p->getDeclaringClass()->getName() . "\n";
        $className = Util::getParameterClassName($p);
        echo "Util::getParameterClassName: " . ($className ?? "NULL") . "\n";
    }
}

echo "Testing Container::make(MaintenanceModeManager::class)...\n";
$manager = $app->make(MaintenanceModeManager::class);
echo "Manager created: " . get_class($manager) . "\n";
