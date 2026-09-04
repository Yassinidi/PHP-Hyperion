<?php

namespace Database\Factories;

use App\Models\Category;
use App\Models\City;
use App\Models\House;
use Illuminate\Database\Eloquent\Factories\Factory;
use Illuminate\Support\Str;

/**
 * @extends Factory<House>
 */
class HouseFactory extends Factory
{
    protected $model = House::class;

    public function definition(): array
    {
        $name = fake()->streetName().' Residence';

        return [
            'name' => $name,
            'slug' => Str::slug($name),
            'thumbnail' => 'houses/default.jpg',
            'about' => fake()->paragraph(),
            'price' => fake()->numberBetween(500000000, 5000000000), // 500M to 5B IDR
            'bedroom' => fake()->numberBetween(2, 6),
            'bathroom' => fake()->numberBetween(1, 4),
            'certificate' => fake()->randomElement(['SHM', 'SHGB', 'HGB']),
            'electric' => fake()->numberBetween(1300, 10000),
            'land_area' => fake()->numberBetween(50, 500),
            'building_area' => fake()->numberBetween(40, 400),
            'category_id' => Category::factory(),
            'city_id' => City::factory(),
        ];
    }
}
