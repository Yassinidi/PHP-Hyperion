<?php
$s = str_repeat("abcdefghij", 10);
for ($i = 0; $i < 200; $i++) {
    $sub = substr($s, 5, 20);
}
echo "OK";
