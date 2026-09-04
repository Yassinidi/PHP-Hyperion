<?php
declare(strict_types=1);

namespace Tests\PHPUnit;

use InvalidArgumentException;
use PHPUnit\Framework\TestCase;

enum OrderStatus: string
{
    case Pending = 'pending';
    case Paid = 'paid';
    case Shipped = 'shipped';
    case Cancelled = 'cancelled';
    case Refunded = 'refunded';

    public function canTransitionTo(OrderStatus $next): bool
    {
        return match ($this) {
            self::Pending => in_array($next, [self::Paid, self::Cancelled], true),
            self::Paid => in_array($next, [self::Shipped, self::Refunded], true),
            self::Shipped => $next === self::Refunded,
            self::Cancelled, self::Refunded => false,
        };
    }
}

readonly class Email
{
    public string $value;

    public function __construct(string $value)
    {
        $normalized = trim(strtolower($value));
        if (!filter_var($normalized, FILTER_VALIDATE_EMAIL)) {
            throw new InvalidArgumentException("Invalid email format: {$value}");
        }
        $this->value = $normalized;
    }

    public function equals(Email $other): bool
    {
        return $this->value === $other->value;
    }

    public function domain(): string
    {
        $parts = explode('@', $this->value);
        return $parts[1] ?? '';
    }
}

readonly class Money
{
    public function __construct(
        public int $amountCents,
        public string $currency = 'USD'
    ) {
        if ($amountCents < 0) {
            throw new InvalidArgumentException("Money amount cannot be negative: {$amountCents}");
        }
    }

    public function add(Money $other): Money
    {
        if ($this->currency !== $other->currency) {
            throw new InvalidArgumentException("Cannot add different currencies: {$this->currency} and {$other->currency}");
        }
        return new Money($this->amountCents + $other->amountCents, $this->currency);
    }

    public function multiply(float $multiplier): Money
    {
        if ($multiplier < 0) {
            throw new InvalidArgumentException("Multiplier cannot be negative: {$multiplier}");
        }
        return new Money((int) round($this->amountCents * $multiplier), $this->currency);
    }

    public function format(): string
    {
        $dollars = number_format($this->amountCents / 100, 2);
        return "{$this->currency} {$dollars}";
    }
}

class OrderItem
{
    public function __construct(
        public string $productId,
        public string $productName,
        public Money $unitPrice,
        public int $quantity
    ) {
        if ($quantity <= 0) {
            throw new InvalidArgumentException("Quantity must be greater than zero");
        }
    }

    public function subtotal(): Money
    {
        return $this->unitPrice->multiply($this->quantity);
    }
}

class Order
{
    /** @var OrderItem[] */
    private array $items = [];
    private OrderStatus $status = OrderStatus::Pending;
    private ?string $transactionId = null;

    public function __construct(
        public string $orderId,
        public Email $customerEmail,
        public string $currency = 'USD'
    ) {}

    public function addItem(OrderItem $item): void
    {
        if ($this->status !== OrderStatus::Pending) {
            throw new InvalidArgumentException("Cannot add items to an order in status: {$this->status->value}");
        }
        if ($item->unitPrice->currency !== $this->currency) {
            throw new InvalidArgumentException("Item currency {$item->unitPrice->currency} does not match order currency {$this->currency}");
        }
        $this->items[] = $item;
    }

    /**
     * @return OrderItem[]
     */
    public function getItems(): array
    {
        return $this->items;
    }

    public function calculateTotal(): Money
    {
        $total = new Money(0, $this->currency);
        foreach ($this->items as $item) {
            $total = $total->add($item->subtotal());
        }
        return $total;
    }

    public function getStatus(): OrderStatus
    {
        return $this->status;
    }

    public function markPaid(string $transactionId): void
    {
        if (!$this->status->canTransitionTo(OrderStatus::Paid)) {
            throw new InvalidArgumentException("Cannot transition from {$this->status->value} to paid");
        }
        $this->status = OrderStatus::Paid;
        $this->transactionId = $transactionId;
    }

    public function getTransactionId(): ?string
    {
        return $this->transactionId;
    }

    public function cancel(): void
    {
        if (!$this->status->canTransitionTo(OrderStatus::Cancelled)) {
            throw new InvalidArgumentException("Cannot transition from {$this->status->value} to cancelled");
        }
        $this->status = OrderStatus::Cancelled;
    }
}

final class DomainModelTest extends TestCase
{
    public function testEmailValueObjectValidationAndEquality(): void
    {
        $email1 = new Email('  USER@Domain.COM  ');
        $this->assertSame('user@domain.com', $email1->value);
        $this->assertSame('domain.com', $email1->domain());

        $email2 = new Email('user@domain.com');
        $this->assertTrue($email1->equals($email2));

        $this->expectException(InvalidArgumentException::class);
        new Email('not-an-email');
    }

    public function testMoneyValueObjectArithmeticAndFormatting(): void
    {
        $m1 = new Money(1550, 'USD'); // $15.50
        $m2 = new Money(450, 'USD');  // $4.50

        $sum = $m1->add($m2);
        $this->assertSame(2000, $sum->amountCents);
        $this->assertSame('USD 20.00', $sum->format());

        $scaled = $m1->multiply(3);
        $this->assertSame(4650, $scaled->amountCents);
        $this->assertSame('USD 46.50', $scaled->format());
    }

    public function testMoneyMismatchedCurrencyThrowsException(): void
    {
        $usd = new Money(1000, 'USD');
        $eur = new Money(1000, 'EUR');

        $this->expectException(InvalidArgumentException::class);
        $this->expectExceptionMessage('Cannot add different currencies');
        $usd->add($eur);
    }

    public function testOrderLifecycleAndCalculation(): void
    {
        $customer = new Email('buyer@hyperion.dev');
        $order = new Order('ORD-1001', $customer, 'USD');

        $this->assertSame(OrderStatus::Pending, $order->getStatus());
        $this->assertCount(0, $order->getItems());
        $this->assertSame(0, $order->calculateTotal()->amountCents);

        $item1 = new OrderItem('PROD-1', 'Mechanical Keyboard', new Money(9999, 'USD'), 1);
        $item2 = new OrderItem('PROD-2', 'Keycap Set', new Money(2500, 'USD'), 2);

        $order->addItem($item1);
        $order->addItem($item2);

        $this->assertCount(2, $order->getItems());
        $total = $order->calculateTotal();
        $this->assertSame(14999, $total->amountCents); // 99.99 + 50.00 = 149.99
        $this->assertSame('USD 149.99', $total->format());

        $order->markPaid('TX-998877');
        $this->assertSame(OrderStatus::Paid, $order->getStatus());
        $this->assertSame('TX-998877', $order->getTransactionId());
    }

    public function testInvalidOrderStatusTransitionThrowsException(): void
    {
        $customer = new Email('buyer@hyperion.dev');
        $order = new Order('ORD-1002', $customer, 'USD');
        $order->markPaid('TX-123');

        $this->expectException(InvalidArgumentException::class);
        $this->expectExceptionMessage('Cannot transition from paid to cancelled');
        $order->cancel();
    }
}
