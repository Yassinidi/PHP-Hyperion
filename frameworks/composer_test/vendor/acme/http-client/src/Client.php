<?php
namespace Acme\HttpClient;

use Acme\Logger\Logger;

class Client {
    private Logger $logger;

    public function __construct(Logger $logger) {
        $this->logger = $logger;
    }

    public function get(string $url): string {
        return $this->logger->log("GET " . $url . " => 200 OK");
    }
}
