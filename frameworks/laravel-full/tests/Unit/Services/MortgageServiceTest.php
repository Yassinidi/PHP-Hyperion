<?php

namespace Tests\Unit\Services;

use App\Models\Bank;
use App\Models\City;
use App\Models\House;
use App\Models\Interest;
use App\Models\MortgageRequest;
use App\Models\User;
use App\Services\MortgageService;
use Database\Seeders\RoleAdminSeeder;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Http\UploadedFile;
use Illuminate\Support\Facades\Auth;
use Illuminate\Support\Facades\Storage;
use Tests\TestCase;

class MortgageServiceTest extends TestCase
{
    use RefreshDatabase;

    private MortgageService $service;

    protected function setUp(): void
    {
        parent::setUp();
        $this->service = new MortgageService;
        $this->seed(RoleAdminSeeder::class);
    }

    /**
     * Test calculateMortgageDetails with standard parameters.
     */
    public function test_calculate_mortgage_details_with_standard_parameters(): void
    {
        $city = City::factory()->create();
        $house = House::factory()->create([
            'city_id' => $city->id,
            'price' => 1000000000,
        ]);
        $bank = Bank::factory()->create();
        $interest = Interest::factory()->create([
            'house_id' => $house->id,
            'bank_id' => $bank->id,
            'interest' => 5,
            'duration' => 15,
        ]);

        $result = $this->service->calculateMortgageDetails($house, $interest, 20);

        $this->assertArrayHasKey('house', $result);
        $this->assertArrayHasKey('interest', $result);
        $this->assertArrayHasKey('housePrice', $result);
        $this->assertArrayHasKey('dpTotalAmount', $result);
        $this->assertArrayHasKey('loanTotalAmount', $result);
        $this->assertArrayHasKey('monthlyAmount', $result);
        $this->assertArrayHasKey('loanInterestTotalAmount', $result);

        $this->assertEquals(1000000000, $result['housePrice']);
        $this->assertEquals(200000000, $result['dpTotalAmount']);
        $this->assertEquals(800000000, $result['loanTotalAmount']);
        $this->assertGreaterThan(0, $result['monthlyAmount']);
    }

    /**
     * Test calculateMortgageDetails with minimum down payment (10%).
     */
    public function test_calculate_mortgage_with_minimum_down_payment(): void
    {
        $city = City::factory()->create();
        $house = House::factory()->create([
            'city_id' => $city->id,
            'price' => 500000000,
        ]);
        $bank = Bank::factory()->create();
        $interest = Interest::factory()->create([
            'house_id' => $house->id,
            'bank_id' => $bank->id,
            'interest' => 8,
            'duration' => 10,
        ]);

        $result = $this->service->calculateMortgageDetails($house, $interest, 10);

        $this->assertEquals(50000000, $result['dpTotalAmount']);
        $this->assertEquals(450000000, $result['loanTotalAmount']);
    }

    /**
     * Test calculateMortgageDetails with maximum down payment (80%).
     */
    public function test_calculate_mortgage_with_maximum_down_payment(): void
    {
        $city = City::factory()->create();
        $house = House::factory()->create([
            'city_id' => $city->id,
            'price' => 1000000000,
        ]);
        $bank = Bank::factory()->create();
        $interest = Interest::factory()->create([
            'house_id' => $house->id,
            'bank_id' => $bank->id,
            'interest' => 5,
            'duration' => 5,
        ]);

        $result = $this->service->calculateMortgageDetails($house, $interest, 80);

        $this->assertEquals(800000000, $result['dpTotalAmount']);
        $this->assertEquals(200000000, $result['loanTotalAmount']);
    }

