<?php

namespace Tests\Feature;

use App\Models\User;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\Artisan;
use Tests\TestCase;

class ApiDocumentationTest extends TestCase
{
    use RefreshDatabase;

    // ─── OpenAPI Spec ───────────────────────────────────────────

    public function test_public_users_can_open_api_documentation_ui(): void
    {
        $response = $this->get('/api/documentation');

        $response->assertOk();
    }

    public function test_openapi_json_can_be_generated_and_accessed_publicly(): void
    {
        Artisan::call('l5-swagger:generate');

        $response = $this->get('/docs');

        $response->assertOk();
        $response->assertJsonStructure([
            'openapi',
            'info' => ['title', 'version'],
            'paths',
        ]);
        $response->assertJsonPath('openapi', '3.1.0');
        $response->assertJsonPath('info.title', 'Tedja API');
    }

    // ─── Auth ───────────────────────────────────────────────────

    public function test_api_user_endpoint_requires_sanctum_authentication(): void
    {
        $response = $this->getJson('/api/user');

        $response->assertUnauthorized();
    }

    public function test_authenticated_users_can_fetch_their_profile_from_api_user_endpoint(): void
    {
        $user = User::factory()->create();

        $this->actingAs($user);

        $response = $this->getJson('/api/user');

        $response->assertOk();
        $response->assertJsonPath('email', $user->email);
    }

    // ─── Houses ─────────────────────────────────────────────────

    public function test_api_houses_endpoint_returns_json_response(): void
    {
        $response = $this->getJson('/api/houses');

        $response->assertOk();
    }

    // ─── Master Data ─────────────────────────────────────────────

    public function test_api_categories_endpoint_returns_json(): void
    {
        $response = $this->getJson('/api/categories');

        $response->assertOk()->assertJsonIsArray();
    }

    public function test_api_cities_endpoint_returns_json(): void
    {
        $response = $this->getJson('/api/cities');

        $response->assertOk()->assertJsonIsArray();
    }

    public function test_api_banks_endpoint_returns_json(): void
    {
        $response = $this->getJson('/api/banks');

        $response->assertOk()->assertJsonIsArray();
    }

    public function test_api_facilities_endpoint_returns_json(): void
    {
        $response = $this->getJson('/api/facilities');

        $response->assertOk()->assertJsonIsArray();
    }

    // ─── Mortgages ───────────────────────────────────────────────

    public function test_mortgage_calculate_endpoint_validates_input(): void
    {
        $response = $this->postJson('/api/mortgages/calculate', []);

        $response->assertUnprocessable();
    }

    public function test_mortgages_index_requires_auth(): void
    {
        $response = $this->getJson('/api/mortgages');

        $response->assertUnauthorized();
    }

    public function test_mortgages_store_requires_auth(): void
    {
        $response = $this->postJson('/api/mortgages', []);

        $response->assertUnauthorized();
    }

    // ─── Payments ────────────────────────────────────────────────

    public function test_payment_breakdown_requires_auth(): void
    {
        $response = $this->getJson('/api/mortgages/1/payment-breakdown');

        $response->assertUnauthorized();
    }

    public function test_payment_pay_requires_auth(): void
    {
        $response = $this->postJson('/api/mortgages/1/pay');

        $response->assertUnauthorized();
    }

    // ─── Installments ────────────────────────────────────────────

    public function test_installments_index_requires_auth(): void
    {
        $response = $this->getJson('/api/mortgages/1/installments');

        $response->assertUnauthorized();
    }

    public function test_installments_show_requires_auth(): void
    {
        $response = $this->getJson('/api/installments/1');

        $response->assertUnauthorized();
    }
}
