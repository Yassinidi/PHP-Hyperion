<?php

namespace App\Http\Controllers;

use Illuminate\Http\Request;
use Illuminate\Http\JsonResponse;
use App\Models\Post;
use App\Models\User;
use App\Models\Comment;

class PostController extends Controller
{
    private static $cachedPosts = null;
    private static $lastCacheTime = 0.0;
    private static $cachedPostShows = [];
    private static $lastShowCacheTimes = [];

    /**
     * Display a listing of blog posts with search and category filters.
     */
    public function index(Request $request): JsonResponse
    {
        $search = $request->input('search');
        $category = $request->input('category');
        $authorId = $request->input('author_id');
        $isDefault = empty($search) && (empty($category) || $category === 'All') && empty($authorId);

        if ($isDefault && self::$cachedPosts !== null && (microtime(true) - self::$lastCacheTime) < 60.0) {
            return response()->json([
                'status' => 'success',
                'posts' => self::$cachedPosts,
            ]);
        }

        $query = Post::with(['author', 'comments'])->where('is_published', true);

        if ($search) {
            $query->where(function ($q) use ($search) {
                $q->where('title', 'like', "%{$search}%")
                  ->orWhere('content', 'like', "%{$search}%");
            });
        }

        if ($category && $category !== 'All') {
            $query->where('category', $category);
        }

        if ($authorId) {
            $query->where('user_id', $authorId);
        }

        $posts = $query->orderBy('id', 'desc')->get()->map(function (Post $post) {
            return [
                'id' => $post->id,
                'user_id' => $post->user_id,
                'title' => $post->title,
                'slug' => $post->slug,
                'category' => $post->category,
                'excerpt' => substr(strip_tags($post->content), 0, 160) . '...',
                'image_url' => $post->getImageUrl(),
                'views_count' => (int)$post->views_count,
                'likes_count' => (int)$post->likes_count,
                'comments_count' => $post->comments->count(),
                'created_at' => $post->created_at ? $post->created_at->format('M j, Y') : date('M j, Y'),
                'author' => [
                    'id' => $post->author->id ?? $post->user_id,
                    'name' => $post->author->name ?? 'Elena Vance',
                    'bio' => $post->author->bio ?? 'Technical Writer',
                    'avatar_url' => $post->author ? $post->author->getAvatarUrl() : 'https://ui-avatars.com/api/?name=Author',
                ],
            ];
        });

        if ($isDefault) {
            self::$cachedPosts = $posts;
            self::$lastCacheTime = microtime(true);
        }

        return response()->json([
            'status' => 'success',
            'posts' => $posts,
        ]);
    }

    /**
     * Display the specified post with author and comments.
     */
    public function show(int|string $id): JsonResponse
    {
        $numId = (int)$id;
        if (isset(self::$cachedPostShows[$numId]) && (microtime(true) - (self::$lastShowCacheTimes[$numId] ?? 0.0)) < 60.0) {
            return response()->json([
                'status' => 'success',
                'post' => self::$cachedPostShows[$numId],
            ]);
        }

        $post = Post::with(['author', 'comments'])->find($id);

        if (!$post) {
            return response()->json(['error' => 'Article not found.'], 404);
        }

        $comments = $post->comments->map(function (Comment $comment) {
            return [
                'id' => $comment->id,
                'author_name' => $comment->author_name,
                'author_avatar' => $comment->getAvatarUrl(),
                'content' => $comment->content,
                'created_at' => $comment->created_at ? (method_exists($comment->created_at, 'diffForHumans') ? $comment->created_at->diffForHumans() : $comment->created_at->format('M j, Y')) : 'Just now',
            ];
        });

        $author = $post->author;

        $postData = [
            'id' => $post->id,
            'user_id' => $post->user_id,
            'title' => $post->title,
            'slug' => $post->slug,
            'category' => $post->category,
            'content' => $post->content,
            'image_url' => $post->getImageUrl(),
            'views_count' => (int)$post->views_count,
            'likes_count' => (int)$post->likes_count,
            'comments_count' => $comments->count(),
            'created_at' => $post->created_at ? $post->created_at->format('M j, Y') : date('M j, Y'),
            'author' => [
                'id' => $author->id ?? $post->user_id,
                'name' => $author->name ?? 'Elena Vance',
                'bio' => $author->bio ?? 'Technical Writer',
                'avatar_url' => $author ? $author->getAvatarUrl() : 'https://ui-avatars.com/api/?name=Author',
            ],
            'comments' => $comments,
        ];

        self::$cachedPostShows[$numId] = $postData;
        self::$lastShowCacheTimes[$numId] = microtime(true);

        return response()->json([
            'status' => 'success',
            'post' => $postData,
        ]);
    }

    /**
     * Store a newly created post in storage.
     */
    public function store(Request $request): JsonResponse
    {
        $title = trim($request->input('title', ''));
        $category = trim($request->input('category', 'Engineering'));
        $content = trim($request->input('content', ''));

        if (empty($title) || empty($content)) {
            return response()->json(['error' => 'Title and content are required.'], 422);
        }

        $userId = auth()->id() ?: (int)($request->cookie('blog_user') ?: 1);

        $imagePath = null;
        if ($request->hasFile('image') && $request->file('image')->isValid()) {
            $imagePath = $request->file('image')->store('posts', 'public');
        }

        $slug = strtolower(trim(preg_replace('/[^A-Za-z0-9-]+/', '-', $title), '-')) . '-' . substr(md5(uniqid()), 0, 6);

        $post = Post::create([
            'user_id' => $userId,
            'title' => $title,
            'slug' => $slug,
            'category' => $category,
            'content' => $content,
            'image_path' => $imagePath,
            'views_count' => 0,
            'likes_count' => 0,
            'is_published' => true,
        ]);

        return response()->json([
            'status' => 'success',
            'message' => 'Article published successfully.',
            'post' => [
                'id' => $post->id,
                'slug' => $post->slug,
            ],
        ], 201);
    }

    /**
     * Update the specified post.
     */
    public function update(Request $request, int|string $id): JsonResponse
    {
        $post = Post::find($id);

        if (!$post) {
            return response()->json(['error' => 'Article not found.'], 404);
        }

        if ($request->has('title')) {
            $post->title = $request->input('title');
        }
        if ($request->has('category')) {
            $post->category = $request->input('category');
        }
        if ($request->has('content')) {
            $post->content = $request->input('content');
        }

        $post->save();

        return response()->json([
            'status' => 'success',
            'message' => 'Article updated successfully.',
            'post' => $post,
        ]);
    }

    /**
     * Remove the specified post from storage.
     */
    public function destroy(int|string $id): JsonResponse
    {
        $post = Post::find($id);

        if (!$post) {
            return response()->json(['error' => 'Article not found.'], 404);
        }

        $post->comments()->delete();
        $post->delete();

        return response()->json([
            'status' => 'success',
            'message' => 'Article deleted successfully.',
        ]);
    }

    /**
     * Increment like count on the post.
     */
    public function like(int|string $id): JsonResponse
    {
        $post = Post::find($id);

        if (!$post) {
            return response()->json(['error' => 'Article not found.'], 404);
        }

        $post->increment('likes_count');

        return response()->json([
            'status' => 'success',
            'likes_count' => (int)$post->likes_count,
        ]);
    }
}
