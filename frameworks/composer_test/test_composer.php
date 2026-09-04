<?php

declare(strict_types=1);

echo "=== Composer Package Manager Architecture Test on Hyperion ===\n\n";

$vendor_dir = __DIR__ . '/vendor';
$lock_file = __DIR__ . '/composer.lock';

// Clean up previous test artifacts
if (file_exists($lock_file)) {
    unlink($lock_file);
}
if (is_dir($vendor_dir)) {
    $files = new RecursiveIteratorIterator(
        new RecursiveDirectoryIterator($vendor_dir, RecursiveDirectoryIterator::SKIP_DOTS),
        RecursiveIteratorIterator::CHILD_FIRST
    );
    foreach ($files as $fileinfo) {
        if ($fileinfo->isDir()) {
            rmdir($fileinfo->getRealPath());
        } else {
            unlink($fileinfo->getRealPath());
        }
    }
    rmdir($vendor_dir);
}

// -------------------------------------------------------------
// Part 1: Semver Constraint Resolution Engine
// -------------------------------------------------------------
echo "[Test 1] Semver Constraint Resolution Engine...\n";

class Semver {
    public static function satisfies(string $version, string $constraint): bool {
        $constraint = trim($constraint);
        
        if ($constraint === '*' || $constraint === '') {
            return true;
        }

        // Caret constraint: ^1.2.3 -> >= 1.2.3 and < 2.0.0
        if (str_starts_with($constraint, '^')) {
            $base = substr($constraint, 1);
            $parts = explode('.', $base);
            $major = (int)($parts[0] ?? 0);
            $next_major = $major + 1;
            return self::version_compare($version, $base, '>=') && self::version_compare($version, "{$next_major}.0.0", '<');
        }

        // Tilde constraint: ~1.2.3 -> >= 1.2.3 and < 1.3.0
        if (str_starts_with($constraint, '~')) {
            $base = substr($constraint, 1);
            $parts = explode('.', $base);
            $major = (int)($parts[0] ?? 0);
            $minor = (int)($parts[1] ?? 0);
            $next_minor = $minor + 1;
            return self::version_compare($version, $base, '>=') && self::version_compare($version, "{$major}.{$next_minor}.0", '<');
        }

        // Exact or range comparisons
        if (str_starts_with($constraint, '>=')) {
            return self::version_compare($version, trim(substr($constraint, 2)), '>=');
        }
        if (str_starts_with($constraint, '<=')) {
            return self::version_compare($version, trim(substr($constraint, 2)), '<=');
        }
        if (str_starts_with($constraint, '>')) {
            return self::version_compare($version, trim(substr($constraint, 1)), '>');
        }
        if (str_starts_with($constraint, '<')) {
            return self::version_compare($version, trim(substr($constraint, 1)), '<');
        }

        return self::version_compare($version, $constraint, '==');
    }

    private static function version_compare(string $v1, string $v2, string $operator): bool {
        return version_compare($v1, $v2, $operator);
    }
}

assert(Semver::satisfies('1.2.5', '^1.2.0'));
assert(!Semver::satisfies('2.0.0', '^1.2.0'));
assert(Semver::satisfies('1.2.5', '~1.2.0'));
assert(!Semver::satisfies('1.3.0', '~1.2.0'));
assert(Semver::satisfies('3.4.1', '>=3.0.0'));
echo "  -> Semver caret, tilde, and range constraints verified.\n";
echo "  -> OK!\n\n";

// -------------------------------------------------------------
// Part 2: Dependency Solver & Lockfile Generation
// -------------------------------------------------------------
echo "[Test 2] Dependency Solver & Lockfile Generation...\n";

$composer_json = [
    'name' => 'hyperion/app',
    'require' => [
        'acme/logger' => '^2.1.0',
        'acme/http-client' => '^1.0.0',
    ],
];

// Available repository pool
$repo_pool = [
    'acme/logger' => [
        '2.1.0' => ['requires' => []],
        '2.1.4' => ['requires' => []],
        '2.2.0' => ['requires' => []],
        '3.0.0' => ['requires' => []],
    ],
    'acme/http-client' => [
        '1.0.0' => ['requires' => ['acme/logger' => '^2.0.0']],
        '1.1.2' => ['requires' => ['acme/logger' => '^2.1.0']],
    ],
];

// Solve dependencies
$locked_packages = [];
foreach ($composer_json['require'] as $package_name => $constraint) {
    $available_versions = array_keys($repo_pool[$package_name] ?? []);
    // Sort descending to find the highest matching version
    usort($available_versions, fn($a, $b) => version_compare($b, $a));

    $selected_version = null;
    foreach ($available_versions as $ver) {
        if (Semver::satisfies($ver, $constraint)) {
            $selected_version = $ver;
            break;
        }
    }

    assert($selected_version !== null, "Could not resolve {$package_name}");
    $locked_packages[] = [
        'name' => $package_name,
        'version' => $selected_version,
        'dist' => [
            'type' => 'zip',
            'shasum' => hash('sha256', "{$package_name}@{$selected_version}"),
        ],
        'autoload' => [
            'psr-4' => [
                str_replace(' ', '', ucwords(str_replace(['/', '-'], ['\\', ' '], $package_name))) . '\\' => 'src/',
            ],
        ],
    ];
}

$lock_data = [
    '_readme' => ['This file locks the dependencies of your project to a known state'],
    'content-hash' => hash('sha256', json_encode($composer_json)),
    'packages' => $locked_packages,
];

file_put_contents($lock_file, json_encode($lock_data, JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES));
assert(file_exists($lock_file));

echo "  -> Solved packages: " . json_encode(array_column($locked_packages, 'version', 'name')) . "\n";
echo "  -> Generated composer.lock (hash: " . substr($lock_data['content-hash'], 0, 12) . "...)\n";
echo "  -> OK!\n\n";

