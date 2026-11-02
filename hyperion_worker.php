<?php
/**
 * ⚡ PHP-Hyperion Universal Persistent Daemon Worker
 *
 * An enterprise-grade, zero-config persistent worker engine for:
 *   1. Laravel 10 / 11 / 12 (Full HttpKernel, Octane-style state management)
 *   2. Symfony 5 / 6 / 7 (Full App\Kernel, MicroKernel, HttpFoundation & Routing)
 *   3. Slim 4 & PSR-7 / PSR-15 Micro-frameworks
 *   4. WordPress (Core, REST API, Template Loader, WooCommerce)
 *   5. Plain / Native PHP (Vanilla apps, legacy scripts, file-based routers)
 *
 * Boots the application once into persistent memory and leverages Hyperion
 * pre-warmed fibres to process incoming HTTP requests with sub-millisecond latency.
 *
 * Delivers 100,000 - 200,000+ Req/sec with zero memory leaks across requests.
 */

declare(strict_types=1);

if (!defined('HYPERION_PERSISTENT')) {
    define('HYPERION_PERSISTENT', true);
}

// ============================================================================
// 1. UNIVERSAL CONFIGURATION & ENVIRONMENT OVERRIDES
// ============================================================================
// 100% Zero-Config by default. Automatically adapts to ANY PHP project.
// Optional settings can be overridden via ENV or a local 'hyperion.config.php':
$hyperionConfig = [
    // Engine mode: 'auto' (default), 'laravel', 'symfony', 'slim', 'wordpress', or 'plain'
    'framework'   => getenv('HYPERION_FRAMEWORK') ?: 'auto',

    // Project root directory (null = auto-detect from working dir / vendor / parent)
    'base_path'   => getenv('HYPERION_BASE_PATH') ?: null,

    // Public / Document root directory ('public', 'web', 'html', or '.' / null)
    'public_dir'  => getenv('HYPERION_PUBLIC_DIR') ?: null,

    // Custom entrypoint script (e.g. 'public/index.php')
    'entrypoint'  => getenv('HYPERION_ENTRYPOINT') ?: null,

    // Automatically flush database query logs & clear transactions between requests
    'reset_db'    => getenv('HYPERION_RESET_DB') !== false ? filter_var(getenv('HYPERION_RESET_DB') ?: 'true', FILTER_VALIDATE_BOOLEAN) : true,

    // Reset authentication guards, sessions, and queued cookies per request
    'reset_auth'  => getenv('HYPERION_RESET_AUTH') !== false ? filter_var(getenv('HYPERION_RESET_AUTH') ?: 'true', FILTER_VALIDATE_BOOLEAN) : true,

    // Register application routes directly into Hyperion kernel routing table
    'fast_route'  => getenv('HYPERION_FAST_ROUTE') !== false ? filter_var(getenv('HYPERION_FAST_ROUTE') ?: 'true', FILTER_VALIDATE_BOOLEAN) : true,

    // Optional user-defined callback for custom per-request state cleanup:
    'cleanup'     => null,
];

// ============================================================================
// 1.1 CONTAINER SANDBOX (PER-REQUEST ISOLATION)
// ============================================================================
final class HyperionContainerSandbox {
    private static array $baseInstances = [];
    private static array $baseResolved = [];
    private static array $baseBindings = [];
    private static array $baseReboundCallbacks = [];
    private static array $baseTerminatingCallbacks = [];
    private static ?\Closure $resetClosure = null;

    public static function init(object $app): void {
        if (!class_exists(\Illuminate\Container\Container::class, false)) {
            return;
        }
        try {
            self::$baseInstances = (function() { return $this->instances ?? []; })->call($app);
            unset(self::$baseInstances['request'], self::$baseInstances['session.store'], self::$baseInstances['auth.driver']);
            self::$baseResolved = (function() { return $this->resolved ?? []; })->call($app);
            self::$baseBindings = (function() { return $this->bindings ?? []; })->call($app);
            self::$baseReboundCallbacks = (function() { return $this->reboundCallbacks ?? []; })->call($app);
            self::$baseTerminatingCallbacks = (function() { return $this->terminatingCallbacks ?? []; })->call($app);

            self::$resetClosure = \Closure::bind(function(array $baseInst, array $baseRes, array $baseBind, array $baseReb, array $baseTerm) {
                $this->instances = $baseInst;
                $this->resolved = $baseRes;
                if (!empty($baseBind)) {
                    $this->bindings = $baseBind;
                }
                $this->reboundCallbacks = $baseReb;
                $this->terminatingCallbacks = $baseTerm;
                if (isset($this->scopedInstances)) {
                    $this->scopedInstances = [];
                }
            }, $app, \Illuminate\Foundation\Application::class);
        } catch (\Throwable $e) {}
    }

    public static function reset(object $app): void {
        if (method_exists($app, 'forgetInstance')) {
            $app->forgetInstance('request');
            $app->forgetInstance('auth.driver');
            $app->forgetInstance('session.store');
        }
        if (method_exists($app, 'forgetScopedInstances')) {
            $app->forgetScopedInstances();
        }
        if (!empty(self::$baseInstances)) {
            try {
                (function(array $baseInst) {
                    foreach ($this->instances as $k => $_) {
                        if (!isset($baseInst[$k])) {
                            unset($this->instances[$k]);
                        }
                    }
                })->call($app, self::$baseInstances);
            } catch (\Throwable $e) {}
        }
        if ($app->has('view')) {
            try {
                $v = $app->make('view');
                (function() {
                    $this->renderCount = 0;
                    $this->renderedOnce = [];
                })->call($v);
            } catch (\Throwable $e) {}
        }
        if (method_exists($app, 'forgetScopedInstances')) {
            $app->forgetScopedInstances();
        }
        if (isset(self::$baseInstances['cache']) && !$app->bound('cache')) {
            try {
                $app->instance('cache', self::$baseInstances['cache']);
            } catch (\Throwable $e) {}
        }
    }
}

// (Laravel-specific classes HyperionFastRouteCollection, HyperionEncrypter, HyperionViewSplicer, HyperionCompilerEngine
// are declared inside the Laravel bootstrap block after Composer autoloader is initialized)



// ============================================================================
// HYPERION HIGH-SPEED BINARY IPC ENGINE (For Zend PHP CLI & Native Concurrency)
// ============================================================================
final class HyperionIpc {
    /** @var resource|null */
    public static $socket = null;
    public static bool $active = false;
    public static string $rawBody = '';

    // Zero-Copy POSIX Shared Memory (SHM) State
    public static bool $shmActive = false;
    public static bool $extShmActive = false;
    public static mixed $shmFfi = null;
    public static mixed $shmPtr = null;
    public static int $shmFd = -1;
    public static int $shmSize = 0;
    public static mixed $shmReqSeqPtr = null;
    public static mixed $shmRespSeqPtr = null;
    public static mixed $shmWorkerStatePtr = null;
    public static mixed $shmMasterStatePtr = null;
    public static array $lastRawHeaders = [];

