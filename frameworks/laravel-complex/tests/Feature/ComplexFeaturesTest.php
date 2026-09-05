<?php

namespace Tests\Feature;

use App\Models\Attachment;
use App\Models\Category;
use App\Models\Customer;
use App\Models\Order;
use App\Models\Product;
use App\Services\AdvancedQueryService;
use App\Services\FileManagerService;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\Storage;
use Tests\TestCase;

class ComplexFeaturesTest extends TestCase
{
    protected FileManagerService $fileService;
    protected AdvancedQueryService $queryService;

    protected function setUp(): void
    {
        parent::setUp();
        $this->fileService = app(FileManagerService::class);
        $this->queryService = app(AdvancedQueryService::class);
    }

    public function test_single_file_storage_lifecycle_and_deletion(): void
    {
        $content = "Complex file storage content " . uniqid();
        $attachment = $this->fileService->storeFile('local', 'tests', 'test_doc.txt', $content);

        $this->assertTrue(Storage::disk('local')->exists($attachment->path));
        $this->assertSame($content, $this->fileService->readFile($attachment->uuid));

        // Soft delete
        $del = $this->fileService->deleteFile($attachment->uuid, false);
        $this->assertTrue($del['soft_deleted']);
        $this->assertFalse(Attachment::where('uuid', $attachment->uuid)->exists());
        $this->assertTrue(Attachment::onlyTrashed()->where('uuid', $attachment->uuid)->exists());

        // Restore
        $restored = $this->fileService->restoreFile($attachment->uuid);
        $this->assertNull($restored->deleted_at);

        // Force delete
        $force = $this->fileService->deleteFile($attachment->uuid, true);
        $this->assertTrue($force['force_deleted']);
        $this->assertFalse(Storage::disk('local')->exists($attachment->path));
    }

    public function test_native_copy_and_unlink_operations(): void
    {
        $res = $this->fileService->testTempAndNativeCopy("Testing native copy and unlink in Hyperion engine");
        $this->assertTrue($res['copy_success']);
        $this->assertTrue($res['content_integrity']);
        $this->assertTrue($res['unlinked_source']);
        $this->assertTrue($res['unlinked_dest']);
    }

    public function test_window_function_product_rankings(): void
    {
        $rankings = $this->queryService->getCategoryProductRankings();
        $this->assertNotEmpty($rankings);
        $this->assertObjectHasProperty('rank_in_category', $rankings->first());
        $this->assertObjectHasProperty('category_avg_price', $rankings->first());
    }

    public function test_subquery_joins_and_aggregations(): void
    {
        $stats = $this->queryService->getProductPerformanceViaJoinSub();
        $this->assertNotEmpty($stats);
        $this->assertObjectHasProperty('gross_revenue', $stats->first());
    }

    public function test_nested_transactions_with_savepoint_isolation(): void
    {
        $result = $this->queryService->testNestedTransactionWithSavepoint();
        $this->assertTrue($result['isolation_passed']);
    }

    public function test_soft_delete_lifecycle_pipeline(): void
    {
        $result = $this->queryService->testSoftDeleteLifecycle();
        $this->assertTrue($result['lifecycle_passed']);
    }

    public function test_modern_php_syntax_guards(): void
    {
        $customer = Customer::first();
        $result = $this->queryService->testModernPhpSyntax(4200.0, $customer);
        $this->assertSame('Business Premium', $result['tier']);
        $this->assertSame('$4,200.00', $result['formatted']);
        $this->assertSame('active', $result['destructured_status']);
    }

    public function test_file_management_http_endpoints(): void
    {
        $response = $this->postJson('/api/v1/files', [
            'filename' => 'api_test.txt',
            'content' => 'API uploaded file test content',
            'disk' => 'local',
            'folder' => 'api_tests',
        ]);

        $response->assertStatus(201);
        $uuid = $response->json('attachment.uuid');

        $showResp = $this->getJson("/api/v1/files/{$uuid}");
        $showResp->assertStatus(200);
        $showResp->assertJsonPath('content', 'API uploaded file test content');

        $delResp = $this->deleteJson("/api/v1/files/{$uuid}?force=1");
        $delResp->assertStatus(200);
        $delResp->assertJsonPath('result.force_deleted', true);
    }

    public function test_reverb_broadcasting_endpoint(): void
    {
        $response = $this->postJson('/api/v1/broadcasting/dispatch', [
            'type' => 'order_status',
        ]);

        $response->assertStatus(200);
        $response->assertJsonPath('event_name', 'order.status.updated');
        $this->assertStringStartsWith('private-orders.', $response->json('channel'));
    }

    public function test_advanced_query_http_endpoints(): void
    {
        $this->getJson('/api/v1/queries/window-rankings')->assertStatus(200);
        $this->getJson('/api/v1/queries/joinsub-performance')->assertStatus(200);
        $this->getJson('/api/v1/queries/cohort-spending')->assertStatus(200);
        $this->postJson('/api/v1/queries/test-savepoints')->assertStatus(200);
        $this->postJson('/api/v1/queries/test-soft-deletes')->assertStatus(200);
        $this->getJson('/api/v1/queries/test-modern-syntax')->assertStatus(200);
    }
}
