<?php

namespace Database\Factories;

use App\Models\Bank;
use Illuminate\Database\Eloquent\Factories\Factory;

/**
 * @extends Factory<Bank>
 */
class BankFactory extends Factory
{
    protected $model = Bank::class;

    public function definition(): array
    {
        return [
            'name' => fake()->randomElement(['BCA', 'Mandiri', 'BNI', 'BRI', 'CIMB', 'Permata']),
            'photo' => 'banks/default.png',
        ];
    }
}
