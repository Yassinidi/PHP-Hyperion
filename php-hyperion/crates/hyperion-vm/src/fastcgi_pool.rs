use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::zend_sapi::{find_php_binary, HttpResponse};

// FastCGI 1.0 Protocol Constants
pub const FCGI_VERSION_1: u8 = 1;
pub const FCGI_BEGIN_REQUEST: u8 = 1;
pub const FCGI_ABORT_REQUEST: u8 = 2;
pub const FCGI_END_REQUEST: u8 = 3;
pub const FCGI_PARAMS: u8 = 4;
pub const FCGI_STDIN: u8 = 5;
pub const FCGI_STDOUT: u8 = 6;
pub const FCGI_STDERR: u8 = 7;
pub const FCGI_DATA: u8 = 8;
pub const FCGI_GET_VALUES: u8 = 9;
pub const FCGI_GET_VALUES_RESULT: u8 = 10;
pub const FCGI_UNKNOWN_TYPE: u8 = 11;

pub const FCGI_RESPONDER: u16 = 1;
pub const FCGI_KEEP_CONN: u8 = 1;

/// Encodes an 8-byte FastCGI record header with content and padding
pub fn encode_record(type_: u8, req_id: u16, content: &[u8]) -> Vec<u8> {
    let content_len = content.len() as u16;
    let padding_len = (8 - (content.len() % 8)) % 8;
    let mut rec = Vec::with_capacity(8 + content.len() + padding_len);
    rec.push(FCGI_VERSION_1);
    rec.push(type_);
    rec.extend_from_slice(&req_id.to_be_bytes());
    rec.extend_from_slice(&content_len.to_be_bytes());
    rec.push(padding_len as u8);
    rec.push(0); // reserved
    rec.extend_from_slice(content);
    if padding_len > 0 {
        rec.resize(rec.len() + padding_len, 0);
    }
    rec
}

/// Encodes a FastCGI name-value pair with variable-length prefixes
pub fn encode_name_value(name: &str, value: &str, out: &mut Vec<u8>) {
    let name_bytes = name.as_bytes();
    let val_bytes = value.as_bytes();
    let nlen = name_bytes.len();
    let vlen = val_bytes.len();

    if nlen < 128 {
        out.push(nlen as u8);
    } else {
        out.extend_from_slice(&((nlen as u32) | 0x8000_0000).to_be_bytes());
    }

    if vlen < 128 {
        out.push(vlen as u8);
    } else {
        out.extend_from_slice(&((vlen as u32) | 0x8000_0000).to_be_bytes());
    }

    out.extend_from_slice(name_bytes);
    out.extend_from_slice(val_bytes);
}

/// Parses raw CGI / FastCGI STDOUT stream into an HttpResponse
pub fn parse_cgi_stdout(raw: &[u8]) -> Result<HttpResponse, String> {
    let header_end = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|p| (p, 4))
        .or_else(|| raw.windows(2).position(|w| w == b"\n\n").map(|p| (p, 2)));

    let (head_bytes, body_bytes) = if let Some((pos, sep_len)) = header_end {
        (&raw[..pos], &raw[pos + sep_len..])
    } else {
        (raw, &[][..])
    };

    let head_str = String::from_utf8_lossy(head_bytes);
    let mut status_code = 200u16;
    let mut status_text = "OK".to_string();
    let mut headers = Vec::new();

    for line in head_str.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some((k, v)) = line.split_once(':') {
            let k_trim = k.trim();
            let v_trim = v.trim();
            if k_trim.eq_ignore_ascii_case("status") {
                let parts: Vec<&str> = v_trim.split_whitespace().collect();
                if let Some(code_str) = parts.first() {
                    if let Ok(code) = code_str.parse::<u16>() {
                        status_code = code;
                        if parts.len() > 1 {
                            status_text = parts[1..].join(" ");
                        }
                    }
                }
            } else {
                headers.push((k_trim.to_string(), v_trim.to_string()));
            }
        }
    }

    Ok(HttpResponse {
        status_code,
        status_text,
        headers,
        body: body_bytes.to_vec(),
    })
}

