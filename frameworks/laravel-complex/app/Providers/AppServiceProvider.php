<?php

namespace App\Providers;

use App\Contracts\PaymentGatewayInterface;
use App\Events\OrderPlaced;
use App\Events\OrderShipped;
use App\Listeners\AuditOrderPlaced;
use App\Listeners\SendOrderShippedNotification;
use App\Models\Order;
use App\Observers\OrderObserver;
use App\Services\StripePaymentGateway;
use Illuminate\Support\Facades\Event;
use Illuminate\Support\ServiceProvider;

class AppServiceProvider extends ServiceProvider
{
    /**
     * Register any application services.
     */
    public function register(): void
    {
        // Service container interface-to-implementation binding
        $this->app->bind(PaymentGatewayInterface::class, StripePaymentGateway::class);
    }

    /**
     * Bootstrap any application services.
     */
    public function boot(): void
    {
        // Model Observer registration
        Order::observe(OrderObserver::class);

        // Event Listener registration
        Event::listen(OrderPlaced::class, AuditOrderPlaced::class);
        Event::listen(OrderShipped::class, SendOrderShippedNotification::class);
    }
}