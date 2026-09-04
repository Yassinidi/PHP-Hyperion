<?php

namespace App\Models;

use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\SoftDeletes;

class HousePhoto extends Model
{
    use SoftDeletes;

    protected $fillable = ['house_id', 'photo'];
}
