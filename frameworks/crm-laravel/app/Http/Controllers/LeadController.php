<?php

namespace App\Http\Controllers;

use Illuminate\Http\Request;
use Illuminate\Http\JsonResponse;
use App\Models\Lead;
use App\Models\ActivityLog;

class LeadController
{
    public function index(Request $request): JsonResponse
    {
        $status = $request->query('status');
        $query = Lead::with('deals')->orderBy('id', 'desc');
        if ($status) {
            $query->where('status', $status);
        }
        return response()->json(['data' => $query->get()]);
    }

    public function show(int $id): JsonResponse
    {
        $lead = Lead::with('deals')->find($id);
        if (!$lead) {
            return response()->json(['error' => 'Lead not found'], 404);
        }
        return response()->json(['data' => $lead]);
    }

    public function store(Request $request): JsonResponse
    {
        $data = $request->validate([
            'first_name' => 'required|string',
            'last_name'  => 'required|string',
            'company'    => 'required|string',
            'email'      => 'required|email',
            'phone'      => 'nullable|string',
            'status'     => 'nullable|string',
            'score'      => 'nullable|integer',
        ]);

        $lead = Lead::create($data);

        ActivityLog::create([
            'action'      => 'lead_created',
            'description' => "Created lead #{$lead->id} {$lead->first_name} {$lead->last_name}",
            'user_id'     => 1,
            'ip_address'  => $request->ip() ?? '127.0.0.1'
        ]);

        return response()->json(['success' => true, 'data' => $lead], 201);
    }
}
