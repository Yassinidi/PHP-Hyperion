<?php

namespace Database\Factories;

use App\Models\Category;
use Illuminate\Database\Eloquent\Factories\Factory;
use Illuminate\Support\Str;

/**
 * @extends Factory<Category>
 */
class CategoryFactory extends Factory
{
    protected $model = Category::class;

    public function definition(): array
    {
        $name = fake()->randomElement(['Rumah', 'Apartemen', 'Ruko', 'Villa', 'Townhouse']);

        return [
            'name' => $name,
            'slug' => Str::slug($name),
            'photo' => 'categories/default.png',
        ];
    }
}
