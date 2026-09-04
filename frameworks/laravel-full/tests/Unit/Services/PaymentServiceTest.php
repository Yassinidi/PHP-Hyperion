<?php

namespace Tests\Unit\Services;

use App\Models\Bank;
use App\Models\City;
use App\Models\House;
use App\Models\Installment;
use App\Models\Interest;
use App\Models\MortgageRequest;
use App\Models\User;
use App\Services\MidtransService;
use App\Services\PaymentService;
use Database\Seeders\RoleAdminSeeder;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Mockery;
use Tests\TestCase;

class PaymentServiceTest extends TestCase
{
    use RefreshDatabase;

    private PaymentService $service;

    private $mockMidtrans;

    protected function setUp(): void
    {
        parent::setUp();
        $this->seed(RoleAdminSeeder::class);
        $this->mockMidtrans = Mockery::mock(MidtransService::class);
        $this->service = new PaymentService($this->mockMidtrans);
    }

    /**
     * Test createPayment calls Midtrans with correct parameters.
     */
    public function test_create_payment_calls_midtrans_with_correct_params(): void
    {
        $user = User::factory()->create();
        $user->assignRole('customer');
        $this->actingAs($user);

        $city = City::factory()->create();
        $house = House::factory()->create([
            'city_id' => $city->id,
            'name' => 'Test House',
        ]);
        $bank = Bank::factory()->create();
        $interest = Interest::factory()->create([
            'house_id' => $house->id,
            'bank_id' => $bank->id,
        ]);
        $mortgage = MortgageRequest::factory()->create([
            'user_id' => $user->id,
            'house_id' => $house->id,
            'interest_id' => $interest->id,
            'monthly_amount' => 10000000,
        ]);

        $this->mockMidtrans->shouldReceive('createSnapToken')
            ->once()
            ->with(Mockery::on(function ($params) use ($mortgage) {
                $expectedInsurance = 900000;
                $expectedTax = 1100000;
                $expectedTotal = 10000000 + $expectedInsurance + $expectedTax;

                return $params['transaction_details']['gross_amount'] == $expectedTotal
                    && $params['custom_field2'] == $mortgage->id;
            }))
            ->andReturn('test-snap-token');

        $result = $this->service->createPayment($mortgage);

        $this->assertEquals('test-snap-token', $result);
    }

    /**
     * Test processNotification creates installment on settlement.
     */
    public function test_process_notification_creates_installment_on_settlement(): void
    {
        $user = User::factory()->create();
        $city = City::factory()->create();
        $house = House::factory()->create(['city_id' => $city->id]);
        $bank = Bank::factory()->create();
        $interest = Interest::factory()->create([
            'house_id' => $house->id,
            'bank_id' => $bank->id,
        ]);
        $mortgage = MortgageRequest::factory()->create([
            'user_id' => $user->id,
            'house_id' => $house->id,
            'interest_id' => $interest->id,
            'monthly_amount' => 10000000,
            'loan_interest_total_amount' => 800000000,
        ]);

        $this->mockMidtrans->shouldReceive('handleNotification')
            ->once()
            ->andReturn([
                'transaction_status' => 'settlement',
                'gross_amount' => 12000000,
                'custom_field2' => $mortgage->id,
            ]);

        $this->service->processNotification();

        $this->assertDatabaseHas('installments', [
            'mortgage_request_id' => $mortgage->id,
            'is_paid' => true,
            'payment_type' => 'Midtrans',
        ]);
    }

    /**
     * Test processNotification creates installment on capture.
     */
    public function test_process_notification_creates_installment_on_capture(): void
    {
        $user = User::factory()->create();
        $city = City::factory()->create();
        $house = House::factory()->create(['city_id' => $city->id]);
        $bank = Bank::factory()->create();
        $interest = Interest::factory()->create([
            'house_id' => $house->id,
            'bank_id' => $bank->id,
        ]);
        $mortgage = MortgageRequest::factory()->create([
            'user_id' => $user->id,
            'house_id' => $house->id,
            'interest_id' => $interest->id,
            'monthly_amount' => 5000000,
            'loan_interest_total_amount' => 400000000,
        ]);

        $this->mockMidtrans->shouldReceive('handleNotification')
            ->once()
            ->andReturn([
                'transaction_status' => 'capture',
                'gross_amount' => 6550000,
                'custom_field2' => $mortgage->id,
            ]);

        $this->service->processNotification();

        $this->assertDatabaseHas('installments', [
            'mortgage_request_id' => $mortgage->id,
            'is_paid' => true,
            'payment_type' => 'Midtrans',
        ]);
    }

    /**
     * Test processNotification ignores pending status.
     */
    public function test_process_notification_ignores_pending_status(): void
    {
        $this->mockMidtrans->shouldReceive('handleNotification')
            ->once()
            ->andReturn([
                'transaction_status' => 'pending',
                'gross_amount' => 12000000,
                'custom_field2' => 1,
            ]);

        $this->service->processNotification();

        $this->assertDatabaseCount('installments', 0);
    }

    /**
     * Test processNotification ignores deny status.
     */
    public function test_process_notification_ignores_deny_status(): void
    {
        $this->mockMidtrans->shouldReceive('handleNotification')
            ->once()
            ->andReturn([
                'transaction_status' => 'deny',
                'gross_amount' => 12000000,
                'custom_field2' => 1,
            ]);

        $this->service->processNotification();

        $this->assertDatabaseCount('installments', 0);
    }

    protected function tearDown(): void
    {
        Mockery::close();
        parent::tearDown();
    }
}
