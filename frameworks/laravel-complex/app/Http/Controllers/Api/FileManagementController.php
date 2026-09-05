<?php

namespace App\Http\Controllers\Api;

use App\Http\Controllers\Controller;
use App\Models\Attachment;
use App\Services\FileManagerService;
use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;

class FileManagementController extends Controller
{
    public function __construct(
        protected FileManagerService $fileManager
    ) {}

    public function index(Request $request): JsonResponse
    {
        $query = Attachment::query();

        if ($request->boolean('with_trashed')) {
            $query->withTrashed();
        } elseif ($request->boolean('only_trashed')) {
            $query->onlyTrashed();
        }

        if ($request->filled('disk')) {
            $query->byDisk($request->query('disk'));
        }

        $attachments = $query->latest()->paginate(15);

        return response()->json($attachments);
    }

    public function store(Request $request): JsonResponse
    {
        $request->validate([
            'filename' => 'required|string|max:255',
            'content' => 'required|string',
            'disk' => 'nullable|string|in:local,public',
            'folder' => 'nullable|string',
        ]);

        $disk = $request->input('disk', 'local');
        $folder = $request->input('folder', 'uploads');
        $filename = $request->input('filename');
        $content = $request->input('content');

        $attachment = $this->fileManager->storeFile(
            disk: $disk,
            folder: $folder,
            filename: $filename,
            content: $content,
            metadata: ['client_ip' => $request->ip(), 'user_agent' => $request->userAgent()]
        );

        return response()->json([
            'message' => 'File uploaded and recorded successfully',
            'attachment' => $attachment,
        ], 201);
    }

    public function show(string $uuid): JsonResponse
    {
        $attachment = Attachment::withTrashed()->where('uuid', $uuid)->firstOrFail();
        $content = $this->fileManager->readFile($attachment->uuid);

        return response()->json([
            'attachment' => $attachment,
            'content' => $content,
        ]);
    }

    public function destroy(Request $request, string $uuid): JsonResponse
    {
        $force = $request->boolean('force', false);
        $result = $this->fileManager->deleteFile($uuid, $force);

        return response()->json([
            'message' => $force ? 'File permanently deleted from disk and DB' : 'File soft-deleted',
            'result' => $result,
        ]);
    }

    public function restore(string $uuid): JsonResponse
    {
        $attachment = $this->fileManager->restoreFile($uuid);
        return response()->json([
            'message' => 'File restored successfully',
            'attachment' => $attachment,
        ]);
    }


    public function batchDelete(Request $request): JsonResponse
    {
        $request->validate([
            'uuids' => 'required|array|min:1',
            'uuids.*' => 'string',
            'force' => 'nullable|boolean',
        ]);

        $uuids = $request->input('uuids');
        $force = $request->boolean('force', false);

        $result = $this->fileManager->batchDeleteFiles($uuids, $force);

        return response()->json([
            'message' => "Batch delete processed for {$result['deleted_count']} files",
            'result' => $result,
        ]);
    }

    public function deleteDirectory(Request $request): JsonResponse
    {
        $request->validate([
            'disk' => 'required|string|in:local,public',
            'directory' => 'required|string',
        ]);

        $disk = $request->input('disk');
        $directory = $request->input('directory');

        $deleted = $this->fileManager->deleteDirectory($disk, $directory);

        return response()->json([
            'directory' => $directory,
            'disk' => $disk,
            'deleted' => $deleted,
        ]);
    }

    public function testNativeOperations(Request $request): JsonResponse
    {
        $testPayload = "⚡ PHP-Hyperion Native Copy & Unlink Engine Verification Payload - " . now()->toIso8601String();
        $results = $this->fileManager->testTempAndNativeCopy($testPayload);

        return response()->json([
            'message' => 'Native copy and unlink operations executed',
            'results' => $results,
        ]);
    }
}
