<?php

namespace App\Console\Commands;

use App\Jobs\ProcessOrderInvoiceJob;
use App\Models\Order;
use Illuminate\Console\Command;

class ProcessOrdersQueueCommand extends Command
{
    /**
     * The name and signature of the console command.
     *
     * @var string
     */
    protected $signature = 'store:process-orders
                            {--limit=5 : Maximum number of orders to dispatch}
                            {--dry-run : Simulate processing without dispatching jobs}';

    /**
     * The console command description.
     *
     * @var string
     */
    protected $description = 'Dispatch asynchronous invoice processing jobs for recent orders with progress bar and statistics';

    /**
     * Execute the console command.
     */
    public function handle(): int
    {
        $startTime = microtime(true);
        $limit = (int) $this->option('limit');
        $isDryRun = (bool) $this->option('dry-run');

        $this->info("⚡ Hyperion Orders Processing Pipeline");
        $this->line("Mode: " . ($isDryRun ? "<fg=yellow>DRY RUN</>" : "<fg=green>DISPATCH LIVE JOBS</>"));
        $this->line("Target limit: {$limit}");

        $orders = Order::with('customer')
            ->latest()
            ->take($limit)
            ->get();

        if ($orders->isEmpty()) {
            $this->warn("No orders available to process.");
            return self::SUCCESS;
        }

        $this->output->newLine();
        $progressBar = $this->output->createProgressBar($orders->count());
        $progressBar->setFormat(' %current%/%max% [%bar%] %percent:3s%% -- %message%');
        $progressBar->setMessage('Initializing queue dispatch...');
        $progressBar->start();

        $rows = [];
        foreach ($orders as $order) {
            $progressBar->setMessage("Order {$order->order_number}");

            if (!$isDryRun) {
                ProcessOrderInvoiceJob::dispatch($order->id);
            }

            $rows[] = [
                $order->order_number,
                $order->customer?->name ?? 'Guest',
                '$' . number_format($order->total_amount, 2),
                $order->status->label(),
                $isDryRun ? 'Simulated' : 'Dispatched',
            ];

            usleep(20000); // 20ms visual step
            $progressBar->advance();
        }

        $progressBar->setMessage('Completed');
        $progressBar->finish();
        $this->output->newLine(2);

        $this->table(
            ['Order #', 'Customer', 'Amount', 'Status', 'Queue Action'],
            $rows
        );

        $duration = round((microtime(true) - $startTime) * 1000, 2);
        $this->info("✅ Successfully handled {$orders->count()} orders in {$duration}ms.");

        return self::SUCCESS;
    }
}
