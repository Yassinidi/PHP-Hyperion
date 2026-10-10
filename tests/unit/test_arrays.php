<?php
    $data = [];
    for ($i = 0; $i < 200; $i++) {
        $data[] = ["id" => $i, "val" => "test"];
    }
    $keys = array_keys($data);
    $vals = array_values($data);
    echo "OK";
    