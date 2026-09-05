<?php

namespace App\Http\Controllers\Api;

use App\Http\Controllers\Controller;
use App\Models\Customer;
use App\Services\AdvancedQueryService;
use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;

class AdvancedQueryController extends Controller
{
    public function __construct(
        protected AdvancedQueryService $queryService
    ) {}

    public function windowRankings(): JsonResponse
    {
        $rankings = $this->queryService->getCategoryProductRankings();

        return response()->json([
            'description' => 'Product pricing rankings partitioned by category using window functions',
            'count' => $rankings->count(),
            'data' => $rankings,
        ]);
    }

    public function joinSubPerformance(): JsonResponse
    {
        $stats = $this->queryService->getProductPerformanceViaJoinSub();

        return response()->json([
            'description' => 'Aggregated order statistics joined using joinSub',
            'count' => $stats->count(),
            'data' => $stats,
        ]);
    }

    public function correlatedCustomerMetrics(): JsonResponse
    {
        $customers = $this->queryService->getCustomerMetricsCorrelated();

        return response()->json([
            'description' => 'Customer metrics computed via correlated subqueries in addSelect',
            'count' => $customers->count(),
            'data' => $customers,
        ]);
    }

    public function cohortSpending(): JsonResponse
    {
        $cohorts = $this->queryService->getCustomerCohortSpending();

        return response()->json([
            'description' => 'Customer spending grouped with conditional aggregates and HAVING clause',
            'count' => $cohorts->count(),
            'data' => $cohorts,
        ]);
    }

    public function testSavepoints(): JsonResponse
    {
        $result = $this->queryService->testNestedTransactionWithSavepoint();

        return response()->json([
            'message' => 'Nested transaction with savepoint executed',
            'result' => $result,
        ]);
    }

    public function testSoftDeletes(): JsonResponse
    {
        $result = $this->queryService->testSoftDeleteLifecycle();

        return response()->json([
            'message' => 'Soft delete full lifecycle executed',
            'result' => $result,
        ]);
    }

    public function testModernSyntax(Request $request): JsonResponse
    {
        $amount = (float) $request->query('amount', 1250.75);
        $customer = Customer::first();

        $result = $this->queryService->testModernPhpSyntax($amount, $customer);

        return response()->json([
            'message' => 'Modern PHP 8.4 syntax executed',
            'result' => $result,
        ]);
    }
}
