<?php

use Illuminate\Contracts\Http\Kernel as HttpKernelContract;
use Illuminate\Http\Request;

define('LARAVEL_START', microtime(true));

$t0 = microtime(true);
require __DIR__.'/vendor/autoload.php';
$t1 = microtime(true);
$app = require __DIR__.'/bootstrap/app.php';
$t2 = microtime(true);
$kernel = $app->make(HttpKernelContract::class);
try {
    $kernel->bootstrap();
} catch (\Throwable $e) {
}
$t3 = microtime(true);

if (function_exists('hyperion_route_register')) {
    $router = $app->make('router');
    foreach ($router->getRoutes()->getRoutes() as $route) {
        foreach ($route->methods() as $m) {
            if ($m !== 'HEAD') {
                hyperion_route_register($m, $route->uri(), 0, $route->getName() ?? '');
            }
        }
    }
}



file_put_contents('/tmp/worker_boot.log', sprintf("Boot completed: Autoload: %.3fs, App: %.3fs, Kernel: %.3fs, Total: %.3fs\n", $t1 - $t0, $t2 - $t1, $t3 - $t2, $t3 - $t0));

while (hyperion_accept_request()) {
    $uri = $_SERVER['REQUEST_URI'] ?? '/';
    $method = $_SERVER['REQUEST_METHOD'] ?? 'GET';
    $path = parse_url($uri, PHP_URL_PATH) ?? '/';

    $request = Request::capture();
    $app->instance('request', $request);

   
        // Full Laravel Kernel pipeline fallback
        $response = $kernel->handle($request);
        $response->send();
        $kernel->terminate($request, $response);
    

    if (method_exists($app, 'forgetInstance')) {
        $app->forgetInstance('request');
        $app->forgetInstance('session');
        $app->forgetInstance('session.store');
        $app->forgetInstance('auth');
    }
    if (class_exists(\Illuminate\Database\Eloquent\Model::class)) {
        \Illuminate\Database\Eloquent\Model::clearBootedModels();
    }
}

