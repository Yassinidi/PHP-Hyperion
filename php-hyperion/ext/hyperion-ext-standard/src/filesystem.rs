use hyperion_core::php_function;
use hyperion_core::memory::nan_box::Value;
use std::path::Path;
use std::fs;

php_function! {
    native_file_exists(filename: Value) {
        if let Some(v) = filename {
            let vd = v.deref();
            if let Some(ptr) = vd.as_string_ptr() {
                let s = unsafe { &*(ptr as *const String) };
                Ok(Value::new_bool(Path::new(s.as_str()).exists()))
            } else {
                Ok(Value::new_bool(false))
            }
        } else {
            Err("file_exists() expects exactly 1 parameter".to_string())
        }
    }
}

pub fn native_touch(
    args: &[Value],
    _ctx: &mut dyn hyperion_core::types::function::NativeContext,
) -> Result<Value, String> {
    if args.is_empty() {
        return Err("touch() expects at least 1 parameter, 0 given".to_string());
    }
    let f_val = args[0].deref();
    if let Some(ptr) = f_val.as_string_ptr() {
        let f = unsafe { &*(ptr as *const String) };
        let path = Path::new(f.as_str());
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                let _ = fs::create_dir_all(parent);
            }
        }
        if !path.exists() {
            match fs::File::create(path) {
                Ok(_) => Ok(Value::new_bool(true)),
                Err(_) => Ok(Value::new_bool(false)),
            }
        } else {
            match fs::OpenOptions::new().write(true).open(path) {
                Ok(_) => Ok(Value::new_bool(true)),
                Err(_) => Ok(Value::new_bool(false)),
            }
        }
    } else {
        Err("touch() expects parameter 1 to be string".to_string())
    }
}

