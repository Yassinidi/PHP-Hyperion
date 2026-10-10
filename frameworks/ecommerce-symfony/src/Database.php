<?php
declare(strict_types=1);

namespace App;

use PDO;

class Database {
    private static ?PDO $pdo = null;

    public static function get(): PDO {
        if (self::$pdo === null) {
            $dbPath = __DIR__ . '/../storage/ecommerce_symfony.sqlite';
            $dir = dirname($dbPath);
            if (!is_dir($dir)) {
                mkdir($dir, 0777, true);
            }
            $exists = file_exists($dbPath);
            self::$pdo = new PDO('sqlite:' . $dbPath);
            self::$pdo->setAttribute(PDO::ATTR_ERRMODE, PDO::ERRMODE_EXCEPTION);
            self::$pdo->setAttribute(PDO::ATTR_DEFAULT_FETCH_MODE, PDO::FETCH_ASSOC);

            if (!$exists || filesize($dbPath) === 0) {
                self::migrateAndSeed(self::$pdo);
            }
        }
        return self::$pdo;
    }

    private static function migrateAndSeed(PDO $pdo): void {
        $pdo->exec("
            CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                email TEXT NOT NULL UNIQUE,
                password TEXT NOT NULL,
                role TEXT NOT NULL DEFAULT 'client',
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS categories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                slug TEXT NOT NULL UNIQUE,
                icon TEXT NOT NULL,
                display_order INTEGER DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS banners (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                subtitle TEXT NOT NULL,
                image_url TEXT NOT NULL,
                badge TEXT NOT NULL,
                is_active INTEGER DEFAULT 1
            );

            CREATE TABLE IF NOT EXISTS products (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                category_id INTEGER NOT NULL,
                name TEXT NOT NULL,
                slug TEXT NOT NULL,
                price REAL NOT NULL,
                stock INTEGER NOT NULL DEFAULT 10,
                rating REAL DEFAULT 4.8,
                image_url TEXT NOT NULL,
                FOREIGN KEY (category_id) REFERENCES categories(id)
            );

            CREATE TABLE IF NOT EXISTS orders (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL,
                total REAL NOT NULL,
                status TEXT NOT NULL DEFAULT 'completed',
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (user_id) REFERENCES users(id)
            );

            CREATE TABLE IF NOT EXISTS order_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                order_id INTEGER NOT NULL,
                product_id INTEGER NOT NULL,
                quantity INTEGER NOT NULL,
                unit_price REAL NOT NULL,
                FOREIGN KEY (order_id) REFERENCES orders(id),
                FOREIGN KEY (product_id) REFERENCES products(id)
            );
        ");

        $hashAdmin = password_hash('Admin@Hyperion2026', PASSWORD_BCRYPT);
        $hashClient = password_hash('Client@Hyperion2026', PASSWORD_BCRYPT);
        $pdo->exec("
            INSERT OR IGNORE INTO users (id, name, email, password, role) VALUES
            (1, 'Admin Hyperion', 'admin@hyperion.store', '{$hashAdmin}', 'admin'),
            (2, 'Sarah Connor', 'sarah@client.store', '{$hashClient}', 'client');

            INSERT OR IGNORE INTO categories (id, name, slug, icon, display_order) VALUES
            (1, 'High-End Laptops', 'laptops', '💻', 1),
            (2, 'Flagship Smartphones', 'smartphones', '📱', 2),
            (3, 'Wireless Audio', 'audio', '🎧', 3),
            (4, 'Wearable Tech', 'wearables', '⌚', 4);

            INSERT OR IGNORE INTO banners (id, title, subtitle, image_url, badge, is_active) VALUES
            (1, 'Symfony 7 Enterprise Speed', 'Full HttpKernel with compiled container pipeline', 'https://images.unsplash.com/photo-1519389950473-47ba0277781c?w=1200&q=80', '⚡ SYMFONY 7', 1),
            (2, 'PHP-Hyperion 60k+ QPS', 'Zero boot latency persistent event-loop', 'https://images.unsplash.com/photo-1550745165-9bc0b252726f?w=1200&q=80', '🚀 SUB-MS LATENCY', 1),
            (3, 'Next-Gen Mobile Workstations', 'Up to 24-core CPUs with 64GB Unified RAM', 'https://images.unsplash.com/photo-1517336714731-489689fd1ca8?w=1200&q=80', '🔥 HOT DEALS', 1);

            INSERT OR IGNORE INTO products (id, category_id, name, slug, price, stock, rating, image_url) VALUES
            (1, 1, 'Hyperion ProBook 16 M4 Max', 'hyperion-probook-16', 3499.99, 15, 4.95, 'https://images.unsplash.com/photo-1517336714731-489689fd1ca8?w=600&q=80'),
            (2, 1, 'Slim UltraAir 14 Featherlight', 'slim-ultraair-14', 1299.99, 28, 4.88, 'https://images.unsplash.com/photo-1541807084-5c52b6b3adef?w=600&q=80'),
            (3, 2, 'CyberPhone Quantum Fold 5', 'cyberphone-quantum-fold-5', 1899.00, 12, 4.90, 'https://images.unsplash.com/photo-1592750475338-74b7b21085ab?w=600&q=80'),
            (4, 2, 'Nexus Nova Titanium 6.7', 'nexus-nova-titanium', 1099.00, 35, 4.82, 'https://images.unsplash.com/photo-1511707171634-5f897ff02560?w=600&q=80'),
            (5, 3, 'AcousticPure Noise-Canceling ANC', 'acousticpure-anc', 399.99, 45, 4.92, 'https://images.unsplash.com/photo-1505740420928-5e560c06d30e?w=600&q=80'),
            (6, 3, 'StudioLink Wireless Earbuds Pro', 'studiolink-earbuds-pro', 249.50, 60, 4.79, 'https://images.unsplash.com/photo-1590658268037-6bf12165a8df?w=600&q=80'),
            (7, 4, 'Chronos Elite Sapphire Watch', 'chronos-elite-watch', 699.00, 20, 4.86, 'https://images.unsplash.com/photo-1523275335684-37898b6baf30?w=600&q=80'),
            (8, 4, 'PulseTrack Active Smart Band', 'pulsetrack-active-band', 149.99, 80, 4.75, 'https://images.unsplash.com/photo-1575311373937-040b8e1fd5b6?w=600&q=80');

            INSERT OR IGNORE INTO orders (id, user_id, total, status) VALUES
            (1, 2, 3499.99, 'completed'),
            (2, 2, 1899.00, 'completed');

            INSERT OR IGNORE INTO order_items (id, order_id, product_id, quantity, unit_price) VALUES
            (1, 1, 1, 1, 3499.99),
            (2, 2, 3, 1, 1899.00);
        ");
    }
}
