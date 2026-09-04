<?php

namespace Tests\Unit\Models;

use App\Models\Bank;
use App\Models\City;
use App\Models\House;
use App\Models\Installment;
use App\Models\Interest;
use App\Models\MortgageRequest;
use App\Models\User;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Tests\TestCase;

class InstallmentTest extends TestCase
{
    use RefreshDatabase;

    /**
     * Test installment belongs to mortgage request.
     */
    public function test_installment_belongs_to_mortgage_request(): void
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
        $installment = Installment::factory()->create([
            'mortgage_request_id' => $mortgage->id,
        ]);

        $this->assertInstanceOf(MortgageRequest::class, $installment->mortgageRequest);
        $this->assertEquals($mortgage->id, $installment->mortgageRequest->id);
    }

    /**
     * Test installment can be marked as paid.
     */
    public function test_installment_can_be_marked_as_paid(): void
    {
        $installment = Installment::factory()->paid()->create();

        $this->assertTrue($installment->is_paid);
    }

    /**
     * Test installment can be marked as unpaid.
     */
    public function test_installment_can_be_marked_as_unpaid(): void
    {
        $installment = Installment::factory()->unpaid()->create();

        $this->assertFalse($installment->is_paid);
    }

    /**
     * Test installment uses soft deletes.
     */
    public function test_installment_uses_soft_deletes(): void
    {
        $installment = Installment::factory()->create();
        $installment->delete();

        $this->assertSoftDeleted('installments', [
            'id' => $installment->id,
        ]);
    }

    /**
     * Test installment factory creates valid data.
     */
    public function test_installment_factory_creates_valid_data(): void
    {
        $installment = Installment::factory()->create();

        $this->assertDatabaseHas('installments', [
            'id' => $installment->id,
        ]);
        $this->assertGreaterThan(0, $installment->sub_total_amount);
        $this->assertGreaterThan(0, $installment->grand_total_amount);
    }
}
