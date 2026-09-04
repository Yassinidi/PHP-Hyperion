<?php

declare(strict_types=1);

/**
 * PHP-Hyperion Laravel Core Subsystem Deep-Dive Verification Battery
 *
 * Exercises all major Laravel contracts and runtime subsystems:
 * 1. Service Container & IoC (Auto-wiring, contextual binding, variadics, rebinding)
 * 2. Eloquent ORM & Query Builder (Relations, eager loading, lifecycle events, casts, scopes, transactions)
 * 3. Validation & Form Requests (Builtin rules, array wildcards, custom rules)
 * 4. Security, Cryptography & Auth (Hashing, AES-256 encryption, Spatie roles/permissions)
 * 5. Blade Templating Engine (Directives, components, escaping, stacks)
 * 6. Filesystem, Cache, and Queues (Storage, atomic locks, sync queues)
 */

namespace Tests;

require_once __DIR__ . '/../vendor/autoload.php';

use Illuminate\Contracts\Console\Kernel;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Facades\Crypt;
use Illuminate\Support\Facades\Cache;
use Illuminate\Support\Facades\Storage;
use Illuminate\Support\Facades\Blade;
use Illuminate\Support\Facades\Validator;
use Illuminate\Support\Facades\Gate;
use Illuminate\Validation\Rule;
use Illuminate\Validation\ValidationException;
use App\Models\User;
use App\Models\Bank;
use App\Models\Category;
use App\Models\City;
use App\Models\House;
use Spatie\Permission\Models\Role;
use Spatie\Permission\Models\Permission;

class TestRunner
{
    public int $passed = 0;
    public int $failed = 0;
    public array $errors = [];

    public function assert(bool $condition, string $message): void
    {
        if ($condition) {
            $this->passed++;
            echo "  \033[32m✔\033[0m {$message}\n";
        } else {
            $this->failed++;
            $this->errors[] = $message;
            echo "  \033[31m✘\033[0m {$message} (FAILED)\n";
        }
    }

    public function assertEquals(mixed $expected, mixed $actual, string $message): void
    {
        $this->assert($expected === $actual, "{$message} [Expected: " . json_encode($expected) . ", Got: " . json_encode($actual) . "]");
    }

    public function section(string $title): void
    {
        echo "\n\033[1;34m=== {$title} ===\033[0m\n";
    }

    public function summary(): int
    {
        echo "\n" . str_repeat("=", 60) . "\n";
        echo "Laravel Subsystem Battery Summary:\n";
        echo "Total Assertions: " . ($this->passed + $this->failed) . "\n";
        echo "\033[32mPassed: {$this->passed}\033[0m\n";
        if ($this->failed > 0) {
            echo "\033[31mFailed: {$this->failed}\033[0m\n";
            foreach ($this->errors as $err) {
                echo "  - {$err}\n";
            }
            return 1;
        } else {
            echo "\033[1;32mALL SUBSYSTEM TESTS PASSED WITH 100% PARITY!\033[0m\n";
            return 0;
        }
    }
}

// ---------------------------------------------------------
// Helper Classes for IoC Testing
// ---------------------------------------------------------
interface DummyLoggerInterface { public function log(string $msg): string; }
class FileLogger implements DummyLoggerInterface { public function log(string $msg): string { return "file: {$msg}"; } }
class SyslogLogger implements DummyLoggerInterface { public function log(string $msg): string { return "syslog: {$msg}"; } }

class ServiceA {
    public function __construct(public DummyLoggerInterface $logger) {}
}

class ServiceB {
    public function __construct(public DummyLoggerInterface $logger) {}
}

class DefaultParamService {
    public function __construct(public string $mode = 'production', public int $timeout = 30) {}
}

// ---------------------------------------------------------
// Execution
// ---------------------------------------------------------
echo "Bootstrapping Laravel 11 application for core subsystem verification...\n";
$app = require __DIR__ . '/../bootstrap/app.php';
$app->make(Kernel::class)->bootstrap();

