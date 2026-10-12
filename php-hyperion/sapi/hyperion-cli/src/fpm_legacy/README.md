# AI Plan: FPM Legacy
**Goal**: Compatibility with traditional web servers (Nginx/Apache) via FastCGI.

## AI Implementation Steps:
1. **FastCGI Protocol**: Implement the FCGI binary protocol (parsing records and param streams).
2. **Process Manager**: Create a master process to manage worker pools.
3. **Request Mapping**: Map FCGI environment variables securely to PHP superglobals.
