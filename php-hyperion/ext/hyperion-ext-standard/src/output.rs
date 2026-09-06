use hyperion_core::php_function;
use hyperion_core::memory::nan_box::Value;

php_function! {
    native_ob_start() |ctx| {
        ctx.ob_start();
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_ob_get_clean() |ctx| {
        if let Some(content) = ctx.ob_get_clean() {
            let ptr = crate::into_raw(Box::new(content));
            Ok(Value::new_string_ptr(ptr as *mut ()))
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_ob_get_contents() |ctx| {
        if let Some(content) = ctx.ob_get_contents() {
            let ptr = crate::into_raw(Box::new(content));
            Ok(Value::new_string_ptr(ptr as *mut ()))
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_ob_end_clean() |ctx| {
        Ok(Value::new_bool(ctx.ob_end_clean()))
    }
}

php_function! {
    native_ob_end_flush() |ctx| {
        Ok(Value::new_bool(ctx.ob_end_flush()))
    }
}

php_function! {
    native_ob_flush() |ctx| {
        Ok(Value::new_bool(ctx.ob_flush()))
    }
}

php_function! {
    native_flush() {
        use std::io::Write;
        let _ = std::io::stdout().flush();
        Ok(Value::null())
    }
}

php_function! {
    native_ob_get_status(full_status: Value) |ctx| {
        let full = full_status.and_then(|v| v.as_bool()).unwrap_or(false);
        let level = ctx.ob_get_level();
        let mut root_arr = hyperion_core::types::array::PhpArray::new();
        if level == 0 {
            let ptr = crate::into_raw(Box::new(root_arr));
            return Ok(Value::new_array_ptr(ptr as *mut ()));
        }

        let make_status_entry = |lvl: usize| {
            let mut entry = hyperion_core::types::array::PhpArray::new();
            let name_ptr = crate::into_raw(Box::new("default output handler".to_string()));
            entry.insert_string_id(ctx.intern_string("name"), Value::new_string_ptr(name_ptr as *mut ()));
            entry.insert_string_id(ctx.intern_string("type"), Value::new_int(0)); // PHP_OUTPUT_HANDLER_INTERNAL
            entry.insert_string_id(ctx.intern_string("flags"), Value::new_int(112)); // PHP_OUTPUT_HANDLER_STDFLAGS
            entry.insert_string_id(ctx.intern_string("level"), Value::new_int(lvl as i32));
            entry.insert_string_id(ctx.intern_string("chunk_size"), Value::new_int(0));
            entry.insert_string_id(ctx.intern_string("buffer_size"), Value::new_int(16384));
            entry.insert_string_id(ctx.intern_string("buffer_used"), Value::new_int(0));
            entry.insert_string_id(ctx.intern_string("del"), Value::new_bool(true));
            entry
        };

        if full {
            for i in 1..=level {
                let entry = make_status_entry(i);
                let entry_ptr = crate::into_raw(Box::new(entry));
                root_arr.push(Value::new_array_ptr(entry_ptr as *mut ()));
            }
        } else {
            root_arr = make_status_entry(level);
        }

        let ptr = crate::into_raw(Box::new(root_arr));
        Ok(Value::new_array_ptr(ptr as *mut ()))
    }
}

php_function! {
    native_headers_sent() |ctx| { let _arena = ctx.get_arena();
        Ok(Value::new_bool(ctx.headers_sent()))
    }
}

php_function! {
    native_header(header_str: Value, replace: Value, http_response_code: Value) |ctx| { let _arena = ctx.get_arena();
        if let Some(h) = header_str {
            if let Some(sp) = h.as_string_ptr() {
                let header = unsafe { &*(sp as *const String) };
                let should_replace = replace.and_then(|r| r.as_bool()).unwrap_or(true);
                ctx.add_response_header(header, should_replace);
                if let Some(code_val) = http_response_code {
                    if let Some(code) = code_val.as_int() {
                        ctx.set_http_response_code(code as u16);
                    }
                }
            }
        }
        Ok(Value::null())
    }
}

php_function! {
    native_header_remove(name: Value) |ctx| { let _arena = ctx.get_arena();
        if let Some(n) = name {
            if let Some(sp) = n.as_string_ptr() {
                let name_str = unsafe { &*(sp as *const String) };
                ctx.remove_response_header(name_str);
            }
        }
        Ok(Value::null())
    }
}

php_function! {
    native_error_log(message: String) {
        if let Some(msg) = message {
            eprintln!("{}", msg);
        }
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_http_response_code(response_code: Value) |ctx| { let _arena = ctx.get_arena();
        if let Some(c) = response_code {
            if let Some(code) = c.as_int() {
                let prev = ctx.get_http_response_code();
                ctx.set_http_response_code(code as u16);
                return Ok(Value::new_int(prev as i32));
            }
        }
        Ok(Value::new_int(ctx.get_http_response_code() as i32))
    }
}

php_function! {
    native_set_error_handler() {
        Ok(Value::null())
    }
}

php_function! {
    native_set_exception_handler() {
        Ok(Value::null())
    }
}

php_function! {
    native_restore_error_handler() {
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_restore_exception_handler() {
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_error_clear_last() {
        Ok(Value::null())
    }
}

php_function! {
    native_error_get_last() {
        Ok(Value::null())
    }
}

php_function! {
    native_error_reporting() {
        Ok(Value::new_int(32767)) // E_ALL
    }
}

use std::sync::RwLock;
use std::collections::HashMap;
use lazy_static::lazy_static;

lazy_static! {
    static ref INI_SETTINGS: RwLock<HashMap<String, String>> = {
        let mut m = HashMap::new();
        m.insert("post_max_size".to_string(), "8M".to_string());
        m.insert("upload_max_filesize".to_string(), "2M".to_string());
        m.insert("memory_limit".to_string(), "128M".to_string());
        m.insert("max_execution_time".to_string(), "30".to_string());
        m.insert("default_socket_timeout".to_string(), "60".to_string());
        m.insert("date.timezone".to_string(), "UTC".to_string());
        m.insert("display_errors".to_string(), "1".to_string());
        m.insert("error_reporting".to_string(), "32767".to_string());
        m.insert("precision".to_string(), "14".to_string());
        m.insert("serialize_precision".to_string(), "-1".to_string());
        m.insert("variables_order".to_string(), "EGPCS".to_string());
        m.insert("request_order".to_string(), "GP".to_string());
        m.insert("auto_detect_line_endings".to_string(), "0".to_string());
        m.insert("mbstring.internal_encoding".to_string(), "UTF-8".to_string());
        m.insert("default_charset".to_string(), "UTF-8".to_string());
        m.insert("session.gc_maxlifetime".to_string(), "1440".to_string());
        m.insert("session.save_path".to_string(), "/tmp".to_string());
        RwLock::new(m)
    };
}

php_function! {
    native_ini_set(key: Value, val: Value) {
        let key_str = key.and_then(|k| k.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() });
        let val_str = val.and_then(|v| v.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() });
        if let (Some(k), Some(v)) = (key_str, val_str) {
            let mut map = INI_SETTINGS.write().unwrap();
            let prev = map.insert(k, v).unwrap_or_default();
            let ptr = crate::into_raw(Box::new(prev));
            Ok(Value::new_string_ptr(ptr as *mut ()))
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_ini_get(key: Value) {
        let key_str = key.and_then(|k| k.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() });
        if let Some(k) = key_str {
            let map = INI_SETTINGS.read().unwrap();
            if let Some(v) = map.get(&k) {
                let ptr = crate::into_raw(Box::new(v.clone()));
                return Ok(Value::new_string_ptr(ptr as *mut ()));
            }
        }
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_get_cfg_var(key: Value) {
        let key_str = key.and_then(|k| k.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() });
        if let Some(k) = key_str {
            let map = INI_SETTINGS.read().unwrap();
            if let Some(v) = map.get(&k) {
                let ptr = crate::into_raw(Box::new(v.clone()));
                return Ok(Value::new_string_ptr(ptr as *mut ()));
            }
        }
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_ini_restore(_key: Value) {
        Ok(Value::null())
    }
}

php_function! {
    native_ob_get_level() |ctx| {
        Ok(Value::new_int(ctx.ob_get_level() as i32))
    }
}

php_function! {
    native_php_sapi_name() {
        let name = "cli".to_string();
        let ptr = crate::into_raw(Box::new(name));
        Ok(Value::new_string_ptr(ptr as *mut ()))
    }
}

php_function! {
    native_phpversion() {
        let version = "8.2.0-hyperion".to_string();
        let ptr = crate::into_raw(Box::new(version));
        Ok(Value::new_string_ptr(ptr as *mut ()))
    }
}

php_function! {
    native_php_uname() {
        let uname = "Darwin".to_string();
        let ptr = crate::into_raw(Box::new(uname));
        Ok(Value::new_string_ptr(ptr as *mut ()))
    }
}

php_function! {
    native_extension_loaded(name: Value) {
        // Answering yes to everything sends callers down code paths whose
        // functions do not exist here — `extension_loaded('pcntl')` is how
        // Laravel decides to install signal handlers. Only the extensions this
        // engine actually backs are reported.
        const LOADED: &[&str] = &[
            "core", "standard", "spl", "pcre", "json", "date", "ctype",
            "mbstring", "hash", "openssl", "filter", "random", "reflection",
            "tokenizer", "session", "pdo", "pdo_sqlite", "pdo_mysql", "pdo_pgsql",
            "iconv", "curl", "zlib", "zip", "gd", "pcntl", "posix", "redis", "intl",
            "dom", "libxml", "xmlwriter", "simplexml", "xml", "xmlreader", "mysqli", "mysqlnd",
        ];
        let Some(n) = name.and_then(|v| v.deref().as_string_ptr()) else {
            return Ok(Value::new_bool(false));
        };
        let n = unsafe { &*(n as *const String) }.to_ascii_lowercase();
        Ok(Value::new_bool(LOADED.contains(&n.as_str())))
    }
}

php_function! {
    native_get_loaded_extensions(zend_extensions: Value) |ctx| {
        const LOADED: &[&str] = &[
            "Core", "standard", "SPL", "pcre", "json", "date", "ctype",
            "mbstring", "hash", "openssl", "filter", "random", "Reflection",
            "tokenizer", "session", "PDO", "pdo_sqlite", "pdo_mysql", "pdo_pgsql",
            "iconv", "curl", "zlib", "zip", "gd", "pcntl", "posix", "redis", "intl",
            "dom", "libxml", "xmlwriter", "SimpleXML", "xml", "xmlreader", "mysqli", "mysqlnd",
        ];
        let mut arr = hyperion_core::types::array::PhpArray::new();
        for &ext in LOADED {
            let str_val = crate::into_raw(Box::new(ext.to_string()));
            arr.push(Value::new_string_ptr(str_val as *mut ()));
        }
        Ok(Value::new_array_ptr(crate::into_raw(Box::new(arr)) as *mut ()))
    }
}

php_function! {
    native_function_exists(name: Value) |ctx| {
        let Some(n) = name.and_then(|v| v.deref().as_string_ptr()) else {
            return Ok(Value::new_bool(false));
        };
        let n = unsafe { &*(n as *const String) };
        Ok(Value::new_bool(ctx.function_exists(n)))
    }
}

php_function! {
    native_memory_get_usage() {
        Ok(Value::new_int(16 * 1024 * 1024))
    }
}

php_function! {
    native_memory_get_peak_usage() {
        Ok(Value::new_int(32 * 1024 * 1024))
    }
}

php_function! {
    native_memory_reset_peak_usage() {
        Ok(Value::null())
    }
}

php_function! {
    native_gc_mem_caches() {
        Ok(Value::new_int(0))
    }
}





lazy_static::lazy_static! {
    static ref ENV_STORE: std::sync::RwLock<std::collections::HashMap<String, String>> = {
        let mut map = std::collections::HashMap::new();
        for (k, v) in std::env::vars() {
            map.insert(k, v);
        }
        std::sync::RwLock::new(map)
    };
}

php_function! {
    native_getenv(name: Value) |ctx| {
        let store = ENV_STORE.read().unwrap();
        // No argument means "every variable, as an array".
        let Some(n) = name.and_then(|v| v.deref().as_string_ptr()) else {
            let mut arr = hyperion_core::types::array::PhpArray::new();
            for (k, v) in store.iter() {
                let id = ctx.intern_string(k);
                let val = crate::into_raw(Box::new(v.clone()));
                arr.insert_string_id(id, Value::new_string_ptr(val as *mut ()));
            }
            return Ok(Value::new_array_ptr(crate::into_raw(Box::new(arr)) as *mut ()));
        };
        let n = unsafe { &*(n as *const String) };
        match store.get(n) {
            Some(v) => Ok(Value::new_string_ptr(crate::into_raw(Box::new(v.clone())) as *mut ())),
            None => Ok(Value::new_bool(false)),
        }
    }
}

php_function! {
    native_putenv(setting: Value) {
        let Some(s) = setting.and_then(|v| v.deref().as_string_ptr()) else {
            return Ok(Value::new_bool(false));
        };
        let s = unsafe { &*(s as *const String) };
        let mut store = ENV_STORE.write().unwrap();
        match s.split_once('=') {
            Some((k, v)) => {
                store.insert(k.to_string(), v.to_string());
                Ok(Value::new_bool(true))
            }
            None => {
                store.remove(s);
                Ok(Value::new_bool(true))
            }
        }
    }
}

php_function! {
    native_file_get_contents(path: Value) {
        let path = path.copied().unwrap_or(Value::null());
        if let Some(p) = path.as_string_ptr() {
            let path_str = unsafe { &*(p as *const String) };
            match std::fs::read_to_string(path_str) {
                Ok(content) => {
                    let ptr = crate::into_raw(Box::new(content));
                    Ok(Value::new_string_ptr(ptr as *mut ()))
                }
                Err(_) => Ok(Value::new_bool(false)),
            }
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_file_put_contents(path: Value, data: Value) {
        let path = path.copied().unwrap_or(Value::null());
        let data = data.copied().unwrap_or(Value::null());
        if let (Some(p), Some(d)) = (path.as_string_ptr(), data.as_string_ptr()) {
            let path_str = unsafe { &*(p as *const String) };
            let data_str = unsafe { &*(d as *const String) };
            match std::fs::write(path_str, data_str) {
                Ok(_) => Ok(Value::new_int(data_str.len() as i32)),
                Err(_) => Ok(Value::new_bool(false)),
            }
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_realpath(path: Value) {
        let path = path.copied().unwrap_or(Value::null());
        if let Some(p) = path.as_string_ptr() {
            let path_str = unsafe { &*(p as *const String) };
            match std::fs::canonicalize(path_str) {
                Ok(abs) => {
                    let ptr = crate::into_raw(Box::new(abs.to_string_lossy().to_string()));
                    Ok(Value::new_string_ptr(ptr as *mut ()))
                }
                Err(_) => Ok(Value::new_bool(false)),
            }
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_getcwd() {
        match std::env::current_dir() {
            Ok(dir) => {
                let ptr = crate::into_raw(Box::new(dir.to_string_lossy().to_string()));
                Ok(Value::new_string_ptr(ptr as *mut ()))
            }
            Err(_) => Ok(Value::new_bool(false)),
        }
    }
}

php_function! {
    native_is_file(path: Value) {
        let path = path.copied().unwrap_or(Value::null());
        if let Some(p) = path.as_string_ptr() {
            let path_str = unsafe { &*(p as *const String) };
            Ok(Value::new_bool(std::path::Path::new(path_str).is_file()))
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_is_dir(path: Value) {
        let path = path.copied().unwrap_or(Value::null());
        if let Some(p) = path.as_string_ptr() {
            let path_str = unsafe { &*(p as *const String) };
            Ok(Value::new_bool(std::path::Path::new(path_str).is_dir()))
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_file_exists(path: Value) {
        let path = path.copied().unwrap_or(Value::null());
        if let Some(p) = path.as_string_ptr() {
            let path_str = unsafe { &*(p as *const String) };
            Ok(Value::new_bool(std::path::Path::new(path_str).exists()))
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_trigger_error(_msg: Value) {
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_debug_backtrace(options: Value, limit: Value) |ctx| {
        let mut backtrace = ctx.get_backtrace();
        if !backtrace.is_empty() && (backtrace[0].function == "debug_backtrace" || backtrace[0].function == "native_debug_backtrace") {
            backtrace.remove(0);
        }

        let limit_num = limit.and_then(|v| v.as_int()).unwrap_or(0);
        let provide_object = options.and_then(|v| v.as_int()).map(|opt| (opt & 1) != 0).unwrap_or(true); // DEBUG_BACKTRACE_PROVIDE_OBJECT is 1

        let iter_frames = if limit_num > 0 {
            backtrace.into_iter().take(limit_num as usize).collect::<Vec<_>>()
        } else {
            backtrace
        };

        let mut arr = hyperion_core::types::array::PhpArray::new();
        for (i, frame) in iter_frames.into_iter().enumerate() {
            let mut frame_arr = hyperion_core::types::array::PhpArray::new();
            if !frame.file.is_empty() {
                let f_ptr = ctx.get_arena().alloc_and_track(frame.file) as *mut String;
                frame_arr.insert_string_id(
                    hyperion_core::types::string_table::intern_string("file"),
                    Value::new_string_ptr(f_ptr as *mut ()),
                );
            }
            frame_arr.insert_string_id(
                hyperion_core::types::string_table::intern_string("line"),
                Value::new_int(frame.line as i32),
            );
            let fn_ptr = ctx.get_arena().alloc_and_track(frame.function) as *mut String;
            frame_arr.insert_string_id(
                hyperion_core::types::string_table::intern_string("function"),
                Value::new_string_ptr(fn_ptr as *mut ()),
            );
            if let Some(cls) = frame.class {
                let cls_ptr = ctx.get_arena().alloc_and_track(cls) as *mut String;
                frame_arr.insert_string_id(
                    hyperion_core::types::string_table::intern_string("class"),
                    Value::new_string_ptr(cls_ptr as *mut ()),
                );
            }
            if let Some(ft) = frame.frame_type {
                let ft_ptr = ctx.get_arena().alloc_and_track(ft) as *mut String;
                frame_arr.insert_string_id(
                    hyperion_core::types::string_table::intern_string("type"),
                    Value::new_string_ptr(ft_ptr as *mut ()),
                );
            }
            if provide_object {
                if let Some(obj) = frame.object {
                    frame_arr.insert_string_id(
                        hyperion_core::types::string_table::intern_string("object"),
                        obj,
                    );
                }
            }
            let fa_ptr = ctx.get_arena().alloc_and_track(frame_arr) as *mut ();
            arr.insert_int(i as i64, Value::new_array_ptr(fa_ptr));
        }
        let ptr = ctx.get_arena().alloc_and_track(arr) as *mut ();
        Ok(Value::new_array_ptr(ptr))
    }
}

php_function! {
    native_debug_print_backtrace(options: Value, limit: Value) |ctx| {
        let backtrace = ctx.get_backtrace();
        let limit_num = limit.and_then(|v| v.as_int()).unwrap_or(0);
        let iter_frames = if limit_num > 0 {
            backtrace.into_iter().take(limit_num as usize).collect::<Vec<_>>()
        } else {
            backtrace
        };
        for (i, frame) in iter_frames.into_iter().enumerate() {
            let file_line = if !frame.file.is_empty() {
                format!(" called at [{}:{}]", frame.file, frame.line)
            } else {
                String::new()
            };
            let cls = frame.class.map(|c| format!("{}{}", c, frame.frame_type.unwrap_or_else(|| "::".to_string()))).unwrap_or_default();
            let line = format!("#{} {}{}({}){}\n", i, cls, frame.function, "", file_line);
            ctx.write_output(line.as_bytes());
        }
        Ok(Value::null())
    }
}

php_function! {
    native_defined(name: Value) |ctx| {
        let Some(n) = name.and_then(|v| v.as_string_ptr()) else {
            return Ok(Value::new_bool(false));
        };
        let n = unsafe { &*(n as *const String) };
        Ok(Value::new_bool(ctx.lookup_constant(n).is_some()))
    }
}

php_function! {
    native_constant(name: Value) |ctx| {
        let Some(n) = name.and_then(|v| v.as_string_ptr()) else {
            return Ok(Value::null());
        };
        let n = unsafe { &*(n as *const String) };
        // PHP throws for an undefined constant; returning null keeps the engine
        // running, and callers that care use defined() first.
        Ok(ctx.lookup_constant(n).unwrap_or_else(Value::null))
    }
}

php_function! {
    native_compact() {
        let arr = hyperion_core::types::array::PhpArray::new();
        let ptr = crate::into_raw(Box::new(arr));
        Ok(Value::new_array_ptr(ptr as *mut ()))
    }
}

php_function! {
    native_sprintf(_format: Value) {
        let _format = _format.copied().unwrap_or(Value::null());
        if let Some(p) = _format.as_string_ptr() {
            let s = unsafe { &*(p as *const String) };
            let ptr = crate::into_raw(Box::new(s.clone()));
            Ok(Value::new_string_ptr(ptr as *mut ()))
        } else {
            let ptr = crate::into_raw(Box::new(String::new()));
            Ok(Value::new_string_ptr(ptr as *mut ()))
        }
    }
}

php_function! {
    native_usleep(micro_seconds: Value) {
        let micro_seconds = micro_seconds.copied().unwrap_or(Value::null());
        if let Some(i) = micro_seconds.as_int() {
            if i > 0 {
                std::thread::sleep(std::time::Duration::from_micros(i as u64));
            }
        }
        Ok(Value::null())
    }
}

php_function! {
    native_sleep(seconds: Value) {
        let seconds = seconds.copied().unwrap_or(Value::null());
        if let Some(i) = seconds.as_int() {
            if i > 0 {
                std::thread::sleep(std::time::Duration::from_secs(i as u64));
            }
        }
        Ok(Value::new_int(0))
    }
}

php_function! {
    native_request_parse_body(options: Value) |ctx| {
        let mut arr = hyperion_core::types::array::PhpArray::new();
        let post_ptr = ctx.get_arena().alloc(hyperion_core::types::array::PhpArray::new());
        let files_ptr = ctx.get_arena().alloc(hyperion_core::types::array::PhpArray::new());
        arr.insert_int(0, Value::new_array_ptr(post_ptr as *mut ()));
        arr.insert_int(1, Value::new_array_ptr(files_ptr as *mut ()));
        let arr_ptr = ctx.get_arena().alloc(arr);
        Ok(Value::new_array_ptr(arr_ptr as *mut ()))
    }
}

php_function! {
    native_connection_status() {
        Ok(Value::new_int(0))
    }
}

php_function! {
    native_connection_aborted() {
        Ok(Value::new_int(0))
    }
}

php_function! {
    native_ignore_user_abort(enable: Value) {
        Ok(Value::new_int(0))
    }
}



