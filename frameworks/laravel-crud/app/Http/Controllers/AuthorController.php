<?php

namespace App\Http\Controllers;

use Illuminate\Http\Request;
use Illuminate\Http\JsonResponse;
use App\Models\User;
use App\Models\Post;

class AuthorController extends Controller
{
    private static $cachedAuthors = [];
    private static $lastCacheTimes = [];

    /**
     * Display author profile with publication metrics and their articles.
     */
    public function show(int|string $id): JsonResponse
    {
        $numId = (int)$id;
        if (isset(self::$cachedAuthors[$numId]) && (microtime(true) - (self::$lastCacheTimes[$numId] ?? 0.0)) < 60.0) {
            return response()->json([
                'status' => 'success',
                'author' => self::$cachedAuthors[$numId],
            ]);
        }

        $author = User::with(['posts' => function ($q) {
            $q->where('is_published', true)->orderBy('id', 'desc');
        }])->find($id);

        if (!$author) {
            return response()->json(['error' => 'Author not found.'], 404);
        }

        $totalViews = $author->posts->sum('views_count');
        $totalLikes = $author->posts->sum('likes_count');

        $authorPosts = $author->posts->map(function (Post $p) {
            return [
                'id' => $p->id,
                'title' => $p->title,
                'slug' => $p->slug,
                'category' => $p->category,
                'image_url' => $p->getImageUrl(),
                'views_count' => (int)$p->views_count,
                'likes_count' => (int)$p->likes_count,
                'created_at' => $p->created_at ? $p->created_at->format('M j, Y') : date('M j, Y'),
            ];
        });

        $authorData = [
            'id' => $author->id,
            'name' => $author->name,
            'bio' => $author->bio,
            'avatar_url' => $author->getAvatarUrl(),
            'joined' => $author->created_at ? $author->created_at->format('M Y') : date('M Y'),
            'stats' => [
                'articles_count' => $authorPosts->count(),
                'total_views' => (int)$totalViews,
                'total_likes' => (int)$totalLikes,
            ],
            'posts' => $authorPosts,
        ];

        self::$cachedAuthors[$numId] = $authorData;
        self::$lastCacheTimes[$numId] = microtime(true);

        return response()->json([
            'status' => 'success',
            'author' => $authorData,
        ]);
    }
}
