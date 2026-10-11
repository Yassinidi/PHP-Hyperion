<?php

namespace App\Http\Controllers;

use Illuminate\Http\Request;
use App\Models\Lead;
use App\Models\Deal;
use App\Models\ActivityLog;

class CrmDashboardController
{
    public function index()
    {
        $totalLeads = Lead::count();
        $totalPipelineValue = Deal::sum('value');
        $dealsWon = Deal::where('stage', 'won')->count();
        $recentLeads = Lead::orderBy('id', 'desc')->take(5)->get();

        return view('dashboard', compact('totalLeads', 'totalPipelineValue', 'dealsWon', 'recentLeads'));
    }

    public function stats()
    {
        return response()->json([
            'total_leads' => Lead::count(),
            'pipeline_value' => (float)Deal::sum('value'),
            'won_deals' => Deal::where('stage', 'won')->count(),
            'recent_deals' => Deal::with('lead')->orderBy('id', 'desc')->take(10)->get()
        ]);
    }
}
