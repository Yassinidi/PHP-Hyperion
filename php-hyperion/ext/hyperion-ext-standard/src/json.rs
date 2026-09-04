use hyperion_core::php_function;
use hyperion_core::memory::nan_box::Value;
use hyperion_core::types::array::PhpArray;
use std::sync::Mutex;

// ============================================================
// 🌐 PHP JSON Module — 5 native functions
// ============================================================

lazy_static::lazy_static! {
    static ref LAST_JSON_ERROR: Mutex<i32> = Mutex::new(0);
    static ref LAST_JSON_ERROR_MSG: Mutex<String> = Mutex::new(String::new());
}

php_function! {
    native_json_encode(value: Value, flags: Value, depth: Value) |ctx| {
        let fl = flags.and_then(|v| v.deref().as_int()).unwrap_or(0) as i64;
        let max_depth = depth.and_then(|v| v.deref().as_int()).unwrap_or(512) as usize;

        if let Some(val) = value {
            match php_value_to_json(val, ctx, fl, max_depth) {
                Ok(json_str) => {
                    *LAST_JSON_ERROR.lock().unwrap() = 0;
                    *LAST_JSON_ERROR_MSG.lock().unwrap() = String::new();
                    let boxed = crate::into_raw(Box::new(json_str));
                    Ok(Value::new_string_ptr(boxed as *mut ()))
                }
                Err(e) => {
                    if *LAST_JSON_ERROR.lock().unwrap() == 0 {
                        *LAST_JSON_ERROR.lock().unwrap() = 5; // JSON_ERROR_UTF8
                    }
                    *LAST_JSON_ERROR_MSG.lock().unwrap() = e.clone();
                    if (fl & 4194304) != 0 { // JSON_THROW_ON_ERROR
                        return Err(e);
                    }
                    Ok(Value::new_bool(false))
                }
            }
        } else {
            let boxed = crate::into_raw(Box::new("null".to_string()));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        }
    }
}

php_function! {
    native_json_decode(json: Value, assoc: Value, depth: Value, flags: Value) |ctx| { 
        let raw_str = match json {
            Some(v) => {
                let deref = v.deref();
                if let Some(s_ptr) = deref.as_string_ptr() {
                    Some(unsafe { (*(s_ptr as *const String)).clone() })
                } else {
                    None
                }
            }
            None => None,
        };

        let fl = flags.and_then(|v| v.deref().as_int()).unwrap_or(0) as i64;
        let mut as_array = assoc.and_then(|v| v.deref().as_bool()).unwrap_or(false);
        if (fl & 1) != 0 { // JSON_OBJECT_AS_ARRAY
            as_array = true;
        }

        if let Some(s) = raw_str {
            if s.trim().is_empty() {
                return Ok(Value::null());
            }
            match serde_json::from_str::<serde_json::Value>(s.as_str()) {
                Ok(parsed) => {
                    *LAST_JSON_ERROR.lock().unwrap() = 0;
                    *LAST_JSON_ERROR_MSG.lock().unwrap() = String::new();
                    Ok(json_to_php_value(&parsed, ctx, as_array))
                }
                Err(e) => {
                    *LAST_JSON_ERROR.lock().unwrap() = 4; // JSON_ERROR_SYNTAX
                    *LAST_JSON_ERROR_MSG.lock().unwrap() = e.to_string();
                    if (fl & 4194304) != 0 { // JSON_THROW_ON_ERROR
                        return Err(format!("Syntax error: {}", e));
                    }
                    Ok(Value::null())
                }
            }
        } else {
            Ok(Value::null())
        }
    }
}

php_function! {
    native_json_last_error() {
        let err = *LAST_JSON_ERROR.lock().unwrap();
        Ok(Value::new_int(err))
    }
}

php_function! {
    native_json_last_error_msg() {
        let msg = LAST_JSON_ERROR_MSG.lock().unwrap().clone();
        if msg.is_empty() {
            let boxed = crate::into_raw(Box::new("No error".to_string()));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        } else {
            let boxed = crate::into_raw(Box::new(msg));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        }
    }
}

// ============================================================
// Helpers
// ============================================================

