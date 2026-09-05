<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Theme & Page Studio - Hyperion No-Code Customizer</title>
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700;800&display=swap" rel="stylesheet">
    <style>
        :root {
            --studio-bg: #090d16;
            --studio-sidebar: #0f1422;
            --studio-card: #151b2e;
            --studio-border: rgba(255, 255, 255, 0.08);
            --studio-accent: #6366f1;
            --studio-green: #10b981;
            --studio-text: #f8fafc;
            --studio-muted: #94a3b8;
        }
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body {
            font-family: 'Inter', sans-serif;
            background: var(--studio-bg);
            color: var(--studio-text);
            height: 100vh;
            overflow: hidden;
            display: flex;
            flex-direction: column;
        }
        /* Top Navigation */
        .studio-navbar {
            height: 60px;
            background: var(--studio-sidebar);
            border-bottom: 1px solid var(--studio-border);
            display: flex;
            align-items: center;
            justify-content: space-between;
            padding: 0 1.5rem;
            z-index: 10;
        }
        .navbar-brand {
            display: flex;
            align-items: center;
            gap: 0.75rem;
            font-weight: 800;
            font-size: 1.1rem;
        }
        .navbar-brand .badge {
            background: rgba(99, 102, 241, 0.2);
            color: #818cf8;
            font-size: 0.75rem;
            padding: 0.2rem 0.5rem;
            border-radius: 4px;
            font-weight: 700;
        }
        .navbar-actions {
            display: flex;
            align-items: center;
            gap: 1rem;
        }
        .btn-studio {
            padding: 0.5rem 1.25rem;
            border-radius: 8px;
            font-weight: 700;
            font-size: 0.85rem;
            cursor: pointer;
            border: none;
            display: inline-flex;
            align-items: center;
            gap: 0.4rem;
            transition: all 0.2s;
        }
        .btn-publish {
            background: var(--studio-green);
            color: white;
            box-shadow: 0 0 15px rgba(16, 185, 129, 0.35);
        }
        .btn-publish:hover {
            opacity: 0.9;
            transform: translateY(-1px);
        }
        .btn-reset {
            background: rgba(255, 255, 255, 0.06);
            color: var(--studio-muted);
            border: 1px solid var(--studio-border);
        }
        .btn-reset:hover {
            color: var(--studio-text);
            background: rgba(255, 255, 255, 0.1);
        }
        /* Studio Layout */
        .studio-body {
            flex: 1;
            display: flex;
            height: calc(100vh - 60px);
        }
        /* Left Sidebar: Controls */
        .studio-sidebar {
            width: 440px;
            min-width: 440px;
            background: var(--studio-sidebar);
            border-right: 1px solid var(--studio-border);
            display: flex;
            flex-direction: column;
            overflow: hidden;
        }
        .tabs-header {
            display: flex;
            border-bottom: 1px solid var(--studio-border);
            background: rgba(0, 0, 0, 0.2);
        }
        .tab-btn {
            flex: 1;
            padding: 0.85rem 0.5rem;
            text-align: center;
            background: none;
            border: none;
            color: var(--studio-muted);
            font-weight: 600;
            font-size: 0.85rem;
            cursor: pointer;
            border-bottom: 2px solid transparent;
            transition: all 0.2s;
        }
        .tab-btn.active {
            color: var(--studio-text);
            border-bottom-color: var(--studio-accent);
            background: rgba(99, 102, 241, 0.08);
        }
        .tab-content-container {
            flex: 1;
            overflow-y: auto;
            padding: 1.5rem;
        }
        .tab-pane {
            display: none;
        }
        .tab-pane.active {
            display: block;
        }
        /* Form Controls */
        .control-group {
            margin-bottom: 1.5rem;
        }
        .control-label {
            display: block;
            font-size: 0.8rem;
            font-weight: 700;
            text-transform: uppercase;
            letter-spacing: 0.05em;
            color: var(--studio-muted);
            margin-bottom: 0.5rem;
        }
        .form-control {
            width: 100%;
            background: var(--studio-card);
            border: 1px solid var(--studio-border);
            border-radius: 8px;
            padding: 0.65rem 0.85rem;
            color: var(--studio-text);
            font-size: 0.9rem;
            outline: none;
            transition: border-color 0.2s;
        }
        .form-control:focus {
            border-color: var(--studio-accent);
        }
        .color-row {
            display: flex;
            align-items: center;
            justify-content: space-between;
            padding: 0.75rem 1rem;
            background: var(--studio-card);
            border: 1px solid var(--studio-border);
            border-radius: 8px;
            margin-bottom: 0.75rem;
        }
        .color-row span {
            font-size: 0.9rem;
            font-weight: 500;
        }
        .color-input {
            width: 40px;
            height: 32px;
            padding: 0;
            border: 1px solid var(--studio-border);
            border-radius: 6px;
            background: none;
            cursor: pointer;
        }
        /* Theme Preset Cards */
        .theme-card {
            background: var(--studio-card);
            border: 2px solid var(--studio-border);
            border-radius: 12px;
            padding: 1.25rem;
            margin-bottom: 1rem;
            cursor: pointer;
            transition: all 0.2s;
        }
        .theme-card:hover {
            border-color: var(--studio-accent);
            transform: translateY(-2px);
        }
        .theme-card.active {
            border-color: var(--studio-accent);
            background: rgba(99, 102, 241, 0.08);
            box-shadow: 0 0 20px rgba(99, 102, 241, 0.25);
        }
        .theme-card-header {
            display: flex;
            align-items: center;
            justify-content: space-between;
            margin-bottom: 0.5rem;
        }
        .theme-card-title {
            font-weight: 700;
            font-size: 1rem;
        }
        .theme-palette {
            display: flex;
            gap: 0.4rem;
            margin-top: 0.75rem;
        }
        .palette-swatch {
            width: 22px;
            height: 22px;
            border-radius: 50%;
            border: 1px solid rgba(255, 255, 255, 0.2);
        }
        /* Section Items */
        .section-item-card {
            background: var(--studio-card);
            border: 1px solid var(--studio-border);
            border-radius: 10px;
            padding: 1rem 1.25rem;
            margin-bottom: 1rem;
        }
        .section-item-header {
            display: flex;
            align-items: center;
            justify-content: space-between;
            margin-bottom: 0.75rem;
        }
        .section-type-badge {
            font-size: 0.75rem;
            font-weight: 700;
            color: #818cf8;
            background: rgba(99, 102, 241, 0.15);
            padding: 0.2rem 0.5rem;
            border-radius: 4px;
        }
        /* Right Preview Area */
        .studio-preview-area {
            flex: 1;
            display: flex;
            flex-direction: column;
            background: #000;
        }
        .preview-toolbar {
            height: 48px;
            background: #111625;
            border-bottom: 1px solid var(--studio-border);
            display: flex;
            align-items: center;
            justify-content: space-between;
            padding: 0 1.5rem;
        }
        .viewport-switchers {
            display: flex;
            gap: 0.5rem;
        }
        .viewport-btn {
            background: rgba(255, 255, 255, 0.06);
            border: 1px solid var(--studio-border);
            color: var(--studio-muted);
            padding: 0.3rem 0.75rem;
            border-radius: 6px;
            font-size: 0.8rem;
            cursor: pointer;
        }
        .viewport-btn.active {
            color: white;
            background: var(--studio-accent);
            border-color: var(--studio-accent);
        }
        .preview-frame-container {
            flex: 1;
            display: flex;
            justify-content: center;
            align-items: center;
            padding: 1rem;
            background: #070a12;
        }
        .preview-iframe {
            width: 100%;
            height: 100%;
            border: 1px solid var(--studio-border);
            border-radius: 8px;
            background: #0a0e17;
            transition: width 0.3s ease;
        }
        .toast-notification {
            position: fixed;
            bottom: 24px;
            right: 24px;
            background: var(--studio-green);
            color: white;
            padding: 0.85rem 1.5rem;
            border-radius: 8px;
            font-weight: 700;
            font-size: 0.9rem;
            box-shadow: 0 10px 30px rgba(0,0,0,0.5);
            z-index: 1000;
            display: none;
            animation: slideUp 0.3s ease;
        }
        @keyframes slideUp {
            from { transform: translateY(20px); opacity: 0; }
            to { transform: translateY(0); opacity: 1; }
        }
    </style>
