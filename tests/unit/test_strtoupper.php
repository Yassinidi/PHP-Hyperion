<?php
    $s = str_repeat("abcdefghij", 10);
    for ($i = 0; $i < 200; $i++) {
        $u = strtoupper($s);
    }
    echo "OK";
    