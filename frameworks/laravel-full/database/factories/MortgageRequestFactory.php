<?php

namespace Database\Factories;

use App\Models\House;
use App\Models\Interest;
use App\Models\MortgageRequest;
use App\Models\User;
use Illuminate\Database\Eloquent\Factories\Factory;

/**
 * @extends Factory<MortgageRequest>
 */
class MortgageRequestFactory extends Factory
{
    protected $model = MortgageRequest::class;

    public function definition(): array
    {
        $house = House::factory()->create();
        $interest = Interest::factory()->create(['house_id' => $house->id]);
        $user = User::factory()->create();

        $housePrice = $house->price;
        $dpPercentage = fake()->randomElement([10, 20, 30, 40, 50]);
        $dpTotal = $housePrice * ($dpPercentage / 100);
        $loanTotal = $housePrice - $dpTotal;

        return [
            'user_id' => $user->id,
            'house_id' => $house->id,
            'interest_id' => $interest->id,
            'duration' => fake()->randomElement([5, 10, 15, 20, 25]),
            'bank_name' => fake()->randomElement(['BCA', 'Mandiri', 'BNI', 'BRI']),
            'interest' => $interest->interest,
            'dp_total_amount' => $dpTotal,
            'dp_percentage' => $dpPercentage,
            'loan_total_amount' => $loanTotal,
            'loan_interest_total_amount' => $loanTotal * 0.05, // Simplified
            'house_price' => $housePrice,
            'monthly_amount' => fake()->numberBetween(1000000, 10000000),
            'status' => fake()->randomElement(['Waiting for Bank', 'Approved', 'Rejected']),
            'documents' => 'documents/test-document.pdf',
        ];
    }

    public function approved(): static
    {
        return $this->state(fn (array $attributes) => [
            'status' => 'Approved',
        ]);
    }

    public function waiting(): static
    {
        return $this->state(fn (array $attributes) => [
            'status' => 'Waiting for Bank',
        ]);
    }
}
