<?php

namespace App\Models;

use Illuminate\Database\Eloquent\Model;

class ActivityLog extends Model
{
    protected $table = 'activity_logs';
    protected $fillable = ['action', 'description', 'user_id', 'ip_address'];
    public $timestamps = true;
}
