<?php

namespace Database\Seeders;

use App\Models\Page;
use App\Models\PageSection;
use App\Models\StoreSetting;
use App\Models\Theme;
use App\Services\ThemeService;
use Illuminate\Database\Seeder;

class ThemeAndPageSeeder extends Seeder
{
    /**
     * Run the database seeds.
     */
    public function run(): void
    {
        // 1. Seed Themes
        $themeService = new ThemeService();
        $presets = $themeService->getDefaultThemePresets();

        foreach ($presets as $slug => $data) {
            Theme::updateOrCreate(
                ['slug' => $slug],
                array_merge($data, [
                    'is_active' => ($slug === 'hyperion-slate'),
                ])
            );
        }

        // 2. Seed Store Settings
        $settings = [
            'store_name' => 'Hyperion Enterprise Store',
            'store_tagline' => 'Ultra-High Performance Pro Commerce',
            'announcement_enabled' => 'true',
            'announcement_text' => '⚡ FLASH LAUNCH: Use coupon code HYPERION20 for 20% off all catalog items!',
            'announcement_link' => '/#products',
            'currency_symbol' => '$',
            'contact_email' => 'support@hyperion-store.io',
        ];

        foreach ($settings as $key => $val) {
            StoreSetting::set($key, $val);
        }

        // 3. Seed Home Page & Modular Sections
        $homePage = Page::updateOrCreate(
            ['slug' => 'home'],
            [
                'title' => 'Home Storefront',
                'meta_description' => 'Browse our high-performance tech hardware, smart devices, and developer tools.',
                'is_published' => true,
                'is_system' => true,
            ]
        );

        // Clear existing sections for clean seed
        $homePage->sections()->delete();

        $sections = [
            [
                'section_type' => 'hero',
                'title' => 'Next-Gen Computing & Enterprise Hardware',
                'subtitle' => 'Precision engineered developer workstations, low-latency neural wearables, and cloud appliances powered by PHP-Hyperion.',
                'sort_order' => 1,
                'is_active' => true,
                'content' => [
                    'badge' => '2026 FLAGSHIP RELEASE',
                    'cta_primary_text' => 'Explore Catalog',
                    'cta_primary_link' => '#products',
                    'cta_secondary_text' => 'Customizer Studio',
                    'cta_secondary_link' => '/admin/customizer',
                    'banner_image' => 'https://images.unsplash.com/photo-1550745165-9bc0b252726f?auto=format&fit=crop&w=1200&q=80',
                ],
            ],
            [
                'section_type' => 'features_bar',
                'title' => 'Why Choose Hyperion Pro Store',
                'subtitle' => 'Engineered for reliability, velocity, and zero-downtime scalability.',
                'sort_order' => 2,
                'is_active' => true,
                'content' => [
                    'features' => [
                        [
                            'icon' => '⚡',
                            'title' => 'Sub-Millisecond Speed',
                            'description' => 'Powered by the Hyperion Rust VM delivering 198,000 req/s with zero-copy caching.',
                        ],
                        [
                            'icon' => '🛡️',
                            'title' => 'Military-Grade Security',
                            'description' => 'End-to-end encrypted transactions, automated audit trails, and instant fraud checks.',
                        ],
                        [
                            'icon' => '🚀',
                            'title' => 'Worldwide Express Delivery',
                            'description' => 'Free 48-hour global courier dispatch on all hardware orders over $100.',
                        ],
                        [
                            'icon' => '🎨',
                            'title' => '100% No-Code Customizer',
                            'description' => 'Switch visual themes, modify styling tokens, and toggle sections in real time.',
                        ],
                    ],
                ],
            ],
            [
                'section_type' => 'flash_sale',
                'title' => 'Limited-Time Velocity Sale',
                'subtitle' => 'Save up to 40% on neural processors, OLED displays, and developer peripherals.',
                'sort_order' => 3,
                'is_active' => true,
                'content' => [
                    'discount_badge' => 'SAVE 20% WITH HYPERION20',
                    'end_time_text' => 'Offer Ends Soon',
                    'button_text' => 'Claim Discount Now',
                    'button_link' => '#products',
                ],
            ],
            [
                'section_type' => 'featured_products',
                'title' => 'Curated Hardware & Peripherals',
                'subtitle' => 'Handpicked by hardware architects for maximum reliability and throughput.',
                'sort_order' => 4,
                'is_active' => true,
                'content' => [
                    'display_limit' => 8,
                    'view_all_link' => '#products',
                ],
            ],
            [
                'section_type' => 'testimonials',
                'title' => 'Trusted by 10,000+ Engineers Worldwide',
                'subtitle' => 'Hear from engineering leads and architects running on Hyperion.',
                'sort_order' => 5,
                'is_active' => true,
                'content' => [
                    'testimonials' => [
                        [
                            'quote' => 'Hyperion transformed our e-commerce stack. We saw server response times drop from 95 ms to 0.7 ms overnight without touching a single line of Laravel code.',
                            'author' => 'Sarah Jenkins',
                            'role' => 'Principal Architect at CloudScale',
                            'rating' => 5,
                        ],
                        [
                            'quote' => 'The customizable theme engine allowed our design team to test live dark mode and cyberpunk themes without waiting on developers. Absolutely incredible tool.',
                            'author' => 'Marcus Vance',
                            'role' => 'Head of Product at NexusTech',
                            'rating' => 5,
                        ],
                        [
                            'quote' => 'Sustained 100,000 concurrent requests during our black friday flash sale with 0 dropped sockets and 0% memory leaks. Unmatched stability.',
                            'author' => 'Dr. Elena Rostova',
                            'role' => 'VP of Infrastructure at HyperByte',
                            'rating' => 5,
                        ],
                    ],
                ],
            ],
            [
                'section_type' => 'newsletter',
                'title' => 'Join the Hyperion Hardware Insider',
                'subtitle' => 'Get early access to benchmark releases, new hardware drops, and secret flash coupons.',
                'sort_order' => 6,
                'is_active' => true,
                'content' => [
                    'incentive' => 'Get 10% off your first checkout when you subscribe today.',
                    'button_text' => 'Subscribe to Insiders',
                ],
            ],
        ];

        foreach ($sections as $sec) {
            PageSection::create(array_merge($sec, ['page_id' => $homePage->id]));
        }

        // 4. Seed an About Us custom page
        $aboutPage = Page::updateOrCreate(
            ['slug' => 'about'],
            [
                'title' => 'About Hyperion Enterprise Store',
                'meta_description' => 'Learn about our engineering philosophy and ultra-high-speed hardware runtime.',
                'is_published' => true,
                'is_system' => false,
            ]
        );
        $aboutPage->sections()->delete();
        PageSection::create([
            'page_id' => $aboutPage->id,
            'section_type' => 'hero',
            'title' => 'Pioneering The Future of High-Speed Commerce',
            'subtitle' => 'We combine cutting-edge hardware components with a pure-Rust asynchronous execution runtime.',
            'sort_order' => 1,
            'is_active' => true,
            'content' => [
                'badge' => 'ABOUT HYPERION',
                'cta_primary_text' => 'Browse Store',
                'cta_primary_link' => '/',
            ],
        ]);
    }
}
