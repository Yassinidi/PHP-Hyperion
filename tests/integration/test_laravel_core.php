<?php
declare(strict_types=1);

/**
 * Tier 2: Laravel Core Subsystems Battery
 * Comprehensive verification of all 10 major Laravel subsystems running on Hyperion.
 */

// Dynamic Framework Path Discovery
$candidatePaths = [
    __DIR__ . '/../../frameworks/ecommerce-laravel',
    dirname(__DIR__, 2) . '/frameworks/ecommerce-laravel',
    __DIR__ . '/../../frameworks/laravel-full',
    __DIR__ . '/../frameworks/laravel-full',
    dirname(__DIR__, 2) . '/frameworks/laravel-full',
    dirname(__DIR__, 2) . '/frameworks/laravel-complex'
];

$frameworkPath = null;
foreach ($candidatePaths as $p) {
    if (file_exists($p . '/vendor/autoload.php') && file_exists($p . '/bootstrap/app.php')) {
        $frameworkPath = realpath($p);
        break;
    }
}

if (!$frameworkPath) {
    fwrite(STDERR, "❌ ERROR: Could not locate Laravel installation with vendor/autoload.php\n");
    exit(1);
}

// Ensure working directory is set to framework path
chdir($frameworkPath);

// Ensure storage framework cache dirs exist
@mkdir($frameworkPath . '/storage/framework/views', 0777, true);
@mkdir($frameworkPath . '/storage/framework/sessions', 0777, true);
@mkdir($frameworkPath . '/storage/framework/cache', 0777, true);

require_once $frameworkPath . '/vendor/autoload.php';
$app = require_once $frameworkPath . '/bootstrap/app.php';
$kernel = $app->make(\Illuminate\Contracts\Console\Kernel::class);
$kernel->bootstrap();

// Ensure database SQLite path is dynamically resolved to current location
$dbPath = $frameworkPath . '/database/database.sqlite';
if (!file_exists($dbPath)) {
    @touch($dbPath);
}
config(['database.connections.sqlite.database' => $dbPath]);
\Illuminate\Support\Facades\DB::purge('sqlite');

$GLOBALS['passed'] = 0;
$GLOBALS['total'] = 0;

function assertSubsystem(string $name, callable $test): void {
    $GLOBALS['total']++;
    echo "[TEST " . sprintf("%02d", $GLOBALS['total']) . "] {$name}... ";
    try {
        $test();
        echo "\033[32mPASSED ✓\033[0m\n";
        $GLOBALS['passed']++;
    } catch (\Throwable $e) {
        echo "\033[31mFAILED ✗\033[0m: " . $e->getMessage() . " (" . basename($e->getFile()) . ":" . $e->getLine() . ")\n";
    }
}

echo "====================================================================\n";
echo " 🚀 LARAVEL CORE SUBSYSTEMS INTEGRATION BATTERY\n";
echo "====================================================================\n";
echo "Framework Path : " . $frameworkPath . "\n";
echo "Framework Ver  : Laravel " . $app->version() . "\n";
echo "PHP Runtime    : " . PHP_VERSION . " (" . PHP_SAPI . ")\n";
echo "====================================================================\n\n";

// 1. Service Container & DI
assertSubsystem("Service Container & Dependency Injection", function () use ($app) {
    $app->bind('test.service', fn() => new class { public function hello() { return 'world'; } });
    $svc = $app->make('test.service');
    if ($svc->hello() !== 'world') {
        throw new \Exception("Container did not resolve binding correctly");
    }

    $app->singleton('test.singleton', fn() => new \stdClass());
    $s1 = $app->make('test.singleton');
    $s2 = $app->make('test.singleton');
    if ($s1 !== $s2) {
        throw new \Exception("Singleton resolved different instances");
    }
});

// 2. Routing & HTTP Pipeline
assertSubsystem("Routing & HTTP Middleware Pipeline", function () use ($app) {
    \Illuminate\Support\Facades\Route::get('/hyperion-core-ping', function () {
        return response()->json(['engine' => 'hyperion', 'status' => 'operational']);
    });

    $request = \Illuminate\Http\Request::create('/hyperion-core-ping', 'GET');
    $response = $app->handle($request);

    if ($response->getStatusCode() !== 200) {
        throw new \Exception("Expected status 200, got: " . $response->getStatusCode());
    }

    $content = json_decode($response->getContent(), true);
    if (!isset($content['engine']) || $content['engine'] !== 'hyperion') {
        throw new \Exception("Unexpected response payload: " . $response->getContent());
    }
});

