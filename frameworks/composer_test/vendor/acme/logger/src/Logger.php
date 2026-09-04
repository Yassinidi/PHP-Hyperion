<?php
namespace Acme\Logger;

class Logger {
    public function log(string $msg): string {
        return "[LOG] " . $msg;
    }
}
