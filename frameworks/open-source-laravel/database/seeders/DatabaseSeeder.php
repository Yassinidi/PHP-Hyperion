<?php

namespace Database\Seeders;

use App\Models\Product;
use App\Models\User;
// use Illuminate\Database\Console\Seeds\WithoutModelEvents;
use Illuminate\Database\Seeder;
use Illuminate\Support\Facades\DB;
use Illuminate\Database\Eloquent\Model;
use Illuminate\Support\Arr;

class DatabaseSeeder extends Seeder
{
    /**
     * Seed the application's database.
     */
    public function run(): void
    {
        // User::factory(10)->create();
        Model::unguard();
        DB::table('users')->truncate();
        DB::table('products')->truncate();
        $options = [
            'Colour' => [
                'Blue',
                'Green',
                'Black',
                'White',
                'Pink',
                'Red',
                'Yellow',
                'Navy',
                'Crimson'
            ],

            'Size' => ['Small', 'Medium', 'Large', 'X Large', 'XX Large']
        ];
        Product::factory(10)
            ->has(
                Product::factory()
                    ->count(3)
                    ->state(function (array $attributes) use ($options) {
                        $size = Arr::random($options['Size'], 1);
                        $colour = Arr::random($options['Colour'], 1);
                        return [
                            'options' => ['Size', 'Colour'],
                            'option_values' => [
                                'Size' => $size[0],
                                'Colour' => $colour[0]
                            ]
                        ];
                    })
                ,
                'variants'
            )
            ->create();

        Model::reguard();
        User::factory()->create([
            'name' => 'Test User',
            'email' => 'test@example.com',
        ]);
    }
}
