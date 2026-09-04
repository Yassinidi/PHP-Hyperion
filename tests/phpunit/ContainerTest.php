<?php
declare(strict_types=1);

namespace Tests\PHPUnit;

use Exception;
use PHPUnit\Framework\TestCase;
use Psr\Container\ContainerExceptionInterface;
use Psr\Container\ContainerInterface;
use Psr\Container\NotFoundExceptionInterface;
use ReflectionClass;
use ReflectionNamedType;
use ReflectionParameter;

class NotFoundException extends Exception implements NotFoundExceptionInterface {}
class ContainerException extends Exception implements ContainerExceptionInterface {}

class Container implements ContainerInterface
{
    /** @var array<string, callable> */
    private array $bindings = [];

    /** @var array<string, object> */
    private array $instances = [];

    /** @var array<string, string> */
    private array $aliases = [];

    /** @var array<string, bool> */
    private array $singletons = [];

    /** @var array<string, bool> */
    private array $buildStack = [];

    public function bind(string $abstract, callable|string|null $concrete = null, bool $singleton = false): void
    {
        $concrete = $concrete ?? $abstract;

        if (is_string($concrete)) {
            $concreteCallback = fn (ContainerInterface $c) => $this->build($concrete);
        } else {
            $concreteCallback = $concrete;
        }

        $this->bindings[$abstract] = $concreteCallback;
        $this->singletons[$abstract] = $singleton;
        unset($this->instances[$abstract]);
    }

    public function singleton(string $abstract, callable|string|null $concrete = null): void
    {
        $this->bind($abstract, $concrete, true);
    }

    public function instance(string $abstract, object $instance): void
    {
        $this->instances[$abstract] = $instance;
    }

    public function alias(string $alias, string $target): void
    {
        $this->aliases[$alias] = $target;
    }

    public function has(string $id): bool
    {
        $target = $this->resolveAlias($id);
        return isset($this->bindings[$target]) || isset($this->instances[$target]) || class_exists($target);
    }

    public function get(string $id): mixed
    {
        $target = $this->resolveAlias($id);

        if (isset($this->instances[$target])) {
            return $this->instances[$target];
        }

        if (isset($this->buildStack[$target])) {
            $cycle = implode(' -> ', array_keys($this->buildStack)) . " -> {$target}";
            throw new ContainerException("Circular dependency detected: {$cycle}");
        }

        $this->buildStack[$target] = true;

        try {
            if (isset($this->bindings[$target])) {
                $object = ($this->bindings[$target])($this);
            } else {
                $object = $this->build($target);
            }

            if (!empty($this->singletons[$target])) {
                $this->instances[$target] = $object;
            }

            return $object;
        } finally {
            unset($this->buildStack[$target]);
        }
    }

    public function build(string $concrete): object
    {
        if (!class_exists($concrete)) {
            throw new NotFoundException("Target class [{$concrete}] does not exist.");
        }

        $reflector = new ReflectionClass($concrete);

        if (!$reflector->isInstantiable()) {
            throw new ContainerException("Target [{$concrete}] is not instantiable (interface or abstract class).");
        }

        $constructor = $reflector->getConstructor();
        if ($constructor === null) {
            return new $concrete;
        }

        $dependencies = [];
        foreach ($constructor->getParameters() as $param) {
            $dependencies[] = $this->resolveParameter($param);
        }

        return $reflector->newInstanceArgs($dependencies);
    }

    private function resolveParameter(ReflectionParameter $param): mixed
    {
        $type = $param->getType();

        if ($type instanceof ReflectionNamedType && !$type->isBuiltin()) {
            $className = $type->getName();
            return $this->get($className);
        }

        if ($param->isDefaultValueAvailable()) {
            return $param->getDefaultValue();
        }

        if ($param->allowsNull()) {
            return null;
        }

        throw new ContainerException("Cannot resolve un-typed parameter [{$param->getName()}] without default value.");
    }

