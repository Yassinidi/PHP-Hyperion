<?php
declare(strict_types=1);

namespace Tests\PHPUnit;

use InvalidArgumentException;
use PDO;

interface PaymentGatewayInterface
{
    public function charge(int $amountCents, string $customerEmail): bool;
    public function refund(string $transactionId): bool;
    public function getTransactionId(): ?string;
}

interface MailerInterface
{
    public function send(string $toEmail, string $subject, string $body): bool;
}

interface AuditLoggerInterface
{
    public function recordEvent(string $eventType, array $payload): void;
}
