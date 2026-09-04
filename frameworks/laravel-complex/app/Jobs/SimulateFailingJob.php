<?php

namespace App\Jobs;

use App\Models\ActivityLog;
use Illuminate\Bus\Queueable;
use Illuminate\Contracts\Queue\ShouldQueue;
use Illuminate\Foundation\Bus\Dispatchable;
use Illuminate\Queue\InteractsWithQueue;
use Illuminate\Queue\SerializesModels;

class SimulateFailingJob implements ShouldQueue
{
    use Dispatchable, InteractsWithQueue, Queueable, SerializesModels;

    public int $tries = 1;

    public function __construct(
        public string $jobToken,
        public bool $shouldFail = true
    ) {}

    public function handle(): void
    {
        if ($this->shouldFail) {
            throw new \RuntimeException("Deliberate queue job failure for testing fault tolerance [Token: {$this->jobToken}]");
        }

        ActivityLog::create([
            'subject_type' => 'App\\Models\\Order',
            'subject_id' => 0,
            'action' => 'job_recovered',
            'description' => "SimulateFailingJob recovered successfully [Token: {$this->jobToken}]",
            'properties' => ['token' => $this->jobToken],
        ]);
    }

    public function failed(\Throwable $exception): void
    {
        ActivityLog::create([
            'subject_type' => 'App\\Models\\Order',
            'subject_id' => 0,
            'action' => 'job_failed_hook_executed',
            'description' => "Job failed hook triggered: {$exception->getMessage()}",
            'properties' => [
                'token' => $this->jobToken,
                'exception_class' => get_class($exception),
                'failed_at' => now()->toIso8601String(),
            ],
        ]);
    }
}
