# AI Plan: Native Extensions Support
**Goal**: Support critical PHP extensions without completely rewriting them.

## AI Implementation Steps:
1. **PDO Header Mapping**: Mock Zend Engine C headers so PDO and DB drivers compile successfully against PHP-H.
2. **Rust Alternatives**: Where C compatibility is too slow, rewrite extensions (like cURL or OpenSSL) purely in Rust and expose them natively to the engine.