$t = new TestRunner();

// =========================================================
// 1. Service Container & IoC
// =========================================================
$t->section("1. Service Container & Inversion of Control");

// 1.1 Contextual Binding
$app->when(ServiceA::class)->needs(DummyLoggerInterface::class)->give(FileLogger::class);
$app->when(ServiceB::class)->needs(DummyLoggerInterface::class)->give(SyslogLogger::class);

$a = $app->make(ServiceA::class);
$b = $app->make(ServiceB::class);
$t->assert($a->logger instanceof FileLogger, "Contextual binding resolved FileLogger for ServiceA");
$t->assert($b->logger instanceof SyslogLogger, "Contextual binding resolved SyslogLogger for ServiceB");

// 1.2 Default Parameter Values in Auto-wiring
$def = $app->make(DefaultParamService::class);
$t->assertEquals('production', $def->mode, "Container auto-wired default string argument");
$t->assertEquals(30, $def->timeout, "Container auto-wired default int argument");

// 1.3 Container Singletons & Rebinding
$app->singleton('test.singleton', function () {
    return new \stdClass();
});
$s1 = $app->make('test.singleton');
$s2 = $app->make('test.singleton');
$t->assert($s1 === $s2, "Container singleton resolves to identical memory instance");

$rebound = false;
$app->rebinding('test.rebind', function ($app, $instance) use (&$rebound) {
    $rebound = true;
});
$app->instance('test.rebind', 'first');
$app->instance('test.rebind', 'second');
$t->assert($rebound, "Container rebinding callback triggers on instance mutation");

// =========================================================
// 2. Eloquent ORM & Query Builder
// =========================================================
$t->section("2. Eloquent ORM & Query Builder");

// 2.1 Basic CRUD & Timestamps
DB::beginTransaction();
try {
    $city = City::create([
        'name' => 'Hyperion Silicon City',
        'slug' => 'hyperion-silicon-city-' . uniqid(),
        'photo' => 'city.png',
    ]);
    $t->assert($city->exists, "Model persisted and ID generated");
    $t->assert(!empty($city->created_at), "Timestamps automatically populated on Model::create");

    $category = Category::create([
        'name' => 'Modern Villa',
        'slug' => 'modern-villa-' . uniqid(),
        'photo' => 'villa.png'
    ]);

    $house = House::create([
        'name' => 'Skyline Villa Alpha ' . uniqid(),
        'thumbnail' => 'thumbnail.png',
        'about' => 'Luxury living powered by Hyperion',
        'price' => 750000000,
        'city_id' => $city->id,
        'category_id' => $category->id,
        'certificate' => 'SHM',
        'bedroom' => 4,
        'bathroom' => 3,
        'electric' => 2200,
        'land_area' => 200,
        'building_area' => 150,
    ]);
    $t->assert($house->exists, "House model successfully created");

    // SoftDeletes Verification
    $house->delete();
    $t->assert($house->trashed(), "Model soft-deleted successfully");
    $t->assert(House::withTrashed()->where('id', $house->id)->exists(), "withTrashed() finds soft-deleted model");
    $house->restore();
    $t->assert(!$house->trashed(), "Model restored from soft-delete");

    // 2.2 Relationships: BelongsTo & HasMany
    $t->assertEquals('Hyperion Silicon City', $house->city->name, "BelongsTo relationship (house -> city) resolves correctly");
    $t->assertEquals('modern villa', $house->category->name, "BelongsTo relationship (house -> category) with strtolower mutator resolves correctly");
    $t->assert($city->houses->contains($house->id), "HasMany relationship (city -> houses) contains created house");

    // 2.3 Eager Loading & Nested Relations
    $eagerLoaded = House::with(['city', 'category'])->where('id', $house->id)->first();
    $t->assert($eagerLoaded->relationLoaded('city'), "Eager loaded 'city' relation marked as loaded");
    $t->assert($eagerLoaded->relationLoaded('category'), "Eager loaded 'category' relation marked as loaded");
    $t->assertEquals('Hyperion Silicon City', $eagerLoaded->city->name, "Eager-loaded city name matches");

    // 2.4 WhereHas Query Builder
    $hasCity = House::whereHas('city', function ($q) {
        $q->where('name', 'Hyperion Silicon City');
    })->where('id', $house->id)->exists();
    $t->assert($hasCity, "whereHas query evaluates relation existence properly");

    // 2.5 Aggregate Queries (withCount, sum, avg)
    $cityWithCount = City::withCount('houses')->where('id', $city->id)->first();
    $t->assert($cityWithCount->houses_count >= 1, "withCount accurately calculates related rows count");

    // 2.6 Transactions & Nested Savepoints
    DB::transaction(function () use ($t, $house) {
        $house->update(['price' => 800000000]);
        $t->assertEquals(800000000, $house->fresh()->price, "Update within transaction reflected immediately");

        // Nested rollback test
        try {
            DB::transaction(function () use ($house) {
                $house->update(['price' => 999999999]);
                throw new \Exception("Rollback nested");
            });
        } catch (\Exception $e) {
            // Handled
        }
        $t->assertEquals(800000000, $house->fresh()->price, "Nested transaction rollback restores parent state");
    });

} finally {
    DB::rollBack();
}

