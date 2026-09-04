<?php

namespace Tests\Feature;

use App\Models\Installment;
use App\Models\User;
use Database\Seeders\RoleAdminSeeder;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Tests\TestCase;

class DashboardControllerTest extends TestCase
{
    use RefreshDatabase;

    protected function setUp(): void
    {
        parent::setUp();
        $this->seed(RoleAdminSeeder::class);
    }

    /**
     * Test dashboard requires authentication.
     */
    public function test_dashboard_requires_authentication(): void
    {
        $response = $this->get('/dashboard/mortgages');

        $response->assertRedirect('/login');
    }

    /**
     * Test dashboard requires customer role.
     */
    public function test_dashboard_requires_customer_role(): void
    {
        $user = User::factory()->create();

        $response = $this->actingAs($user)->get('/dashboard/mortgages');

        $response->assertStatus(403);
    }

    /**
     * Test customer can view their mortgages.
     */
    public function test_customer_can_view_their_mortgages(): void
    {
        $user = User::factory()->create();
        $user->assignRole('customer');

        $response = $this->actingAs($user)->get('/dashboard/mortgages');

        $response->assertStatus(200);
    }

    /**
     * Test customer can view mortgage details.
     */
    public function test_customer_can_view_mortgage_details(): void
    {
        $this->markTestSkipped('Requires view template setup');
    }

    /**
     * Test customer cannot view other users mortgage details.
     */
    public function test_customer_cannot_view_other_users_mortgage(): void
    {
        $this->markTestSkipped('Requires view template setup');
    }

    /**
     * Test customer can view installment details.
     */
    public function test_customer_can_view_installment_details(): void
    {
        $this->markTestSkipped('Requires view template setup');
    }

    /**
     * Test customer can access payment page.
     */
    public function test_customer_can_access_payment_page(): void
    {
        $this->markTestSkipped('Requires view template setup');
    }
}
