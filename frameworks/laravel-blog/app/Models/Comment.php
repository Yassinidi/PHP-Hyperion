<?php

namespace App\Models;

use Illuminate\Database\Eloquent\Factories\HasFactory;
use Illuminate\Database\Eloquent\Model;

class Comment extends Model
{
    use HasFactory;

    protected $fillable = [
        'post_id',
        'user_id',
        'author_name',
        'author_avatar',
        'content',
    ];

    public function post()
    {
        return $this->belongsTo(Post::class);
    }

    public function user()
    {
        return $this->belongsTo(User::class);
    }

    public function getAvatarUrl()
    {
        if ($this->author_avatar && file_exists(storage_path('app/public/' . $this->author_avatar))) {
            return '/storage/' . $this->author_avatar;
        }
        if ($this->user) {
            return $this->user->avatar_url;
        }
        return 'https://ui-avatars.com/api/?name=' . urlencode($this->author_name) . '&background=10b981&color=fff';
    }

    public function getAvatarUrlAttribute()
    {
        return $this->getAvatarUrl();
    }
}
