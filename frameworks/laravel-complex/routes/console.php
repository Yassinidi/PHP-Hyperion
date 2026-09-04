<?php

use Illuminate\Foundation\Console\ClosureCommand;
use Illuminate\Foundation\Inspiring;
use Illuminate\Support\Facades\Artisan;

Artisan::command('inspire', function () {
    /** @var ClosureCommand $this */
    $this->comment(Inspiring::quote());
})->purpose('Display an inspiring quote');

// Task Scheduling
\Illuminate\Support\Facades\Schedule::command('store:audit-inventory')
    ->hourly()
    ->name('inventory:audit')
    ->description('Perform hourly warehouse inventory audit');

\Illuminate\Support\Facades\Schedule::job(new \App\Jobs\GenerateInventoryReportJob('scheduled_auto'))
    ->everyTenMinutes()
    ->name('inventory:generate_report')
    ->description('Compile inventory valuation and category aggregates into cache');

\Illuminate\Support\Facades\Schedule::command('store:process-orders --limit=10')
    ->everyThirtyMinutes()
    ->name('orders:process_queue')
    ->description('Dispatch async invoices for recently confirmed orders');

