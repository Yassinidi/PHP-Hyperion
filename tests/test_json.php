<?php
    $data = [];
    for ($i = 0; $i < 100; $i++) {
        $data[] = ["id" => $i, "val" => "test"];
    }
    $j = json_encode($data);
    $d = json_decode($j, true);
    echo "OK";
    