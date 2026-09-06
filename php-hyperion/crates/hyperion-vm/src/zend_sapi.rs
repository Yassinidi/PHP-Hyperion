use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::io::{Read, Write};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineType {
    Auto,
    Vm,
    Php84,
}

impl EngineType {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "vm" | "hyperion" => EngineType::Vm,
            "php84" | "php" | "zend" | "c" => EngineType::Php84,
            _ => EngineType::Auto,
        }
    }
}

/// Detects the PHP 8.4 executable path on the system.
pub fn find_php_binary() -> Result<PathBuf, String> {
    if let Ok(path) = std::env::var("HYPERION_PHP_BINARY") {
        let p = PathBuf::from(path);
        if p.is_file() {
            return Ok(p);
        }
    }
    if let Ok(path) = std::env::var("PHP_BINARY") {
        let p = PathBuf::from(path);
        if p.is_file() {
            return Ok(p);
        }
    }

    // Common php-cgi locations
    let candidates = [
        "/opt/homebrew/bin/php-cgi",
        "/usr/local/bin/php-cgi",
        "/usr/bin/php-cgi",
        "/opt/homebrew/bin/php",
        "/usr/local/bin/php",
        "/usr/bin/php",
    ];

    for candidate in candidates {
        let p = PathBuf::from(candidate);
        if p.is_file() {
            return Ok(p);
        }
    }

    // Fallback: search PATH via which
    if let Ok(output) = Command::new("which").arg("php-cgi").output() {
        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path_str.is_empty() && Path::new(&path_str).is_file() {
                return Ok(PathBuf::from(path_str));
            }
        }
    }

    if let Ok(output) = Command::new("which").arg("php").output() {
        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path_str.is_empty() && Path::new(&path_str).is_file() {
                return Ok(PathBuf::from(path_str));
            }
        }
    }

    Err("Could not find a valid PHP 8.4 or php-cgi binary on this system. Please set HYPERION_PHP_BINARY.".to_string())
}

/// Detects the standard PHP CLI executable path.
pub fn find_php_cli() -> Result<PathBuf, String> {
    if let Ok(path) = std::env::var("HYPERION_PHP_CLI") {
        let p = PathBuf::from(path);
        if p.is_file() {
            return Ok(p);
        }
    }

    let candidates = [
        "/opt/homebrew/bin/php",
        "/usr/local/bin/php",
        "/usr/bin/php",
    ];

    for candidate in candidates {
        let p = PathBuf::from(candidate);
        if p.is_file() {
            return Ok(p);
        }
    }

    find_php_binary()
}

/// Executes a script in CLI mode via Zend PHP 8.4 engine.
pub fn execute_cli(
    script_path: &str,
    script_args: &[String],
    docroot: Option<&str>,
) -> Result<i32, String> {
    let php_bin = find_php_cli()?;
    let mut cmd = Command::new(php_bin);

    if let Some(dr) = docroot {
        cmd.current_dir(dr);
    } else if let Some(parent) = Path::new(script_path).parent() {
        if parent.is_dir() {
            cmd.current_dir(parent);
        }
    }

    cmd.arg(script_path);
    for arg in script_args {
        cmd.arg(arg);
    }

    cmd.stdin(Stdio::inherit());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    match cmd.status() {
        Ok(status) => Ok(status.code().unwrap_or(1)),
        Err(e) => Err(format!("Failed to execute PHP 8.4 CLI: {}", e)),
    }
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status_code: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl HttpResponse {
    pub fn new(status_code: u16, status_text: &str, headers: Vec<(String, String)>, body: Vec<u8>) -> Self {
        Self {
            status_code,
            status_text: status_text.to_string(),
            headers,
            body,
        }
    }

    pub fn to_http_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        let status_line = format!("HTTP/1.1 {} {}\r\n", self.status_code, self.status_text);
        out.extend_from_slice(status_line.as_bytes());

        let mut has_content_length = false;
        let mut has_server = false;
        let mut has_connection = false;

        for (k, v) in &self.headers {
            let lower = k.to_lowercase();
            if lower == "status" {
                continue; // Skip CGI Status header
            }
            if lower == "content-length" {
                has_content_length = true;
            } else if lower == "server" {
                has_server = true;
            } else if lower == "connection" {
                has_connection = true;
            }
            let line = format!("{}: {}\r\n", k, v);
            out.extend_from_slice(line.as_bytes());
        }

        if !has_content_length {
            let line = format!("Content-Length: {}\r\n", self.body.len());
            out.extend_from_slice(line.as_bytes());
        }
        if !has_server {
            out.extend_from_slice(b"Server: PHP-Hyperion/1.0.0 (PHP 8.4 Hybrid SAPI)\r\n");
        }
        if !has_connection {
            out.extend_from_slice(b"Connection: close\r\n");
        }

        out.extend_from_slice(b"\r\n");
        out.extend_from_slice(&self.body);
        out
    }
}

