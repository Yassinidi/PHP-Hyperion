<?php

namespace App\Models;

use Illuminate\Database\Eloquent\Builder;
use Illuminate\Database\Eloquent\Factories\HasFactory;
use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\MorphTo;
use Illuminate\Database\Eloquent\SoftDeletes;
use Illuminate\Support\Facades\Storage;

class Attachment extends Model
{
    use HasFactory, SoftDeletes;

    protected $fillable = [
        'uuid',
        'filename',
        'original_filename',
        'disk',
        'path',
        'mime_type',
        'size_bytes',
        'sha256_hash',
        'attachable_id',
        'attachable_type',
        'metadata',
    ];

    protected $casts = [
        'size_bytes' => 'integer',
        'metadata' => 'array',
    ];

    protected static function booted(): void
    {
        // When permanently deleted, remove the underlying file from disk
        static::forceDeleted(function (Attachment $attachment) {
            if (Storage::disk($attachment->disk)->exists($attachment->path)) {
                Storage::disk($attachment->disk)->delete($attachment->path);
            }
        });
    }

    public function attachable(): MorphTo
    {
        return $this->morphTo();
    }

    public function scopeByDisk(Builder $query, string $disk): Builder
    {
        return $query->where('disk', $disk);
    }

    public function scopeLargeFiles(Builder $query, int $thresholdBytes = 1048576): Builder
    {
        return $query->where('size_bytes', '>=', $thresholdBytes);
    }

    public function scopeActive(Builder $query): Builder
    {
        return $query->whereNull('deleted_at');
    }
}
