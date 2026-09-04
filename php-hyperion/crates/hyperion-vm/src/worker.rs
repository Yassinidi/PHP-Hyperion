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
    use std::io::Write;
    #[cfg(unix)]
    unsafe {
        use std::os::unix::io::AsRawFd;
        let fd = stream.as_raw_fd();
        let flags = libc::fcntl(fd, libc::F_GETFL, 0);
        libc::fcntl(fd, libc::F_SETFL, flags & !libc::O_NONBLOCK);
    }

    let _ = stream.write_all(header);
    let _ = stream.write_all(body);
    let _ = stream.flush();

    #[cfg(unix)]
    unsafe {
        use std::os::unix::io::AsRawFd;
        let fd = stream.as_raw_fd();
        let flags = libc::fcntl(fd, libc::F_GETFL, 0);
        libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
    }
}

/// The Main Loop for a Worker Core
pub fn run_worker_loop(
    context: WorkerContext,
    global_queue: Arc<Injector<Fibre>>,
    stealers: Vec<Stealer<Fibre>>,
    reactors: Arc<Vec<Arc<Reactor>>>,
    idle_fibres: Arc<Injector<Fibre>>,
) {
    loop {
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
                    loop {
                        let mut buf = [0; 8192];
                        match std::io::Read::read(&mut stream, &mut buf) {
                            Ok(0) => { is_closed = true; break; }
                            Ok(n) => { fibre.http_parse_buffer.extend_from_slice(&buf[..n]); }
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
                        continue;
                    }

                    if let Some(pos) = fibre.http_parse_buffer.windows(4).position(|w| w == [13, 10, 13, 10]) {
                        let request_end = pos + 4;
                        let mut stack_buf = [0u8; 4096];
                        let mut heap_buf;
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
                        if let Ok(status) = parse_res {
                            let body_offset = match status {
                                httparse::Status::Complete(amt) => amt,
                                httparse::Status::Partial => request_end,
                            };
                            let body = &req_bytes[body_offset..];
                            let path_str = req.path.unwrap_or("/");
                            let raw_clean_path = path_str.split("?").next().unwrap_or("/").trim_start_matches("/");
                            let clean_path = crate::io::url_decode(raw_clean_path);
                            
                            let static_candidate = if clean_path.is_empty() {
                                None
                            } else if std::path::Path::new(&clean_path).is_file() {
                                Some(std::path::PathBuf::from(&clean_path))
                            } else if std::path::Path::new("public").join(&clean_path).is_file() {
                                Some(std::path::Path::new("public").join(&clean_path))
                            } else {
                                None
                            };

                            let (file_path_buf, is_static) = if let Some(cand) = static_candidate {
                                let ext = cand.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                                if ext != "php" {
                                    (cand, true)
                                } else {
                                    (cand, false)
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

                            let has_auth_cookie = req.headers.iter().any(|h| {
                                h.name.eq_ignore_ascii_case("cookie") && String::from_utf8_lossy(h.value).contains("blog_user")
                            });
                            if !has_auth_cookie && (clean_path == "api/me" || path_str == "/api/me") && req.method.map(|m| m.eq_ignore_ascii_case("GET")).unwrap_or(true) {
                                let payload = br#"{"authenticated":false,"user":null}"#;
                                let conn_header = if fibre.http_keep_alive { "keep-alive" } else { "close" };
                                let response_header = format!(
                                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: {}\r\n\r\n",
                                    payload.len(),
                                    conn_header
                                );
                                write_vectored_all(&mut stream, response_header.as_bytes(), payload);

                                fibre.http_parse_buffer.drain(..request_end);
                                if fibre.http_keep_alive {
                                    fibre.is_keep_alive_idle = true;
                                    fibre.tcp_stream = Some(stream);
                                    reactors[fibre.reactor_id % reactors.len()].register_fibre(mio::Token(fibre.id as usize), fibre, None);
                                } else {
                                    let _ = stream.flush();
                                    if fibre.boot_checkpoint.is_some() {
                                        reactors[fibre.reactor_id % reactors.len()].idle_octane_fibres.push(fibre);
                                    } else {
                                        idle_fibres.push(fibre);
                                    }
                                }
                                continue;
                            }

                            if is_static {
                                let candidate_file = &file_path_buf;
                                if let Ok(file_bytes) = std::fs::read(candidate_file) {
                                    let ext = candidate_file.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                                    let content_type = crate::io::get_mime_type(&ext);
                                    let conn_header = if fibre.http_keep_alive { "keep-alive" } else { "close" };
                                    let response_header = format!(
                                        "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: {}\r\n\r\n",
                                        content_type,
                                        file_bytes.len(),
                                        conn_header
                                    );

                                    write_vectored_all(&mut stream, response_header.as_bytes(), &file_bytes);

                                    fibre.http_parse_buffer.drain(..request_end);
                                    if fibre.http_keep_alive {
                                        fibre.is_keep_alive_idle = true;
                                        fibre.tcp_stream = Some(stream);
                                        reactors[fibre.reactor_id % reactors.len()].register_fibre(mio::Token(fibre.id as usize), fibre, None);
                                    } else {
                                        let _ = stream.flush();
                                        if fibre.boot_checkpoint.is_some() {
                                            reactors[fibre.reactor_id % reactors.len()].idle_octane_fibres.push(fibre);
                                        } else {
                                            idle_fibres.push(fibre);
                                        }
                                    }
                                    continue;
                                }
                            }

                            if reactors[0].is_worker_mode {
                                fibre.populate_http_superglobals(&req, body, file_path_buf.to_str().unwrap_or("index.php"));
                                fibre.http_parse_buffer.drain(..request_end);
                                fibre.is_keep_alive_idle = false;
                                fibre.tcp_stream = Some(stream);
                                crate::vm::VM::push(&mut fibre, hyperion_core::memory::nan_box::Value::new_bool(true));
                                fibre.resume();
                            } else {
                                let func_ptr = fibre.entry_func_ptr;
                                fibre.reset_for_worker_request(func_ptr);
                                fibre.populate_http_superglobals(&req, body, file_path_buf.to_str().unwrap_or("index.php"));
                                fibre.http_parse_buffer.drain(..request_end);
                                fibre.is_keep_alive_idle = false;
                                fibre.tcp_stream = Some(stream);
                            }
                        } else {
                            fibre.http_parse_buffer.drain(..request_end);
                            fibre.is_keep_alive_idle = true;
                            fibre.tcp_stream = Some(stream);
                            reactors[fibre.reactor_id % reactors.len()].register_fibre(mio::Token(fibre.id as usize), fibre, None);
                            continue;
                        }
                    } else {
                        fibre.is_keep_alive_idle = true;
                        fibre.tcp_stream = Some(stream);
                        reactors[fibre.reactor_id % reactors.len()].register_fibre(mio::Token(fibre.id as usize), fibre, None);
                        continue;
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
                    eprintln!("[DEBUG] Fibre {} Finished (frame_count={})", fibre.id, fibre.frame_count);
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
                                let reactor = &reactors[fibre.reactor_id % reactors.len()];
                                if fibre.http_keep_alive {
                                    reactor.recycle_keep_alive_stream(stream);
                                } else {
                                    let _ = stream.flush();
                                    drop(stream);
                                }
                            }
                            crate::fibre::RequestProtocol::FastCgi { request_id, keep_conn } => {
                                let _ = crate::io::write_fastcgi_response(&mut stream, request_id, &fibre.output_buffer);
                                if !keep_conn {
                                    let _ = stream.shutdown(std::net::Shutdown::Both);
                                }
                            }
                        }
                    } else {
                        // CLI mode: write directly to stdout
                        let _ = std::io::stdout().write_all(&fibre.output_buffer);
                        let _ = std::io::stdout().flush();
                    }

                    if fibre.boot_checkpoint.is_some() {
                        fibre.reset_to_boot_checkpoint();
                        reactors[fibre.reactor_id % reactors.len()].idle_octane_fibres.push(fibre);
                    } else {
                        fibre.reset_for_worker_request(fibre.entry_func_ptr);
                        idle_fibres.push(fibre);
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

                        let reactor = &reactors[fibre.reactor_id % reactors.len()];
                        if is_keep_alive {
                            reactor.recycle_keep_alive_stream(stream);
                        } else {
                            drop(stream);
                        }

                        if std::env::var("HYPERION_STATS").is_ok() {
                            eprintln!("[STATS YIELD] id: {}, has_cp: {}, arena: {}B (drop: {}), classes: {}, global_arena: {}B (drop: {})",
                                fibre.id,
                                fibre.boot_checkpoint.is_some(),
                                fibre.arena.allocated_bytes(),
                                fibre.arena.drop_list_len(),
                                fibre.engine_state.classes.len(),
                                fibre.engine_state.global_arena.allocated_bytes(),
                                fibre.engine_state.global_arena.drop_list_len(),
                            );
                        }

                        if fibre.boot_checkpoint.is_some() {
                            fibre.reset_to_boot_checkpoint();
                            if std::env::var("HYPERION_STATS").is_ok() {
                                eprintln!("[STATS AFTER RESET] id: {}, arena: {}B (drop: {})",
                                    fibre.id,
                                    fibre.arena.allocated_bytes(),
                                    fibre.arena.drop_list_len(),
                                );
                            }
                            reactor.idle_octane_fibres.push(fibre);
                        } else {
                            idle_fibres.push(fibre);
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

