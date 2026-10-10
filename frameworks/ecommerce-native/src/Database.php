<?php
declare(strict_types=1);

namespace EcommerceNative;

use PDO;

class Database {
    private static ?PDO $instance = null;

    public static function getConnection(): PDO {
        if (self::$instance === null) {
            $dbPath = dirname(__DIR__) . '/database/ecommerce.sqlite';
            $isNew = !file_exists($dbPath) || filesize($dbPath) === 0;

            self::$instance = new PDO("sqlite:{$dbPath}", null, null, [
                PDO::ATTR_ERRMODE => PDO::ERRMODE_EXCEPTION,
                PDO::ATTR_DEFAULT_FETCH_MODE => PDO::FETCH_ASSOC,
            ]);

            // Performance optimizations for SQLite
            self::$instance->exec("PRAGMA journal_mode = WAL;");
            self::$instance->exec("PRAGMA synchronous = NORMAL;");

            if ($isNew) {
                self::initSchema(self::$instance);
            }
        }
        return self::$instance;
    }

    public static function resetConnection(): void {
        self::$instance = null;
    }

    private static function initSchema(PDO $db): void {
        $db->exec("
            CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                email TEXT UNIQUE NOT NULL,
                password_hash TEXT NOT NULL,
                role TEXT NOT NULL CHECK(role IN ('admin', 'client')),
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS categories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                slug TEXT UNIQUE NOT NULL,
                description TEXT,
                icon TEXT,
                is_active INTEGER DEFAULT 1
            );

            CREATE TABLE IF NOT EXISTS products (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                category_id INTEGER NOT NULL,
                title TEXT NOT NULL,
                slug TEXT UNIQUE NOT NULL,
                sku TEXT UNIQUE NOT NULL,
                price REAL NOT NULL,
                stock INTEGER NOT NULL,
                description TEXT,
                image_url TEXT,
                is_active INTEGER DEFAULT 1,
                FOREIGN KEY (category_id) REFERENCES categories(id)
            );

            CREATE TABLE IF NOT EXISTS banners (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                subtitle TEXT,
                image_url TEXT,
                link_url TEXT,
                position TEXT DEFAULT 'hero',
                is_active INTEGER DEFAULT 1
            );

            CREATE TABLE IF NOT EXISTS orders (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL,
                total_amount REAL NOT NULL,
                status TEXT DEFAULT 'completed',
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

        // Seed users (Admin & Client)
        $adminPass = password_hash('admin123', PASSWORD_BCRYPT);
        $clientPass = password_hash('client123', PASSWORD_BCRYPT);

        $db->exec("
            INSERT OR IGNORE INTO users (name, email, password_hash, role) VALUES 
            ('Store Administrator', 'admin@store.com', '{$adminPass}', 'admin'),
            ('Jane Doe', 'client@store.com', '{$clientPass}', 'client');

            INSERT OR IGNORE INTO categories (name, slug, description, icon) VALUES
            ('Electronics', 'electronics', 'High-end gadgets and electronics', '⚡'),
            ('Apparel & Fashion', 'fashion', 'Designer clothes and accessories', '👗'),
            ('Home & Living', 'home-living', 'Furniture and interior decoration', '🛋️'),
            ('Books & Audio', 'books', 'Bestselling books and podcasts', '📚');

            INSERT OR IGNORE INTO banners (title, subtitle, image_url, link_url, position) VALUES
            ('Flash Sale: 40% Off Tech', 'Explore next-generation laptops and smartphones today.', '/img/banner-tech.jpg', '/api/products?category_id=1', 'hero'),
            ('Autumn Fashion Collection', 'Trendy apparel tailored for the modern aesthetic.', '/img/banner-fashion.jpg', '/api/products?category_id=2', 'hero'),
            ('Elevate Your Living Space', 'Premium minimalism for smart contemporary homes.', '/img/banner-home.jpg', '/api/products?category_id=3', 'hero');

            INSERT OR IGNORE INTO products (category_id, title, slug, sku, price, stock, description, image_url) VALUES
            (1, 'Hyperion Pro Laptop 16\"', 'hyperion-pro-laptop', 'TECH-LAP-001', 1899.99, 45, 'Blazing fast workstation with M-class chip and 64GB RAM.', '/img/p-laptop.jpg'),
            (1, 'Quantum Wireless Earbuds', 'quantum-wireless-earbuds', 'TECH-EAR-002', 199.99, 120, 'Spatial audio with active noise cancellation.', '/img/p-earbuds.jpg'),
            (2, 'Merino Wool Minimalist Sweater', 'merino-wool-sweater', 'FASH-SWE-001', 129.50, 80, '100% fine Italian merino wool knitwear.', '/img/p-sweater.jpg'),
            (2, 'Italian Leather Crossbody Bag', 'leather-crossbody-bag', 'FASH-BAG-002', 249.00, 35, 'Handcrafted full-grain calfskin leather.', '/img/p-bag.jpg'),
            (3, 'Ergonomic Mesh Task Chair', 'ergonomic-task-chair', 'HOME-CHR-001', 450.00, 25, 'Engineered lumbar support with breathable mesh.', '/img/p-chair.jpg'),
            (3, 'Ceramic Minimalist Table Lamp', 'ceramic-table-lamp', 'HOME-LMP-002', 85.00, 60, 'Warm diffused ambient lighting for workspaces.', '/img/p-lamp.jpg'),
            (4, 'Designing Data-Intensive Applications', 'ddia-book', 'BOOK-DAT-001', 49.99, 150, 'The definitive guide to distributed architecture.', '/img/p-book1.jpg'),
            (4, 'Clean Architecture in Modern Systems', 'clean-arch-book', 'BOOK-ARC-002', 44.99, 90, 'Software patterns and enterprise domain modeling.', '/img/p-book2.jpg');

            INSERT OR IGNORE INTO orders (user_id, total_amount, status) VALUES
            (2, 2099.98, 'completed');

            INSERT OR IGNORE INTO order_items (order_id, product_id, quantity, unit_price) VALUES
            (1, 1, 1, 1899.99),
            (1, 2, 1, 199.99);
        ");
    }
}
