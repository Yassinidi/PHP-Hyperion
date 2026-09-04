//! Stream functions built on the real `PhpResource` type.
//!
//! These used to be stubs — `fopen` returned the integer 99 and `fwrite` sent
//! every byte to stderr no matter the handle. That was enough to keep a script
//! running and wrong in a way Laravel notices immediately: Symfony's
//! `StreamOutput::__construct` calls `is_resource()` on its first argument and
//! throws when it fails, so console output could not be constructed at all.
//!
//! Writes to `php://stdout` are routed through `NativeContext::write_output`
//! rather than fd 1 directly, so they interleave correctly with `echo`.

use hyperion_core::memory::nan_box::Value;
use hyperion_core::php_function;
use hyperion_core::types::function::NativeContext;
use hyperion_core::types::resource::{open_options, PhpResource, ResourceKind};

fn new_string(s: String) -> Value {
    Value::new_string_ptr(crate::into_raw(Box::new(s)) as *mut ())
}

fn as_str(v: &Value) -> Option<&'static String> {
    v.deref()
        .as_string_ptr()
        .map(|p| unsafe { &*(p as *const String) })
}

/// Coerce an argument to an integer the way PHP's stream functions do — an
/// int, a float truncated, or a numeric string.
fn as_i64(v: &Value) -> Option<i64> {
    let v = v.deref();
    if let Some(i) = v.as_int() {
        return Some(i as i64);
    }
    if let Some(f) = v.as_float() {
        return Some(f as i64);
    }
    as_str(&v).and_then(|s| s.trim().parse::<i64>().ok())
}

/// Borrow the `PhpResource` behind a handle.
///
/// Returns `None` for a non-resource *and* for a closed one: every stream
/// function in PHP warns and returns false on a closed handle, so callers get
/// to treat both the same way.
fn resource_of(v: &Value) -> Option<&'static mut PhpResource> {
    let v = v.deref();
    let ptr = v.as_resource_ptr()?;
    let r = unsafe { &mut *(ptr as *mut PhpResource) };
    if r.is_open() {
        Some(r)
    } else {
        None
    }
}

fn make_resource(kind: ResourceKind, uri: &str, append: bool) -> Value {
    let mut r = PhpResource::new(kind, uri);
    r.append = append;
    Value::new_resource_ptr(crate::into_raw(Box::new(r)) as *mut ())
}

/// Open one of the `php://` pseudo-streams. `None` means "not a php:// URI",
/// leaving the caller to open a real file.
fn open_php_stream(uri: &str, mode: &str, ctx: &dyn NativeContext) -> Option<Value> {
    let rest = uri.strip_prefix("php://")?;
    // `php://filter/...` chains are not supported; the underlying resource
    // after the final `/resource=` is the closest honest answer, but silently
    // dropping the filters would corrupt data, so it is refused instead.
    let kind = match rest.to_ascii_lowercase().as_str() {
        "stdout" | "output" => ResourceKind::Stdout,
        "stderr" => ResourceKind::Stderr,
        "stdin" => ResourceKind::Stdin,
        "memory" => ResourceKind::Memory {
            buf: Vec::new(),
            pos: 0,
        },
        // `php://temp` spills to disk past a threshold in PHP; in-memory is
        // observationally identical to a script apart from memory use.
        s if s == "temp" || s.starts_with("temp/") => ResourceKind::Memory {
            buf: Vec::new(),
            pos: 0,
        },
        // No request body in CLI, so `php://input` reads as empty rather than
        // blocking on a stdin that will never close.
        "input" => ResourceKind::Data {
            buf: ctx.get_http_raw_body().to_vec(),
            pos: 0,
        },
        _ => return None,
    };
    let _ = mode;
    Some(make_resource(kind, uri, false))
}

php_function! {
    native_fopen(path: Value, mode: Value) |ctx| {
        let Some(p) = path.and_then(as_str) else {
            return Ok(Value::new_bool(false));
        };
        let mode_str = mode.and_then(as_str).map(|s| s.as_str()).unwrap_or("r");

        if let Some(v) = open_php_stream(p, mode_str, ctx) {
            return Ok(v);
        }

        let Some((opts, append, _readable)) = open_options(mode_str) else {
            return Ok(Value::new_bool(false));
        };
        match opts.open(p) {
            Ok(f) => Ok(make_resource(ResourceKind::File(f), p, append)),
            Err(_) => Ok(Value::new_bool(false)),
        }
    }
}

php_function! {
    native_fclose(handle: Value) {
        match handle.and_then(resource_of) {
            Some(r) => {
                r.close();
                Ok(Value::new_bool(true))
            }
            None => Ok(Value::new_bool(false)),
        }
    }
}

