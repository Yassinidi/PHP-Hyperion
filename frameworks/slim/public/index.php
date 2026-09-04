<?php

declare(strict_types=1);

require_once __DIR__ . '/../vendor/autoload.php';

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Slim\Factory\AppFactory;
use DI\Container;

$container = new Container();
AppFactory::setContainer($container);
$app = AppFactory::create();

$app->addRoutingMiddleware();
$errorMiddleware = $app->addErrorMiddleware(true, true, true);

// Endpoint 1: Root Ping
$app->get('/', function (Request $request, Response $response) {
    $data = [
        'framework' => 'Slim 4',
        'engine' => 'PHP-Hyperion 8.4',
        'status' => 'active',
        'timestamp' => microtime(true),
    ];
    $payload = json_encode($data);
    $response->getBody()->write($payload);
    return $response->withHeader('Content-Type', 'application/json');
});

// Endpoint 2: Route with Regex Parameter
$app->get('/users/{id:[0-9]+}', function (Request $request, Response $response, array $args) {
    $id = (int)$args['id'];
    $data = [
        'id' => $id,
        'name' => "User_{$id}",
        'role' => 'developer',
        'engine' => 'PHP-Hyperion 8.4',
    ];
    $payload = json_encode($data);
    $response->getBody()->write($payload);
    return $response->withHeader('Content-Type', 'application/json');
});

// Endpoint 3: JSON POST
$app->post('/users', function (Request $request, Response $response) {
    $body = (string)$request->getBody();
    $input = json_decode($body, true) ?: [];
    $data = [
        'id' => rand(100, 999),
        'name' => $input['name'] ?? 'Anonymous',
        'role' => $input['role'] ?? 'guest',
        'created' => true,
    ];
    $payload = json_encode($data);
    $response->getBody()->write($payload);
    return $response->withHeader('Content-Type', 'application/json')->withStatus(201);
});

$app->run();
