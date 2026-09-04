<?php
    $s = str_repeat("hello world 12345 foo bar ", 50);
    for ($i = 0; $i < 20; $i++) {
        preg_match_all("/[a-z]+[0-9]+/", $s, $m);
    }
    echo "OK";
    