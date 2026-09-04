<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>⚡ PHP-Hyperion — Full Multi-Page Laravel Blog & Publishing Studio</title>
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700;800&family=Outfit:wght@600;700;800&display=swap" rel="stylesheet">
    <style>
        :root {
            --bg: #090d16;
            --surface: #0f172a;
            --card-bg: rgba(18, 24, 43, 0.75);
            --card-border: rgba(255, 255, 255, 0.08);
            --accent: #6366f1;
            --accent-hover: #4f46e5;
            --accent-glow: rgba(99, 102, 241, 0.35);
            --success: #10b981;
            --danger: #f43f5e;
            --warning: #f59e0b;
            --text-primary: #f8fafc;
            --text-secondary: #94a3b8;
            --text-muted: #64748b;
        }

        * {
            box-sizing: border-box;
            margin: 0;
            padding: 0;
            font-family: 'Inter', -apple-system, sans-serif;
        }

        body {
            background-color: var(--bg);
            color: var(--text-primary);
            min-height: 100vh;
            background-image: 
                radial-gradient(circle at 10% 10%, rgba(99, 102, 241, 0.12), transparent 40%),
                radial-gradient(circle at 90% 80%, rgba(16, 185, 129, 0.08), transparent 40%);
            display: flex;
            flex-direction: column;
        }

        /* Top Navbar */
        .navbar {
            background: rgba(15, 23, 42, 0.85);
            backdrop-filter: blur(16px);
            -webkit-backdrop-filter: blur(16px);
            border-bottom: 1px solid var(--card-border);
            padding: 14px 32px;
            display: flex;
            justify-content: space-between;
            align-items: center;
            position: sticky;
            top: 0;
            z-index: 100;
        }

        .nav-left {
            display: flex;
            align-items: center;
            gap: 32px;
        }

        .brand {
            display: flex;
            align-items: center;
            gap: 12px;
            text-decoration: none;
            color: var(--text-primary);
            cursor: pointer;
        }

        .brand-logo {
            width: 38px;
            height: 38px;
            border-radius: 10px;
            background: linear-gradient(135deg, #6366f1, #a855f7);
            display: flex;
            align-items: center;
            justify-content: center;
            font-size: 18px;
            box-shadow: 0 4px 14px var(--accent-glow);
        }

        .brand-title {
            font-family: 'Outfit', sans-serif;
            font-size: 19px;
            font-weight: 700;
            letter-spacing: -0.5px;
        }

        .nav-links {
            display: flex;
            align-items: center;
            gap: 8px;
        }

        .nav-link {
            color: var(--text-secondary);
            text-decoration: none;
            padding: 8px 14px;
            border-radius: 10px;
            font-size: 14px;
            font-weight: 500;
            cursor: pointer;
            transition: all 0.2s;
            display: flex;
            align-items: center;
            gap: 6px;
        }

        .nav-link:hover, .nav-link.active {
            color: #fff;
            background: rgba(255, 255, 255, 0.06);
        }

        .nav-link.active {
            color: #818cf8;
            background: rgba(99, 102, 241, 0.12);
        }

        .nav-right {
            display: flex;
            align-items: center;
            gap: 16px;
        }

        .user-pill {
            display: flex;
            align-items: center;
            gap: 10px;
            background: rgba(255, 255, 255, 0.05);
            padding: 5px 12px 5px 5px;
            border-radius: 30px;
            border: 1px solid var(--card-border);
            cursor: pointer;
        }

        .user-pill img {
            width: 30px;
            height: 30px;
            border-radius: 50%;
            object-fit: cover;
            border: 2px solid var(--accent);
        }

        /* Common Buttons */
        .btn {
            background: var(--accent);
            color: #fff;
            border: none;
            padding: 10px 18px;
            border-radius: 10px;
            font-size: 14px;
            font-weight: 600;
            cursor: pointer;
            transition: all 0.2s ease;
            display: inline-flex;
            align-items: center;
            gap: 8px;
            text-decoration: none;
        }

        .btn:hover {
            background: var(--accent-hover);
            box-shadow: 0 4px 16px var(--accent-glow);
            transform: translateY(-1px);
        }

        .btn-secondary {
            background: rgba(255, 255, 255, 0.06);
            color: var(--text-primary);
            border: 1px solid var(--card-border);
        }

        .btn-secondary:hover {
            background: rgba(255, 255, 255, 0.12);
            box-shadow: none;
        }

        .btn-danger {
            background: rgba(244, 63, 94, 0.15);
            color: #fb7185;
            border: 1px solid rgba(244, 63, 94, 0.3);
        }

        .btn-danger:hover {
            background: var(--danger);
            color: #fff;
        }

        .btn-sm {
            padding: 6px 12px;
            font-size: 13px;
            border-radius: 8px;
        }

        /* Container Layout */
        .container {
            max-width: 1200px;
            width: 100%;
            margin: 0 auto;
            padding: 32px 24px;
            flex: 1;
        }

        .page-view {
            display: none;
            animation: fadeIn 0.25s ease-out;
        }

        .page-view.active {
            display: block;
        }

        @keyframes fadeIn {
            from { opacity: 0; transform: translateY(6px); }
            to { opacity: 1; transform: translateY(0); }
        }

        /* Glass Cards */
        .card {
            background: var(--card-bg);
            backdrop-filter: blur(12px);
            -webkit-backdrop-filter: blur(12px);
            border: 1px solid var(--card-border);
            border-radius: 20px;
            padding: 24px;
            box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
        }

        /* 1. Hero Spotlight */
        .hero-banner {
            background: linear-gradient(135deg, rgba(99, 102, 241, 0.15), rgba(168, 85, 247, 0.15));
            border: 1px solid rgba(99, 102, 241, 0.25);
            border-radius: 24px;
            padding: 40px;
            margin-bottom: 40px;
            display: flex;
            justify-content: space-between;
            align-items: center;
            position: relative;
            overflow: hidden;
        }

        .hero-title {
            font-family: 'Outfit', sans-serif;
            font-size: 36px;
            font-weight: 800;
            line-height: 1.2;
            margin-bottom: 12px;
            background: linear-gradient(135deg, #fff, #a5b4fc);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
        }

        .hero-desc {
            font-size: 16px;
            color: var(--text-secondary);
            max-width: 620px;
            line-height: 1.6;
            margin-bottom: 24px;
        }

        /* Search & Filters */
        .feed-controls {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 28px;
            gap: 20px;
            flex-wrap: wrap;
        }

        .search-box {
            position: relative;
            flex: 1;
            max-width: 380px;
        }

        .search-box input {
            width: 100%;
            background: rgba(15, 23, 42, 0.8);
            border: 1px solid var(--card-border);
            color: var(--text-primary);
            padding: 10px 16px 10px 38px;
            border-radius: 12px;
            font-size: 14px;
            outline: none;
        }

        .search-box input:focus {
            border-color: var(--accent);
        }

        .search-icon {
            position: absolute;
            left: 12px;
            top: 50%;
            transform: translateY(-50%);
            color: var(--text-muted);
        }

        .category-pills {
            display: flex;
            gap: 8px;
            flex-wrap: wrap;
        }

        .category-pill {
            background: rgba(255, 255, 255, 0.04);
            color: var(--text-secondary);
            padding: 7px 16px;
            border-radius: 30px;
            font-size: 13px;
            font-weight: 500;
            cursor: pointer;
            border: 1px solid var(--card-border);
            transition: all 0.2s;
        }

        .category-pill:hover, .category-pill.active {
            background: rgba(99, 102, 241, 0.18);
            color: #a5b4fc;
            border-color: rgba(99, 102, 241, 0.4);
        }

        /* Post Cards Grid */
        .posts-grid {
            display: grid;
            grid-template-columns: repeat(auto-fill, minmax(350px, 1fr));
            gap: 24px;
        }

        .post-card {
            background: var(--card-bg);
            border: 1px solid var(--card-border);
            border-radius: 20px;
            overflow: hidden;
            display: flex;
            flex-direction: column;
            cursor: pointer;
            transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
        }

        .post-card:hover {
            transform: translateY(-4px);
            border-color: rgba(99, 102, 241, 0.4);
            box-shadow: 0 12px 30px rgba(0, 0, 0, 0.4);
        }

        .card-cover {
            width: 100%;
            height: 180px;
            object-fit: cover;
            background: linear-gradient(135deg, #1e1b4b, #312e81);
        }

        .card-body {
            padding: 22px;
            display: flex;
            flex-direction: column;
            flex: 1;
        }

        .card-meta {
            display: flex;
            align-items: center;
            justify-content: space-between;
            margin-bottom: 12px;
        }

        .card-badge {
            background: rgba(99, 102, 241, 0.15);
            color: #a5b4fc;
            font-size: 12px;
            font-weight: 600;
            padding: 4px 10px;
            border-radius: 6px;
        }

        .card-title {
            font-family: 'Outfit', sans-serif;
            font-size: 18px;
            font-weight: 700;
            line-height: 1.35;
            margin-bottom: 8px;
        }

        .card-excerpt {
            color: var(--text-secondary);
            font-size: 13px;
            line-height: 1.6;
            margin-bottom: 18px;
            flex: 1;
        }

        .card-footer {
            display: flex;
            align-items: center;
            justify-content: space-between;
            padding-top: 14px;
            border-top: 1px solid var(--card-border);
        }

        .author-box {
            display: flex;
            align-items: center;
            gap: 10px;
        }

        .author-avatar {
            width: 32px;
            height: 32px;
            border-radius: 50%;
            object-fit: cover;
            border: 2px solid var(--accent);
        }

        .author-name {
            font-size: 13px;
            font-weight: 600;
        }

        .card-stats {
            display: flex;
            align-items: center;
            gap: 12px;
            font-size: 12px;
            color: var(--text-muted);
        }

        /* 2. Single Post Reader View */
        .reader-container {
            max-width: 820px;
            margin: 0 auto;
        }

        .reader-hero-cover {
            width: 100%;
            height: 320px;
            border-radius: 20px;
            object-fit: cover;
            margin-bottom: 28px;
        }

        .reader-title {
            font-family: 'Outfit', sans-serif;
            font-size: 38px;
            font-weight: 800;
            line-height: 1.25;
            margin-bottom: 16px;
        }

        .reader-meta-bar {
            display: flex;
            align-items: center;
            justify-content: space-between;
            padding: 16px 0;
            border-bottom: 1px solid var(--card-border);
            margin-bottom: 28px;
        }

        .reader-body {
            font-size: 16px;
            line-height: 1.8;
            color: #cbd5e1;
            margin-bottom: 40px;
            white-space: pre-line;
        }

        .action-bar {
            display: flex;
            gap: 12px;
            padding: 20px 0;
            border-top: 1px solid var(--card-border);
            border-bottom: 1px solid var(--card-border);
            margin-bottom: 40px;
        }

        /* Comments */
        .comments-section {
            margin-top: 32px;
        }

        .comment-card {
            background: rgba(255, 255, 255, 0.03);
            border: 1px solid var(--card-border);
            border-radius: 14px;
            padding: 16px;
            margin-bottom: 14px;
        }

        .comment-header {
            display: flex;
            align-items: center;
            justify-content: space-between;
            margin-bottom: 8px;
        }

        /* 3. Studio Form */
        .studio-container {
            max-width: 800px;
            margin: 0 auto;
        }

        .form-group {
            margin-bottom: 20px;
        }

        label {
            display: block;
            font-size: 13px;
            font-weight: 600;
            color: var(--text-secondary);
            margin-bottom: 8px;
        }

        input[type="text"], input[type="email"], input[type="password"], select, textarea {
            width: 100%;
            background: rgba(15, 23, 42, 0.8);
            border: 1px solid var(--card-border);
            color: var(--text-primary);
            padding: 12px 16px;
            border-radius: 12px;
            font-size: 14px;
            outline: none;
            transition: all 0.2s;
        }

        input:focus, select:focus, textarea:focus {
            border-color: var(--accent);
            box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.2);
        }

        textarea {
            min-height: 220px;
            resize: vertical;
        }

        /* 4. Dashboard KPIs */
        .kpi-grid {
            display: grid;
            grid-template-columns: repeat(4, 1fr);
            gap: 20px;
            margin-bottom: 32px;
        }

        .kpi-card {
            background: var(--card-bg);
            border: 1px solid var(--card-border);
            border-radius: 16px;
            padding: 20px;
            text-align: center;
        }

        .kpi-num {
            font-family: 'Outfit', sans-serif;
            font-size: 32px;
            font-weight: 800;
            color: var(--accent);
            margin-bottom: 4px;
        }

        .kpi-label {
            font-size: 12px;
            color: var(--text-muted);
            text-transform: uppercase;
            letter-spacing: 0.5px;
            font-weight: 600;
        }

        /* Table */
        .data-table {
            width: 100%;
            border-collapse: collapse;
            text-align: left;
        }

        .data-table th, .data-table td {
            padding: 14px 16px;
            border-bottom: 1px solid var(--card-border);
            font-size: 14px;
        }

        .data-table th {
            color: var(--text-muted);
            font-weight: 600;
            text-transform: uppercase;
            font-size: 12px;
        }

        /* Toast */
        .toast {
            position: fixed;
            bottom: 24px;
            right: 24px;
            background: #1e293b;
            color: #fff;
            padding: 14px 20px;
            border-radius: 12px;
            border-left: 4px solid var(--accent);
            box-shadow: 0 10px 25px rgba(0,0,0,0.5);
            z-index: 1000;
            animation: slideIn 0.3s ease;
        }

        @keyframes slideIn {
            from { transform: translateX(100%); opacity: 0; }
            to { transform: translateX(0); opacity: 1; }
        }

        /* Modal */
        .modal-overlay {
            position: fixed;
            top: 0; left: 0; right: 0; bottom: 0;
            background: rgba(0,0,0,0.8);
            backdrop-filter: blur(8px);
            display: flex;
            align-items: center;
            justify-content: center;
            z-index: 200;
        }

        .modal-card {
            background: var(--surface);
            border: 1px solid var(--card-border);
            border-radius: 20px;
            max-width: 500px;
            width: 90%;
            padding: 28px;
        }
    </style>
</head>
<body>

    <!-- Global Top Navigation -->
    <header class="navbar">
        <div class="nav-left">
            <div class="brand" onclick="navigateTo('home')">
                <div class="brand-logo">⚡</div>
                <div>
                    <div class="brand-title">Hyperion Blog</div>
                    <div style="font-size: 11px; color: var(--text-muted);">Native JIT • Laravel 11</div>
                </div>
            </div>

            <nav class="nav-links">
                <a class="nav-link active" id="nav-home" onclick="navigateTo('home')">🏠 Feed</a>
                <a class="nav-link" id="nav-studio" onclick="navigateTo('studio')">✍️ Write Story</a>
                <a class="nav-link" id="nav-dashboard" onclick="navigateTo('dashboard')">📊 Studio Dashboard</a>
            </nav>
        </div>

        <div class="nav-right" id="nav-user-container">
            <!-- Rendered by JS -->
            <button class="btn btn-sm btn-secondary" onclick="showAuthModal('login')">Sign In</button>
            <button class="btn btn-sm" onclick="showAuthModal('register')">Get Started</button>
        </div>
    </header>

    <!-- Main Dynamic Application Container -->
    <main class="container">

        <!-- ================================================================= -->
        <!-- VIEW 1: HOME FEED -->
        <!-- ================================================================= -->
        <section class="page-view active" id="view-home">
            <!-- Hero Spotlight -->
            <div class="hero-banner">
                <div>
                    <span class="card-badge" style="margin-bottom: 12px; display: inline-block;">FEATURED STORY</span>
                    <h1 class="hero-title">Architecting Next-Gen Async PHP on Rust</h1>
                    <p class="hero-desc">Discover how cooperative M:N fiber scheduling, JIT trace compilation, and non-blocking I/O deliver over 2,000 requests per second with sub-millisecond latency.</p>
                    <button class="btn" onclick="openPost(1)">📖 Read Full Article</button>
                </div>
            </div>

            <!-- Feed Controls (Search & Categories) -->
            <div class="feed-controls">
                <div class="category-pills">
                    <div class="category-pill active" onclick="setCategory('All')">All Topics</div>
                    <div class="category-pill" onclick="setCategory('Architecture')">🏛️ Architecture</div>
                    <div class="category-pill" onclick="setCategory('Performance & JIT')">⚡ Performance & JIT</div>
                    <div class="category-pill" onclick="setCategory('Engineering')">⚙️ Engineering</div>
                    <div class="category-pill" onclick="setCategory('Deep Dive')">🔍 Deep Dive</div>
                </div>

                <div class="search-box">
                    <span class="search-icon">🔍</span>
                    <input type="text" id="search-input" placeholder="Search articles, topics, keywords..." oninput="handleSearch(this.value)">
                </div>
            </div>

            <!-- Posts Grid -->
            <div class="posts-grid" id="posts-grid-container">
                <!-- Populated dynamically via JS -->
            </div>
        </section>

        <!-- ================================================================= -->
        <!-- VIEW 2: POST READER -->
        <!-- ================================================================= -->
        <section class="page-view" id="view-reader">
            <div class="reader-container">
                <button class="btn btn-sm btn-secondary" style="margin-bottom: 20px;" onclick="navigateTo('home')">← Back to Articles</button>

                <img id="reader-cover" src="" class="reader-hero-cover" style="display: none;" alt="Cover Banner">
                <span class="card-badge" id="reader-category" style="margin-bottom: 12px; display: inline-block;">Category</span>
                <h1 class="reader-title" id="reader-title"></h1>

                <div class="reader-meta-bar">
                    <div class="author-box" id="reader-author-box">
                        <img id="reader-author-avatar" src="" class="author-avatar" alt="Avatar">
                        <div>
                            <div class="author-name" id="reader-author-name"></div>
                            <div style="font-size: 12px; color: var(--text-muted);" id="reader-date"></div>
                        </div>
                    </div>

                    <div style="display: flex; gap: 16px; font-size: 13px; color: var(--text-muted);">
                        <span id="reader-views">👁️ 0 views</span>
                        <span>⚡ Hyperion JIT Active</span>
                    </div>
                </div>

                <div class="reader-body" id="reader-content"></div>

                <!-- Like & Share Bar -->
                <div class="action-bar">
                    <button class="btn btn-secondary" id="btn-like-post" onclick="handleLikePost()">
                        ❤️ Like (<span id="reader-likes-count">0</span>)
                    </button>
                    <button class="btn btn-secondary" onclick="navigator.clipboard.writeText(window.location.href); showToast('🔗 Article link copied to clipboard!');">
                        🔗 Share
                    </button>
                </div>

                <!-- Discussion & Comments -->
                <div class="comments-section">
                    <h3 style="font-family: 'Outfit'; font-size: 22px; margin-bottom: 16px;">💬 Discussion (<span id="reader-comments-count">0</span>)</h3>

                    <!-- Post Comment Form -->
                    <div class="card" style="margin-bottom: 24px;">
                        <form onsubmit="handlePostComment(event)">
                            <div class="form-group" id="comment-guest-name-box">
                                <label for="comment-author-name">Your Name</label>
                                <input type="text" id="comment-author-name" placeholder="e.g. Alex Tech Lead">
                            </div>
                            <div class="form-group">
                                <label for="comment-content">Join the discussion</label>
                                <textarea id="comment-content" required placeholder="Write your thoughts, questions, or insights..." style="min-height: 90px;"></textarea>
                            </div>
                            <button type="submit" class="btn btn-sm">💬 Post Comment</button>
                        </form>
                    </div>

                    <!-- Comments List -->
                    <div id="reader-comments-list"></div>
                </div>
            </div>
        </section>

        <!-- ================================================================= -->
        <!-- VIEW 3: WRITING STUDIO -->
        <!-- ================================================================= -->
        <section class="page-view" id="view-studio">
            <div class="studio-container">
                <div style="margin-bottom: 28px;">
                    <h1 style="font-family: 'Outfit'; font-size: 32px; font-weight: 800;">✍️ Publishing Studio</h1>
                    <p style="color: var(--text-secondary); font-size: 14px;">Write and publish high-performance technical articles on PHP-Hyperion.</p>
                </div>

                <div class="card">
                    <form onsubmit="handlePublishStory(event)">
                        <div class="form-group">
                            <label for="studio-title">Article Title</label>
                            <input type="text" id="studio-title" required placeholder="e.g. Deep Dive into Fiber Scheduling & Event Reactors">
                        </div>

                        <div class="form-group">
                            <label for="studio-category">Topic Category</label>
                            <select id="studio-category">
                                <option value="Architecture">🏛️ Architecture</option>
                                <option value="Performance & JIT">⚡ Performance & JIT</option>
                                <option value="Engineering">⚙️ Engineering</option>
                                <option value="Deep Dive">🔍 Deep Dive</option>
                                <option value="General">📰 General</option>
                            </select>
                        </div>

                        <div class="form-group">
                            <label>Featured Cover Image</label>
                            <div style="display: flex; gap: 12px; align-items: center;">
                                <label for="studio-image-file" class="btn btn-sm btn-secondary" style="cursor: pointer;">
                                    🖼️ Choose Image File
                                </label>
                                <input type="file" id="studio-image-file" accept="image/*" style="display: none;" onchange="previewStudioImage(event)">
                                <span id="studio-image-label" style="font-size: 12px; color: var(--text-muted);">No cover selected</span>
                            </div>
                            <input type="hidden" id="studio-image-base64">
                            <img id="studio-image-preview" src="" style="width: 100%; height: 160px; object-fit: cover; border-radius: 12px; margin-top: 12px; display: none;" alt="Preview">
                        </div>

                        <div class="form-group">
                            <label for="studio-content">Article Content (Markdown supported)</label>
                            <textarea id="studio-content" required placeholder="Draft your story here..."></textarea>
                        </div>

                        <div style="display: flex; justify-content: flex-end; gap: 12px; margin-top: 24px;">
                            <button type="button" class="btn btn-secondary" onclick="navigateTo('home')">Cancel</button>
                            <button type="submit" id="btn-publish-story" class="btn">🚀 Publish Article</button>
                        </div>
                    </form>
                </div>
            </div>
        </section>

        <!-- ================================================================= -->
        <!-- VIEW 4: AUTHOR PROFILE -->
        <!-- ================================================================= -->
        <section class="page-view" id="view-profile">
            <div style="max-width: 860px; margin: 0 auto;">
                <div class="card" style="margin-bottom: 32px; display: flex; gap: 28px; align-items: center;">
                    <div style="position: relative;">
                        <img id="profile-avatar" src="" class="author-avatar" style="width: 90px; height: 90px; border-width: 3px;" alt="Avatar">
                        <label for="profile-photo-file" style="position: absolute; bottom: 0; right: 0; background: var(--accent); color: #fff; width: 28px; height: 28px; border-radius: 50%; display: flex; align-items: center; justify-content: center; cursor: pointer; border: 2px solid var(--surface);" title="Change Avatar">✎</label>
                        <input type="file" id="profile-photo-file" accept="image/*" style="display: none;" onchange="uploadProfilePhoto(event)">
                    </div>
                    <div style="flex: 1;">
                        <h2 id="profile-name" style="font-family: 'Outfit'; font-size: 26px; font-weight: 700;"></h2>
                        <p id="profile-email" style="font-size: 13px; color: var(--text-secondary); margin-bottom: 6px;"></p>
                        <p id="profile-bio" style="font-size: 14px; color: var(--text-muted); line-height: 1.5;"></p>
                    </div>
                </div>

                <h3 style="font-family: 'Outfit'; font-size: 22px; margin-bottom: 20px;">Published Stories</h3>
                <div class="posts-grid" id="profile-posts-container"></div>
            </div>
        </section>

        <!-- ================================================================= -->
        <!-- VIEW 5: STUDIO ANALYTICS DASHBOARD -->
        <!-- ================================================================= -->
        <section class="page-view" id="view-dashboard">
            <div style="margin-bottom: 28px;">
                <h1 style="font-family: 'Outfit'; font-size: 32px; font-weight: 800;">📊 Studio Analytics & Management</h1>
                <p style="color: var(--text-secondary); font-size: 14px;">Monitor readership, engagement metrics, and manage your articles.</p>
            </div>

            <!-- KPI Cards -->
            <div class="kpi-grid">
                <div class="kpi-card">
                    <div class="kpi-num" id="kpi-posts">0</div>
                    <div class="kpi-label">Total Articles</div>
                </div>
                <div class="kpi-card">
                    <div class="kpi-num" id="kpi-views" style="color: var(--success);">0</div>
                    <div class="kpi-label">Total Views</div>
                </div>
                <div class="kpi-card">
                    <div class="kpi-num" id="kpi-likes" style="color: #fb7185;">0</div>
                    <div class="kpi-label">Total Likes</div>
                </div>
                <div class="kpi-card">
                    <div class="kpi-num" id="kpi-comments" style="color: #38bdf8;">0</div>
                    <div class="kpi-label">Total Comments</div>
                </div>
            </div>

            <!-- Manage Posts Table -->
            <div class="card">
                <h3 style="font-family: 'Outfit'; font-size: 20px; font-weight: 700; margin-bottom: 20px;">Manage Your Articles</h3>
                <table class="data-table">
                    <thead>
                        <tr>
                            <th>Article</th>
                            <th>Topic</th>
                            <th>Views</th>
                            <th>Likes</th>
                            <th>Published</th>
                            <th style="text-align: right;">Actions</th>
                        </tr>
                    </thead>
                    <tbody id="dashboard-posts-table-body">
                        <!-- Populated by JS -->
                    </tbody>
                </table>
            </div>
        </section>

    </main>

    <!-- Auth Modal (Sign In / Register) -->
    <div class="modal-overlay" id="auth-modal" style="display: none;">
        <div class="modal-card">
            <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px;">
                <h3 id="auth-modal-title" style="font-family: 'Outfit'; font-size: 22px;">Sign In to Hyperion</h3>
                <button class="btn btn-sm btn-secondary" onclick="closeAuthModal()">✕</button>
            </div>

            <!-- Register Form -->
            <form id="modal-form-register" onsubmit="handleAuthRegister(event)" style="display: none;">
                <div class="form-group">
                    <label>Full Name</label>
                    <input type="text" id="auth-reg-name" required placeholder="Elena Vance">
                </div>
                <div class="form-group">
                    <label>Email Address</label>
                    <input type="email" id="auth-reg-email" required placeholder="elena@hyperion.io">
                </div>
                <div class="form-group">
                    <label>Password</label>
                    <input type="password" id="auth-reg-password" required placeholder="••••••••">
                </div>
                <div class="form-group">
                    <label>Bio</label>
                    <input type="text" id="auth-reg-bio" placeholder="Principal Systems Architect">
                </div>
                <button type="submit" class="btn" style="width: 100%;">Create Account</button>
                <div style="text-align: center; margin-top: 14px; font-size: 13px; color: var(--text-secondary);">
                    Already have an account? <a style="color: var(--accent); cursor: pointer;" onclick="showAuthModal('login')">Sign In</a>
                </div>
            </form>

            <!-- Login Form -->
            <form id="modal-form-login" onsubmit="handleAuthLogin(event)">
                <div class="form-group">
                    <label>Email Address</label>
                    <input type="email" id="auth-login-email" required placeholder="elena@hyperion.io">
                </div>
                <div class="form-group">
                    <label>Password</label>
                    <input type="password" id="auth-login-password" required placeholder="••••••••">
                </div>
                <button type="submit" class="btn" style="width: 100%;">Sign In</button>
                <div style="text-align: center; margin-top: 14px; font-size: 13px; color: var(--text-secondary);">
                    New writer? <a style="color: var(--accent); cursor: pointer;" onclick="showAuthModal('register')">Create an account</a>
                </div>
            </form>
        </div>
    </div>

    <!-- Edit Post Modal -->
    <div class="modal-overlay" id="edit-modal" style="display: none;">
        <div class="modal-card">
            <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px;">
                <h3 style="font-family: 'Outfit'; font-size: 20px;">✏️ Edit Article</h3>
                <button class="btn btn-sm btn-secondary" onclick="closeEditModal()">✕</button>
            </div>
            <form onsubmit="handleSaveEditPost(event)">
                <input type="hidden" id="edit-post-id">
                <div class="form-group">
                    <label>Title</label>
                    <input type="text" id="edit-post-title" required>
                </div>
                <div class="form-group">
                    <label>Category</label>
                    <select id="edit-post-category">
                        <option value="Architecture">Architecture</option>
                        <option value="Performance & JIT">Performance & JIT</option>
                        <option value="Engineering">Engineering</option>
                        <option value="Deep Dive">Deep Dive</option>
                        <option value="General">General</option>
                    </select>
                </div>
                <div class="form-group">
                    <label>Content</label>
                    <textarea id="edit-post-content" required style="min-height: 140px;"></textarea>
                </div>
                <div style="display: flex; justify-content: flex-end; gap: 10px; margin-top: 20px;">
                    <button type="button" class="btn btn-secondary" onclick="closeEditModal()">Cancel</button>
                    <button type="submit" class="btn">Save Changes</button>
                </div>
            </form>
        </div>
    </div>

    <!-- Application JavaScript Engine -->
    <script>
        let currentUser = null;
        let allPosts = [];
        let activeCategory = 'All';
        let currentOpenPostId = null;

        async function init() {
            await checkAuth();
            await loadFeed();
        }

        // Navigation Routing
        function navigateTo(viewName) {
            document.querySelectorAll('.page-view').forEach(el => el.classList.remove('active'));
            document.querySelectorAll('.nav-link').forEach(el => el.classList.remove('active'));

            const targetView = document.getElementById('view-' + viewName);
            if (targetView) targetView.classList.add('active');

            const navLink = document.getElementById('nav-' + viewName);
            if (navLink) navLink.classList.add('active');

            window.scrollTo({ top: 0, behavior: 'smooth' });

            if (viewName === 'dashboard') loadDashboard();
            if (viewName === 'profile' && currentUser) loadAuthorProfile(currentUser.id);
            if (viewName === 'home') loadFeed();
        }

        // Authentication
        async function checkAuth() {
            try {
                const res = await fetch('/api/me');
                const data = await res.json();
                currentUser = data.user;
                renderNavAuth();
            } catch (e) {
                currentUser = null;
                renderNavAuth();
            }
        }

        function renderNavAuth() {
            const container = document.getElementById('nav-user-container');
            const guestBox = document.getElementById('comment-guest-name-box');

            if (currentUser) {
                if (guestBox) guestBox.style.display = 'none';
                container.innerHTML = `
                    <div class="user-pill" onclick="navigateTo('profile')">
                        <img src="${currentUser.avatar_url}" alt="Avatar">
                        <span style="font-size: 13px; font-weight: 600;">${currentUser.name}</span>
                    </div>
                    <button class="btn btn-sm btn-secondary" onclick="handleLogout()">Sign Out</button>
                `;
            } else {
                if (guestBox) guestBox.style.display = 'block';
                container.innerHTML = `
                    <button class="btn btn-sm btn-secondary" onclick="showAuthModal('login')">Sign In</button>
                    <button class="btn btn-sm" onclick="showAuthModal('register')">Get Started</button>
                `;
            }
        }

        function showAuthModal(mode) {
            document.getElementById('auth-modal').style.display = 'flex';
            document.getElementById('auth-modal-title').textContent = mode === 'register' ? 'Join Hyperion Blog' : 'Sign In to Hyperion';
            document.getElementById('modal-form-register').style.display = mode === 'register' ? 'block' : 'none';
            document.getElementById('modal-form-login').style.display = mode === 'login' ? 'block' : 'none';
        }

        function closeAuthModal() {
            document.getElementById('auth-modal').style.display = 'none';
        }

        async function handleAuthLogin(e) {
            e.preventDefault();
            const email = document.getElementById('auth-login-email').value;
            const password = document.getElementById('auth-login-password').value;

            const res = await fetch('/api/login', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ email, password })
            });
            const data = await res.json();
            if (res.ok) {
                currentUser = data.user;
                closeAuthModal();
                renderNavAuth();
                showToast('👋 Welcome back, ' + currentUser.name);
            } else {
                showToast(data.error || 'Login failed', 'error');
            }
        }

        async function handleAuthRegister(e) {
            e.preventDefault();
            const name = document.getElementById('auth-reg-name').value;
            const email = document.getElementById('auth-reg-email').value;
            const password = document.getElementById('auth-reg-password').value;
            const bio = document.getElementById('auth-reg-bio').value;

            const res = await fetch('/api/register', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ name, email, password, bio })
            });
            const data = await res.json();
            if (res.ok) {
                currentUser = data.user;
                closeAuthModal();
                renderNavAuth();
                showToast('🎉 Account created successfully!');
            } else {
                showToast(data.error || 'Registration failed', 'error');
            }
        }

        async function handleLogout() {
            await fetch('/api/logout', { method: 'POST' });
            currentUser = null;
            renderNavAuth();
            showToast('👋 Signed out.');
            navigateTo('home');
        }

        // Feed & Search
        async function loadFeed() {
            let url = '/api/posts';
            const params = [];
            if (activeCategory !== 'All') params.push(`category=${encodeURIComponent(activeCategory)}`);
            const searchVal = document.getElementById('search-input')?.value;
            if (searchVal) params.push(`search=${encodeURIComponent(searchVal)}`);
            if (params.length > 0) url += '?' + params.join('&');

            const res = await fetch(url);
            const data = await res.json();
            allPosts = Array.isArray(data.posts) ? data.posts : (data.posts && data.posts.items ? data.posts.items : []);
            renderPostsGrid();
        }

        function setCategory(cat) {
            activeCategory = cat;
            document.querySelectorAll('.category-pill').forEach(p => {
                p.classList.toggle('active', p.textContent.includes(cat) || (cat === 'All' && p.textContent.includes('All')));
            });
            loadFeed();
        }

        function handleSearch(query) {
            loadFeed();
        }

        function renderPostsGrid() {
            const container = document.getElementById('posts-grid-container');
            if (allPosts.length === 0) {
                container.innerHTML = `
                    <div class="card" style="grid-column: 1 / -1; text-align: center; padding: 48px;">
                        <h3>No articles found</h3>
                        <p style="color: var(--text-muted); margin-top: 6px;">Try adjusting your search query or topic filter.</p>
                    </div>
                `;
                return;
            }

            container.innerHTML = allPosts.map(post => {
                const authorName = post.author ? post.author.name : 'Hyperion Author';
                const authorAvatar = post.author ? post.author.avatar_url : 'https://ui-avatars.com/api/?name=Author';

                const coverImg = post.image_url ? 
                    `<img src="${post.image_url}" class="card-cover" alt="Cover">` : 
                    `<div class="card-cover" style="display:flex;align-items:center;justify-content:center;font-size:32px;">📄</div>`;

                return `
                    <article class="post-card" onclick="openPost(${post.id})">
                        ${coverImg}
                        <div class="card-body">
                            <div class="card-meta">
                                <span class="card-badge">${post.category}</span>
                                <span style="font-size: 12px; color: var(--text-muted);">${post.created_at}</span>
                            </div>
                            <h3 class="card-title">${escapeHtml(post.title)}</h3>
                            <p class="card-excerpt">${escapeHtml(post.excerpt)}</p>
                            <div class="card-footer">
                                <div class="author-box">
                                    <img src="${authorAvatar}" class="author-avatar" alt="${authorName}">
                                    <span class="author-name">${authorName}</span>
                                </div>
                                <div class="card-stats">
                                    <span>👁️ ${post.views_count}</span>
                                    <span>❤️ ${post.likes_count}</span>
                                    <span>💬 ${post.comments_count}</span>
                                </div>
                            </div>
                        </div>
                    </article>
                `;
            }).join('');
        }

        // Single Post Reader
        async function openPost(id) {
            currentOpenPostId = id;
            const res = await fetch(`/api/posts/${id}`);
            const data = await res.json();
            if (!res.ok) {
                showToast('Article not found', 'error');
                return;
            }

            const post = data.post;
            document.getElementById('reader-title').textContent = post.title;
            document.getElementById('reader-category').textContent = post.category;
            document.getElementById('reader-content').textContent = post.content;
            document.getElementById('reader-views').textContent = `👁️ ${post.views_count} views`;
            document.getElementById('reader-likes-count').textContent = post.likes_count;
            document.getElementById('reader-date').textContent = post.created_at;

            const author = post.author || { name: 'Hyperion Author', bio: 'Technical Writer', avatar_url: 'https://ui-avatars.com/api/?name=Author' };
            document.getElementById('reader-author-name').textContent = author.name;
            document.getElementById('reader-author-avatar').src = author.avatar_url;

            const coverEl = document.getElementById('reader-cover');
            if (post.image_url) {
                coverEl.src = post.image_url;
                coverEl.style.display = 'block';
            } else {
                coverEl.style.display = 'none';
            }

            renderComments(post.comments || []);
            navigateTo('reader');
        }

        function renderComments(comments) {
            const list = Array.isArray(comments) ? comments : (comments && comments.items ? comments.items : []);
            document.getElementById('reader-comments-count').textContent = list.length;
            const container = document.getElementById('reader-comments-list');
            if (list.length === 0) {
                container.innerHTML = '<p style="color: var(--text-muted); font-size: 14px;">No comments yet. Start the discussion!</p>';
                return;
            }

            container.innerHTML = list.map(c => `
                <div class="comment-card">
                    <div class="comment-header">
                        <div style="display: flex; align-items: center; gap: 8px;">
                            <img src="${c.author_avatar}" style="width: 24px; height: 24px; border-radius: 50%;" alt="Avatar">
                            <strong style="font-size: 13px;">${escapeHtml(c.author_name)}</strong>
                        </div>
                        <span style="font-size: 11px; color: var(--text-muted);">${c.created_at}</span>
                    </div>
                    <p style="font-size: 14px; line-height: 1.5; color: #cbd5e1;">${escapeHtml(c.content)}</p>
                </div>
            `).join('');
        }

        async function handleLikePost() {
            if (!currentOpenPostId) return;
            const res = await fetch(`/api/posts/${currentOpenPostId}/like`, { method: 'POST' });
            const data = await res.json();
            if (res.ok) {
                document.getElementById('reader-likes-count').textContent = data.likes_count;
                showToast('❤️ You liked this article!');
            }
        }

        async function handlePostComment(e) {
            e.preventDefault();
            if (!currentOpenPostId) return;

            const content = document.getElementById('comment-content').value;
            const authorName = document.getElementById('comment-author-name')?.value || 'Guest Reader';

            const res = await fetch(`/api/posts/${currentOpenPostId}/comments`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ content, author_name: authorName })
            });
            if (res.ok) {
                document.getElementById('comment-content').value = '';
                showToast('💬 Comment posted!');
                openPost(currentOpenPostId);
            }
        }

        // Publishing Studio
        function previewStudioImage(event) {
            const file = event.target.files[0];
            if (file) {
                document.getElementById('studio-image-label').textContent = file.name;
                const reader = new FileReader();
                reader.onload = function(e) {
                    const preview = document.getElementById('studio-image-preview');
                    preview.src = e.target.result;
                    preview.style.display = 'block';
                    document.getElementById('studio-image-base64').value = e.target.result.split(',')[1];
                };
                reader.readAsDataURL(file);
            }
        }

        async function handlePublishStory(e) {
            e.preventDefault();
            const title = document.getElementById('studio-title').value;
            const category = document.getElementById('studio-category').value;
            const content = document.getElementById('studio-content').value;
            const imageBase64 = document.getElementById('studio-image-base64').value;

            const res = await fetch('/api/posts', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    title, category, content,
                    image_base64: imageBase64 || undefined
                })
            });

            if (res.ok) {
                const data = await res.json();
                showToast('🚀 Story published successfully!');
                document.getElementById('studio-title').value = '';
                document.getElementById('studio-content').value = '';
                document.getElementById('studio-image-preview').style.display = 'none';
                document.getElementById('studio-image-base64').value = '';
                openPost(data.post.id);
            } else {
                showToast('Failed to publish', 'error');
            }
        }

        // Dashboard & Author Analytics
        async function loadDashboard() {
            const res = await fetch('/api/dashboard/stats');
            const data = await res.json();
            if (!res.ok) return;

            document.getElementById('kpi-posts').textContent = data.stats.total_posts;
            document.getElementById('kpi-views').textContent = data.stats.total_views;
            document.getElementById('kpi-likes').textContent = data.stats.total_likes;
            document.getElementById('kpi-comments').textContent = data.stats.total_comments;

            const tableBody = document.getElementById('dashboard-posts-table-body');
            if (data.posts.length === 0) {
                tableBody.innerHTML = '<tr><td colspan="6" style="text-align: center; color: var(--text-muted);">No articles published yet.</td></tr>';
                return;
            }

            tableBody.innerHTML = data.posts.map(p => `
                <tr>
                    <td style="font-weight: 600; cursor: pointer; color: var(--accent);" onclick="openPost(${p.id})">${escapeHtml(p.title)}</td>
                    <td><span class="card-badge">${p.category}</span></td>
                    <td>👁️ ${p.views_count}</td>
                    <td>❤️ ${p.likes_count}</td>
                    <td>${p.created_at}</td>
                    <td style="text-align: right;">
                        <button class="btn btn-sm btn-secondary" onclick="openEditModal(${p.id}, '${escapeQuotes(p.title)}', '${p.category}')">Edit</button>
                        <button class="btn btn-sm btn-danger" onclick="handleDeletePost(${p.id})">Delete</button>
                    </td>
                </tr>
            `).join('');
        }

        function openEditModal(id, title, category) {
            document.getElementById('edit-post-id').value = id;
            document.getElementById('edit-post-title').value = title;
            document.getElementById('edit-post-category').value = category;
            document.getElementById('edit-modal').style.display = 'flex';
        }

        function closeEditModal() {
            document.getElementById('edit-modal').style.display = 'none';
        }

        async function handleSaveEditPost(e) {
            e.preventDefault();
            const id = document.getElementById('edit-post-id').value;
            const title = document.getElementById('edit-post-title').value;
            const category = document.getElementById('edit-post-category').value;
            const content = document.getElementById('edit-post-content').value;

            const res = await fetch(`/api/posts/${id}`, {
                method: 'PUT',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ title, category, content })
            });
            if (res.ok) {
                closeEditModal();
                showToast('✏️ Article updated successfully!');
                loadDashboard();
            }
        }

        async function handleDeletePost(id) {
            if (!confirm('Are you sure you want to delete this article?')) return;
            const res = await fetch(`/api/posts/${id}`, { method: 'DELETE' });
            if (res.ok) {
                showToast('🗑️ Article deleted.');
                loadDashboard();
            }
        }

        async function loadAuthorProfile(authorId) {
            const res = await fetch(`/api/authors/${authorId}`);
            const data = await res.json();
            if (!res.ok) return;

            const author = data.author;
            document.getElementById('profile-avatar').src = author.avatar_url;
            document.getElementById('profile-name').textContent = author.name;
            document.getElementById('profile-email').textContent = author.email;
            document.getElementById('profile-bio').textContent = author.bio || 'Technical Writer at Hyperion Blog';

            const container = document.getElementById('profile-posts-container');
            container.innerHTML = (author.posts || []).map(p => `
                <article class="post-card" onclick="openPost(${p.id})">
                    <div class="card-body">
                        <div class="card-meta">
                            <span class="card-badge">${p.category}</span>
                            <span style="font-size: 12px; color: var(--text-muted);">${p.created_at}</span>
                        </div>
                        <h3 class="card-title">${escapeHtml(p.title)}</h3>
                        <div class="card-footer">
                            <span>👁️ ${p.views_count} views</span>
                            <span>❤️ ${p.likes_count} likes</span>
                        </div>
                    </div>
                </article>
            `).join('');
        }

        async function uploadProfilePhoto(e) {
            const file = e.target.files[0];
            if (!file) return;
            const reader = new FileReader();
            reader.onload = async function(ev) {
                const base64 = ev.target.result.split(',')[1];
                const res = await fetch('/api/profile/photo', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ photo_base64: base64 })
                });
                if (res.ok) {
                    const data = await res.json();
                    document.getElementById('profile-avatar').src = data.avatar_url;
                    if (currentUser) currentUser.avatar_url = data.avatar_url;
                    renderNavAuth();
                    showToast('📷 Avatar updated successfully!');
                }
            };
            reader.readAsDataURL(file);
        }

        function showToast(msg, type = 'success') {
            const toast = document.createElement('div');
            toast.className = 'toast';
            toast.style.borderLeftColor = type === 'error' ? 'var(--danger)' : 'var(--accent)';
            toast.innerHTML = `<span>${msg}</span>`;
            document.body.appendChild(toast);
            setTimeout(() => toast.remove(), 4000);
        }

        function escapeHtml(str) {
            if (!str) return '';
            return str.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;").replace(/'/g, "&#039;");
        }

        function escapeQuotes(str) {
            if (!str) return '';
            return str.replace(/'/g, "\\'");
        }

        window.addEventListener('DOMContentLoaded', init);
    </script>
</body>
</html>
