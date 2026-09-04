use hyperion_core::memory::nan_box::Value;
use hyperion_core::php_function;
use hyperion_core::types::array::{ArrayKey, PhpArray};
use hyperion_core::types::reference::PhpRef;
use hyperion_core::types::resource::{PhpResource, ResourceKind};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Mutex;
use std::time::Duration;
use ureq::{Agent, AgentBuilder};

pub const CURLOPT_PORT: i64 = 3;
pub const CURLOPT_TIMEOUT: i64 = 13;
pub const CURLOPT_USERAGENT: i64 = 10018;
pub const CURLOPT_REFERER: i64 = 10016;
pub const CURLOPT_ENCODING: i64 = 10102;
pub const CURLOPT_COOKIE: i64 = 10022;
pub const CURLOPT_COOKIEFILE: i64 = 10031;
pub const CURLOPT_COOKIEJAR: i64 = 10082;
pub const CURLOPT_SSL_VERIFYPEER: i64 = 64;
pub const CURLOPT_SSL_VERIFYHOST: i64 = 81;
pub const CURLOPT_NOBODY: i64 = 44;
pub const CURLOPT_HTTPGET: i64 = 80;
pub const CURLOPT_URL: i64 = 10002;
pub const CURLOPT_RETURNTRANSFER: i64 = 19913;
pub const CURLOPT_HEADER: i64 = 42;
pub const CURLOPT_POST: i64 = 47;
pub const CURLOPT_POSTFIELDS: i64 = 10015;
pub const CURLOPT_HTTPHEADER: i64 = 10023;
pub const CURLOPT_CUSTOMREQUEST: i64 = 10036;
pub const CURLOPT_FOLLOWLOCATION: i64 = 52;
pub const CURLOPT_MAXREDIRS: i64 = 68;
pub const CURLOPT_TIMEOUT_MS: i64 = 155;

pub const CURLINFO_EFFECTIVE_URL: i64 = 1048577;
pub const CURLINFO_HTTP_CODE: i64 = 2097154;
pub const CURLINFO_RESPONSE_CODE: i64 = 2097154;
pub const CURLINFO_HEADER_SIZE: i64 = 2097163;
pub const CURLINFO_SIZE_DOWNLOAD: i64 = 3145736;
pub const CURLINFO_TOTAL_TIME: i64 = 3145731;
pub const CURLINFO_CONTENT_TYPE: i64 = 1048594;
pub const CURLINFO_SPEED_DOWNLOAD: i64 = 3145737;

pub const CURLE_OK: i64 = 0;
pub const CURLM_OK: i64 = 0;
pub const CURLM_CALL_MULTI_PERFORM: i64 = -1;
pub const CURLMSG_DONE: i64 = 1;

#[derive(Debug, Clone)]
pub struct CurlHandleState {
    pub id: usize,
    pub url: String,
    pub method: String,
    pub headers: HashMap<String, String>,
    pub post_fields: Option<Vec<u8>>,
    pub return_transfer: bool,
    pub header_in_body: bool,
    pub follow_location: bool,
    pub max_redirs: usize,
    pub timeout: Duration,
    pub cookie_file: Option<String>,
    pub cookie_jar: Option<String>,
    pub ssl_verify: bool,
    // Results from last execution
    pub last_http_code: i64,
    pub last_header_size: i64,
    pub last_size_download: i64,
    pub last_content_type: String,
    pub last_headers: String,
    pub last_body: Vec<u8>,
    pub last_error: String,
    pub last_errno: i64,
    pub total_time_secs: f64,
}

impl CurlHandleState {
    pub fn new(id: usize, url: String) -> Self {
        Self {
            id,
            url,
            method: "GET".to_string(),
            headers: HashMap::new(),
            post_fields: None,
            return_transfer: false,
            header_in_body: false,
            follow_location: true,
            max_redirs: 10,
            timeout: Duration::from_secs(30),
            cookie_file: None,
            cookie_jar: None,
            ssl_verify: true,
            last_http_code: 0,
            last_header_size: 0,
            last_size_download: 0,
            last_content_type: String::new(),
            last_headers: String::new(),
            last_body: Vec::new(),
            last_error: String::new(),
            last_errno: 0,
            total_time_secs: 0.0,
        }
    }

