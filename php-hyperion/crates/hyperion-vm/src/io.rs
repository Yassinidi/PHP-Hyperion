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
}

pub fn get_global_response_cache() -> &'static DashMap<String, PreformattedHttpResponses> {
    static CACHE: std::sync::OnceLock<DashMap<String, PreformattedHttpResponses>> = std::sync::OnceLock::new();
    CACHE.get_or_init(DashMap::new)
}

static LAST_SEEN_DB_VERSION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

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

pub fn url_decode(s: &str) -> String {
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
    String::from_utf8_lossy(&result).into_owned()
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
        for unparker in &self.unparkers {
            unparker.unpark();
        }
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
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);
        if let Ok(status) = req.parse(buffer) {
            let request_end = match status {
                httparse::Status::Complete(amt) => amt,
                httparse::Status::Partial => return None,
            };
            let method_str = req.method.unwrap_or("GET");
            if method_str.eq_ignore_ascii_case("GET") {
                let has_auth = req.headers.iter().any(|h| h.name.eq_ignore_ascii_case("authorization"));
                if !has_auth {
                    check_and_invalidate_cache();
                    let cache = get_global_response_cache();
                    let path_str = req.path.unwrap_or("/");
                    let alt_key = if path_str.starts_with('/') { path_str.to_string() } else { format!("/{}", path_str) };
                    if let Some(cached) = cache.get(path_str).or_else(|| cache.get(&alt_key)) {
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
                        use std::io::Write;
                        let mut data = bytes.as_slice();
                        while !data.is_empty() {
                            match stream.write(data) {
                                Ok(0) => break,
                                Ok(n) => data = &data[n..],
                                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock || e.raw_os_error() == Some(35) || e.raw_os_error() == Some(11) => {
                                    std::thread::yield_now();
                                }
                                Err(_) => break,
                            }
                        }
                        return Some((keep_alive, request_end));
                    }
                }
            }
        }
        None
    }

    pub fn dispatch_http_request(&self, mut stream: mio::net::TcpStream, buffer: Vec<u8>) {
        // Fast Path: Check Generational HTTP Response Cache for GET requests
        if let Some((keep_alive, _req_end)) = Self::try_serve_cached(&mut stream, &buffer) {
            if keep_alive {
                self.recycle_keep_alive_stream(stream);
            } else {
                use std::io::Write;
                let _ = stream.flush();
                let _ = stream.shutdown(std::net::Shutdown::Both);
            }
            return;
        }

        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);
        
        if let Ok(status) = req.parse(&buffer) {
            let body_offset = match status {
                httparse::Status::Complete(amt) => amt,
                httparse::Status::Partial => return, // shouldn't happen here
            };

            let path_str = req.path.unwrap_or("/");
            let method_str = req.method.unwrap_or("GET");
            if std::env::var("HYPERION_VERBOSE").is_ok() {
                let now = chrono::Local::now().format("%a %b %e %H:%M:%S %Y");
                if let Ok(peer_addr) = stream.peer_addr() {
                    eprintln!("[{}] {} Accepted: {} {}", now, peer_addr, method_str, path_str);
                }
            }
            let raw_clean_path = path_str.split('?').next().unwrap_or("/").trim_start_matches('/');
            let clean_path = url_decode(raw_clean_path);

            let candidate_file = if clean_path.is_empty() {

                if let Some(ref rs) = self.routing_script {
                    std::path::PathBuf::from(rs)
                } else {
                    std::path::PathBuf::from("index.php")
                }
            } else if std::path::Path::new(&clean_path).is_file() {
                std::path::PathBuf::from(&clean_path)
            } else if std::path::Path::new("public").join(&clean_path).is_file() {
                std::path::Path::new("public").join(&clean_path)
            } else if let Some(ref rs) = self.routing_script {
                std::path::PathBuf::from(rs)
            } else {
                std::path::PathBuf::from(&clean_path)
            };


            // If it is an existing static file and NOT a PHP script, serve it directly!
            if candidate_file.is_file() {
                let ext = candidate_file.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                if ext != "php" {
                    if let Ok(file_bytes) = std::fs::read(&candidate_file) {
                        let content_type = get_mime_type(&ext);
                        use std::io::Write;
                        let header = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            content_type,
                            file_bytes.len()
                        );
                        let mut to_write = Vec::with_capacity(header.len() + file_bytes.len());
                        to_write.extend_from_slice(header.as_bytes());
                        to_write.extend_from_slice(&file_bytes);
                        let mut data = to_write.as_slice();
                        while !data.is_empty() {
                            match stream.write(data) {
                                Ok(0) => break,
                                Ok(n) => data = &data[n..],
                                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock || e.raw_os_error() == Some(35) || e.raw_os_error() == Some(11) => {
                                    std::thread::yield_now();
                                }
                                Err(_) => break,
                            }
                        }
                        let _ = stream.flush();
                        let _ = stream.shutdown(std::net::Shutdown::Both);
                        
                        if std::env::var("HYPERION_VERBOSE").is_ok() {
                            let now = chrono::Local::now().format("%a %b %e %H:%M:%S %Y");
                            if let Ok(peer_addr) = stream.peer_addr() {
                                eprintln!("[{}] {} [200]: GET {}", now, peer_addr, path_str);
                            }
                        }
                        return;
                    }
                }
            }


            let file_path = if candidate_file.is_file() && candidate_file.extension().and_then(|e| e.to_str()) == Some("php") {
                candidate_file.to_string_lossy().into_owned()
            } else if self.routing_script.is_some() {
                self.routing_script.as_ref().unwrap().clone()
            } else {
                path_str.trim_start_matches('/').to_string()
            };

            let engine_env = std::env::var("HYPERION_ENGINE").unwrap_or_else(|_| "auto".to_string()).to_lowercase();
            let use_zend = engine_env == "php84" || engine_env == "zend" || engine_env == "php" 
                || (engine_env == "auto" && crate::zend_sapi::file_requires_php84(&file_path));

            if use_zend {
                let peer_addr_str = stream.peer_addr().ok().map(|a| a.to_string());
                let local_addr_str = stream.local_addr().ok().map(|a| a.to_string());
                let query_str = path_str.split_once('?').map(|x| x.1).unwrap_or("");
                let docroot = std::env::current_dir().map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|_| ".".to_string());
                
                let mut req_headers = Vec::new();
                for h in req.headers.iter() {
                    if !h.name.is_empty() {
                        if let Ok(v) = std::str::from_utf8(h.value) {
                            req_headers.push((h.name, v));
                        }
                    }
                }
                
                let body = &buffer[body_offset..];
                match crate::zend_sapi::execute_http(
                    &file_path,
                    &docroot,
                    method_str,
                    path_str,
                    query_str,
                    &req_headers,
                    body,
                    peer_addr_str.as_deref(),
                    local_addr_str.as_deref(),
                ) {
                    Ok(resp) => {
                        let resp_bytes = resp.to_http_bytes();
                        use std::io::Write;
                        let mut data = resp_bytes.as_slice();
                        while !data.is_empty() {
                            match stream.write(data) {
                                Ok(0) => break,
                                Ok(n) => data = &data[n..],
                                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock || e.raw_os_error() == Some(35) || e.raw_os_error() == Some(11) => {
                                    std::thread::yield_now();
                                }
                                Err(_) => break,
                            }
                        }
                        let _ = stream.flush();
                        let _ = stream.shutdown(std::net::Shutdown::Both);
                        return;
                    }
                    Err(e) => {
                        eprintln!("Zend SAPI Error: {}", e);
                        use std::io::Write;
                        let err_resp = format!("HTTP/1.1 500 Internal Server Error\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nZend Engine Error: {}\n", e);
                        let _ = stream.write_all(err_resp.as_bytes());
                        let _ = stream.flush();
                        let _ = stream.shutdown(std::net::Shutdown::Both);
                        return;
                    }
                }
            }
            
            let func_ptr = if !file_path.is_empty() && std::path::Path::new(&file_path).is_file() {
                match self.engine_state.compile_and_load_script(&file_path) {
                    Ok(fptr) => fptr,
                    Err(e) => {
                        eprintln!("HTTP SAPI: Failed to compile script '{}': {}", file_path, e);
                        use std::io::Write;
                        let response = format!("HTTP/1.1 500 Internal Server Error\r\n\r\nFailed to compile script: {}", e);
                        let _ = stream.write_all(response.as_bytes());
                        return;
                    }
                }
            } else {
                let mut chunk = hyperion_bytecode::Chunk::new();
                let response_text = format!("Hello from PHP-Hyperion! You requested: {}", path_str);
                let str_val = Box::into_raw(Box::new(response_text));
                let val = hyperion_core::memory::nan_box::Value::new_string_ptr(str_val as *mut ());
                let idx = chunk.add_constant(val);
                
                chunk.write_opcode(hyperion_bytecode::Opcode::Constant);
                chunk.write_short(idx);
                chunk.write_opcode(hyperion_bytecode::Opcode::Echo);
                chunk.write_opcode(hyperion_bytecode::Opcode::Return);
                
                let func = Box::new(crate::types::function::PhpFunction::new("main".to_string(), 0, chunk));
                crate::types::function::FunctionPtr(Box::into_raw(func))
            };
            
            let body = &buffer[body_offset..];
            if self.is_worker_mode {
                let mut fibre = loop {
                    match self.idle_octane_fibres.steal() {
                        crossbeam_deque::Steal::Success(f) => break f,
                        crossbeam_deque::Steal::Retry => continue,
                        crossbeam_deque::Steal::Empty => {
                            std::thread::sleep(std::time::Duration::from_micros(100));
                        }
                    }
                };

                fibre.populate_http_superglobals(&req, body, &file_path);
                fibre.http_parse_buffer.clear();
                fibre.is_keep_alive_idle = false;
                fibre.tcp_stream = Some(stream);
                fibre.reactor_id = self.id;
                fibre.protocol = crate::fibre::RequestProtocol::Http;
                
                if let Some(pos) = buffer.windows(4).position(|w| w == b"\r\n\r\n") {
                    let request_end = pos + 4;
                    if request_end < buffer.len() {
                        fibre.http_parse_buffer.extend_from_slice(&buffer[request_end..]);
                    }
                }
                
                crate::vm::VM::push(&mut fibre, hyperion_core::memory::nan_box::Value::new_bool(true));
                fibre.resume();
                self.global_queue.push(fibre);
                for unparker in &self.unparkers {
                    unparker.unpark();
                }
                return;
            }
            
            static NEXT_FIBRE_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(100_000);

            let mut fibre = match self.idle_fibres.steal().success() {
                Some(mut f) => {
                    f.reset_for_worker_request(func_ptr);
                    f
                }

                None => {
                    let id = NEXT_FIBRE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let mut f = Fibre::new(id, func_ptr, std::sync::Arc::clone(&self.engine_state));
                    f.reset_for_worker_request(func_ptr);
                    f
                }
            };
            
            fibre.populate_http_superglobals(&req, body, &file_path);
            
            fibre.http_parse_buffer.clear();
            fibre.is_keep_alive_idle = false;
            fibre.tcp_stream = Some(stream);
            fibre.reactor_id = self.id;
            fibre.protocol = crate::fibre::RequestProtocol::Http;
            
            if let Some(pos) = buffer.windows(4).position(|w| w == b"\r\n\r\n") {
                let request_end = pos + 4;
                if request_end < buffer.len() {
                    fibre.http_parse_buffer.extend_from_slice(&buffer[request_end..]);
                }
            }
            
            fibre.resume();
            self.global_queue.push(fibre);
            
            for unparker in &self.unparkers {
                unparker.unpark();
            }
        }
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
                    crossbeam_deque::Steal::Success((mut stream, buffer)) => {
                        let token = Token(next_token_id);
                        next_token_id += 1;
                        if let Ok(_) = self.registry.register(&mut stream, token, Interest::READABLE) {
                            pending_streams.insert(token, (stream, buffer));
                        }
                    }
                    crossbeam_deque::Steal::Retry => continue,
                    crossbeam_deque::Steal::Empty => break,
                }
            }

            let poll_result = self.poll.lock().unwrap().poll(&mut events, Some(Duration::from_millis(1)));
            if poll_result.is_ok() {
                for event in events.iter() {
                    match event.token() {
                        
                        SAPI_LISTENER_TOKEN => {
                            if let Some(listener) = &*self.sapi_listener.lock().unwrap() {
                                loop {
                                    match listener.accept() {
                                        Ok((mut stream, _addr)) => {
                                            let _ = stream.set_nodelay(true);
                                            #[cfg(target_os = "macos")]
                                            unsafe {
                                                use std::os::unix::io::AsRawFd;
                                                let optval: libc::c_int = 1;
                                                libc::setsockopt(
                                                    stream.as_raw_fd(),
                                                    libc::SOL_SOCKET,
                                                    libc::SO_NOSIGPIPE,
                                                    &optval as *const _ as *const libc::c_void,
                                                    std::mem::size_of::<libc::c_int>() as libc::socklen_t,
                                                );
                                                let buf_size: libc::c_int = 262144;
                                                libc::setsockopt(
                                                    stream.as_raw_fd(),
                                                    libc::SOL_SOCKET,
                                                    libc::SO_SNDBUF,
                                                    &buf_size as *const _ as *const libc::c_void,
                                                    std::mem::size_of::<libc::c_int>() as libc::socklen_t,
                                                );
                                            }
                                            let mut buffer = Vec::new();
                                            let mut buf = [0; 4096];
                                            let mut is_blocked = false;
                                            loop {
                                                match stream.read(&mut buf) {
                                                    Ok(0) => break, // EOF
                                                    Ok(n) => {
                                                        buffer.extend_from_slice(&buf[..n]);
                                                    }
                                                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                                        is_blocked = true;
                                                        break;
                                                    }
                                                    Err(_e) => {
                                                        break;
                                                    }
                                                }
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
                                            if let Some((keep_alive, req_end)) = Self::try_serve_cached(&mut stream, &buffer) {
                                                if keep_alive {
                                                    buffer.drain(..req_end);
                                                    let token = Token(next_token_id);
                                                    next_token_id += 1;
                                                    if let Err(e) = self.registry.register(&mut stream, token, Interest::READABLE) {
                                                        eprintln!("Failed to register keep-alive stream: {}", e);
                                                    } else {
                                                        pending_streams.insert(token, (stream, buffer));
                                                    }
                                                } else {
                                                    use std::io::Write;
                                                    let _ = stream.flush();
                                                    let _ = stream.shutdown(std::net::Shutdown::Both);
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
                                            #[cfg(target_os = "macos")]
                                            unsafe {
                                                use std::os::unix::io::AsRawFd;
                                                let optval: libc::c_int = 1;
                                                libc::setsockopt(
                                                    stream.as_raw_fd(),
                                                    libc::SOL_SOCKET,
                                                    libc::SO_NOSIGPIPE,
                                                    &optval as *const _ as *const libc::c_void,
                                                    std::mem::size_of::<libc::c_int>() as libc::socklen_t,
                                                );
                                            }
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
                                                
                                                if let Some(mut fibre) = self.idle_octane_fibres.steal().success() {
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
                                for unparker in &self.unparkers {
                                    unparker.unpark();
                                }
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