/// Single FastCGI request executor over a Unix socket stream
fn execute_fcgi_on_stream(
    stream: &mut UnixStream,
    params: &[(&str, &str)],
    body: &[u8],
) -> Result<HttpResponse, String> {
    let req_id = 1u16;

    // 1. FCGI_BEGIN_REQUEST
    let mut begin_body = [0u8; 8];
    begin_body[0..2].copy_from_slice(&FCGI_RESPONDER.to_be_bytes());
    begin_body[2] = FCGI_KEEP_CONN;
    let begin_rec = encode_record(FCGI_BEGIN_REQUEST, req_id, &begin_body);
    stream
        .write_all(&begin_rec)
        .map_err(|e| format!("FCGI write begin failed: {}", e))?;

    // 2. FCGI_PARAMS
    let mut params_payload = Vec::with_capacity(2048);
    for (k, v) in params {
        encode_name_value(k, v, &mut params_payload);
    }

    if params_payload.is_empty() {
        let empty_params = encode_record(FCGI_PARAMS, req_id, &[]);
        stream
            .write_all(&empty_params)
            .map_err(|e| format!("FCGI write empty params failed: {}", e))?;
    } else {
        for chunk in params_payload.chunks(65535) {
            let rec = encode_record(FCGI_PARAMS, req_id, chunk);
            stream
                .write_all(&rec)
                .map_err(|e| format!("FCGI write params failed: {}", e))?;
        }
        let empty_params = encode_record(FCGI_PARAMS, req_id, &[]);
        stream
            .write_all(&empty_params)
            .map_err(|e| format!("FCGI write end params failed: {}", e))?;
    }

    // 3. FCGI_STDIN
    if !body.is_empty() {
        for chunk in body.chunks(65535) {
            let rec = encode_record(FCGI_STDIN, req_id, chunk);
            stream
                .write_all(&rec)
                .map_err(|e| format!("FCGI write stdin failed: {}", e))?;
        }
    }
    let empty_stdin = encode_record(FCGI_STDIN, req_id, &[]);
    stream
        .write_all(&empty_stdin)
        .map_err(|e| format!("FCGI write end stdin failed: {}", e))?;
    stream.flush().map_err(|e| format!("FCGI flush failed: {}", e))?;

    // 4. Read Response Records
    let mut stdout_buf = Vec::new();
    let mut stderr_buf = Vec::new();
    let mut header_buf = [0u8; 8];

    loop {
        stream
            .read_exact(&mut header_buf)
            .map_err(|e| format!("FCGI read header failed: {}", e))?;
        let rec_type = header_buf[1];
        let content_len = u16::from_be_bytes([header_buf[4], header_buf[5]]) as usize;
        let padding_len = header_buf[6] as usize;

        let mut content = vec![0u8; content_len];
        if content_len > 0 {
            stream
                .read_exact(&mut content)
                .map_err(|e| format!("FCGI read content failed: {}", e))?;
        }

        if padding_len > 0 {
            let mut pad = vec![0u8; padding_len];
            stream
                .read_exact(&mut pad)
                .map_err(|e| format!("FCGI read pad failed: {}", e))?;
        }

        match rec_type {
            FCGI_STDOUT => {
                stdout_buf.extend_from_slice(&content);
            }
            FCGI_STDERR => {
                stderr_buf.extend_from_slice(&content);
            }
            FCGI_END_REQUEST => {
                break;
            }
            _ => {}
        }
    }

    if !stderr_buf.is_empty() && std::env::var("HYPERION_VERBOSE").is_ok() {
        eprintln!(
            "[FastCGI stderr] {}",
            String::from_utf8_lossy(&stderr_buf)
        );
    }

    parse_cgi_stdout(&stdout_buf)
}

/// Persistent FastCGI Process Supervisor and Connection Pool
pub struct FastCgiPool {
    sock_path: PathBuf,
    child_pid: AtomicU32,
    idle_connections: Mutex<Vec<UnixStream>>,
}

