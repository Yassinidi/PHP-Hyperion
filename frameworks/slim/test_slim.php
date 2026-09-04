<?php

require_once __DIR__ . '/vendor/autoload.php';

use DI\ContainerBuilder;
use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Psr\Http\Server\RequestHandlerInterface as RequestHandler;
use Slim\Factory\AppFactory;
use Slim\Psr7\Factory\ServerRequestFactory;
use Slim\Psr7\Factory\StreamFactory;

echo "=== Slim Framework 4 Test on Hyperion ===\n";

// 1. Build PHP-DI Container
$containerBuilder = new ContainerBuilder();
$containerBuilder->addDefinitions([
    'db.config' => [
        'driver' => 'sqlite',
        'database' => ':memory:',
    ],
    'user.service' => function () {
        return new class {
            public function getUser(int $id): array {
                return ['id' => $id, 'name' => "User_{$id}", 'role' => 'developer'];
            }
        };
    }
]);
$container = $containerBuilder->build();
AppFactory::setContainer($container);

// 2. Create Slim App
$app = AppFactory::create();

// 3. Add Custom Middleware (Timing & Request Header)
$app->add(function (Request $request, RequestHandler $handler): Response {
    $start = microtime(true);
    $response = $handler->handle($request);
    $elapsed = round((microtime(true) - $start) * 1000, 3);
    return $response
        ->withHeader('X-Response-Time', "{$elapsed}ms")
        ->withHeader('X-Powered-By', 'Hyperion-VM');
});

// 4. Register Routes
$app->get('/', function (Request $request, Response $response): Response {
    $payload = json_encode(['framework' => 'Slim 4', 'engine' => 'PHP-Hyperion', 'status' => 'active']);
    $response->getBody()->write($payload);
    return $response->withHeader('Content-Type', 'application/json');
});

$app->get('/users/{id:[0-9]+}', function (Request $request, Response $response, array $args): Response {
    $id = (int)$args['id'];
    $userService = $this->get('user.service');
    $user = $userService->getUser($id);
    $response->getBody()->write(json_encode($user));
    return $response->withHeader('Content-Type', 'application/json');
});

$app->post('/users', function (Request $request, Response $response): Response {
    $data = (array)$request->getParsedBody();
    $result = [
        'id' => 42,
        'name' => $data['name'] ?? 'Anonymous',
        'created_at' => date('Y-m-d H:i:s'),
    ];
    $response->getBody()->write(json_encode($result));
    return $response->withStatus(201)->withHeader('Content-Type', 'application/json');
});

$app->addRoutingMiddleware();
$app->addErrorMiddleware(true, true, true);

// 5. Test Suite Assertions

// Test 1: GET /
echo "\n[Test 1] Dispatching GET /...\n";
$req1 = (new ServerRequestFactory())->createServerRequest('GET', '/');
$res1 = $app->handle($req1);
assert($res1->getStatusCode() === 200, "Expected status 200");
assert($res1->getHeaderLine('X-Powered-By') === 'Hyperion-VM', "Expected custom middleware header");
$body1 = json_decode((string)$res1->getBody(), true);
assert($body1['framework'] === 'Slim 4', "Expected framework Slim 4");
echo "  -> Status: " . $res1->getStatusCode() . " | Body: " . (string)$res1->getBody() . "\n";
echo "  -> OK!\n";

// Test 2: GET /users/123 (with regex param and DI resolution)
echo "\n[Test 2] Dispatching GET /users/123...\n";
$req2 = (new ServerRequestFactory())->createServerRequest('GET', '/users/123');
$res2 = $app->handle($req2);
assert($res2->getStatusCode() === 200, "Expected status 200");
$body2 = json_decode((string)$res2->getBody(), true);
assert($body2['id'] === 123, "Expected user id 123");
assert($body2['name'] === 'User_123', "Expected user name User_123");
echo "  -> Status: " . $res2->getStatusCode() . " | User: " . (string)$res2->getBody() . "\n";
echo "  -> OK!\n";

// Test 3: POST /users with JSON parsed body
echo "\n[Test 3] Dispatching POST /users...\n";
$stream = (new StreamFactory())->createStream(json_encode(['name' => 'Alice']));
$req3 = (new ServerRequestFactory())
    ->createServerRequest('POST', '/users')
    ->withHeader('Content-Type', 'application/json')
    ->withParsedBody(['name' => 'Alice'])
    ->withBody($stream);
$res3 = $app->handle($req3);
assert($res3->getStatusCode() === 201, "Expected status 201");
$body3 = json_decode((string)$res3->getBody(), true);
assert($body3['id'] === 42, "Expected id 42");
assert($body3['name'] === 'Alice', "Expected Alice");
echo "  -> Status: " . $res3->getStatusCode() . " | Result: " . (string)$res3->getBody() . "\n";
echo "  -> OK!\n";

// Test 4: 404 Not Found
echo "\n[Test 4] Dispatching GET /non-existent-route (404)...\n";
$req4 = (new ServerRequestFactory())->createServerRequest('GET', '/non-existent-route');
$res4 = $app->handle($req4);
assert($res4->getStatusCode() === 404, "Expected status 404");
echo "  -> Status: " . $res4->getStatusCode() . "\n";
echo "  -> OK!\n";

echo "\n=== ALL SLIM 4 TESTS PASSED 100% ===\n";
