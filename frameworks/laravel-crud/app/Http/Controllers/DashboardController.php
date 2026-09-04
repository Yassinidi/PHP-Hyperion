<?php

namespace App\Http\Controllers;

use Illuminate\Http\Request;
use Illuminate\Http\JsonResponse;
use App\Models\Post;
use App\Models\Comment;

class DashboardController extends Controller
{
    private static $cachedStats = [];
    private static $lastCacheTimes = [];

    /**
     * Display studio analytics and manage user's articles.
     */
    public function stats(Request $request): JsonResponse
    {
        $userId = auth()->id() ?: (int)($request->cookie('blog_user') ?: 1);

        if (isset(self::$cachedStats[$userId]) && (microtime(true) - (self::$lastCacheTimes[$userId] ?? 0.0)) < 60.0) {
            return response()->json(self::$cachedStats[$userId]);
        }

        $posts = Post::with('comments')
            ->where('user_id', $userId)
            ->orderBy('id', 'desc')
            ->get();

        $totalViews = $posts->sum('views_count');
        $totalLikes = $posts->sum('likes_count');
        $totalComments = $posts->reduce(function ($carry, Post $p) {
            return $carry + $p->comments->count();
        }, 0);

        $articles = $posts->map(function (Post $p) {
            return [
                'id' => $p->id,
                'title' => $p->title,
                'category' => $p->category,
                'views_count' => (int)$p->views_count,
                'likes_count' => (int)$p->likes_count,
                'comments_count' => $p->comments->count(),
                'created_at' => $p->created_at ? $p->created_at->format('M j, Y') : date('M j, Y'),
            ];
        });

        $responseData = [
            'status' => 'success',
            'metrics' => [
                'total_articles' => $articles->count(),
                'total_views' => (int)$totalViews,
                'total_likes' => (int)$totalLikes,
                'total_comments' => (int)$totalComments,
            ],
            'articles' => $articles,
        ];

        self::$cachedStats[$userId] = $responseData;
        self::$lastCacheTimes[$userId] = microtime(true);

        return response()->json($responseData);
    }
}
