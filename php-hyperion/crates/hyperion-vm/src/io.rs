use crate::fibre::Fibre;
use crossbeam_utils::sync::Unparker;

use dashmap::DashMap;
use mio::{Events, Poll, Token, Interest};
use mio::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::io::Read;
use std::collections::HashMap;
use std::collections::BTreeMap;
use std::time::Instant;

pub use crossbeam_deque::Injector;

pub const FCGI_BEGIN_REQUEST: u8 = 1;
pub const FCGI_ABORT_REQUEST: u8 = 2;
pub const FCGI_END_REQUEST: u8   = 3;
pub const FCGI_PARAMS: u8        = 4;
pub const FCGI_STDIN: u8         = 5;
pub const FCGI_STDOUT: u8        = 6;
pub const FCGI_STDERR: u8        = 7;

const SAPI_LISTENER_TOKEN: Token = Token(usize::MAX - 1);
const FASTCGI_LISTENER_TOKEN: Token = Token(usize::MAX - 2);

#[derive(Clone)]
pub struct PreformattedHttpResponses {
    pub keep_alive_bytes: Arc<Vec<u8>>,
    pub close_bytes: Arc<Vec<u8>>,
    pub expires_at: Option<std::time::Instant>,
}

pub fn get_global_response_cache() -> &'static DashMap<String, PreformattedHttpResponses> {
    static CACHE: std::sync::OnceLock<DashMap<String, PreformattedHttpResponses>> = std::sync::OnceLock::new();
    CACHE.get_or_init(DashMap::new)
}

static LAST_SEEN_DB_VERSION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

static NEXT_UNPARKER_IDX: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

#[inline(always)]
pub fn unpark_one_worker(unparkers: &[Unparker]) {
    if !unparkers.is_empty() {
        let idx = NEXT_UNPARKER_IDX.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % unparkers.len();
        unparkers[idx].unpark();
    }
}

#[inline(always)]
pub fn check_and_invalidate_cache() {
    let current_v = hyperion_core::DB_MUTATION_VERSION.load(std::sync::atomic::Ordering::Acquire);
    if current_v != LAST_SEEN_DB_VERSION.load(std::sync::atomic::Ordering::Relaxed) {
        get_global_response_cache().clear();
        LAST_SEEN_DB_VERSION.store(current_v, std::sync::atomic::Ordering::Relaxed);
    }
}

pub fn clear_global_response_cache() {
    get_global_response_cache().clear();
    let current_v = hyperion_core::DB_MUTATION_VERSION.fetch_add(1, std::sync::atomic::Ordering::Release);
    LAST_SEEN_DB_VERSION.store(current_v + 1, std::sync::atomic::Ordering::Relaxed);
}

#[derive(Debug)]
pub struct FastCgiRecord {
    pub type_: u8,
    pub request_id: u16,
    pub content: Vec<u8>,
}

pub fn url_decode<'a>(s: &'a str) -> std::borrow::Cow<'a, str> {
    if !s.as_bytes().iter().any(|&b| b == b'%' || b == b'+') {
        return std::borrow::Cow::Borrowed(s);
    }
    let mut result = Vec::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex_str) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                if let Ok(byte) = u8::from_str_radix(hex_str, 16) {
                    result.push(byte);
                    i += 3;
                    continue;
                }
            }
        }
        if bytes[i] == b'+' {
            result.push(b' ');
        } else {
            result.push(bytes[i]);
        }
        i += 1;
    }
    std::borrow::Cow::Owned(String::from_utf8_lossy(&result).into_owned())
}

