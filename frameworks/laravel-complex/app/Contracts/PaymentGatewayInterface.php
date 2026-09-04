<?php

namespace App\Contracts;

interface PaymentGatewayInterface
{
    /**
     * Charge payment for an order
     *
     * @param float $amount
     * @param string $currency
     * @param array $metadata
     * @return array ['success' => bool, 'transaction_id' => string, 'gateway' => string]
     */
    public function charge(float $amount, string $currency = 'USD', array $metadata = []): array;
}
