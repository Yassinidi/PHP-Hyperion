<?php
    for ($i = 0; $i < 20; $i++) {
        try {
            throw new Exception("test err " . $i);
        } catch (Exception $e) {
            $msg = $e->getMessage();
        }
    }
    echo "OK";
    