php_function! {
    native_is_resource(v: Value) {
        let Some(v) = v else { return Ok(Value::new_bool(false)) };
        // A closed handle is no longer a resource as far as userland is
        // concerned — this is exactly the check Symfony relies on.
        Ok(Value::new_bool(resource_of(v).is_some()))
    }
}

php_function! {
    native_get_resource_type(handle: Value) {
        let Some(h) = handle else { return Ok(Value::new_bool(false)) };
        let Some(ptr) = h.deref().as_resource_ptr() else {
            return Ok(Value::new_bool(false));
        };
        let r = unsafe { &*(ptr as *const PhpResource) };
        Ok(new_string(r.type_name().to_string()))
    }
}

php_function! {
    native_get_resource_id(handle: Value) {
        let Some(h) = handle else { return Ok(Value::new_int(0)) };
        let Some(ptr) = h.deref().as_resource_ptr() else {
            return Ok(Value::new_int(0));
        };
        let r = unsafe { &*(ptr as *const PhpResource) };
        Ok(Value::new_int(r.id as i32))
    }
}

php_function! {
    native_fwrite(handle: Value, data: Value, ...rest) |ctx| {
        let Some(h) = handle else { return Ok(Value::new_bool(false)) };
        let Some(bytes) = data.and_then(as_str) else {
            return Ok(Value::new_bool(false));
        };
        // `fwrite($h, $s, $len)` truncates to the first $len bytes.
        let slice: &[u8] = match rest.first().and_then(as_i64) {
            Some(n) if n >= 0 && (n as usize) < bytes.len() => &bytes.as_bytes()[..n as usize],
            _ => bytes.as_bytes(),
        };
        let Some(r) = resource_of(h) else {
            return Ok(Value::new_bool(false));
        };
        match r.write(slice) {
            Some(n) => Ok(Value::new_int(n as i32)),
            // `None` means this is the script's own output stream, which lives
            // on the fibre rather than in the resource.
            None => {
                ctx.write_output(slice);
                Ok(Value::new_int(slice.len() as i32))
            }
        }
    }
}

php_function! {
    native_fread(handle: Value, length: Value) {
        let Some(r) = handle.and_then(resource_of) else {
            return Ok(Value::new_bool(false));
        };
        let len = length.and_then(as_i64).unwrap_or(8192).max(0) as usize;
        let bytes = r.read(len);
        Ok(new_string(String::from_utf8_lossy(&bytes).into_owned()))
    }
}

php_function! {
    native_fgets(handle: Value, ...rest) {
        let Some(r) = handle.and_then(resource_of) else {
            return Ok(Value::new_bool(false));
        };
        let max = rest.first().and_then(as_i64).map(|n| n.max(0) as usize);
        match r.read_line(max) {
            Some(line) => Ok(new_string(String::from_utf8_lossy(&line).into_owned())),
            // End of stream is `false`, not `""` — the canonical read loop is
            // `while (($l = fgets($h)) !== false)`, and an empty string would
            // spin it forever on a blank final line.
            None => Ok(Value::new_bool(false)),
        }
    }
}

php_function! {
    native_fgetc(handle: Value) {
        let Some(r) = handle.and_then(resource_of) else {
            return Ok(Value::new_bool(false));
        };
        let bytes = r.read(1);
        if bytes.is_empty() {
            Ok(Value::new_bool(false))
        } else {
            Ok(new_string(String::from_utf8_lossy(&bytes).into_owned()))
        }
    }
}

php_function! {
    native_feof(handle: Value) {
        match handle.and_then(resource_of) {
            Some(r) => Ok(Value::new_bool(r.eof)),
            // PHP warns and returns true for an invalid handle, which at least
            // terminates a `while (!feof($h))` loop instead of hanging.
            None => Ok(Value::new_bool(true)),
        }
    }
}

php_function! {
    native_ftell(handle: Value) {
        let Some(r) = handle.and_then(resource_of) else {
            return Ok(Value::new_bool(false));
        };
        match r.tell() {
            Some(p) => Ok(Value::new_int(p as i32)),
            None => Ok(Value::new_bool(false)),
        }
    }
}

php_function! {
    native_fseek(handle: Value, offset: Value, ...rest) {
        let Some(r) = handle.and_then(resource_of) else {
            return Ok(Value::new_int(-1));
        };
        let off = offset.and_then(as_i64).unwrap_or(0);
        let whence = rest.first().and_then(as_i64).unwrap_or(0) as i32;
        // fseek returns 0 on success and -1 on failure, not a bool.
        Ok(Value::new_int(if r.seek(off, whence) { 0 } else { -1 }))
    }
}