    pub fn execute(&mut self) -> Result<Vec<u8>, String> {
        if self.url.is_empty() {
            self.last_error = "No URL set".to_string();
            self.last_errno = 3; // CURLE_URL_MALFORMAT
            return Err(self.last_error.clone());
        }

        let start_instant = std::time::Instant::now();

        let agent_builder = AgentBuilder::new()
            .timeout(self.timeout)
            .redirects(if self.follow_location { self.max_redirs as u32 } else { 0 });

        let agent: Agent = agent_builder.build();

        let method = self.method.to_uppercase();
        let mut req = agent.request(&method, &self.url);

        for (k, v) in &self.headers {
            req = req.set(k, v);
        }

        // Handle cookies from file if configured
        if let Some(ref cookie_path) = self.cookie_file {
            if let Ok(cookie_content) = std::fs::read_to_string(cookie_path) {
                for line in cookie_content.lines() {
                    let line = line.trim();
                    if line.starts_with('#') || line.is_empty() {
                        continue;
                    }
                    let parts: Vec<&str> = line.split('\t').collect();
                    if parts.len() >= 7 {
                        let name = parts[5];
                        let val = parts[6];
                        req = req.set("Cookie", &format!("{}={}", name, val));
                    }
                }
            }
        }

        let send_res = if let Some(ref body_bytes) = self.post_fields {
            req.send_bytes(body_bytes)
        } else {
            req.call()
        };

        self.total_time_secs = start_instant.elapsed().as_secs_f64();

        match send_res {
            Ok(resp) => {
                let status = resp.status() as i64;
                self.last_http_code = status;
                self.last_content_type = resp.content_type().to_string();

                let mut headers_str = format!("HTTP/1.1 {} {}\r\n", status, resp.status_text());
                for header_name in resp.headers_names() {
                    if let Some(val) = resp.header(&header_name) {
                        headers_str.push_str(&format!("{}: {}\r\n", header_name, val));
                    }
                }
                headers_str.push_str("\r\n");
                self.last_headers = headers_str;
                self.last_header_size = self.last_headers.len() as i64;

                let mut reader = resp.into_reader();
                let mut body = Vec::new();
                let _ = reader.read_to_end(&mut body);
                self.last_size_download = body.len() as i64;
                self.last_body = body.clone();
                self.last_error.clear();
                self.last_errno = 0;

                // If cookie jar configured, write Set-Cookie headers
                if let Some(ref jar_path) = self.cookie_jar {
                    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(jar_path) {
                        let _ = writeln!(f, "# Netscape HTTP Cookie File");
                    }
                }

                let mut full_output = Vec::new();
                if self.header_in_body {
                    full_output.extend_from_slice(self.last_headers.as_bytes());
                }
                full_output.extend_from_slice(&body);
                Ok(full_output)
            }
            Err(ureq::Error::Status(code, resp)) => {
                let status = code as i64;
                self.last_http_code = status;
                self.last_content_type = resp.content_type().to_string();

                let mut headers_str = format!("HTTP/1.1 {} {}\r\n", status, resp.status_text());
                for header_name in resp.headers_names() {
                    if let Some(val) = resp.header(&header_name) {
                        headers_str.push_str(&format!("{}: {}\r\n", header_name, val));
                    }
                }
                headers_str.push_str("\r\n");
                self.last_headers = headers_str;
                self.last_header_size = self.last_headers.len() as i64;

                let mut reader = resp.into_reader();
                let mut body = Vec::new();
                let _ = reader.read_to_end(&mut body);
                self.last_size_download = body.len() as i64;
                self.last_body = body.clone();
                self.last_error.clear();
                self.last_errno = 0;

                let mut full_output = Vec::new();
                if self.header_in_body {
                    full_output.extend_from_slice(self.last_headers.as_bytes());
                }
                full_output.extend_from_slice(&body);
                Ok(full_output)
            }
            Err(ureq::Error::Transport(t)) => {
                self.last_http_code = 0;
                self.last_error = t.to_string();
                self.last_errno = 7; // CURLE_COULDNT_CONNECT
                Err(self.last_error.clone())
            }
        }
    }
}

