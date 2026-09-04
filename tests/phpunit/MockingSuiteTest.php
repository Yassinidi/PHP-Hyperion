<?php
namespace Tests\PHPUnit;

require_once __DIR__ . '/fixtures.php';

use PHPUnit\Framework\TestCase;

interface NotificationServiceInterface
{
    public function notify(string $recipient, string $message): bool;
}

class OrderProcessor
{
    public function __construct(
        private PaymentGatewayInterface $gateway,
        private NotificationServiceInterface $notifier
    ) {}

    public function processOrder(string $user, int $amount, string $currency): array
    {
        $charged = $this->gateway->charge($amount, $currency);
        if (!$charged) {
            return ['status' => 'failed', 'reason' => 'payment_declined'];
        }

        $txId = $this->gateway->getTransactionId();
        $this->notifier->notify($user, "Order processed successfully: {$txId}");

        return [
            'status' => 'success',
            'transaction_id' => $txId,
            'user' => $user,
            'amount' => $amount,
        ];
    }
}

class MockingSuiteTest extends TestCase
{
    public function testMockCreationAndStubbing(): void
    {
        $gatewayStub = $this->createStub(PaymentGatewayInterface::class);
        $gatewayStub->method('charge')->willReturn(true);
        $gatewayStub->method('getTransactionId')->willReturn('TX_HYPERION_9988');

        $this->assertTrue($gatewayStub->charge(100, 'USD'));
        $this->assertSame('TX_HYPERION_9988', $gatewayStub->getTransactionId());
    }

    public function testMockExpectationsAndInvocations(): void
    {
        $gatewayMock = $this->createMock(PaymentGatewayInterface::class);
        $gatewayMock->expects($this->once())
            ->method('charge')
            ->with(250, 'EUR')
            ->willReturn(true);

        $gatewayMock->expects($this->once())
            ->method('getTransactionId')
            ->willReturn('TX_MOCK_5544');

        $notifierMock = $this->createMock(NotificationServiceInterface::class);
        $notifierMock->expects($this->once())
            ->method('notify')
            ->with('alice@example.com', 'Order processed successfully: TX_MOCK_5544')
            ->willReturn(true);

        $processor = new OrderProcessor($gatewayMock, $notifierMock);
        $result = $processor->processOrder('alice@example.com', 250, 'EUR');

        $this->assertSame('success', $result['status']);
        $this->assertSame('TX_MOCK_5544', $result['transaction_id']);
        $this->assertSame(250, $result['amount']);
    }

    public function testMockDeclinedPaymentNeverNotifies(): void
    {
        $gatewayMock = $this->createMock(PaymentGatewayInterface::class);
        $gatewayMock->expects($this->once())
            ->method('charge')
            ->willReturn(false);

        $notifierMock = $this->createMock(NotificationServiceInterface::class);
        $notifierMock->expects($this->never())
            ->method('notify');

        $processor = new OrderProcessor($gatewayMock, $notifierMock);
        $result = $processor->processOrder('bob@example.com', 500, 'USD');

        $this->assertSame('failed', $result['status']);
        $this->assertSame('payment_declined', $result['reason']);
    }

    public function testConfiguredMockHelper(): void
    {
        $gatewayMock = $this->createConfiguredMock(PaymentGatewayInterface::class, [
            'charge' => true,
            'getTransactionId' => 'TX_CONFIGURED_1234',
            'refund' => true,
        ]);

        $this->assertTrue($gatewayMock->charge(50, 'GBP'));
        $this->assertSame('TX_CONFIGURED_1234', $gatewayMock->getTransactionId());
        $this->assertTrue($gatewayMock->refund('TX_CONFIGURED_1234'));
    }
}
