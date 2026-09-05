<?php

namespace App\Services;

use App\Events\FileDeletedBroadcast;
use App\Models\Attachment;
use Illuminate\Database\Eloquent\Model;
use Illuminate\Support\Facades\Storage;
use Illuminate\Support\Str;

class FileManagerService
{
    /**
     * Store a file to disk and record an Attachment model
     */
    public function storeFile(
        string $disk,
        string $folder,
        string $filename,
        string $content,
        ?Model $attachable = null,
        array $metadata = []
    ): Attachment {
        $uuid = (string) Str::uuid();
        $safeFilename = $uuid . '_' . preg_replace('/[^a-zA-Z0-9._-]/', '_', $filename);
        $path = trim($folder, '/') . '/' . $safeFilename;

        Storage::disk($disk)->put($path, $content);

        $size = Storage::disk($disk)->size($path);
        $hash = hash('sha256', $content);
        $mime = Storage::disk($disk)->mimeType($path) ?: 'application/octet-stream';

        $attachment = new Attachment([
            'uuid' => $uuid,
            'filename' => $safeFilename,
            'original_filename' => $filename,
            'disk' => $disk,
            'path' => $path,
            'mime_type' => $mime,
            'size_bytes' => $size,
            'sha256_hash' => $hash,
            'metadata' => $metadata,
        ]);

        if ($attachable) {
            $attachment->attachable()->associate($attachable);
        }

        $attachment->save();

        return $attachment;
    }

    /**
     * Read content of a stored file
     */
    public function readFile(int|string $idOrUuid): string
    {
        $attachment = $this->resolveAttachment($idOrUuid);

        if (!Storage::disk($attachment->disk)->exists($attachment->path)) {
            throw new \RuntimeException("Physical file not found on disk: {$attachment->path}");
        }

        return Storage::disk($attachment->disk)->get($attachment->path);
    }

    /**
     * Delete an individual file (soft or force) and dispatch broadcast
     */
    public function deleteFile(int|string $idOrUuid, bool $force = false): array
    {
        $attachment = $this->resolveAttachment($idOrUuid, true);
        $disk = $attachment->disk;
        $path = $attachment->path;
        $uuid = $attachment->uuid;

        $physicalDeleted = false;

        if ($force) {
            // Force delete model, which triggers forceDeleted event to clean storage
            $attachment->forceDelete();
            $physicalDeleted = !Storage::disk($disk)->exists($path);
        } else {
            // Soft delete the database record
            $attachment->delete();
        }

        // Fire realtime broadcast event
        event(new FileDeletedBroadcast($uuid, $attachment->original_filename, $force));

        return [
            'uuid' => $uuid,
            'soft_deleted' => !$force,
            'force_deleted' => $force,
            'physical_deleted' => $physicalDeleted,
            'disk' => $disk,
            'path' => $path,
        ];
    }

    /**
     * Batch delete multiple files
     */
    public function batchDeleteFiles(array $idsOrUuids, bool $force = false): array
    {
        $results = [];
        $pathsToDelete = [];
        $disk = 'local';

        $attachments = Attachment::withTrashed()
            ->where(function ($query) use ($idsOrUuids) {
                $query->whereIn('id', $idsOrUuids)
                    ->orWhereIn('uuid', $idsOrUuids);
            })
            ->get();

        foreach ($attachments as $att) {
            $disk = $att->disk;
            $pathsToDelete[] = $att->path;

            if ($force) {
                $att->forceDelete();
            } else {
                $att->delete();
            }

            $results[] = [
                'id' => $att->id,
                'uuid' => $att->uuid,
                'filename' => $att->original_filename,
            ];
        }

        if ($force && !empty($pathsToDelete)) {
            Storage::disk($disk)->delete($pathsToDelete);
        }

        return [
            'deleted_count' => count($results),
            'force' => $force,
            'files' => $results,
        ];
    }

    /**
     * Delete an entire directory recursively
     */
    public function deleteDirectory(string $disk, string $directory): bool
    {
        if (Storage::disk($disk)->exists($directory)) {
            return Storage::disk($disk)->deleteDirectory($directory);
        }

        return false;
    }

    /**
     * Restore a soft-deleted attachment
     */
    public function restoreFile(int|string $idOrUuid): Attachment
    {
        $attachment = Attachment::onlyTrashed()
            ->where('id', $idOrUuid)
            ->orWhere('uuid', $idOrUuid)
            ->firstOrFail();

        $attachment->restore();

        return $attachment;
    }

    /**
     * Test temporary file generation, native copy() and unlink() verification
     */
    public function testTempAndNativeCopy(string $content): array
    {
        $tmpDir = sys_get_temp_dir();
        $sourceFile = $tmpDir . '/hyp_source_' . uniqid() . '.txt';
        $destFile = $tmpDir . '/hyp_dest_' . uniqid() . '.txt';

        file_put_contents($sourceFile, $content);
        $sourceExists = file_exists($sourceFile);

        // Native copy() test
        $copyResult = copy($sourceFile, $destFile);
        $destExists = file_exists($destFile);
        $copiedContent = $destExists ? file_get_contents($destFile) : null;

        // Native unlink() test
        $unlinkedSource = unlink($sourceFile);
        $unlinkedDest = unlink($destFile);

        return [
            'source_created' => $sourceExists,
            'copy_success' => $copyResult,
            'dest_created' => $destExists,
            'content_integrity' => $copiedContent === $content,
            'unlinked_source' => $unlinkedSource && !file_exists($sourceFile),
            'unlinked_dest' => $unlinkedDest && !file_exists($destFile),
        ];
    }

    protected function resolveAttachment(int|string $idOrUuid, bool $withTrashed = false): Attachment
    {
        $query = $withTrashed ? Attachment::withTrashed() : Attachment::query();

        if (is_numeric($idOrUuid)) {
            return $query->where('id', (int) $idOrUuid)->firstOrFail();
        }

        return $query->where('uuid', $idOrUuid)->firstOrFail();
    }
}