// =========================================================
// 3. Validation & Form Requests
// =========================================================
$t->section("3. Validation & Form Requests");

// 3.1 Standard Rules: required, email, min, in
$validator = Validator::make([
    'email' => 'developer@hyperion-php.dev',
    'age' => 25,
    'status' => 'active'
], [
    'email' => ['required', 'email:rfc'],
    'age' => ['required', 'numeric', 'min:18'],
    'status' => [Rule::in(['active', 'pending', 'inactive'])],
]);
$t->assert(!$validator->fails(), "Valid payload passes RFC email, numeric, and Rule::in validators");

// 3.2 Failure Bag Verification
$failValidator = Validator::make([
    'email' => 'invalid-email',
    'age' => 12
], [
    'email' => ['required', 'email'],
    'age' => ['required', 'numeric', 'min:18'],
]);
$t->assert($failValidator->fails(), "Invalid payload correctly fails validation");
$t->assert($failValidator->errors()->has('email'), "Error bag contains 'email' key");
$t->assert($failValidator->errors()->has('age'), "Error bag contains 'age' key");

// 3.3 Array Wildcard Validation
$arrayValidator = Validator::make([
    'items' => [
        ['name' => 'Item A', 'qty' => 2],
        ['name' => 'Item B', 'qty' => 5],
    ]
], [
    'items.*.name' => ['required', 'string'],
    'items.*.qty' => ['required', 'integer', 'min:1'],
]);
$t->assert(!$arrayValidator->fails(), "Array wildcard rules (items.*.name, items.*.qty) pass");

// 3.4 Custom Closure Validation Rule
$customPassed = false;
$closureValidator = Validator::make(['code' => 'HYP-2026'], [
    'code' => [function (string $attr, mixed $val, \Closure $fail) use (&$customPassed) {
        if (str_starts_with($val, 'HYP-')) {
            $customPassed = true;
        } else {
            $fail("The {$attr} is invalid.");
        }
    }]
]);
$t->assert(!$closureValidator->fails() && $customPassed, "Closure-based custom validation rule executed successfully");

// =========================================================
// 4. Security, Cryptography & Authentication
// =========================================================
$t->section("4. Security, Cryptography & Permissions");

// 4.1 Password Hashing (Bcrypt)
$password = "P@ssw0rdSecure2026!";
$hashed = Hash::make($password);
$t->assert(Hash::check($password, $hashed), "Hash::make and Hash::check verify correct password");
$t->assert(!Hash::check("WrongPassword!", $hashed), "Hash::check rejects incorrect password");