pub fn get_mime_type(ext: &str) -> &'static str {
    match ext {
        "ico" => "image/x-icon",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "css" => "text/css; charset=UTF-8",
        "js" | "mjs" => "application/javascript; charset=UTF-8",
        "json" => "application/json",
        "txt" => "text/plain; charset=UTF-8",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "eot" => "application/vnd.ms-fontobject",
        "otf" => "font/otf",
        "wasm" => "application/wasm",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mp3" => "audio/mpeg",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}

pub fn is_safe_static_request(clean_path: &str) -> bool {
    if clean_path.is_empty() || clean_path.contains("..") || clean_path.contains('\0') || !clean_path.contains('.') {
        return false;
    }
    if clean_path.starts_with('.') || clean_path.contains("/.") {
        return false;
    }
    let lower = clean_path.to_lowercase();
    let forbidden_exts = [
        ".env", ".sql", ".sqlite", ".sqlite3", ".db", ".lock",
        ".yml", ".yaml", ".toml", ".sh", ".bat", ".bak", ".dist",
        ".git", ".md", ".log", ".php"
    ];
    for ext in forbidden_exts {
        if lower.ends_with(ext) {
            return false;
        }
    }
    true
}

pub fn resolve_safe_static_file(clean_path: &str) -> Option<std::path::PathBuf> {
    if !is_safe_static_request(clean_path) {
        return None;
    }
    if let Ok(canon_public) = std::fs::canonicalize("public") {
        let target = canon_public.join(clean_path);
        if let Ok(canon_target) = std::fs::canonicalize(&target) {
            if canon_target.starts_with(&canon_public) && canon_target.is_file() {
                return Some(canon_target);
            }
        }
    }
    if let Ok(canon_cwd) = std::fs::canonicalize(".") {
        let target = canon_cwd.join(clean_path);
        if let Ok(canon_target) = std::fs::canonicalize(&target) {
            if canon_target.starts_with(&canon_cwd) && canon_target.is_file() {
                return Some(canon_target);
            }
        }
    }
    None
}

pub struct Reactor {
    pub id: usize,
    pub poll: Mutex<Poll>,
    pub registry: mio::Registry,
    pub suspended_fibres: DashMap<Token, Fibre>,
    pub timeouts: Mutex<BTreeMap<Instant, Vec<Token>>>,
    pub global_queue: Arc<Injector<Fibre>>,
    pub unparkers: Vec<Unparker>,
    sapi_listener: Mutex<Option<TcpListener>>,
    fastcgi_listener: Mutex<Option<TcpListener>>,
    pub engine_state: Arc<crate::fibre::GlobalEngineState>,
    pub routing_script: Option<String>,
    pub idle_fibres: Arc<Injector<Fibre>>,
    pub idle_octane_fibres: Arc<Injector<Fibre>>,
    pub keep_alive_queue: Arc<Injector<(mio::net::TcpStream, Vec<u8>)>>,
    pub is_worker_mode: bool,
}

#[inline(always)]
pub fn write_all_socket(stream: &mut mio::net::TcpStream, bytes: &[u8]) {
    use std::io::Write;
    let mut data = bytes;
    let mut retries = 0;
    while !data.is_empty() && retries < 100_000 {
        match stream.write(data) {
            Ok(0) => break,
            Ok(n) => {
                data = &data[n..];
                retries = 0;
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock || e.raw_os_error() == Some(35) || e.raw_os_error() == Some(11) => {
                retries += 1;
                std::thread::yield_now();
            }
            Err(_) => break,
        }
    }
}

impl Reactor {
    pub fn new(
        global_queue: Arc<Injector<Fibre>>,
        unparkers: Vec<Unparker>,
        engine_state: Arc<crate::fibre::GlobalEngineState>,
        routing_script: Option<String>,
        is_worker_mode: bool,
    ) -> Self {
        Self::new_with_idle_fibres(
            0,
            global_queue,
            unparkers,
            engine_state,
            routing_script,
            is_worker_mode,
            Arc::new(Injector::new()),
            Arc::new(Injector::new()),
        )
    }

    pub fn new_with_idle_fibres(
        id: usize,
        global_queue: Arc<Injector<Fibre>>,
        unparkers: Vec<Unparker>,
        engine_state: Arc<crate::fibre::GlobalEngineState>,
        routing_script: Option<String>,
        is_worker_mode: bool,
        idle_fibres: Arc<Injector<Fibre>>,
        idle_octane_fibres: Arc<Injector<Fibre>>,
    ) -> Self {
        let poll = Poll::new().expect("Failed to create Poll");
        let registry = poll.registry().try_clone().expect("Failed to clone registry");
        
        Self {
            id,
            poll: Mutex::new(poll),
            registry,
            suspended_fibres: DashMap::new(),
            timeouts: Mutex::new(BTreeMap::new()),
            global_queue,
            unparkers,
            sapi_listener: Mutex::new(None),
            fastcgi_listener: Mutex::new(None),
            engine_state,
            routing_script,
            idle_fibres,
            idle_octane_fibres,
            keep_alive_queue: Arc::new(Injector::new()),
            is_worker_mode,
        }
    }

    pub fn recycle_keep_alive_stream(&self, stream: mio::net::TcpStream) {
        self.recycle_keep_alive_stream_with_buffer(stream, Vec::new());
    }

    pub fn recycle_keep_alive_stream_with_buffer(&self, stream: mio::net::TcpStream, buffer: Vec<u8>) {
        self.keep_alive_queue.push((stream, buffer));
    }


    /// Bind the embedded HTTP Server to a port and register it with the Reactor
    pub fn bind_sapi(&self, addr: std::net::SocketAddr) {
        let domain = if addr.is_ipv6() { socket2::Domain::IPV6 } else { socket2::Domain::IPV4 };
        let socket = socket2::Socket::new(domain, socket2::Type::STREAM, Some(socket2::Protocol::TCP))
            .expect("Failed to create SAPI socket");
        let _ = socket.set_reuse_address(true);
        #[cfg(unix)]
        unsafe {
            use std::os::unix::io::AsRawFd;
            let optval: libc::c_int = 1;
            libc::setsockopt(
                socket.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_REUSEPORT,
                &optval as *const _ as *const libc::c_void,
                std::mem::size_of::<libc::c_int>() as libc::socklen_t,
            );
        }
        let _ = socket.set_nonblocking(true);
        let _ = socket.set_nodelay(true);
        let sock_addr = socket2::SockAddr::from(addr);
        socket.bind(&sock_addr).expect("Failed to bind SAPI port");
        socket.listen(4096).expect("Failed to listen on SAPI port");
        
        let std_listener: std::net::TcpListener = socket.into();
        let mut listener = TcpListener::from_std(std_listener);
        self.registry.register(&mut listener, SAPI_LISTENER_TOKEN, Interest::READABLE)
            .expect("Failed to register SAPI listener");
        *self.sapi_listener.lock().unwrap() = Some(listener);
        let now = chrono::Local::now().format("%a %b %e %H:%M:%S %Y");
        eprintln!("[{}] PHP 8.4.1 Development Server (http://{}) started", now, addr);
    }

    /// Bind the FastCGI Responder to a port and register it with the Reactor
    pub fn bind_fastcgi(&self, addr: std::net::SocketAddr) {
        let domain = if addr.is_ipv6() { socket2::Domain::IPV6 } else { socket2::Domain::IPV4 };
        let socket = socket2::Socket::new(domain, socket2::Type::STREAM, Some(socket2::Protocol::TCP))
            .expect("Failed to create FastCGI socket");
        let _ = socket.set_reuse_address(true);
        #[cfg(unix)]
        unsafe {
            use std::os::unix::io::AsRawFd;
            let optval: libc::c_int = 1;
            libc::setsockopt(
                socket.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_REUSEPORT,
                &optval as *const _ as *const libc::c_void,
                std::mem::size_of::<libc::c_int>() as libc::socklen_t,
            );
        }
        let _ = socket.set_nonblocking(true);
        let _ = socket.set_nodelay(true);
        let sock_addr = socket2::SockAddr::from(addr);
        socket.bind(&sock_addr).expect("Failed to bind FastCGI port");
        socket.listen(4096).expect("Failed to listen on FastCGI port");
        
        let std_listener: std::net::TcpListener = socket.into();
        let mut listener = TcpListener::from_std(std_listener);
        self.registry.register(&mut listener, FASTCGI_LISTENER_TOKEN, Interest::READABLE)
            .expect("Failed to register FastCGI listener");
        *self.fastcgi_listener.lock().unwrap() = Some(listener);
        eprintln!("🚀 PHP-Hyperion SAPI FastCGI Responder listening on tcp://{}", addr);
    }

    pub fn register_fibre(&self, token: Token, fibre: Fibre, timeout: Option<Duration>) {
        // Insert FIRST to avoid race condition where Reactor processes the event before insertion!
        self.suspended_fibres.insert(token, fibre);

        if let Some(mut f) = self.suspended_fibres.get_mut(&token) {
            if let Some(stream) = f.tcp_stream.as_mut() {
                if let Err(e) = self.registry.reregister(
                    stream,
                    token,
                    Interest::READABLE,
                ).or_else(|e1| {
                    self.registry.register(
                        stream,
                        token,
                        Interest::READABLE,
                    ).map_err(|e2| format!("reregister: {}, register: {}", e1, e2))
                }) {
                    eprintln!("Failed to register socket for fibre {}: {}", token.0, e);
                }
            }
        }
        
        if let Some(dur) = timeout {
            let expiry = Instant::now() + dur;
            self.timeouts.lock().unwrap().entry(expiry).or_default().push(token);
        }
    }

    #[inline(always)]
    pub fn try_serve_cached(stream: &mut mio::net::TcpStream, buffer: &[u8]) -> Option<(bool, usize)> {
        // Enforce 64 KB HTTP Request Header Limit (HYP-SEC-02)
        if let Some(pos) = buffer.windows(4).position(|w| w == b"\r\n\r\n") {
            let request_end = pos + 4;
            if request_end > 65_536 {
                write_all_socket(stream, b"HTTP/1.1 431 Request Header Fields Too Large\r\nContent-Type: text/plain\r\nContent-Length: 32\r\nConnection: close\r\n\r\nRequest Header Fields Too Large\n");
                let _ = stream.shutdown(std::net::Shutdown::Both);
                return Some((false, request_end));
            }
        } else if buffer.len() > 65_536 {
            write_all_socket(stream, b"HTTP/1.1 431 Request Header Fields Too Large\r\nContent-Type: text/plain\r\nContent-Length: 32\r\nConnection: close\r\n\r\nRequest Header Fields Too Large\n");
            let _ = stream.shutdown(std::net::Shutdown::Both);
            return Some((false, buffer.len()));
        }

        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);
        if let Ok(status) = req.parse(buffer) {
            let request_end = match status {
                httparse::Status::Complete(amt) => amt,
                httparse::Status::Partial => return None,
            };
            if request_end > 65_536 {
                write_all_socket(stream, b"HTTP/1.1 431 Request Header Fields Too Large\r\nContent-Type: text/plain\r\nContent-Length: 32\r\nConnection: close\r\n\r\nRequest Header Fields Too Large\n");
                let _ = stream.shutdown(std::net::Shutdown::Both);
                return Some((false, request_end));
            }
            let method_str = req.method.unwrap_or("GET");
            if method_str.eq_ignore_ascii_case("OPTIONS") {
                let version = req.version.unwrap_or(1);
                let mut keep_alive = version == 1;
                for header in req.headers.iter() {
                    if header.name.eq_ignore_ascii_case("connection") {
                        if header.value.eq_ignore_ascii_case(b"close") {
                            keep_alive = false;
                        } else if header.value.eq_ignore_ascii_case(b"keep-alive") {
                            keep_alive = true;
                        }
                    }
                }
                let conn_str = if keep_alive { "keep-alive" } else { "close" };
                let resp = format!(
                    "HTTP/1.1 204 No Content\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, PUT, DELETE, PATCH, OPTIONS\r\nAccess-Control-Allow-Headers: *\r\nConnection: {}\r\nContent-Length: 0\r\n\r\n",
                    conn_str
                );
                write_all_socket(stream, resp.as_bytes());
                return Some((keep_alive, request_end));
            }
            if method_str.eq_ignore_ascii_case("GET") || method_str.eq_ignore_ascii_case("HEAD") {
                let path_str = req.path.unwrap_or("/");
                // RFC 9111: Bypass shared cache if client sends Authorization, Cookie, or no-cache/no-store
                let bypass_cache = req.headers.iter().any(|h| {
                    h.name.eq_ignore_ascii_case("authorization") ||
                    h.name.eq_ignore_ascii_case("cookie") ||
                    (h.name.eq_ignore_ascii_case("cache-control") && {
                        let c = std::str::from_utf8(h.value).unwrap_or("");
                        c.contains("no-cache") || c.contains("no-store")
                    }) ||
                    (h.name.eq_ignore_ascii_case("pragma") && {
                        let c = std::str::from_utf8(h.value).unwrap_or("");
                        c.contains("no-cache")
                    })
                });
                if !bypass_cache {
                    check_and_invalidate_cache();
                    let cache = get_global_response_cache();
                    let cached_opt = cache.get(path_str).or_else(|| {
                        if path_str.starts_with('/') {
                            cache.get(&path_str[1..])
                        } else {
                            cache.get(&format!("/{}", path_str))
                        }
                    });
                    if let Some(cached) = cached_opt {
                        if let Some(exp) = cached.expires_at {
                            if std::time::Instant::now() > exp {
                                drop(cached);
                                cache.remove(path_str);
                                if path_str.starts_with('/') {
                                    cache.remove(&path_str[1..]);
                                } else {
                                    cache.remove(&format!("/{}", path_str));
                                }
                                return None;
                            }
                        }
                        let version = req.version.unwrap_or(1);
                        let mut keep_alive = version == 1;
                        for header in req.headers.iter() {
                            if header.name.eq_ignore_ascii_case("connection") {
                                if header.value.eq_ignore_ascii_case(b"close") {
                                    keep_alive = false;
                                } else if header.value.eq_ignore_ascii_case(b"keep-alive") {
                                    keep_alive = true;
                                }
                            }
                        }
                        let bytes = if keep_alive { &cached.keep_alive_bytes } else { &cached.close_bytes };
                        write_all_socket(stream, bytes.as_slice());
                        return Some((keep_alive, request_end));
                    }
                }
            }
        }
        None
    }

    pub fn dispatch_http_request(&self, mut stream: mio::net::TcpStream, mut buffer: Vec<u8>) {
        // Fast Path: Check Generational HTTP Response Cache for GET requests
        if let Some((keep_alive, req_end)) = Self::try_serve_cached(&mut stream, &buffer) {
            if keep_alive {
                buffer.drain(..req_end);
                self.recycle_keep_alive_stream_with_buffer(stream, buffer);
            } else {
                use std::io::Write;
                let _ = stream.flush();
                let _ = stream.shutdown(std::net::Shutdown::Both);
            }
            return;
        }

        let mut fibre = match self.idle_octane_fibres.steal() {
            crossbeam_deque::Steal::Success(f) => f,
            _ => match self.idle_fibres.steal().success() {
                Some(f) => f,
                None => {
                    static NEXT_FIBRE_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(100_000);
                    let id = NEXT_FIBRE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if let Some(ref proto) = *self.engine_state.boot_prototype.read().unwrap() {
                        proto.clone_for_worker(id)
                    } else {
                        let func_ptr = self.routing_script.as_deref()
                            .and_then(|path| self.engine_state.opcache.get(path).map(|e| e.func_ptr))
                            .unwrap_or(crate::types::function::FunctionPtr(std::ptr::null_mut()));
                        let mut f = Fibre::new(id, func_ptr, std::sync::Arc::clone(&self.engine_state));
                        f.reset_for_worker_request(func_ptr);
                        f
                    }
                }
            },
        };

        fibre.tcp_stream = Some(stream);
        fibre.http_parse_buffer = buffer;
        fibre.is_keep_alive_idle = true;
        fibre.reactor_id = self.id;
        fibre.protocol = crate::fibre::RequestProtocol::Http;
        self.global_queue.push(fibre);
        unpark_one_worker(&self.unparkers);
    }


    /// Simulation method to bypass OS I/O events in stress tests
    pub fn simulate_wake_all(&self) {
        let mut keys_to_wake = Vec::new();
        for item in self.suspended_fibres.iter() {
            keys_to_wake.push(*item.key());
        }
        for key in keys_to_wake {
            if let Some((_, mut fibre)) = self.suspended_fibres.remove(&key) {
                fibre.resume();
                self.global_queue.push(fibre);
                if !self.unparkers.is_empty() {
                    self.unparkers[0].unpark();
                }
            }
        }
    }

    /// The Reactor Loop that runs on a dedicated OS Thread
    pub fn run(&self) {
        let mut pending_streams: std::collections::HashMap<Token, (mio::net::TcpStream, Vec<u8>)> = std::collections::HashMap::new();
        let mut next_token_id = 1_000_000_000;
        let mut events = Events::with_capacity(1024);
        loop {
            loop {
                match self.keep_alive_queue.steal() {
                    crossbeam_deque::Steal::Success((mut stream, mut buffer)) => {
                        let mut keep_conn = true;
                        while buffer.windows(4).any(|w| w == b"\r\n\r\n") {
                            if let Some((keep_alive, req_end)) = Self::try_serve_cached(&mut stream, &buffer) {
                                buffer.drain(..req_end);
                                if !keep_alive {
                                    keep_conn = false;
                                    use std::io::Write;
                                    let _ = stream.flush();
                                    let _ = stream.shutdown(std::net::Shutdown::Both);
                                    break;
                                }
                            } else {
                                break;
                            }
                        }
                        if keep_conn {
                            let token = Token(next_token_id);
                            next_token_id += 1;
                            if let Ok(_) = self.registry.register(&mut stream, token, Interest::READABLE) {
                                pending_streams.insert(token, (stream, buffer));
                            }
                        }
                    }
                    crossbeam_deque::Steal::Retry => continue,
                    crossbeam_deque::Steal::Empty => break,
                }
            }

            let poll_timeout = if pending_streams.is_empty() {
                Some(Duration::from_micros(50))
            } else {
                Some(Duration::from_micros(10))
            };
            let poll_result = self.poll.lock().unwrap().poll(&mut events, poll_timeout);
            if poll_result.is_ok() {
                for event in events.iter() {
                    match event.token() {
                        
                        SAPI_LISTENER_TOKEN => {
                            if let Some(listener) = &*self.sapi_listener.lock().unwrap() {
                                loop {
                                    match listener.accept() {
                                        Ok((mut stream, _addr)) => {
                                            let _ = stream.set_nodelay(true);
                                            let mut buffer = Vec::new();
                                            let mut buf = [0; 4096];
                                            let mut is_blocked = false;
                                            let mut is_closed = false;
                                            loop {
                                                match stream.read(&mut buf) {
                                                    Ok(0) => { is_closed = true; break; } // EOF
                                                    Ok(n) => {
                                                        buffer.extend_from_slice(&buf[..n]);
                                                        let header_len = match buffer.windows(4).position(|w| w == b"\r\n\r\n") {
                                                            Some(pos) => pos + 4,
                                                            None => buffer.len(),
                                                        };
                                                        if header_len > 65_536 {
                                                            write_all_socket(&mut stream, b"HTTP/1.1 431 Request Header Fields Too Large\r\nConnection: close\r\n\r\n");
                                                            let _ = stream.shutdown(std::net::Shutdown::Both);
                                                            is_closed = true;
                                                            break;
                                                        }
                                                        if buffer.len() > 16 * 1024 * 1024 {
                                                            write_all_socket(&mut stream, b"HTTP/1.1 413 Payload Too Large\r\nConnection: close\r\n\r\n");
                                                            let _ = stream.shutdown(std::net::Shutdown::Both);
                                                            is_closed = true;
                                                            break;
                                                        }
                                                    }
                                                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock || e.raw_os_error() == Some(35) || e.raw_os_error() == Some(11) => {
                                                        is_blocked = true;
                                                        break;
                                                    }
                                                    Err(_e) => {
                                                        is_closed = true;
                                                        break;
                                                    }
                                                }
                                            }

                                            if is_closed {
                                                continue;
                                            }

                                            if !buffer.windows(4).any(|w| w == b"\r\n\r\n") {
                                                if is_blocked {
                                                    let token = Token(next_token_id);
                                                    next_token_id += 1;
                                                    if let Err(e) = self.registry.register(&mut stream, token, Interest::READABLE) {
                                                        eprintln!("Failed to register pending stream: {}", e);
                                                    } else {
                                                        pending_streams.insert(token, (stream, buffer));
                                                    }
                                                }
                                                continue;
                                            }

                                            // Direct Reactor cache fast-path
                                            let mut keep_conn = true;
                                            let mut served_any = false;
                                            while buffer.windows(4).any(|w| w == b"\r\n\r\n") {
                                                if let Some((keep_alive, req_end)) = Self::try_serve_cached(&mut stream, &buffer) {
                                                    served_any = true;
                                                    buffer.drain(..req_end);
                                                    if !keep_alive {
                                                        keep_conn = false;
                                                        use std::io::Write;
                                                        let _ = stream.flush();
                                                        let _ = stream.shutdown(std::net::Shutdown::Both);
                                                        break;
                                                    }
                                                } else {
                                                    break;
                                                }
                                            }
                                            if !keep_conn {
                                                continue;
                                            }
                                            if served_any {
                                                if buffer.windows(4).any(|w| w == b"\r\n\r\n") {
                                                    self.dispatch_http_request(stream, buffer);
                                                } else {
                                                    let token = Token(next_token_id);
                                                    next_token_id += 1;
                                                    if let Err(e) = self.registry.register(&mut stream, token, Interest::READABLE) {
                                                        eprintln!("Failed to register keep-alive stream: {}", e);
                                                    } else {
                                                        pending_streams.insert(token, (stream, buffer));
                                                    }
                                                }
                                                continue;
                                            }

                                            // Fully read request
                                            self.dispatch_http_request(stream, buffer);
                                        }
                                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock || e.raw_os_error() == Some(35) || e.raw_os_error() == Some(11) => {
                                            break; // No more pending connections
                                        }
                                        Err(e) => {
                                            eprintln!("Error accepting connection: {}", e);
                                            break;
                                        }
                                    }
                                }
                            }
                        }

                        token if token.0 >= 1_000_000_000 => {
                            if let Some(mut entry) = pending_streams.remove(&token) {
                                let stream = &mut entry.0;
                                let buffer = &mut entry.1;
                                
                                let mut is_closed = false;
                                loop {
                                    let mut buf = [0; 8192];
                                    match stream.read(&mut buf) {
                                        Ok(0) => { is_closed = true; break; }
                                        Ok(n) => {
                                            buffer.extend_from_slice(&buf[..n]);
                                            let header_len = match buffer.windows(4).position(|w| w == b"\r\n\r\n") {
                                                Some(pos) => pos + 4,
                                                None => buffer.len(),
                                            };
                                            if header_len > 65_536 {
                                                write_all_socket(stream, b"HTTP/1.1 431 Request Header Fields Too Large\r\nConnection: close\r\n\r\n");
                                                let _ = stream.shutdown(std::net::Shutdown::Both);
                                                is_closed = true;
                                                break;
                                            }
                                            if buffer.len() > 16 * 1024 * 1024 {
                                                write_all_socket(stream, b"HTTP/1.1 413 Payload Too Large\r\nConnection: close\r\n\r\n");
                                                let _ = stream.shutdown(std::net::Shutdown::Both);
                                                is_closed = true;
                                                break;
                                            }
                                        }
                                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock || e.raw_os_error() == Some(35) || e.raw_os_error() == Some(11) => {
                                            break;
                                        }
                                        Err(_) => {
                                            is_closed = true;
                                            break;
                                        }
                                    }
                                }

                                if is_closed {
                                    let _ = self.registry.deregister(stream);
                                    let _ = stream.shutdown(std::net::Shutdown::Both);
                                    continue;
                                }

                                // Burst-serve all buffered/pipelined cached requests directly
                                let mut keep_conn = true;
                                while buffer.windows(4).any(|w| w == b"\r\n\r\n") {
                                    if let Some((keep_alive, req_end)) = Self::try_serve_cached(stream, buffer) {
                                        buffer.drain(..req_end);
                                        if !keep_alive {
                                            keep_conn = false;
                                            use std::io::Write;
                                            let _ = stream.flush();
                                            let _ = self.registry.deregister(stream);
                                            let _ = stream.shutdown(std::net::Shutdown::Both);
                                            break;
                                        }
                                    } else {
                                        break;
                                    }
                                }
                                if !keep_conn {
                                    continue;
                                }

                                if buffer.windows(4).any(|w| w == b"\r\n\r\n") {
                                    // There is a dynamic request to dispatch to worker thread
                                    let (mut s, b) = entry;
                                    let _ = self.registry.deregister(&mut s);
                                    self.dispatch_http_request(s, b);
                                } else {
                                    // Re-arm readable and keep connection alive in pending_streams
                                    let _ = self.registry.reregister(stream, token, Interest::READABLE).or_else(|_| {
                                        self.registry.register(stream, token, Interest::READABLE)
                                    });
                                    pending_streams.insert(token, entry);
                                }
                            }
                        }
FASTCGI_LISTENER_TOKEN => {
                            if let Some(listener) = &*self.fastcgi_listener.lock().unwrap() {
                                loop {
                                    match listener.accept() {
                                        Ok((mut stream, _addr)) => {
                                            let _ = stream.set_nodelay(true);
                                            // eprintln!("Accepted new FastCGI connection from {}", addr);
                                            let mut buffer = Vec::new();
                                            let mut buf = [0; 4096];
                                            let mut retries = 0;
                                            
                                            loop {
                                                match stream.read(&mut buf) {
                                                    Ok(0) => break, // EOF
                                                    Ok(n) => {
                                                        buffer.extend_from_slice(&buf[..n]);
                                                        break;
                                                    }
                                                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                                        if buffer.is_empty() && retries < 10_000 {
                                                            retries += 1;
                                                            std::thread::yield_now();
                                                            continue;
                                                        }
                                                        break;
                                                    }
                                                    Err(_e) => {
                                                        break;
                                                    }
                                                }
                                            }

                                            if !buffer.is_empty() {
                                                let (records, _bytes_consumed) = parse_fastcgi_records(&buffer);
                                                
                                                let mut request_id = 0;
                                                let mut keep_conn = false;
                                                let mut params_data = Vec::new();
                                                let mut stdin_data = Vec::new();
                                                
                                                for record in records {
                                                    request_id = record.request_id;
                                                    match record.type_ {
                                                        1 => { // FCGI_BEGIN_REQUEST
                                                            if record.content.len() >= 3 {
                                                                keep_conn = (record.content[2] & 1) != 0;
                                                            }
                                                        }
                                                        4 => { // FCGI_PARAMS
                                                            params_data.extend_from_slice(&record.content);
                                                        }
                                                        5 => { // FCGI_STDIN
                                                            stdin_data.extend_from_slice(&record.content);
                                                        }
                                                        _ => {}
                                                    }
                                                }
                                                
                                                let params = parse_fastcgi_params(&params_data);
                                                let script_filename = params.get("SCRIPT_FILENAME").cloned()
                                                    .unwrap_or_else(|| "index.php".to_string());
                                                
                                                let octane_fibre = self.idle_octane_fibres.steal().success().or_else(|| {
                                                    static NEXT_FIBRE_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(200_000);
                                                    let id = NEXT_FIBRE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                                    self.engine_state.boot_prototype.read().unwrap().as_ref().map(|p| p.clone_for_worker(id))
                                                });
                                                if let Some(mut fibre) = octane_fibre {
                                                    fibre.populate_fastcgi_superglobals(&params, &stdin_data);
                                                    fibre.tcp_stream = Some(stream);
                                                    fibre.protocol = crate::fibre::RequestProtocol::FastCgi { request_id, keep_conn };
                                                    crate::vm::VM::push(&mut fibre, hyperion_core::memory::nan_box::Value::new_bool(true));
                                                    fibre.resume();
                                                    self.global_queue.push(fibre);
                                                    if let Some(unparker) = self.unparkers.first() {
                                                        unparker.unpark();
                                                    }
                                                    continue;
                                                }
                                                
                                                match self.engine_state.compile_and_load_script(&script_filename) {
                                                    Ok(func_ptr) => {
                                                        let mut fibre = if let Some(mut idle) = self.idle_fibres.steal().success() {
                                                            idle.clear(func_ptr);
                                                            idle.id = 1000 + request_id as u64;
                                                            idle
                                                        } else {
                                                            let mut idle = Fibre::new(1000 + request_id as u64, func_ptr, Arc::clone(&self.engine_state));
                                                            idle.clear(func_ptr);
                                                            idle
                                                        };
                                                        
                                                        fibre.populate_fastcgi_superglobals(&params, &stdin_data);
                                                        
                                                        fibre.tcp_stream = Some(stream);
                                                        fibre.protocol = crate::fibre::RequestProtocol::FastCgi { request_id, keep_conn };
                                                        fibre.resume();
                                                        self.global_queue.push(fibre);
                                                        
                                                        if let Some(unparker) = self.unparkers.first() {
                                                            unparker.unpark();
                                                        }
                                                    }
                                                    Err(e) => {
                                                        eprintln!("FastCGI SAPI: Failed to compile script '{}': {}", script_filename, e);
                                                        let _ = write_fastcgi_error(&mut stream, request_id, &e);
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                            break; // No more pending connections
                                        }
                                        Err(e) => {
                                            eprintln!("Error accepting connection: {}", e);
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        token => {
                            if let Some((_, mut fibre)) = self.suspended_fibres.remove(&token) {
                                if let Some(ref mut stream) = fibre.tcp_stream {
                                    let _ = self.registry.deregister(stream);
                                }
                                fibre.resume();
                                self.global_queue.push(fibre);
                                unpark_one_worker(&self.unparkers);
                            } else {
                                eprintln!("Reactor: Token {} fired but NOT FOUND", token.0);
                            }
                        }
                    }
                }
            }
            
            // 5. Timer Wheel sweep would happen here to clear out timed-out Fibres
            let now = Instant::now();
            let mut expired_tokens = Vec::new();
            {
                let mut timeouts_guard = self.timeouts.lock().unwrap();
                let rest = timeouts_guard.split_off(&now);
                let expired = std::mem::replace(&mut *timeouts_guard, rest);
                for (_, tokens) in expired {
                    expired_tokens.extend(tokens);
                }
            }

            for token in expired_tokens {
                if let Some((_, mut fibre)) = self.suspended_fibres.remove(&token) {
                    eprintln!("Fibre {} timed out", token.0);
                    fibre.kill();
                }
            }
        }
    }
}

pub fn parse_fastcgi_records(buffer: &[u8]) -> (Vec<FastCgiRecord>, usize) {
    let mut records = Vec::new();
    let mut offset = 0;
    while offset + 8 <= buffer.len() {
        let _version = buffer[offset];
        let type_ = buffer[offset + 1];
        let request_id = u16::from_be_bytes([buffer[offset + 2], buffer[offset + 3]]);
        let content_length = u16::from_be_bytes([buffer[offset + 4], buffer[offset + 5]]) as usize;
        let padding_length = buffer[offset + 6] as usize;
        
        let record_size = 8 + content_length + padding_length;
        if offset + record_size > buffer.len() {
            // Partial record
            break;
        }
        
        let content = buffer[offset + 8..offset + 8 + content_length].to_vec();
        records.push(FastCgiRecord {
            type_,
            request_id,
            content,
        });
        offset += record_size;
    }
    (records, offset)
}

pub fn parse_fastcgi_params(content: &[u8]) -> HashMap<String, String> {
    let mut params = HashMap::new();
    let mut offset = 0;
    while offset < content.len() {
        if offset >= content.len() { break; }
        let mut name_len = content[offset] as usize;
        if name_len & 0x80 != 0 {
            if offset + 4 > content.len() { break; }
            name_len = (((content[offset] & 0x7f) as usize) << 24)
                | ((content[offset + 1] as usize) << 16)
                | ((content[offset + 2] as usize) << 8)
                | (content[offset + 3] as usize);
            offset += 4;
        } else {
            offset += 1;
        }
        
        if offset >= content.len() { break; }
        let mut val_len = content[offset] as usize;
        if val_len & 0x80 != 0 {
            if offset + 4 > content.len() { break; }
            val_len = (((content[offset] & 0x7f) as usize) << 24)
                | ((content[offset + 1] as usize) << 16)
                | ((content[offset + 2] as usize) << 8)
                | (content[offset + 3] as usize);
            offset += 4;
        } else {
            offset += 1;
        }
        
        if offset + name_len + val_len > content.len() { break; }
        
        let name = String::from_utf8_lossy(&content[offset..offset + name_len]).into_owned();
        offset += name_len;
        let val = String::from_utf8_lossy(&content[offset..offset + val_len]).into_owned();
        offset += val_len;
        
        params.insert(name, val);
    }
    params
}

fn has_cgi_headers(output: &[u8]) -> bool {
    let mut header_end = None;
    for i in 0..output.len() {
        if i + 4 <= output.len() && &output[i..i+4] == b"\r\n\r\n" {
            header_end = Some(i);
            break;
        }
        if i + 2 <= output.len() && &output[i..i+2] == b"\n\n" {
            header_end = Some(i);
            break;
        }
    }
    if let Some(end) = header_end {
        let prefix = &output[..end];
        prefix.contains(&b':')
    } else {
        false
    }
}

pub fn write_fastcgi_response<W: std::io::Write>(
    writer: &mut W,
    request_id: u16,
    output: &[u8],
) -> std::io::Result<()> {
    let has_headers = has_cgi_headers(output);
    let mut full_payload = Vec::new();
    if !has_headers {
        full_payload.extend_from_slice(b"Status: 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\n\r\n");
    }
    full_payload.extend_from_slice(output);

    // Chunking the stdout
    let chunk_size = 65000;
    let mut offset = 0;
    while offset < full_payload.len() {
        let end = std::cmp::min(offset + chunk_size, full_payload.len());
        let chunk = &full_payload[offset..end];
        
        let header = [
            1, // version
            FCGI_STDOUT,
            (request_id >> 8) as u8,
            (request_id & 0xff) as u8,
            (chunk.len() >> 8) as u8,
            (chunk.len() & 0xff) as u8,
            0, // padding
            0, // reserved
        ];
        writer.write_all(&header)?;
        writer.write_all(chunk)?;
        offset = end;
    }

    // Empty stdout record to indicate EOF
    let empty_header = [
        1, // version
        FCGI_STDOUT,
        (request_id >> 8) as u8,
        (request_id & 0xff) as u8,
        0, 0, // content_length = 0
        0, 0, // padding, reserved
    ];
    writer.write_all(&empty_header)?;

    // End request record
    let end_content = [
        0, 0, 0, 0, // app_status = 0 (success)
        0, // protocol_status = FCGI_REQUEST_COMPLETE (0)
        0, 0, 0, // reserved
    ];
    let end_header = [
        1, // version
        FCGI_END_REQUEST,
        (request_id >> 8) as u8,
        (request_id & 0xff) as u8,
        0, 8, // content_length = 8
        0, 0, // padding, reserved
    ];
    writer.write_all(&end_header)?;
    writer.write_all(&end_content)?;
    writer.flush()?;

    Ok(())
}

pub fn write_fastcgi_error<W: std::io::Write>(writer: &mut W, request_id: u16, error_msg: &str) -> std::io::Result<()> {
    let stdout_content = format!("Status: 500 Internal Server Error\r\nContent-Type: text/plain\r\n\r\n{}", error_msg);
    let stdout_bytes = stdout_content.as_bytes();
    
    // Write FCGI_STDOUT record (type = 6)
    let header = [
        1, // version
        FCGI_STDOUT, // type (FCGI_STDOUT = 6)
        (request_id >> 8) as u8,
        (request_id & 0xff) as u8,
        (stdout_bytes.len() >> 8) as u8,
        (stdout_bytes.len() & 0xff) as u8,
        0, // padding
        0, // reserved
    ];
    writer.write_all(&header)?;
    writer.write_all(stdout_bytes)?;
    
    // Write FCGI_END_REQUEST record (type = 3)
    let end_content = [
        0, 0, 0, 1, // app_status (1 for error)
        0, // protocol_status (FCGI_REQUEST_COMPLETE = 0)
        0, 0, 0, // reserved
    ];
    let end_header = [
        1, // version
        FCGI_END_REQUEST, // type (FCGI_END_REQUEST = 3)
        (request_id >> 8) as u8,
        (request_id & 0xff) as u8,
        0,
        8, // content_length (8 bytes)
        0, // padding
        0, // reserved
    ];
    writer.write_all(&end_header)?;
    writer.write_all(&end_content)?;
    writer.flush()?;
    Ok(())
}