pub struct CurlMultiState {
    pub handles: Vec<usize>,
    pub completed: Vec<(usize, i64)>, // (handle_id, http_code)
}

lazy_static::lazy_static! {
    static ref CURL_HANDLES: Mutex<HashMap<usize, CurlHandleState>> = Mutex::new(HashMap::new());
    static ref CURL_MULTI_HANDLES: Mutex<HashMap<usize, CurlMultiState>> = Mutex::new(HashMap::new());
    static ref NEXT_CURL_ID: Mutex<usize> = Mutex::new(1);
}

fn next_id() -> usize {
    let mut guard = NEXT_CURL_ID.lock().unwrap();
    let id = *guard;
    *guard += 1;
    id
}

fn get_handle_id(val: Option<&Value>) -> Option<usize> {
    let val = val?.deref();
    if let Some(res_ptr) = val.as_resource_ptr() {
        let res = unsafe { &*(res_ptr as *const PhpResource) };
        if let ResourceKind::Curl(id) = res.kind {
            return Some(id);
        }
    }
    None
}

fn get_multi_id(val: Option<&Value>) -> Option<usize> {
    let val = val?.deref();
    if let Some(res_ptr) = val.as_resource_ptr() {
        let res = unsafe { &*(res_ptr as *const PhpResource) };
        if let ResourceKind::CurlMulti(id) = res.kind {
            return Some(id);
        }
    }
    None
}

