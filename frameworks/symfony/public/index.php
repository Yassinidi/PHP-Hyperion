<?php

declare(strict_types=1);

require_once __DIR__ . '/../vendor/autoload.php';

use Symfony\Component\HttpFoundation\Request;
use Symfony\Component\HttpFoundation\JsonResponse;
use Symfony\Component\Routing\Route;
use Symfony\Component\Routing\RouteCollection;
use Symfony\Component\Routing\RequestContext;
use Symfony\Component\Routing\Matcher\UrlMatcher;
use Symfony\Component\Routing\Exception\ResourceNotFoundException;

$request = Request::createFromGlobals();

$routes = new RouteCollection();
$routes->add('home', new Route('/', ['_controller' => function (Request $req) {
    return new JsonResponse([
        'framework' => 'Symfony 7',
        'engine' => 'PHP-Hyperion 8.4',
        'status' => 'success',
        'timestamp' => microtime(true),
    ]);
}]));

$routes->add('user_show', new Route('/users/{id}', ['_controller' => function (Request $req, array $params) {
    return new JsonResponse([
        'framework' => 'Symfony 7',
        'user_id' => (int)($params['id'] ?? 0),
        'name' => 'Developer_' . ($params['id'] ?? 0),
    ]);
}], ['id' => '\d+']));

$context = new RequestContext();
$context->fromRequest($request);
$matcher = new UrlMatcher($routes, $context);

try {
    $parameters = $matcher->match($request->getPathInfo());
    $controller = $parameters['_controller'];
    $response = $controller($request, $parameters);
} catch (ResourceNotFoundException $e) {
    $response = new JsonResponse(['error' => 'Not Found'], 404);
} catch (Exception $e) {
    $response = new JsonResponse(['error' => $e->getMessage()], 500);
}

$response->send();