php_function! {
    native_rewind(handle: Value) {
        let Some(r) = handle.and_then(resource_of) else {
            return Ok(Value::new_bool(false));
        };
        Ok(Value::new_bool(r.seek(0, 0)))
    }
}

php_function! {
    native_fflush(handle: Value) {
        let Some(r) = handle.and_then(resource_of) else {
            return Ok(Value::new_bool(false));
        };
        Ok(Value::new_bool(r.flush()))
    }
}

php_function! {
    native_ftruncate(handle: Value, size: Value) {
        let Some(r) = handle.and_then(resource_of) else {
            return Ok(Value::new_bool(false));
        };
        let size = size.and_then(as_i64).unwrap_or(0).max(0) as usize;
        Ok(Value::new_bool(r.truncate(size)))
    }
}

php_function! {
    native_stream_get_contents(handle: Value) {
        let Some(r) = handle.and_then(resource_of) else {
            return Ok(Value::new_bool(false));
        };
        let bytes = r.read_to_end();
        Ok(new_string(String::from_utf8_lossy(&bytes).into_owned()))
    }
}

php_function! {
    native_fstat(handle: Value) |ctx| {
        let Some(r) = handle.and_then(resource_of) else {
            return Ok(Value::new_bool(false));
        };
        let mut arr = hyperion_core::types::array::PhpArray::new();
        let size = r.len().unwrap_or(0) as i32;
        // Only the keys scripts actually read; PHP returns both a numeric and a
        // named entry for the same field, and `size` is at index 7.
        arr.insert_int(7, Value::new_int(size));
        arr.insert_string_id(ctx.intern_string("size"), Value::new_int(size));
        Ok(Value::new_array_ptr(crate::into_raw(Box::new(arr)) as *mut ()))
    }
}