php_function! {
    native_curl_init(url: Value) {
        let id = next_id();
        let url_str = if let Some(u) = url {
            let ud = u.deref();
            if let Some(sp) = ud.as_string_ptr() {
                unsafe { (*(sp as *const String)).clone() }
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let handle = CurlHandleState::new(id, url_str);
        {
            let mut guard = CURL_HANDLES.lock().unwrap();
            guard.insert(id, handle);
        }

        let res = PhpResource::new(ResourceKind::Curl(id), "curl");
        let ptr = crate::into_raw(Box::new(res));
        Ok(Value::new_resource_ptr(ptr as *mut ()))
    }
}

php_function! {
    native_curl_setopt(ch: Value, option: Value, val: Value) {
        let Some(handle_id) = get_handle_id(ch) else {
            return Ok(Value::new_bool(false));
        };
        let Some(opt_val) = option else {
            return Ok(Value::new_bool(false));
        };
        let opt_int = opt_val.deref().as_int().unwrap_or(0) as i64;
        let Some(v) = val else {
            return Ok(Value::new_bool(false));
        };
        let vd = v.deref();

        let mut guard = CURL_HANDLES.lock().unwrap();
        let Some(handle) = guard.get_mut(&handle_id) else {
            return Ok(Value::new_bool(false));
        };

        match opt_int {
            CURLOPT_URL => {
                if let Some(sp) = vd.as_string_ptr() {
                    handle.url = unsafe { (*(sp as *const String)).clone() };
                }
            }
            CURLOPT_RETURNTRANSFER => {
                handle.return_transfer = vd.as_bool().unwrap_or_else(|| vd.as_int().map(|i| i != 0).unwrap_or(false));
            }
            CURLOPT_HEADER => {
                handle.header_in_body = vd.as_bool().unwrap_or_else(|| vd.as_int().map(|i| i != 0).unwrap_or(false));
            }
            CURLOPT_POST => {
                let is_post = vd.as_bool().unwrap_or_else(|| vd.as_int().map(|i| i != 0).unwrap_or(false));
                if is_post {
                    handle.method = "POST".to_string();
                }
            }
            CURLOPT_POSTFIELDS => {
                if let Some(sp) = vd.as_string_ptr() {
                    let s = unsafe { &*(sp as *const String) };
                    handle.post_fields = Some(s.as_bytes().to_vec());
                } else if let Some(arr_ptr) = vd.as_array_ptr() {
                    let arr = unsafe { &*(arr_ptr as *const PhpArray) };
                    let mut parts = Vec::new();
                    for (k, val_entry) in arr.elements.iter() {
                        let vd = val_entry.deref();
                        let key_str = match k {
                            ArrayKey::StringId(sid) => hyperion_core::types::string_table::lookup_string(*sid).unwrap_or_default(),
                            ArrayKey::Int(i) => i.to_string(),
                        };
                        let val_str = if let Some(sp) = vd.as_string_ptr() {
                            unsafe { (*(sp as *const String)).clone() }
                        } else if let Some(i) = vd.as_int() {
                            i.to_string()
                        } else {
                            String::new()
                        };
                        parts.push(format!("{}={}", urlencoding_encode(&key_str), urlencoding_encode(&val_str)));
                    }
                    handle.post_fields = Some(parts.join("&").into_bytes());
                }
            }
            CURLOPT_CUSTOMREQUEST => {
                if let Some(sp) = vd.as_string_ptr() {
                    handle.method = unsafe { (*(sp as *const String)).clone() };
                }
            }
            CURLOPT_TIMEOUT => {
                let secs = vd.as_int().unwrap_or(30);
                handle.timeout = Duration::from_secs(secs.max(1) as u64);
            }
            CURLOPT_TIMEOUT_MS => {
                let ms = vd.as_int().unwrap_or(30000);
                handle.timeout = Duration::from_millis(ms.max(1) as u64);
            }
            CURLOPT_HTTPHEADER => {
                if let Some(arr_ptr) = vd.as_array_ptr() {
                    let arr = unsafe { &*(arr_ptr as *const PhpArray) };
                    for (_, entry) in arr.elements.iter() {
                        let ed = entry.deref();
                        if let Some(sp) = ed.as_string_ptr() {
                            let header_line = unsafe { &*(sp as *const String) };
                            if let Some((k, val)) = header_line.split_once(':') {
                                handle.headers.insert(k.trim().to_string(), val.trim().to_string());
                            }
                        }
                    }
                }
            }
            CURLOPT_FOLLOWLOCATION => {
                handle.follow_location = vd.as_bool().unwrap_or_else(|| vd.as_int().map(|i| i != 0).unwrap_or(true));
            }
            CURLOPT_COOKIEFILE => {
                if let Some(sp) = vd.as_string_ptr() {
                    handle.cookie_file = Some(unsafe { (*(sp as *const String)).clone() });
                }
            }
            CURLOPT_COOKIEJAR => {
                if let Some(sp) = vd.as_string_ptr() {
                    handle.cookie_jar = Some(unsafe { (*(sp as *const String)).clone() });
                }
            }
            CURLOPT_SSL_VERIFYPEER => {
                handle.ssl_verify = vd.as_bool().unwrap_or_else(|| vd.as_int().map(|i| i != 0).unwrap_or(true));
            }
            _ => {}
        }

        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_curl_setopt_array(ch: Value, options: Value) |ctx| {
        let Some(_handle_id) = get_handle_id(ch) else {
            return Ok(Value::new_bool(false));
        };
        let Some(opts) = options else {
            return Ok(Value::new_bool(false));
        };
        let opts_d = opts.deref();
        let Some(arr_ptr) = opts_d.as_array_ptr() else {
            return Ok(Value::new_bool(false));
        };
        let arr = unsafe { &*(arr_ptr as *const PhpArray) };

        for (k, v) in arr.elements.iter() {
            let opt_id = match k {
                ArrayKey::Int(i) => *i,
                ArrayKey::StringId(sid) => {
                    let name = hyperion_core::types::string_table::lookup_string(*sid).unwrap_or_default();
                    name.parse().unwrap_or(0)
                }
            };
            let opt_val = Value::new_int(opt_id as i32);
            let _ = native_curl_setopt(&[ch.copied().unwrap_or_else(Value::null), opt_val, *v], ctx);
        }

        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_curl_exec(ch: Value) |ctx| {
        let Some(handle_id) = get_handle_id(ch) else {
            return Ok(Value::new_bool(false));
        };

        let mut handle = {
            let guard = CURL_HANDLES.lock().unwrap();
            match guard.get(&handle_id) {
                Some(h) => h.clone(),
                None => return Ok(Value::new_bool(false)),
            }
        };

        let res = handle.execute();

        // Update handle state
        {
            let mut guard = CURL_HANDLES.lock().unwrap();
            if let Some(h) = guard.get_mut(&handle_id) {
                *h = handle.clone();
            }
        }

        match res {
            Ok(output_bytes) => {
                if handle.return_transfer {
                    let s = String::from_utf8_lossy(&output_bytes).into_owned();
                    let ptr = ctx.get_arena().alloc_and_track(s);
                    Ok(Value::new_string_ptr(ptr as *mut ()))
                } else {
                    let _ = std::io::stdout().write_all(&output_bytes);
                    Ok(Value::new_bool(true))
                }
            }
            Err(_) => Ok(Value::new_bool(false)),
        }
    }
}

php_function! {
    native_curl_getinfo(ch: Value, opt: Value) |ctx| {
        let Some(handle_id) = get_handle_id(ch) else {
            return Ok(Value::null());
        };

        let guard = CURL_HANDLES.lock().unwrap();
        let Some(handle) = guard.get(&handle_id) else {
            return Ok(Value::null());
        };

        if let Some(opt_val) = opt {
            let opt_int = opt_val.deref().as_int().unwrap_or(0) as i64;
            match opt_int {
                CURLINFO_HTTP_CODE => Ok(Value::new_int(handle.last_http_code as i32)),
                CURLINFO_HEADER_SIZE => Ok(Value::new_int(handle.last_header_size as i32)),
                CURLINFO_SIZE_DOWNLOAD => Ok(Value::new_int(handle.last_size_download as i32)),
                CURLINFO_TOTAL_TIME => Ok(Value::new_float(handle.total_time_secs)),
                CURLINFO_EFFECTIVE_URL => {
                    let ptr = ctx.get_arena().alloc_and_track(handle.url.clone());
                    Ok(Value::new_string_ptr(ptr as *mut ()))
                }
                CURLINFO_CONTENT_TYPE => {
                    let ptr = ctx.get_arena().alloc_and_track(handle.last_content_type.clone());
                    Ok(Value::new_string_ptr(ptr as *mut ()))
                }
                _ => Ok(Value::new_int(handle.last_http_code as i32)),
            }
        } else {
            // Return full info array
            let mut arr = PhpArray::new();
            let http_code_id = ctx.intern_string("http_code");
            arr.insert_string_id(http_code_id, Value::new_int(handle.last_http_code as i32));

            let header_size_id = ctx.intern_string("header_size");
            arr.insert_string_id(header_size_id, Value::new_int(handle.last_header_size as i32));

            let size_download_id = ctx.intern_string("size_download");
            arr.insert_string_id(size_download_id, Value::new_int(handle.last_size_download as i32));

            let total_time_id = ctx.intern_string("total_time");
            arr.insert_string_id(total_time_id, Value::new_float(handle.total_time_secs));

            let url_id = ctx.intern_string("url");
            let url_ptr = ctx.get_arena().alloc_and_track(handle.url.clone());
            arr.insert_string_id(url_id, Value::new_string_ptr(url_ptr as *mut ()));

            let content_type_id = ctx.intern_string("content_type");
            let ct_ptr = ctx.get_arena().alloc_and_track(handle.last_content_type.clone());
            arr.insert_string_id(content_type_id, Value::new_string_ptr(ct_ptr as *mut ()));

            let ptr = ctx.get_arena().alloc_and_track(arr);
            Ok(Value::new_array_ptr(ptr as *mut ()))
        }
    }
}

php_function! {
    native_curl_error(ch: Value) |ctx| {
        let Some(handle_id) = get_handle_id(ch) else {
            let ptr = ctx.get_arena().alloc_and_track(String::new());
            return Ok(Value::new_string_ptr(ptr as *mut ()));
        };
        let guard = CURL_HANDLES.lock().unwrap();
        let err_msg = guard.get(&handle_id).map(|h| h.last_error.clone()).unwrap_or_default();
        let ptr = ctx.get_arena().alloc_and_track(err_msg);
        Ok(Value::new_string_ptr(ptr as *mut ()))
    }
}

php_function! {
    native_curl_errno(ch: Value) {
        let Some(handle_id) = get_handle_id(ch) else {
            return Ok(Value::new_int(0));
        };
        let guard = CURL_HANDLES.lock().unwrap();
        let errno = guard.get(&handle_id).map(|h| h.last_errno).unwrap_or(0);
        Ok(Value::new_int(errno as i32))
    }
}

php_function! {
    native_curl_close(ch: Value) {
        if let Some(handle_id) = get_handle_id(ch) {
            let mut guard = CURL_HANDLES.lock().unwrap();
            guard.remove(&handle_id);
        }
        Ok(Value::null())
    }
}

php_function! {
    native_curl_reset(ch: Value) {
        if let Some(handle_id) = get_handle_id(ch) {
            let mut guard = CURL_HANDLES.lock().unwrap();
            if let Some(handle) = guard.get_mut(&handle_id) {
                *handle = CurlHandleState::new(handle_id, String::new());
            }
        }
        Ok(Value::null())
    }
}

php_function! {
    native_curl_version() |ctx| {
        let mut arr = PhpArray::new();
        let ver_id = ctx.intern_string("version");
        let ver_ptr = ctx.get_arena().alloc_and_track("8.4.0".to_string());
        arr.insert_string_id(ver_id, Value::new_string_ptr(ver_ptr as *mut ()));

        let host_id = ctx.intern_string("host");
        let host_ptr = ctx.get_arena().alloc_and_track(format!("{}-hyperion", std::env::consts::OS));
        arr.insert_string_id(host_id, Value::new_string_ptr(host_ptr as *mut ()));

        let ssl_id = ctx.intern_string("ssl_version");
        let ssl_ptr = ctx.get_arena().alloc_and_track("rustls/0.23".to_string());
        arr.insert_string_id(ssl_id, Value::new_string_ptr(ssl_ptr as *mut ()));

        let ptr = ctx.get_arena().alloc_and_track(arr);
        Ok(Value::new_array_ptr(ptr as *mut ()))
    }
}

// --- Multi Handle Support ---

php_function! {
    native_curl_multi_init() {
        let id = next_id();
        {
            let mut guard = CURL_MULTI_HANDLES.lock().unwrap();
            guard.insert(id, CurlMultiState {
                handles: Vec::new(),
                completed: Vec::new(),
            });
        }
        let res = PhpResource::new(ResourceKind::CurlMulti(id), "curl_multi");
        let ptr = crate::into_raw(Box::new(res));
        Ok(Value::new_resource_ptr(ptr as *mut ()))
    }
}

php_function! {
    native_curl_multi_add_handle(mh: Value, ch: Value) {
        let (Some(multi_id), Some(handle_id)) = (get_multi_id(mh), get_handle_id(ch)) else {
            return Ok(Value::new_int(-1));
        };
        let mut guard = CURL_MULTI_HANDLES.lock().unwrap();
        if let Some(multi) = guard.get_mut(&multi_id) {
            if !multi.handles.contains(&handle_id) {
                multi.handles.push(handle_id);
            }
        }
        Ok(Value::new_int(0)) // CURLM_OK
    }
}

php_function! {
    native_curl_multi_remove_handle(mh: Value, ch: Value) {
        let (Some(multi_id), Some(handle_id)) = (get_multi_id(mh), get_handle_id(ch)) else {
            return Ok(Value::new_int(-1));
        };
        let mut guard = CURL_MULTI_HANDLES.lock().unwrap();
        if let Some(multi) = guard.get_mut(&multi_id) {
            multi.handles.retain(|&id| id != handle_id);
        }
        Ok(Value::new_int(0)) // CURLM_OK
    }
}

php_function! {
    native_curl_multi_exec(mh: Value, still_running: Value) {
        let Some(multi_id) = get_multi_id(mh) else {
            return Ok(Value::new_int(-1));
        };

        let handles_to_run = {
            let guard = CURL_MULTI_HANDLES.lock().unwrap();
            guard.get(&multi_id).map(|m| m.handles.clone()).unwrap_or_default()
        };

        let mut completed_list = Vec::new();

        for hid in handles_to_run {
            let mut handle = {
                let guard = CURL_HANDLES.lock().unwrap();
                match guard.get(&hid) {
                    Some(h) => h.clone(),
                    None => continue,
                }
            };

            let _ = handle.execute();
            completed_list.push((hid, handle.last_http_code));

            let mut guard = CURL_HANDLES.lock().unwrap();
            if let Some(h) = guard.get_mut(&hid) {
                *h = handle;
            }
        }

        {
            let mut guard = CURL_MULTI_HANDLES.lock().unwrap();
            if let Some(multi) = guard.get_mut(&multi_id) {
                multi.completed.extend(completed_list);
            }
        }

        // still_running is passed by reference: update it to 0
        if let Some(arg) = still_running {
            if let Some(ref_ptr) = arg.as_ref_ptr() {
                let r = unsafe { &mut *(ref_ptr as *mut PhpRef) };
                r.set(Value::new_int(0));
            }
        }

        Ok(Value::new_int(0)) // CURLM_OK
    }
}

php_function! {
    native_curl_multi_select(mh: Value, timeout: Value) {
        let _ = timeout;
        let _ = mh;
        Ok(Value::new_int(1))
    }
}

php_function! {
    native_curl_multi_info_read(mh: Value, msgs_in_queue: Value) |ctx| {
        let Some(multi_id) = get_multi_id(mh) else {
            return Ok(Value::new_bool(false));
        };

        let completed_opt = {
            let mut guard = CURL_MULTI_HANDLES.lock().unwrap();
            guard.get_mut(&multi_id).and_then(|m| {
                if !m.completed.is_empty() {
                    Some(m.completed.remove(0))
                } else {
                    None
                }
            })
        };

        if let Some((hid, _code)) = completed_opt {
            let mut arr = PhpArray::new();
            let msg_id = ctx.intern_string("msg");
            arr.insert_string_id(msg_id, Value::new_int(CURLMSG_DONE as i32));

            let res_id = ctx.intern_string("result");
            arr.insert_string_id(res_id, Value::new_int(CURLE_OK as i32));

            let handle_key_id = ctx.intern_string("handle");
            let res = PhpResource::new(ResourceKind::Curl(hid), "curl");
            let ptr = crate::into_raw(Box::new(res));
            arr.insert_string_id(handle_key_id, Value::new_resource_ptr(ptr as *mut ()));

            if let Some(arg) = msgs_in_queue {
                if let Some(ref_ptr) = arg.as_ref_ptr() {
                    let r = unsafe { &mut *(ref_ptr as *mut PhpRef) };
                    r.set(Value::new_int(0));
                }
            }

            let arr_ptr = ctx.get_arena().alloc_and_track(arr);
            Ok(Value::new_array_ptr(arr_ptr as *mut ()))
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_curl_multi_close(mh: Value) {
        if let Some(multi_id) = get_multi_id(mh) {
            let mut guard = CURL_MULTI_HANDLES.lock().unwrap();
            guard.remove(&multi_id);
        }
        Ok(Value::null())
    }
}

fn urlencoding_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}
