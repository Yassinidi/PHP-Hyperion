<?php

namespace Tests\Unit\Models;

use App\Models\Category;
use App\Models\City;
use App\Models\House;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Tests\TestCase;

class HouseTest extends TestCase
{
    use RefreshDatabase;

    /**
     * Test house belongs to category.
     */
    public function test_house_belongs_to_category(): void
    {
        $category = Category::factory()->create();
        $house = House::factory()->create(['category_id' => $category->id]);

        $this->assertInstanceOf(Category::class, $house->category);
        $this->assertEquals($category->id, $house->category->id);
    }

    /**
     * Test house belongs to city.
     */
    public function test_house_belongs_to_city(): void
    {
        $city = City::factory()->create();
        $house = House::factory()->create(['city_id' => $city->id]);

        $this->assertInstanceOf(City::class, $house->city);
        $this->assertEquals($city->id, $house->city->id);
    }

    /**
     * Test house has many photos.
     */
    public function test_house_has_many_photos(): void
    {
        $house = House::factory()->create();

        $this->assertCount(0, $house->photos);
    }

    /**
     * Test house has many facilities.
     */
    public function test_house_has_many_facilities(): void
    {
        $house = House::factory()->create();

        $this->assertCount(0, $house->facilities);
    }

    /**
     * Test house has many interests.
     */
    public function test_house_has_many_interests(): void
    {
        $house = House::factory()->create();

        $this->assertCount(0, $house->interests);
    }

    /**
     * Test house uses soft deletes.
     */
    public function test_house_uses_soft_deletes(): void
    {
        $house = House::factory()->create();
        $house->delete();

        $this->assertSoftDeleted('houses', [
            'id' => $house->id,
        ]);
    }

    /**
     * Test house auto-generates slug from name.
     */
    public function test_house_auto_generates_slug(): void
    {
        $house = House::factory()->create(['name' => 'Test House Residence']);

        $this->assertNotEmpty($house->slug);
        $this->assertStringContainsString('-', $house->slug);
    }
}
