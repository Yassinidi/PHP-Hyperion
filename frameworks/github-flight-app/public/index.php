<?php
require __DIR__ . '/../vendor/autoload.php';

Flight::route('/', function() {
    Flight::json(['status' => 'success', 'framework' => 'Flight PHP', 'message' => 'Hello from Flight REST microframework!']);
});

Flight::route('/users', function() {
    Flight::json([
        ['id' => 1, 'name' => 'Ada Lovelace', 'role' => 'Mathematician'],
        ['id' => 2, 'name' => 'Alan Turing', 'role' => 'Computer Scientist'],
        ['id' => 3, 'name' => 'Grace Hopper', 'role' => 'Systems Architect']
    ]);
});

Flight::start();
