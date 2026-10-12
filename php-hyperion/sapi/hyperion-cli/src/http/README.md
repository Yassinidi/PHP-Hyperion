# AI Plan: Embedded HTTP Server
**Goal**: A built-in high-performance web server (like Go's `net/http`).

## AI Implementation Steps:
1. **TCP Listener**: Bind to a port using `runtime::io` non-blocking sockets.
2. **HTTP Parser**: Write a fast HTTP/1.1 request parser to handle headers and body.
3. **Superglobals Setup**: Populate `$_SERVER`, `$_GET`, `$_POST`, `$_COOKIE` per request.
4. **Worker Dispatch**: Spawn a lightweight `Fibre` for every incoming HTTP request to handle it asynchronously.
