<?php

namespace App\Http\Controllers;

use Illuminate\Http\Request;
use Illuminate\Http\JsonResponse;
use App\Models\Lead;
use App\Models\Deal;
use App\Models\ActivityLog;
use Illuminate\Support\Facades\DB;

class DealController
{
    public function index(): JsonResponse
    {
        return response()->json(['data' => Deal::with('lead')->orderBy('id', 'desc')->get()]);
    }

    public function createDeal(Request $request): JsonResponse
    {
        $leadId = (int)$request->input('lead_id', 0);
        $title = $request->input('title', 'Enterprise License Deal');
        $value = (float)$request->input('value', 10000.0);
        $stage = $request->input('stage', 'proposal');

        return DB::transaction(function () use ($leadId, $title, $value, $stage, $request) {
            $lead = Lead::find($leadId);
            if (!$lead) {
                throw new \Exception("Lead ID {$leadId} not found");
            }

            $deal = Deal::create([
                'lead_id'     => $lead->id,
                'title'       => $title,
                'value'       => $value,
                'stage'       => $stage,
                'probability' => 75
            ]);

            $lead->increment('score', 10);

            ActivityLog::create([
                'action'      => 'deal_created',
                'description' => "Deal #{$deal->id} of \${$value} attached to lead #{$lead->id}",
                'user_id'     => 1,
                'ip_address'  => $request->ip() ?? '127.0.0.1'
            ]);

            return response()->json([
                'success' => true,
                'deal_id' => $deal->id,
                'value'   => $deal->value,
                'new_lead_score' => $lead->score
            ], 201);
        });
    }
}
