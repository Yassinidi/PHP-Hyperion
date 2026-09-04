<?php

namespace App\Models;

use Illuminate\Database\Eloquent\Factories\HasFactory;
use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\SoftDeletes;
use Illuminate\Support\Str;

class House extends Model
{
    use HasFactory, SoftDeletes;

    protected $fillable = [
        'name',
        'slug',
        'thumbnail',
        'certificate',
        'about',
        'price',
        'bedroom',
        'bathroom',
        'electric',
        'land_area',
        'building_area',
        'category_id',
        'city_id',
    ];

    public function setNameAttribute($value)
    {
        $this->attributes['name'] = $value;
        $this->attributes['slug'] = Str::slug($value);
    }

    public function category()
    {
        return $this->belongsTo(Category::class);
    }

    public function city()
    {
        return $this->belongsTo(City::class);
    }

    public function photos()
    {
        return $this->hasMany(HousePhoto::class);
    }

    public function facilities()
    {
        return $this->hasMany(HouseFacility::class);
    }

    public function interests()
    {
        return $this->hasMany(Interest::class);
    }
}
