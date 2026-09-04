<?php

namespace Tests\Unit\Services;

use App\Services\MidtransService;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Tests\TestCase;

class MidtransServiceTest extends TestCase
{
    use RefreshDatabase;

    private MidtransService $service;

    protected function setUp(): void
    {
        parent::setUp();
        $this->service = new MidtransService;
    }

    /**
     * Test createSnapToken returns token from Midtrans API.
     */
    public function test_create_snap_token_returns_token(): void
    {
        $this->markTestSkipped('Requires Midtrans API mocking or sandbox environment');
    }

    /**
     * Test handleNotification parses webhook correctly.
     */
    public function test_handle_notification_parses_webhook(): void
    {
        $this->markTestSkipped('Requires Midtrans notification object setup');
    }

    /**
     * Test handleNotification handles invalid input.
     */
    public function test_handle_notification_handles_invalid_input(): void
    {
        $this->expectException(\Exception::class);

        $invalidNotification = null;
        $this->service->handleNotification($invalidNotification);
    }
}
