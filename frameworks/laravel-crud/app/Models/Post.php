<?php

namespace App\Models;

use Illuminate\Database\Eloquent\Factories\HasFactory;
use Illuminate\Database\Eloquent\Model;

class Post extends Model
{
    use HasFactory;

    protected $fillable = [
        'user_id',
        'title',
        'slug',
        'content',
        'category',
        'image_path',
        'views_count',
        'likes_count',
        'is_published',
    ];

    protected $casts = [
        'is_published' => 'boolean',
        'views_count' => 'integer',
        'likes_count' => 'integer',
    ];

    public function user()
    {
        return $this->belongsTo(User::class);
    }

    public function author()
    {
        return $this->belongsTo(User::class, 'user_id');
    }

    public function comments()
    {
        return $this->hasMany(Comment::class)->orderBy('created_at', 'desc');
    }

    public function getImageUrl()
    {
        if ($this->image_path && file_exists(storage_path('app/public/' . $this->image_path))) {
            return '/storage/' . $this->image_path;
        }
        return null;
    }

    public function getImageUrlAttribute()
    {
        return $this->getImageUrl();
    }
}
