<?php

namespace Database\Factories;

use App\Models\Installment;
use App\Models\MortgageRequest;
use Illuminate\Database\Eloquent\Factories\Factory;

/**
 * @extends Factory<Installment>
 */
class InstallmentFactory extends Factory
{
    protected $model = Installment::class;

    public function definition(): array
    {
        $monthlyAmount = fake()->numberBetween(1000000, 10000000);
        $tax = $monthlyAmount * 0.11;
        $insurance = 900000;

        return [
            'mortgage_request_id' => MortgageRequest::factory(),
            'no_of_payment' => fake()->numberBetween(1, 300),
            'sub_total_amount' => $monthlyAmount,
            'total_tax_amount' => $tax,
            'insurance_amount' => $insurance,
            'grand_total_amount' => $monthlyAmount + $tax + $insurance,
            'proof' => null,
            'is_paid' => fake()->boolean(),
            'payment_type' => fake()->randomElement(['Midtrans', 'Manual']),
            'remaining_loan_amount' => fake()->numberBetween(0, 1000000000),
        ];
    }

    public function paid(): static
    {
        return $this->state(fn (array $attributes) => [
            'is_paid' => true,
        ]);
    }

    public function unpaid(): static
    {
        return $this->state(fn (array $attributes) => [
            'is_paid' => false,
        ]);
    }
}
