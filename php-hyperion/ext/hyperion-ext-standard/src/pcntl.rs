use hyperion_core::memory::nan_box::Value;
use hyperion_core::php_function;
use hyperion_core::types::array::PhpArray;
use std::collections::HashMap;
use std::sync::Mutex;

lazy_static::lazy_static! {
    static ref REGISTERED_SIGNALS: Mutex<HashMap<i32, Value>> = Mutex::new(HashMap::new());
    static ref ASYNC_SIGNALS_ENABLED: Mutex<bool> = Mutex::new(false);
}

php_function! {
    native_pcntl_signal(signo: Value, handler: Value) {
        let Some(signo_v) = signo else {
            return Ok(Value::new_bool(false));
        };
        let sig = signo_v.deref().as_int().unwrap_or(0);
        let handler_val = handler.copied().unwrap_or_else(Value::null);

        let mut g = REGISTERED_SIGNALS.lock().unwrap();
        g.insert(sig, handler_val);
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_pcntl_signal_get_handler(signo: Value) {
        let Some(signo_v) = signo else {
            return Ok(Value::new_int(0)); // SIG_DFL
        };
        let sig = signo_v.deref().as_int().unwrap_or(0);
        let g = REGISTERED_SIGNALS.lock().unwrap();
        if let Some(h) = g.get(&sig) {
            Ok(*h)
        } else {
            Ok(Value::new_int(0)) // SIG_DFL = 0
        }
    }
}

php_function! {
    native_pcntl_signal_dispatch() {
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_pcntl_async_signals(enable: Value) {
        let mut g = ASYNC_SIGNALS_ENABLED.lock().unwrap();
        let prev = *g;
        if let Some(v) = enable {
            *g = v.deref().as_bool().unwrap_or(false);
        }
        Ok(Value::new_bool(prev))
    }
}

php_function! {
    native_pcntl_alarm(seconds: Value) {
        let _ = seconds;
        Ok(Value::new_int(0))
    }
}

php_function! {
    native_pcntl_fork() {
        unsafe {
            let pid = libc::fork();
            Ok(Value::new_int(pid as i32))
        }
    }
}

php_function! {
    native_pcntl_wait(status: Value, options: Value) {
        let _ = (status, options);
        Ok(Value::new_int(-1))
    }
}

php_function! {
    native_pcntl_waitpid(pid: Value, status: Value, options: Value) {
        let _ = (pid, status, options);
        Ok(Value::new_int(-1))
    }
}

php_function! {
    native_pcntl_wexitstatus(status: Value) {
        let s = status.and_then(|v| v.deref().as_int()).unwrap_or(0);
        Ok(Value::new_int((s >> 8) & 0xFF))
    }
}

php_function! {
    native_pcntl_wifexited(status: Value) {
        let s = status.and_then(|v| v.deref().as_int()).unwrap_or(0);
        Ok(Value::new_bool((s & 0x7F) == 0))
    }
}

php_function! {
    native_pcntl_wifsignaled(status: Value) {
        let s = status.and_then(|v| v.deref().as_int()).unwrap_or(0);
        Ok(Value::new_bool((s & 0x7F) != 0 && (s & 0x7F) != 0x7F))
    }
}

php_function! {
    native_pcntl_wtermsig(status: Value) {
        let s = status.and_then(|v| v.deref().as_int()).unwrap_or(0);
        Ok(Value::new_int(s & 0x7F))
    }
}

php_function! {
    native_pcntl_wifstopped(status: Value) {
        let s = status.and_then(|v| v.deref().as_int()).unwrap_or(0);
        Ok(Value::new_bool((s & 0xFF) == 0x7F))
    }
}

php_function! {
    native_pcntl_wstopsig(status: Value) {
        let s = status.and_then(|v| v.deref().as_int()).unwrap_or(0);
        Ok(Value::new_int((s >> 8) & 0xFF))
    }
}

php_function! {
    native_posix_getpid() {
        let pid = std::process::id();
        Ok(Value::new_int(pid as i32))
    }
}

php_function! {
    native_posix_getppid() {
        unsafe {
            let ppid = libc::getppid();
            Ok(Value::new_int(ppid as i32))
        }
    }
}

php_function! {
    native_posix_getuid() {
        unsafe {
            let uid = libc::getuid();
            Ok(Value::new_int(uid as i32))
        }
    }
}

php_function! {
    native_posix_geteuid() {
        unsafe {
            let euid = libc::geteuid();
            Ok(Value::new_int(euid as i32))
        }
    }
}

php_function! {
    native_posix_getgid() {
        unsafe {
            let gid = libc::getgid();
            Ok(Value::new_int(gid as i32))
        }
    }
}

php_function! {
    native_posix_getegid() {
        unsafe {
            let egid = libc::getegid();
            Ok(Value::new_int(egid as i32))
        }
    }
}

php_function! {
    native_posix_isatty(fd: Value) {
        let Some(f) = fd else {
            return Ok(Value::new_bool(false));
        };
        let fd_int = f.deref().as_int().unwrap_or(1);
        unsafe {
            let is_tty = libc::isatty(fd_int);
            Ok(Value::new_bool(is_tty == 1))
        }
    }
}

php_function! {
    native_posix_ttyname(fd: Value) {
        let _ = fd;
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_posix_kill(pid: Value, sig: Value) {
        let (Some(p), Some(s)) = (pid, sig) else {
            return Ok(Value::new_bool(false));
        };
        let pid_int = p.deref().as_int().unwrap_or(0);
        let sig_int = s.deref().as_int().unwrap_or(0);

        unsafe {
            let res = libc::kill(pid_int, sig_int);
            Ok(Value::new_bool(res == 0))
        }
    }
}

php_function! {
    native_posix_getpwuid(uid: Value) |ctx| {
        let uid_val = uid.and_then(|v| v.deref().as_int()).unwrap_or_else(|| unsafe { libc::getuid() as i32 });
        unsafe {
            let pwd = libc::getpwuid(uid_val as libc::uid_t);
            if pwd.is_null() {
                return Ok(Value::new_bool(false));
            }
            let name_str = std::ffi::CStr::from_ptr((*pwd).pw_name).to_str().unwrap_or("").to_string();
            let dir_str = std::ffi::CStr::from_ptr((*pwd).pw_dir).to_str().unwrap_or("").to_string();
            let shell_str = std::ffi::CStr::from_ptr((*pwd).pw_shell).to_str().unwrap_or("").to_string();

            let mut map = PhpArray::new();
            let name_ptr = crate::into_raw(Box::new(name_str));
            let dir_ptr = crate::into_raw(Box::new(dir_str));
            let shell_ptr = crate::into_raw(Box::new(shell_str));

            map.insert_string_id(ctx.intern_string("name"), Value::new_string_ptr(name_ptr as *mut ()));
            map.insert_string_id(ctx.intern_string("uid"), Value::new_int((*pwd).pw_uid as i32));
            map.insert_string_id(ctx.intern_string("gid"), Value::new_int((*pwd).pw_gid as i32));
            map.insert_string_id(ctx.intern_string("dir"), Value::new_string_ptr(dir_ptr as *mut ()));
            map.insert_string_id(ctx.intern_string("shell"), Value::new_string_ptr(shell_ptr as *mut ()));

            let arr_ptr = crate::into_raw(Box::new(map));
            Ok(Value::new_array_ptr(arr_ptr as *mut ()))
        }
    }
}

php_function! {
    native_posix_getpwnam(username: Value) |ctx| {
        let Some(u) = username else {
            return Ok(Value::new_bool(false));
        };
        let uname = if u.is_string() {
            unsafe { &*(u.as_string_ptr().unwrap() as *const String) }.clone()
        } else {
            String::new()
        };
        let c_uname = std::ffi::CString::new(uname).unwrap_or_default();
        unsafe {
            let pwd = libc::getpwnam(c_uname.as_ptr());
            if pwd.is_null() {
                return Ok(Value::new_bool(false));
            }
            let name_str = std::ffi::CStr::from_ptr((*pwd).pw_name).to_str().unwrap_or("").to_string();
            let dir_str = std::ffi::CStr::from_ptr((*pwd).pw_dir).to_str().unwrap_or("").to_string();
            let shell_str = std::ffi::CStr::from_ptr((*pwd).pw_shell).to_str().unwrap_or("").to_string();

            let mut map = PhpArray::new();
            let name_ptr = crate::into_raw(Box::new(name_str));
            let dir_ptr = crate::into_raw(Box::new(dir_str));
            let shell_ptr = crate::into_raw(Box::new(shell_str));

            map.insert_string_id(ctx.intern_string("name"), Value::new_string_ptr(name_ptr as *mut ()));
            map.insert_string_id(ctx.intern_string("uid"), Value::new_int((*pwd).pw_uid as i32));
            map.insert_string_id(ctx.intern_string("gid"), Value::new_int((*pwd).pw_gid as i32));
            map.insert_string_id(ctx.intern_string("dir"), Value::new_string_ptr(dir_ptr as *mut ()));
            map.insert_string_id(ctx.intern_string("shell"), Value::new_string_ptr(shell_ptr as *mut ()));

            let arr_ptr = crate::into_raw(Box::new(map));
            Ok(Value::new_array_ptr(arr_ptr as *mut ()))
        }
    }
}
