<?php

declare(strict_types=1);

require_once __DIR__ . '/vendor/autoload.php';

use Symfony\Component\HttpFoundation\Request;
use Symfony\Component\HttpFoundation\Response;
use Symfony\Component\HttpFoundation\JsonResponse;
use Symfony\Component\Routing\Route;
use Symfony\Component\Routing\RouteCollection;
use Symfony\Component\Routing\RequestContext;
use Symfony\Component\Routing\Matcher\UrlMatcher;
use Symfony\Component\DependencyInjection\ContainerBuilder;
use Symfony\Component\DependencyInjection\Reference;
use Symfony\Component\EventDispatcher\EventDispatcher;
use Symfony\Component\EventDispatcher\GenericEvent;
use Symfony\Component\Console\Application as ConsoleApp;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Input\ArrayInput;
use Symfony\Component\Console\Input\InputArgument;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Output\BufferedOutput;
use Symfony\Component\Console\Output\OutputInterface;

echo "=== Symfony 7 Components Test on Hyperion ===\n\n";

// -------------------------------------------------------------
// Test 1: Symfony Routing & URL Matching
// -------------------------------------------------------------
echo "[Test 1] Symfony Routing & URL Matching...\n";

$routes = new RouteCollection();
$routes->add('home', new Route('/', ['_controller' => 'HomeController']));
$routes->add('user_show', new Route('/user/{id}', [
    '_controller' => 'UserController::show',
], ['id' => '\d+']));

$context = new RequestContext();
$context->fromRequest(Request::create('/user/42'));

$matcher = new UrlMatcher($routes, $context);
$parameters = $matcher->match('/user/42');

assert($parameters['_route'] === 'user_show', 'Expected user_show route');
assert($parameters['id'] === '42', 'Expected id=42');
echo "  -> Matched route: {$parameters['_route']} (id={$parameters['id']})\n";
echo "  -> OK!\n\n";

// -------------------------------------------------------------
// Test 2: HttpFoundation Request/Response & JSON serialization
// -------------------------------------------------------------
echo "[Test 2] HttpFoundation Request & JsonResponse...\n";

$request = Request::create(
    '/api/v1/resource',
    'POST',
    [],
    [],
    [],
    ['CONTENT_TYPE' => 'application/json'],
    json_encode(['action' => 'benchmark', 'framework' => 'Symfony 7'])
);

assert($request->getMethod() === 'POST');
assert($request->getContentTypeFormat() === 'json');

$data = json_decode($request->getContent(), true);
assert($data['framework'] === 'Symfony 7');

$response = new JsonResponse([
    'status' => 'success',
    'received' => $data,
    'engine' => 'PHP-Hyperion 8.4',
], 201);
$response->headers->set('X-Custom-Header', 'HyperionValue');

assert($response->getStatusCode() === 201);
assert($response->headers->get('X-Custom-Header') === 'HyperionValue');
echo "  -> Response Status: " . $response->getStatusCode() . " | Content: " . $response->getContent() . "\n";
echo "  -> OK!\n\n";

// -------------------------------------------------------------
// Test 3: DependencyInjection Container
// -------------------------------------------------------------
echo "[Test 3] DependencyInjection Container...\n";

class LoggerService {
    public array $logs = [];
    public function log(string $msg): void {
        $this->logs[] = $msg;
    }
}

class UserService {
    public LoggerService $logger;
    public string $prefix;

    public function __construct(LoggerService $logger, string $prefix = 'USER') {
        $this->logger = $logger;
        $this->prefix = $prefix;
    }

    public function find(int $id): string {
        $this->logger->log("Found user {$id}");
        return "{$this->prefix}_{$id}";
    }
}

$container = new ContainerBuilder();
$container->register('app.logger', LoggerService::class);
$container->register('app.user_service', UserService::class)
    ->addArgument(new Reference('app.logger'))
    ->addArgument('MEMBER')
    ->setPublic(true);

$container->compile();

/** @var UserService $userService */
$userService = $container->get('app.user_service');
$userName = $userService->find(99);

assert($userName === 'MEMBER_99', 'Expected MEMBER_99');
assert($userService->logger->logs[0] === 'Found user 99');
echo "  -> Resolved service result: {$userName} (logs: " . json_encode($userService->logger->logs) . ")\n";
echo "  -> OK!\n\n";

// -------------------------------------------------------------
// Test 4: EventDispatcher & GenericEvent
// -------------------------------------------------------------
echo "[Test 4] EventDispatcher...\n";

$dispatcher = new EventDispatcher();
$dispatched = false;

$dispatcher->addListener('order.placed', function (GenericEvent $event) use (&$dispatched) {
    $dispatched = true;
    $order = $event->getSubject();
    $event->setArgument('processed_by', 'HyperionDispatcher');
});

$orderEvent = new GenericEvent(['id' => 1001, 'total' => 49.99]);
$dispatcher->dispatch($orderEvent, 'order.placed');

assert($dispatched === true, 'Expected event to be dispatched');
assert($orderEvent->getArgument('processed_by') === 'HyperionDispatcher');
echo "  -> Event processed successfully with argument: " . $orderEvent->getArgument('processed_by') . "\n";
echo "  -> OK!\n\n";

// -------------------------------------------------------------
// Test 5: Symfony Console Application & Command
// -------------------------------------------------------------
echo "[Test 5] Symfony Console...\n";

class GreetCommand extends Command {
    protected static ?string $defaultName = 'app:greet';

    protected function configure(): void {
        $this->setName('app:greet')
            ->setDescription('Greets someone')
            ->addArgument('name', InputArgument::OPTIONAL, 'Who do you want to greet?', 'World');
    }

    protected function execute(InputInterface $input, OutputInterface $output): int {
        $name = $input->getArgument('name');
        $output->writeln("Hello, {$name} from Symfony Console on Hyperion!");
        return Command::SUCCESS;
    }
}

$console = new ConsoleApp('HyperionApp', '1.0.0');
$console->setAutoExit(false);
$console->add(new GreetCommand());

$input = new ArrayInput(['command' => 'app:greet', 'name' => 'Developer']);
$output = new BufferedOutput();
$exitCode = $console->run($input, $output);

$outputText = trim($output->fetch());
assert($exitCode === Command::SUCCESS, "Expected exit code 0, got {$exitCode}");
assert(str_contains($outputText, 'Hello, Developer from Symfony Console on Hyperion!'));
echo "  -> Output: {$outputText}\n";
echo "  -> OK!\n\n";

echo "=== ALL SYMFONY 7 TESTS PASSED 100% ===\n";
