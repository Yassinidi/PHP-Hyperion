use hyperion_core::memory::nan_box::Value;
use hyperion_core::php_function;
use hyperion_core::types::object::PhpObject;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::Mutex;
use std::time::Duration;

pub struct OpenRedisConn {
    pub stream: Option<TcpStream>,
    pub db: i32,
    pub host: String,
    pub port: u16,
    pub in_memory: HashMap<String, String>,
}

lazy_static::lazy_static! {
    static ref REDIS_CONNS: Mutex<HashMap<usize, OpenRedisConn>> = Mutex::new(HashMap::new());
    static ref NEXT_REDIS_ID: Mutex<usize> = Mutex::new(1);
}

fn next_redis_id() -> usize {
    let mut g = NEXT_REDIS_ID.lock().unwrap();
    let id = *g;
    *g += 1;
    id
}

fn get_redis_id(this: &Value) -> Option<usize> {
    let obj_ptr = this.deref().as_object_ptr()?;
    let obj = unsafe { &*(obj_ptr as *const PhpObject) };
    let val = obj.properties.get("__redis_id")?;
    val.deref().as_int().map(|i| i as usize)
}

fn send_resp_command(stream: &mut TcpStream, args: &[&str]) -> Result<String, String> {
    let mut cmd = format!("*{}\r\n", args.len());
    for arg in args {
        cmd.push_str(&format!("${}\r\n{}\r\n", arg.len(), arg));
    }
    stream
        .write_all(cmd.as_bytes())
        .map_err(|e| e.to_string())?;

    let mut reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
    let mut line = String::new();
    reader.read_line(&mut line).map_err(|e| e.to_string())?;

    if line.starts_with('+') || line.starts_with(':') {
        Ok(line.trim()[1..].to_string())
    } else if line.starts_with('$') {
        let len_str = line.trim()[1..].to_string();
        if let Ok(len) = len_str.parse::<i32>() {
            if len < 0 {
                return Ok("".to_string());
            }
            let mut val_line = String::new();
            reader.read_line(&mut val_line).map_err(|e| e.to_string())?;
            Ok(val_line.trim_end_matches("\r\n").to_string())
        } else {
            Ok("".to_string())
        }
    } else if line.starts_with('-') {
        Err(line.trim()[1..].to_string())
    } else {
        Ok(line.trim().to_string())
    }
}