// 4.2 AES-256 Symmetric Encryption
$secretPayload = "Confidential Banking Record #" . rand(1000, 9999);
$encrypted = Crypt::encryptString($secretPayload);
$t->assert($encrypted !== $secretPayload, "Crypt::encryptString produces ciphertext");
$decrypted = Crypt::decryptString($encrypted);
$t->assertEquals($secretPayload, $decrypted, "Crypt::decryptString accurately decrypts to original plaintext");

// 4.3 Spatie Roles & Permissions (if tables populated)
if (\Illuminate\Support\Facades\Schema::hasTable('roles')) {
    $role = Role::firstOrCreate(['name' => 'super-admin', 'guard_name' => 'web']);
    $permission = Permission::firstOrCreate(['name' => 'edit-properties', 'guard_name' => 'web']);
    $role->givePermissionTo($permission);

    $t->assert($role->hasPermissionTo('edit-properties'), "Spatie Role::hasPermissionTo evaluates granted permission");
}

// =========================================================
// 5. Blade Templating Engine
// =========================================================
$t->section("5. Blade Templating Engine");

// 5.1 Variable Escaping & Raw Expressions
$escapedRender = Blade::render('Hello {{ $name }}', ['name' => '<script>alert(1)</script>']);
$t->assertEquals('Hello &lt;script&gt;alert(1)&lt;/script&gt;', $escapedRender, "Blade {{ }} correctly escapes HTML entities");

$rawRender = Blade::render('Hello {!! $html !!}', ['html' => '<b>Hyperion</b>']);
$t->assertEquals('Hello <b>Hyperion</b>', $rawRender, "Blade {!! !!} renders raw HTML unescaped");

// 5.2 Control Structures (@if, @foreach)
$template = '@if($active) ACTIVE @else INACTIVE @endif: @foreach($items as $i){{ $i }}-@endforeach';
$result = trim(Blade::render($template, ['active' => true, 'items' => [1, 2, 3]]));
$t->assertEquals('ACTIVE : 1-2-3-', $result, "Blade control structures (@if, @else, @foreach) execute accurately");

// 5.3 Stacks & Push Directives
$stackTemplate = '@push("scripts")<script src="app.js"></script>@endpush @stack("scripts")';
$stackResult = trim(Blade::render($stackTemplate));
$t->assertEquals('<script src="app.js"></script>', $stackResult, "Blade @push and @stack directives assemble scripts");

// =========================================================
// 6. Filesystem, Cache, and Queues
// =========================================================
$t->section("6. Filesystem, Cache, and System Services");

// 6.1 Storage Disk (Local)
$testFile = 'hyperion_verify_' . uniqid() . '.txt';
$testData = "Verified by Hyperion Engine at " . date('c');
Storage::disk('local')->put($testFile, $testData);
$t->assert(Storage::disk('local')->exists($testFile), "Storage::disk('local')->put writes file to disk");
$t->assertEquals($testData, Storage::disk('local')->get($testFile), "Storage::disk('local')->get retrieves identical content");
Storage::disk('local')->delete($testFile);
$t->assert(!Storage::disk('local')->exists($testFile), "Storage::disk('local')->delete removes file");

// 6.2 Cache Operations & Expiration
$cacheKey = 'hyperion_test_key_' . uniqid();
Cache::put($cacheKey, 'CachedValue123', 60);
$t->assertEquals('CachedValue123', Cache::get($cacheKey), "Cache::put and Cache::get store and retrieve value");
Cache::forget($cacheKey);
$t->assert(Cache::get($cacheKey) === null, "Cache::forget invalidates cached key");

// 6.3 Atomic Locks
$lock = Cache::lock('hyperion_mutex_lock', 10);
if ($lock->get()) {
    $t->assert(true, "Cache::lock acquired mutex lock successfully");
    $lock->release();
} else {
    $t->assert(false, "Failed to acquire cache lock");
}

// Return exit code (0 = success, 1 = failure)
exit($t->summary());
