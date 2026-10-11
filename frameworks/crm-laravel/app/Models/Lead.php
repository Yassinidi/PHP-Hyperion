<?php

namespace App\Models;

use Illuminate\Database\Eloquent\Model;

class Lead extends Model
{
    protected $table = 'leads';
    protected $fillable = ['first_name', 'last_name', 'company', 'email', 'phone', 'status', 'score'];
    public $timestamps = true;

    public function deals()
    {
        return $this->hasMany(Deal::class, 'lead_id');
    }
}