    /**
     * Test createMortgageRequest creates record successfully.
     */
    public function test_create_mortgage_request_creates_record(): void
    {
        $user = User::factory()->create();
        $user->assignRole('customer');
        Auth::login($user);

        $city = City::factory()->create();
        $house = House::factory()->create([
            'city_id' => $city->id,
            'price' => 1000000000,
        ]);
        $bank = Bank::factory()->create();
        $interest = Interest::factory()->create([
            'house_id' => $house->id,
            'bank_id' => $bank->id,
            'interest' => 5,
            'duration' => 15,
        ]);

        $details = $this->service->calculateMortgageDetails($house, $interest, 20);
        $documentPath = 'documents/test.pdf';

        $mortgageRequest = $this->service->createMortgageRequest($details, $documentPath);

        $this->assertInstanceOf(MortgageRequest::class, $mortgageRequest);
        $this->assertDatabaseHas('mortgage_requests', [
            'id' => $mortgageRequest->id,
            'user_id' => $user->id,
            'house_id' => $house->id,
            'status' => 'Waiting for Bank',
            'dp_percentage' => 20,
        ]);

        $this->assertEquals(200000000, $mortgageRequest->dp_total_amount);
        $this->assertEquals(800000000, $mortgageRequest->loan_total_amount);
    }

    /**
     * Test uploadDocuments stores file correctly.
     */
    public function test_upload_documents_stores_file(): void
    {
        Storage::fake('public');

        $file = UploadedFile::fake()->create('document.pdf', 100);
        $request = new \Illuminate\Http\Request;
        $request->files->set('documents', $file);

        $path = $this->service->uploadDocuments($request);

        $this->assertNotNull($path);
        $this->assertStringContainsString('documents/', $path);
        Storage::disk('public')->assertExists($path);
    }

    /**
     * Test uploadDocuments returns null when no file.
     */
    public function test_upload_documents_returns_null_when_no_file(): void
    {
        $request = new \Illuminate\Http\Request;

        $path = $this->service->uploadDocuments($request);

        $this->assertNull($path);
    }

    /**
     * Test getUserMortgages returns only user's mortgages.
     */
    public function test_get_user_mortgages_returns_only_users_data(): void
    {
        $user1 = User::factory()->create();
        $user1->assignRole('customer');
        $user2 = User::factory()->create();
        $user2->assignRole('customer');

        $city = City::factory()->create();
        $house = House::factory()->create(['city_id' => $city->id]);
        $bank = Bank::factory()->create();
        $interest = Interest::factory()->create([
            'house_id' => $house->id,
            'bank_id' => $bank->id,
        ]);

        MortgageRequest::factory()->create([
            'user_id' => $user1->id,
            'house_id' => $house->id,
            'interest_id' => $interest->id,
        ]);
        MortgageRequest::factory()->create([
            'user_id' => $user2->id,
            'house_id' => $house->id,
            'interest_id' => $interest->id,
        ]);

        $result = $this->service->getUserMortgages($user1->id);

        $this->assertCount(1, $result);
        $this->assertEquals($user1->id, $result->first()->user_id);
    }

    /**
     * Test getUserMortgages with search filter.
     */
    public function test_get_user_mortgages_with_search(): void
    {
        $user = User::factory()->create();
        $user->assignRole('customer');

        $city = City::factory()->create();
        $house = House::factory()->create([
            'city_id' => $city->id,
            'name' => 'Test House',
        ]);
        $bank = Bank::factory()->create(['name' => 'BCA']);
        $interest = Interest::factory()->create([
            'house_id' => $house->id,
            'bank_id' => $bank->id,
        ]);

        MortgageRequest::factory()->create([
            'user_id' => $user->id,
            'house_id' => $house->id,
            'interest_id' => $interest->id,
            'bank_name' => 'BCA',
        ]);

        $result = $this->service->getUserMortgages($user->id, 'BCA');

        $this->assertCount(1, $result);
    }

    /**
     * Test getMortgageDetails loads relationships.
     */
    public function test_get_mortgage_details_loads_relationships(): void
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
        ]);

        $result = $this->service->getMortgageDetails($mortgage);

        $this->assertArrayHasKey('mortgageRequest', $result);
        $this->assertArrayHasKey('totalTaxAmount', $result);
        $this->assertArrayHasKey('insurance', $result);
        $this->assertTrue($result['mortgageRequest']->relationLoaded('house'));
    }
}
