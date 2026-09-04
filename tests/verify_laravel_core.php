<?php
declare(strict_types=1);

/**
 * Tier 2: Laravel Core Subsystems Battery
 * Comprehensive verification of all 10 major Laravel subsystems running on Hyperion.
 */

require_once __DIR__ . '/../laravel-full/vendor/autoload.php';

$app = require_once __DIR__ . '/../laravel-full/bootstrap/app.php';
$kernel = $app->make(\Illuminate\Contracts\Console\Kernel::class);
$kernel->bootstrap();

$GLOBALS['passed'] = 0;
$GLOBALS['total'] = 0;

function assertSubsystem(string $name, callable $test): void {
    $GLOBALS['total']++;
    echo "[TEST " . $GLOBALS['total'] . "] {$name}... ";
    try {
        $test();
        echo "PASSED\n";
        $GLOBALS['passed']++;
    } catch (\Throwable $e) {
        echo "FAILED: " . $e->getMessage() . " in " . $e->getFile() . ":" . $e->getLine() . "\n";
        echo $e->getTraceAsString() . "\n";
    }
}

echo "=======================================================\n";
echo "Hyperion PHP - Laravel Core Subsystems Battery\n";
echo "Laravel Framework Version: " . $app->version() . "\n";
echo "=======================================================\n\n";

// 1. Service Container & DI
assertSubsystem("1. Service Container & Dependency Injection", function () use ($app) {
    // Basic binding
    $app->bind('test.service', fn() => new class { public function hello() { return 'world'; } });
    $svc = $app->make('test.service');
    if ($svc->hello() !== 'world') {
        throw new \Exception("Container did not resolve binding correctly");
    }

    // Singleton binding
    $app->singleton('test.singleton', fn() => new \stdClass());
    $s1 = $app->make('test.singleton');
    $s2 = $app->make('test.singleton');
    if ($s1 !== $s2) {
        throw new \Exception("Singleton resolved different instances");
    }
});

// 2. Routing & HTTP Pipeline
assertSubsystem("2. Routing & HTTP Middleware Pipeline", function () use ($app) {
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

// 3. Eloquent ORM & Query Builder
assertSubsystem("3. Eloquent ORM & Query Builder", function () {
    $count = \App\Models\User::count();
    if ($count < 1) {
        throw new \Exception("Expected at least 1 user in database, found {$count}");
    }

    $admin = \App\Models\User::first();
    if (!$admin || empty($admin->email)) {
        throw new \Exception("Failed to retrieve admin user via Eloquent");
    }

    // Check relationship
    $categories = \App\Models\Category::with('houses')->get();
    if ($categories->isEmpty()) {
        throw new \Exception("Failed to query categories with Eloquent relation");
    }
});

// 4. Blade Templating Engine
assertSubsystem("4. Blade Templating Engine", function () {
    $rendered = \Illuminate\Support\Facades\Blade::render(
        'Hello, {{ $name }}! 1 + 1 = {{ 1 + 1 }}',
        ['name' => 'Hyperion Engine']
    );

    if ($rendered !== 'Hello, Hyperion Engine! 1 + 1 = 2') {
        throw new \Exception("Blade rendering output mismatch: '{$rendered}'");
    }
});

// 5. Event Dispatcher & Listeners
assertSubsystem("5. Event Dispatcher & Listeners", function () {
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
assertSubsystem("6. Cache Repository (Array / File Store)", function () {
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
assertSubsystem("7. Session Management", function () use ($app) {
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
assertSubsystem("8. Validation Engine", function () {
    $validator = \Illuminate\Support\Facades\Validator::make(
        ['email' => 'admin@tedja.test', 'quantity' => 5],
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
assertSubsystem("9. Logging Subsystem (Monolog)", function () {
    \Illuminate\Support\Facades\Log::info("Hyperion Core Subsystems Verification Test Log");
});

// 10. Artisan Console Kernel
assertSubsystem("10. Artisan Console Kernel Execution", function () {
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
echo "\n=======================================================\n";
echo "Results: {$passed}/{$total} Subsystems PASSED\n";
echo "=======================================================\n";

if ($passed !== $total) {
    exit(1);
}