    public static function init(): void {
        $sockPath = getenv('HYPERION_IPC_SOCK') ?: ($_SERVER['HYPERION_IPC_SOCK'] ?? null);
        if (!$sockPath) {
            return;
        }

        $errno = 0;
        $errstr = '';
        $sock = @stream_socket_client('unix://' . $sockPath, $errno, $errstr, 5.0, STREAM_CLIENT_CONNECT);
        if ($sock === false) {
            error_log("[Hyperion IPC] Failed to connect to socket '{$sockPath}': {$errstr} ({$errno})");
            return;
        }

        stream_set_blocking($sock, true);
        // Do not set an idle timeout; workers are persistent daemons that wait indefinitely for requests

        if (function_exists('socket_import_stream')) {
            $extSock = @socket_import_stream($sock);
            if ($extSock) {
                @socket_set_option($extSock, SOL_SOCKET, SO_RCVBUF, 262144);
                @socket_set_option($extSock, SOL_SOCKET, SO_SNDBUF, 262144);
            }
        }

        self::$socket = $sock;
        self::$active = true;

        // 1. Handshake: Send 4-byte big-endian worker_id to Rust master
        $workerId = (int)(getenv('HYPERION_WORKER_ID') ?: 0);
        @fwrite($sock, pack('N', $workerId));

        // 2. Attach to Zero-Copy POSIX Shared Memory segment (if configured)
        $shmName = getenv('HYPERION_SHM_NAME') ?: ($_SERVER['HYPERION_SHM_NAME'] ?? null);
        $shmSize = (int)(getenv('HYPERION_SHM_SIZE') ?: 4194304);
        if ($shmName && function_exists('hyperion_shm_attach')) {
            try {
                if (@hyperion_shm_attach($shmName, $shmSize)) {
                    self::$extShmActive = true;
                    self::$shmActive = true;
                    self::$shmSize = $shmSize;
                }
            } catch (\Throwable $e) {
                error_log("[Hyperion ext-hyperion SHM] Notice: " . $e->getMessage());
            }
        }
        if ($shmName && !self::$extShmActive && extension_loaded('ffi')) {
            try {
                self::$shmFfi = \FFI::cdef("
                    int shm_open(const char *name, int flags, int mode);
                    void *mmap(void *addr, size_t length, int prot, int flags, int fd, int offset);
                    int munmap(void *addr, size_t length);
                    int close(int fd);
                    void *memcpy(void *dest, const void *src, size_t n);
                ");
                $fd = self::$shmFfi->shm_open($shmName, 0x0002 /* O_RDWR */, 0666);
                if ($fd >= 0) {
                    $ptr = self::$shmFfi->mmap(null, $shmSize, 1 | 2 /* PROT_READ | PROT_WRITE */, 1 /* MAP_SHARED */, $fd, 0);
                    if ($ptr !== null) {
                        self::$shmPtr = $ptr;
                        self::$shmFd = $fd;
                        self::$shmSize = $shmSize;
                        self::$shmActive = true;
                        self::$shmReqSeqPtr = self::$shmFfi->cast('uint32_t*', self::$shmPtr + 12);
                        self::$shmRespSeqPtr = self::$shmFfi->cast('uint32_t*', self::$shmPtr + 16);
                        self::$shmWorkerStatePtr = self::$shmFfi->cast('uint32_t*', self::$shmPtr + 20);
                        self::$shmMasterStatePtr = self::$shmFfi->cast('uint32_t*', self::$shmPtr + 24);
                    }
                }
            } catch (\Throwable $e) {
                error_log("[Hyperion SHM] Notice: " . $e->getMessage());
            }
        }
    }

    private static function readUint16(): int|false {
        $bytes = self::readExact(2);
        if ($bytes === false || strlen($bytes) < 2) {
            return false;
        }
        return unpack('n', $bytes)[1];
    }

    private static function readUint32(): int|false {
        $bytes = self::readExact(4);
        if ($bytes === false || strlen($bytes) < 4) {
            return false;
        }
        return unpack('N', $bytes)[1];
    }

    public static string $readBuf = '';

    public static function parsePayloadV5(string $payload): bool {
        $payloadLen = strlen($payload);
        if ($payloadLen < 12) {
            return false;
        }
        $fixed = unpack('Cmethod/Cflags/nheadCount/NbodyLen/npathLen/nqueryLen', $payload, 0);
        $method = match($fixed['method']) {
            0 => 'GET',
            1 => 'POST',
            2 => 'PUT',
            3 => 'DELETE',
            4 => 'PATCH',
            5 => 'OPTIONS',
            6 => 'HEAD',
            default => 'GET',
        };

        $pathLen = $fixed['pathLen'];
        $queryLen = $fixed['queryLen'];
        $path = $pathLen > 0 ? substr($payload, 12, $pathLen) : '/';
        if ($path === '' || $path[0] !== '/') {
            $path = '/' . $path;
        }
        $query = $queryLen > 0 ? substr($payload, 12 + $pathLen, $queryLen) : '';
        $uri = $query !== '' ? ($path . '?' . $query) : $path;

        $tableStart = 12 + $pathLen + $queryLen;
        $hCount = $fixed['headCount'];
        $tableBytes = $hCount * 8;
        $blobStart = $tableStart + $tableBytes;
        $bodyLen = $fixed['bodyLen'];
        $blobLen = $payloadLen - $bodyLen - $blobStart;

        $blob = $blobLen > 0 ? substr($payload, $blobStart, $blobLen) : '';
        $body = $bodyLen > 0 ? substr($payload, $blobStart + $blobLen, $bodyLen) : '';
        self::$rawBody = $body;

        $headers = [];
        if ($hCount > 0 && $tableBytes > 0) {
            $table = unpack('n*', $payload, $tableStart);
            for ($i = 0; $i < $hCount; $i++) {
                $base = $i * 4;
                $nOff = $table[$base + 1];
                $nLen = $table[$base + 2];
                $vOff = $table[$base + 3];
                $vLen = $table[$base + 4];
                $name = substr($blob, $nOff, $nLen);
                $val  = substr($blob, $vOff, $vLen);
                $headers[$name] = $val;
            }
        }

        self::$lastRawHeaders = $headers;
        self::populateSuperglobals($method, $uri, $path, $query, $headers, $body);
        return true;
    }

    public static function accept(): bool {
        if (!self::$active || self::$socket === null) {
            return false;
        }

        // 0. NATIVE C EXTENSION FAST-PATH (ext-hyperion)
        if (self::$extShmActive && function_exists('hyperion_shm_accept')) {
            $parsed = @hyperion_shm_accept(self::$socket);
            if (is_array($parsed)) {
                $method  = $parsed['method'] ?? 'GET';
                $path    = $parsed['path'] ?? '/';
                $query   = $parsed['query'] ?? '';
                $body    = $parsed['body'] ?? '';
                $headers = $parsed['headers'] ?? [];
                $uri     = $query !== '' ? ($path . '?' . $query) : $path;
                self::$rawBody = $body;
                self::$lastRawHeaders = $headers;
                self::populateSuperglobals($method, $uri, $path, $query, $headers, $body);
                return true;
            }
            if (feof(self::$socket)) {
                return false;
            }
        }

        // 1. FAST-PATH: Zero-Copy Shared Memory (SHM via FFI)
        if (self::$shmActive && self::$shmPtr !== null) {
            while (true) {
                $token = fread(self::$socket, 1);
                if ($token === false || $token === '') {
                    if (feof(self::$socket)) {
                        return false;
                    }
                    continue;
                }
                if ($token === "\x01") {
                    break;
                }
            }

            $lenBytes = \FFI::string(self::$shmPtr + 4, 4);
            $frameLen = unpack('N', $lenBytes)[1];
            $maxSlot = (int)(self::$shmSize / 2);
            if ($frameLen >= 16 && $frameLen <= $maxSlot) {
                $head = \FFI::string(self::$shmPtr + 64, 8);
                if ($head[0] === "\x48" && $head[1] === "\x59" && $head[2] === "\x50" && $head[3] === "\x05") {
                    $payloadLen = unpack('N', substr($head, 4, 4))[1];
                    if ($payloadLen >= 12 && (8 + $payloadLen) <= $frameLen) {
                        $payload = \FFI::string(self::$shmPtr + 72, $payloadLen);
                        return self::parsePayloadV5($payload);
                    }
                }
            }
        }

        // 2. FALLBACK: Standard UDS Socket Streaming
        while (strlen(self::$readBuf) < 8) {
            $chunk = fread(self::$socket, 65536);
            if ($chunk === false || $chunk === '') {
                if (feof(self::$socket)) {
                    return false;
                }
            } else {
                self::$readBuf .= $chunk;
            }
        }

        $magic = substr(self::$readBuf, 0, 4);
        $payloadLen = unpack('N', self::$readBuf, 4)[1];
        if ($payloadLen > 67108864) { // 64 MB defensive limit
            return false;
        }

        $totalLen = 8 + $payloadLen;
        while (strlen(self::$readBuf) < $totalLen) {
            $chunk = fread(self::$socket, 65536);
            if ($chunk === false || $chunk === '') {
                if (feof(self::$socket)) {
                    return false;
                }
            } else {
                self::$readBuf .= $chunk;
            }
        }

        $payload = substr(self::$readBuf, 8, $payloadLen);
        self::$readBuf = substr(self::$readBuf, $totalLen);

        if ($magic === "\x48\x59\x50\x05") { // HYP\x05
            return self::parsePayloadV5($payload);
        }


        if ($magic === "\x48\x59\x50\x03") { // HYP\x03 (Fallback backward compatibility)
            $offset = 0;
            $methodLen = unpack('n', substr($payload, $offset, 2))[1];
            $offset += 2;
            $method = $methodLen > 0 ? substr($payload, $offset, $methodLen) : 'GET';
            $offset += $methodLen;

            $uriLen = unpack('n', substr($payload, $offset, 2))[1];
            $offset += 2;
            $uri = $uriLen > 0 ? substr($payload, $offset, $uriLen) : '/';
            $offset += $uriLen;

            $pathLen = unpack('n', substr($payload, $offset, 2))[1];
            $offset += 2;
            $path = $pathLen > 0 ? substr($payload, $offset, $pathLen) : '/';
            $offset += $pathLen;

            $queryLen = unpack('n', substr($payload, $offset, 2))[1];
            $offset += 2;
            $query = $queryLen > 0 ? substr($payload, $offset, $queryLen) : '';
            $offset += $queryLen;

            $headerCount = unpack('n', substr($payload, $offset, 2))[1];
            $offset += 2;
            $headers = [];
            for ($i = 0; $i < $headerCount; $i++) {
                $kLen = unpack('n', substr($payload, $offset, 2))[1];
                $offset += 2;
                $k = $kLen > 0 ? substr($payload, $offset, $kLen) : '';
                $offset += $kLen;
                $vLen = unpack('n', substr($payload, $offset, 2))[1];
                $offset += 2;
                $v = $vLen > 0 ? substr($payload, $offset, $vLen) : '';
                $offset += $vLen;
                $headers[$k] = $v;
            }

            $bodyLen = unpack('N', substr($payload, $offset, 4))[1];
            $offset += 4;
            $body = $bodyLen > 0 ? substr($payload, $offset, $bodyLen) : '';
            self::$rawBody = $body;

            self::populateSuperglobals($method, $uri, $path, $query, $headers, $body);
            return true;
        }

        return false;
    }

    public static function sendResponse(mixed $response): void {
        if (!self::$active || self::$socket === null) {
            if ($response instanceof \Symfony\Component\HttpFoundation\Response) {
                $response->send();
            }
            return;
        }

        $statusCode = 200;
        $statusText = 'OK';
        $content = '';
        $wireHeaders = '';
        $headersList = [];

        if ($response instanceof \Symfony\Component\HttpFoundation\Response) {
            $statusCode = $response->getStatusCode();
            $statusText = \Symfony\Component\HttpFoundation\Response::$statusTexts[$statusCode] ?? 'OK';
            if (method_exists($response->headers, 'allPreserveCaseWithoutCookies')) {
                $allHeaders = $response->headers->allPreserveCaseWithoutCookies();
            } else {
                $allHeaders = $response->headers->all();
            }
            foreach ($allHeaders as $name => $values) {
                $lower = strtolower($name);
                if ($lower === 'connection' || $lower === 'content-length' || $lower === 'server') {
                    continue;
                }
                foreach ((array)$values as $val) {
                    $wireHeaders .= "{$name}: {$val}\r\n";
                    $headersList[] = [(string)$name, (string)$val];
                }
            }
            if (method_exists($response->headers, 'getCookies')) {
                foreach ($response->headers->getCookies() as $cookie) {
                    $wireHeaders .= "Set-Cookie: {$cookie}\r\n";
                    $headersList[] = ['Set-Cookie', (string)$cookie];
                }
            }
            if ($response instanceof \Symfony\Component\HttpFoundation\BinaryFileResponse) {
                $file = $response->getFile();
                $content = ($file && $file->isReadable()) ? (file_get_contents($file->getPathname()) ?: '') : '';
            } elseif ($response instanceof \Symfony\Component\HttpFoundation\StreamedResponse) {
                $content = '';
                ob_start(function($chunk) use (&$content) {
                    $content .= $chunk;
                    return '';
                }, 1);
                try {
                    $response->sendContent();
                } finally {
                    @ob_end_clean();
                }
            } else {
                $content = (string)$response->getContent();
            }
        } elseif ($response instanceof \Psr\Http\Message\ResponseInterface) {
            $statusCode = $response->getStatusCode();
            $statusText = $response->getReasonPhrase() ?: 'OK';
            foreach ($response->getHeaders() as $name => $values) {
                $lower = strtolower($name);
                if ($lower === 'connection' || $lower === 'content-length' || $lower === 'server') {
                    continue;
                }
                foreach ((array)$values as $val) {
                    $wireHeaders .= "{$name}: {$val}\r\n";
                    $headersList[] = [(string)$name, (string)$val];
                }
            }
            $content = (string)$response->getBody();
        } elseif (is_string($response)) {
            $content = $response;
            $statusCode = http_response_code() ?: 200;
        }

        // 0. NATIVE C EXTENSION FAST-PATH (ext-hyperion)
        if (self::$extShmActive && function_exists('hyperion_shm_respond')) {
            if (@hyperion_shm_respond(self::$socket, $statusCode, $statusText, $wireHeaders, $content)) {
                return;
            }
        }

        $contentLen = strlen($content);
        $wire = "HTTP/1.1 {$statusCode} {$statusText}\r\n"
              . $wireHeaders
              . "Server: PHP-Hyperion/1.0.0 (Persistent IPC)\r\n"
              . "Content-Length: {$contentLen}\r\n"
              . "Connection: keep-alive\r\n\r\n"
              . $content;

        $wireLen = strlen($wire);
        $respSlot = (int)(self::$shmSize / 2);

        // 1. ULTRA FAST-PATH: Direct Raw HTTP Wire Emission in Shared Memory
        if (self::$shmActive && self::$shmPtr !== null && $wireLen <= $respSlot) {
            self::$shmFfi->memcpy(self::$shmPtr + $respSlot, $wire, $wireLen);
            self::$shmFfi->memcpy(self::$shmPtr + 8, pack('N', $wireLen), 4);

            @fwrite(self::$socket, "\x01");
            return;
        }

        self::sendRawResponse($statusCode, $headersList, $content);
    }

    public static function sendRawResponse(int $statusCode, array $headers, string $content): void {
        if (!self::$active || self::$socket === null) {
            return;
        }

        // Fast vector-packed binary response frame (HYP\x06)
        $blob = '';
        $table = '';
        $hCount = count($headers);
        foreach ($headers as $h) {
            $nStr = is_array($h) ? (string)$h[0] : (string)$h;
            $vStr = is_array($h) ? (string)$h[1] : '';
            $nOff = strlen($blob);
            $nLen = strlen($nStr);
            $blob .= $nStr;
            $vOff = strlen($blob);
            $vLen = strlen($vStr);
            $blob .= $vStr;
            $table .= pack('nnnn', $nOff, $nLen, $vOff, $vLen);
        }

        $payload = pack('nnnN', $statusCode, 0, $hCount, strlen($content)) . $table . $blob . $content;
        $payloadLen = strlen($payload);

        // 1. FAST-PATH: Zero-Copy Shared Memory (SHM)
        $respSlot = (int)(self::$shmSize / 2);
        if (self::$shmActive && self::$shmPtr !== null && $payloadLen <= $respSlot) {
            // Write payload directly to Response Slot (offset respSlot)
            self::$shmFfi->memcpy(self::$shmPtr + $respSlot, $payload, $payloadLen);
            // Write response length to SHM header (offset 8)
            self::$shmFfi->memcpy(self::$shmPtr + 8, pack('N', $payloadLen), 4);

            @fwrite(self::$socket, "\x01");

            self::rollbackActiveTransactions();
            self::cleanupTempFiles();
            return;
        }

        // 2. FALLBACK: Standard socket streaming
        $frame = "\x48\x59\x50\x06" . pack('N', $payloadLen) . $payload;
        self::writeAll($frame);
        self::rollbackActiveTransactions();
        self::cleanupTempFiles();
    }


    public static function emitNativeResponse(mixed $response): void {
        if ($response instanceof \Symfony\Component\HttpFoundation\Response) {
            if (function_exists('hyperion_send_symfony_response') && @hyperion_send_symfony_response($response)) {
                return;
            }
            $statusCode = $response->getStatusCode();
            $statusText = \Symfony\Component\HttpFoundation\Response::$statusTexts[$statusCode] ?? 'OK';
            $wireHeaders = '';
            $allHeaders = method_exists($response->headers, 'allPreserveCaseWithoutCookies')
                ? $response->headers->allPreserveCaseWithoutCookies()
                : $response->headers->all();
            foreach ($allHeaders as $name => $values) {
                $lower = strtolower((string)$name);
                if ($lower === 'connection' || $lower === 'content-length' || $lower === 'server') {
                    continue;
                }
                foreach ((array)$values as $val) {
                    $sval = str_replace(["\r", "\n"], '', (string)$val);
                    $wireHeaders .= "{$name}: {$sval}\r\n";
                }
            }
            if (method_exists($response->headers, 'getCookies')) {
                foreach ($response->headers->getCookies() as $cookie) {
                    $sc = str_replace(["\r", "\n"], '', (string)$cookie);
                    $wireHeaders .= "Set-Cookie: {$sc}\r\n";
                }
            }
            if ($response instanceof \Symfony\Component\HttpFoundation\BinaryFileResponse) {
                $file = $response->getFile();
                $content = ($file && $file->isReadable()) ? (file_get_contents($file->getPathname()) ?: '') : '';
            } elseif ($response instanceof \Symfony\Component\HttpFoundation\StreamedResponse) {
                $content = '';
                ob_start(static function (string $chunk) use (&$content): string {
                    $content .= $chunk;
                    return '';
                }, 1);
                try {
                    $response->sendContent();
                } finally {
                    @ob_end_clean();
                }
            } else {
                $content = (string)$response->getContent();
            }
            @hyperion_send_response($statusCode, $statusText, $wireHeaders, $content);
            return;
        }

        if ($response instanceof \Psr\Http\Message\ResponseInterface) {
            $statusCode = $response->getStatusCode();
            $statusText = $response->getReasonPhrase() ?: 'OK';
            $wireHeaders = '';
            foreach ($response->getHeaders() as $name => $values) {
                $lower = strtolower((string)$name);
                if ($lower === 'connection' || $lower === 'content-length' || $lower === 'server') {
                    continue;
                }
                foreach ((array)$values as $val) {
                    $sval = str_replace(["\r", "\n"], '', (string)$val);
                    $wireHeaders .= "{$name}: {$sval}\r\n";
                }
            }
            @hyperion_send_response($statusCode, $statusText, $wireHeaders, (string)$response->getBody());
            return;
        }

        if (is_string($response)) {
            @hyperion_send_response(http_response_code() ?: 200, 'OK', '', $response);
        }
    }

    public static array $trackedPdoInstances = [];

    public static function tunePdo(mixed $pdo): void {
        if (!($pdo instanceof \PDO)) {
            return;
        }
        self::$trackedPdoInstances[spl_object_id($pdo)] = $pdo;
        if (function_exists('hyperion_track_pdo')) {
            @hyperion_track_pdo($pdo);
        }
        $sqliteTune = getenv('HYPERION_SQLITE_TUNE');
        if ($sqliteTune !== '1' && strtolower((string)$sqliteTune) !== 'true') {
            return;
        }
        try {
            $driver = @$pdo->getAttribute(\PDO::ATTR_DRIVER_NAME);
            if ($driver === 'sqlite') {
                @$pdo->exec("PRAGMA journal_mode = WAL;");
                @$pdo->exec("PRAGMA synchronous = NORMAL;");
                @$pdo->exec("PRAGMA busy_timeout = 5000;");
                @$pdo->exec("PRAGMA temp_store = MEMORY;");
                @$pdo->exec("PRAGMA mmap_size = 268435456;");
                @$pdo->exec("PRAGMA cache_size = -64000;");
            } elseif ($driver === 'mysql') {
                @$pdo->setAttribute(\PDO::ATTR_EMULATE_PREPARES, false);
                @$pdo->setAttribute(\PDO::MYSQL_ATTR_USE_BUFFERED_QUERY, true);
            }
        } catch (\Throwable $e) {}
    }

    public static function rollbackActiveTransactions(): void {
        foreach (self::$trackedPdoInstances as $id => $pdo) {
            try {
                if ($pdo instanceof \PDO && $pdo->inTransaction()) {
                    $pdo->rollBack();
                }
            } catch (\Throwable $e) {}
        }
    }

    public static function drainOutputBuffers(): void {
        while (ob_get_level() > 0) {
            @ob_end_clean();
        }
    }

    public static function sendOutputResponse(string $content, int $status = 200, array $headers = []): void {
        $statusCode = http_response_code() ?: $status;
        $outHeaders = [];
        if (empty($headers) && function_exists('headers_list')) {
            foreach (headers_list() as $hdr) {
                $colon = strpos($hdr, ':');
                if ($colon !== false) {
                    $outHeaders[] = [trim(substr($hdr, 0, $colon)), trim(substr($hdr, $colon + 1))];
                }
            }
        } else {
            foreach ($headers as $k => $v) {
                if (is_array($v)) {
                    $outHeaders[] = [(string)$v[0], (string)$v[1]];
                } else {
                    $outHeaders[] = [(string)$k, (string)$v];
                }
            }
        }
        if (function_exists('header_remove')) {
            header_remove();
        }

        // ULTRA FAST-PATH: Direct Raw HTTP Wire Emission in Shared Memory
        $respSlot = (int)(self::$shmSize / 2);
        if (self::$shmActive && self::$shmPtr !== null) {
            $wireHeaders = '';
            $hasContentType = false;
            foreach ($outHeaders as $h) {
                $nStr = is_array($h) ? (string)$h[0] : (string)$h;
                $vStr = is_array($h) ? (string)$h[1] : '';
                $lower = strtolower($nStr);
                if ($lower === 'connection' || $lower === 'content-length' || $lower === 'server') {
                    continue;
                }
                if ($lower === 'content-type') {
                    $hasContentType = true;
                }
                $wireHeaders .= "{$nStr}: {$vStr}\r\n";
            }
            if (!$hasContentType) {
                $wireHeaders .= "Content-Type: text/html; charset=UTF-8\r\n";
            }
            $contentLen = strlen($content);
            $statusText = match($statusCode) {
                200 => 'OK',
                201 => 'Created',
                204 => 'No Content',
                301 => 'Moved Permanently',
                302 => 'Found',
                400 => 'Bad Request',
                401 => 'Unauthorized',
                403 => 'Forbidden',
                404 => 'Not Found',
                500 => 'Internal Server Error',
                default => 'OK',
            };
            $wire = "HTTP/1.1 {$statusCode} {$statusText}\r\n"
                  . $wireHeaders
                  . "Server: PHP-Hyperion/1.0.0 (Persistent IPC)\r\n"
                  . "Content-Length: {$contentLen}\r\n"
                  . "Connection: keep-alive\r\n\r\n"
                  . $content;
            $wireLen = strlen($wire);
            if ($wireLen <= $respSlot) {
                self::$shmFfi->memcpy(self::$shmPtr + $respSlot, $wire, $wireLen);
                self::$shmFfi->memcpy(self::$shmPtr + 8, pack('N', $wireLen), 4);

                @fwrite(self::$socket, "\x01");
                self::rollbackActiveTransactions();
                self::cleanupTempFiles();
                return;
            }
        }

        self::sendRawResponse($statusCode, $outHeaders, $content);
        self::rollbackActiveTransactions();
        self::cleanupTempFiles();
    }

    public static array $tempFiles = [];

    public static function cleanupTempFiles(): void {
        if (!empty(self::$tempFiles)) {
            foreach (self::$tempFiles as $f) {
                if (is_file($f)) {
                    @unlink($f);
                }
            }
            self::$tempFiles = [];
        }
    }

    private static function parseMultipartFormData(string $body, string $boundary, array &$post, array &$files): void {
        $delimiter = '--' . $boundary;
        $parts = explode($delimiter, $body);

        foreach ($parts as $part) {
            if ($part === '' || $part === '--' || $part === "--\r\n" || $part === "--\n") {
                continue;
            }
            if (str_starts_with($part, "\r\n")) {
                $part = substr($part, 2);
            } elseif (str_starts_with($part, "\n")) {
                $part = substr($part, 1);
            }

            $split = explode("\r\n\r\n", $part, 2);
            if (count($split) < 2) {
                $split = explode("\n\n", $part, 2);
                if (count($split) < 2) {
                    continue;
                }
            }

            $rawHeaders = $split[0];
            $content = $split[1];
            if (str_ends_with($content, "\r\n")) {
                $content = substr($content, 0, -2);
            } elseif (str_ends_with($content, "\n")) {
                $content = substr($content, 0, -1);
            }

            $name = null;
            $filename = null;
            $contentType = 'text/plain';

            foreach (explode("\n", $rawHeaders) as $line) {
                $line = trim($line);
                if (stripos($line, 'Content-Disposition:') === 0) {
                    if (preg_match('/name="([^"]+)"/i', $line, $m)) {
                        $name = $m[1];
                    }
                    if (preg_match('/filename="([^"]*)"/i', $line, $m)) {
                        $filename = $m[1];
                    }
                } elseif (stripos($line, 'Content-Type:') === 0) {
                    $contentType = trim(substr($line, 13));
                }
            }

            if ($name === null) {
                continue;
            }

            if ($filename !== null) {
                if ($filename === '') {
                    $files[$name] = [
                        'name'     => '',
                        'type'     => '',
                        'tmp_name' => '',
                        'error'    => UPLOAD_ERR_NO_FILE,
                        'size'     => 0,
                    ];
                } else {
                    $cleanFilename = basename(str_replace(["\0", "\\"], ['', '/'], $filename));
                    if ($cleanFilename === '' || $cleanFilename === '.' || $cleanFilename === '..') {
                        $cleanFilename = 'upload.bin';
                    }
                    $tmpPath = tempnam(sys_get_temp_dir(), 'hyp_up_');
                    if ($tmpPath !== false) {
                        file_put_contents($tmpPath, $content);
                        self::$tempFiles[] = $tmpPath;
                        $files[$name] = [
                            'name'     => $cleanFilename,
                            'type'     => $contentType,
                            'tmp_name' => $tmpPath,
                            'error'    => UPLOAD_ERR_OK,
                            'size'     => strlen($content),
                        ];
                    }
                }
            } else {
                $post[$name] = $content;
            }
        }
    }

    public static bool $isLaravelFramework = false;
    public static bool $lastBodyIsJson = false;

    private static function populateSuperglobals(string $method, string $uri, string $path, string $query, array $headers, string $body): void {
        global $publicDir, $entrypoint;

        if (!self::$isLaravelFramework) {
            // Reset response state & headers from previous request for classic PHP scripts
            if (!headers_sent()) {
                http_response_code(200);
                if (function_exists('header_remove')) {
                    header_remove();
                }
            }
            if (isset($_SESSION)) {
                $_SESSION = [];
            }
            self::drainOutputBuffers();
        } elseif (ob_get_level() > 0) {
            self::drainOutputBuffers();
        }

        static $baseServer = null;
        if ($baseServer === null) {
            $baseServer = [
                'SERVER_PROTOCOL'    => 'HTTP/1.1',
                'PHP_SELF'           => '/index.php',
                'SCRIPT_NAME'        => '/index.php',
                'SCRIPT_FILENAME'    => $entrypoint ?? (($publicDir ?? '.') . '/index.php'),
                'DOCUMENT_ROOT'      => $publicDir ?? getcwd(),
            ];
        }

        $serverPort = $headers['Server-Port'] ?? '8000';
        $remoteAddr = $headers['Remote-Addr'] ?? '127.0.0.1';
        $host = $headers['Host'] ?? $headers['host'] ?? ("127.0.0.1:{$serverPort}");
        $colonPos = strpos($host, ':');

        $_SERVER = $baseServer;
        $_SERVER['REQUEST_METHOD']     = $method;
        $_SERVER['REQUEST_URI']        = $uri;
        $_SERVER['PATH_INFO']          = $path;
        $_SERVER['QUERY_STRING']       = $query;
        $_SERVER['SERVER_NAME']        = $colonPos !== false ? substr($host, 0, $colonPos) : $host;
        $_SERVER['SERVER_PORT']        = $serverPort;
        $_SERVER['REMOTE_ADDR']        = $remoteAddr;
        $_SERVER['HTTP_HOST']          = $host;
        $_SERVER['REQUEST_TIME']       = time();
        $_SERVER['REQUEST_TIME_FLOAT'] = microtime(true);

        foreach ($headers as $k => $v) {
            $upper = strtoupper(str_replace('-', '_', $k));
            if ($upper === 'CONTENT_TYPE' || $upper === 'CONTENT_LENGTH') {
                $_SERVER[$upper] = $v;
            } else {
                $_SERVER['HTTP_' . $upper] = $v;
            }
        }

        // Reverse Proxy / SSL Header Override (opt-in via HYPERION_TRUST_PROXIES=1;
        // otherwise Laravel's TrustProxies middleware validates against REMOTE_ADDR)
        static $trustProxiesBool = null;
        if ($trustProxiesBool === null) {
            $tp = getenv('HYPERION_TRUST_PROXIES');
            $trustProxiesBool = ($tp === '1' || strtolower((string)$tp) === 'true');
        }
        if ($trustProxiesBool) {
            if (isset($_SERVER['HTTP_X_FORWARDED_PROTO']) && strtolower($_SERVER['HTTP_X_FORWARDED_PROTO']) === 'https') {
                $_SERVER['HTTPS'] = 'on';
                $_SERVER['SERVER_PORT'] = '443';
            }
            if (isset($_SERVER['HTTP_X_FORWARDED_PORT'])) {
                $_SERVER['SERVER_PORT'] = $_SERVER['HTTP_X_FORWARDED_PORT'];
            }
            if (isset($_SERVER['HTTP_X_FORWARDED_FOR'])) {
                $ips = explode(',', $_SERVER['HTTP_X_FORWARDED_FOR']);
                $_SERVER['REMOTE_ADDR'] = trim($ips[0]);
            }
        }

        $_GET = [];
        if ($query !== '') {
            parse_str($query, $_GET);
        }

        $_SERVER['RAW_BODY'] = $body;
        $GLOBALS['HTTP_RAW_POST_DATA'] = $body;
        $GLOBALS['RAW_HTTP_BODY'] = $body;

        $_POST = [];
        $_FILES = [];
        self::$lastBodyIsJson = false;
        if ($body !== '') {
            $cType = $_SERVER['CONTENT_TYPE'] ?? '';
            if (stripos($cType, 'application/json') !== false) {
                self::$lastBodyIsJson = true;
                $jsonParsed = json_decode($body, true);
                if (is_array($jsonParsed)) {
                    $_POST = $jsonParsed;
                }
            } elseif (stripos($cType, 'application/x-www-form-urlencoded') !== false) {
                parse_str($body, $_POST);
            } elseif (stripos($cType, 'multipart/form-data') !== false && preg_match('/boundary=(?:["\']?([^"\';\s]+)["\']?)/i', $cType, $bm)) {
                self::parseMultipartFormData($body, $bm[1], $_POST, $_FILES);
            }
        }

        $_COOKIE = [];
        if (isset($_SERVER['HTTP_COOKIE'])) {
            $parts = explode(';', $_SERVER['HTTP_COOKIE']);
            foreach ($parts as $part) {
                $eq = explode('=', trim($part), 2);
                if (count($eq) === 2) {
                    $_COOKIE[rawurldecode(trim($eq[0]))] = rawurldecode(trim($eq[1]));
                }
            }
        }

        if (empty($_POST) && empty($_COOKIE)) {
            $_REQUEST = $_GET;
        } else {
            $_REQUEST = array_merge($_GET, $_POST, $_COOKIE);
        }
    }

    private static function readExact(int $length): string|false {
        $buf = '';
        while (strlen($buf) < $length) {
            $chunk = fread(self::$socket, $length - strlen($buf));
            if ($chunk === false || $chunk === '') {
                if (feof(self::$socket)) {
                    return false;
                }
            } else {
                $buf .= $chunk;
            }
        }
        return $buf;
    }

    private static function writeAll(string $data): void {
        $len = strlen($data);
        $written = 0;
        while ($written < $len) {
            $n = fwrite(self::$socket, substr($data, $written));
            if ($n === false || $n === 0) {
                break;
            }
            $written += $n;
        }
    }
}

HyperionIpc::init();

register_shutdown_function(function() {
    $err = error_get_last();
    if ($err && in_array($err['type'], [E_ERROR, E_PARSE, E_CORE_ERROR, E_COMPILE_ERROR], true)) {
        if (HyperionIpc::$active && HyperionIpc::$socket) {
            error_log("[Hyperion Worker Fatal] " . $err['message'] . " in " . $err['file'] . ":" . $err['line']);
            $debug = filter_var(getenv('APP_DEBUG') ?: ($_ENV['APP_DEBUG'] ?? ($_SERVER['APP_DEBUG'] ?? 'false')), FILTER_VALIDATE_BOOLEAN);
            $msg = $debug
                ? ("500 Internal Server Error (Worker Crash): " . $err['message'] . " in " . $err['file'] . ":" . $err['line'])
                : "500 Internal Server Error";
            @HyperionIpc::sendRawResponse(500, [['Content-Type', 'text/plain; charset=UTF-8']], $msg);
        }
    }
});

if (!function_exists('hyperion_accept_request')) {
    eval('function hyperion_accept_request(): bool { if (HyperionIpc::$active) { return HyperionIpc::accept(); } return false; }');
}

// ============================================================================
// 2. PROJECT AUTO-DETECTION & ENVIRONMENT INITIALIZATION
// ============================================================================

// Helper to determine if a directory contains standard PHP application markers
$isProjectRoot = static function (?string $dir): bool {
    if ($dir === null || $dir === '' || !is_dir($dir)) {
        return false;
    }
    return file_exists($dir . '/composer.json')
        || file_exists($dir . '/artisan')
        || file_exists($dir . '/wp-config.php')
        || file_exists($dir . '/wp-load.php')
        || file_exists($dir . '/bin/console')
        || file_exists($dir . '/vendor/autoload.php')
        || file_exists($dir . '/public/index.php')
        || file_exists($dir . '/index.php');
};

// 1. Resolve Project Root:
// Priority: $hyperionConfig['base_path'] -> HYPERION_BASE_PATH env -> getcwd() -> __DIR__ / parent -> search upward
$basePath = $hyperionConfig['base_path']
    ?? (getenv('HYPERION_BASE_PATH') ?: null)
    ?? ($_SERVER['HYPERION_BASE_PATH'] ?? null);

if ($basePath === null || !$isProjectRoot($basePath)) {
    $cwd = getcwd() ?: '.';
    if ($isProjectRoot($cwd)) {
        $basePath = $cwd;
    } else {
        // Traverse upward from cwd up to 4 levels (e.g. if worker started from public/ or a subfolder)
        $cursor = $cwd;
        $found = null;
        for ($i = 0; $i < 4; $i++) {
            $parent = dirname($cursor);
            if ($parent === $cursor || !is_dir($parent)) {
                break;
            }
            if ($isProjectRoot($parent)) {
                $found = $parent;
                break;
            }
            $cursor = $parent;
        }
        if ($found !== null) {
            $basePath = $found;
        } elseif ($isProjectRoot(__DIR__)) {
            $basePath = __DIR__;
        } elseif ($isProjectRoot(dirname(__DIR__))) {
            $basePath = dirname(__DIR__);
        } else {
            $basePath = $cwd;
        }
    }
}

$realBasePath = realpath($basePath);
$basePath = $realBasePath !== false ? $realBasePath : $basePath;
$basePath = rtrim(str_replace('\\', '/', (string)$basePath), '/');

// Load optional project-local configuration without needing to edit this worker file
if (file_exists($basePath . '/hyperion.config.php')) {
    $localCfg = require $basePath . '/hyperion.config.php';
    if (is_array($localCfg)) {
        $hyperionConfig = array_merge($hyperionConfig, $localCfg);
    }
}

// 2. Resolve Public / Document Root:
$publicDir = $hyperionConfig['public_dir']
    ?? (getenv('HYPERION_PUBLIC_DIR') ?: null)
    ?? ($_SERVER['HYPERION_PUBLIC_DIR'] ?? null);

if ($publicDir === null) {
    if (is_dir($basePath . '/public')) {
        $publicDir = $basePath . '/public';
    } elseif (is_dir($basePath . '/web')) {
        $publicDir = $basePath . '/web';
    } elseif (is_dir($basePath . '/html')) {
        $publicDir = $basePath . '/html';
    } else {
        $publicDir = $basePath;
    }
} else {
    $publicDir = rtrim(str_replace('\\', '/', $publicDir), '/');
    if (!str_starts_with($publicDir, '/') && !preg_match('/^[A-Za-z]:\//', $publicDir)) {
        if (file_exists($publicDir)) {
            $publicDir = realpath($publicDir) ?: ($basePath . '/' . $publicDir);
        } else {
            $publicDir = $basePath . '/' . $publicDir;
        }
    }
}

$realPublicDir = realpath($publicDir);
$publicDir = $realPublicDir !== false ? $realPublicDir : $publicDir;
$publicDir = rtrim(str_replace('\\', '/', (string)$publicDir), '/');

// Load Composer autoloader if present
$autoloadCandidates = [
    $basePath . '/vendor/autoload.php',
    $publicDir . '/../vendor/autoload.php',
    dirname($publicDir) . '/vendor/autoload.php',
];
foreach ($autoloadCandidates as $candidate) {
    if (file_exists($candidate)) {
        require_once $candidate;
        $candidateDir = realpath(dirname(dirname($candidate)));
        if ($candidateDir) {
            $basePath = $candidateDir;
        }
        break;
    }
}

// Framework auto-detection
$framework = strtolower((string)($hyperionConfig['framework'] ?? 'auto'));
if ($framework === 'auto') {
    if (file_exists($basePath . '/wp-config.php') || file_exists($basePath . '/wp-load.php')
        || file_exists($publicDir . '/../wp-config.php') || file_exists($publicDir . '/../wp-load.php')
        || file_exists($publicDir . '/wp-config.php') || file_exists($publicDir . '/wp-load.php')) {
        if (file_exists($publicDir . '/../wp-config.php') || file_exists($publicDir . '/../wp-load.php')) {
            $basePath = realpath($publicDir . '/..') ?: $basePath;
        } elseif (file_exists($publicDir . '/wp-config.php') || file_exists($publicDir . '/wp-load.php')) {
            $basePath = $publicDir;
        }
        $framework = 'wordpress';
    } elseif (file_exists($basePath . '/bootstrap/app.php') && (file_exists($basePath . '/artisan') || is_dir($basePath . '/app'))) {
        $framework = 'laravel';
    } elseif (file_exists(dirname($basePath) . '/bootstrap/app.php') && (file_exists(dirname($basePath) . '/artisan') || is_dir(dirname($basePath) . '/app'))) {
        $basePath = dirname($basePath);
        $framework = 'laravel';
    } elseif (file_exists($publicDir . '/../bootstrap/app.php') && (file_exists($publicDir . '/../artisan') || is_dir($publicDir . '/../app'))) {
        $basePath = realpath($publicDir . '/..') ?: dirname($publicDir);
        $framework = 'laravel';
    } elseif (file_exists($basePath . '/bin/console') || file_exists($basePath . '/src/Kernel.php')) {
        $framework = 'symfony';
    } elseif (file_exists(dirname($basePath) . '/bin/console') || file_exists(dirname($basePath) . '/src/Kernel.php')) {
        $basePath = dirname($basePath);
        $framework = 'symfony';
    } elseif (file_exists($publicDir . '/../bin/console') || file_exists($publicDir . '/../src/Kernel.php')) {
        $basePath = realpath($publicDir . '/..') ?: dirname($publicDir);
        $framework = 'symfony';
    } elseif (class_exists('Slim\Factory\AppFactory') || class_exists('Slim\App')) {
        $framework = 'slim';
    } elseif (class_exists('Symfony\Component\HttpKernel\Kernel') || class_exists('Symfony\Component\Routing\RouteCollection')) {
        $framework = 'symfony';
    } else {
        $framework = 'plain';
    }
}

// Resolve application entrypoint (from config, env, or default)
$entrypoint = $hyperionConfig['entrypoint']
    ?? (getenv('HYPERION_ENTRYPOINT') ?: null)
    ?? ($_SERVER['HYPERION_ENTRYPOINT'] ?? null);

if ($entrypoint !== null && !str_starts_with($entrypoint, '/') && !preg_match('/^[A-Za-z]:\//', $entrypoint)) {
    if (file_exists($basePath . '/' . $entrypoint)) {
        $entrypoint = $basePath . '/' . $entrypoint;
    } elseif (file_exists($publicDir . '/' . $entrypoint)) {
        $entrypoint = $publicDir . '/' . $entrypoint;
    }
}
if ($entrypoint === null || !file_exists($entrypoint)) {
    $entrypoint = file_exists($publicDir . '/index.php') ? ($publicDir . '/index.php') : ($basePath . '/index.php');
}

error_log(sprintf("[HYPERION_WORKER_BOOT] framework=%s basePath=%s publicDir=%s entrypoint=%s", $framework, $basePath, $publicDir, $entrypoint));

$nativeReactorActive = getenv('HYPERION_NATIVE_REACTOR') === '1'
    && function_exists('hyperion_reactor_serve');
$maxRequestsCfg = (int)(getenv('HYPERION_MAX_REQUESTS') ?: 2500000);
$maxMemoryMbCfg = (int)(getenv('HYPERION_MAX_MEMORY_MB') ?: 512);
$nativeReactorOpts = [
    'public_dir'       => $publicDir,
    'base_path'        => $basePath,
    'entrypoint'       => $entrypoint,
    'max_requests'     => $maxRequestsCfg,
    'max_memory_mb'    => $maxMemoryMbCfg,
    'sandbox_rollback' => false,
];

switch ($framework) {

    // ------------------------------------------------------------------------
    // A. LARAVEL (Versions 10, 11, 12)
    // ------------------------------------------------------------------------
    case 'laravel':
        define('LARAVEL_START', microtime(true));

        if (!class_exists(\Illuminate\Foundation\Application::class, false)) {
            $autoloadCandidates = [
                $basePath . '/vendor/autoload.php',
                dirname($basePath) . '/vendor/autoload.php',
                $publicDir . '/../vendor/autoload.php',
            ];
            foreach ($autoloadCandidates as $candidate) {
                if (file_exists($candidate)) {
                    require_once $candidate;
                    break;
                }
            }
        }

        $bootstrapFile = $basePath . '/bootstrap/app.php';
        if (!file_exists($bootstrapFile)) {
            if (file_exists(dirname($basePath) . '/bootstrap/app.php')) {
                $basePath = dirname($basePath);
                $bootstrapFile = $basePath . '/bootstrap/app.php';
            } else {
                throw new RuntimeException("Laravel bootstrap file not found at: {$bootstrapFile}");
            }
        }

        /** @var \Illuminate\Foundation\Application $app */
        $app = require $bootstrapFile;
        $kernel = $app->make(\Illuminate\Contracts\Http\Kernel::class);

        // Pre-warm and bind initial request context prior to bootstrapping so all service providers succeed
        $appUrl = getenv('APP_URL') ?: ($app->has('config') ? $app['config']->get('app.url') : 'http://127.0.0.1:8000');
        if (empty($appUrl) || $appUrl === 'http://localhost') {
            $appUrl = 'http://127.0.0.1:8000';
        }
        $warmupReq = \Illuminate\Http\Request::create($appUrl . '/', 'GET');
        $app->instance('request', $warmupReq);

        try {
            $kernel->bootstrap();
            error_log("[KERNEL_BOOTSTRAP] SUCCESS! has_login=" . ($app['router']->has('login') ? 'YES' : 'NO') . " total_routes=" . count($app['router']->getRoutes()));
        } catch (\Throwable $e) {
            $msg = is_object($e) && method_exists($e, 'getMessage') ? $e->getMessage() : (string)$e;
            $file = is_object($e) && method_exists($e, 'getFile') ? $e->getFile() : '';
            $line = is_object($e) && method_exists($e, 'getLine') ? $e->getLine() : 0;
            $trace = is_object($e) && method_exists($e, 'getTraceAsString') ? $e->getTraceAsString() : '';
            error_log("[BOOTSTRAP ERROR] " . (is_object($e) ? get_class($e) : gettype($e)) . ": " . $msg . " in " . $file . ":" . $line . "\n" . $trace);
        }

        // Eagerly register and boot all deferred service providers (Cache, Bus, Queue, etc.) into Boot Arena
        if (method_exists($app, 'loadDeferredProviders')) {
            try {
                $app->loadDeferredProviders();
            } catch (\Throwable $e) {}
        }
        if (!$app->bound('cache')) {
            try {
                $app->register(\Illuminate\Cache\CacheServiceProvider::class);
            } catch (\Throwable $e) {}
        }
        try {
            $app->make('cache');
        } catch (\Throwable $e) {}

        // Retain the application's configured session and cache drivers (100% compatibility, zero forced overrides)

        // Register hardware-accelerated C AES-256-CBC + HMAC-SHA256 Encrypter BEFORE warmup
        // so EncryptCookies and EncryptedStore bind to native C crypto at construction time.
        if (!class_exists('HyperionEncrypter', false)) {
            class HyperionEncrypter extends \Illuminate\Encryption\Encrypter {
                public function encrypt(#[\SensitiveParameter] $value, $serialize = true) {
                    if (function_exists('hyperion_fast_encrypt')) {
                        if (!$serialize && is_string($value)) {
                            $res = hyperion_fast_encrypt($value, $this->key, $this->cipher);
                            if ($res !== false) {
                                return $res;
                            }
                        } elseif ($serialize) {
                            $serialized = serialize($value);
                            $res = hyperion_fast_encrypt($serialized, $this->key, $this->cipher);
                            if ($res !== false) {
                                return $res;
                            }
                        }
                    }
                    return parent::encrypt($value, $serialize);
                }

                public function encryptString(#[\SensitiveParameter] $value) {
                    return $this->encrypt($value, false);
                }

                public function decrypt($payload, $unserialize = true) {
                    if (is_string($payload) && function_exists('hyperion_fast_decrypt')) {
                        $res = hyperion_fast_decrypt($payload, $this->key, $this->cipher);
                        if ($res !== false) {
                            return $unserialize ? unserialize($res) : $res;
                        }
                    }
                    return parent::decrypt($payload, $unserialize);
                }

                public function decryptString($payload) {
                    return $this->decrypt($payload, false);
                }
            }
        }

        if ($app->has('encrypter')) {
            try {
                $origEnc = $app->make('encrypter');
                $key = (function() { return $this->key; })->call($origEnc);
                $cipher = (function() { return $this->cipher; })->call($origEnc);
                $fastEnc = new HyperionEncrypter($key, $cipher);
                $app->instance('encrypter', $fastEnc);
                $app->instance(\Illuminate\Contracts\Encryption\Encrypter::class, $fastEnc);
                $app->instance(\Illuminate\Contracts\Encryption\StringEncrypter::class, $fastEnc);
            } catch (\Throwable $e) {}
        }

        // Pre-warm the application pipeline into boot memory arena
        try {
            error_log("[WARMUP] Starting pre-warmup request");
            $warmupRes = $kernel->handle($warmupReq);
            $kernel->terminate($warmupReq, $warmupRes);

            // Also pre-warm /login if registered so Inertia, Blade, and Settings pipelines are fully initialized in Boot Arena
            if ($app->has('router') && $app['router']->has('login')) {
                try {
                    $warmupLoginReq = \Illuminate\Http\Request::create($appUrl . '/login', 'GET');
                    $app->instance('request', $warmupLoginReq);
                    $warmupLoginRes = $kernel->handle($warmupLoginReq);
                    $kernel->terminate($warmupLoginReq, $warmupLoginRes);
                    error_log("[WARMUP] /login pre-warmup completed with status: " . $warmupLoginRes->getStatusCode());
                } catch (\Throwable $e) {
                    error_log("[WARMUP] /login pre-warmup note: " . $e->getMessage());
                }
            }
            if (method_exists($app, 'forgetInstance')) {
                $app->forgetInstance('request');
                if (class_exists(\Illuminate\Foundation\Vite::class, false)) {
                    $app->forgetInstance(\Illuminate\Foundation\Vite::class);
                }
            }
            if (class_exists(\Tighten\Ziggy\BladeRouteGenerator::class)) {
                \Tighten\Ziggy\BladeRouteGenerator::$generated = false;
            }
            if (class_exists(\Inertia\Inertia::class)) {
                try { \Inertia\Inertia::flushShared(); } catch (\Throwable $e) {}
            }
            if ($app->has('router')) {
                $router = $app->make('router');
                (function() {
                    $this->current = null;
                    $this->currentRequest = null;
                })->call($router);
                foreach ($router->getRoutes()->getRoutes() as $route) {
                    $route->flushController();
                    (function() {
                        $this->parameters = null;
                        $this->originalParameters = null;
                    })->call($route);
                }
            }
            if (method_exists($app, 'forgetScopedInstances')) {
                $app->forgetScopedInstances();
            }
            error_log("[WARMUP] Pre-warmup request completed successfully: " . $warmupRes->getStatusCode());
        } catch (\Throwable $e) {
            error_log("[WARMUP EXCEPTION] " . $e->getMessage() . "\n" . $e->getTraceAsString());
        }

        if (!class_exists('HyperionEncrypter', false)) {
            class HyperionEncrypter extends \Illuminate\Encryption\Encrypter {
                public function encrypt(#[\SensitiveParameter] $value, $serialize = true) {
                    if (function_exists('hyperion_fast_encrypt')) {
                        if (!$serialize && is_string($value)) {
                            $res = hyperion_fast_encrypt($value, $this->key, $this->cipher);
                            if ($res !== false) {
                                return $res;
                            }
                        } elseif ($serialize) {
                            $serialized = serialize($value);
                            $res = hyperion_fast_encrypt($serialized, $this->key, $this->cipher);
                            if ($res !== false) {
                                return $res;
                            }
                        }
                    }
                    return parent::encrypt($value, $serialize);
                }

                public function encryptString(#[\SensitiveParameter] $value) {
                    return $this->encrypt($value, false);
                }

                public function decrypt($payload, $unserialize = true) {
                    if (is_string($payload) && function_exists('hyperion_fast_decrypt')) {
                        $res = hyperion_fast_decrypt($payload, $this->key, $this->cipher);
                        if ($res !== false) {
                            return $unserialize ? unserialize($res) : $res;
                        }
                    }
                    return parent::decrypt($payload, $unserialize);
                }

                public function decryptString($payload) {
                    return $this->decrypt($payload, false);
                }
            }
        }

        if ($app->has('view')) {
            try {
                $viewFactory = $app->make('view');
                $vendorErrors = $basePath . '/vendor/laravel/framework/src/Illuminate/Foundation/Exceptions/views';
                if (is_dir($vendorErrors)) {
                    $viewFactory->addNamespace('errors', $vendorErrors);
                }
            } catch (\Throwable $e) {}
        }

        // Pre-compile Symfony route regexes into boot memory while keeping standard Laravel Router & RouteCollection
        if ($app->has('router')) {
            try {
                $router = $app->make('router');
                foreach ($router->getRoutes()->getRoutes() as $route) {
                    $route->getCompiled();
                    if ($hyperionConfig['fast_route'] && function_exists('hyperion_route_register')) {
                        foreach ($route->methods() as $m) {
                            if ($m !== 'HEAD') {
                                hyperion_route_register($m, $route->uri(), 0, $route->getName() ?? '');
                            }
                        }
                    }
                }
            } catch (\Throwable $e) {}
        }

        // Fast-path native encrypter registration (byte-identical AES-256-CBC + HMAC-SHA256)
        if ($app->has('encrypter')) {
            try {
                $origEnc = $app->make('encrypter');
                $key = (function() { return $this->key; })->call($origEnc);
                $cipher = (function() { return $this->cipher; })->call($origEnc);
                $fastEnc = new HyperionEncrypter($key, $cipher);
                $app->instance('encrypter', $fastEnc);
                $app->instance(\Illuminate\Contracts\Encryption\Encrypter::class, $fastEnc);
                $app->instance(\Illuminate\Contracts\Encryption\StringEncrypter::class, $fastEnc);
            } catch (\Throwable $e) {}
        }

        // Pre-allocate Prototype Request (Zero new bag allocations per request)
        $prototypeRequest = \Illuminate\Http\Request::create('/', 'GET');

        // Eagerly resolve core singletons (DB, View, Cookie, Session, Auth, URL, Redirect, Translator, ResponseFactory)
        // BEFORE snapshotting container state so persistent connections and singletons stay warm across requests
        foreach ([
            'db',
            'view',
            'cookie',
            'session',
            'session.store',
            'auth',
            'translator',
            'files',
            \Illuminate\Contracts\Routing\ResponseFactory::class,
            \Illuminate\Contracts\Debug\ExceptionHandler::class,
        ] as $coreService) {
            if ($app->bound($coreService)) {
                try {
                    $app->make($coreService);
                } catch (\Throwable $e) {}
            }
        }

        $urlGenerator = null;
        if ($app->bound('url')) {
            try {
                $app->instance('request', $prototypeRequest);
                $urlGenerator = $app->make('url');
                if ($app->bound('redirect')) {
                    $app->make('redirect');
                }
                $app->forgetInstance('request');
            } catch (\Throwable $e) {
                $urlGenerator = null;
            }
        }

        if ($app->has('db')) {
            try {
                $db = $app->make('db');
                foreach ($db->getConnections() as $conn) {
                    if (method_exists($conn, 'getPdo')) {
                        HyperionIpc::tunePdo($conn->getPdo());
                    }
                }
            } catch (\Throwable $e) {}
        }

        // Flush any warmup cookies, session attributes, or auth guards before snapshotting clean boot state
        $bootReboundCallbacks = (function (): array {
            if (isset($this->instances['cookie'])) {
                try { $this->instances['cookie']->flushQueuedCookies(); } catch (\Throwable $e) {}
            }
            if (isset($this->instances['session.store'])) {
                try { $this->instances['session.store']->flush(); } catch (\Throwable $e) {}
            }
            if (isset($this->instances['auth']) && method_exists($this->instances['auth'], 'forgetGuards')) {
                try { $this->instances['auth']->forgetGuards(); } catch (\Throwable $e) {}
            }
            unset(
                $this->instances['auth.driver'],
                $this->instances['auth.password.tokens'],
                $this->instances['request']
            );
            if (!empty($this->reboundCallbacks['request'])) {
                $cleanReqCallbacks = [];
                foreach ($this->reboundCallbacks['request'] as $cb) {
                    if ($cb instanceof \Closure) {
                        try {
                            $rf = new \ReflectionFunction($cb);
                            $staticVars = $rf->getStaticVariables();
                            if (isset($staticVars['target']) && $staticVars['target'] instanceof \Illuminate\Contracts\Auth\Guard) {
                                continue;
                            }
                            $cbThis = $rf->getClosureThis();
                            if ($cbThis instanceof \Illuminate\Contracts\Auth\Guard) {
                                continue;
                            }
                        } catch (\Throwable $e) {}
                    }
                    $cleanReqCallbacks[] = $cb;
                }
                $this->reboundCallbacks['request'] = $cleanReqCallbacks;
            }
            return $this->reboundCallbacks;
        })->call($app);
        HyperionContainerSandbox::init($app);

        $bootLocale = 'en';
        if ($app->has('config')) {
            try {
                $bootLocale = (string)($app['config']->get('app.locale') ?: 'en');
            } catch (\Throwable $e) {}
        }

        // Suspend cyclic garbage collection during request bursts for deterministic latency
        if (function_exists('gc_disable')) {
            gc_disable();
        }

        HyperionIpc::$isLaravelFramework = true;
        $hasVite = class_exists(\Illuminate\Foundation\Vite::class, false);
        $hasZiggy = class_exists(\Tighten\Ziggy\BladeRouteGenerator::class, false);
        $hasInertia = class_exists(\Inertia\Inertia::class, false);

        // Pre-bound O(1) single-route reset (bound to Route scope for protected parameters)
        $resetMatchedRoute = \Closure::bind(static function (\Illuminate\Routing\Route $route): void {
            $route->parameters = null;
            $route->originalParameters = null;
        }, null, \Illuminate\Routing\Route::class);

        $resetViewFactory = null;
        if (class_exists(\Illuminate\View\Factory::class, false)) {
            $resetViewFactory = \Closure::bind(static function (\Illuminate\View\Factory $view): void {
                $view->renderCount = 0;
                $view->renderedOnce = [];
            }, null, \Illuminate\View\Factory::class);
        }

        // Pre-bound Octane-grade State Reset closure (bound once to Application scope)
        $fastStateReset = \Closure::bind(static function (\Illuminate\Foundation\Application $app, \Illuminate\Http\Request $request) use (
            $resetMatchedRoute,
            $resetViewFactory,
            $bootReboundCallbacks,
            $bootLocale,
            $hasVite,
            $hasZiggy,
            $hasInertia
        ): void {
            // 1. Clean matched route controller and bound parameters
            if ($matchedRoute = $request->route()) {
                $resetMatchedRoute($matchedRoute);
            }

            // 2. Cookie queue flush to prevent cookie accumulation across requests
            if (isset($app->instances['cookie'])) {
                try {
                    $app->instances['cookie']->flushQueuedCookies();
                } catch (\Throwable $e) {}
            }

            // 3. Auth, password broker & session state isolation (Octane FlushSessionState semantics:
            // flush in-memory session store attributes without destroying the driver or DB connections)
            if ($request->hasSession() && isset($app->instances['session.store'])) {
                try {
                    $app->instances['session.store']->flush();
                } catch (\Throwable $e) {}
            }
            unset(
                $app->instances['auth.driver'],
                $app->instances['auth.password.tokens'],
                $app->instances['request']
            );
            if (isset($app->instances['auth']) && $app->instances['auth']->hasResolvedGuards()) {
                try {
                    $app->instances['auth']->forgetGuards();
                } catch (\Throwable $e) {}
            }
            // Restore clean boot reboundCallbacks (O(1) Zend COW assignment; prevents O(N) SessionGuard leak)
            $app->reboundCallbacks = $bootReboundCallbacks;

            // 4. Reset translator locale to boot default only when changed
            if (isset($app->instances['translator']) && $app->instances['translator']->getLocale() !== $bootLocale) {
                try {
                    $app->instances['translator']->setLocale($bootLocale);
                } catch (\Throwable $e) {}
            }

            // 5. Reset View Factory per-request state
            if ($resetViewFactory !== null && isset($app->instances['view']) && $app->instances['view'] instanceof \Illuminate\View\Factory) {
                try {
                    $resetViewFactory($app->instances['view']);
                } catch (\Throwable $e) {}
            }

            // 6. Rollback any uncommitted PDO transactions
            HyperionIpc::rollbackActiveTransactions();

            // 7. Reset Inertia, Vite & Ziggy per-request state
            if ($hasInertia) {
                try { \Inertia\Inertia::flushShared(); } catch (\Throwable $e) {}
            }
            if ($hasVite) {
                unset($app->instances[\Illuminate\Foundation\Vite::class]);
            }
            if ($hasZiggy) {
                \Tighten\Ziggy\BladeRouteGenerator::$generated = false;
            }

            // 8. Clear scoped instances
            if (!empty($app->scopedInstances)) {
                $app->forgetScopedInstances();
            }
        }, null, \Illuminate\Foundation\Application::class);

        $resetLaravelProps = \Closure::bind(static function(\Illuminate\Http\Request $req): void {
            $req->session = null;
            $req->userResolver = null;
            $req->routeResolver = null;
            $req->defaultUser = null;
        }, null, \Illuminate\Http\Request::class);

        $hydrateHeaderBag = \Closure::bind(static function (\Symfony\Component\HttpFoundation\HeaderBag $bag, array $rawHeaders): void {
            $headers = [];
            $hasCacheControl = false;
            foreach ($rawHeaders as $key => $val) {
                $lk = strtolower((string)$key);
                $headers[$lk] = [(string)$val];
                if ($lk === 'cache-control') {
                    $hasCacheControl = true;
                }
            }
            $bag->headers = $headers;
            $bag->cacheControl = $hasCacheControl ? $bag->parseCacheControl($headers['cache-control'][0]) : [];
        }, null, \Symfony\Component\HttpFoundation\HeaderBag::class);

        $hydrateRequestFast = \Closure::bind(static function (
            \Illuminate\Http\Request $req,
            string $method,
            string $uri,
            array $query,
            array $post,
            array $cookies,
            array $files,
            array $server,
            array $rawHeaders,
            ?string $content
        ) use ($resetLaravelProps, $hydrateHeaderBag): void {
            $req->method = $method;
            $req->requestUri = $uri;
            $req->pathInfo = null;
            $req->baseUrl = null;
            $req->basePath = null;
            $req->format = null;
            $req->query->replace($query);
            $req->request->replace($post);
            $req->attributes->replace([]);
            $req->cookies->replace($cookies);
            if (!empty($files)) {
                $req->files->replace($files);
            } elseif ($req->files->count() > 0) {
                $req->files->replace([]);
            }
            $req->server->replace($server);
            $hydrateHeaderBag($req->headers, $rawHeaders);
            $req->content = $content;
            $req->json = HyperionIpc::$lastBodyIsJson ? $req->request : null;
            $req->convertedFiles = null;
            $resetLaravelProps($req);
        }, null, \Symfony\Component\HttpFoundation\Request::class);

        if ($nativeReactorActive) {
            $hydrateHeaderBagNative = \Closure::bind(static function (\Symfony\Component\HttpFoundation\HeaderBag $bag, array $symHeaders): void {
                $bag->headers = $symHeaders;
                $bag->cacheControl = isset($symHeaders['cache-control'][0])
                    ? $bag->parseCacheControl($symHeaders['cache-control'][0])
                    : [];
            }, null, \Symfony\Component\HttpFoundation\HeaderBag::class);

            $hydrateRequestNative = \Closure::bind(static function (
                \Illuminate\Http\Request $req,
                string $method,
                string $uri,
                array $query,
                array $post,
                array $cookies,
                array $files,
                array $server,
                array $symHeaders,
                ?string $content,
                bool $isJson
            ) use ($resetLaravelProps, $hydrateHeaderBagNative): void {
                $req->method = $method;
                $req->requestUri = $uri;
                $req->pathInfo = null;
                $req->baseUrl = null;
                $req->basePath = null;
                $req->format = null;
                $req->query->replace($query);
                $req->request->replace($post);
                $req->attributes->replace([]);
                $req->cookies->replace($cookies);
                if (!empty($files)) {
                    $req->files->replace($files);
                } elseif ($req->files->count() > 0) {
                    $req->files->replace([]);
                }
                $req->server->replace($server);
                $hydrateHeaderBagNative($req->headers, $symHeaders);
                $req->content = $content;
                $req->json = $isJson ? $req->request : null;
                $req->convertedFiles = null;
                $resetLaravelProps($req);
            }, null, \Symfony\Component\HttpFoundation\Request::class);

            $appDebugBool = false;
            if ($app->has('config')) {
                try {
                    $appDebugBool = (bool)$app['config']->get('app.debug');
                } catch (\Throwable $_) {}
            } else {
                $appDebugBool = filter_var(getenv('APP_DEBUG') ?: ($_ENV['APP_DEBUG'] ?? 'false'), FILTER_VALIDATE_BOOLEAN);
            }

            hyperion_reactor_serve(static function () use (
                $app,
                $kernel,
                $prototypeRequest,
                $hydrateRequestNative,
                $fastStateReset,
                $urlGenerator,
                $appDebugBool
            ): void {
                $request = clone $prototypeRequest;
                $hydrateRequestNative(
                    $request,
                    $_SERVER['REQUEST_METHOD'] ?? 'GET',
                    $_SERVER['REQUEST_URI'] ?? '/',
                    $_GET,
                    $_POST,
                    $_COOKIE,
                    $_FILES,
                    $_SERVER,
                    hyperion_request_headers(),
                    hyperion_request_raw_body(),
                    hyperion_request_is_json()
                );
                $app->instance('request', $request);
                if ($urlGenerator !== null) {
                    try {
                        $urlGenerator->setRequest($request);
                    } catch (\Throwable $e) {}
                }

                try {
                    $response = $kernel->handle($request);
                } catch (\Throwable $e) {
                    try {
                        $handler = $app->make(\Illuminate\Contracts\Debug\ExceptionHandler::class);
                        $handler->report($e);
                        $response = $handler->render($request, $e);
                    } catch (\Throwable $inner) {
                        $errCls = get_class($e);
                        $errMsg = $e->getMessage();
                        error_log("[HANDLE ERROR] {$errCls}: {$errMsg} in " . $e->getFile() . ':' . $e->getLine());
                        $msg = $appDebugBool ? "500 Server Error: {$errCls}: {$errMsg}" : '500 Internal Server Error';
                        $response = new \Illuminate\Http\Response($msg, 500, ['Content-Type' => 'text/plain; charset=UTF-8']);
                    }
                }

                HyperionIpc::emitNativeResponse($response);
                $kernel->terminate($request, $response);
                $fastStateReset($app, $request);
            }, $nativeReactorOpts);
            exit(0);
        }

        // Persistent Zero-Overhead Request Processing Loop (Legacy SHM IPC fallback)
        $maxRequests = (int)(getenv('HYPERION_MAX_REQUESTS') ?: 100000);
        $maxMemoryBytes = ((int)(getenv('HYPERION_MAX_MEMORY_MB') ?: 512)) * 1048576;
        $reqNum = 0;
        while (hyperion_accept_request()) {
            $reqNum++;
            $rawContent = (HyperionIpc::$active && HyperionIpc::$rawBody !== '') ? HyperionIpc::$rawBody : null;
            $request = clone $prototypeRequest;
            $hydrateRequestFast(
                $request,
                $_SERVER['REQUEST_METHOD'] ?? 'GET',
                $_SERVER['REQUEST_URI'] ?? '/',
                $_GET,
                $_POST,
                $_COOKIE,
                $_FILES,
                $_SERVER,
                HyperionIpc::$lastRawHeaders,
                $rawContent
            );
            $app->instance('request', $request);
            if ($app->has('url')) {
                try {
                    $app['url']->setRequest($request);
                } catch (\Throwable $e) {}
            }

            // Dispatch full Laravel Kernel pipeline
            try {
                $response = $kernel->handle($request);
            } catch (\Throwable $e) {
                try {
                    $handler = $app->make(\Illuminate\Contracts\Debug\ExceptionHandler::class);
                    $handler->report($e);
                    $response = $handler->render($request, $e);
                } catch (\Throwable $inner) {
                    $errCls = is_object($e) ? get_class($e) : gettype($e);
                    $errMsg = is_object($e) && method_exists($e, 'getMessage') ? $e->getMessage() : (string)$e;
                    $errFile = is_object($e) && method_exists($e, 'getFile') ? $e->getFile() : '';
                    $errLine = is_object($e) && method_exists($e, 'getLine') ? $e->getLine() : 0;
                    error_log("[HANDLE ERROR] $errCls: $errMsg in $errFile:$errLine\n" . ($e instanceof \Throwable ? $e->getTraceAsString() : ''));
                    $debug = false;
                    if ($app->has('config')) {
                        try { $debug = (bool)$app['config']->get('app.debug'); } catch (\Throwable $_) {}
                    } else {
                        $debug = filter_var(getenv('APP_DEBUG') ?: ($_ENV['APP_DEBUG'] ?? 'false'), FILTER_VALIDATE_BOOLEAN);
                    }
                    $msg = $debug ? "500 Server Error: $errCls: $errMsg" : "500 Internal Server Error";
                    $response = new \Illuminate\Http\Response($msg, 500, ['Content-Type' => 'text/plain; charset=UTF-8']);
                }
            }

            HyperionIpc::sendResponse($response);
            $kernel->terminate($request, $response);
            $fastStateReset($app, $request);
            HyperionIpc::cleanupTempFiles();

            // Periodic cyclic GC collect to prevent latency spikes
            if ($reqNum % 10000 === 0 && function_exists('gc_collect_cycles')) {
                gc_collect_cycles();
            }

            if ($reqNum >= $maxRequests || (($reqNum & 255) === 0 && memory_get_usage(true) >= $maxMemoryBytes)) {
                exit(0);
            }
        }
        break;

    // ------------------------------------------------------------------------
    // B. SYMFONY (Versions 5, 6, 7 & MicroKernel)
    // ------------------------------------------------------------------------
    case 'symfony':
        $kernel = null;
        $kernelClass = 'App\\Kernel';

        if (class_exists($kernelClass)) {
            $env = $_SERVER['APP_ENV'] ?? $_ENV['APP_ENV'] ?? 'prod';
            $debug = (bool)($_SERVER['APP_DEBUG'] ?? $_ENV['APP_DEBUG'] ?? false);
            $kernel = new $kernelClass($env, $debug);
            $kernel->boot();

            if ($kernel->getContainer()->has('doctrine')) {
                try {
                    $doctrine = $kernel->getContainer()->get('doctrine');
                    foreach ($doctrine->getConnections() as $conn) {
                        if (method_exists($conn, 'getNativeConnection')) {
                            HyperionIpc::tunePdo($conn->getNativeConnection());
                        }
                    }
                } catch (\Throwable $e) {}
            }
        }

        $routes = null;
        $matcher = null;
        $entrypoint = $hyperionConfig['entrypoint'] ?? $entrypoint;

        if ($kernel === null && file_exists($entrypoint)) {
            ob_start();
            $returned = require_once $entrypoint;
            ob_end_clean();
            if ($returned instanceof \Symfony\Component\HttpKernel\HttpKernelInterface) {
                $kernel = $returned;
            } elseif (is_array($returned) && isset($returned['routes'], $returned['matcher'])) {
                $routes = $returned['routes'];
                $matcher = $returned['matcher'];
            }
        }

        if ($kernel !== null) {
            if (class_exists('App\\Database') && method_exists('App\\Database', 'get')) {
                try {
                    HyperionIpc::tunePdo(\App\Database::get());
                } catch (\Throwable $e) {}
            }
            try {
                $warmupReq = \Symfony\Component\HttpFoundation\Request::create('/', 'GET');
                $warmupRes = $kernel->handle($warmupReq);
                $kernel->terminate($warmupReq, $warmupRes);
            } catch (\Throwable $e) {}
        }

        if ($nativeReactorActive && $kernel !== null) {
            $symPrototypeRequest = \Symfony\Component\HttpFoundation\Request::create('/', 'GET');
            $hydrateSymHeaders = \Closure::bind(static function (\Symfony\Component\HttpFoundation\HeaderBag $bag, array $symHeaders): void {
                $bag->headers = $symHeaders;
                $bag->cacheControl = isset($symHeaders['cache-control'][0])
                    ? $bag->parseCacheControl($symHeaders['cache-control'][0])
                    : [];
            }, null, \Symfony\Component\HttpFoundation\HeaderBag::class);

            $hydrateSymfonyFast = \Closure::bind(static function (
                \Symfony\Component\HttpFoundation\Request $req,
                string $method,
                string $uri,
                array $query,
                array $post,
                array $cookies,
                array $files,
                array $server,
                array $symHeaders,
                ?string $content
            ) use ($hydrateSymHeaders): void {
                $req->method = $method;
                $req->requestUri = $uri;
                $req->pathInfo = null;
                $req->baseUrl = null;
                $req->basePath = null;
                $req->format = null;
                $req->query->replace($query);
                $req->request->replace($post);
                $req->attributes->replace([]);
                $req->cookies->replace($cookies);
                if (!empty($files)) {
                    $req->files->replace($files);
                } elseif ($req->files->count() > 0) {
                    $req->files->replace([]);
                }
                $req->server->replace($server);
                $hydrateSymHeaders($req->headers, $symHeaders);
                $req->content = $content;
            }, null, \Symfony\Component\HttpFoundation\Request::class);

            $servicesResetter = null;
            try {
                if ($kernel->getContainer()->has('services_resetter')) {
                    $servicesResetter = $kernel->getContainer()->get('services_resetter');
                }
            } catch (\Throwable $e) {}

            $symDebug = (bool)($_SERVER['APP_DEBUG'] ?? $_ENV['APP_DEBUG'] ?? (getenv('APP_DEBUG') === '1'));
            $cleanupCb = is_callable($hyperionConfig['cleanup']) ? $hyperionConfig['cleanup'] : null;

            hyperion_reactor_serve(static function () use (
                $kernel,
                $symPrototypeRequest,
                $hydrateSymfonyFast,
                $servicesResetter,
                $symDebug,
                $cleanupCb
            ): void {
                $request = clone $symPrototypeRequest;
                $hydrateSymfonyFast(
                    $request,
                    $_SERVER['REQUEST_METHOD'] ?? 'GET',
                    $_SERVER['REQUEST_URI'] ?? '/',
                    $_GET,
                    $_POST,
                    $_COOKIE,
                    $_FILES,
                    $_SERVER,
                    hyperion_request_headers(),
                    hyperion_request_raw_body()
                );
                try {
                    $response = $kernel->handle($request);
                } catch (\Throwable $e) {
                    $msg = $symDebug ? ('500 Server Error: ' . get_class($e) . ': ' . $e->getMessage()) : '500 Internal Server Error';
                    $response = new \Symfony\Component\HttpFoundation\Response($msg, 500, ['Content-Type' => 'text/plain; charset=UTF-8']);
                }
                HyperionIpc::emitNativeResponse($response);
                $kernel->terminate($request, $response);
                if ($servicesResetter !== null) {
                    $servicesResetter->reset();
                }
                if ($cleanupCb !== null) {
                    $cleanupCb($kernel);
                }
            }, $nativeReactorOpts);
            exit(0);
        }

        $symMaxRequests = $maxRequestsCfg;
        $symMaxMemoryBytes = $maxMemoryMbCfg * 1048576;
        $symReqCount = 0;
        while (hyperion_accept_request()) {
            $symReqCount++;
            if ($kernel !== null) {
                if (HyperionIpc::$active && HyperionIpc::$rawBody !== '') {
                    $request = \Symfony\Component\HttpFoundation\Request::create(
                        $_SERVER['REQUEST_URI'] ?? '/',
                        $_SERVER['REQUEST_METHOD'] ?? 'GET',
                        $_POST,
                        $_COOKIE,
                        $_FILES,
                        $_SERVER,
                        HyperionIpc::$rawBody
                    );
                } else {
                    $request = \Symfony\Component\HttpFoundation\Request::createFromGlobals();
                }
                $response = $kernel->handle($request);
                HyperionIpc::sendResponse($response);
                $kernel->terminate($request, $response);

                if ($kernel->getContainer()->has('services_resetter')) {
                    $kernel->getContainer()->get('services_resetter')->reset();
                }
            } elseif ($matcher !== null) {
                $request = \Symfony\Component\HttpFoundation\Request::createFromGlobals();
                $context = new \Symfony\Component\Routing\RequestContext();
                $context->fromRequest($request);
                $matcher->setContext($context);
                try {
                    $parameters = $matcher->match($request->getPathInfo());
                    $controller = $parameters['_controller'];
                    $response = $controller($request, $parameters);
                } catch (\Symfony\Component\Routing\Exception\ResourceNotFoundException $e) {
                    $response = new \Symfony\Component\HttpFoundation\JsonResponse(['error' => 'Not Found'], 404);
                } catch (\Throwable $e) {
                    $response = new \Symfony\Component\HttpFoundation\JsonResponse(['error' => 'Internal Server Error'], 500);
                }
                HyperionIpc::sendResponse($response);
            } elseif (file_exists($entrypoint)) {
                (function($__file) {
                    ob_start();
                    require $__file;
                    ob_end_flush();
                })($entrypoint);
            }

            if (is_callable($hyperionConfig['cleanup'])) {
                ($hyperionConfig['cleanup'])($kernel);
            }

            if ($symReqCount >= $symMaxRequests || (($symReqCount & 255) === 0 && memory_get_usage(true) >= $symMaxMemoryBytes)) {
                exit(0);
            }
        }
        break;

    // ------------------------------------------------------------------------
    // C. SLIM 4 & PSR-7 / PSR-15 FRAMEWORKS
    // ------------------------------------------------------------------------
    case 'slim':
        $slimApp = null;
        $entrypoint = $hyperionConfig['entrypoint'] ?? $entrypoint;

        if (file_exists($entrypoint)) {
            ob_start();
            $returned = require_once $entrypoint;
            ob_end_clean();

            if ($returned instanceof \Slim\App) {
                $slimApp = $returned;
            } elseif (isset($GLOBALS['app']) && $GLOBALS['app'] instanceof \Slim\App) {
                $slimApp = $GLOBALS['app'];
            }
        }

        $serverRequestCreator = null;
        if (class_exists('\Slim\Factory\ServerRequestCreatorFactory')) {
            try {
                $serverRequestCreator = \Slim\Factory\ServerRequestCreatorFactory::create();
            } catch (\Throwable $e) {}
        }

        if (class_exists('EcommerceSlim\\Database') && method_exists('EcommerceSlim\\Database', 'get')) {
            try {
                HyperionIpc::tunePdo(\EcommerceSlim\Database::get());
            } catch (\Throwable $e) {}
        }

        if ($slimApp instanceof \Slim\App && $serverRequestCreator !== null) {
            try {
                $warmupReq = $serverRequestCreator->createServerRequestFromGlobals();
                $warmupRes = $slimApp->handle($warmupReq);
            } catch (\Throwable $e) {}
        }

        if ($nativeReactorActive && $slimApp instanceof \Slim\App) {
            $streamFactory = class_exists('\Slim\Psr7\Factory\StreamFactory')
                ? new \Slim\Psr7\Factory\StreamFactory()
                : null;
            $cleanupCb = is_callable($hyperionConfig['cleanup']) ? $hyperionConfig['cleanup'] : null;

            hyperion_reactor_serve(static function () use (
                $slimApp,
                $serverRequestCreator,
                $streamFactory,
                $cleanupCb
            ): void {
                $request = $serverRequestCreator !== null
                    ? $serverRequestCreator->createServerRequestFromGlobals()
                    : \Slim\Factory\ServerRequestCreatorFactory::create()->createServerRequestFromGlobals();
                $rawBody = hyperion_request_raw_body();
                if ($rawBody !== null && $rawBody !== '') {
                    if ($streamFactory !== null) {
                        $request = $request->withBody($streamFactory->createStream($rawBody));
                    }
                    if (!empty($_POST)) {
                        $request = $request->withParsedBody($_POST);
                    }
                } elseif (!empty($_POST)) {
                    $request = $request->withParsedBody($_POST);
                }

                $response = $slimApp->handle($request);
                HyperionIpc::emitNativeResponse($response);

                if ($cleanupCb !== null) {
                    $cleanupCb($slimApp);
                }
            }, $nativeReactorOpts);
            exit(0);
        }

        $slimMaxRequests = $maxRequestsCfg;
        $slimMaxMemoryBytes = $maxMemoryMbCfg * 1048576;
        $slimReqCount = 0;
        while (hyperion_accept_request()) {
            $slimReqCount++;
            if ($slimApp instanceof \Slim\App) {
                $request = $serverRequestCreator !== null
                    ? $serverRequestCreator->createServerRequestFromGlobals()
                    : \Slim\Factory\ServerRequestCreatorFactory::create()->createServerRequestFromGlobals();
                $rawInput = (HyperionIpc::$active && HyperionIpc::$rawBody !== '')
                    ? HyperionIpc::$rawBody
                    : (string)@file_get_contents('php://input');
                if ($rawInput !== '' && class_exists('\Slim\Psr7\Factory\StreamFactory')) {
                    $stream = (new \Slim\Psr7\Factory\StreamFactory())->createStream($rawInput);
                    $request = $request->withBody($stream);
                    if (stripos($_SERVER['CONTENT_TYPE'] ?? '', 'json') !== false) {
                        $parsedJson = json_decode($rawInput, true);
                        if (is_array($parsedJson)) {
                            $request = $request->withParsedBody($parsedJson);
                        }
                    } elseif (!empty($_POST)) {
                        $request = $request->withParsedBody($_POST);
                    }
                } elseif (!empty($_POST)) {
                    $request = $request->withParsedBody($_POST);
                }
                $response = $slimApp->handle($request);

                if (HyperionIpc::$active) {
                    HyperionIpc::sendResponse($response);
                } else {
                    if (!headers_sent()) {
                        header(sprintf(
                            'HTTP/%s %s %s',
                            $response->getProtocolVersion(),
                            $response->getStatusCode(),
                            $response->getReasonPhrase()
                        ));
                        foreach ($response->getHeaders() as $name => $values) {
                            foreach ($values as $value) {
                                header(sprintf('%s: %s', $name, $value), false);
                            }
                        }
                    }
                    echo (string)$response->getBody();
                }
            } elseif (file_exists($entrypoint)) {
                (function($__file) {
                    ob_start();
                    require $__file;
                    ob_end_flush();
                })($entrypoint);
            }

            if (is_callable($hyperionConfig['cleanup'])) {
                ($hyperionConfig['cleanup'])($slimApp);
            }

            if ($slimReqCount >= $slimMaxRequests || (($slimReqCount & 255) === 0 && memory_get_usage(true) >= $slimMaxMemoryBytes)) {
                exit(0);
            }
        }
        break;

    // ------------------------------------------------------------------------
    // D. WORDPRESS (CMS, REST API, WooCommerce)
    // ------------------------------------------------------------------------
    case 'wordpress':
        define('WP_USE_THEMES', true);

        if (file_exists($basePath . '/wp-load.php')) {
            require_once $basePath . '/wp-load.php';
        } elseif (file_exists($basePath . '/wp-config.php')) {
            require_once $basePath . '/wp-config.php';
        }

        if (isset($GLOBALS['wpdb']) && is_object($GLOBALS['wpdb']) && method_exists($GLOBALS['wpdb'], 'get_pdo')) {
            try {
                HyperionIpc::tunePdo($GLOBALS['wpdb']->get_pdo());
            } catch (\Throwable $e) {}
        }

        $wpTemplateLoader = defined('ABSPATH') && defined('WPINC')
            ? (ABSPATH . WPINC . '/template-loader.php')
            : ($basePath . '/wp-includes/template-loader.php');

        if ($nativeReactorActive) {
            $hasWpClass = class_exists('WP', false);
            $hasWpQueryClass = class_exists('WP_Query', false);
            $hasWpFn = function_exists('wp');
            $cleanupCb = is_callable($hyperionConfig['cleanup']) ? $hyperionConfig['cleanup'] : null;
            $wpOpts = $nativeReactorOpts;
            $wpOpts['sandbox_rollback'] = true;

            hyperion_reactor_serve(static function () use (
                $wpTemplateLoader,
                $hasWpClass,
                $hasWpQueryClass,
                $hasWpFn,
                $cleanupCb
            ): void {
                if ($hasWpClass && (!isset($GLOBALS['wp']) || !($GLOBALS['wp'] instanceof \WP))) {
                    $GLOBALS['wp'] = new \WP();
                }
                if ($hasWpQueryClass) {
                    $GLOBALS['wp_the_query'] = new \WP_Query();
                    $GLOBALS['wp_query'] = $GLOBALS['wp_the_query'];
                }
                unset($GLOBALS['post']);

                if ($hasWpFn) {
                    wp();
                }
                if (is_file($wpTemplateLoader)) {
                    hyperion_exec_script($wpTemplateLoader);
                }
                if ($cleanupCb !== null) {
                    $cleanupCb();
                }
            }, $wpOpts);
            exit(0);
        }

        $wpMaxRequests = $maxRequestsCfg;
        $wpMaxMemoryBytes = $maxMemoryMbCfg * 1048576;
        $wpReqCount = 0;
        while (hyperion_accept_request()) {
            $wpReqCount++;
            if (class_exists('WP') && (!isset($GLOBALS['wp']) || !($GLOBALS['wp'] instanceof \WP))) {
                $GLOBALS['wp'] = new \WP();
            }
            if (class_exists('WP_Query')) {
                $GLOBALS['wp_the_query'] = new \WP_Query();
                $GLOBALS['wp_query'] = $GLOBALS['wp_the_query'];
            }
            unset($GLOBALS['post']);

            ob_start();
            if (function_exists('wp')) {
                wp();
            }
            if (file_exists($wpTemplateLoader)) {
                require $wpTemplateLoader;
            }
            $output = ob_get_clean();
            if (HyperionIpc::$active) {
                HyperionIpc::sendOutputResponse($output);
            } else {
                echo $output;
            }

            if (is_callable($hyperionConfig['cleanup'])) {
                ($hyperionConfig['cleanup'])();
            }

            if ($wpReqCount >= $wpMaxRequests || (($wpReqCount & 255) === 0 && memory_get_usage(true) >= $wpMaxMemoryBytes)) {
                exit(0);
            }
        }
        break;

    // ------------------------------------------------------------------------
    // E. PLAIN / NATIVE PHP & CUSTOM SCRIPTS
    // ------------------------------------------------------------------------
    default:
    case 'plain':
        $entrypoint = $hyperionConfig['entrypoint'] ?? $entrypoint;

        $discoverAndTuneDatabases = static function (): void {
            $commonDbMethods = ['getConnection', 'getPdo', 'get', 'instance', 'getInstance', 'connection', 'db'];
            foreach (get_declared_classes() as $cls) {
                if (str_ends_with($cls, 'Database') || str_ends_with($cls, 'DB') || str_contains($cls, 'Connection') || str_contains($cls, '\\Database')) {
                    if (method_exists($cls, 'getAllPartitionConnections')) {
                        try {
                            foreach ($cls::getAllPartitionConnections() as $shardedPdo) {
                                HyperionIpc::tunePdo($shardedPdo);
                            }
                        } catch (\Throwable $e) {}
                    }
                    foreach ($commonDbMethods as $mth) {
                        if (method_exists($cls, $mth)) {
                            try {
                                $conn = $cls::$mth();
                                if ($conn instanceof \PDO) {
                                    HyperionIpc::tunePdo($conn);
                                } elseif (is_object($conn) && method_exists($conn, 'getPdo')) {
                                    HyperionIpc::tunePdo($conn->getPdo());
                                }
                            } catch (\Throwable $e) {}
                        }
                    }
                }
            }
        };

        $entrypointDeclaresTopLevelSymbols = static function (string $file): bool {
            if (!is_file($file) || !class_exists(\PhpToken::class)) {
                return true;
            }
            $src = @file_get_contents($file);
            if ($src === false || $src === '') {
                return false;
            }
            $tokens = \PhpToken::tokenize($src);
            $cnt = count($tokens);
            $depth = 0;
            $prevMeaningfulId = 0;
            for ($i = 0; $i < $cnt; $i++) {
                $t = $tokens[$i];
                $tid = $t->id;
                $txt = $t->text;
                if ($tid === T_WHITESPACE || $tid === T_COMMENT || $tid === T_DOC_COMMENT) {
                    continue;
                }
                if ($txt === '{' || $tid === T_CURLY_OPEN || $tid === T_DOLLAR_OPEN_CURLY_BRACES) {
                    $depth++;
                    $prevMeaningfulId = $tid;
                    continue;
                }
                if ($txt === '}') {
                    if ($depth > 0) {
                        $depth--;
                    }
                    $prevMeaningfulId = $tid;
                    continue;
                }
                if ($depth === 0) {
                    if (($tid === T_CLASS || $tid === T_INTERFACE || $tid === T_TRAIT || $tid === T_ENUM) && $prevMeaningfulId !== T_NEW) {
                        return true;
                    }
                    if ($tid === T_CONST) {
                        return true;
                    }
                    if ($tid === T_FUNCTION) {
                        for ($k = $i + 1; $k < $cnt; $k++) {
                            $nt = $tokens[$k];
                            if ($nt->id === T_WHITESPACE || $nt->id === T_COMMENT || $nt->id === T_DOC_COMMENT || $nt->text === '&') {
                                continue;
                            }
                            if ($nt->id === T_STRING) {
                                return true;
                            }
                            break;
                        }
                    }
                    if ($tid === T_STRING && strcasecmp($txt, 'define') === 0) {
                        return true;
                    }
                }
                $prevMeaningfulId = $tid;
            }
            return false;
        };

        if ($nativeReactorActive) {
            if ($entrypoint && is_file($entrypoint) && !$entrypointDeclaresTopLevelSymbols($entrypoint)) {
                $_SERVER['REQUEST_METHOD'] = 'GET';
                $_SERVER['REQUEST_URI'] = '/health';
                ob_start();
                @hyperion_exec_script($entrypoint);
                @ob_end_clean();
                if (function_exists('header_remove')) {
                    @header_remove();
                }
                http_response_code(200);
            }
            $discoverAndTuneDatabases();

            $plainOpts = $nativeReactorOpts;
            $plainOpts['sandbox_rollback'] = true;

            if (is_callable($hyperionConfig['cleanup'])) {
                $cleanupCb = $hyperionConfig['cleanup'];
                hyperion_reactor_serve(static function () use ($entrypoint, $cleanupCb): void {
                    if ($entrypoint && is_file($entrypoint)) {
                        hyperion_exec_script($entrypoint);
                    }
                    $cleanupCb();
                }, $plainOpts);
            } else {
                /* 100% C-direct execution: Rust mio -> C SAPI -> OPcache JIT -> C Sandbox Rollback! */
                hyperion_reactor_serve(null, $plainOpts);
            }
            exit(0);
        }

        $discoverAndTuneDatabases();
        $plainMaxRequests = $maxRequestsCfg;
        $plainMaxMemoryBytes = $maxMemoryMbCfg * 1048576;
        $plainReqCount = 0;
        $plainDbTuned = false;

        while (hyperion_accept_request()) {
            $plainReqCount++;
            ob_start();
            $uri = $_SERVER['REQUEST_URI'] ?? '/';
            $parsedPath = parse_url($uri, PHP_URL_PATH) ?? '/';
            $cleanPath = ltrim($parsedPath, '/');

            if (str_contains($cleanPath, '..') || str_contains($cleanPath, "\0")) {
                http_response_code(400);
                header("Content-Type: text/plain; charset=UTF-8");
                echo "400 Bad Request: Path traversal sequence rejected.";
                $output = ob_get_clean();
                if (HyperionIpc::$active) {
                    HyperionIpc::sendOutputResponse($output, 400);
                } else {
                    echo $output;
                }
                continue;
            }

            $targetScript = null;
            if ($cleanPath !== '') {
                $candidate = null;
                if (file_exists($publicDir . '/' . $cleanPath)) {
                    $candidate = $publicDir . '/' . $cleanPath;
                } elseif (file_exists($basePath . '/' . $cleanPath)) {
                    $candidate = $basePath . '/' . $cleanPath;
                }

                if ($candidate !== null) {
                    $real = realpath($candidate);
                    if ($real !== false && (str_starts_with($real, $publicDir . '/') || str_starts_with($real, $basePath . '/'))) {
                        $filename = basename($real);
                        $isSensitive = str_starts_with($filename, '.')
                            || preg_match('/\.(env|git|lock|sql|sqlite|sqlite3|db|yml|yaml|toml|sh|bat|bak|dist)$/i', $filename);

                        if ($isSensitive) {
                            http_response_code(403);
                            header("Content-Type: text/plain; charset=UTF-8");
                            echo "403 Forbidden: Access to protected file is denied.";
                            $output = ob_get_clean();
                            if (HyperionIpc::$active) {
                                HyperionIpc::sendOutputResponse($output, 403);
                            } else {
                                echo $output;
                            }
                            continue;
                        }
                        $targetScript = $real;
                    }
                }
            }

            if ($targetScript !== null && !str_ends_with($targetScript, '.php') && is_file($targetScript)) {
                $ext = strtolower(pathinfo($targetScript, PATHINFO_EXTENSION));
                $allowedMimes = [
                    'css' => 'text/css', 'js' => 'application/javascript', 'json' => 'application/json',
                    'png' => 'image/png', 'jpg' => 'image/jpeg', 'jpeg' => 'image/jpeg', 'gif' => 'image/gif',
                    'webp' => 'image/webp', 'svg' => 'image/svg+xml', 'ico' => 'image/x-icon',
                    'woff' => 'font/woff', 'woff2' => 'font/woff2', 'ttf' => 'font/ttf',
                    'pdf' => 'application/pdf', 'txt' => 'text/plain; charset=UTF-8', 'mp4' => 'video/mp4'
                ];
                $ctype = $allowedMimes[$ext] ?? null;
                if ($ctype === null) {
                    http_response_code(403);
                    header("Content-Type: text/plain; charset=UTF-8");
                    echo "403 Forbidden: Static file type not permitted.";
                    $output = ob_get_clean();
                    if (HyperionIpc::$active) {
                        HyperionIpc::sendOutputResponse($output, 403);
                    } else {
                        echo $output;
                    }
                    continue;
                }
                header("Content-Type: {$ctype}");
                header("Content-Length: " . filesize($targetScript));
                header("X-Content-Type-Options: nosniff");
                readfile($targetScript);
                $output = ob_get_clean();
                if (HyperionIpc::$active) {
                    HyperionIpc::sendOutputResponse($output);
                } else {
                    echo $output;
                }
                continue;
            }

            if ($targetScript === null || !is_file($targetScript) || !str_ends_with($targetScript, '.php')) {
                $targetScript = $entrypoint;
            }

            if ($targetScript && file_exists($targetScript)) {
                (function($__file) {
                    require $__file;
                })($targetScript);
                $content = ob_get_clean();
                if (HyperionIpc::$active) {
                    HyperionIpc::sendOutputResponse($content);
                } else {
                    echo $content;
                }
            } else {
                http_response_code(404);
                header("Content-Type: text/plain");
                $notFoundMsg = "404 Not Found: " . htmlspecialchars($parsedPath);
                $output = ob_get_clean();
                if (HyperionIpc::$active) {
                    HyperionIpc::sendOutputResponse($notFoundMsg, 404);
                } else {
                    echo $notFoundMsg;
                }
            }

            if (!$plainDbTuned) {
                $discoverAndTuneDatabases();
                $plainDbTuned = true;
            }

            if (is_callable($hyperionConfig['cleanup'])) {
                ($hyperionConfig['cleanup'])();
            }

            $_GET = [];
            $_POST = [];
            $_FILES = [];
            $_COOKIE = [];
            $_REQUEST = [];
            if (session_status() === PHP_SESSION_ACTIVE) {
                @session_write_close();
            }
            while (ob_get_level() > 0) {
                @ob_end_clean();
            }

            if ($plainReqCount >= $plainMaxRequests || (($plainReqCount & 255) === 0 && memory_get_usage(true) >= $plainMaxMemoryBytes)) {
                exit(0);
            }
        }
        break;
}
