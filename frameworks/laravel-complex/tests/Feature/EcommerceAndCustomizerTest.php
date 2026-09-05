<?php

namespace Tests\Feature;

use App\Enums\OrderStatus;
use App\Models\ActivityLog;
use App\Models\Order;
use App\Models\Page;
use App\Models\PageSection;
use App\Models\Product;
use App\Models\Theme;
use App\Services\CartService;
use App\Services\ThemeService;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Tests\TestCase;

class EcommerceAndCustomizerTest extends TestCase
{
    protected function setUp(): void
    {
        parent::setUp();
        // Ensure seeder is executed
        $this->seed(\Database\Seeders\ThemeAndPageSeeder::class);
    }

    public function test_active_theme_resolution_and_css_variables(): void
    {
        $themeService = app(ThemeService::class);
        $activeTheme = $themeService->getActiveTheme();

        $this->assertNotNull($activeTheme);
        $this->assertEquals('hyperion-slate', $activeTheme->slug);

        $css = $activeTheme->toCssVariables();
        $this->assertStringContainsString('--accent: #6366f1;', $css);
        $this->assertStringContainsString('--bg-base: #0a0e17;', $css);
        $this->assertStringContainsString('--theme-radius: 12px;', $css);
    }

    public function test_theme_switching_api(): void
    {
        $response = $this->postJson('/api/v1/customizer/theme/activate', [
            'slug' => 'cyberpunk-neon',
        ]);

        $response->assertStatus(200)
            ->assertJson([
                'success' => true,
                'theme' => [
                    'slug' => 'cyberpunk-neon',
                    'is_active' => true,
                ],
            ]);

        $this->assertTrue(Theme::where('slug', 'cyberpunk-neon')->first()->is_active);
        $this->assertFalse(Theme::where('slug', 'hyperion-slate')->first()->is_active);
    }

    public function test_theme_styles_update_api(): void
    {
        $response = $this->postJson('/api/v1/customizer/theme/styles', [
            'primary_color' => '#e11d48',
            'border_radius' => '18px',
        ]);

        $response->assertStatus(200)
            ->assertJson([
                'success' => true,
                'theme' => [
                    'primary_color' => '#e11d48',
                    'border_radius' => '18px',
                ],
            ]);

        $theme = app(ThemeService::class)->getActiveTheme();
        $this->assertEquals('#e11d48', $theme->primary_color);
        $this->assertEquals('18px', $theme->border_radius);
    }

    public function test_page_section_toggle_and_content_update(): void
    {
        $section = PageSection::first();
        $this->assertNotNull($section);

        $response = $this->postJson("/api/v1/customizer/sections/{$section->id}", [
            'title' => 'Customized Section Headline',
            'is_active' => false,
        ]);

        $response->assertStatus(200)->assertJson(['success' => true]);

        $section->refresh();
        $this->assertEquals('Customized Section Headline', $section->title);
        $this->assertFalse($section->is_active);
    }

    public function test_shopping_cart_lifecycle_and_coupon_calculation(): void
    {
        $product = Product::first();
        $this->assertNotNull($product);

        $cartService = app(CartService::class);
        $cartService->clearCart();

        // 1. Add item
        $summary = $cartService->addItem($product->id, 2);
        $this->assertEquals(2, $summary['item_count']);
        $this->assertEquals($product->price * 2, $summary['subtotal']);

        // 2. Apply Coupon HYPERION20 (20% off)
        $couponRes = $cartService->applyCoupon('HYPERION20');
        $this->assertTrue($couponRes['success']);

        $discountedSummary = $couponRes['summary'];
        $expectedDiscount = round($summary['subtotal'] * 0.20, 2);
        $this->assertEquals($expectedDiscount, $discountedSummary['discount_amount']);

        // 3. Verify total is calculated accurately
        $expectedTax = round(($summary['subtotal'] - $expectedDiscount) * 0.08, 2);
        $this->assertEquals($expectedTax, $discountedSummary['tax']);
    }

    public function test_full_checkout_flow_and_order_tracking(): void
    {
        $product = Product::where('stock', '>', 5)->first();
        $this->assertNotNull($product);
        $initialStock = $product->stock;

        $cartService = app(CartService::class);
        $cartService->clearCart();
        $summary = $cartService->addItem($product->id, 2);

        $checkoutData = [
            'first_name' => 'Jordan',
            'last_name' => 'Belfort',
            'email' => 'jordan@wallstreet-tech.com',
            'phone' => '+1 (555) 839-2019',
            'address' => '742 Evergreen Terrace',
            'city' => 'Springfield',
            'state' => 'OR',
            'postal_code' => '97477',
            'country' => 'United States',
            'shipping_speed' => 'standard',
            'payment_method' => 'credit_card',
        ];

        $cart = $cartService->getCart();
        $response = $this->withSession(['hyperion_ecommerce_cart' => $cart])
            ->post(route('checkout.process'), $checkoutData);

        // Assert created Order
        $order = Order::whereHas('customer', fn($q) => $q->where('email', 'jordan@wallstreet-tech.com'))->latest()->first();
        $this->assertNotNull($order);
        $this->assertEquals(OrderStatus::Paid, $order->status);
        $this->assertCount(1, $order->items);

        // Assert Stock Decremented
        $product->refresh();
        $this->assertEquals($initialStock - 2, $product->stock);

        // Assert ActivityLog Recorded
        $activity = ActivityLog::where('subject_id', $order->id)->first();
        $this->assertNotNull($activity);
        $this->assertStringContainsString($order->order_number, $activity->description);

        // Assert Redirected to Tracking
        $response->assertRedirect(route('store.track', ['order_number' => $order->order_number]));

        // Assert Tracking page renders with Order Number
        $trackResponse = $this->get(route('store.track', ['order_number' => $order->order_number]));
        $trackResponse->assertStatus(200)
            ->assertSee($order->order_number)
            ->assertSee('Order Confirmed & Placed!', false)
            ->assertSee('Cleanroom Prep');
    }

    public function test_customizer_studio_page_renders_successfully(): void
    {
        $response = $this->get(route('customizer.index'));
        $response->assertStatus(200)
            ->assertSee('Hyperion Theme & Page Studio', false)
            ->assertSee('No-Code Live Editor')
            ->assertSee('Hyperion Slate')
            ->assertSee('Cyberpunk Neon')
            ->assertSee('Minimalist Luxe');
    }
}

