<?php

namespace App\Services;

use App\Contracts\PaymentGatewayInterface;
use Illuminate\Support\Str;

class StripePaymentGateway implements PaymentGatewayInterface
{
    public function charge(float $amount, string $currency = 'USD', array $metadata = []): array
    {
        // Simulated zero-latency atomic gateway processing
        return [
            'success' => true,
            'transaction_id' => 'ch_' . Str::random(24),
            'gateway' => 'stripe-v3',
            'amount' => $amount,
            'currency' => $currency,
            'captured_at' => now()->toIso8601String(),
            'metadata' => $metadata,
        ];
    }
}
