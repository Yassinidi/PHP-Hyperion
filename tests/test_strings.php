<?php
    for ($i = 0; $i < 100; $i++) {
        $s = "test_string_" . $i . "_" . str_repeat("abcdefghij", 10);
        $upper = strtoupper($s);
        $sub = substr($upper, 5, 20);
        $replaced = str_replace("ABC", "XYZ", $sub);
        $parts = explode("_", $replaced);
    }
    echo "OK";
    