// 3. Database & Eloquent Query Builder
assertSubsystem("Database Connection & Eloquent Builder", function () {
    $res = \Illuminate\Support\Facades\DB::select('SELECT 1 as val');
    if (empty($res) || (int)$res[0]->val !== 1) {
        throw new \Exception("Database raw query failed");
    }
});

// 4. Blade Templating Engine
assertSubsystem("Blade Templating & Template Rendering", function () {
    $rendered = \Illuminate\Support\Facades\Blade::render(
        'Hello, {{ $name }}! 1 + 1 = {{ 1 + 1 }}',
        ['name' => 'Hyperion Engine']
    );

    if ($rendered !== 'Hello, Hyperion Engine! 1 + 1 = 2') {
        throw new \Exception("Blade rendering output mismatch: '{$rendered}'");
    }
});

// 5. Event Dispatcher & Listeners
assertSubsystem("Event Dispatcher & Asynchronous Listeners", function () {
    $invoked = false;
    \Illuminate\Support\Facades\Event::listen('hyperion.test.event', function ($payload) use (&$invoked) {
        if ($payload === 'payload_ok') {
            $invoked = true;
        }
    });

    \Illuminate\Support\Facades\Event::dispatch('hyperion.test.event', ['payload_ok']);

    if (!$invoked) {
        throw new \Exception("Event listener was not invoked with dispatched payload");
    }
});

// 6. Cache Repository
assertSubsystem("Cache Repository (Array / File Store)", function () {
    \Illuminate\Support\Facades\Cache::put('hyperion_test_key', 'super_fast', 60);
    $val = \Illuminate\Support\Facades\Cache::get('hyperion_test_key');
    if ($val !== 'super_fast') {
        throw new \Exception("Cache get returned '{$val}', expected 'super_fast'");
    }

    $rememberVal = \Illuminate\Support\Facades\Cache::remember('hyperion_remember_test', 60, function () {
        return 9999;
    });
    if ($rememberVal !== 9999) {
        throw new \Exception("Cache remember returned '{$rememberVal}', expected 9999");
    }
});

// 7. Session Management
assertSubsystem("Session Management & State Store", function () use ($app) {
    $session = $app->make('session')->driver();
    $session->setId('hyperion_test_session_id');
    $session->start();
    $session->put('auth_token', 'xyz-12345');

    if ($session->get('auth_token') !== 'xyz-12345') {
        throw new \Exception("Session get failed to retrieve stored token");
    }
    $session->forget('auth_token');
    if ($session->has('auth_token')) {
        throw new \Exception("Session forget failed to remove token");
    }
});

// 8. Validation Engine
assertSubsystem("Validation Engine & Request Rules", function () {
    $validator = \Illuminate\Support\Facades\Validator::make(
        ['email' => 'admin@example.test', 'quantity' => 5],
        ['email' => 'required|email', 'quantity' => 'required|integer|min:1|max:10']
    );

    if ($validator->fails()) {
        throw new \Exception("Valid payload failed validation: " . json_encode($validator->errors()->all()));
    }

    $failValidator = \Illuminate\Support\Facades\Validator::make(
        ['email' => 'not-an-email', 'quantity' => 99],
        ['email' => 'required|email', 'quantity' => 'required|integer|min:1|max:10']
    );

    if ($failValidator->passes()) {
        throw new \Exception("Invalid payload unexpectedly passed validation");
    }
});

// 9. Logging Subsystem (Monolog)
assertSubsystem("Logging Subsystem (Monolog Handler)", function () {
    \Illuminate\Support\Facades\Log::info("Hyperion Core Subsystems Verification Test Log");
});

// 10. Artisan Console Kernel
assertSubsystem("Artisan Console Kernel Execution", function () {
    $exitCode = \Illuminate\Support\Facades\Artisan::call('about');
    if ($exitCode !== 0) {
        throw new \Exception("Artisan call 'about' failed with exit code: {$exitCode}");
    }
    $output = \Illuminate\Support\Facades\Artisan::output();
    if (empty($output) || !str_contains($output, 'Laravel')) {
        throw new \Exception("Artisan call 'about' output was empty or unexpected");
    }
});

$passed = $GLOBALS['passed'];
$total = $GLOBALS['total'];
echo "\n====================================================================\n";
printf(" Subsystem Score: %d/%d Passed (%s)\n", $passed, $total, $passed === $total ? "\033[32m100% SUCCESS\033[0m" : "\033[31mFAILURES DETECTED\033[0m");
echo "====================================================================\n\n";

exit($passed === $total ? 0 : 1);
