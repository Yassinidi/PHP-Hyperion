<?php

declare(strict_types=1);

namespace tests;

use Flight;
use flight\core\Loader;
use flight\Engine;
use tests\classes\User;
use PHPUnit\Framework\TestCase;
use ReflectionClass;

class AutoloadTest extends TestCase
{
    private Engine $app;

    protected function setUp(): void
    {
        $this->app = new Engine();
        $this->app->path(__DIR__ . '/classes');
    }

    protected function tearDown(): void
    {
        $dirsProperty = (new ReflectionClass(Loader::class))->getProperty('dirs');

        if (PHP_VERSION_ID < 80100) {
            $dirsProperty->setAccessible(true);
        }

        $dirsProperty->setValue(null, []);
    }

    // Autoload a class
    public function testAutoload(): void
    {
        $this->app->register('user', User::class);

        $loaders = spl_autoload_functions();

        $user = $this->app->user();

        self::assertTrue(count($loaders) > 0);
        self::assertIsObject($user);
        self::assertInstanceOf(User::class, $user);
    }

    // Check autoload failure
    public function testMissingClass(): void
    {
        $test = null;
        $this->app->register('test', 'NonExistentClass');

        if (class_exists('NonExistentClass')) {
            $test = $this->app->test();
        }

        self::assertNull($test);
    }

    // Flight::path() must load namespaced classes that Composer PSR-4 does not map
    public function testPathAutoloadsNamespacedClassOutsideComposerPsr4(): void
    {
        $class = \app\middleware\Something::class;

        self::assertFalse(class_exists($class));

        Flight::path(__DIR__ . '/path_autoload_fixtures');

        self::assertTrue(class_exists($class));
        self::assertTrue(class_exists(Engine::class));
    }
}