impl FastCgiPool {
    pub fn new() -> Result<Self, String> {
        let php_cgi = find_php_binary()?;
        let uid = unsafe { libc::getuid() };
        let run_dir = PathBuf::from(format!("/tmp/hyperion-{}", uid));
        let _ = std::fs::create_dir_all(&run_dir);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&run_dir, std::fs::Permissions::from_mode(0o700));
        }
        let sock_path = run_dir.join(format!("fcgi-{}.sock", std::process::id()));
        let sock_name = sock_path.to_string_lossy().into_owned();

        if sock_path.exists() {
            let _ = std::fs::remove_file(&sock_path);
        }

        let num_workers = std::env::var("PHP_CLI_SERVER_WORKERS")
            .ok()
            .and_then(|w| w.parse::<usize>().ok())
            .unwrap_or(16)
            .max(4);

        let mut cmd = Command::new(&php_cgi);
        cmd.arg("-b").arg(&sock_name);
        cmd.arg("-d").arg("opcache.enable_cli=1");
        cmd.arg("-d").arg("opcache.jit_buffer_size=128M");
        cmd.arg("-d").arg("opcache.jit=tracing");
        cmd.arg("-d").arg("opcache.memory_consumption=256");
        cmd.arg("-d").arg("opcache.interned_strings_buffer=32");
        cmd.arg("-d").arg("opcache.max_accelerated_files=32531");
        cmd.arg("-d").arg("realpath_cache_size=4096k");
        cmd.arg("-d").arg("realpath_cache_ttl=600");

        cmd.env("PHP_FCGI_CHILDREN", num_workers.to_string());
        cmd.env("PHP_FCGI_MAX_REQUESTS", "50000");

        let child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn persistent FastCGI daemon: {}", e))?;
        let child_pid = child.id();

        // Wait up to 3 seconds for the socket to accept connections
        let start = Instant::now();
        let mut connected = false;
        while start.elapsed() < Duration::from_secs(3) {
            if sock_path.exists() {
                if let Ok(stream) = UnixStream::connect(&sock_path) {
                    drop(stream);
                    connected = true;
                    break;
                }
            }
            std::thread::sleep(Duration::from_millis(15));
        }

        if !connected {
            return Err(format!(
                "FastCGI daemon failed to create socket within 3s at {}",
                sock_name
            ));
        }

        // Register atexit handler to ensure child process is terminated on exit
        extern "C" fn cleanup_on_exit() {
            let uid = unsafe { libc::getuid() };
            let sock_name = format!("/tmp/hyperion-{}/fcgi-{}.sock", uid, std::process::id());
            let _ = std::fs::remove_file(&sock_name);
        }
        unsafe {
            libc::atexit(cleanup_on_exit);
        }

        Ok(Self {
            sock_path,
            child_pid: AtomicU32::new(child_pid),
            idle_connections: Mutex::new(Vec::new()),
        })
    }

    fn get_connection(&self) -> Result<UnixStream, String> {
        if let Ok(mut lock) = self.idle_connections.lock() {
            if let Some(stream) = lock.pop() {
                return Ok(stream);
            }
        }
        UnixStream::connect(&self.sock_path)
            .map_err(|e| format!("Failed to connect to FastCGI socket {:?}: {}", self.sock_path, e))
    }

    fn return_connection(&self, stream: UnixStream) {
        if let Ok(mut lock) = self.idle_connections.lock() {
            if lock.len() < 32 {
                lock.push(stream);
            }
        }
    }

    pub fn execute_request(
        &self,
        script_path: &str,
        docroot: &str,
        method: &str,
        uri: &str,
        query_string: &str,
        headers: &[(&str, &str)],
        body: &[u8],
        remote_addr: Option<&str>,
        server_addr: Option<&str>,
    ) -> Result<HttpResponse, String> {
        let abs_script_path = if Path::new(script_path).is_absolute() {
            PathBuf::from(script_path)
        } else {
            Path::new(docroot).join(script_path)
        };
        let abs_script_str = abs_script_path.to_string_lossy();

        let script_name = if let Some(pos) = uri.find('?') {
            &uri[..pos]
        } else {
            uri
        };

        let mut params: Vec<(String, String)> = Vec::with_capacity(32);
        params.push(("REDIRECT_STATUS".to_string(), "200".to_string()));
        params.push(("REQUEST_METHOD".to_string(), method.to_string()));
        params.push(("SCRIPT_FILENAME".to_string(), abs_script_str.to_string()));
        params.push(("SCRIPT_NAME".to_string(), script_name.to_string()));
        params.push(("DOCUMENT_ROOT".to_string(), docroot.to_string()));
        params.push(("REQUEST_URI".to_string(), uri.to_string()));
        params.push(("QUERY_STRING".to_string(), query_string.to_string()));
        params.push((
            "SERVER_SOFTWARE".to_string(),
            "PHP-Hyperion/1.0.0 (FastCGI Persistent Daemon)".to_string(),
        ));
        params.push(("SERVER_PROTOCOL".to_string(), "HTTP/1.1".to_string()));
        params.push(("GATEWAY_INTERFACE".to_string(), "CGI/1.1".to_string()));

        if let Some(ra) = remote_addr {
            if let Some((ip, port)) = ra.split_once(':') {
                params.push(("REMOTE_ADDR".to_string(), ip.to_string()));
                params.push(("REMOTE_PORT".to_string(), port.to_string()));
            } else {
                params.push(("REMOTE_ADDR".to_string(), ra.to_string()));
            }
        } else {
            params.push(("REMOTE_ADDR".to_string(), "127.0.0.1".to_string()));
            params.push(("REMOTE_PORT".to_string(), "80".to_string()));
        }

        if let Some(sa) = server_addr {
            if let Some((ip, port)) = sa.split_once(':') {
                params.push(("SERVER_NAME".to_string(), ip.to_string()));
                params.push(("SERVER_PORT".to_string(), port.to_string()));
            } else {
                params.push(("SERVER_NAME".to_string(), sa.to_string()));
            }
        } else {
            params.push(("SERVER_NAME".to_string(), "localhost".to_string()));
            params.push(("SERVER_PORT".to_string(), "8000".to_string()));
        }

        for (name, val) in headers {
            let name_upper = name.to_uppercase();
            if name_upper == "CONTENT-TYPE" {
                params.push(("CONTENT_TYPE".to_string(), val.to_string()));
            } else if name_upper == "CONTENT-LENGTH" {
                params.push(("CONTENT_LENGTH".to_string(), val.to_string()));
            } else {
                let env_key = format!("HTTP_{}", name_upper.replace('-', "_"));
                params.push((env_key, val.to_string()));
            }
        }

        if !body.is_empty() {
            params.push(("CONTENT_LENGTH".to_string(), body.len().to_string()));
        }

        let ref_params: Vec<(&str, &str)> = params
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        // 1. Try pooled connection
        let mut stream = self.get_connection()?;
        let res = execute_fcgi_on_stream(&mut stream, &ref_params, body);

        match res {
            Ok(resp) => {
                self.return_connection(stream);
                Ok(resp)
            }
            Err(_retry_err) => {
                // Retry once with a freshly dialed socket in case previous keep-alive expired
                let mut fresh_stream = UnixStream::connect(&self.sock_path)
                    .map_err(|e| format!("FastCGI reconnect failed: {}", e))?;
                let fresh_res = execute_fcgi_on_stream(&mut fresh_stream, &ref_params, body)?;
                self.return_connection(fresh_stream);
                Ok(fresh_res)
            }
        }
    }
}

impl Drop for FastCgiPool {
    fn drop(&mut self) {
        let pid = self.child_pid.load(Ordering::Relaxed);
        if pid > 0 {
            unsafe {
                libc::kill(pid as i32, libc::SIGTERM);
            }
        }
        if self.sock_path.exists() {
            let _ = std::fs::remove_file(&self.sock_path);
        }
    }
}

static GLOBAL_FASTCGI_POOL: OnceLock<FastCgiPool> = OnceLock::new();

pub fn get_global_fastcgi_pool() -> Result<&'static FastCgiPool, String> {
    if let Some(pool) = GLOBAL_FASTCGI_POOL.get() {
        return Ok(pool);
    }
    match FastCgiPool::new() {
        Ok(pool) => {
            let _ = GLOBAL_FASTCGI_POOL.set(pool);
            Ok(GLOBAL_FASTCGI_POOL.get().unwrap())
        }
        Err(e) => Err(e),
    }
}
