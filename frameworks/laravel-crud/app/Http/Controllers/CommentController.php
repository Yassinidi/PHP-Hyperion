<?php

namespace App\Http\Controllers;

use Illuminate\Http\Request;
use Illuminate\Http\JsonResponse;
use App\Models\Post;
use App\Models\Comment;

class CommentController extends Controller
{
    /**
     * Store a newly created comment for a specific post.
     */
    public function store(Request $request, int|string $postId): JsonResponse
    {
        $post = Post::find($postId);

        if (!$post) {
            return response()->json(['error' => 'Article not found.'], 404);
        }

        $content = trim($request->input('content', ''));
        $authorName = trim($request->input('author_name', ''));

        if (empty($content)) {
            return response()->json(['error' => 'Comment content is required.'], 422);
        }

        $user = auth()->user();
        $userId = $user ? $user->id : null;
        $name = $user ? $user->name : (!empty($authorName) ? $authorName : 'Guest Reader');
        $avatar = $user ? $user->profile_photo : null;

        $comment = Comment::create([
            'post_id' => $post->id,
            'user_id' => $userId,
            'author_name' => $name,
            'author_avatar' => $avatar,
            'content' => $content,
        ]);

        return response()->json([
            'status' => 'success',
            'comment' => [
                'id' => $comment->id,
                'author_name' => $comment->author_name,
                'author_avatar' => $comment->getAvatarUrl(),
                'content' => $comment->content,
                'created_at' => $comment->created_at ? $comment->created_at->format('M j, Y · g:i a') : date('M j, Y · g:i a'),
            ],
        ], 201);
    }
}
