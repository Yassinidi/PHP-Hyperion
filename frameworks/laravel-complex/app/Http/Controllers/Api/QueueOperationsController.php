<?php

namespace App\Http\Controllers\Api;

use App\Enums\OrderStatus;
use App\Events\OrderShipped;
use App\Http\Controllers\Controller;
use App\Jobs\GenerateInventoryReportJob;
use App\Jobs\ProcessOrderInvoiceJob;
use App\Jobs\SimulateFailingJob;
use App\Models\Order;
use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;
use Illuminate\Support\Facades\Artisan;
use Illuminate\Support\Facades\Bus;
use Illuminate\Support\Facades\Cache;
use Illuminate\Support\Facades\DB;

class QueueOperationsController extends Controller
{
    public function stats(): JsonResponse
    {
        $pendingJobs = DB::table('jobs')->count();
        $failedJobs = DB::table('failed_jobs')->count();
        $recentFailed = DB::table('failed_jobs')->latest()->take(5)->get();
        $batches = DB::table('job_batches')->latest()->take(5)->get();
        $notificationsCount = DB::table('notifications')->count();
        $cachedReport = Cache::get('inventory_report_latest');

        return response()->json([
            'status' => 'online',
            'queue_driver' => config('queue.default'),
            'metrics' => [
                'pending_jobs' => $pendingJobs,
                'failed_jobs' => $failedJobs,
                'total_notifications' => $notificationsCount,
                'cache_cached' => $cachedReport !== null,
            ],
            'cached_inventory_valuation' => $cachedReport['total_valuation'] ?? null,
            'batches' => $batches,
            'recent_failures' => $recentFailed,
            'server_timestamp' => now()->toIso8601String(),
        ]);
    }

    public function dispatchJob(Request $request): JsonResponse
    {
        $type = $request->input('type', 'report');

        if ($type === 'report') {
            GenerateInventoryReportJob::dispatch('manual_api');
            return response()->json([
                'message' => 'GenerateInventoryReportJob dispatched to queue.',
                'job' => GenerateInventoryReportJob::class,
                'queue' => 'default',
            ]);
        }

        if ($type === 'fail') {
            $token = 'ERR-' . strtoupper(substr(md5((string) microtime()), 0, 8));
            SimulateFailingJob::dispatch($token, true);
            return response()->json([
                'message' => "SimulateFailingJob dispatched to queue (will fail on execution).",
                'token' => $token,
            ]);
        }

        if ($type === 'batch') {
            $batch = Bus::batch([
                new GenerateInventoryReportJob('batch_sub_1'),
                new GenerateInventoryReportJob('batch_sub_2'),
                new GenerateInventoryReportJob('batch_sub_3'),
            ])->name('Warehouse Aggregate Batch Pipeline')->dispatch();

            return response()->json([
                'message' => 'Batch dispatched successfully via Bus::batch().',
                'batch_id' => $batch->id,
                'total_jobs' => $batch->totalJobs,
            ]);
        }

        if ($type === 'invoice') {
            $order = Order::latest()->first();
            if (!$order) {
                return response()->json(['error' => 'No orders found'], 404);
            }
            ProcessOrderInvoiceJob::dispatch($order->id);
            return response()->json([
                'message' => "ProcessOrderInvoiceJob dispatched for order {$order->order_number}.",
                'order_id' => $order->id,
            ]);
        }

        return response()->json(['error' => "Unknown job type [{$type}]"], 400);
    }

    public function shipOrder(int $id): JsonResponse
    {
        $order = Order::with('customer')->findOrFail($id);
        $order->status = OrderStatus::Completed;
        $order->save();

        $tracking = 'TRK-' . strtoupper(substr(md5((string) $order->id . microtime()), 0, 8));
        event(new OrderShipped($order, $tracking));

        return response()->json([
            'message' => "Order {$order->order_number} marked as shipped.",
            'order_id' => $order->id,
            'tracking_number' => $tracking,
            'queued_listener' => 'SendOrderShippedNotification pushed to database queue',
        ]);
    }

    public function workOnce(): JsonResponse
    {
        $exitCode = Artisan::call('queue:work', ['--once' => true]);
        $output = Artisan::output();

        return response()->json([
            'message' => 'Single queue worker cycle executed.',
            'exit_code' => $exitCode,
            'output' => trim($output),
            'remaining_jobs' => DB::table('jobs')->count(),
        ]);
    }

    public function retryFailed(): JsonResponse
    {
        $exitCode = Artisan::call('queue:retry', ['id' => ['all']]);
        $output = Artisan::output();

        return response()->json([
            'message' => 'queue:retry all triggered.',
            'exit_code' => $exitCode,
            'output' => trim($output),
            'remaining_failed' => DB::table('failed_jobs')->count(),
        ]);
    }
}