    public function call(callable $callable, array $parameters = []): mixed
    {
        return $callable($this, ...$parameters);
    }

    private function resolveAlias(string $id): string
    {
        return $this->aliases[$id] ?? $id;
    }
}

// Dummy service classes for DI testing
interface LoggerInterface { public function log(string $msg): string; }
class FileLogger implements LoggerInterface { public function log(string $msg): string { return "[FILE] " . $msg; } }
class DatabaseLogger implements LoggerInterface { public function log(string $msg): string { return "[DB] " . $msg; } }

class ConfigService
{
    public function __construct(public string $appEnv = 'testing', public int $port = 8080) {}
}

class PaymentProcessor
{
    public function __construct(
        public LoggerInterface $logger,
        public ConfigService $config
    ) {}

    public function pay(int $amount): string
    {
        return $this->logger->log("Processed payment of \${$amount} in {$this->config->appEnv}");
    }
}

class CircularA { public function __construct(public CircularB $b) {} }
class CircularB { public function __construct(public CircularA $a) {} }

final class ContainerTest extends TestCase
{
    private Container $container;

    protected function setUp(): void
    {
        $this->container = new Container();
    }

    public function testBasicAutoWiringWithoutBindings(): void
    {
        $config = $this->container->get(ConfigService::class);
        $this->assertInstanceOf(ConfigService::class, $config);
        $this->assertSame('testing', $config->appEnv);
        $this->assertSame(8080, $config->port);
    }

    public function testInterfaceBindingAndSingletonResolution(): void
    {
        $this->container->singleton(LoggerInterface::class, FileLogger::class);

        $logger1 = $this->container->get(LoggerInterface::class);
        $logger2 = $this->container->get(LoggerInterface::class);

        $this->assertInstanceOf(FileLogger::class, $logger1);
        $this->assertSame($logger1, $logger2); // Identity check for singleton
    }

    public function testTransientBindingCreatesNewInstances(): void
    {
        $this->container->bind(ConfigService::class, function () {
            return new ConfigService('production', 9000);
        });

        $c1 = $this->container->get(ConfigService::class);
        $c2 = $this->container->get(ConfigService::class);

        $this->assertSame('production', $c1->appEnv);
        $this->assertSame(9000, $c1->port);
        $this->assertNotSame($c1, $c2); // Different instances
    }

    public function testNestedAutoWiringWithDependencies(): void
    {
        $this->container->singleton(LoggerInterface::class, DatabaseLogger::class);
        $this->container->singleton(ConfigService::class, fn() => new ConfigService('staging', 443));

        $processor = $this->container->get(PaymentProcessor::class);

        $this->assertInstanceOf(PaymentProcessor::class, $processor);
        $this->assertInstanceOf(DatabaseLogger::class, $processor->logger);
        $this->assertSame('staging', $processor->config->appEnv);

        $result = $processor->pay(500);
        $this->assertSame('[DB] Processed payment of $500 in staging', $result);
    }

    public function testAliasResolution(): void
    {
        $this->container->instance('app.config', new ConfigService('custom-alias', 3000));
        $this->container->alias('cfg', 'app.config');

        $this->assertTrue($this->container->has('cfg'));
        $resolved = $this->container->get('cfg');
        $this->assertInstanceOf(ConfigService::class, $resolved);
        $this->assertSame('custom-alias', $resolved->appEnv);
    }

    public function testNonExistentClassThrowsNotFoundException(): void
    {
        $this->expectException(NotFoundException::class);
        $this->expectExceptionMessage('Target class [NonExistentVendor\\FooBar] does not exist');
        $this->container->get('NonExistentVendor\\FooBar');
    }

    public function testCircularDependencyDetectionThrowsContainerException(): void
    {
        $this->expectException(ContainerException::class);
        $this->expectExceptionMessage('Circular dependency detected');
        $this->container->get(CircularA::class);
    }
}
