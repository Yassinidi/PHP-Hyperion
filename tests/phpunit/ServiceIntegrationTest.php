<?php
declare(strict_types=1);

namespace Tests\PHPUnit;

require_once __DIR__ . '/fixtures.php';
require_once __DIR__ . '/DatabaseRepositoryTest.php';

use PDO;
use RuntimeException;
use PHPUnit\Framework\TestCase;

class OrderCheckoutService
{
    public function __construct(
        private UserRepository $userRepo,
        private OrderRepository $orderRepo,
        private PaymentGatewayInterface $paymentGateway,
        private MailerInterface $mailer,
        private AuditLoggerInterface $auditLogger
    ) {}

    public function checkout(int $userId, string $orderNumber, array $items): array
    {
        $user = $this->userRepo->findById($userId);
        if (!$user) {
            throw new RuntimeException("User with ID {$userId} does not exist.");
        }

        if (empty($items)) {
            throw new RuntimeException("Cannot checkout an empty cart.");
        }

        $totalCents = 0;
        foreach ($items as $item) {
            $totalCents += $item['unit_price_cents'] * $item['quantity'];
        }

        // 1. Charge payment via gateway
        $chargeSuccess = $this->paymentGateway->charge($totalCents, $user['email']);
        if (!$chargeSuccess) {
            $this->auditLogger->recordEvent('checkout.failed.payment_declined', [
                'user_id' => $userId,
                'order_number' => $orderNumber,
                'amount_cents' => $totalCents,
            ]);
            throw new RuntimeException("Payment of \${$totalCents} was declined.");
        }

        $txId = $this->paymentGateway->getTransactionId() ?? 'TX-DEFAULT';

        // 2. Persist to database
        $orderId = $this->orderRepo->createOrderWithItems($orderNumber, $userId, $totalCents, 'USD', $items);

        // 3. Send confirmation email
        $this->mailer->send(
            $user['email'],
            "Order Confirmation #{$orderNumber}",
            "Thank you for your order of " . count($items) . " items. Transaction ID: {$txId}"
        );

        // 4. Record audit event
        $this->auditLogger->recordEvent('checkout.success', [
            'order_id' => $orderId,
            'order_number' => $orderNumber,
            'user_id' => $userId,
            'tx_id' => $txId,
            'total_cents' => $totalCents,
        ]);

        return [
            'order_id' => $orderId,
            'order_number' => $orderNumber,
            'transaction_id' => $txId,
            'total_cents' => $totalCents,
            'status' => 'pending',
        ];
    }
}

final class ServiceIntegrationTest extends TestCase
{
    private PDO $pdo;
    private UserRepository $userRepo;
    private OrderRepository $orderRepo;

    protected function setUp(): void
    {
        $this->pdo = new PDO('sqlite::memory:');
        $this->pdo->setAttribute(PDO::ATTR_ERRMODE, PDO::ERRMODE_EXCEPTION);

        $this->userRepo = new UserRepository($this->pdo);
        $this->userRepo->createTable();

        $this->orderRepo = new OrderRepository($this->pdo);
        $this->orderRepo->createTables();
    }

    public function testSuccessfulCheckoutFlowWithMocks(): void
    {
        $userId = $this->userRepo->insert('David Miller', 'david@hyperion.dev');

        // Create Mocks
        $gatewayMock = $this->createMock(PaymentGatewayInterface::class);
        $gatewayMock->expects($this->once())
            ->method('charge')
            ->with(25000, 'david@hyperion.dev')
            ->willReturn(true);
        $gatewayMock->expects($this->once())
            ->method('getTransactionId')
            ->willReturn('TX-HYPERION-9988');

        $mailerMock = $this->createMock(MailerInterface::class);
        $mailerMock->expects($this->once())
            ->method('send')
            ->with(
                'david@hyperion.dev',
                'Order Confirmation #ORD-SUCCESS-1',
                $this->stringContains('Transaction ID: TX-HYPERION-9988')
            )
            ->willReturn(true);

        $loggerMock = $this->createMock(AuditLoggerInterface::class);
        $loggerMock->expects($this->once())
            ->method('recordEvent')
            ->with(
                'checkout.success',
                $this->callback(function (array $payload) use ($userId) {
                    return $payload['order_number'] === 'ORD-SUCCESS-1'
                        && $payload['user_id'] === $userId
                        && $payload['tx_id'] === 'TX-HYPERION-9988'
                        && $payload['total_cents'] === 25000;
                })
            );

        $service = new OrderCheckoutService(
            $this->userRepo,
            $this->orderRepo,
            $gatewayMock,
            $mailerMock,
            $loggerMock
        );

        $items = [
            ['name' => 'Hyperion Server License', 'unit_price_cents' => 25000, 'quantity' => 1]
        ];

        $result = $service->checkout($userId, 'ORD-SUCCESS-1', $items);

        $this->assertSame('ORD-SUCCESS-1', $result['order_number']);
        $this->assertSame('TX-HYPERION-9988', $result['transaction_id']);
        $this->assertSame(25000, $result['total_cents']);

        // Verify Database Persistence
        $order = $this->orderRepo->findOrderWithItems($result['order_id']);
        $this->assertNotNull($order);
        $this->assertSame('ORD-SUCCESS-1', $order['order_number']);
        $this->assertSame(25000, (int) $order['total_cents']);
        $this->assertCount(1, $order['items']);
    }

    public function testDeclinedPaymentAbortsAndNeverSendsEmail(): void
    {
        $userId = $this->userRepo->insert('Elena Rostova', 'elena@hyperion.dev');

        $gatewayMock = $this->createMock(PaymentGatewayInterface::class);
        $gatewayMock->expects($this->once())
            ->method('charge')
            ->willReturn(false); // Declined!

        $mailerMock = $this->createMock(MailerInterface::class);
        $mailerMock->expects($this->never())->method('send'); // Must NEVER be called!

        $loggerMock = $this->createMock(AuditLoggerInterface::class);
        $loggerMock->expects($this->once())
            ->method('recordEvent')
            ->with(
                'checkout.failed.payment_declined',
                $this->callback(fn (array $p) => $p['order_number'] === 'ORD-DECLINED-1')
            );

        $service = new OrderCheckoutService(
            $this->userRepo,
            $this->orderRepo,
            $gatewayMock,
            $mailerMock,
            $loggerMock
        );

        $items = [
            ['name' => 'Cloud Node', 'unit_price_cents' => 5000, 'quantity' => 2]
        ];

        $this->expectException(RuntimeException::class);
        $this->expectExceptionMessage('Payment of $10000 was declined');

        try {
            $service->checkout($userId, 'ORD-DECLINED-1', $items);
        } finally {
            // Verify NO order was persisted to database
            $pendingOrders = $this->orderRepo->findOrdersByStatus('pending');
            $this->assertCount(0, $pendingOrders);
        }
    }
}
