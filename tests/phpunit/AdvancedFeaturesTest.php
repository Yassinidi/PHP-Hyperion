<?php
namespace Tests\PHPUnit;

use PHPUnit\Framework\TestCase;
use PHPUnit\Framework\Attributes\DataProvider;
use PHPUnit\Framework\Attributes\TestWith;
use PHPUnit\Framework\Attributes\Depends;
use PHPUnit\Framework\Attributes\Test;

class AdvancedFeaturesTest extends TestCase
{
    private static int $classSetupCounter = 0;
    private int $instanceSetupCounter = 0;

    public static function setUpBeforeClass(): void
    {
        self::$classSetupCounter = 100;
    }

    public static function tearDownAfterClass(): void
    {
        self::$classSetupCounter = 0;
    }

    protected function setUp(): void
    {
        $this->instanceSetupCounter++;
    }

    public static function additionProvider(): array
    {
        return [
            'zeros' => [0, 0, 0],
            'positive integers' => [15, 25, 40],
            'negative integers' => [-10, 5, -5],
            'large numbers' => [1000, 2500, 3500],
        ];
    }

    #[DataProvider('additionProvider')]
    public function testAdditionWithDataProvider(int $a, int $b, int $expected): void
    {
        $this->assertSame($expected, $a + $b);
        $this->assertSame(100, self::$classSetupCounter);
    }

    #[TestWith([10, 20, 30])]
    #[TestWith([5, 5, 10])]
    #[TestWith([100, -50, 50])]
    public function testAdditionWithTestWithAttribute(int $x, int $y, int $expected): void
    {
        $this->assertSame($expected, $x + $y);
    }

    public function testInitialStateProducesValue(): array
    {
        $state = ['step' => 1, 'token' => 'hyperion_alpha_7'];
        $this->assertSame(1, $state['step']);
        return $state;
    }

    #[Depends('testInitialStateProducesValue')]
    public function testDependentStep(array $passedState): array
    {
        $this->assertSame(1, $passedState['step']);
        $this->assertSame('hyperion_alpha_7', $passedState['token']);
        $passedState['step'] = 2;
        $passedState['validated'] = true;
        return $passedState;
    }

    #[Depends('testInitialStateProducesValue')]
    public function testFinalDependentVerification(array $passedState): void
    {
        $this->assertSame(1, $passedState['step']);
        $this->assertSame('hyperion_alpha_7', $passedState['token']);
    }

    #[Test]
    public function customNamedMethodWithTestAttribute(): void
    {
        $this->assertSame(1, $this->instanceSetupCounter);
        $this->assertTrue(true);
    }
}
