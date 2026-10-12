use crate::fibre::Fibre;
use crate::io::Reactor;
use crate::scheduler::WorkerContext;
use crate::vm::{ExecutionResult, YieldReason, VM};
use crossbeam_deque::{Injector, Stealer};
use mio::Token;
use std::fmt::Write as FmtWrite;
use std::io::Write;
use std::sync::Arc;

#[inline(always)]
fn write_vectored_all(stream: &mut mio::net::TcpStream, header: &[u8], body: &[u8]) {
    #[cfg(unix)]
    unsafe {
        use std::os::unix::io::AsRawFd;
        let fd = stream.as_raw_fd();
        let mut iov = [
            libc::iovec {
                iov_base: header.as_ptr() as *mut libc::c_void,
                iov_len: header.len(),
            },
            libc::iovec {
                iov_base: body.as_ptr() as *mut libc::c_void,
                iov_len: body.len(),
            },
        ];
        let mut retries = 0;
        while (iov[0].iov_len > 0 || iov[1].iov_len > 0) && retries < 100_000 {
            let (iov_ptr, iov_cnt) = if iov[0].iov_len > 0 {
                (iov.as_mut_ptr(), 2)
            } else {
                (iov.as_mut_ptr().add(1), 1)
            };
            let n = libc::writev(fd, iov_ptr, iov_cnt);
            if n > 0 {
                let mut written = n as usize;
                if iov[0].iov_len > 0 {
                    if written < iov[0].iov_len {
                        iov[0].iov_base = (iov[0].iov_base as *const u8).add(written) as *mut libc::c_void;
                        iov[0].iov_len -= written;
                        written = 0;
                    } else {
                        written -= iov[0].iov_len;
                        iov[0].iov_len = 0;
                    }
                }
                if written > 0 && iov[1].iov_len > 0 {
                    if written < iov[1].iov_len {
                        iov[1].iov_base = (iov[1].iov_base as *const u8).add(written) as *mut libc::c_void;
                        iov[1].iov_len -= written;
                    } else {
                        iov[1].iov_len = 0;
                    }
                }
                retries = 0;
            } else if n == -1 {
                let err = std::io::Error::last_os_error().raw_os_error();
                if err == Some(libc::EWOULDBLOCK) || err == Some(libc::EAGAIN) {
                    retries += 1;
                    std::thread::yield_now();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }
    #[cfg(not(unix))]
    {
        use std::io::Write;
        let _ = stream.write_all(header);
        let _ = stream.write_all(body);
        let _ = stream.flush();
    }
}

#[inline(always)]
fn try_read_next_http_request(stream: &mut mio::net::TcpStream, buffer: &mut Vec<u8>) -> Result<bool, ()> {
    use std::io::Read;
    let mut buf = [0u8; 8192];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => return Err(()), // EOF
            Ok(n) => {
                buffer.extend_from_slice(&buf[..n]);
                if buffer.len() > 16 * 1024 * 1024 {
                    return Err(());
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock || e.raw_os_error() == Some(35) || e.raw_os_error() == Some(11) => {
                break;
            }
            Err(_) => return Err(()),
        }
    }
    Ok(buffer.windows(4).any(|w| w == [13, 10, 13, 10]))
}



/// The Main Loop for a Worker Core
pub fn run_worker_loop(
    context: WorkerContext,
    global_queue: Arc<Injector<Fibre>>,
    stealers: Vec<Stealer<Fibre>>,
    reactors: Arc<Vec<Arc<Reactor>>>,
    idle_fibres: Arc<Injector<Fibre>>,
) {
    'worker_loop: loop {
        while let Ok((task_id, result)) = reactors[0].engine_state.db_completion_rx.try_recv() {
            if let Some((_, mut fibre)) = reactors[0].engine_state.parked_db_fibres.remove(&task_id) {
                match result {
                    Ok(val) => {
                        VM::push(&mut fibre, val);
                    }
                    Err(err_msg) => {
                        let _ = VM::create_and_throw_native_error(&mut fibre, &err_msg);
                    }
                }
                fibre.resume();
                global_queue.push(fibre);
                for unparker in &reactors[0].unparkers {
                    unparker.unpark();
                }
            } else {
                reactors[0].engine_state.pending_db_results.insert(task_id, result);
            }
        }

        if let Some(mut fibre) = context.find_task(&global_queue, &stealers) {
            if fibre.is_keep_alive_idle {
                if let Some(mut stream) = fibre.tcp_stream.take() {
                    let mut is_closed = false;
                    'keep_alive_burst: for _ in 0..64 {
                        loop {
                            let mut buf = [0; 8192];
                            match std::io::Read::read(&mut stream, &mut buf) {
                                Ok(0) => { is_closed = true; break; }
                                Ok(n) => {
                                    fibre.http_parse_buffer.extend_from_slice(&buf[..n]);
                                    let header_len = match fibre.http_parse_buffer.windows(4).position(|w| w == [13, 10, 13, 10]) {
                                        Some(pos) => pos + 4,
                                        None => fibre.http_parse_buffer.len(),
                                    };
                                    if header_len > 65_536 {
                                        use std::io::Write;
                                        let _ = stream.write_all(b"HTTP/1.1 431 Request Header Fields Too Large\r\nConnection: close\r\n\r\n");
                                        let _ = stream.flush();
                                        is_closed = true;
                                        break;
                                    }
                                }
                                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock || e.raw_os_error() == Some(35) || e.raw_os_error() == Some(11) => { break; }
                                Err(_) => { is_closed = true; break; }
                            }
                        }

                        if is_closed {
                            drop(stream);
                            fibre.is_keep_alive_idle = false;
                            if fibre.boot_checkpoint.is_some() {
                                reactors[fibre.reactor_id % reactors.len()].idle_octane_fibres.push(fibre);
                            } else {
                                idle_fibres.push(fibre);
                            }
                            continue 'worker_loop;
                        }

                        let pos_opt = fibre.http_parse_buffer.windows(4).position(|w| w == [13, 10, 13, 10]);
                        if let Some(pos) = pos_opt {
                            let request_end = pos + 4;
                            if request_end > 65_536 {
                                use std::io::Write;
                                let _ = stream.write_all(b"HTTP/1.1 431 Request Header Fields Too Large\r\nConnection: close\r\n\r\n");
                                let _ = stream.flush();
                                drop(stream);
                                fibre.http_parse_buffer.clear();
                                fibre.is_keep_alive_idle = false;
                                if fibre.boot_checkpoint.is_some() {
                                    reactors[fibre.reactor_id % reactors.len()].idle_octane_fibres.push(fibre);
                                } else {
                                    idle_fibres.push(fibre);
                                }
                                continue 'worker_loop;
                            }
                            let mut stack_buf = [0u8; 4096];
                            let heap_buf;
                            let req_bytes: &[u8] = if request_end <= 4096 {
                                stack_buf[..request_end].copy_from_slice(&fibre.http_parse_buffer[..request_end]);
                                &stack_buf[..request_end]
                            } else {
                                heap_buf = fibre.http_parse_buffer[..request_end].to_vec();
                                &heap_buf
                            };
                            let mut headers = [httparse::EMPTY_HEADER; 64];
                            let mut req = httparse::Request::new(&mut headers);
                            let parse_res = req.parse(req_bytes);
                            if let Ok(httparse::Status::Complete(_)) = parse_res {
                                let content_len = req.headers.iter()
                                    .find(|h| h.name.eq_ignore_ascii_case("content-length"))
                                    .and_then(|h| std::str::from_utf8(h.value).ok())
                                    .and_then(|s| s.trim().parse::<usize>().ok())
                                    .unwrap_or(0);

                                if content_len > 16 * 1024 * 1024 {
                                    use std::io::Write;
                                    let _ = stream.write_all(b"HTTP/1.1 413 Payload Too Large\r\nConnection: close\r\n\r\n");
                                    let _ = stream.flush();
                                    drop(stream);
                                    fibre.http_parse_buffer.clear();
                                    fibre.is_keep_alive_idle = false;
                                    if fibre.boot_checkpoint.is_some() {
                                        reactors[fibre.reactor_id % reactors.len()].idle_octane_fibres.push(fibre);
                                    } else {
                                        idle_fibres.push(fibre);
                                    }
                                    continue 'worker_loop;
                                }

                                let total_req_len = request_end + content_len;
                                if fibre.http_parse_buffer.len() < total_req_len {
                                    break 'keep_alive_burst;
                                }

                                let body_bytes = fibre.http_parse_buffer[request_end..total_req_len].to_vec();
                                let body = &body_bytes;
                                let path_str = req.path.unwrap_or("/");
                                let raw_clean_path = path_str.split("?").next().unwrap_or("/").trim_start_matches("/");
                                let clean_path = crate::io::url_decode(raw_clean_path);

                                let is_unsafe_path = clean_path.contains("..")
                                    || clean_path.contains('\0')
                                    || clean_path.contains('\\')
                                    || clean_path.split('/').any(|seg| seg.starts_with('.'));

                                let static_candidate = if clean_path.is_empty() || is_unsafe_path || !clean_path.contains('.') {
                                    None
                                } else {
                                    static CANON_DOCROOT: std::sync::OnceLock<Option<std::path::PathBuf>> = std::sync::OnceLock::new();
                                    let docroot_opt = CANON_DOCROOT.get_or_init(|| {
                                        if let Ok(c) = std::fs::canonicalize("public") {
                                            if c.is_dir() { return Some(c); }
                                        }
                                        std::fs::canonicalize(".").ok()
                                    });
                                    if let Some(docroot) = docroot_opt {
                                        let candidate = docroot.join(&*clean_path);
                                        if candidate.is_file() {
                                            if let Ok(canon_cand) = candidate.canonicalize() {
                                                if canon_cand.starts_with(docroot) {
                                                    Some(candidate)
                                                } else {
                                                    None
                                                }
                                            } else {
                                                None
                                            }
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    }
                                };

                                let (file_path_buf, is_static) = if let Some(cand) = static_candidate {
                                    let ext = cand.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                                    let is_sensitive = matches!(ext.as_str(), "php" | "phtml" | "php3" | "php4" | "php5" | "php7" | "php8" | "phps" | "inc" | "env" | "log" | "yml" | "yaml" | "json" | "lock");
                                    if !is_sensitive {
                                        (cand, true)
                                    } else {
                                        (std::path::PathBuf::from(reactors[0].routing_script.as_deref().unwrap_or("index.php")), false)
                                    }
                                } else if clean_path.is_empty() {
                                    (std::path::PathBuf::from(reactors[0].routing_script.as_deref().unwrap_or("index.php")), false)
                                } else {
                                    (std::path::PathBuf::from(reactors[0].routing_script.as_deref().unwrap_or(&clean_path)), false)
                                };

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
                                fibre.http_keep_alive = keep_alive;

                                let has_auth = req.headers.iter().any(|h| h.name.eq_ignore_ascii_case("authorization"));
                                let has_cookie = req.headers.iter().any(|h| h.name.eq_ignore_ascii_case("cookie"));
                                if !has_auth && !has_cookie && req.method.map(|m| m.eq_ignore_ascii_case("GET")).unwrap_or(true) {
                                    crate::io::check_and_invalidate_cache();
                                    let cache = crate::io::get_global_response_cache();
                                    let cached_opt = cache.get(path_str).or_else(|| {
                                        if path_str.starts_with('/') {
                                            cache.get(&path_str[1..])
                                        } else {
                                            cache.get(&format!("/{}", path_str))
                                        }
                                    });
                                    if let Some(cached) = cached_opt {
                                        let is_expired = cached.expires_at.map(|exp| std::time::Instant::now() > exp).unwrap_or(false);
                                        if is_expired {
                                            drop(cached);
                                            cache.remove(path_str);
                                            if path_str.starts_with('/') {
                                                cache.remove(&path_str[1..]);
                                            } else {
                                                cache.remove(&format!("/{}", path_str));
                                            }
                                        } else {
                                            let bytes = if fibre.http_keep_alive { &cached.keep_alive_bytes } else { &cached.close_bytes };
                                            use std::io::Write;
                                            let mut to_write = bytes.as_slice();
                                            while !to_write.is_empty() {
                                                match stream.write(to_write) {
                                                    Ok(0) => break,
                                                    Ok(n) => to_write = &to_write[n..],
                                                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock || e.raw_os_error() == Some(35) || e.raw_os_error() == Some(11) => {
                                                        std::thread::yield_now();
                                                    }
                                                    Err(_) => break,
                                                }
                                            }

                                            fibre.http_parse_buffer.drain(..total_req_len);
                                            if fibre.http_keep_alive {
                                                continue 'keep_alive_burst;
                                            } else {
                                                let _ = stream.flush();
                                                drop(stream);
                                                if fibre.boot_checkpoint.is_some() {
                                                    reactors[fibre.reactor_id % reactors.len()].idle_octane_fibres.push(fibre);
                                                } else {
                                                    idle_fibres.push(fibre);
                                                }
                                                continue 'worker_loop;
                                            }
                                        }
                                    }
                                }

                                if is_static {
                                    let candidate_file = &file_path_buf;
                                    if let Ok(file_bytes) = std::fs::read(candidate_file) {
                                        let ext = candidate_file.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                                        let content_type = crate::io::get_mime_type(&ext);
                                        let conn_header = if fibre.http_keep_alive { "keep-alive" } else { "close" };
                                        let response_header = format!(
                                            "HTTP/1.1 200 OK\r\nServer: PHP-Hyperion/1.0.0\r\nX-Content-Type-Options: nosniff\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: {}\r\n\r\n",
                                            content_type,
                                            file_bytes.len(),
                                            conn_header
                                        );

                                        write_vectored_all(&mut stream, response_header.as_bytes(), &file_bytes);

                                        fibre.http_parse_buffer.drain(..total_req_len);
                                        if fibre.http_keep_alive {
                                            continue 'keep_alive_burst;
                                        } else {
                                            let _ = stream.flush();
                                            if fibre.boot_checkpoint.is_some() {
                                                reactors[fibre.reactor_id % reactors.len()].idle_octane_fibres.push(fibre);
                                            } else {
                                                idle_fibres.push(fibre);
                                            }
                                            continue 'worker_loop;
                                        }
                                    }
                                }

                                if let Some(pool) = crate::ipc_worker_pool::get_global_ipc_pool() {
                                    let method_str = req.method.unwrap_or("GET");
                                    let query_str = path_str.split('?').nth(1).unwrap_or("");
                                    let mut req_headers = Vec::with_capacity(req.headers.len());
                                    for h in req.headers.iter() {
                                        let name = h.name;
                                        let val = std::str::from_utf8(h.value).unwrap_or("");
                                        req_headers.push((name, val));
                                    }

                                    let resp_res = pool.execute_request(
                                        method_str,
                                        path_str,
                                        &clean_path,
                                        query_str,
                                        &req_headers,
                                        body,
                                        None,
                                        8000,
                                    );

                                    match resp_res {
                                        Ok(resp) => {
                                            let conn_header = if fibre.http_keep_alive { "keep-alive" } else { "close" };
                                            let mut resp_header_bytes = Vec::with_capacity(512);
                                            let _ = write!(
                                                resp_header_bytes,
                                                "HTTP/1.1 {} {}\r\n",
                                                resp.status_code, resp.status_text
                                            );
                                            let mut has_content_type = false;
                                            let mut has_server = false;
                                            for (k, v) in &resp.headers {
                                                if k.eq_ignore_ascii_case("content-type") {
                                                    has_content_type = true;
                                                }
                                                if k.eq_ignore_ascii_case("content-length") || k.eq_ignore_ascii_case("connection") {
                                                    continue;
                                                }
                                                if k.eq_ignore_ascii_case("server") {
                                                    has_server = true;
                                                }
                                                let _ = write!(resp_header_bytes, "{}: {}\r\n", k, v);
                                            }
                                            if !has_content_type {
                                                resp_header_bytes.extend_from_slice(b"Content-Type: text/html; charset=UTF-8\r\n");
                                            }
                                            if !has_server {
                                                resp_header_bytes.extend_from_slice(b"Server: PHP-Hyperion/1.0.0 (Persistent IPC)\r\n");
                                            }
                                            let _ = write!(
                                                resp_header_bytes,
                                                "Content-Length: {}\r\nConnection: {}\r\n\r\n",
                                                resp.body.len(),
                                                conn_header
                                            );

                                            write_vectored_all(&mut stream, &resp_header_bytes, &resp.body);

                                            // RFC 9111 & Intelligent Gateway Micro-Caching
                                            if !method_str.eq_ignore_ascii_case("GET") && !method_str.eq_ignore_ascii_case("HEAD") && (200..=399).contains(&resp.status_code) {
                                                crate::io::clear_global_response_cache();
                                            }

                                            if resp.status_code == 200 && method_str.eq_ignore_ascii_case("GET") && !resp.body.is_empty() {
                                                let cache_control = resp.headers.iter()
                                                    .find(|(n, _)| n.eq_ignore_ascii_case("cache-control"))
                                                    .map(|(_, v)| v.to_ascii_lowercase())
                                                    .unwrap_or_default();
                                                let is_public = cache_control.contains("public");
                                                let has_no_store = cache_control.contains("no-store");
                                                let is_unshared = has_no_store || cache_control.contains("private") || cache_control.contains("no-cache");

                                                static MICROCACHE_ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
                                                let microcache_enabled = *MICROCACHE_ENABLED.get_or_init(|| {
                                                    std::env::var("HYPERION_MICROCACHE").map(|v| v != "0").unwrap_or(true)
                                                });

                                                static MICROCACHE_TTL_SECS: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
                                                let microcache_ttl_secs = *MICROCACHE_TTL_SECS.get_or_init(|| {
                                                    std::env::var("HYPERION_MICROCACHE_TTL").ok().and_then(|v| v.parse().ok()).unwrap_or(60)
                                                });

                                                let is_sensitive_route = path_str.starts_with("/admin") 
                                                    || path_str.starts_with("/login") 
                                                    || path_str.starts_with("/cart") 
                                                    || path_str.starts_with("/checkout") 
                                                    || path_str.starts_with("/auth") 
                                                    || path_str.starts_with("/logout") 
                                                    || path_str.starts_with("/register") 
                                                    || path_str.contains("/api/auth") 
                                                    || path_str.contains("/api/cart/checkout");

                                                let is_guest_request = !has_auth && !has_cookie;

                                                let should_rfc = is_public && !is_unshared;
                                                let should_microcache = microcache_enabled && is_guest_request && !has_no_store && !is_sensitive_route;

                                                if should_rfc || should_microcache {
                                                    let ttl = if should_rfc && !should_microcache {
                                                        let max_age = cache_control.split(',')
                                                            .find(|part| part.trim().starts_with("max-age="))
                                                            .and_then(|part| part.trim()["max-age=".len()..].parse::<u64>().ok());
                                                        max_age.map(std::time::Duration::from_secs)
                                                    } else {
                                                        Some(std::time::Duration::from_secs(microcache_ttl_secs))
                                                    };

                                                    let expires_at = ttl.map(|d| std::time::Instant::now() + d);

                                                    let mut cache_header_keep = Vec::with_capacity(512);
                                                    let mut cache_header_close = Vec::with_capacity(512);
                                                    let _ = write!(cache_header_keep, "HTTP/1.1 200 OK\r\n");
                                                    let _ = write!(cache_header_close, "HTTP/1.1 200 OK\r\n");
                                                    let mut has_ct = false;
                                                    for (k, v) in &resp.headers {
                                                        if k.eq_ignore_ascii_case("content-length") 
                                                            || k.eq_ignore_ascii_case("connection") 
                                                            || k.eq_ignore_ascii_case("set-cookie") 
                                                            || k.eq_ignore_ascii_case("status") {
                                                            continue;
                                                        }
                                                        if k.eq_ignore_ascii_case("content-type") {
                                                            has_ct = true;
                                                        }
                                                        let _ = write!(cache_header_keep, "{}: {}\r\n", k, v);
                                                        let _ = write!(cache_header_close, "{}: {}\r\n", k, v);
                                                    }
                                                    if !has_ct {
                                                        let ct = if resp.body.starts_with(b"{") || resp.body.starts_with(b"[") {
                                                            "application/json"
                                                        } else {
                                                            "text/html; charset=UTF-8"
                                                        };
                                                        let _ = write!(cache_header_keep, "Content-Type: {}\r\n", ct);
                                                        let _ = write!(cache_header_close, "Content-Type: {}\r\n", ct);
                                                    }
                                                    let _ = write!(cache_header_keep, "Content-Length: {}\r\nConnection: keep-alive\r\n\r\n", resp.body.len());
                                                    let _ = write!(cache_header_close, "Content-Length: {}\r\nConnection: close\r\n\r\n", resp.body.len());
                                                    let mut full_keep = cache_header_keep;
                                                    full_keep.extend_from_slice(&resp.body);
                                                    let mut full_close = cache_header_close;
                                                    full_close.extend_from_slice(&resp.body);

                                                    let cache_entry = crate::io::PreformattedHttpResponses {
                                                        keep_alive_bytes: Arc::new(full_keep),
                                                        close_bytes: Arc::new(full_close),
                                                        expires_at,
                                                    };
                                                    crate::io::get_global_response_cache().insert(path_str.to_string(), cache_entry.clone());
                                                    if !path_str.starts_with('/') {
                                                        crate::io::get_global_response_cache().insert(format!("/{}", path_str), cache_entry.clone());
                                                    }
                                                    let stripped = path_str.trim_start_matches('/');
                                                    if !stripped.is_empty() {
                                                        crate::io::get_global_response_cache().insert(stripped.to_string(), cache_entry);
                                                    }
                                                }
                                            }

                                            fibre.http_parse_buffer.drain(..total_req_len);
                                            if fibre.http_keep_alive {
                                                continue 'keep_alive_burst;
                                            } else {
                                                let _ = stream.flush();
                                                drop(stream);
                                                if fibre.boot_checkpoint.is_some() {
                                                    reactors[fibre.reactor_id % reactors.len()].idle_octane_fibres.push(fibre);
                                                } else {
                                                    idle_fibres.push(fibre);
                                                }
                                                continue 'worker_loop;
                                            }
                                        }
                                        Err(err) => {
                                            eprintln!("[IPC Worker Error] {}", err);
                                            let err_msg = "HTTP/1.1 502 Bad Gateway\r\nServer: PHP-Hyperion/1.0.0\r\nContent-Type: text/plain; charset=UTF-8\r\nContent-Length: 15\r\nConnection: close\r\n\r\n502 Bad Gateway";
                                            let _ = stream.write_all(err_msg.as_bytes());
                                            let _ = stream.flush();
                                            drop(stream);
                                            fibre.http_parse_buffer.clear();
                                            if fibre.boot_checkpoint.is_some() {
                                                reactors[fibre.reactor_id % reactors.len()].idle_octane_fibres.push(fibre);
                                            } else {
                                                idle_fibres.push(fibre);
                                            }
                                            continue 'worker_loop;
                                        }
                                    }
                                }

                                if fibre.boot_checkpoint.is_some() {
                                    fibre.output_buffer.clear();
                                    fibre.response_headers.clear();
                                    fibre.headers_sent = false;
                                    fibre.response_status_code = 200;
                                    fibre.populate_http_superglobals(&req, body, file_path_buf.to_str().unwrap_or("index.php"));
                                    fibre.http_parse_buffer.drain(..total_req_len);
                                    fibre.is_keep_alive_idle = false;
                                    crate::vm::VM::push(&mut fibre, hyperion_core::memory::nan_box::Value::new_bool(true));
                                    fibre.resume();
                                } else {
                                    let func_ptr = fibre.entry_func_ptr;
                                    fibre.reset_for_worker_request(func_ptr);
                                    fibre.populate_http_superglobals(&req, body, file_path_buf.to_str().unwrap_or("index.php"));
                                    fibre.http_parse_buffer.drain(..total_req_len);
                                    fibre.is_keep_alive_idle = false;
                                }
                                break 'keep_alive_burst;
                            } else {
                                use std::io::Write;
                                let _ = stream.write_all(b"HTTP/1.1 400 Bad Request\r\nConnection: close\r\n\r\n");
                                let _ = stream.flush();
                                drop(stream);
                                fibre.http_parse_buffer.clear();
                                fibre.is_keep_alive_idle = false;
                                if fibre.boot_checkpoint.is_some() {
                                    reactors[fibre.reactor_id % reactors.len()].idle_octane_fibres.push(fibre);
                                } else {
                                    idle_fibres.push(fibre);
                                }
                                continue 'worker_loop;
                            }
                        } else {
                            if fibre.http_parse_buffer.len() > 65_536 {
                                use std::io::Write;
                                let _ = stream.write_all(b"HTTP/1.1 431 Request Header Fields Too Large\r\nConnection: close\r\n\r\n");
                                let _ = stream.flush();
                                drop(stream);
                                fibre.http_parse_buffer.clear();
                                fibre.is_keep_alive_idle = false;
                                if fibre.boot_checkpoint.is_some() {
                                    reactors[fibre.reactor_id % reactors.len()].idle_octane_fibres.push(fibre);
                                } else {
                                    idle_fibres.push(fibre);
                                }
                                continue 'worker_loop;
                            }
                            break 'keep_alive_burst;
                        }
                    }

                    fibre.tcp_stream = Some(stream);
                    if fibre.is_keep_alive_idle {
                        reactors[fibre.reactor_id % reactors.len()].register_fibre(mio::Token(fibre.id as usize), fibre, None);
                        continue 'worker_loop;
                    }
                }
            }

            let exec_res = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                VM::run_fibre(&mut fibre)
            })) {
                Ok(res) => res,
                Err(panic_err) => {
                    let msg = if let Some(s) = panic_err.downcast_ref::<&str>() {
                        s.to_string()
                    } else if let Some(s) = panic_err.downcast_ref::<String>() {
                        s.clone()
                    } else {
                        "Worker panic during execution".to_string()
                    };
                    eprintln!("Fibre {} Worker Panic: {}", fibre.id, msg);
                    ExecutionResult::Error(msg)
                }
            };
            match exec_res {


                ExecutionResult::Finished => {
                    if std::env::var("HYPERION_DEBUG").is_ok() {
                        eprintln!("[DEBUG] Fibre {} Finished (frame_count={})", fibre.id, fibre.frame_count);
                    }
                    if fibre.frame_count > 0 {
                        context.local_queue.push(fibre);
                        continue;
                    }
                    fibre.flush_all_ob_buffers();
                    if let Some(mut stream) = fibre.tcp_stream.take() {
                        match fibre.protocol {
                            crate::fibre::RequestProtocol::Http => {
                                if fibre.headers_sent {
                                    if !fibre.output_buffer.is_empty() {
                                        let chunk_header = format!("{:X}\r\n", fibre.output_buffer.len());
                                        let _ = stream.write_all(chunk_header.as_bytes());
                                        let _ = stream.write_all(&fibre.output_buffer);
                                        let _ = stream.write_all(b"\r\n");

                                    }
                                    
                                    let mut data = b"0\r\n\r\n".as_slice();
                                    while !data.is_empty() {
                                        match std::io::Write::write(&mut stream, data) {
                                            Ok(0) => break,
                                            Ok(n) => data = &data[n..],
                                            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock || e.raw_os_error() == Some(35) || e.raw_os_error() == Some(11) => {
                                                std::thread::yield_now();
                                            }
                                            Err(_) => break,
                                        }
                                    }
                                } else {
                                      let status_code = fibre.response_status_code;
                                      let status_text = match status_code {
                                          200 => "OK",
                                          201 => "Created",
                                          202 => "Accepted",
                                          204 => "No Content",
                                          301 => "Moved Permanently",
                                          302 => "Found",
                                          303 => "See Other",
                                          304 => "Not Modified",
                                          307 => "Temporary Redirect",
                                          308 => "Permanent Redirect",
                                          400 => "Bad Request",
                                          401 => "Unauthorized",
                                          403 => "Forbidden",
                                          404 => "Not Found",
                                          405 => "Method Not Allowed",
                                          419 => "Page Expired",
                                          422 => "Unprocessable Entity",
                                          429 => "Too Many Requests",
                                          500 => "Internal Server Error",
                                          502 => "Bad Gateway",
                                          503 => "Service Unavailable",
                                          _ => "OK",
                                      };

                                       let mut response_header = String::with_capacity(512);
                                       let _ = write!(response_header, "HTTP/1.1 {} {}\r\n", status_code, status_text);
                                       let mut has_content_type = false;
                                       for (name, val) in &fibre.response_headers {
                                           if name.eq_ignore_ascii_case("content-type") {
                                               has_content_type = true;
                                           }
                                           if name.eq_ignore_ascii_case("content-length") {
                                               continue;
                                           }
                                           let _ = write!(response_header, "{}: {}\r\n", name, val);
                                       }
                                       if !has_content_type {
                                           let content_type = if fibre.output_buffer.starts_with(b"{") || fibre.output_buffer.starts_with(b"[") {
                                               "application/json"
                                           } else {
                                               "text/html; charset=UTF-8"
                                           };
                                           let _ = write!(response_header, "Content-Type: {}\r\n", content_type);
                                       }
                                       let conn_header = if fibre.http_keep_alive { "keep-alive" } else { "close" };
                                       let _ = write!(response_header, "Content-Length: {}\r\nConnection: {}\r\n\r\n", fibre.output_buffer.len(), conn_header);
                                          
                                       write_vectored_all(&mut stream, response_header.as_bytes(), &fibre.output_buffer);

                                          if fibre.response_status_code == 200 && fibre.http_request_method == "GET" && !fibre.output_buffer.is_empty() {
                                              let cache_control = fibre.response_headers.iter()
                                                  .find(|(n, _)| n.eq_ignore_ascii_case("cache-control"))
                                                  .map(|(_, v)| v.to_ascii_lowercase())
                                                  .unwrap_or_default();
                                              
                                              // RFC 9111 Strict Compliance: Only cache responses explicitly marked as "public"
                                              // and never store responses marked with "no-store", "no-cache", or "private".
                                              let is_public = cache_control.contains("public");
                                              let is_unshared = cache_control.contains("no-store")
                                                  || cache_control.contains("private")
                                                  || cache_control.contains("no-cache");

                                              if is_public && !is_unshared {
                                                  let mut cache_header = String::with_capacity(512);
                                                  let _ = write!(cache_header, "HTTP/1.1 {} {}\r\n", status_code, status_text);
                                                  let mut has_content_type = false;
                                                  for (name, val) in &fibre.response_headers {
                                                      if name.eq_ignore_ascii_case("content-type") {
                                                          has_content_type = true;
                                                      }
                                                      // RFC 9111 & RFC 6265: Shared cache MUST NEVER store Set-Cookie headers
                                                      // Also skip hop-by-hop framing headers.
                                                      if name.eq_ignore_ascii_case("content-length") 
                                                          || name.eq_ignore_ascii_case("set-cookie") 
                                                          || name.eq_ignore_ascii_case("connection")
                                                          || name.eq_ignore_ascii_case("status") {
                                                          continue;
                                                      }
                                                      let _ = write!(cache_header, "{}: {}\r\n", name, val);
                                                  }
                                                  if !has_content_type {
                                                      let content_type = if fibre.output_buffer.starts_with(b"{") || fibre.output_buffer.starts_with(b"[") {
                                                          "application/json"
                                                      } else {
                                                          "text/html; charset=UTF-8"
                                                      };
                                                      let _ = write!(cache_header, "Content-Type: {}\r\n", content_type);
                                                  }
                                                  let ka_header = format!("{}Content-Length: {}\r\nConnection: keep-alive\r\n\r\n", cache_header, fibre.output_buffer.len());
                                                  let cl_header = format!("{}Content-Length: {}\r\nConnection: close\r\n\r\n", cache_header, fibre.output_buffer.len());

                                                  let mut ka_bytes = Vec::with_capacity(ka_header.len() + fibre.output_buffer.len());
                                                  ka_bytes.extend_from_slice(ka_header.as_bytes());
                                                  ka_bytes.extend_from_slice(&fibre.output_buffer);
                                                  let mut cl_bytes = Vec::with_capacity(cl_header.len() + fibre.output_buffer.len());
                                                  cl_bytes.extend_from_slice(cl_header.as_bytes());
                                                  cl_bytes.extend_from_slice(&fibre.output_buffer);
                                                  let entry = crate::io::PreformattedHttpResponses {
                                                      keep_alive_bytes: std::sync::Arc::new(ka_bytes),
                                                      close_bytes: std::sync::Arc::new(cl_bytes),
                                                      expires_at: None,
                                                  };
                                                  crate::io::get_global_response_cache().insert(fibre.http_request_uri.clone(), entry.clone());
                                                  if !fibre.http_request_uri.starts_with('/') {
                                                      crate::io::get_global_response_cache().insert(format!("/{}", fibre.http_request_uri), entry.clone());
                                                  }
                                                  let stripped = fibre.http_request_uri.trim_start_matches('/');
                                                  if !stripped.is_empty() {
                                                      crate::io::get_global_response_cache().insert(stripped.to_string(), entry);
                                                  }
                                              }
                                          }
                                  }
                                
                                if std::env::var("HYPERION_VERBOSE").is_ok() {
                                     let now = chrono::Local::now().format("%a %b %e %H:%M:%S %Y");
                                     if let Ok(peer_addr) = stream.peer_addr() {
                                         eprintln!("[{}] {} [{}]: {} {}", now, peer_addr, fibre.response_status_code, fibre.http_request_method, fibre.http_request_uri);
                                     }
                                }

                                if std::env::var("HYPERION_STATS").is_ok() {
                                    eprintln!("[STATS] id: {}, arena: {}B (drop: {}), classes: {}, opcache: {}, global_arena: {}B (drop: {})",
                                        fibre.id,
                                        fibre.arena.allocated_bytes(),
                                        fibre.arena.drop_list_len(),
                                        fibre.engine_state.classes.len(),
                                        fibre.engine_state.opcache.len(),
                                        fibre.engine_state.global_arena.allocated_bytes(),
                                        fibre.engine_state.global_arena.drop_list_len(),
                                    );
                                }
                                if fibre.http_keep_alive {
                                    if fibre.boot_checkpoint.is_some() {
                                        fibre.reset_to_boot_checkpoint();
                                    } else {
                                        fibre.reset_for_worker_request(fibre.entry_func_ptr);
                                    }
                                    let mut has_req = fibre.http_parse_buffer.windows(4).any(|w| w == [13, 10, 13, 10]);
                                    if !has_req {
                                        match try_read_next_http_request(&mut stream, &mut fibre.http_parse_buffer) {
                                            Ok(true) => has_req = true,
                                            Ok(false) => {
                                                for _ in 0..10 {
                                                    std::thread::yield_now();
                                                    if let Ok(true) = try_read_next_http_request(&mut stream, &mut fibre.http_parse_buffer) {
                                                        has_req = true;
                                                        break;
                                                    }
                                                }
                                            }
                                            Err(_) => {
                                                drop(stream);
                                                if fibre.boot_checkpoint.is_some() {
                                                    reactors[fibre.reactor_id % reactors.len()].idle_octane_fibres.push(fibre);
                                                } else {
                                                    idle_fibres.push(fibre);
                                                }
                                                continue 'worker_loop;
                                            }
                                        }
                                    }

                                    if has_req {
                                        fibre.tcp_stream = Some(stream);
                                        fibre.is_keep_alive_idle = true;
                                        context.local_queue.push(fibre);
                                        continue 'worker_loop;
                                    } else {
                                        let reactor = &reactors[fibre.reactor_id % reactors.len()];
                                        let leftover = std::mem::take(&mut fibre.http_parse_buffer);
                                        reactor.recycle_keep_alive_stream_with_buffer(stream, leftover);
                                        if fibre.boot_checkpoint.is_some() {
                                            reactor.idle_octane_fibres.push(fibre);
                                        } else {
                                            idle_fibres.push(fibre);
                                        }
                                        continue 'worker_loop;
                                    }
                                } else {
                                    let _ = stream.flush();
                                    drop(stream);
                                    if fibre.boot_checkpoint.is_some() {
                                        fibre.reset_to_boot_checkpoint();
                                        reactors[fibre.reactor_id % reactors.len()].idle_octane_fibres.push(fibre);
                                    } else {
                                        fibre.reset_for_worker_request(fibre.entry_func_ptr);
                                        idle_fibres.push(fibre);
                                    }
                                }
                            }
                            crate::fibre::RequestProtocol::FastCgi { request_id, keep_conn } => {
                                let _ = crate::io::write_fastcgi_response(&mut stream, request_id, &fibre.output_buffer);
                                if !keep_conn {
                                    let _ = stream.shutdown(std::net::Shutdown::Both);
                                }
                                if fibre.boot_checkpoint.is_some() {
                                    fibre.reset_to_boot_checkpoint();
                                    reactors[fibre.reactor_id % reactors.len()].idle_octane_fibres.push(fibre);
                                } else {
                                    fibre.reset_for_worker_request(fibre.entry_func_ptr);
                                    idle_fibres.push(fibre);
                                }
                            }
                        }
                    } else {
                        // CLI mode: write directly to stdout
                        let _ = std::io::stdout().write_all(&fibre.output_buffer);
                        let _ = std::io::stdout().flush();
                        if fibre.boot_checkpoint.is_some() {
                            fibre.reset_to_boot_checkpoint();
                            reactors[fibre.reactor_id % reactors.len()].idle_octane_fibres.push(fibre);
                        } else {
                            fibre.reset_for_worker_request(fibre.entry_func_ptr);
                            idle_fibres.push(fibre);
                        }
                    }
                }
                ExecutionResult::Yielded(YieldReason::OutputFlush) => {
                    if let Some(mut stream) = fibre.tcp_stream.take() {
                        match fibre.protocol {
                            crate::fibre::RequestProtocol::Http => {
                                if !fibre.headers_sent {
                                    let header = "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n";
                                    let _ = stream.write_all(header.as_bytes());
                                    fibre.headers_sent = true;
                                }
                                let chunk_header = format!("{:X}\r\n", fibre.output_buffer.len());
                                let _ = stream.write_all(chunk_header.as_bytes());
                                let _ = stream.write_all(&fibre.output_buffer);
                                let _ = stream.write_all(b"\r\n");
                            }
                            crate::fibre::RequestProtocol::FastCgi { request_id, .. } => {
                                let _ = crate::io::write_fastcgi_response(&mut stream, request_id, &fibre.output_buffer);
                            }
                        }
                        fibre.tcp_stream = Some(stream);
                    } else {
                        // CLI mode
                        let _ = std::io::stdout().write_all(&fibre.output_buffer);
                        let _ = std::io::stdout().flush();
                    }
                    fibre.output_buffer.clear();
                    idle_fibres.push(fibre);
                }
                ExecutionResult::Yielded(YieldReason::Network)
                | ExecutionResult::Yielded(YieldReason::Disk) => {
                    // Suspend the fibre and let Reactor wake it up later.
                    reactors[fibre.reactor_id % reactors.len()].register_fibre(Token(fibre.id as usize), fibre, None);
                }
                ExecutionResult::Yielded(YieldReason::Database(task_id)) => {
                    fibre.state = crate::fibre::FibreState::Suspended;
                    reactors[0].engine_state.parked_db_fibres.insert(task_id, fibre);
                    if let Some((_, result)) = reactors[0].engine_state.pending_db_results.remove(&task_id) {
                        if let Some((_, mut fibre)) = reactors[0].engine_state.parked_db_fibres.remove(&task_id) {
                            match result {
                                Ok(val) => {
                                    VM::push(&mut fibre, val);
                                }
                                Err(err_msg) => {
                                    let _ = VM::create_and_throw_native_error(&mut fibre, &err_msg);
                                }
                            }
                            fibre.resume();
                            global_queue.push(fibre);
                            for unparker in &reactors[0].unparkers {
                                unparker.unpark();
                            }
                        }
                    }
                }
                ExecutionResult::Yielded(YieldReason::AsyncInclude(path)) => {
                    let global_queue_clone = global_queue.clone();
                    let engine_state = fibre.engine_state.clone();
                    let unparkers_clone = reactors[0].unparkers.clone();
                    let _ = std::thread::Builder::new()
                        .name("hyperion-async-include".to_string())
                        .stack_size(16 * 1024 * 1024)
                        .spawn(move || {
                            match engine_state.compile_and_load_script(&path) {
                                Ok(_) => {
                                    fibre.resume();
                                    global_queue_clone.push(fibre);
                                    for unparker in &unparkers_clone {
                                        unparker.unpark();
                                    }
                                }

                            Err(err) => {
                                eprintln!("Fatal Error: require(): Failed compiling required '{}': {}", path, err);
                                if let Some(mut stream) = fibre.tcp_stream.take() {
                                    match fibre.protocol {
                                        crate::fibre::RequestProtocol::Http => {
                                            let response = format!("HTTP/1.1 500 Internal Server Error\r\n\r\nFatal Error: require(): Failed compiling required '{}': {}", path, err);
                                            let _ = stream.write_all(response.as_bytes());
                                        }
                                        crate::fibre::RequestProtocol::FastCgi { request_id, .. } => {
                                            let _ = crate::io::write_fastcgi_error(&mut stream, request_id, &format!("Fatal Error: require(): Failed compiling required '{}': {}", path, err));
                                        }
                                    }
                                }
                            }
                        }
                    });
                }
                ExecutionResult::Yielded(YieldReason::WaitForHttpRequest) => {
                    fibre.flush_all_ob_buffers();
                    if let Some(mut stream) = fibre.tcp_stream.take() {
                        match fibre.protocol {
                            crate::fibre::RequestProtocol::Http => {
                                if fibre.headers_sent {
                                    if !fibre.output_buffer.is_empty() {
                                        let chunk_header = format!("{:X}\r\n", fibre.output_buffer.len());
                                        let _ = stream.write_all(chunk_header.as_bytes());
                                        let _ = stream.write_all(&fibre.output_buffer);
                                        let _ = stream.write_all(b"\r\n");
                                    }
                                    let _ = stream.write_all(b"0\r\n\r\n");
                                } else {
                                    let status_code = fibre.response_status_code;
                                    let status_text = match status_code {
                                        200 => "OK",
                                        201 => "Created",
                                        202 => "Accepted",
                                        204 => "No Content",
                                        301 => "Moved Permanently",
                                        302 => "Found",
                                        400 => "Bad Request",
                                        401 => "Unauthorized",
                                        403 => "Forbidden",
                                        404 => "Not Found",
                                        405 => "Method Not Allowed",
                                        422 => "Unprocessable Entity",
                                        500 => "Internal Server Error",
                                        _ => "OK",
                                    };
                                    let mut response_headers = String::with_capacity(512);
                                    let _ = write!(response_headers, "HTTP/1.1 {} {}\r\n", status_code, status_text);
                                    let mut has_content_type = false;
                                    for (key, val) in &fibre.response_headers {
                                        if key.eq_ignore_ascii_case("content-type") {
                                            has_content_type = true;
                                        }
                                        if key.eq_ignore_ascii_case("content-length") {
                                            continue;
                                        }
                                        let _ = write!(response_headers, "{}: {}\r\n", key, val);
                                    }
                                    if !has_content_type {
                                        response_headers.push_str("Content-Type: text/html; charset=UTF-8\r\n");
                                    }
                                    let is_keep_alive = fibre.http_keep_alive;
                                    let conn_header = if is_keep_alive { "keep-alive" } else { "close" };
                                    let _ = write!(response_headers, "Content-Length: {}\r\nConnection: {}\r\n\r\n", fibre.output_buffer.len(), conn_header);
                                    write_vectored_all(&mut stream, response_headers.as_bytes(), &fibre.output_buffer);
                                }
                            }
                            crate::fibre::RequestProtocol::FastCgi { request_id, .. } => {
                                let _ = crate::io::write_fastcgi_response(&mut stream, request_id, &fibre.output_buffer);
                            }
                        }
                        let _ = stream.flush();

                        let is_keep_alive = fibre.http_keep_alive;
                        fibre.output_buffer.clear();
                        fibre.response_headers.clear();
                        fibre.headers_sent = false;
                        fibre.response_status_code = 200;
                        fibre.autoload_state.clear();
                        fibre.autoload_frame_depths.clear();

                        if is_keep_alive {
                            if fibre.boot_checkpoint.is_some() {
                                fibre.reset_to_boot_checkpoint();
                            }
                            let mut has_req = fibre.http_parse_buffer.windows(4).any(|w| w == [13, 10, 13, 10]);
                            if !has_req {
                                match try_read_next_http_request(&mut stream, &mut fibre.http_parse_buffer) {
                                    Ok(true) => has_req = true,
                                    Ok(false) => {
                                        for _ in 0..10 {
                                            std::thread::yield_now();
                                            if let Ok(true) = try_read_next_http_request(&mut stream, &mut fibre.http_parse_buffer) {
                                                has_req = true;
                                                break;
                                            }
                                        }
                                    }
                                    Err(_) => {
                                        drop(stream);
                                        if fibre.boot_checkpoint.is_some() {
                                            reactors[fibre.reactor_id % reactors.len()].idle_octane_fibres.push(fibre);
                                        } else {
                                            idle_fibres.push(fibre);
                                        }
                                        continue 'worker_loop;
                                    }
                                }
                            }

                            if has_req {
                                fibre.tcp_stream = Some(stream);
                                fibre.is_keep_alive_idle = true;
                                context.local_queue.push(fibre);
                                continue 'worker_loop;
                            } else {
                                let reactor = &reactors[fibre.reactor_id % reactors.len()];
                                let leftover = std::mem::take(&mut fibre.http_parse_buffer);
                                reactor.recycle_keep_alive_stream_with_buffer(stream, leftover);
                                if fibre.boot_checkpoint.is_some() {
                                    reactor.idle_octane_fibres.push(fibre);
                                } else {
                                    idle_fibres.push(fibre);
                                }
                            }
                        } else {
                            drop(stream);
                            if fibre.boot_checkpoint.is_some() {
                                fibre.reset_to_boot_checkpoint();
                                reactors[fibre.reactor_id % reactors.len()].idle_octane_fibres.push(fibre);
                            } else {
                                idle_fibres.push(fibre);
                            }
                        }
                    }
                }
                ExecutionResult::Yielded(YieldReason::GeneratorYield) => {
                    // Top-level generator execution yielded. Generators are iterated by parent fibres,
                    // so we do not push them back to the ready queue.
                }
                ExecutionResult::Preempted => {
                    // Out of Gas (Time slice expired), put it back in the queue
                    fibre.resume();
                    global_queue.push(fibre);
                    for unparker in &reactors[0].unparkers {
                        unparker.unpark();
                    }
                }
                ExecutionResult::Error(e) => {
                    eprintln!("Fibre {} Error: {}", fibre.id, e);
                    if let Some(mut stream) = fibre.tcp_stream.take() {
                        match fibre.protocol {
                            crate::fibre::RequestProtocol::Http => {
                                let body = format!("{}", e);
                                let response = format!("HTTP/1.1 500 Internal Server Error\r\nContent-Type: text/plain; charset=UTF-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body);
                                let _ = stream.write_all(response.as_bytes());
                                let _ = stream.flush();
                                let _ = stream.shutdown(std::net::Shutdown::Both);
                            }
                            crate::fibre::RequestProtocol::FastCgi { request_id, .. } => {
                                let _ = crate::io::write_fastcgi_error(&mut stream, request_id, &e);
                                let _ = stream.shutdown(std::net::Shutdown::Both);
                            }
                        }
                    }
                    if fibre.boot_checkpoint.is_some() {
                        fibre.reset_to_boot_checkpoint();
                        reactors[fibre.reactor_id % reactors.len()].idle_octane_fibres.push(fibre);
                    } else {
                        fibre.reset_for_worker_request(fibre.entry_func_ptr);
                        idle_fibres.push(fibre);
                    }
                }
                ExecutionResult::UncaughtException(e) => {
                    eprintln!("Fibre {} Uncaught Exception: {}", fibre.id, e);
                    if let Some(mut stream) = fibre.tcp_stream.take() {
                        match fibre.protocol {
                            crate::fibre::RequestProtocol::Http => {
                                let body = format!("Uncaught Exception: {}", e);
                                let response = format!(
                                    "HTTP/1.1 500 Internal Server Error\r\nContent-Type: text/plain; charset=UTF-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                    body.len(),
                                    body
                                );
                                let _ = stream.write_all(response.as_bytes());
                                let _ = stream.flush();
                                let _ = stream.shutdown(std::net::Shutdown::Both);
                            }
                            crate::fibre::RequestProtocol::FastCgi { request_id, .. } => {
                                let _ = crate::io::write_fastcgi_error(&mut stream, request_id, &format!("Uncaught Exception: {}", e));
                                let _ = stream.shutdown(std::net::Shutdown::Both);
                            }
                        }
                    }
                    if fibre.boot_checkpoint.is_some() {
                        fibre.reset_to_boot_checkpoint();
                        reactors[fibre.reactor_id % reactors.len()].idle_octane_fibres.push(fibre);
                    } else {
                        fibre.reset_for_worker_request(fibre.entry_func_ptr);
                        idle_fibres.push(fibre);
                    }
                }
            }
        } else {
            context.parker.park_timeout(std::time::Duration::from_millis(1));
        }
    }
}

