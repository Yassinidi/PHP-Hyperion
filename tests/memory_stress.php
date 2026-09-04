<?php
// Heavy memory allocation test per request

$req_count = 0;

$data = [];
for ($i = 0; $i < 500; $i++) {
    $s = "test_string_" . $i . "_" . str_repeat("abcdefghij", 20);
    $upper = strtoupper($s);
    $sub = substr($upper, 10, 50);
    $replaced = str_replace("ABC", "XYZ", $sub);
    $parts = explode("_", $replaced);
    $data[] = [
        'id' => $i,
        'str' => $replaced,
        'parts' => $parts,
        'date' => date('Y-m-d H:i:s'),
    ];
}

$json = json_encode($data);
$decoded = json_decode($json, true);

// Regex stress
preg_match_all('/XYZ[A-Z0-9]+/', $json, $matches);

try {
    throw new Exception("Stress test exception for stack and message tracking");
} catch (Exception $e) {
    $msg = $e->getMessage();
}

echo "OK, items=" . count($decoded) . ", len=" . strlen($json);