/// Executes an HTTP request via the Zend PHP 8.4 CGI SAPI.
pub fn execute_http(
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
    let php_cgi = find_php_binary()?;
    let mut cmd = Command::new(php_cgi);

    cmd.arg("-q"); // Quiet mode: suppresses extraneous headers
    cmd.arg("-d").arg("opcache.enable_cli=1");
    cmd.arg("-d").arg("opcache.jit_buffer_size=64M");

    if !docroot.is_empty() && Path::new(docroot).is_dir() {
        cmd.current_dir(docroot);
    } else if let Some(parent) = Path::new(script_path).parent() {
        if parent.is_dir() {
            cmd.current_dir(parent);
        }
    }

    // Required CGI environment variables
    cmd.env("REDIRECT_STATUS", "200");
    cmd.env("REQUEST_METHOD", method);
    let abs_script_path = if Path::new(script_path).is_absolute() {
        PathBuf::from(script_path)
    } else {
        Path::new(docroot).join(script_path)
    };
    cmd.env("SCRIPT_FILENAME", &abs_script_path);

    let script_name = if let Some(pos) = uri.find('?') {
        &uri[..pos]
    } else {
        uri
    };
    cmd.env("SCRIPT_NAME", script_name);
    cmd.env("DOCUMENT_ROOT", docroot);
    cmd.env("REQUEST_URI", uri);
    cmd.env("QUERY_STRING", query_string);
    cmd.env("SERVER_SOFTWARE", "PHP-Hyperion/1.0.0 (PHP 8.4 Hybrid SAPI)");
    cmd.env("SERVER_PROTOCOL", "HTTP/1.1");
    cmd.env("GATEWAY_INTERFACE", "CGI/1.1");

    if let Some(ra) = remote_addr {
        if let Some((ip, port)) = ra.split_once(':') {
            cmd.env("REMOTE_ADDR", ip);
            cmd.env("REMOTE_PORT", port);
        } else {
            cmd.env("REMOTE_ADDR", ra);
        }
    } else {
        cmd.env("REMOTE_ADDR", "127.0.0.1");
        cmd.env("REMOTE_PORT", "80");
    }

    if let Some(sa) = server_addr {
        if let Some((ip, port)) = sa.split_once(':') {
            cmd.env("SERVER_NAME", ip);
            cmd.env("SERVER_PORT", port);
        } else {
            cmd.env("SERVER_NAME", sa);
            cmd.env("SERVER_PORT", "80");
        }
    } else {
        cmd.env("SERVER_NAME", "localhost");
        cmd.env("SERVER_PORT", "8000");
    }

    // Translate HTTP headers into CGI environment variables
    for (name, val) in headers {
        let name_upper = name.to_uppercase();
        if name_upper == "CONTENT-TYPE" {
            cmd.env("CONTENT_TYPE", *val);
        } else if name_upper == "CONTENT-LENGTH" {
            cmd.env("CONTENT_LENGTH", *val);
        } else {
            let env_key = format!("HTTP_{}", name_upper.replace('-', "_"));
            cmd.env(env_key, *val);
        }
    }

    if !body.is_empty() {
        cmd.env("CONTENT_LENGTH", body.len().to_string());
    }

    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| format!("Failed to spawn PHP 8.4 engine: {}", e))?;

    if !body.is_empty() {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(body);
            let _ = stdin.flush();
        }
    } else {
        // Drop stdin to signal EOF immediately
        drop(child.stdin.take());
    }

    let output = child.wait_with_output().map_err(|e| format!("PHP 8.4 engine process failed: {}", e))?;

    let stdout = output.stdout;
    let stderr = output.stderr;

    // Parse CGI output into headers and body
    let mut status_code = 200u16;
    let mut status_text = "OK".to_string();
    let mut resp_headers = Vec::new();
    let mut resp_body = Vec::new();

    let split_pos = stdout.windows(4).position(|w| w == b"\r\n\r\n");
    let (header_bytes, body_bytes) = if let Some(pos) = split_pos {
        (&stdout[..pos], &stdout[pos + 4..])
    } else if let Some(pos) = stdout.windows(2).position(|w| w == b"\n\n") {
        (&stdout[..pos], &stdout[pos + 2..])
    } else {
        // No header separator found; raw output
        (b"".as_slice(), stdout.as_slice())
    };

    if !header_bytes.is_empty() {
        let header_str = String::from_utf8_lossy(header_bytes);
        for line in header_str.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some((k, v)) = line.split_once(':') {
                let key = k.trim();
                let val = v.trim();
                if key.eq_ignore_ascii_case("status") {
                    if let Some((code_str, text_str)) = val.split_once(' ') {
                        if let Ok(c) = code_str.parse::<u16>() {
                            status_code = c;
                            status_text = text_str.trim().to_string();
                        }
                    } else if let Ok(c) = val.parse::<u16>() {
                        status_code = c;
                    }
                } else {
                    resp_headers.push((key.to_string(), val.to_string()));
                }
            }
        }
    }

    resp_body.extend_from_slice(body_bytes);

    if resp_body.is_empty() && !stderr.is_empty() && status_code == 200 {
        status_code = 500;
        status_text = "Internal Server Error".to_string();
        resp_headers.push(("Content-Type".to_string(), "text/plain; charset=utf-8".to_string()));
        resp_body.extend_from_slice(&stderr);
    }

    Ok(HttpResponse::new(status_code, &status_text, resp_headers, resp_body))
}