</head>
<body>
    <!-- Top Navbar -->
    <header class="studio-navbar">
        <div class="navbar-brand">
            <span style="font-size: 1.4rem;">🎨</span>
            <span>Hyperion Theme & Page Studio</span>
            <span class="badge">No-Code Live Editor</span>
        </div>
        <div class="navbar-actions">
            <button type="button" onclick="resetToFactoryDefaults()" class="btn-studio btn-reset">
                ↺ Reset Preset
            </button>
            <button type="button" onclick="publishAllChanges()" class="btn-studio btn-publish" id="btn-save-publish">
                ✓ Save & Publish Live
            </button>
            <a href="/" target="_blank" class="btn-studio btn-reset" style="text-decoration: none;">
                ↗ Open Storefront
            </a>
        </div>
    </header>

    <!-- Studio Body -->
    <div class="studio-body">
        <!-- Left Sidebar -->
        <aside class="studio-sidebar">
            <div class="tabs-header">
                <button type="button" class="tab-btn active" onclick="switchTab('tab-presets')">Themes</button>
                <button type="button" class="tab-btn" onclick="switchTab('tab-styles')">Styling</button>
                <button type="button" class="tab-btn" onclick="switchTab('tab-sections')">Sections</button>
                <button type="button" class="tab-btn" onclick="switchTab('tab-settings')">Settings</button>
            </div>

            <div class="tab-content-container">
                <!-- Tab 1: Theme Presets -->
                <div id="tab-presets" class="tab-pane active">
                    <h3 style="font-size: 1.1rem; margin-bottom: 0.5rem;">Theme Presets</h3>
                    <p style="font-size: 0.85rem; color: var(--studio-muted); margin-bottom: 1.25rem;">Select any pre-engineered theme preset with 1-click:</p>

                    @foreach($themes as $th)
                        <div class="theme-card {{ $th->is_active ? 'active' : '' }}" onclick="selectThemePreset('{{ $th->slug }}')">
                            <div class="theme-card-header">
                                <span class="theme-card-title">{{ $th->name }}</span>
                                @if($th->is_active)
                                    <span style="color: var(--studio-green); font-size: 0.8rem; font-weight: 700;">● Active</span>
                                @endif
                            </div>
                            <p style="font-size: 0.8rem; color: var(--studio-muted); line-height: 1.4;">{{ $th->description }}</p>
                            <div class="theme-palette">
                                <div class="palette-swatch" style="background: {{ $th->bg_base }};" title="Base Background"></div>
                                <div class="palette-swatch" style="background: {{ $th->bg_card }};" title="Card Surface"></div>
                                <div class="palette-swatch" style="background: {{ $th->primary_color }};" title="Primary Accent"></div>
                                <div class="palette-swatch" style="background: {{ $th->accent_color }};" title="Secondary Accent"></div>
                            </div>
                        </div>
                    @endforeach
                </div>

                <!-- Tab 2: Live Styling Tokens -->
                <div id="tab-styles" class="tab-pane">
                    <h3 style="font-size: 1.1rem; margin-bottom: 0.5rem;">Visual Tokens</h3>
                    <p style="font-size: 0.85rem; color: var(--studio-muted); margin-bottom: 1.25rem;">Fine-tune your brand palette in real time without code:</p>

                    <div class="color-row">
                        <span>Primary Brand Accent</span>
                        <input type="color" id="input-primary-color" value="{{ $activeTheme->primary_color }}" class="color-input" onchange="previewStyleChange()">
                    </div>
                    <div class="color-row">
                        <span>Secondary Accent (Success)</span>
                        <input type="color" id="input-accent-color" value="{{ $activeTheme->accent_color }}" class="color-input" onchange="previewStyleChange()">
                    </div>
                    <div class="color-row">
                        <span>Base Background</span>
                        <input type="color" id="input-bg-base" value="{{ $activeTheme->bg_base }}" class="color-input" onchange="previewStyleChange()">
                    </div>
                    <div class="color-row">
                        <span>Card Surfaces</span>
                        <input type="color" id="input-bg-card" value="{{ $activeTheme->bg_card }}" class="color-input" onchange="previewStyleChange()">
                    </div>
                    <div class="color-row">
                        <span>Primary Text</span>
                        <input type="color" id="input-text-main" value="{{ $activeTheme->text_main }}" class="color-input" onchange="previewStyleChange()">
                    </div>

                    <div class="control-group" style="margin-top: 1.25rem;">
                        <label class="control-label">Corner Border Radius</label>
                        <select id="input-border-radius" class="form-control" onchange="previewStyleChange()">
                            <option value="4px" {{ $activeTheme->border_radius === '4px' ? 'selected' : '' }}>Sharp (4px)</option>
                            <option value="8px" {{ $activeTheme->border_radius === '8px' ? 'selected' : '' }}>Modern (8px)</option>
                            <option value="12px" {{ $activeTheme->border_radius === '12px' ? 'selected' : '' }}>Rounded (12px)</option>
                            <option value="18px" {{ $activeTheme->border_radius === '18px' ? 'selected' : '' }}>Soft Curved (18px)</option>
                            <option value="9999px" {{ $activeTheme->border_radius === '9999px' ? 'selected' : '' }}>Pill Rounded (9999px)</option>
                        </select>
                    </div>

                    <div class="control-group">
                        <label class="control-label">Typography Font Family</label>
                        <select id="input-font-family" class="form-control" onchange="previewStyleChange()">
                            <option value="'Plus Jakarta Sans', sans-serif" {{ str_contains($activeTheme->font_family, 'Plus Jakarta Sans') ? 'selected' : '' }}>Plus Jakarta Sans (Default)</option>
                            <option value="'Inter', sans-serif" {{ str_contains($activeTheme->font_family, 'Inter') ? 'selected' : '' }}>Inter (Clean Tech)</option>
                            <option value="'Outfit', sans-serif" {{ str_contains($activeTheme->font_family, 'Outfit') ? 'selected' : '' }}>Outfit (Futuristic Display)</option>
                            <option value="'Fira Code', monospace" {{ str_contains($activeTheme->font_family, 'Fira Code') ? 'selected' : '' }}>Fira Code (Developer Vibe)</option>
                        </select>
                    </div>
                </div>

                <!-- Tab 3: Dynamic Page Sections -->
                <div id="tab-sections" class="tab-pane">
                    <h3 style="font-size: 1.1rem; margin-bottom: 0.5rem;">Home Page Sections</h3>
                    <p style="font-size: 0.85rem; color: var(--studio-muted); margin-bottom: 1.25rem;">Toggle visibility or edit section headlines without writing code:</p>

                    <div id="sections-list">
                        @foreach($sections as $sec)
                            <div class="section-item-card" data-section-id="{{ $sec->id }}">
                                <div class="section-item-header">
                                    <span class="section-type-badge">{{ strtoupper($sec->section_type) }}</span>
                                    <label style="display: flex; align-items: center; gap: 0.4rem; font-size: 0.85rem; cursor: pointer;">
                                        <input type="checkbox" class="section-toggle" {{ $sec->is_active ? 'checked' : '' }} onchange="toggleSectionActive({{ $sec->id }}, this.checked)">
                                        <span>Show</span>
                                    </label>
                                </div>
                                <div class="control-group" style="margin-bottom: 0.75rem;">
                                    <label style="font-size: 0.75rem; color: var(--studio-muted);">Title</label>
                                    <input type="text" class="form-control section-title-input" value="{{ $sec->title }}" oninput="updateSectionLive({{ $sec->id }})">
                                </div>
                                <div class="control-group" style="margin-bottom: 0;">
                                    <label style="font-size: 0.75rem; color: var(--studio-muted);">Subtitle</label>
                                    <input type="text" class="form-control section-subtitle-input" value="{{ $sec->subtitle }}" oninput="updateSectionLive({{ $sec->id }})">
                                </div>
                            </div>
                        @endforeach
                    </div>
                </div>

                <!-- Tab 4: Store Settings -->
                <div id="tab-settings" class="tab-pane">
                    <h3 style="font-size: 1.1rem; margin-bottom: 0.5rem;">Storefront Settings</h3>
                    <p style="font-size: 0.85rem; color: var(--studio-muted); margin-bottom: 1.25rem;">Configure global store headers and announcements:</p>

                    <div class="control-group">
                        <label class="control-label">Store Brand Name</label>
                        <input type="text" id="setting-store-name" value="{{ $settings['store_name'] ?? 'Hyperion Enterprise Store' }}" class="form-control">
                    </div>

                    <div class="control-group">
                        <label class="control-label">Announcement Bar Active</label>
                        <select id="setting-announcement-enabled" class="form-control">
                            <option value="true" {{ ($settings['announcement_enabled'] ?? 'true') === 'true' ? 'selected' : '' }}>Enabled</option>
                            <option value="false" {{ ($settings['announcement_enabled'] ?? 'true') === 'false' ? 'selected' : '' }}>Disabled</option>
                        </select>
                    </div>

                    <div class="control-group">
                        <label class="control-label">Announcement Message</label>
                        <input type="text" id="setting-announcement-text" value="{{ $settings['announcement_text'] ?? '' }}" class="form-control">
                    </div>

                    <div class="control-group">
                        <label class="control-label">Announcement Link</label>
                        <input type="text" id="setting-announcement-link" value="{{ $settings['announcement_link'] ?? '' }}" class="form-control">
                    </div>
                </div>
            </div>
        </aside>

        <!-- Right Live Preview -->
        <section class="studio-preview-area">
            <div class="preview-toolbar">
                <div class="viewport-switchers">
                    <button type="button" class="viewport-btn active" onclick="setViewport('100%')">🖥️ Desktop</button>
                    <button type="button" class="viewport-btn" onclick="setViewport('768px')">📱 Tablet</button>
                    <button type="button" class="viewport-btn" onclick="setViewport('390px')">📱 Mobile</button>
                </div>
                <div style="font-size: 0.8rem; color: var(--studio-muted);">
                    🔗 Target: <code>http://127.0.0.1:8001/</code>
                </div>
            </div>
            <div class="preview-frame-container">
                <iframe id="preview-iframe" class="preview-iframe" src="/"></iframe>
            </div>
        </section>
    </div>

    <div id="toast" class="toast-notification">✓ Theme styles published successfully!</div>

    <script>
    function switchTab(tabId) {
        document.querySelectorAll('.tab-btn').forEach(btn => btn.classList.remove('active'));
        document.querySelectorAll('.tab-pane').forEach(pane => pane.classList.remove('active'));
        
        event.target.classList.add('active');
        document.getElementById(tabId).classList.add('active');
    }

    function setViewport(width) {
        document.querySelectorAll('.viewport-btn').forEach(btn => btn.classList.remove('active'));
        event.target.classList.add('active');
        document.getElementById('preview-iframe').style.width = width;
    }

    function previewStyleChange() {
        const primary = document.getElementById('input-primary-color').value;
        const accent = document.getElementById('input-accent-color').value;
        const bgBase = document.getElementById('input-bg-base').value;
        const bgCard = document.getElementById('input-bg-card').value;
        const textMain = document.getElementById('input-text-main').value;
        const radius = document.getElementById('input-border-radius').value;
        const font = document.getElementById('input-font-family').value;

        const iframe = document.getElementById('preview-iframe');
        if (iframe && iframe.contentDocument) {
            const root = iframe.contentDocument.documentElement;
            root.style.setProperty('--accent', primary);
            root.style.setProperty('--accent-green', accent);
            root.style.setProperty('--bg-base', bgBase);
            root.style.setProperty('--bg-card', bgCard);
            root.style.setProperty('--text-main', textMain);
            root.style.setProperty('--theme-radius', radius);
            root.style.setProperty('--theme-font', font);
            iframe.contentDocument.body.style.fontFamily = font;
            iframe.contentDocument.body.style.backgroundColor = bgBase;
            iframe.contentDocument.body.style.color = textMain;
        }
    }

    async function selectThemePreset(slug) {
        try {
            const res = await fetch('/api/v1/customizer/theme/activate', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    'X-CSRF-TOKEN': '{{ csrf_token() }}',
                    'Accept': 'application/json'
                },
                body: JSON.stringify({ slug: slug })
            });
            const data = await res.json();
            if (data.success) {
                showToast(`Switched to preset: ${data.theme.name}`);
                // Update inputs
                document.getElementById('input-primary-color').value = data.theme.primary_color;
                document.getElementById('input-accent-color').value = data.theme.accent_color;
                document.getElementById('input-bg-base').value = data.theme.bg_base;
                document.getElementById('input-bg-card').value = data.theme.bg_card;
                document.getElementById('input-text-main').value = data.theme.text_main;
                document.getElementById('input-border-radius').value = data.theme.border_radius;
                
                // Reload iframe to reflect new preset
                document.getElementById('preview-iframe').src = '/?t=' + Date.now();
                
                // Update card active styles
                document.querySelectorAll('.theme-card').forEach(card => card.classList.remove('active'));
                event.currentTarget.classList.add('active');
            }
        } catch (e) {
            console.error(e);
        }
    }

    async function publishAllChanges() {
        const btn = document.getElementById('btn-save-publish');
        btn.innerText = '⏳ Publishing...';

        const stylePayload = {
            primary_color: document.getElementById('input-primary-color').value,
            accent_color: document.getElementById('input-accent-color').value,
            bg_base: document.getElementById('input-bg-base').value,
            bg_card: document.getElementById('input-bg-card').value,
            text_main: document.getElementById('input-text-main').value,
            border_radius: document.getElementById('input-border-radius').value,
            font_family: document.getElementById('input-font-family').value,
        };

        const settingsPayload = {
            store_name: document.getElementById('setting-store-name').value,
            announcement_enabled: document.getElementById('setting-announcement-enabled').value,
            announcement_text: document.getElementById('setting-announcement-text').value,
            announcement_link: document.getElementById('setting-announcement-link').value,
        };

        try {
            await fetch('/api/v1/customizer/theme/styles', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    'X-CSRF-TOKEN': '{{ csrf_token() }}',
                    'Accept': 'application/json'
                },
                body: JSON.stringify(stylePayload)
            });

            await fetch('/api/v1/customizer/settings', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    'X-CSRF-TOKEN': '{{ csrf_token() }}',
                    'Accept': 'application/json'
                },
                body: JSON.stringify(settingsPayload)
            });

            // Save any section edits
            const sectionCards = document.querySelectorAll('.section-item-card');
            for (const card of sectionCards) {
                const id = card.dataset.sectionId;
                const title = card.querySelector('.section-title-input').value;
                const subtitle = card.querySelector('.section-subtitle-input').value;
                const active = card.querySelector('.section-toggle').checked;

                await fetch(`/api/v1/customizer/sections/${id}`, {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json',
                        'X-CSRF-TOKEN': '{{ csrf_token() }}',
                        'Accept': 'application/json'
                    },
                    body: JSON.stringify({ title, subtitle, is_active: active })
                });
            }

            showToast('✓ All Theme & Page Changes Published!');
            document.getElementById('preview-iframe').src = '/?t=' + Date.now();
        } catch (e) {
            console.error(e);
            alert('Failed to publish changes: ' + e);
        } finally {
            btn.innerText = '✓ Save & Publish Live';
        }
    }

    async function toggleSectionActive(sectionId, isActive) {
        await fetch(`/api/v1/customizer/sections/${sectionId}`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'X-CSRF-TOKEN': '{{ csrf_token() }}',
                'Accept': 'application/json'
            },
            body: JSON.stringify({ is_active: isActive })
        });
        document.getElementById('preview-iframe').src = '/?t=' + Date.now();
    }

    async function updateSectionLive(sectionId) {
        // Debounce update if needed
    }

    async function resetToFactoryDefaults() {
        if (!confirm('Reset current theme to factory preset defaults?')) return;
        const res = await fetch('/api/v1/customizer/reset', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'X-CSRF-TOKEN': '{{ csrf_token() }}',
                'Accept': 'application/json'
            },
            body: JSON.stringify({ slug: 'hyperion-slate' })
        });
        const data = await res.json();
        if (data.success) {
            showToast('✓ Theme reset to factory defaults.');
            setTimeout(() => window.location.reload(), 600);
        }
    }

    function showToast(msg) {
        const toast = document.getElementById('toast');
        toast.innerText = msg;
        toast.style.display = 'block';
        setTimeout(() => { toast.style.display = 'none'; }, 3000);
    }
    </script>
</body>
</html>
