<?php

namespace App\Events;

use Illuminate\Broadcasting\Channel;
use Illuminate\Broadcasting\InteractsWithSockets;
use Illuminate\Contracts\Broadcasting\ShouldBroadcastNow;
use Illuminate\Foundation\Events\Dispatchable;
use Illuminate\Queue\SerializesModels;

class FileDeletedBroadcast implements ShouldBroadcastNow
{
    use Dispatchable, InteractsWithSockets, SerializesModels;

    public function __construct(
        public string $uuid,
        public string $filename,
        public bool $forceDeleted
    ) {}

    public function broadcastOn(): array
    {
        return [
            new Channel('files'),
        ];
    }

    public function broadcastAs(): string
    {
        return 'file.deleted';
    }

    public function broadcastWith(): array
    {
        return [
            'uuid' => $this->uuid,
            'filename' => $this->filename,
            'force_deleted' => $this->forceDeleted,
            'deleted_at' => now()->toIso8601String(),
            'engine' => 'hyperion',
        ];
    }
}
