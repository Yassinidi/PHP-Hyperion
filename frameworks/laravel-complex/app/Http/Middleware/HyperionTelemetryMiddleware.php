<?php

namespace App\Http\Middleware;

use Closure;
use Illuminate\Http\Request;
use Illuminate\Support\Str;
use Symfony\Component\HttpFoundation\Response;

class HyperionTelemetryMiddleware
{
    public function handle(Request $request, Closure $next): Response
    {
        $startTime = hrtime(true);

        $response = $next($request);

        $durationMs = round((hrtime(true) - $startTime) / 1e6, 2);
        $peakMemory = memory_get_peak_usage(true);

        $response->headers->set('X-Hyperion-Engine', 'v1.0-Release');
        $response->headers->set('X-Execution-Time-Ms', (string) $durationMs);
        $response->headers->set('X-Memory-Peak', (string) $peakMemory);
        $response->headers->set('X-Request-Trace-Id', (string) Str::uuid());

        return $response;
    }
}