// -------------------------------------------------------------
// Part 3: Package Downloader & File System Installer
// -------------------------------------------------------------
echo "[Test 3] Package Downloader & File System Installer...\n";

// Mock installer: creating virtual vendor packages
foreach ($locked_packages as $pkg) {
    $pkg_name = $pkg['name'];
    $pkg_dir = "{$vendor_dir}/{$pkg_name}/src";
    if (!is_dir($pkg_dir)) {
        mkdir($pkg_dir, 0777, true);
    }

    if ($pkg_name === 'acme/logger') {
        file_put_contents("{$pkg_dir}/Logger.php", '<?php
namespace Acme\\Logger;

class Logger {
    public function log(string $msg): string {
        return "[LOG] " . $msg;
    }
}
');
    } elseif ($pkg_name === 'acme/http-client') {
        file_put_contents("{$pkg_dir}/Client.php", '<?php
namespace Acme\\HttpClient;

use Acme\\Logger\\Logger;

class Client {
    private Logger $logger;

    public function __construct(Logger $logger) {
        $this->logger = $logger;
    }

    public function get(string $url): string {
        return $this->logger->log("GET " . $url . " => 200 OK");
    }
}
');
    }
}

assert(file_exists("{$vendor_dir}/acme/logger/src/Logger.php"));
assert(file_exists("{$vendor_dir}/acme/http-client/src/Client.php"));
echo "  -> Installed packages to vendor/ directory successfully.\n";
echo "  -> OK!\n\n";

// -------------------------------------------------------------
// Part 4: Autoload Generator (Composer AutoloadGenerator)
// -------------------------------------------------------------
echo "[Test 4] Composer AutoloadGenerator...\n";

$composer_internal_dir = "{$vendor_dir}/composer";
if (!is_dir($composer_internal_dir)) {
    mkdir($composer_internal_dir, 0777, true);
}

// Generate autoload_psr4.php
$psr4_map = [
    'Acme\\Logger\\' => ["{$vendor_dir}/acme/logger/src"],
    'Acme\\HttpClient\\' => ["{$vendor_dir}/acme/http-client/src"],
];

$psr4_code = "<?php\n\nreturn " . var_export($psr4_map, true) . ";\n";
file_put_contents("{$composer_internal_dir}/autoload_psr4.php", $psr4_code);

// Generate ClassLoader.php
file_put_contents("{$composer_internal_dir}/ClassLoader.php", '<?php
namespace Composer\\Autoload;

class ClassLoader {
    private array $prefixLengthsPsr4 = [];
    private array $prefixDirsPsr4 = [];

    public function addPsr4(string $prefix, array|string $paths, bool $prepend = false): void {
        $paths = (array)$paths;
        $length = strlen($prefix);
        if ($prefix[$length - 1] !== "\\\\") {
            $prefix .= "\\\\";
            $length++;
        }
        $firstChar = $prefix[0];
        if (!isset($this->prefixDirsPsr4[$firstChar][$prefix])) {
            $this->prefixLengthsPsr4[$firstChar][$prefix] = $length;
            $this->prefixDirsPsr4[$firstChar][$prefix] = $paths;
        } else {
            $this->prefixDirsPsr4[$firstChar][$prefix] = array_merge($this->prefixDirsPsr4[$firstChar][$prefix], $paths);
        }
    }

    public function loadClass(string $class): ?bool {
        if ($file = $this->findFile($class)) {
            require $file;
            return true;
        }
        return null;
    }

    public function findFile(string $class): string|false {
        $firstChar = $class[0];
        if (isset($this->prefixLengthsPsr4[$firstChar])) {
            foreach ($this->prefixLengthsPsr4[$firstChar] as $prefix => $length) {
                if (str_starts_with($class, $prefix)) {
                    $subPath = substr($class, $length);
                    $relPath = str_replace("\\\\", "/", $subPath) . ".php";
                    foreach ($this->prefixDirsPsr4[$firstChar][$prefix] as $dir) {
                        $file = "{$dir}/{$relPath}";
                        if (file_exists($file)) {
                            return $file;
                        }
                    }
                }
            }
        }
        return false;
    }

    public function register(bool $prepend = false): void {
        spl_autoload_register([$this, "loadClass"], true, $prepend);
    }
}
');

// Generate vendor/autoload.php
file_put_contents("{$vendor_dir}/autoload.php", '<?php

require_once __DIR__ . "/composer/ClassLoader.php";

$loader = new \\Composer\\Autoload\\ClassLoader();
$map = require __DIR__ . "/composer/autoload_psr4.php";
foreach ($map as $namespace => $path) {
    $loader->addPsr4($namespace, $path);
}
$loader->register(true);

return $loader;
');

assert(file_exists("{$vendor_dir}/autoload.php"));
echo "  -> Generated vendor/autoload.php and Composer\\Autoload\\ClassLoader.\n";
echo "  -> OK!\n\n";

// -------------------------------------------------------------
// Part 5: End-to-End Package Runtime Verification
// -------------------------------------------------------------
echo "[Test 5] End-to-End Package Runtime Verification...\n";

// Require the generated composer autoloader
$autoloader = require "{$vendor_dir}/autoload.php";
assert($autoloader instanceof \Composer\Autoload\ClassLoader);

// Instantiate autoloader classes
$logger = new \Acme\Logger\Logger();
$client = new \Acme\HttpClient\Client($logger);

$response = $client->get('https://hyperion.dev/api/v1/ping');
assert($response === '[LOG] GET https://hyperion.dev/api/v1/ping => 200 OK');

echo "  -> Client response: {$response}\n";
echo "  -> OK!\n\n";

echo "=== ALL COMPOSER TESTS PASSED 100% ===\n";