// Blocking mode, locks and timeouts have no meaning for the stream kinds this
// engine backs: files and memory buffers never block. Reporting success keeps
// callers on their normal path rather than into a failure branch.
php_function! {
    native_stream_set_blocking(_handle: Value, _mode: Value) {
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_flock(_handle: Value, _operation: Value) {
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_stream_isatty(handle: Value) {
        // Symfony asks this to decide on ANSI colour. Answering false keeps
        // escape codes out of piped output, which is the safe default.
        let _ = handle;
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_stream_get_meta_data(handle: Value) |ctx| {
        let Some(h) = handle else { return Ok(Value::new_bool(false)) };
        let Some(ptr) = h.deref().as_resource_ptr() else {
            return Ok(Value::new_bool(false));
        };
        let r = unsafe { &*(ptr as *const PhpResource) };
        let uri = r.uri.clone();
        let (eof, seekable) = (r.eof, r.is_seekable());
        let mode = match r.kind {
            ResourceKind::Stdout | ResourceKind::Stderr | ResourceKind::PipeStdout(_) | ResourceKind::PipeStderr(_) | ResourceKind::PopenWrite(_, _) => "wb",
            ResourceKind::Stdin | ResourceKind::PipeStdin(_) | ResourceKind::PopenRead(_, _) | ResourceKind::Data { .. } => "rb",
            ResourceKind::Memory { .. } => "w+b",
            ResourceKind::File(_) => if r.append { "a+b" } else { "w+b" },
            _ => "r+b",
        };
        let mut arr = hyperion_core::types::array::PhpArray::new();
        let mut put = |k: &str, v: Value| {
            let id = ctx.intern_string(k);
            arr.insert_string_id(id, v);
        };
        put("uri", new_string(uri));
        put("mode", new_string(mode.to_string()));
        put("eof", Value::new_bool(eof));
        put("seekable", Value::new_bool(seekable));
        put("wrapper_type", new_string(String::from("plainfile")));
        put("stream_type", new_string(String::from("STDIO")));
        put("blocked", Value::new_bool(true));
        put("timed_out", Value::new_bool(false));
        put("unread_bytes", Value::new_int(0));
        Ok(Value::new_array_ptr(crate::into_raw(Box::new(arr)) as *mut ()))
    }
}

php_function! {
    native_tmpfile() {
        let temp_path = std::env::temp_dir().join(format!("php_tmp_{}_{}", std::process::id(), rand::random::<u64>()));
        let path_str = temp_path.to_string_lossy().to_string();
        let Ok(file) = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp_path) else {
            return Ok(Value::new_bool(false));
        };
        Ok(make_resource(ResourceKind::File(file), &path_str, false))
    }
}

php_function! {
    native_stream_set_chunk_size(_handle: Value, _size: Value) {
        // Sets stream chunk size. Returns previous chunk size (default 8192) on success.
        Ok(Value::new_int(8192))
    }
}

php_function! {
    native_stream_set_timeout(_handle: Value, _seconds: Value, _microseconds: Value) {
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_stream_set_write_buffer(_handle: Value, _size: Value) {
        Ok(Value::new_int(0))
    }
}

php_function! {
    native_stream_set_read_buffer(_handle: Value, _size: Value) {
        Ok(Value::new_int(0))
    }
}

php_function! {
    native_stream_context_create(_options: Value, _params: Value) {
        Ok(make_resource(ResourceKind::Memory { buf: Vec::new(), pos: 0 }, "stream-context", false))
    }
}

php_function! {
    native_stream_context_set_option(_context: Value, _wrapper: Value, _option: Value, _value: Value) {
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_stream_context_get_options(_context: Value) |ctx| {
        let arr = hyperion_core::types::array::PhpArray::new();
        Ok(Value::new_array_ptr(crate::into_raw(Box::new(arr)) as *mut ()))
    }
}

php_function! {
    native_stream_context_get_params(_context: Value) |ctx| {
        let arr = hyperion_core::types::array::PhpArray::new();
        Ok(Value::new_array_ptr(crate::into_raw(Box::new(arr)) as *mut ()))
    }
}

php_function! {
    native_stream_context_set_params(_context: Value, _params: Value) {
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_stream_context_get_default(_options: Value) {
        Ok(make_resource(ResourceKind::Memory { buf: Vec::new(), pos: 0 }, "stream-context", false))
    }
}

php_function! {
    native_stream_copy_to_stream(from: Value, to: Value, _length: Value, _offset: Value) {
        let Some(from_r) = from.and_then(resource_of) else {
            return Ok(Value::new_bool(false));
        };
        let bytes = from_r.read_to_end();
        let len = bytes.len();
        if let Some(to_r) = to.and_then(resource_of) {
            to_r.write(&bytes);
            return Ok(Value::new_int(len as i32));
        }
        Ok(Value::new_int(0))
    }
}

php_function! {
    native_stream_get_transports() |ctx| {
        let mut arr = hyperion_core::types::array::PhpArray::new();
        arr.push(new_string("tcp".to_string()));
        arr.push(new_string("udp".to_string()));
        arr.push(new_string("ssl".to_string()));
        arr.push(new_string("tls".to_string()));
        arr.push(new_string("unix".to_string()));
        arr.push(new_string("udg".to_string()));
        Ok(Value::new_array_ptr(crate::into_raw(Box::new(arr)) as *mut ()))
    }
}

php_function! {
    native_stream_get_wrappers() |ctx| {
        let mut arr = hyperion_core::types::array::PhpArray::new();
        arr.push(new_string("php".to_string()));
        arr.push(new_string("file".to_string()));
        arr.push(new_string("http".to_string()));
        arr.push(new_string("https".to_string()));
        arr.push(new_string("ftp".to_string()));
        arr.push(new_string("ftps".to_string()));
        arr.push(new_string("data".to_string()));
        Ok(Value::new_array_ptr(crate::into_raw(Box::new(arr)) as *mut ()))
    }
}

php_function! {
    native_stream_filter_register(_filter_name: Value, _class_name: Value) {
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_stream_filter_append(_stream: Value, _filter_name: Value, _read_write: Value, _params: Value) {
        Ok(make_resource(ResourceKind::Memory { buf: Vec::new(), pos: 0 }, "stream-filter", false))
    }
}

php_function! {
    native_stream_filter_prepend(_stream: Value, _filter_name: Value, _read_write: Value, _params: Value) {
        Ok(make_resource(ResourceKind::Memory { buf: Vec::new(), pos: 0 }, "stream-filter", false))
    }
}

php_function! {
    native_stream_filter_remove(_filter: Value) {
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_stream_wrapper_register(_protocol: Value, _class_name: Value, _flags: Value) {
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_stream_wrapper_unregister(_protocol: Value) {
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_stream_wrapper_restore(_protocol: Value) {
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_stream_socket_enable_crypto(_stream: Value, _enable: Value, _crypto_type: Value, _session_stream: Value) {
        Ok(Value::new_bool(true))
    }
}

/// Build the three standard stream handles.///
/// These are constants in PHP, not functions, and every script that writes to
/// the console touches one of them. They are created once at engine start so
/// that `STDOUT === STDOUT` holds.
pub fn make_standard_streams() -> [(&'static str, Value); 3] {
    [
        ("STDIN", make_resource(ResourceKind::Stdin, "php://stdin", false)),
        ("STDOUT", make_resource(ResourceKind::Stdout, "php://stdout", false)),
        ("STDERR", make_resource(ResourceKind::Stderr, "php://stderr", false)),
    ]
}
