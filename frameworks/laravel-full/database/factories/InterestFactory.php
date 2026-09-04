<?php

namespace Database\Factories;

use App\Models\Bank;
use App\Models\House;
use App\Models\Interest;
use Illuminate\Database\Eloquent\Factories\Factory;

/**
 * @extends Factory<Interest>
 */
class InterestFactory extends Factory
{
    protected $model = Interest::class;

    public function definition(): array
    {
        return [
            'house_id' => House::factory(),
            'bank_id' => Bank::factory(),
            'interest' => fake()->randomFloat(2, 3, 15), // 3% to 15%
            'duration' => fake()->randomElement([5, 10, 15, 20, 25, 30]),
        ];
    }
}
