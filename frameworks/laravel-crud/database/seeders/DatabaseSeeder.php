<?php

namespace Database\Seeders;

use App\Models\Category;
use App\Models\Product;
use Illuminate\Database\Seeder;
use Illuminate\Support\Str;

class DatabaseSeeder extends Seeder
{
    /**
     * Seed the application's database.
     */
    public function run(): void
    {
        $categories = [
            [
                'name' => 'High-Tech & Compute',
                'slug' => 'high-tech-compute',
                'icon' => '⚡',
                'color' => '#6366f1',
                'description' => 'Cutting-edge workstations, servers, and embedded silicon modules.'
            ],
            [
                'name' => 'Audio & Peripherals',
                'slug' => 'audio-peripherals',
                'icon' => '🎧',
                'color' => '#ec4899',
                'description' => 'Acoustic monitoring, mechanical keyboards, and precision peripherals.'
            ],
            [
                'name' => 'Workspace & Ergonomics',
                'slug' => 'workspace-ergonomics',
                'icon' => '🪑',
                'color' => '#10b981',
                'description' => 'Motorized sit-stand frames, monitor arms, and ergonomic seats.'
            ],
            [
                'name' => 'Networking & Storage',
                'slug' => 'networking-storage',
                'icon' => '🌐',
                'color' => '#0ea5e9',
                'description' => '10GbE fiber switches, NVMe storage arrays, and mesh nodes.'
            ],
        ];

        $categoryMap = [];
        foreach ($categories as $cat) {
            $categoryMap[$cat['slug']] = Category::create($cat);
        }

        $products = [
            [
                'category_slug' => 'high-tech-compute',
                'name' => 'Hyperion Silicon RISC-V Accelerator V2',
                'sku' => 'HYP-ACC-001',
                'price' => 1299.00,
                'cost' => 650.00,
                'stock' => 45,
                'status' => 'in_stock',
                'description' => 'Dedicated PCIe neural inference accelerator card with 128 TOPs compute density and zero-copy DMA pipeline.',
                'rating' => 4.95,
                'featured' => true,
                'image_url' => 'https://images.unsplash.com/photo-1591488320449-011701bb6704?w=600&auto=format&fit=crop&q=80',
            ],
            [
                'category_slug' => 'high-tech-compute',
                'name' => 'Quantum Core DevStation Pro (64 Cores)',
                'sku' => 'QNT-DEV-64C',
                'price' => 3899.00,
                'cost' => 2100.00,
                'stock' => 12,
                'status' => 'in_stock',
                'description' => 'Liquid-cooled 64-core developer workstation optimized for instant kernel compilation and large language models.',
                'rating' => 4.88,
                'featured' => true,
                'image_url' => 'https://images.unsplash.com/photo-1587202372775-e229f172b9d7?w=600&auto=format&fit=crop&q=80',
            ],
            [
                'category_slug' => 'audio-peripherals',
                'name' => 'StudioFlow Planar Magnetic Headphones',
                'sku' => 'AUD-PLN-990',
                'price' => 449.00,
                'cost' => 180.00,
                'stock' => 85,
                'status' => 'in_stock',
                'description' => 'Ultra-wide soundstage planar magnetic studio monitors with CNC aluminum frame and lambskin ear cushions.',
                'rating' => 4.92,
                'featured' => false,
                'image_url' => 'https://images.unsplash.com/photo-1505740420928-5e560c06d30e?w=600&auto=format&fit=crop&q=80',
            ],
            [
                'category_slug' => 'audio-peripherals',
                'name' => 'AeroMechanical Split 65% Keyboard',
                'sku' => 'KBD-AER-65S',
                'price' => 289.00,
                'cost' => 110.00,
                'stock' => 8,
                'status' => 'low_stock',
                'description' => 'Ergonomic split layout mechanical keyboard with hot-swappable tactile switches and per-key RGB illumination.',
                'rating' => 4.79,
                'featured' => true,
                'image_url' => 'https://images.unsplash.com/photo-1587829741301-dc798b83add3?w=600&auto=format&fit=crop&q=80',
            ],
            [
                'category_slug' => 'workspace-ergonomics',
                'name' => 'Vertex ErgoChair Kinetic Pro',
                'sku' => 'ERG-CHR-700',
                'price' => 799.00,
                'cost' => 340.00,
                'stock' => 0,
                'status' => 'out_of_stock',
                'description' => 'Dynamic lumbar synchronous tilt chair engineered with breathable elastomeric mesh and multi-axis 4D armrests.',
                'rating' => 4.85,
                'featured' => false,
                'image_url' => 'https://images.unsplash.com/photo-1580481077195-c946f338d728?w=600&auto=format&fit=crop&q=80',
            ],
            [
                'category_slug' => 'networking-storage',
                'name' => 'FlashBlade 8-Bay NVMe Enterprise SAN',
                'sku' => 'NET-SAN-08N',
                'price' => 2199.00,
                'cost' => 1050.00,
                'stock' => 19,
                'status' => 'in_stock',
                'description' => 'Dual 25GbE SFP28 NVMe storage array capable of 2.4 Million IOPS and hardware RAID encryption.',
                'rating' => 4.97,
                'featured' => true,
                'image_url' => 'https://images.unsplash.com/photo-1544716278-ca5e3f4abd8c?w=600&auto=format&fit=crop&q=80',
            ],
        ];

        foreach ($products as $p) {
            $catSlug = $p['category_slug'];
            unset($p['category_slug']);
            $p['category_id'] = $categoryMap[$catSlug]->id ?? null;
            $p['slug'] = Str::slug($p['name']);
            Product::create($p);
        }
    }
}
