<?php

$dbPath = __DIR__ . '/../frameworks/crm-laravel/database/database.sqlite';
if (file_exists($dbPath)) {
    unlink($dbPath);
}

$pdo = new PDO("sqlite:{$dbPath}");
$pdo->setAttribute(PDO::ATTR_ERRMODE, PDO::ERRMODE_EXCEPTION);

$pdo->exec("PRAGMA journal_mode = WAL;");
$pdo->exec("PRAGMA synchronous = NORMAL;");

$pdo->exec("
CREATE TABLE IF NOT EXISTS leads (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    first_name TEXT NOT NULL,
    last_name TEXT NOT NULL,
    company TEXT NOT NULL,
    email TEXT UNIQUE NOT NULL,
    phone TEXT,
    status TEXT DEFAULT 'new',
    score INTEGER DEFAULT 0,
    created_at DATETIME,
    updated_at DATETIME
);

CREATE TABLE IF NOT EXISTS contacts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    full_name TEXT NOT NULL,
    email TEXT UNIQUE NOT NULL,
    title TEXT,
    organization TEXT,
    created_at DATETIME,
    updated_at DATETIME
);

CREATE TABLE IF NOT EXISTS deals (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    lead_id INTEGER NOT NULL,
    title TEXT NOT NULL,
    value REAL NOT NULL,
    stage TEXT DEFAULT 'proposal',
    probability INTEGER DEFAULT 50,
    created_at DATETIME,
    updated_at DATETIME,
    FOREIGN KEY(lead_id) REFERENCES leads(id)
);

CREATE TABLE IF NOT EXISTS activity_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    action TEXT NOT NULL,
    description TEXT,
    user_id INTEGER,
    ip_address TEXT,
    created_at DATETIME,
    updated_at DATETIME
);
");

$stmtLead = $pdo->prepare("INSERT INTO leads (first_name, last_name, company, email, phone, status, score, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'))");
$stmtDeal = $pdo->prepare("INSERT INTO deals (lead_id, title, value, stage, probability, created_at, updated_at) VALUES (?, ?, ?, ?, ?, datetime('now'), datetime('now'))");

$companies = ['Acme Corp', 'Stark Industries', 'Wayne Enterprises', 'Cyberdyne', 'Initech', 'Umbrella Corp', 'Massive Dynamic', 'Hooli', 'Pied Piper', 'Gekko & Co'];
for ($i = 1; $i <= 50; $i++) {
    $fn = "LeadFirstName" . $i;
    $ln = "LeadLastName" . $i;
    $comp = $companies[$i % count($companies)];
    $email = "lead{$i}@" . strtolower(str_replace(' ', '', $comp)) . ".com";
    $phone = "+1-555-010" . sprintf("%02d", $i);
    $status = ($i % 3 === 0) ? 'qualified' : (($i % 5 === 0) ? 'contacted' : 'new');
    $score = $i * 5;
    $stmtLead->execute([$fn, $ln, $comp, $email, $phone, $status, $score]);
    
    if ($i % 2 === 0) {
        $dealVal = 5000.0 + ($i * 1250.0);
        $stage = ($i % 4 === 0) ? 'won' : 'negotiation';
        $stmtDeal->execute([$i, "Enterprise Contract {$i}", $dealVal, $stage, 80]);
    }
}

echo "✅ Seeded 50 leads and deals into: {$dbPath}\n";