/// Helper to detect if a file contains PHP 8.4 specific syntax or extensions
/// that require the Zend engine.
pub fn file_requires_php84(file_path: &str) -> bool {
    // If the docroot or project root has wp-config.php or wp-load.php, or file is part of WordPress:
    if std::path::Path::new("wp-config.php").is_file() 
        || std::path::Path::new("wp-load.php").is_file()
        || std::path::Path::new("../wp-config.php").is_file()
        || file_path.contains("wp-admin")
        || file_path.contains("wp-includes")
        || file_path.contains("wp-login")
        || file_path.contains("wp-") {
        return true;
    }

    if let Ok(content) = std::fs::read_to_string(file_path) {
        // PHP 8.4 Property Hooks
        if content.contains("get =>") || content.contains("set =>") || content.contains("get {") || content.contains("set {") {
            return true;
        }
        // PHP 8.4 Asymmetric Visibility
        if content.contains("(set)") {
            return true;
        }
        // PHP 8.4 new MyClass()->method() without parentheses
        if content.contains("new ") && content.contains(")->") {
            return true;
        }
        // PHP 8.4 new \Dom\HTMLDocument
        if content.contains("HTMLDocument") || content.contains("Dom\\") {
            return true;
        }
        // PHP 8.4 array functions
        if content.contains("array_find") || content.contains("array_find_key") || content.contains("array_any") || content.contains("array_all") {
            return true;
        }
        // PHP 8.4 mbstring functions
        if content.contains("mb_trim") || content.contains("mb_ltrim") || content.contains("mb_rtrim") || content.contains("mb_ucfirst") || content.contains("mb_lcfirst") {
            return true;
        }
        // PHP 8.4 #[Deprecated] attribute
        if content.contains("#[Deprecated") || content.contains("#[\\Deprecated") {
            return true;
        }
        // PHP 8.4 PDO driver subclasses
        if content.contains("Pdo\\") || content.contains("Pdo\\Mysql") || content.contains("Pdo\\Sqlite") || content.contains("Pdo\\Pgsql") {
            return true;
        }
        // Native MySQLi
        if content.contains("mysqli") {
            return true;
        }
        // WordPress & large framework markers
        if content.contains("wp-load.php") || content.contains("wp-config.php") || content.contains("wp-blog-header.php") || content.contains("ABSPATH") {
            return true;
        }
    }
    false
}
