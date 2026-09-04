<?php

namespace Database\Seeders;

use App\Models\Category;
use App\Models\Customer;
use App\Models\Order;
use App\Models\OrderItem;
use App\Models\Product;
use App\Models\Review;
use App\Models\Tag;
use Illuminate\Database\Seeder;
use Illuminate\Support\Str;

class EcommerceSeeder extends Seeder
{
    public function run(): void
    {
        // 1. Tags
        $tagNames = ['Trending', 'Best Seller', 'New Arrival', 'Limited Edition', 'Eco-Friendly', 'Pro'];
        $tags = [];
        foreach ($tagNames as $name) {
            $tags[] = Tag::create([
                'name' => $name,
                'slug' => Str::slug($name),
            ]);
        }

        // 2. Categories with parent/child relations
        $electronics = Category::create([
            'name' => 'Electronics',
            'slug' => 'electronics',
            'icon' => 'cpu',
            'is_featured' => true,
        ]);

        $laptops = Category::create([
            'parent_id' => $electronics->id,
            'name' => 'Laptops & Computers',
            'slug' => 'laptops-computers',
            'icon' => 'laptop',
        ]);

        $audio = Category::create([
            'parent_id' => $electronics->id,
            'name' => 'Audio & Headphones',
            'slug' => 'audio-headphones',
            'icon' => 'headphones',
        ]);

        $smartphones = Category::create([
            'parent_id' => $electronics->id,
            'name' => 'Smartphones & Tablets',
            'slug' => 'smartphones-tablets',
            'icon' => 'smartphone',
        ]);

        $home = Category::create([
            'name' => 'Home & Living',
            'slug' => 'home-living',
            'icon' => 'home',
            'is_featured' => true,
        ]);

        $furniture = Category::create([
            'parent_id' => $home->id,
            'name' => 'Ergonomic Furniture',
            'slug' => 'ergonomic-furniture',
            'icon' => 'armchair',
        ]);

        $categories = [$laptops, $audio, $smartphones, $furniture];

        // 3. Products
        $productSamples = [
            ['name' => 'Apex Pro Ultrabook 16"', 'price' => 1899.00, 'cat' => $laptops, 'stock' => 25],
            ['name' => 'ZenBook Hyper Thin 14"', 'price' => 1249.00, 'cat' => $laptops, 'stock' => 18],
            ['name' => 'WorkStation Rig X9000', 'price' => 2899.00, 'cat' => $laptops, 'stock' => 10],
            ['name' => 'SonicBlast Noise-Canceling ANC', 'price' => 299.00, 'cat' => $audio, 'stock' => 60],
            ['name' => 'StudioFlow Wireless Earbuds', 'price' => 149.00, 'cat' => $audio, 'stock' => 100],
            ['name' => 'BassPulse High-Res DAC Speaker', 'price' => 450.00, 'cat' => $audio, 'stock' => 35],
            ['name' => 'Nebula Phone 15 Pro Max', 'price' => 1199.00, 'cat' => $smartphones, 'stock' => 45],
            ['name' => 'Quantum Slate OLED 12"', 'price' => 799.00, 'cat' => $smartphones, 'stock' => 30],
            ['name' => 'ErgoChair Autonomous Pro', 'price' => 489.00, 'cat' => $furniture, 'stock' => 20],
            ['name' => 'Smart Desk Motorized Bamboo', 'price' => 699.00, 'cat' => $furniture, 'stock' => 15],
            ['name' => 'Nordic Lumbar Executive Recliner', 'price' => 850.00, 'cat' => $furniture, 'stock' => 12],
            ['name' => 'Magnetic Desk Organizer Mat', 'price' => 59.00, 'cat' => $furniture, 'stock' => 150],
        ];

        $products = [];
        foreach ($productSamples as $idx => $sample) {
            $p = Product::create([
                'category_id' => $sample['cat']->id,
                'sku' => 'SKU-' . strtoupper(Str::random(6)),
                'name' => $sample['name'],
                'slug' => Str::slug($sample['name']) . '-' . ($idx + 1),
                'description' => "Engineered for elite performance and durability. Features aerospace-grade craftsmanship and industrial testing standards.",
                'price' => $sample['price'],
                'stock' => $sample['stock'],
                'is_active' => true,
            ]);

            // Attach 2 random tags
            $p->tags()->attach([$tags[$idx % count($tags)]->id, $tags[($idx + 1) % count($tags)]->id]);
            $products[] = $p;
        }

        // 4. Customers
        $customerData = [
            ['name' => 'Alex Turner', 'email' => 'alex@example.com', 'tier' => 'vip'],
            ['name' => 'Elena Rostova', 'email' => 'elena@example.com', 'tier' => 'gold'],
            ['name' => 'Marcus Vance', 'email' => 'marcus@example.com', 'tier' => 'standard'],
            ['name' => 'Sophia Lin', 'email' => 'sophia@example.com', 'tier' => 'vip'],
            ['name' => 'David Kim', 'email' => 'david@example.com', 'tier' => 'standard'],
        ];

        $customers = [];
        foreach ($customerData as $c) {
            $customers[] = Customer::create($c);
        }

        // 5. Orders & OrderItems
        foreach ($customers as $i => $cust) {
            $orderTotal = 0.0;
            $orderItemsData = [];

            // 2 items per order
            $p1 = $products[$i % count($products)];
            $p2 = $products[($i + 3) % count($products)];

            $orderItemsData[] = [
                'product_id' => $p1->id,
                'quantity' => 1,
                'unit_price' => $p1->price,
                'subtotal' => $p1->price,
            ];
            $orderTotal += $p1->price;

            $orderItemsData[] = [
                'product_id' => $p2->id,
                'quantity' => 2,
                'unit_price' => $p2->price,
                'subtotal' => $p2->price * 2,
            ];
            $orderTotal += $p2->price * 2;

            $order = Order::create([
                'customer_id' => $cust->id,
                'order_number' => 'ORD-' . strtoupper(Str::random(8)),
                'total_amount' => $orderTotal,
                'status' => 'paid',
                'payment_method' => ($i % 2 === 0) ? 'credit_card' : 'paypal',
            ]);

            foreach ($orderItemsData as $oid) {
                $order->items()->create($oid);
            }
        }

        // 6. Reviews
        $comments = [
            'Absolutely exceeded my expectations! Unrivaled build quality and lightning performance.',
            'Stunning design and ergonomic excellence. Recommended to all colleagues.',
            'Incredible value for the premium engineering. Sub-millisecond response speeds.',
            'Five stars across the board. Solid craftsmanship.',
            'Works seamlessly right out of the box. Highly satisfied customer.',
        ];

        foreach ($products as $idx => $prod) {
            Review::create([
                'product_id' => $prod->id,
                'customer_id' => $customers[$idx % count($customers)]->id,
                'rating' => 4 + ($idx % 2), // 4 or 5
                'comment' => $comments[$idx % count($comments)],
            ]);
        }
    }
}