php_function! {
    native_is_file(filename: Value) {
        if let Some(v) = filename {
            let vd = v.deref();
            if let Some(ptr) = vd.as_string_ptr() {
                let s = unsafe { &*(ptr as *const String) };
                Ok(Value::new_bool(Path::new(s.as_str()).is_file()))
            } else {
                Ok(Value::new_bool(false))
            }
        } else {
            Err("is_file() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_is_dir(filename: Value) {
        if let Some(v) = filename {
            let vd = v.deref();
            if let Some(ptr) = vd.as_string_ptr() {
                let s = unsafe { &*(ptr as *const String) };
                Ok(Value::new_bool(Path::new(s.as_str()).is_dir()))
            } else {
                Ok(Value::new_bool(false))
            }
        } else {
            Err("is_dir() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_file_get_contents(filename: String) |ctx| {
        if let Some(f) = filename {
            if f == "php://input" {
                let raw_body = ctx.get_http_raw_body();
                let content = unsafe { String::from_utf8_unchecked(raw_body.to_vec()) };
                let boxed = crate::into_raw(Box::new(content));
                return Ok(Value::new_string_ptr(boxed as *mut ()));
            }
            match fs::read(f.as_str()) {
                Ok(bytes) => {
                    let content = unsafe { String::from_utf8_unchecked(bytes) };
                    let boxed = crate::into_raw(Box::new(content));
                    Ok(Value::new_string_ptr(boxed as *mut ()))
                }
                Err(_) => Ok(Value::new_bool(false)),
            }
        } else {
            Err("file_get_contents() expects at least 1 parameter".to_string())
        }
    }
}

php_function! {
    native_file_put_contents(filename: Value, data: Value, ...rest) {
        let Some(f_val) = filename else {
            return Err("file_put_contents() expects at least 2 parameters".to_string());
        };
        let Some(d_val) = data else {
            return Err("file_put_contents() expects at least 2 parameters".to_string());
        };

        let f_deref = f_val.deref();
        let Some(f_ptr) = f_deref.as_string_ptr() else {
            return Ok(Value::new_bool(false));
        };
        let file_path = unsafe { &*(f_ptr as *const String) };

        let mut bytes = Vec::new();
        let d_deref = d_val.deref();
        if let Some(s_ptr) = d_deref.as_string_ptr() {
            let s = unsafe { &*(s_ptr as *const String) };
            bytes.extend_from_slice(s.as_bytes());
        } else if let Some(r_ptr) = d_deref.as_resource_ptr() {
            let r = unsafe { &mut *(r_ptr as *mut hyperion_core::types::resource::PhpResource) };
            bytes = r.read_to_end();
        } else if let Some(i) = d_deref.as_int() {
            bytes.extend_from_slice(i.to_string().as_bytes());
        } else if let Some(f) = d_deref.as_float() {
            bytes.extend_from_slice(f.to_string().as_bytes());
        } else if let Some(arr_ptr) = d_deref.as_array_ptr() {
            let arr = unsafe { &*(arr_ptr as *const hyperion_core::types::array::PhpArray) };
            for item in arr.elements.values() {
                let id = item.deref();
                if let Some(s_ptr) = id.as_string_ptr() {
                    let s = unsafe { &*(s_ptr as *const String) };
                    bytes.extend_from_slice(s.as_bytes());
                } else if let Some(i) = id.as_int() {
                    bytes.extend_from_slice(i.to_string().as_bytes());
                }
            }
        }

        let flags = rest.first().copied().and_then(|v| v.as_int()).unwrap_or(0);
        let is_append = (flags & 8) != 0;

        let mut opts = fs::OpenOptions::new();
        opts.create(true).write(true);
        if is_append {
            opts.append(true);
        } else {
            opts.truncate(true);
        }

        match opts.open(file_path.as_str()) {
            Ok(mut file) => {
                use std::io::Write;
                match file.write_all(&bytes) {
                    Ok(_) => Ok(Value::new_int(bytes.len() as i32)),
                    Err(_) => Ok(Value::new_bool(false)),
                }
            }
            Err(_) => Ok(Value::new_bool(false)),
        }
    }
}

php_function! {
    native_unlink(filename: String) {
        if let Some(f) = filename {
            match fs::remove_file(f.as_str()) {
                Ok(_) => Ok(Value::new_bool(true)),
                Err(_) => Ok(Value::new_bool(false)),
            }
        } else {
            Err("unlink() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_mkdir(filename: String, mode: Value, recursive: Value) {
        if let Some(f) = filename {
            let is_recursive = recursive.and_then(|v| {
                if let Some(b) = v.as_bool() {
                    Some(b)
                } else if let Some(i) = v.as_int() {
                    Some(i != 0)
                } else {
                    None
                }
            }).unwrap_or(false);

            let res = if is_recursive {
                fs::create_dir_all(f.as_str())
            } else {
                fs::create_dir(f.as_str())
            };

            match res {
                Ok(_) => Ok(Value::new_bool(true)),
                Err(_) => Ok(Value::new_bool(false)),
            }
        } else {
            Err("mkdir() expects at least 1 parameter".to_string())
        }
    }
}

php_function! {
    native_rmdir(filename: String) {
        if let Some(f) = filename {
            match fs::remove_dir(f.as_str()) {
                Ok(_) => Ok(Value::new_bool(true)),
                Err(_) => Ok(Value::new_bool(false)),
            }
        } else {
            Err("rmdir() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_filesize(filename: String) {
        if let Some(f) = filename {
            match fs::metadata(f.as_str()) {
                Ok(meta) => Ok(Value::new_int(meta.len() as i32)),
                Err(_) => Ok(Value::new_bool(false)),
            }
        } else {
            Err("filesize() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_realpath(filename: String) {
        if let Some(f) = filename {
            match fs::canonicalize(f.as_str()) {
                Ok(path) => {
                    let s = path.to_string_lossy().into_owned();
                    let boxed = crate::into_raw(Box::new(s));
                    Ok(Value::new_string_ptr(boxed as *mut ()))
                }
                Err(_) => Ok(Value::new_bool(false)),
            }
        } else {
            Err("realpath() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_basename(path: String, suffix: String) {
        if let Some(p) = path {
            let mut b = Path::new(p.as_str()).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            if let Some(suf) = suffix {
                if !suf.is_empty() && b.ends_with(suf.as_str()) {
                    b.truncate(b.len() - suf.len());
                }
            }
            let boxed = crate::into_raw(Box::new(b));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        } else {
            Err("basename() expects at least 1 parameter".to_string())
        }
    }
}

php_function! {
    native_dirname(path: String) {
        if let Some(p) = path {
            let d = Path::new(p.as_str()).parent().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| ".".to_string());
            let boxed = crate::into_raw(Box::new(d));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        } else {
            Err("dirname() expects at least 1 parameter".to_string())
        }
    }
}

php_function! {
    native_stream_resolve_include_path(filename: String) {
        if let Some(f) = filename {
            let path = Path::new(f.as_str());
            if path.exists() {
                match fs::canonicalize(path) {
                    Ok(abs) => {
                        let s = abs.to_string_lossy().into_owned();
                        let boxed = crate::into_raw(Box::new(s));
                        Ok(Value::new_string_ptr(boxed as *mut ()))
                    }
                    Err(_) => {
                        // Return the original path if canonicalize fails
                        let boxed = crate::into_raw(Box::new(f.clone()));
                        Ok(Value::new_string_ptr(boxed as *mut ()))
                    }
                }
            } else {
                Ok(Value::new_bool(false))
            }
        } else {
            Err("stream_resolve_include_path() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_is_readable(filename: String) {
        if let Some(f) = filename {
            let readable = Path::new(f.as_str()).exists();
            Ok(Value::new_bool(readable))
        } else {
            Err("is_readable() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_is_writable(filename: String) {
        if let Some(f) = filename {
            let writable = Path::new(f.as_str()).exists();
            Ok(Value::new_bool(writable))
        } else {
            Err("is_writable() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_glob(pattern: String) |ctx| { 
        if let Some(p) = pattern {
            use hyperion_core::types::array::PhpArray;
            let mut arr = PhpArray::new();
            if let Ok(paths) = glob::glob(p.as_str()) {
                let mut i = 0i64;
                for path in paths.flatten() {
                    let s = path.to_string_lossy().into_owned();
                    let boxed = crate::into_raw(Box::new(s));
                    arr.insert_int(i, Value::new_string_ptr(boxed as *mut ()));
                    i += 1;
                }
            }
            let ptr = ctx.get_arena().alloc(arr);
            Ok(Value::new_array_ptr(ptr as *mut ()))
        } else {
            Err("glob() expects at least 1 parameter".to_string())
        }
    }
}

php_function! {
    native_clearstatcache(clear_realpath_cache: Value, filename: Value) {
        Ok(Value::null())
    }
}

php_function! {
    native_tempnam(dir: String, prefix: String) {
        if let (Some(d), Some(p)) = (dir, prefix) {
            let temp_dir = if d.is_empty() { std::env::temp_dir() } else { std::path::PathBuf::from(d.as_str()) };
            let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros();
            let mut file_name = p.to_string();
            file_name.push_str(&format!("{:x}", t));
            let path = temp_dir.join(file_name);
            match std::fs::File::create(&path) {
                Ok(_) => {
                    let s = path.to_string_lossy().into_owned();
                    let boxed = crate::into_raw(Box::new(s));
                    Ok(Value::new_string_ptr(boxed as *mut ()))
                }
                Err(_) => Ok(Value::new_bool(false)),
            }
        } else {
            Err("tempnam() expects exactly 2 parameters".to_string())
        }
    }
}

php_function! {
    native_umask(mask: Value) {
        // Just return a dummy mask for now (022)
        Ok(Value::new_int(0o022))
    }
}

php_function! {
    native_chmod(filename: String, permissions: Value) {
        if let (Some(f), Some(p)) = (filename, permissions) {
            let perm_val = if p.is_int() {
                p.as_int().unwrap_or(0)
            } else {
                0
            };
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let perms = std::fs::Permissions::from_mode(perm_val as u32);
                match std::fs::set_permissions(f.as_str(), perms) {
                    Ok(_) => Ok(Value::new_bool(true)),
                    Err(_) => Ok(Value::new_bool(false)),
                }
            }
            #[cfg(not(unix))]
            {
                // Dummy for non-unix
                Ok(Value::new_bool(true))
            }
        } else {
            Err("chmod() expects exactly 2 parameters".to_string())
        }
    }
}

php_function! {
    native_fileperms(filename: String) {
        if let Some(f) = filename {
            match fs::metadata(f.as_str()) {
                #[cfg(unix)]
                Ok(meta) => {
                    use std::os::unix::fs::PermissionsExt;
                    Ok(Value::new_int(meta.permissions().mode() as i32))
                }
                #[cfg(not(unix))]
                Ok(_) => Ok(Value::new_int(0o777)),
                Err(_) => Ok(Value::new_bool(false)),
            }
        } else {
            Err("fileperms() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_rename(from: String, to: String) {
        if let (Some(f), Some(t)) = (from, to) {
            match std::fs::rename(f.as_str(), t.as_str()) {
                Ok(_) => Ok(Value::new_bool(true)),
                Err(_) => Ok(Value::new_bool(false)),
            }
        } else {
            Err("rename() expects exactly 2 parameters".to_string())
        }
    }
}

php_function! {
    native_sys_get_temp_dir() {
        // Trailing separators are stripped: PHP returns "/tmp", not "/tmp/",
        // and callers concatenate their own separator.
        let dir = std::env::temp_dir();
        let s = dir.to_string_lossy().trim_end_matches('/').to_string();
        let s = if s.is_empty() { "/tmp".to_string() } else { s };
        Ok(Value::new_string_ptr(crate::into_raw(Box::new(s)) as *mut ()))
    }
}

php_function! {
    native_scandir(directory: String, sorting_order: Value) |ctx| {
        if let Some(d) = directory {
            let path = Path::new(d.as_str());
            if !path.is_dir() {
                return Ok(Value::new_bool(false));
            }
            let Ok(entries) = fs::read_dir(path) else {
                return Ok(Value::new_bool(false));
            };

            let mut names = vec![".".to_string(), "..".to_string()];
            for entry in entries.flatten() {
                names.push(entry.file_name().to_string_lossy().into_owned());
            }

            let order = sorting_order.and_then(|v| v.as_int()).unwrap_or(0);
            if order == 1 {
                names.sort_by(|a, b| b.cmp(a));
            } else if order == 0 {
                names.sort();
            }

            let mut arr = hyperion_core::types::array::PhpArray::new();
            for (i, name) in names.into_iter().enumerate() {
                let boxed = crate::into_raw(Box::new(name));
                arr.insert_int(i as i64, Value::new_string_ptr(boxed as *mut ()));
            }
            let ptr = ctx.get_arena().alloc(arr);
            Ok(Value::new_array_ptr(ptr as *mut ()))
        } else {
            Err("scandir() expects at least 1 parameter".to_string())
        }
    }
}

php_function! {
    native_filemtime(filename: String) {
        if let Some(f) = filename {
            if let Ok(meta) = fs::metadata(f.as_str()) {
                if let Ok(mtime) = meta.modified() {
                    let dur = mtime.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
                    return Ok(Value::new_int(dur.as_secs() as i32));
                }
            }
            Ok(Value::new_bool(false))
        } else {
            Err("filemtime() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_fileatime(filename: String) {
        if let Some(f) = filename {
            if let Ok(meta) = fs::metadata(f.as_str()) {
                if let Ok(atime) = meta.accessed() {
                    let dur = atime.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
                    return Ok(Value::new_int(dur.as_secs() as i32));
                }
            }
            Ok(Value::new_bool(false))
        } else {
            Err("fileatime() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_filectime(filename: String) {
        if let Some(f) = filename {
            if let Ok(meta) = fs::metadata(f.as_str()) {
                if let Ok(ctime) = meta.created() {
                    let dur = ctime.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
                    return Ok(Value::new_int(dur.as_secs() as i32));
                }
            }
            Ok(Value::new_bool(false))
        } else {
            Err("filectime() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_is_link(filename: String) {
        if let Some(f) = filename {
            let is_symlink = fs::symlink_metadata(f.as_str()).map(|m| m.file_type().is_symlink()).unwrap_or(false);
            Ok(Value::new_bool(is_symlink))
        } else {
            Err("is_link() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_is_executable(filename: String) {
        if let Some(f) = filename {
            let p = Path::new(f.as_str());
            Ok(Value::new_bool(p.exists()))
        } else {
            Err("is_executable() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_pathinfo(path: String, flags: Value) |ctx| {
        if let Some(p) = path {
            let path_obj = Path::new(p.as_str());
            let dirname = path_obj.parent().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let basename = path_obj.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let extension = path_obj.extension().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let filename = path_obj.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();

            if let Some(f) = flags.and_then(|v| v.as_int()) {
                let res = match f {
                    1 => dirname,
                    2 => basename,
                    4 => extension,
                    8 => filename,
                    _ => basename,
                };
                let boxed = crate::into_raw(Box::new(res));
                return Ok(Value::new_string_ptr(boxed as *mut ()));
            }

            let mut arr = hyperion_core::types::array::PhpArray::new();
            if !dirname.is_empty() {
                arr.insert_string_id(ctx.intern_string("dirname"), Value::new_string_ptr(crate::into_raw(Box::new(dirname)) as *mut ()));
            }
            arr.insert_string_id(ctx.intern_string("basename"), Value::new_string_ptr(crate::into_raw(Box::new(basename)) as *mut ()));
            if !extension.is_empty() {
                arr.insert_string_id(ctx.intern_string("extension"), Value::new_string_ptr(crate::into_raw(Box::new(extension)) as *mut ()));
            }
            arr.insert_string_id(ctx.intern_string("filename"), Value::new_string_ptr(crate::into_raw(Box::new(filename)) as *mut ()));

            let ptr = ctx.get_arena().alloc(arr);
            Ok(Value::new_array_ptr(ptr as *mut ()))
        } else {
            Err("pathinfo() expects at least 1 parameter".to_string())
        }
    }
}

php_function! {
    native_is_uploaded_file(filename: Value) {
        if let Some(v) = filename {
            let vd = v.deref();
            if let Some(ptr) = vd.as_string_ptr() {
                let s = unsafe { &*(ptr as *const String) };
                let p = Path::new(s.as_str());
                Ok(Value::new_bool(p.exists() && p.is_file()))
            } else {
                Ok(Value::new_bool(false))
            }
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_move_uploaded_file(from: Value, to: Value) {
        let (Some(from_v), Some(to_v)) = (from, to) else {
            return Ok(Value::new_bool(false));
        };
        let (Some(from_ptr), Some(to_ptr)) = (from_v.deref().as_string_ptr(), to_v.deref().as_string_ptr()) else {
            return Ok(Value::new_bool(false));
        };
        let from_str = unsafe { &*(from_ptr as *const String) };
        let to_str = unsafe { &*(to_ptr as *const String) };

        let from_path = Path::new(from_str);
        let to_path = Path::new(to_str);

        if !from_path.exists() {
            return Ok(Value::new_bool(false));
        }

        if let Some(parent) = to_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        // Try rename first, fallback to copy + remove
        if fs::rename(from_path, to_path).is_ok() {
            Ok(Value::new_bool(true))
        } else if fs::copy(from_path, to_path).is_ok() {
            let _ = fs::remove_file(from_path);
            Ok(Value::new_bool(true))
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_getcwd() {
        match std::env::current_dir() {
            Ok(p) => {
                let s = p.to_string_lossy().into_owned();
                let boxed = crate::into_raw(Box::new(s));
                Ok(Value::new_string_ptr(boxed as *mut ()))
            }
            Err(_) => Ok(Value::new_bool(false)),
        }
    }
}

php_function! {
    native_chdir(directory: Value) {
        if let Some(d) = directory {
            let dd = d.deref();
            if let Some(ptr) = dd.as_string_ptr() {
                let s = unsafe { &*(ptr as *const String) };
                match std::env::set_current_dir(s) {
                    Ok(_) => Ok(Value::new_bool(true)),
                    Err(_) => Ok(Value::new_bool(false)),
                }
            } else {
                Ok(Value::new_bool(false))
            }
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

struct DirHandle {
    entries: Vec<String>,
    position: usize,
}

static NEXT_DIR_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
lazy_static::lazy_static! {
    static ref DIR_HANDLES: dashmap::DashMap<usize, std::sync::Mutex<DirHandle>> = dashmap::DashMap::new();
}

php_function! {
    native_opendir(path: Value) {
        let Some(p_val) = path else {
            return Err("opendir() expects at least 1 parameter".to_string());
        };
        let p_deref = p_val.deref();
        let Some(ptr) = p_deref.as_string_ptr() else {
            return Ok(Value::new_bool(false));
        };
        let s = unsafe { &*(ptr as *const String) };
        let path = Path::new(s.as_str());
        if !path.is_dir() {
            return Ok(Value::new_bool(false));
        }
        let Ok(read_dir) = fs::read_dir(path) else {
            return Ok(Value::new_bool(false));
        };
        let mut entries = vec![".".to_string(), "..".to_string()];
        for entry in read_dir.flatten() {
            entries.push(entry.file_name().to_string_lossy().into_owned());
        }
        let id = NEXT_DIR_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        DIR_HANDLES.insert(id, std::sync::Mutex::new(DirHandle { entries, position: 0 }));
        Ok(Value::new_int(id as i32))
    }
}

php_function! {
    native_readdir(handle: Value) {
        let Some(h_val) = handle else {
            return Err("readdir() expects exactly 1 parameter".to_string());
        };
        let id = match h_val.deref().as_int() {
            Some(i) if i > 0 => i as usize,
            _ => return Ok(Value::new_bool(false)),
        };
        let Some(mutex) = DIR_HANDLES.get(&id) else {
            return Ok(Value::new_bool(false));
        };
        let mut guard = mutex.lock().unwrap();
        if guard.position < guard.entries.len() {
            let name = guard.entries[guard.position].clone();
            guard.position += 1;
            let boxed = crate::into_raw(Box::new(name));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_rewinddir(handle: Value) {
        let Some(h_val) = handle else {
            return Err("rewinddir() expects exactly 1 parameter".to_string());
        };
        let id = match h_val.deref().as_int() {
            Some(i) if i > 0 => i as usize,
            _ => return Ok(Value::new_bool(false)),
        };
        if let Some(mutex) = DIR_HANDLES.get(&id) {
            let mut guard = mutex.lock().unwrap();
            guard.position = 0;
        }
        Ok(Value::null())
    }
}

php_function! {
    native_closedir(handle: Value) {
        let Some(h_val) = handle else {
            return Err("closedir() expects exactly 1 parameter".to_string());
        };
        let id = match h_val.deref().as_int() {
            Some(i) if i > 0 => i as usize,
            _ => return Ok(Value::new_bool(false)),
        };
        DIR_HANDLES.remove(&id);
        Ok(Value::new_bool(true))
    }
}


