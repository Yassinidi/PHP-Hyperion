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

        // Share current active theme & store settings globally with all views
        \Illuminate\Support\Facades\View::composer('*', function ($view) {
            try {
                $themeService = app(\App\Services\ThemeService::class);
                $cartService = app(\App\Services\CartService::class);
                $currentTheme = $themeService->getActiveTheme();
                $siteSettings = \App\Models\StoreSetting::getAllGrouped();
                $cartCount = $cartService->getItemCount();
                $view->with([
                    'currentTheme' => $currentTheme,
                    'siteSettings' => $siteSettings,
                    'cartCount' => $cartCount,
                ]);
            } catch (\Throwable $e) {
                // Fallback gracefully during early migrations/CLI boot
            }
        });
    }
}