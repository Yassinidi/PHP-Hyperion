<?php
namespace Tests\PHPUnit;

use PHPUnit\Framework\TestCase;
use stdClass;
use InvalidArgumentException;
use RuntimeException;

class AssertionSuiteTest extends TestCase
{
    private array $fixtureData;

    protected function setUp(): void
    {
        $this->fixtureData = [
            'id' => 101,
            'username' => 'hyperion_dev',
            'email' => 'dev@hyperion.engine',
            'scores' => [98, 100, 95],
            'active' => true,
            'role' => null,
        ];
    }

    public function testScalarAndIdentityAssertions(): void
    {
        $this->assertEquals(101, $this->fixtureData['id']);
        $this->assertSame(101, $this->fixtureData['id']);
        $this->assertNotSame('101', $this->fixtureData['id']);
        $this->assertTrue($this->fixtureData['active']);
        $this->assertFalse(!$this->fixtureData['active']);
        $this->assertNull($this->fixtureData['role']);
        $this->assertNotNull($this->fixtureData['username']);
    }

    public function testCollectionAndArrayAssertions(): void
    {
        $this->assertCount(3, $this->fixtureData['scores']);
        $this->assertNotEmpty($this->fixtureData['scores']);
        $this->assertEmpty([]);
        $this->assertContains(100, $this->fixtureData['scores']);
        $this->assertArrayHasKey('email', $this->fixtureData);
        $this->assertArrayNotHasKey('missing_key', $this->fixtureData);
    }

    public function testTypeAndHierarchyAssertions(): void
    {
        $this->assertIsArray($this->fixtureData['scores']);
        $this->assertIsString($this->fixtureData['username']);
        $this->assertIsInt($this->fixtureData['id']);
        $this->assertIsBool($this->fixtureData['active']);

        $obj = new stdClass();
        $this->assertInstanceOf(stdClass::class, $obj);
    }

    public function testStringAndRegexAssertions(): void
    {
        $email = $this->fixtureData['email'];
        $this->assertStringContainsString('@hyperion', $email);
        $this->assertStringStartsWith('dev@', $email);
        $this->assertStringEndsWith('.engine', $email);
        $this->assertMatchesRegularExpression('/^[a-z]+@[a-z.]+\.[a-z]+$/', $email);
    }

    public function testExpectedExceptionHandling(): void
    {
        $this->expectException(InvalidArgumentException::class);
        $this->expectExceptionMessage('Invalid user identifier provided');
        $this->expectExceptionCode(400);

        throw new InvalidArgumentException('Invalid user identifier provided', 400);
    }
}
