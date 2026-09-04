<?php

namespace Database\Seeders;

use App\Models\User;
use App\Models\Post;
use App\Models\Comment;
use Illuminate\Database\Seeder;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Str;

class DatabaseSeeder extends Seeder
{
    public function run(): void
    {
        $elena = User::create([
            'name' => 'Elena Vance',
            'email' => 'elena@hyperion.io',
            'password' => Hash::make('password123'),
            'bio' => 'Principal Systems Architect specializing in Rust, Fibers, and High-Performance Runtimes.',
        ]);

        $linus = User::create([
            'name' => 'Linus Torvald',
            'email' => 'linus@kernel.org',
            'password' => Hash::make('password123'),
            'bio' => 'Low-level runtime developer & JIT compiler enthusiast.',
        ]);

        $post1 = Post::create([
            'user_id' => $elena->id,
            'title' => 'Architecting High-Throughput Async PHP Runtimes with Rust',
            'slug' => 'architecting-high-throughput-async-php-runtimes-with-rust-' . Str::random(6),
            'content' => "Building scalable web services requires eliminating thread starvation and CPU idling. In this deep dive, we explore how combining Rust's zero-cost abstractions with an asynchronous Mio reactor enables PHP to process over 2,000 requests per second with sub-millisecond latency.\n\nKey architectural pillars:\n1. M:N cooperative fiber scheduling\n2. Work-stealing thread pools\n3. Non-blocking SQLite & Redis database completion queues\n4. Zero-allocation string interning and nan-boxing.",
            'category' => 'Architecture',
            'views_count' => 142,
            'likes_count' => 19,
            'is_published' => true,
        ]);

        $post2 = Post::create([
            'user_id' => $linus->id,
            'title' => 'Inside the JIT Trace Compiler: AArch64 Machine Code Generation',
            'slug' => 'inside-the-jit-trace-compiler-aarch64-machine-code-generation-' . Str::random(6),
            'content' => "Tracing JIT compilers identify frequently executed bytecode loops and generate linear native machine code traces directly. By recording loop execution and generating dynasm assembly for AArch64 registers, we achieve up to 5x performance improvements over standard interpreted PHP.\n\nTopics covered:\n- Hot-loop loop counter thresholds\n- De-optimization bailouts for dynamic type mutations\n- Direct machine stack frame alignment and register preservation.",
            'category' => 'Performance & JIT',
            'views_count' => 280,
            'likes_count' => 38,
            'is_published' => true,
        ]);

        $post3 = Post::create([
            'user_id' => $elena->id,
            'title' => 'How Laravel 11 Scales on Fiber-Based Non-Blocking Sockets',
            'slug' => 'how-laravel-11-scales-on-fiber-based-non-blocking-sockets-' . Str::random(6),
            'content' => "Laravel 11 brings streamlined service providers and lightweight application bootstrap pipelines. When coupled with an in-memory persistent worker reactor, Laravel avoids cold-boot overhead entirely, processing requests directly in memory with Go-like response times.",
            'category' => 'Engineering',
            'views_count' => 95,
            'likes_count' => 12,
            'is_published' => true,
        ]);

        // Add Comments
        Comment::create([
            'post_id' => $post1->id,
            'user_id' => $linus->id,
            'author_name' => 'Linus Torvald',
            'content' => 'Fascinating breakdown of the M:N scheduler. The work-stealing deque implementation is remarkably clean.',
        ]);

        Comment::create([
            'post_id' => $post1->id,
            'user_id' => null,
            'author_name' => 'Sophia Dev',
            'content' => 'The 0.43ms response times are incredible! Does this support keep-alive connections out of the box?',
        ]);

        Comment::create([
            'post_id' => $post2->id,
            'user_id' => $elena->id,
            'author_name' => 'Elena Vance',
            'content' => 'Great explanation of the AArch64 C ABI register calling convention. Handling 24-byte return structs via out-pointers was key.',
        ]);
    }
}
