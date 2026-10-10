<?php
    $s = str_repeat("abcdefghij", 10);
    for ($i = 0; $i < 200; $i++) {
        $r = str_replace("abc", "xyz", $s);
    }
    echo "OK";
    