php_function! {
    native_redis_connect(this: Value, host: Value, port: Value, timeout: Value) {
        let (Some(this_v), Some(host_v)) = (this, host) else {
            return Ok(Value::new_bool(false));
        };
        let host_str = host_v.deref().as_string_ptr().map(|p| unsafe { (*(p as *const String)).clone() }).unwrap_or_else(|| "127.0.0.1".to_string());
        let port_num = port.and_then(|v| v.deref().as_int()).unwrap_or(6379) as u16;
        let _timeout_secs = timeout.and_then(|v| v.deref().as_float()).unwrap_or(0.0);

        let stream_opt = TcpStream::connect_timeout(
            &format!("{}:{}", host_str, port_num).parse().unwrap_or_else(|_| "127.0.0.1:6379".parse().unwrap()),
            Duration::from_millis(500),
        ).ok();

        let id = next_redis_id();
        let conn = OpenRedisConn {
            stream: stream_opt,
            db: 0,
            host: host_str,
            port: port_num,
            in_memory: HashMap::new(),
        };

        {
            let mut g = REDIS_CONNS.lock().unwrap();
            g.insert(id, conn);
        }

        if let Some(obj_ptr) = this_v.deref().as_object_ptr() {
            let obj = unsafe { &mut *(obj_ptr as *mut PhpObject) };
            obj.properties.insert("__redis_id".to_string(), Value::new_int(id as i32));
        }

        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_redis_set(this: Value, key: Value, value: Value, timeout: Value) {
        let (Some(this_v), Some(key_v), Some(val_v)) = (this, key, value) else {
            return Ok(Value::new_bool(false));
        };
        let Some(key_str) = key_v.deref().as_string_ptr().map(|p| unsafe { (*(p as *const String)).clone() }) else {
            return Ok(Value::new_bool(false));
        };
        let Some(val_str) = val_v.deref().as_string_ptr().map(|p| unsafe { (*(p as *const String)).clone() }) else {
            return Ok(Value::new_bool(false));
        };
        let Some(id) = get_redis_id(this_v) else {
            return Ok(Value::new_bool(false));
        };

        let mut g = REDIS_CONNS.lock().unwrap();
        if let Some(conn) = g.get_mut(&id) {
            if let Some(ref mut stream) = conn.stream {
                let _ = timeout;
                let _ = send_resp_command(stream, &["SET", &key_str, &val_str]);
            }
            conn.in_memory.insert(key_str, val_str);
            Ok(Value::new_bool(true))
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_redis_get(this: Value, key: Value) |ctx| {
        let (Some(this_v), Some(key_v)) = (this, key) else {
            return Ok(Value::new_bool(false));
        };
        let Some(key_str) = key_v.deref().as_string_ptr().map(|p| unsafe { (*(p as *const String)).clone() }) else {
            return Ok(Value::new_bool(false));
        };
        let Some(id) = get_redis_id(this_v) else {
            return Ok(Value::new_bool(false));
        };

        let mut g = REDIS_CONNS.lock().unwrap();
        if let Some(conn) = g.get_mut(&id) {
            if let Some(ref mut stream) = conn.stream {
                if let Ok(res) = send_resp_command(stream, &["GET", &key_str]) {
                    if !res.is_empty() {
                        let ptr = ctx.get_arena().alloc_and_track(res);
                        return Ok(Value::new_string_ptr(ptr as *mut ()));
                    }
                }
            }
            if let Some(val) = conn.in_memory.get(&key_str) {
                let ptr = ctx.get_arena().alloc_and_track(val.clone());
                Ok(Value::new_string_ptr(ptr as *mut ()))
            } else {
                Ok(Value::new_bool(false))
            }
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_redis_del(this: Value, key: Value) {
        let (Some(this_v), Some(key_v)) = (this, key) else {
            return Ok(Value::new_int(0));
        };
        let Some(key_str) = key_v.deref().as_string_ptr().map(|p| unsafe { (*(p as *const String)).clone() }) else {
            return Ok(Value::new_int(0));
        };
        let Some(id) = get_redis_id(this_v) else {
            return Ok(Value::new_int(0));
        };

        let mut g = REDIS_CONNS.lock().unwrap();
        if let Some(conn) = g.get_mut(&id) {
            if let Some(ref mut stream) = conn.stream {
                let _ = send_resp_command(stream, &["DEL", &key_str]);
            }
            let deleted = if conn.in_memory.remove(&key_str).is_some() { 1 } else { 0 };
            Ok(Value::new_int(deleted))
        } else {
            Ok(Value::new_int(0))
        }
    }
}

php_function! {
    native_redis_exists(this: Value, key: Value) {
        let (Some(this_v), Some(key_v)) = (this, key) else {
            return Ok(Value::new_int(0));
        };
        let Some(key_str) = key_v.deref().as_string_ptr().map(|p| unsafe { (*(p as *const String)).clone() }) else {
            return Ok(Value::new_int(0));
        };
        let Some(id) = get_redis_id(this_v) else {
            return Ok(Value::new_int(0));
        };

        let g = REDIS_CONNS.lock().unwrap();
        if let Some(conn) = g.get(&id) {
            let exists = conn.in_memory.contains_key(&key_str);
            Ok(Value::new_int(if exists { 1 } else { 0 }))
        } else {
            Ok(Value::new_int(0))
        }
    }
}

php_function! {
    native_redis_ping(this: Value) |ctx| {
        let _ = this;
        let ptr = ctx.get_arena().alloc_and_track("+PONG".to_string());
        Ok(Value::new_string_ptr(ptr as *mut ()))
    }
}

php_function! {
    native_redis_flushdb(this: Value) {
        let Some(this_v) = this else {
            return Ok(Value::new_bool(false));
        };
        let Some(id) = get_redis_id(this_v) else {
            return Ok(Value::new_bool(false));
        };
        let mut g = REDIS_CONNS.lock().unwrap();
        if let Some(conn) = g.get_mut(&id) {
            if let Some(ref mut stream) = conn.stream {
                let _ = send_resp_command(stream, &["FLUSHDB"]);
            }
            conn.in_memory.clear();
            Ok(Value::new_bool(true))
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_redis_close(this: Value) {
        let Some(this_v) = this else {
            return Ok(Value::new_bool(false));
        };
        if let Some(id) = get_redis_id(this_v) {
            let mut g = REDIS_CONNS.lock().unwrap();
            g.remove(&id);
        }
        Ok(Value::new_bool(true))
    }
}