fn escape_json_string(s: &str, flags: i64, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => {
                if (flags & 8) != 0 { // JSON_HEX_QUOT
                    out.push_str("\\u0022");
                } else {
                    out.push_str("\\\"");
                }
            }
            '\\' => out.push_str("\\\\"),
            '<' => {
                if (flags & 1) != 0 { // JSON_HEX_TAG
                    out.push_str("\\u003C");
                } else {
                    out.push('<');
                }
            }
            '>' => {
                if (flags & 1) != 0 { // JSON_HEX_TAG
                    out.push_str("\\u003E");
                } else {
                    out.push('>');
                }
            }
            '&' => {
                if (flags & 2) != 0 { // JSON_HEX_AMP
                    out.push_str("\\u0026");
                } else {
                    out.push('&');
                }
            }
            '\'' => {
                if (flags & 4) != 0 { // JSON_HEX_APOS
                    out.push_str("\\u0027");
                } else {
                    out.push('\'');
                }
            }
            '/' => {
                if (flags & 64) != 0 { // JSON_UNESCAPED_SLASHES
                    out.push('/');
                } else {
                    out.push_str("\\/");
                }
            }
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\x08' => out.push_str("\\b"),
            '\x0C' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                use std::fmt::Write;
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c if (c as u32) > 0x7F => {
                if (flags & 256) != 0 { // JSON_UNESCAPED_UNICODE
                    out.push(c);
                } else {
                    use std::fmt::Write;
                    let code = c as u32;
                    if code <= 0xFFFF {
                        let _ = write!(out, "\\u{:04x}", code);
                    } else {
                        let u = code - 0x10000;
                        let high = 0xD800 + (u >> 10);
                        let low = 0xDC00 + (u & 0x3FF);
                        let _ = write!(out, "\\u{:04x}\\u{:04x}", high, low);
                    }
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

fn php_value_to_json(
    v: &Value,
    ctx: &mut dyn hyperion_core::types::function::NativeContext,
    flags: i64,
    max_depth: usize,
) -> Result<String, String> {
    let mut out = String::with_capacity(256);
    let mut visited = std::collections::HashSet::new();
    php_value_to_json_buf(v, ctx, 0, 0, flags, max_depth, &mut visited, &mut out)?;
    Ok(out)
}

fn php_value_to_json_buf(
    v: &Value,
    ctx: &mut dyn hyperion_core::types::function::NativeContext,
    depth: usize,
    indent_level: usize,
    flags: i64,
    max_depth: usize,
    visited: &mut std::collections::HashSet<usize>,
    out: &mut String,
) -> Result<(), String> {
    if depth >= max_depth {
        *LAST_JSON_ERROR.lock().unwrap() = 1; // JSON_ERROR_DEPTH
        return Err("Maximum stack depth exceeded".to_string());
    }
    let deref = v.deref();
    if deref.is_null() {
        out.push_str("null");
        return Ok(());
    }
    if let Some(b) = deref.as_bool() {
        out.push_str(if b { "true" } else { "false" });
        return Ok(());
    }
    if let Some(i) = deref.as_int() {
        use std::fmt::Write;
        let _ = write!(out, "{}", i);
        return Ok(());
    }
    if let Some(f) = deref.as_float() {
        if f.is_nan() || f.is_infinite() {
            *LAST_JSON_ERROR.lock().unwrap() = 2; // JSON_ERROR_INF_OR_NAN
            return Err("Inf and NaN cannot be JSON encoded".to_string());
        }
        use std::fmt::Write;
        if (flags & 1024) != 0 && f.fract() == 0.0 { // JSON_PRESERVE_ZERO_FRACTION
            let _ = write!(out, "{:.1}", f);
        } else {
            let _ = write!(out, "{}", f);
        }
        return Ok(());
    }
    if let Some(sp) = deref.as_string_ptr() {
        let s = unsafe { &*(sp as *const String) };
        if (flags & 32) != 0 { // JSON_NUMERIC_CHECK
            if !s.starts_with('0') || s.len() == 1 || s.starts_with("0.") {
                if let Ok(i) = s.parse::<i64>() {
                    use std::fmt::Write;
                    let _ = write!(out, "{}", i);
                    return Ok(());
                } else if let Ok(f) = s.parse::<f64>() {
                    if f.is_finite() {
                        use std::fmt::Write;
                        let _ = write!(out, "{}", f);
                        return Ok(());
                    }
                }
            }
        }
        escape_json_string(s, flags, out);
        return Ok(());
    }
    let pretty = (flags & 128) != 0; // JSON_PRETTY_PRINT
    if let Some(ap) = deref.as_array_ptr() {
        let ptr_val = ap as usize;
        if visited.contains(&ptr_val) {
            *LAST_JSON_ERROR.lock().unwrap() = 6; // JSON_ERROR_RECURSION
            out.push_str("\"*RECURSION*\"");
            return Ok(());
        }
        visited.insert(ptr_val);
        let arr = unsafe { &*(ap as *const PhpArray) };
        let force_object = (flags & 16) != 0; // JSON_FORCE_OBJECT

        if arr.is_packed && !force_object {
            if arr.packed.is_empty() {
                out.push_str("[]");
            } else {
                out.push('[');
                if pretty { out.push('\n'); }
                for (idx, val) in arr.packed.iter().enumerate() {
                    if idx > 0 {
                        out.push(',');
                        if pretty { out.push('\n'); }
                    }
                    if pretty {
                        for _ in 0..(indent_level + 1) { out.push_str("    "); }
                    }
                    php_value_to_json_buf(val, ctx, depth + 1, indent_level + 1, flags, max_depth, visited, out)?;
                }
                if pretty {
                    out.push('\n');
                    for _ in 0..indent_level { out.push_str("    "); }
                }
                out.push(']');
            }
        } else if arr.is_packed && force_object {
            if arr.packed.is_empty() {
                out.push_str("{}");
            } else {
                out.push('{');
                if pretty { out.push('\n'); }
                for (idx, val) in arr.packed.iter().enumerate() {
                    if idx > 0 {
                        out.push(',');
                        if pretty { out.push('\n'); }
                    }
                    if pretty {
                        for _ in 0..(indent_level + 1) { out.push_str("    "); }
                    }
                    use std::fmt::Write;
                    let _ = write!(out, "\"{}\":", idx);
                    if pretty { out.push(' '); }
                    php_value_to_json_buf(val, ctx, depth + 1, indent_level + 1, flags, max_depth, visited, out)?;
                }
                if pretty {
                    out.push('\n');
                    for _ in 0..indent_level { out.push_str("    "); }
                }
                out.push('}');
            }
        } else {
            let is_list = !force_object && arr.elements.keys().enumerate().all(|(i, k)| {
                matches!(k, hyperion_core::types::array::ArrayKey::Int(n) if *n == i as i64)
            });
            if arr.elements.is_empty() {
                if is_list { out.push_str("[]"); } else { out.push_str("{}"); }
            } else if is_list {
                out.push('[');
                if pretty { out.push('\n'); }
                for (idx, val) in arr.elements.values().enumerate() {
                    if idx > 0 {
                        out.push(',');
                        if pretty { out.push('\n'); }
                    }
                    if pretty {
                        for _ in 0..(indent_level + 1) { out.push_str("    "); }
                    }
                    php_value_to_json_buf(val, ctx, depth + 1, indent_level + 1, flags, max_depth, visited, out)?;
                }
                if pretty {
                    out.push('\n');
                    for _ in 0..indent_level { out.push_str("    "); }
                }
                out.push(']');
            } else {
                out.push('{');
                if pretty { out.push('\n'); }
                for (idx, (k, val)) in arr.elements.iter().enumerate() {
                    if idx > 0 {
                        out.push(',');
                        if pretty { out.push('\n'); }
                    }
                    if pretty {
                        for _ in 0..(indent_level + 1) { out.push_str("    "); }
                    }
                    match k {
                        hyperion_core::types::array::ArrayKey::Int(i) => {
                            use std::fmt::Write;
                            let _ = write!(out, "\"{}\":", i);
                        }
                        hyperion_core::types::array::ArrayKey::StringId(s) => {
                            let k_str = ctx.lookup_string(*s).unwrap_or_default();
                            escape_json_string(&k_str, flags, out);
                            out.push(':');
                        }
                    }
                    if pretty { out.push(' '); }
                    php_value_to_json_buf(val, ctx, depth + 1, indent_level + 1, flags, max_depth, visited, out)?;
                }
                if pretty {
                    out.push('\n');
                    for _ in 0..indent_level { out.push_str("    "); }
                }
                out.push('}');
            }
        }
        visited.remove(&ptr_val);
        return Ok(());
    }
    if let Some(op) = deref.as_object_ptr() {
        let ptr_val = op as usize;
        if visited.contains(&ptr_val) {
            *LAST_JSON_ERROR.lock().unwrap() = 6; // JSON_ERROR_RECURSION
            out.push_str("\"*RECURSION*\"");
            return Ok(());
        }
        visited.insert(ptr_val);

        let obj = unsafe { &*(op as *const hyperion_core::types::object::PhpObject) };
        let class_name = if let Some(ref name) = obj.class_name {
            name.clone()
        } else {
            ctx.get_class_name(obj.class_id).unwrap_or_default()
        };

        let has_json_serialize = !class_name.is_empty()
            && (ctx.implements_interface(&class_name, "JsonSerializable")
                || ctx.implements_interface(&class_name, "\\JsonSerializable")
                || ctx.has_method(&class_name, "jsonSerialize"));

        if has_json_serialize {
            let obj_val = deref;
            if let Ok(serialized_val) = ctx.call_method_synchronously(obj_val, "jsonSerialize", vec![]) {
                let res = php_value_to_json_buf(&serialized_val, ctx, depth + 1, indent_level, flags, max_depth, visited, out);
                visited.remove(&ptr_val);
                return res;
            }
        }

        if obj.properties.is_empty() {
            out.push_str("{}");
        } else {
            out.push('{');
            if pretty { out.push('\n'); }
            for (idx, (k, val)) in obj.properties.iter().enumerate() {
                if idx > 0 {
                    out.push(',');
                    if pretty { out.push('\n'); }
                }
                if pretty {
                    for _ in 0..(indent_level + 1) { out.push_str("    "); }
                }
                escape_json_string(k, flags, out);
                out.push(':');
                if pretty { out.push(' '); }
                php_value_to_json_buf(val, ctx, depth + 1, indent_level + 1, flags, max_depth, visited, out)?;
            }
            if pretty {
                out.push('\n');
                for _ in 0..indent_level { out.push_str("    "); }
            }
            out.push('}');
        }
        visited.remove(&ptr_val);
        return Ok(());
    }
    out.push_str("null");
    Ok(())
}

fn json_to_php_value(json: &serde_json::Value, ctx: &mut dyn hyperion_core::types::function::NativeContext, _as_array: bool) -> Value {
    match json {
        serde_json::Value::Null => Value::null(),
        serde_json::Value::Bool(b) => Value::new_bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::new_int(i as i32)
            } else if let Some(f) = n.as_f64() {
                Value::new_float(f)
            } else {
                Value::new_int(0)
            }
        }
        serde_json::Value::String(s) => {
            let boxed = crate::into_raw(Box::new(s.clone()));
            Value::new_string_ptr(boxed as *mut ())
        }
        serde_json::Value::Array(items) => {
            let mut php_arr = PhpArray::new();
            for (i, item) in items.iter().enumerate() {
                php_arr.insert_int(i as i64, json_to_php_value(item, ctx, _as_array));
            }
            let ptr = ctx.get_arena().alloc(php_arr);
            Value::new_array_ptr(ptr as *mut ())
        }
        serde_json::Value::Object(map) => {
            let mut php_arr = PhpArray::new();
            for (k, v) in map.iter() {
                php_arr.insert_string_id(ctx.intern_string(k), json_to_php_value(v, ctx, _as_array));
            }
            let ptr = ctx.get_arena().alloc(php_arr);
            Value::new_array_ptr(ptr as *mut ())
        }
    }
}
