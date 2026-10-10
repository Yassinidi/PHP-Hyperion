<?php
declare(strict_types=1);

require_once __DIR__ . '/../vendor/autoload.php';

use App\Kernel;
use Symfony\Component\HttpFoundation\Request;

$kernel = new Kernel($_SERVER['APP_ENV'] ?? 'prod', (bool)($_SERVER['APP_DEBUG'] ?? false));
$kernel->boot();

if (!defined('HYPERION_PERSISTENT')) {
    $request = Request::createFromGlobals();
    $response = $kernel->handle($request);
    $response->send();
    $kernel->terminate($request, $response);
}

return $kernel;
