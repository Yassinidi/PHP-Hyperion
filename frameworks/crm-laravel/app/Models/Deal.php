<?php

namespace App\Models;

use Illuminate\Database\Eloquent\Model;

class Deal extends Model
{
    protected $table = 'deals';
    protected $fillable = ['lead_id', 'title', 'value', 'stage', 'probability'];
    public $timestamps = true;

    public function lead()
    {
        return $this->belongsTo(Lead::class, 'lead_id');
    }
}
