use hyperion_core::php_function;
use hyperion_core::memory::nan_box::Value;
use hyperion_core::types::array::PhpArray;

// ============================================================
// 🔤 PHP String Module — ~40 native functions
// ============================================================

// --- Basic Processing ---

pub fn native_strncmp(args: &[Value], _ctx: &mut dyn hyperion_core::types::function::NativeContext) -> Result<Value, String> {
    if args.len() < 3 {
        return Err("strncmp() expects at least 3 parameters".to_string());
    }
    let str1 = if args[0].is_string() {
        unsafe { &*(args[0].as_string_ptr().unwrap() as *const String) }.clone()
    } else {
        "".to_string()
    };
    let str2 = if args[1].is_string() {
        unsafe { &*(args[1].as_string_ptr().unwrap() as *const String) }.clone()
    } else {
        "".to_string()
    };
    let len = if let Some(l) = args[2].as_int() {
        l as usize
    } else {
        0
    };

    let b1 = &str1.as_bytes()[..len.min(str1.len())];
    let b2 = &str2.as_bytes()[..len.min(str2.len())];

    let cmp = b1.cmp(b2);
    let res = match cmp {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    };
    Ok(Value::new_int(res))

}

php_function! {
    native_strlen(s: String) {
        if let Some(s) = s {
            Ok(Value::new_int(s.len() as i32))
        } else {
            Err("strlen() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_urldecode(s: String) {
        if let Some(input) = s {
            let mut output = String::new();
            let mut chars = input.chars().peekable();
            while let Some(c) = chars.next() {
                if c == '%' {
                    let mut hex = String::new();
                    if let Some(h1) = chars.next() { hex.push(h1); }
                    if let Some(h2) = chars.next() { hex.push(h2); }
                    if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                        output.push(byte as char);
                    } else {
                        output.push('%');
                        output.push_str(&hex);
                    }
                } else if c == '+' {
                    output.push(' ');
                } else {
                    output.push(c);
                }
            }
            Ok(Value::new_string_ptr(crate::into_raw(Box::new(output)) as *mut ()))
        } else {
            Ok(Value::new_string_ptr(crate::into_raw(Box::new("".to_string())) as *mut ()))
        }
    }
}

php_function! {
    native_rawurldecode(s: String) {
        if let Some(input) = s {
            let mut output = String::new();
            let mut chars = input.chars().peekable();
            while let Some(c) = chars.next() {
                if c == '%' {
                    let mut hex = String::new();
                    if let Some(h1) = chars.next() { hex.push(h1); }
                    if let Some(h2) = chars.next() { hex.push(h2); }
                    if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                        output.push(byte as char);
                    } else {
                        output.push('%');
                        output.push_str(&hex);
                    }
                } else {
                    output.push(c);
                }
            }
            Ok(Value::new_string_ptr(crate::into_raw(Box::new(output)) as *mut ()))
        } else {
            Ok(Value::new_string_ptr(crate::into_raw(Box::new("".to_string())) as *mut ()))
        }
    }
}

php_function! {
    native_parse_url(url: String, component: Value) |ctx| {
        if let Some(url_ref) = url {
            let mut s = url_ref.clone();
            let comp = component.and_then(|v| v.as_int()).unwrap_or(-1);
            
            // Extract fragment
            let fragment = if let Some(idx) = s.find('#') {
                let frag = s[idx + 1..].to_string();
                s = s[..idx].to_string();
                Some(frag)
            } else {
                None
            };

            // Extract query
            let query = if let Some(idx) = s.find('?') {
                let q = s[idx + 1..].to_string();
                s = s[..idx].to_string();
                Some(q)
            } else {
                None
            };

            // Extract scheme
            let scheme = if let Some(idx) = s.find("://") {
                let sch = s[..idx].to_string();
                s = s[idx + 3..].to_string();
                Some(sch)
            } else {
                None
            };

            let mut user = None;
            let mut pass = None;
            let mut host = None;
            let mut port = None;
            let mut path = None;

            if scheme.is_some() || s.starts_with("//") {
                if s.starts_with("//") {
                    s = s[2..].to_string();
                }
                let (authority, p) = if let Some(slash_idx) = s.find('/') {
                    (&s[..slash_idx], &s[slash_idx..])
                } else {
                    (s.as_str(), "")
                };
                if !p.is_empty() {
                    path = Some(p.to_string());
                }

                let (userinfo, hostport) = if let Some(at_idx) = authority.find('@') {
                    (&authority[..at_idx], &authority[at_idx + 1..])
                } else {
                    ("", authority)
                };

                if !userinfo.is_empty() {
                    if let Some(colon_idx) = userinfo.find(':') {
                        user = Some(userinfo[..colon_idx].to_string());
                        pass = Some(userinfo[colon_idx + 1..].to_string());
                    } else {
                        user = Some(userinfo.to_string());
                    }
                }

                if let Some(colon_idx) = hostport.rfind(':') {
                    host = Some(hostport[..colon_idx].to_string());
                    if let Ok(p_num) = hostport[colon_idx + 1..].parse::<i32>() {
                        port = Some(p_num);
                    }
                } else if !hostport.is_empty() {
                    host = Some(hostport.to_string());
                }
            } else {
                if !s.is_empty() {
                    path = Some(s);
                }
            }

            match comp {
                0 => Ok(scheme.map(|v| Value::new_string_ptr(crate::into_raw(Box::new(v)) as *mut ())).unwrap_or(Value::null())),
                1 => Ok(host.map(|v| Value::new_string_ptr(crate::into_raw(Box::new(v)) as *mut ())).unwrap_or(Value::null())),
                2 => Ok(port.map(Value::new_int).unwrap_or(Value::null())),
                3 => Ok(user.map(|v| Value::new_string_ptr(crate::into_raw(Box::new(v)) as *mut ())).unwrap_or(Value::null())),
                4 => Ok(pass.map(|v| Value::new_string_ptr(crate::into_raw(Box::new(v)) as *mut ())).unwrap_or(Value::null())),
                5 => Ok(path.map(|v| Value::new_string_ptr(crate::into_raw(Box::new(v)) as *mut ())).unwrap_or(Value::null())),
                6 => Ok(query.map(|v| Value::new_string_ptr(crate::into_raw(Box::new(v)) as *mut ())).unwrap_or(Value::null())),
                7 => Ok(fragment.map(|v| Value::new_string_ptr(crate::into_raw(Box::new(v)) as *mut ())).unwrap_or(Value::null())),
                _ => {
                    let mut arr = PhpArray::new();
                    if let Some(v) = scheme {
                        arr.insert_string_id(hyperion_core::types::string_table::intern_string("scheme"), Value::new_string_ptr(crate::into_raw(Box::new(v)) as *mut ()));
                    }
                    if let Some(v) = host {
                        arr.insert_string_id(hyperion_core::types::string_table::intern_string("host"), Value::new_string_ptr(crate::into_raw(Box::new(v)) as *mut ()));
                    }
                    if let Some(v) = port {
                        arr.insert_string_id(hyperion_core::types::string_table::intern_string("port"), Value::new_int(v));
                    }
                    if let Some(v) = user {
                        arr.insert_string_id(hyperion_core::types::string_table::intern_string("user"), Value::new_string_ptr(crate::into_raw(Box::new(v)) as *mut ()));
                    }
                    if let Some(v) = pass {
                        arr.insert_string_id(hyperion_core::types::string_table::intern_string("pass"), Value::new_string_ptr(crate::into_raw(Box::new(v)) as *mut ()));
                    }
                    if let Some(v) = path {
                        arr.insert_string_id(hyperion_core::types::string_table::intern_string("path"), Value::new_string_ptr(crate::into_raw(Box::new(v)) as *mut ()));
                    }
                    if let Some(v) = query {
                        arr.insert_string_id(hyperion_core::types::string_table::intern_string("query"), Value::new_string_ptr(crate::into_raw(Box::new(v)) as *mut ()));
                    }
                    if let Some(v) = fragment {
                        arr.insert_string_id(hyperion_core::types::string_table::intern_string("fragment"), Value::new_string_ptr(crate::into_raw(Box::new(v)) as *mut ()));
                    }
                    Ok(Value::new_array_ptr(crate::into_raw(Box::new(arr)) as *mut ()))
                }
            }
        } else {
            Ok(Value::null())
        }
    }
}

php_function! {
    native_strtolower(s: String) {
        if let Some(s) = s {
            let result = crate::into_raw(Box::new(s.to_lowercase()));
            Ok(Value::new_string_ptr(result as *mut ()))
        } else {
            Err("strtolower() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_strtoupper(s: String) {
        if let Some(s) = s {
            let result = crate::into_raw(Box::new(s.to_uppercase()));
            Ok(Value::new_string_ptr(result as *mut ()))
        } else {
            Err("strtoupper() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_strrev(s: String) {
        if let Some(s) = s {
            let reversed: String = s.chars().rev().collect();
            let result = crate::into_raw(Box::new(reversed));
            Ok(Value::new_string_ptr(result as *mut ()))
        } else {
            Err("strrev() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_ucfirst(s: String) {
        if let Some(s) = s {
            let mut chars = s.chars();
            let result = match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().to_string() + chars.as_str(),
            };
            let boxed = crate::into_raw(Box::new(result));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        } else {
            Err("ucfirst() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_lcfirst(s: String) {
        if let Some(s) = s {
            let mut chars = s.chars();
            let result = match chars.next() {
                None => String::new(),
                Some(c) => c.to_lowercase().to_string() + chars.as_str(),
            };
            let boxed = crate::into_raw(Box::new(result));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        } else {
            Err("lcfirst() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_ucwords(s: String) {
        if let Some(s) = s {
            let result = s.split(' ')
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");
            let boxed = crate::into_raw(Box::new(result));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        } else {
            Err("ucwords() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_trim(s: Value, characters: Value) {
        let s_str = if let Some(val) = s {
            if let Some(sp) = val.deref().as_string_ptr() {
                unsafe { (*(sp as *const String)).clone() }
            } else if let Some(i) = val.deref().as_int() {
                i.to_string()
            } else if let Some(f) = val.deref().as_float() {
                f.to_string()
            } else if let Some(b) = val.deref().as_bool() {
                if b { "1".to_string() } else { "".to_string() }
            } else {
                "".to_string()
            }
        } else {
            return Err("trim() expects at least 1 parameter".to_string());
        };

        let chars_opt = characters.and_then(|c| {
            if let Some(sp) = c.deref().as_string_ptr() {
                Some(unsafe { (*(sp as *const String)).clone() })
            } else {
                None
            }
        });

        let res = if let Some(chars) = chars_opt {
            s_str.trim_matches(|c| chars.contains(c)).to_string()
        } else {
            s_str.trim_matches(&[' ', '\n', '\r', '\t', '\x0B', '\0'][..]).to_string()
        };
        let result = crate::into_raw(Box::new(res));
        Ok(Value::new_string_ptr(result as *mut ()))
    }
}

php_function! {
    native_ltrim(s: Value, characters: Value) {
        let s_str = if let Some(val) = s {
            if let Some(sp) = val.deref().as_string_ptr() {
                unsafe { (*(sp as *const String)).clone() }
            } else if let Some(i) = val.deref().as_int() {
                i.to_string()
            } else if let Some(f) = val.deref().as_float() {
                f.to_string()
            } else if let Some(b) = val.deref().as_bool() {
                if b { "1".to_string() } else { "".to_string() }
            } else {
                "".to_string()
            }
        } else {
            return Err("ltrim() expects at least 1 parameter".to_string());
        };

        let chars_opt = characters.and_then(|c| {
            if let Some(sp) = c.deref().as_string_ptr() {
                Some(unsafe { (*(sp as *const String)).clone() })
            } else {
                None
            }
        });

        let res = if let Some(chars) = chars_opt {
            s_str.trim_start_matches(|c| chars.contains(c)).to_string()
        } else {
            s_str.trim_start_matches(&[' ', '\n', '\r', '\t', '\x0B', '\0'][..]).to_string()
        };
        let result = crate::into_raw(Box::new(res));
        Ok(Value::new_string_ptr(result as *mut ()))
    }
}

php_function! {
    native_rtrim(s: Value, characters: Value) {
        let s_str = if let Some(val) = s {
            if let Some(sp) = val.deref().as_string_ptr() {
                unsafe { (*(sp as *const String)).clone() }
            } else if let Some(i) = val.deref().as_int() {
                i.to_string()
            } else if let Some(f) = val.deref().as_float() {
                f.to_string()
            } else if let Some(b) = val.deref().as_bool() {
                if b { "1".to_string() } else { "".to_string() }
            } else {
                "".to_string()
            }
        } else {
            return Err("rtrim() expects at least 1 parameter".to_string());
        };

        let chars_opt = characters.and_then(|c| {
            if let Some(sp) = c.deref().as_string_ptr() {
                Some(unsafe { (*(sp as *const String)).clone() })
            } else {
                None
            }
        });

        let res = if let Some(chars) = chars_opt {
            s_str.trim_end_matches(|c| chars.contains(c)).to_string()
        } else {
            s_str.trim_end_matches(&[' ', '\n', '\r', '\t', '\x0B', '\0'][..]).to_string()
        };
        let result = crate::into_raw(Box::new(res));
        Ok(Value::new_string_ptr(result as *mut ()))
    }
}

// --- Search Functions ---

php_function! {
    native_strpos(haystack: Value, needle: Value, offset: Value) {
        if haystack.is_some() && needle.is_some() {
            let h = str_val_to_string(haystack);
            let n = str_val_to_string(needle);
            let offset_val = offset.and_then(|v| v.as_int()).unwrap_or(0) as i64;
            let h_len = h.len() as i64;
            let start = if offset_val < 0 {
                let s = h_len + offset_val;
                if s < 0 { return Ok(Value::new_bool(false)); }
                s as usize
            } else {
                if offset_val as usize > h.len() {
                    return Ok(Value::new_bool(false));
                }
                offset_val as usize
            };

            if n.is_empty() {
                return Ok(Value::new_int(start as i32));
            }

            match h[start..].find(n.as_str()) {
                Some(pos) => Ok(Value::new_int((start + pos) as i32)),
                None => Ok(Value::new_bool(false)),
            }
        } else {
            Err("strpos() expects at least 2 parameters".to_string())
        }
    }
}

php_function! {
    native_stripos(haystack: Value, needle: Value, offset: Value) {
        if haystack.is_some() && needle.is_some() {
            let h = str_val_to_string(haystack);
            let n = str_val_to_string(needle);
            let offset_val = offset.and_then(|v| v.as_int()).unwrap_or(0) as i64;
            let h_len = h.len() as i64;
            let start = if offset_val < 0 {
                let s = h_len + offset_val;
                if s < 0 { return Ok(Value::new_bool(false)); }
                s as usize
            } else {
                if offset_val as usize > h.len() {
                    return Ok(Value::new_bool(false));
                }
                offset_val as usize
            };

            if n.is_empty() {
                return Ok(Value::new_int(start as i32));
            }

            let h_sub = h[start..].to_lowercase();
            let n_lower = n.to_lowercase();
            match h_sub.find(&n_lower) {
                Some(pos) => Ok(Value::new_int((start + pos) as i32)),
                None => Ok(Value::new_bool(false)),
            }
        } else {
            Err("stripos() expects at least 2 parameters".to_string())
        }
    }
}

php_function! {
    native_strrpos(haystack: Value, needle: Value, offset: Value) {
        if haystack.is_some() && needle.is_some() {
            let h = str_val_to_string(haystack);
            let n = str_val_to_string(needle);
            let offset_val = offset.and_then(|v| v.as_int()).unwrap_or(0) as i64;
            let h_len = h.len() as i64;
            if offset_val >= 0 {
                let start = offset_val as usize;
                if start > h.len() {
                    return Ok(Value::new_bool(false));
                }
                if n.is_empty() {
                    return Ok(Value::new_int(h.len() as i32));
                }
                match h[start..].rfind(n.as_str()) {
                    Some(pos) => Ok(Value::new_int((start + pos) as i32)),
                    None => Ok(Value::new_bool(false)),
                }
            } else {
                let end = h_len + offset_val;
                if end < 0 {
                    return Ok(Value::new_bool(false));
                }
                let end = end as usize;
                let slice = if end >= h.len() { &h[..] } else { &h[..end] };
                if n.is_empty() {
                    return Ok(Value::new_int(end as i32));
                }
                match slice.rfind(n.as_str()) {
                    Some(pos) => Ok(Value::new_int(pos as i32)),
                    None => Ok(Value::new_bool(false)),
                }
            }
        } else {
            Err("strrpos() expects at least 2 parameters".to_string())
        }
    }
}

php_function! {
    native_strripos(haystack: Value, needle: Value, offset: Value) {
        if haystack.is_some() && needle.is_some() {
            let h = str_val_to_string(haystack);
            let n = str_val_to_string(needle);
            let offset_val = offset.and_then(|v| v.as_int()).unwrap_or(0) as i64;
            let h_len = h.len() as i64;
            let n_lower = n.to_lowercase();
            if offset_val >= 0 {
                let start = offset_val as usize;
                if start > h.len() {
                    return Ok(Value::new_bool(false));
                }
                if n.is_empty() {
                    return Ok(Value::new_int(h.len() as i32));
                }
                let h_sub = h[start..].to_lowercase();
                match h_sub.rfind(&n_lower) {
                    Some(pos) => Ok(Value::new_int((start + pos) as i32)),
                    None => Ok(Value::new_bool(false)),
                }
            } else {
                let end = h_len + offset_val;
                if end < 0 {
                    return Ok(Value::new_bool(false));
                }
                let end = end as usize;
                let slice = if end >= h.len() { &h[..] } else { &h[..end] };
                if n.is_empty() {
                    return Ok(Value::new_int(end as i32));
                }
                let slice_lower = slice.to_lowercase();
                match slice_lower.rfind(&n_lower) {
                    Some(pos) => Ok(Value::new_int(pos as i32)),
                    None => Ok(Value::new_bool(false)),
                }
            }
        } else {
            Err("strripos() expects at least 2 parameters".to_string())
        }
    }
}

php_function! {
    native_strstr(haystack: Value, needle: Value, before_needle: Value) {
        if haystack.is_some() && needle.is_some() {
            let h = str_val_to_string(haystack);
            let n = str_val_to_string(needle);
            let before = before_needle.and_then(|v| v.as_bool()).unwrap_or(false);
            match h.find(n.as_str()) {
                Some(pos) => {
                    let result_str = if before {
                        h[..pos].to_string()
                    } else {
                        h[pos..].to_string()
                    };
                    let result = crate::into_raw(Box::new(result_str));
                    Ok(Value::new_string_ptr(result as *mut ()))
                }
                None => Ok(Value::new_bool(false)),
            }
        } else {
            Err("strstr() expects at least 2 parameters".to_string())
        }
    }
}

php_function! {
    native_stristr(haystack: Value, needle: Value, before_needle: Value) {
        if haystack.is_some() && needle.is_some() {
            let h = str_val_to_string(haystack);
            let n = str_val_to_string(needle);
            let before = before_needle.and_then(|v| v.as_bool()).unwrap_or(false);
            let h_lower = h.to_lowercase();
            let n_lower = n.to_lowercase();
            match h_lower.find(&n_lower) {
                Some(pos) => {
                    let result_str = if before {
                        h[..pos].to_string()
                    } else {
                        h[pos..].to_string()
                    };
                    let result = crate::into_raw(Box::new(result_str));
                    Ok(Value::new_string_ptr(result as *mut ()))
                }
                None => Ok(Value::new_bool(false)),
            }
        } else {
            Err("stristr() expects at least 2 parameters".to_string())
        }
    }
}

php_function! {
    native_strpbrk(haystack: Value, char_list: Value) {
        if haystack.is_some() && char_list.is_some() {
            let h = str_val_to_string(haystack);
            let chars = str_val_to_string(char_list);
            for (i, c) in h.char_indices() {
                if chars.contains(c) {
                    let result = crate::into_raw(Box::new(h[i..].to_string()));
                    return Ok(Value::new_string_ptr(result as *mut ()));
                }
            }
            Ok(Value::new_bool(false))
        } else {
            Err("strpbrk() expects exactly 2 parameters".to_string())
        }
    }
}

// --- Replace Functions ---

fn str_val_to_string(val: Option<&Value>) -> String {
    match val {
        Some(v) => {
            let v = v.deref();
            if let Some(s_ptr) = v.as_string_ptr() {
                unsafe { (*(s_ptr as *const String)).clone() }
            } else if let Some(i) = v.as_int() {
                i.to_string()
            } else if let Some(f) = v.as_float() {
                f.to_string()
            } else if let Some(b) = v.as_bool() {
                if b { "1".to_string() } else { "".to_string() }
            } else if v.is_object() {
                if let Some(obj_ptr) = v.as_object_ptr() {
                    let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
                    if let Some(prop_val) = obj.properties.get("value") {
                        if let Some(s_ptr) = prop_val.deref().as_string_ptr() {
                            return unsafe { (*(s_ptr as *const String)).clone() };
                        }
                    }
                }
                "".to_string()
            } else {
                "".to_string()
            }
        }
        None => "".to_string(),
    }
}

fn str_val_to_string_with_ctx(
    val: Option<&Value>,
    ctx: &mut (dyn hyperion_core::types::function::NativeContext + '_),
) -> String {
    match val {
        Some(v) => {
            let v = v.deref();
            if let Some(s_ptr) = v.as_string_ptr() {
                unsafe { (*(s_ptr as *const String)).clone() }
            } else if let Some(i) = v.as_int() {
                i.to_string()
            } else if let Some(f) = v.as_float() {
                f.to_string()
            } else if let Some(b) = v.as_bool() {
                if b { "1".to_string() } else { "".to_string() }
            } else if v.is_object() {
                if let Ok(res) = ctx.call_method_synchronously(v, "__toString", vec![]) {
                    let res = res.deref();
                    if let Some(s_ptr) = res.as_string_ptr() {
                        return unsafe { (*(s_ptr as *const String)).clone() };
                    } else if let Some(i) = res.as_int() {
                        return i.to_string();
                    } else if let Some(f) = res.as_float() {
                        return f.to_string();
                    }
                }
                if let Some(obj_ptr) = v.as_object_ptr() {
                    let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
                    if let Some(prop_val) = obj.properties.get("value") {
                        if let Some(s_ptr) = prop_val.deref().as_string_ptr() {
                            return unsafe { (*(s_ptr as *const String)).clone() };
                        }
                    }
                }
                "".to_string()
            } else {
                "".to_string()
            }
        }
        None => "".to_string(),
    }
}

fn str_val_to_string_list_with_ctx(
    val: Option<&Value>,
    ctx: &mut (dyn hyperion_core::types::function::NativeContext + '_),
) -> (Vec<String>, bool) {
    match val {
        Some(v) if v.is_array() => {
            let arr_ptr = v.as_array_ptr().unwrap() as *const PhpArray;
            let arr = unsafe { &*arr_ptr };
            let list = arr.elements.values().map(|el| str_val_to_string_with_ctx(Some(el), ctx)).collect();
            (list, true)
        }
        Some(v) => (vec![str_val_to_string_with_ctx(Some(v), ctx)], false),
        None => (vec![], false),
    }
}

fn str_val_to_string_list(val: Option<&Value>) -> (Vec<String>, bool) {
    match val {
        Some(v) if v.is_array() => {
            let arr_ptr = v.as_array_ptr().unwrap() as *const PhpArray;
            let arr = unsafe { &*arr_ptr };
            let list = arr.elements.values().map(|el| str_val_to_string(Some(el))).collect();
            (list, true)
        }
        Some(v) => (vec![str_val_to_string(Some(v))], false),
        None => (vec![], false),
    }
}

fn perform_str_replace(
    search_list: &[String],
    replace_list: &[String],
    replace_is_array: bool,
    mut subj: String,
    case_insensitive: bool,
) -> String {
    for (i, s) in search_list.iter().enumerate() {
        if s.is_empty() {
            continue;
        }
        let r = if replace_is_array {
            replace_list.get(i).map(|s| s.as_str()).unwrap_or("")
        } else {
            replace_list.first().map(|s| s.as_str()).unwrap_or("")
        };
        if case_insensitive {
            let lower_s = s.to_lowercase();
            let mut result = String::with_capacity(subj.len());
            let mut last = 0;
            let lower_subj = subj.to_lowercase();
            for (start, _) in lower_subj.match_indices(&lower_s) {
                result.push_str(&subj[last..start]);
                result.push_str(r);
                last = start + s.len();
            }
            result.push_str(&subj[last..]);
            subj = result;
        } else {
            subj = subj.replace(s.as_str(), r);
        }
    }
    subj
}

fn execute_str_replace(
    search: Option<&Value>,
    replace: Option<&Value>,
    subject: Option<&Value>,
    case_insensitive: bool,
    ctx: &mut (dyn hyperion_core::types::function::NativeContext + '_),
) -> Result<Value, String> {
    let (search_list, _) = str_val_to_string_list_with_ctx(search, ctx);
    let (replace_list, replace_is_array) = str_val_to_string_list_with_ctx(replace, ctx);

    let subj_val = match subject {
        Some(v) => *v,
        None => return Ok(Value::new_string_ptr(crate::into_raw(Box::new(String::new())) as *mut ())),
    };

    if subj_val.is_array() {
        let arr_ptr = subj_val.as_array_ptr().unwrap() as *const PhpArray;
        let arr = unsafe { &*arr_ptr };
        let mut new_arr = PhpArray::new();
        for (key, val) in &arr.elements {
            let s = str_val_to_string_with_ctx(Some(val), ctx);
            let replaced = perform_str_replace(&search_list, &replace_list, replace_is_array, s, case_insensitive);
            let boxed = crate::into_raw(Box::new(replaced));
            new_arr.elements.insert(key.clone(), Value::new_string_ptr(boxed as *mut ()));
        }
        let ptr = ctx.get_arena().alloc_and_track(new_arr);
        Ok(Value::new_array_ptr(ptr as *mut ()))
    } else {
        let s = str_val_to_string_with_ctx(Some(&subj_val), ctx);
        let replaced = perform_str_replace(&search_list, &replace_list, replace_is_array, s, case_insensitive);
        let boxed = crate::into_raw(Box::new(replaced));
        Ok(Value::new_string_ptr(boxed as *mut ()))
    }
}

php_function! {
    native_str_replace(search: Value, replace: Value, subject: Value) |ctx| {
        execute_str_replace(search, replace, subject, false, ctx)
    }
}

php_function! {
    native_str_ireplace(search: Value, replace: Value, subject: Value) |ctx| {
        execute_str_replace(search, replace, subject, true, ctx)
    }
}

php_function! {
    native_substr_replace(string_val: Value, replacement_val: Value, start_val: Value, length_val: Value) |ctx| {
        let s = if let Some(v) = string_val {
            if let Some(p) = v.as_string_ptr() {
                unsafe { &*(p as *const String) }.clone()
            } else {
                return Err("substr_replace() expects string for parameter 1".to_string());
            }
        } else {
            return Err("substr_replace() expects at least 3 parameters".to_string());
        };

        let r = if let Some(v) = replacement_val {
            if let Some(p) = v.as_string_ptr() {
                unsafe { &*(p as *const String) }.clone()
            } else {
                return Err("substr_replace() expects string for parameter 2".to_string());
            }
        } else {
            return Err("substr_replace() expects at least 3 parameters".to_string());
        };

        let start_int = if let Some(v) = start_val {
            v.as_int().unwrap_or(0) as i64
        } else {
            0
        };

        let s_bytes = s.as_bytes();
        let r_bytes = r.as_bytes();
        let s_len = s_bytes.len() as i64;
        let start_idx = if start_int < 0 {
            (s_len + start_int).max(0) as usize
        } else {
            (start_int as usize).min(s_bytes.len())
        };

        let end_idx = if let Some(lv) = length_val {
            if let Some(len_int) = lv.as_int() {
                let len_i64 = len_int as i64;
                if len_i64 < 0 {
                    ((s_len + len_i64).max(start_idx as i64) as usize).min(s_bytes.len())
                } else {
                    (start_idx + len_i64 as usize).min(s_bytes.len())
                }
            } else {
                s_bytes.len()
            }
        } else {
            s_bytes.len()
        };

        let mut result_bytes = Vec::with_capacity(start_idx + r_bytes.len() + (s_bytes.len().saturating_sub(end_idx)));
        result_bytes.extend_from_slice(&s_bytes[..start_idx]);
        result_bytes.extend_from_slice(r_bytes);
        if end_idx < s_bytes.len() {
            result_bytes.extend_from_slice(&s_bytes[end_idx..]);
        }

        let result = unsafe { String::from_utf8_unchecked(result_bytes) };
        let str_ptr = ctx.get_arena().alloc_and_track(result);
        Ok(Value::new_string_ptr(str_ptr as *mut ()))
    }
}

php_function! {
    native_strtr(string: Value, from: Value, to: Value) |ctx| {
        if let (Some(s_val), Some(f_val)) = (string, from) {
            let s = if let Some(p) = s_val.as_string_ptr() { unsafe { &*(p as *const String) }.clone() } else { String::new() };
            
            if let Some(to_val) = to {
                let f = if let Some(p) = f_val.as_string_ptr() { unsafe { &*(p as *const String) }.clone() } else { String::new() };
                let t = if let Some(p) = to_val.as_string_ptr() { unsafe { &*(p as *const String) }.clone() } else { String::new() };
                
                let mut result = String::with_capacity(s.len());
                for c in s.chars() {
                    if let Some(idx) = f.find(c) {
                        if let Some(to_c) = t.chars().nth(idx) {
                            result.push(to_c);
                        } else {
                            result.push(c);
                        }
                    } else {
                        result.push(c);
                    }
                }
                let boxed = crate::into_raw(Box::new(result));
                Ok(Value::new_string_ptr(boxed as *mut ()))
            } else if let Some(arr_ptr) = f_val.as_array_ptr() {
                // from is an array of replace pairs
                let arr = unsafe { &*(arr_ptr as *const hyperion_core::types::array::PhpArray) };
                let mut result = s.clone();
                for (k, v) in arr.elements.iter() {
                    let k_str = match k {
                        hyperion_core::types::array::ArrayKey::StringId(p) => ctx.lookup_string(*p).unwrap_or_default(),
                        hyperion_core::types::array::ArrayKey::Int(i) => i.to_string(),
                    };
                    let v_str = if let Some(p) = v.as_string_ptr() { unsafe { &*(p as *const String) }.clone() } else { String::new() };
                    result = result.replace(&k_str, &v_str);
                }
                let boxed = crate::into_raw(Box::new(result));
                Ok(Value::new_string_ptr(boxed as *mut ()))
            } else {
                let boxed = crate::into_raw(Box::new(s));
                Ok(Value::new_string_ptr(boxed as *mut ()))
            }
        } else {
            Err("strtr() expects at least 2 parameters".to_string())
        }
    }
}

php_function! {
    native_levenshtein(str1: String, str2: String, ins_cost: Value, rep_cost: Value, del_cost: Value) {
        let s1 = str1.cloned().unwrap_or_default();
        let s2 = str2.cloned().unwrap_or_default();
        let ins = ins_cost.and_then(|v| v.as_int()).unwrap_or(1) as usize;
        let rep = rep_cost.and_then(|v| v.as_int()).unwrap_or(1) as usize;
        let del = del_cost.and_then(|v| v.as_int()).unwrap_or(1) as usize;

        let v1: Vec<char> = s1.chars().collect();
        let v2: Vec<char> = s2.chars().collect();
        let l1 = v1.len();
        let l2 = v2.len();

        if l1 == 0 {
            return Ok(Value::new_int((l2 * ins) as i32));
        }
        if l2 == 0 {
            return Ok(Value::new_int((l1 * del) as i32));
        }

        let mut dp = vec![vec![0usize; l2 + 1]; l1 + 1];
        for i in 0..=l1 {
            dp[i][0] = i * del;
        }
        for j in 0..=l2 {
            dp[0][j] = j * ins;
        }

        for i in 1..=l1 {
            for j in 1..=l2 {
                if v1[i - 1] == v2[j - 1] {
                    dp[i][j] = dp[i - 1][j - 1];
                } else {
                    let cost_del = dp[i - 1][j] + del;
                    let cost_ins = dp[i][j - 1] + ins;
                    let cost_rep = dp[i - 1][j - 1] + rep;
                    dp[i][j] = cost_del.min(cost_ins).min(cost_rep);
                }
            }
        }

        Ok(Value::new_int(dp[l1][l2] as i32))
    }
}

// --- Split/Join Functions ---

php_function! {
    native_explode(separator: String, string: String, limit: Value) |ctx| { 
        if let (Some(sep), Some(s)) = (separator, string) {
            if sep.is_empty() {
                return Err("explode(): Empty delimiter".to_string());
            }
            let lim_opt = limit.and_then(|v| v.deref().as_int());
            let pieces: Vec<&str> = match lim_opt {
                Some(lim) if lim > 1 => s.splitn(lim as usize, sep.as_str()).collect(),
                Some(lim) if lim == 0 || lim == 1 => vec![s.as_str()],
                Some(lim) if lim < 0 => {
                    let all: Vec<&str> = s.split(sep.as_str()).collect();
                    let keep = (all.len() as i64 + lim as i64).max(0) as usize;
                    all[..keep].to_vec()
                }
                _ => s.split(sep.as_str()).collect(),
            };
            let mut php_arr = PhpArray::new();
            for (i, piece) in pieces.iter().enumerate() {
                let boxed_str = crate::into_raw(Box::new(piece.to_string()));
                let val = Value::new_string_ptr(boxed_str as *mut ());
                php_arr.insert_int(i as i64, val);
            }
            let arr_ptr = ctx.get_arena().alloc(php_arr);
            Ok(Value::new_array_ptr(arr_ptr as *mut ()))
        } else {
            Err("explode() expects at least 2 parameters".to_string())
        }
    }
}

pub fn native_implode(
    args: &[Value],
    ctx: &mut dyn hyperion_core::types::function::NativeContext,
) -> Result<Value, String> {
    if args.is_empty() {
        return Err("implode() expects at least 1 parameter, 0 given".to_string());
    }

    let (glue_str, pieces_val) = if args.len() == 1 {
        (String::new(), args[0].deref())
    } else {
        let arg0 = args[0].deref();
        let arg1 = args[1].deref();
        if arg0.is_array() && !arg1.is_array() {
            let glue = if let Some(sp) = arg1.as_string_ptr() {
                unsafe { &*(sp as *const String) }.clone()
            } else if let Some(i) = arg1.as_int() {
                i.to_string()
            } else {
                String::new()
            };
            (glue, arg0)
        } else {
            let glue = if let Some(sp) = arg0.as_string_ptr() {
                unsafe { &*(sp as *const String) }.clone()
            } else if let Some(i) = arg0.as_int() {
                i.to_string()
            } else {
                String::new()
            };
            (glue, arg1)
        }
    };

    let pieces_val = pieces_val.deref();
    if let Some(arr_ptr) = pieces_val.as_array_ptr() {
        let arr = unsafe { &*(arr_ptr as *const PhpArray) };
        let elements: Vec<Value> = arr.elements.values().cloned().collect();
        let mut parts: Vec<String> = Vec::with_capacity(elements.len());
        for v in elements {
            let v_deref = v.deref();
            if let Some(sp) = v_deref.as_string_ptr() {
                parts.push(unsafe { &*(sp as *const String) }.clone());
            } else if let Some(i) = v_deref.as_int() {
                parts.push(i.to_string());
            } else if let Some(f) = v_deref.as_float() {
                parts.push(f.to_string());
            } else if let Some(b) = v_deref.as_bool() {
                parts.push(if b { "1".to_string() } else { String::new() });
            } else if v_deref.is_object() {
                if let Ok(res) = ctx.call_method_synchronously(v_deref, "__toString", vec![]) {
                    if let Some(sp) = res.as_string_ptr() {
                        parts.push(unsafe { &*(sp as *const String) }.clone());
                    } else {
                        parts.push(String::new());
                    }
                } else {
                    parts.push(String::new());
                }
            } else {
                parts.push(String::new());
            }
        }
        let result = crate::into_raw(Box::new(parts.join(&glue_str)));
        Ok(Value::new_string_ptr(result as *mut ()))
    } else if let Some(sp) = pieces_val.as_string_ptr() {
        let s = unsafe { &*(sp as *const String) }.clone();
        let result = crate::into_raw(Box::new(s));
        Ok(Value::new_string_ptr(result as *mut ()))
    } else {
        let result = crate::into_raw(Box::new(String::new()));
        Ok(Value::new_string_ptr(result as *mut ()))
    }
}

// join is an alias for implode
php_function! {
    native_join(glue: String, pieces: Value) {
        // Delegate to implode logic
        if let (Some(g), Some(p)) = (glue, pieces) {
            if let Some(arr_ptr) = p.as_array_ptr() {
                let arr = unsafe { &*(arr_ptr as *const PhpArray) };
                let parts: Vec<String> = arr.elements.values().map(|v| {
                    if let Some(sp) = v.as_string_ptr() {
                        unsafe { &*(sp as *const String) }.clone()
                    } else if let Some(i) = v.as_int() {
                        i.to_string()
                    } else {
                        String::new()
                    }
                }).collect();
                let result = crate::into_raw(Box::new(parts.join(g.as_str())));
                Ok(Value::new_string_ptr(result as *mut ()))
            } else {
                Err("join() expects parameter 2 to be array".to_string())
            }
        } else {
            Err("join() expects at least 2 parameters".to_string())
        }
    }
}

php_function! {
    native_str_split(string: String, length: Value) |ctx| { 
        if let Some(s) = string {
            let chunk_len = length.and_then(|v| v.deref().as_int()).unwrap_or(1);
            let chunk_len = (chunk_len.max(1) as usize).min(s.len().max(1));
            let mut php_arr = PhpArray::new();
            let mut i = 0i64;
            let mut start = 0;
            while start < s.len() {
                let end = (start + chunk_len).min(s.len());
                let boxed = crate::into_raw(Box::new(s[start..end].to_string()));
                php_arr.insert_int(i, Value::new_string_ptr(boxed as *mut ()));
                start = end;
                i += 1;
            }
            let arr_ptr = ctx.get_arena().alloc(php_arr);
            Ok(Value::new_array_ptr(arr_ptr as *mut ()))
        } else {
            Err("str_split() expects at least 1 parameter".to_string())
        }
    }
}

php_function! {
    native_chunk_split(body: String, chunklen: Value, end: String) {
        if let Some(s) = body {
            let chunk = chunklen.and_then(|v| v.deref().as_int()).unwrap_or(76);
            let chunk = (chunk.max(1) as usize).min(s.len().max(1));
            let ending = end.cloned().unwrap_or_else(|| "\r\n".to_string());
            let mut result = String::new();
            let mut start = 0;
            while start < s.len() {
                let e = (start + chunk).min(s.len());
                result.push_str(&s[start..e]);
                result.push_str(&ending);
                start = e;
            }
            let boxed = crate::into_raw(Box::new(result));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        } else {
            Err("chunk_split() expects at least 1 parameter".to_string())
        }
    }
}

php_function! {
    native_wordwrap(string: String, width: Value, brk: String, cut_long: Value) {
        if let Some(s) = string {
            let w = width.and_then(|v| v.deref().as_int()).unwrap_or(75).max(1) as usize;
            let br_str = brk.cloned().unwrap_or_else(|| "\n".to_string());
            let _cut = cut_long.and_then(|v| v.deref().as_bool()).unwrap_or(false);
            // Simple word-wrap
            let mut result = String::new();
            let mut line_len = 0;
            for word in s.split(' ') {
                if line_len + word.len() > w && line_len > 0 {
                    result.push_str(&br_str);
                    line_len = 0;
                }
                if line_len > 0 {
                    result.push(' ');
                    line_len += 1;
                }
                result.push_str(word);
                line_len += word.len();
            }
            let boxed = crate::into_raw(Box::new(result));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        } else {
            Err("wordwrap() expects at least 1 parameter".to_string())
        }
    }
}

// --- Extract/Generate Functions ---

php_function! {
    native_substr(string: Value, start: Value, length: Value) {
        let s = str_val_to_string(string);
        if let Some(st) = start {
            let bytes = s.as_bytes();
            let total_len = bytes.len();
            let mut start_idx = st.as_int().unwrap_or(0);
            if start_idx < 0 { start_idx = (total_len as i32 + start_idx).max(0); }
            let start_idx = start_idx as usize;
            if start_idx >= total_len {
                let boxed = crate::into_raw(Box::new(String::new()));
                return Ok(Value::new_string_ptr(boxed as *mut ()));
            }
            let slice: &[u8] = if let Some(l) = length.and_then(|v| v.as_int()) {
                if l < 0 {
                    let end_pos = (total_len as i64) + (l as i64);
                    if end_pos <= start_idx as i64 {
                        &[]
                    } else {
                        let end = (end_pos as usize).min(total_len);
                        &bytes[start_idx..end]
                    }
                } else {
                    let end = start_idx.saturating_add(l as usize).min(total_len);
                    &bytes[start_idx..end]
                }
            } else {
                &bytes[start_idx..]
            };
            let out_str = unsafe { String::from_utf8_unchecked(slice.to_vec()) };
            let boxed = crate::into_raw(Box::new(out_str));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        } else {
            Err("substr() expects at least 2 parameters".to_string())
        }
    }
}

php_function! {
    native_str_repeat(input: String, times: Value) {
        if let (Some(s), Some(t)) = (input, times) {
            let n = t.deref().to_int_coerced();
            if n <= 0 {
                let result = crate::into_raw(Box::new(String::new()));
                return Ok(Value::new_string_ptr(result as *mut ()));
            }
            let n = (n as usize).min(50_000_000);
            let result = crate::into_raw(Box::new(s.repeat(n)));
            Ok(Value::new_string_ptr(result as *mut ()))
        } else {
            Err("str_repeat() expects exactly 2 parameters".to_string())
        }
    }
}

php_function! {
    native_str_pad(input: String, length: Value, pad_string: String, pad_type: Value) {
        if let (Some(s), Some(l)) = (input, length) {
            let target_len = l.deref().to_int_coerced();
            if target_len <= s.len() as i64 {
                let boxed = crate::into_raw(Box::new(s.clone()));
                return Ok(Value::new_string_ptr(boxed as *mut ()));
            }
            let target_len = (target_len as usize).min(50_000_000);
            let pad = pad_string.cloned().unwrap_or_else(|| " ".to_string());
            if pad.is_empty() {
                let boxed = crate::into_raw(Box::new(s.clone()));
                return Ok(Value::new_string_ptr(boxed as *mut ()));
            }
            let pad_t = pad_type.and_then(|v| v.deref().as_int()).unwrap_or(1); // STR_PAD_RIGHT=1, LEFT=0, BOTH=2
            
            let diff = target_len.saturating_sub(s.len());
            let result = match pad_t {
                0 => { // STR_PAD_LEFT
                    let padding: String = pad.chars().cycle().take(diff).collect();
                    format!("{}{}", padding, s)
                }
                2 => { // STR_PAD_BOTH
                    let left = diff / 2;
                    let right = diff - left;
                    let lpad: String = pad.chars().cycle().take(left).collect();
                    let rpad: String = pad.chars().cycle().take(right).collect();
                    format!("{}{}{}", lpad, s, rpad)
                }
                _ => { // STR_PAD_RIGHT (default)
                    let padding: String = pad.chars().cycle().take(diff).collect();
                    format!("{}{}", s, padding)
                }
            };
            let boxed = crate::into_raw(Box::new(result));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        } else {
            Err("str_pad() expects at least 2 parameters".to_string())
        }
    }
}

php_function! {
    native_str_shuffle(s: String) {
        if let Some(s) = s {
            use rand::seq::SliceRandom;
            let mut chars: Vec<char> = s.chars().collect();
            let mut rng = rand::thread_rng();
            chars.shuffle(&mut rng);
            let result: String = chars.into_iter().collect();
            let boxed = crate::into_raw(Box::new(result));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        } else {
            Err("str_shuffle() expects exactly 1 parameter".to_string())
        }
    }
}

// --- PHP 8.x String Functions ---

php_function! {
    native_str_starts_with(haystack: String, needle: String) {
        if let (Some(h), Some(n)) = (haystack, needle) {
            Ok(Value::new_bool(h.starts_with(n.as_str())))
        } else {
            Err("str_starts_with() expects exactly 2 parameters".to_string())
        }
    }
}

php_function! {
    native_str_ends_with(haystack: String, needle: String) {
        if let (Some(h), Some(n)) = (haystack, needle) {
            Ok(Value::new_bool(h.ends_with(n.as_str())))
        } else {
            Err("str_ends_with() expects exactly 2 parameters".to_string())
        }
    }
}

php_function! {
    native_str_contains(haystack: String, needle: String) {
        if let (Some(h), Some(n)) = (haystack, needle) {
            Ok(Value::new_bool(h.contains(n.as_str())))
        } else {
            Err("str_contains() expects exactly 2 parameters".to_string())
        }
    }
}

// --- Encoding Functions ---

php_function! {
    native_ord(string: Value) {
        if let Some(val) = string {
            if let Some(s_ptr) = val.as_string_ptr() {
                let s = unsafe { &*(s_ptr as *const String) };
                let c = s.bytes().next().unwrap_or(0);
                Ok(Value::new_int(c as i32))
            } else if let Some(i) = val.as_int() {
                let s = i.to_string();
                let c = s.bytes().next().unwrap_or(0);
                Ok(Value::new_int(c as i32))
            } else {
                Ok(Value::new_int(0))
            }
        } else {
            Err("ord() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_chr(ascii: Value) {
        if let Some(v) = ascii {
            let code = v.as_int().unwrap_or(0) as u8;
            let result = crate::into_raw(Box::new(String::from(code as char)));
            Ok(Value::new_string_ptr(result as *mut ()))
        } else {
            Err("chr() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_bin2hex(string: String) {
        if let Some(s) = string {
            let hex: String = s.bytes().map(|b| format!("{:02x}", b)).collect();
            let boxed = crate::into_raw(Box::new(hex));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        } else {
            Err("bin2hex() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_hex2bin(hex_string: String) {
        if let Some(s) = hex_string {
            let bytes: Result<Vec<u8>, _> = (0..s.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&s[i..i+2], 16))
                .collect();
            match bytes {
                Ok(b) => {
                    let result = crate::into_raw(Box::new(unsafe { String::from_utf8_unchecked(b) }));
                    Ok(Value::new_string_ptr(result as *mut ()))
                }
                Err(_) => Ok(Value::new_bool(false)),
            }
        } else {
            Err("hex2bin() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_base64_encode(data: String) {
        if let Some(s) = data {
            use base64::Engine;
            let encoded = base64::engine::general_purpose::STANDARD.encode(s.as_bytes());
            let boxed = crate::into_raw(Box::new(encoded));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        } else {
            Err("base64_encode() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_base64_decode(data: String) {
        if let Some(s) = data {
            use base64::Engine;
            match base64::engine::general_purpose::STANDARD.decode(s.as_bytes()) {
                Ok(decoded) => {
                    let result = crate::into_raw(Box::new(unsafe { String::from_utf8_unchecked(decoded) }));
                    Ok(Value::new_string_ptr(result as *mut ()))
                }
                Err(_) => Ok(Value::new_bool(false)),
            }
        } else {
            Err("base64_decode() expects exactly 1 parameter".to_string())
        }
    }
}

// --- Format Functions ---

php_function! {
    native_nl2br(string: String) {
        if let Some(s) = string {
            let result = s.replace("\n", "<br />\n");
            let boxed = crate::into_raw(Box::new(result));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        } else {
            Err("nl2br() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_number_format(number: Value, decimals: Value, dec_point: String, thousands_sep: String) {
        if let Some(num) = number {
            let n = if let Some(f) = num.as_float() { f } 
                    else if let Some(i) = num.as_int() { i as f64 } 
                    else { 0.0 };
            let dec = decimals.and_then(|v| v.deref().as_int()).unwrap_or(0).max(0).min(100) as usize;
            let dp = dec_point.cloned().unwrap_or_else(|| ".".to_string());
            let ts = thousands_sep.cloned().unwrap_or_else(|| ",".to_string());
            
            let formatted = format!("{:.prec$}", n, prec = dec);
            let parts: Vec<&str> = formatted.split('.').collect();
            let int_part = parts[0];
            
            // Add thousands separator
            let negative = int_part.starts_with('-');
            let digits = if negative { &int_part[1..] } else { int_part };
            let mut with_sep = String::new();
            for (i, c) in digits.chars().rev().enumerate() {
                if i > 0 && i % 3 == 0 {
                    with_sep.push_str(&ts.chars().rev().collect::<String>());
                }
                with_sep.push(c);
            }
            let int_formatted: String = with_sep.chars().rev().collect();
            let int_formatted = if negative { format!("-{}", int_formatted) } else { int_formatted };
            
            let result = if dec > 0 && parts.len() > 1 {
                format!("{}{}{}", int_formatted, dp, parts[1])
            } else {
                int_formatted
            };
            let boxed = crate::into_raw(Box::new(result));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        } else {
            Err("number_format() expects at least 1 parameter".to_string())
        }
    }
}

fn value_to_string_for_sprintf(ctx: &mut Option<&mut dyn hyperion_core::types::function::NativeContext>, val: &Value) -> String {
    if let Some(s) = val.as_string_ptr() {
        unsafe { (*(s as *const String)).clone() }
    } else if let Some(i) = val.as_int() {
        i.to_string()
    } else if let Some(f) = val.as_float() {
        f.to_string()
    } else if let Some(b) = val.as_bool() {
        if b { "1".to_string() } else { String::new() }
    } else if val.is_null() {
        String::new()
    } else if val.as_array_ptr().is_some() {
        "Array".to_string()
    } else if val.as_object_ptr().is_some() {
        if let Some(ctx_ref) = ctx {
            let obj_ptr = val.as_object_ptr().unwrap() as *const hyperion_core::types::object::PhpObject;
            let obj = unsafe { &*obj_ptr };
            let class_name = if let Some(ref name) = obj.class_name {
                name.clone()
            } else {
                ctx_ref.get_class_name(obj.class_id).unwrap_or_default()
            };
            if !class_name.is_empty() && ctx_ref.has_method(&class_name, "__toString") {
                let mut arr = hyperion_core::types::array::PhpArray::new();
                arr.insert_int(0, *val);
                let m_ptr = ctx_ref.get_arena().alloc_and_track("__toString".to_string());
                arr.insert_int(1, Value::new_string_ptr(m_ptr as *mut ()));
                let arr_ptr = ctx_ref.get_arena().alloc_and_track(arr);
                let callable_arr = Value::new_array_ptr(arr_ptr as *mut ());
                if let Ok(res) = ctx_ref.call_callable_synchronously(callable_arr, vec![]) {
                    if let Some(s_ptr) = res.as_string_ptr() {
                        return unsafe { (*(s_ptr as *const String)).clone() };
                    }
                }
            }
        }
        "Object".to_string()
    } else {
        String::new()
    }
}

fn value_to_int_for_sprintf(val: &Value) -> i64 {
    if let Some(i) = val.as_int() {
        i as i64
    } else if let Some(f) = val.as_float() {
        f as i64
    } else if let Some(b) = val.as_bool() {
        if b { 1 } else { 0 }
    } else if let Some(s) = val.as_string_ptr() {
        let s = unsafe { &*(s as *const String) };
        s.trim().parse::<i64>().unwrap_or(0)
    } else {
        0
    }
}

fn value_to_float_for_sprintf(val: &Value) -> f64 {
    if let Some(f) = val.as_float() {
        f
    } else if let Some(i) = val.as_int() {
        i as f64
    } else if let Some(b) = val.as_bool() {
        if b { 1.0 } else { 0.0 }
    } else if let Some(s) = val.as_string_ptr() {
        let s = unsafe { &*(s as *const String) };
        s.trim().parse::<f64>().unwrap_or(0.0)
    } else {
        0.0
    }
}

pub fn format_sprintf(mut ctx: Option<&mut dyn hyperion_core::types::function::NativeContext>, fmt: &str, args: &[Value]) -> Result<String, String> {
    let mut result = String::new();
    let chars: Vec<char> = fmt.chars().collect();
    let mut i = 0;
    let mut auto_arg_idx = 0;

    while i < chars.len() {
        if chars[i] == '%' {
            i += 1;
            if i >= chars.len() {
                result.push('%');
                break;
            }
            if chars[i] == '%' {
                result.push('%');
                i += 1;
                continue;
            }

            // 1. Positional argument: digits followed by '$'
            let mut pos_arg: Option<usize> = None;
            let mut j = i;
            while j < chars.len() && chars[j].is_ascii_digit() {
                j += 1;
            }
            if j < chars.len() && chars[j] == '$' && j > i {
                let num_str: String = chars[i..j].iter().collect();
                if let Ok(num) = num_str.parse::<usize>() {
                    if num > 0 {
                        pos_arg = Some(num - 1);
                        i = j + 1;
                    }
                }
            }

            // 2. Flags: '-' (left-align), '+' (sign), ' ' (space sign), '0' (zero-pad), ''c' (custom pad char)
            let mut left_align = false;
            let mut show_sign = false;
            let mut space_sign = false;
            let mut pad_char = ' ';
            let mut zero_pad = false;

            while i < chars.len() {
                match chars[i] {
                    '-' => { left_align = true; i += 1; }
                    '+' => { show_sign = true; i += 1; }
                    ' ' => { space_sign = true; i += 1; }
                    '0' => { zero_pad = true; i += 1; }
                    '\'' => {
                        i += 1;
                        if i < chars.len() {
                            pad_char = chars[i];
                            i += 1;
                        }
                    }
                    _ => break,
                }
            }
            if zero_pad && !left_align && pad_char == ' ' {
                pad_char = '0';
            }

            // 3. Width
            let mut width: Option<usize> = None;
            let width_start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            if i > width_start {
                let w_str: String = chars[width_start..i].iter().collect();
                width = w_str.parse::<usize>().ok();
            }

            // 4. Precision
            let mut precision: Option<usize> = None;
            if i < chars.len() && chars[i] == '.' {
                i += 1;
                let prec_start = i;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
                if i > prec_start {
                    let p_str: String = chars[prec_start..i].iter().collect();
                    precision = p_str.parse::<usize>().ok();
                } else {
                    precision = Some(0);
                }
            }

            if i >= chars.len() {
                break;
            }

            // 5. Specifier
            let specifier = chars[i];
            i += 1;

            let arg_index = if let Some(p) = pos_arg {
                p
            } else {
                let curr = auto_arg_idx;
                auto_arg_idx += 1;
                curr
            };

            let default_val = Value::null();
            let arg_val = args.get(arg_index).unwrap_or(&default_val);

            let mut formatted = match specifier {
                's' => {
                    let mut s = value_to_string_for_sprintf(&mut ctx, arg_val);
                    if let Some(p) = precision {
                        if s.len() > p {
                            s.truncate(p);
                        }
                    }
                    s
                }
                'd' | 'i' => {
                    let num = value_to_int_for_sprintf(arg_val);
                    let prefix = if num >= 0 {
                        if show_sign { "+" } else if space_sign { " " } else { "" }
                    } else {
                        "-"
                    };
                    let abs_str = num.abs().to_string();
                    format!("{}{}", prefix, abs_str)
                }
                'u' => {
                    let num = value_to_int_for_sprintf(arg_val) as u64;
                    num.to_string()
                }
                'f' | 'F' => {
                    let num = value_to_float_for_sprintf(arg_val);
                    let prec = precision.unwrap_or(6).min(100);
                    let prefix = if num >= 0.0 {
                        if show_sign { "+" } else if space_sign { " " } else { "" }
                    } else {
                        ""
                    };
                    format!("{}{:.prec$}", prefix, num, prec = prec)
                }
                'x' => {
                    let num = value_to_int_for_sprintf(arg_val) as u64;
                    format!("{:x}", num)
                }
                'X' => {
                    let num = value_to_int_for_sprintf(arg_val) as u64;
                    format!("{:X}", num)
                }
                'b' => {
                    let num = value_to_int_for_sprintf(arg_val) as u64;
                    format!("{:b}", num)
                }
                'o' => {
                    let num = value_to_int_for_sprintf(arg_val) as u64;
                    format!("{:o}", num)
                }
                'c' => {
                    let num = value_to_int_for_sprintf(arg_val);
                    let ch = (num as u8) as char;
                    ch.to_string()
                }
                'e' => {
                    let num = value_to_float_for_sprintf(arg_val);
                    let prec = precision.unwrap_or(6);
                    format!("{:.prec$e}", num, prec = prec)
                }
                'E' => {
                    let num = value_to_float_for_sprintf(arg_val);
                    let prec = precision.unwrap_or(6);
                    format!("{:.prec$E}", num, prec = prec)
                }
                _ => {
                    result.push('%');
                    result.push(specifier);
                    continue;
                }
            };

            // Apply width and padding
            if let Some(w) = width {
                if formatted.len() < w {
                    let pad_len = w - formatted.len();
                    let padding: String = std::iter::repeat(pad_char).take(pad_len).collect();
                    if left_align {
                        formatted.push_str(&padding);
                    } else if pad_char == '0' && (formatted.starts_with('-') || formatted.starts_with('+') || formatted.starts_with(' ')) {
                        let sign = formatted.remove(0);
                        formatted = format!("{}{}{}", sign, padding, formatted);
                    } else {
                        formatted = format!("{}{}", padding, formatted);
                    }
                }
            }

            result.push_str(&formatted);
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    Ok(result)
}

php_function! {
    native_sprintf(format: String, ...rest) |ctx| {
        if let Some(fmt) = format {
            let res = format_sprintf(Some(ctx), fmt, rest)?;
            let boxed = crate::into_raw(Box::new(res));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        } else {
            Err("sprintf() expects at least 1 parameter".to_string())
        }
    }
}

php_function! {
    native_printf(format: String, ...rest) |ctx| {
        if let Some(fmt) = format {
            let res = format_sprintf(Some(ctx), fmt, rest)?;
            ctx.write_output(res.as_bytes());
            Ok(Value::new_int(res.len() as i32))
        } else {
            Err("printf() expects at least 1 parameter".to_string())
        }
    }
}

// --- Misc ---

php_function! {
    native_str_word_count(string: String) {
        if let Some(s) = string {
            let count = s.split_whitespace().count();
            Ok(Value::new_int(count as i32))
        } else {
            Err("str_word_count() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_substr_count(haystack: String, needle: String) {
        if let (Some(h), Some(n)) = (haystack, needle) {
            let count = h.matches(n.as_str()).count();
            Ok(Value::new_int(count as i32))
        } else {
            Err("substr_count() expects exactly 2 parameters".to_string())
        }
    }
}

php_function! {
    native_str_rev(string: String) {
        if let Some(s) = string {
            let reversed: String = s.chars().rev().collect();
            let boxed = crate::into_raw(Box::new(reversed));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        } else {
            Err("strrev() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_md5(string: String) {
        if let Some(s) = string {
            use md5::Digest;
            let hash = md5::Md5::digest(s.as_bytes());
            let hex: String = hash.iter().map(|b| format!("{:02x}", b)).collect();
            let boxed = crate::into_raw(Box::new(hex));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        } else {
            Err("md5() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_sha1(string: String) {
        if let Some(s) = string {
            use sha1::Digest;
            let hash = sha1::Sha1::digest(s.as_bytes());
            let hex: String = hash.iter().map(|b| format!("{:02x}", b)).collect();
            let boxed = crate::into_raw(Box::new(hex));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        } else {
            Err("sha1() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_md5_file(filename: String, binary: Value) {
        if let Some(f) = filename {
            use md5::Digest;
            let data = match std::fs::read(&f) {
                Ok(bytes) => bytes,
                Err(_) => return Ok(Value::new_bool(false)),
            };
            let hash = md5::Md5::digest(&data);
            let is_binary = binary.map(|b| b.deref()).and_then(|b| b.as_bool()).unwrap_or(false);
            if is_binary {
                let s = String::from_utf8_lossy(&hash).into_owned();
                let boxed = crate::into_raw(Box::new(s));
                Ok(Value::new_string_ptr(boxed as *mut ()))
            } else {
                let hex: String = hash.iter().map(|b| format!("{:02x}", b)).collect();
                let boxed = crate::into_raw(Box::new(hex));
                Ok(Value::new_string_ptr(boxed as *mut ()))
            }
        } else {
            Err("md5_file() expects at least 1 parameter".to_string())
        }
    }
}

php_function! {
    native_sha1_file(filename: String, binary: Value) {
        if let Some(f) = filename {
            use sha1::Digest;
            let data = match std::fs::read(&f) {
                Ok(bytes) => bytes,
                Err(_) => return Ok(Value::new_bool(false)),
            };
            let hash = sha1::Sha1::digest(&data);
            let is_binary = binary.map(|b| b.deref()).and_then(|b| b.as_bool()).unwrap_or(false);
            if is_binary {
                let s = String::from_utf8_lossy(&hash).into_owned();
                let boxed = crate::into_raw(Box::new(s));
                Ok(Value::new_string_ptr(boxed as *mut ()))
            } else {
                let hex: String = hash.iter().map(|b| format!("{:02x}", b)).collect();
                let boxed = crate::into_raw(Box::new(hex));
                Ok(Value::new_string_ptr(boxed as *mut ()))
            }
        } else {
            Err("sha1_file() expects at least 1 parameter".to_string())
        }
    }
}

php_function! {
    native_htmlspecialchars(string: Value, flags: Value) |ctx| {
        if let Some(str_val) = string {
            let mut ctx_opt: Option<&mut dyn hyperion_core::types::function::NativeContext> = Some(ctx);
            let s = if str_val.is_null() {
                String::new()
            } else if let Some(s_ptr) = str_val.as_string_ptr() {
                unsafe { (*(s_ptr as *const String)).clone() }
            } else if let Some(i) = str_val.as_int() {
                i.to_string()
            } else if let Some(f) = str_val.as_float() {
                f.to_string()
            } else if let Some(b) = str_val.as_bool() {
                if b { "1".to_string() } else { String::new() }
            } else if str_val.is_object() {
                value_to_string_for_sprintf(&mut ctx_opt, &str_val)
            } else {
                String::new()
            };
            // Default in PHP 8 is ENT_QUOTES | ENT_SUBSTITUTE | ENT_HTML401 (81)
            let f = flags.and_then(|v| v.as_int()).unwrap_or(81) as u32;
            let mut result = String::with_capacity(s.len());
            
            let ent_compat = (f & 3) == 2;
            let ent_quotes = (f & 3) == 3;
            
            for c in s.chars() {
                match c {
                    '&' => result.push_str("&amp;"),
                    '<' => result.push_str("&lt;"),
                    '>' => result.push_str("&gt;"),
                    '"' if ent_compat || ent_quotes => result.push_str("&quot;"),
                    '\'' if ent_quotes => {
                        let html5 = (f & 48) == 48; // ENT_HTML5
                        if html5 {
                            result.push_str("&apos;");
                        } else {
                            result.push_str("&#039;");
                        }
                    },
                    _ => result.push(c),
                }
            }
            
            let boxed = crate::into_raw(Box::new(result));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        } else {
            Err("htmlspecialchars() expects at least 1 parameter".to_string())
        }
    }
}

php_function! {
    native_htmlspecialchars_decode(string: Value, flags: Value) {
        if let Some(str_val) = string {
            let s = if str_val.is_null() {
                String::new()
            } else if let Some(s_ptr) = str_val.as_string_ptr() {
                unsafe { (*(s_ptr as *const String)).clone() }
            } else if let Some(i) = str_val.as_int() {
                i.to_string()
            } else if let Some(f) = str_val.as_float() {
                f.to_string()
            } else if let Some(b) = str_val.as_bool() {
                if b { "1".to_string() } else { String::new() }
            } else {
                String::new()
            };
            let f = flags.and_then(|v| v.as_int()).unwrap_or(81) as u32;
            
            let ent_compat = (f & 3) == 2;
            let ent_quotes = (f & 3) == 3;
            let html5 = (f & 48) == 48;
            
            let mut result = s.replace("&amp;", "&")
                              .replace("&lt;", "<")
                              .replace("&gt;", ">");
                              
            if ent_compat || ent_quotes {
                result = result.replace("&quot;", "\"");
            }
            if ent_quotes {
                result = result.replace("&#039;", "'")
                               .replace("&#39;", "'");
                if html5 {
                    result = result.replace("&apos;", "'");
                }
            }
            
            let boxed = crate::into_raw(Box::new(result));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        } else {
            Err("htmlspecialchars_decode() expects at least 1 parameter".to_string())
        }
    }
}

php_function! {
    native_strip_tags(string: Value, allowed_tags: Value) {
        if let Some(str_val) = string {
            let s = if str_val.is_null() {
                String::new()
            } else if let Some(s_ptr) = str_val.as_string_ptr() {
                unsafe { (*(s_ptr as *const String)).clone() }
            } else if let Some(i) = str_val.as_int() {
                i.to_string()
            } else if let Some(f) = str_val.as_float() {
                f.to_string()
            } else if let Some(b) = str_val.as_bool() {
                if b { "1".to_string() } else { String::new() }
            } else {
                String::new()
            };

            let mut allowed_set: std::collections::HashSet<String> = std::collections::HashSet::new();
            if let Some(allowed) = allowed_tags {
                if let Some(arr_ptr) = allowed.as_array_ptr() {
                    let arr = unsafe { &*(arr_ptr as *const hyperion_core::types::array::PhpArray) };
                    for (_, val) in arr.elements.iter() {
                        if let Some(sp) = val.as_string_ptr() {
                            let tag_str = unsafe { &*(sp as *const String) };
                            let clean = tag_str.trim().trim_start_matches('<').trim_end_matches('>').trim().to_lowercase();
                            if !clean.is_empty() {
                                allowed_set.insert(clean);
                            }
                        }
                    }
                } else if let Some(sp) = allowed.as_string_ptr() {
                    let allowed_s = unsafe { &*(sp as *const String) };
                    let mut i = 0;
                    let bytes = allowed_s.as_bytes();
                    while i < bytes.len() {
                        if bytes[i] == b'<' {
                            if let Some(end) = allowed_s[i..].find('>') {
                                let tag = &allowed_s[i+1..i+end].trim().to_lowercase();
                                if !tag.is_empty() {
                                    allowed_set.insert(tag.clone());
                                }
                                i += end + 1;
                                continue;
                            }
                        }
                        i += 1;
                    }
                }
            }

            let mut result = String::with_capacity(s.len());
            let bytes = s.as_bytes();
            let len = bytes.len();
            let mut i = 0;

            while i < len {
                if bytes[i] == b'<' {
                    // Check for comments
                    if i + 4 <= len && &bytes[i..i+4] == b"<!--" {
                        if let Some(end) = s[i+4..].find("-->") {
                            i = i + 4 + end + 3;
                            continue;
                        } else {
                            break;
                        }
                    }

                    // Find matching '>'
                    let mut in_quote = None;
                    let mut j = i + 1;
                    while j < len {
                        let b = bytes[j];
                        if let Some(q) = in_quote {
                            if b == q {
                                in_quote = None;
                            }
                        } else if b == b'"' || b == b'\'' {
                            in_quote = Some(b);
                        } else if b == b'>' {
                            break;
                        }
                        j += 1;
                    }

                    if j < len && bytes[j] == b'>' {
                        let tag_content = &s[i+1..j];
                        let trimmed = tag_content.trim();
                        let tag_name = if trimmed.starts_with('/') {
                            trimmed[1..].trim_start()
                        } else {
                            trimmed
                        };
                        let name_end = tag_name.find(|c: char| c.is_whitespace() || c == '/' || c == '>').unwrap_or(tag_name.len());
                        let clean_tag = tag_name[..name_end].to_lowercase();

                        if !clean_tag.is_empty() && allowed_set.contains(&clean_tag) {
                            result.push_str(&s[i..=j]);
                        }
                        i = j + 1;
                        continue;
                    }
                }
                result.push(bytes[i] as char);
                i += 1;
            }

            let boxed = crate::into_raw(Box::new(result));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        } else {
            Err("strip_tags() expects at least 1 parameter".to_string())
        }
    }
}

php_function! {
    native_htmlentities(string: String, flags: Value) {
        if let Some(s) = string {
            // For now, htmlentities acts mostly like htmlspecialchars because a full
            // HTML entity translation table is massive and not strictly required for Laravel booting.
            // We apply the same basic replacements as htmlspecialchars.
            let f = flags.and_then(|v| v.as_int()).unwrap_or(81) as u32;
            let mut result = String::with_capacity(s.len());
            
            let ent_compat = (f & 3) == 2;
            let ent_quotes = (f & 3) == 3;
            
            for c in s.chars() {
                match c {
                    '&' => result.push_str("&amp;"),
                    '<' => result.push_str("&lt;"),
                    '>' => result.push_str("&gt;"),
                    '"' if ent_compat || ent_quotes => result.push_str("&quot;"),
                    '\'' if ent_quotes => {
                        let html5 = (f & 48) == 48;
                        if html5 {
                            result.push_str("&apos;");
                        } else {
                            result.push_str("&#039;");
                        }
                    },
                    _ => result.push(c),
                }
            }
            
            let boxed = crate::into_raw(Box::new(result));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        } else {
            Err("htmlentities() expects at least 1 parameter".to_string())
        }
    }
}

php_function! {
    native_html_entity_decode(string: String, flags: Value, encoding: Value) {
        if let Some(s) = string {
            let mut result = s.clone();
            result = result.replace("&amp;", "&");
            result = result.replace("&lt;", "<");
            result = result.replace("&gt;", ">");
            result = result.replace("&quot;", "\"");
            result = result.replace("&apos;", "'");
            result = result.replace("&#039;", "'");
            result = result.replace("&#39;", "'");
            let boxed = crate::into_raw(Box::new(result));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        } else {
            Err("html_entity_decode() expects at least 1 parameter".to_string())
        }
    }
}
php_function! {
    native_vsprintf(format: String, values: Value) |ctx| {
        if let Some(fmt) = format {
            let mut rest = Vec::new();
            if let Some(arr_ptr) = values.unwrap().as_array_ptr() {
                let arr = unsafe { &*(arr_ptr as *const hyperion_core::types::array::PhpArray) };
                for (_k, val) in arr.elements.iter() {
                    rest.push(val.clone());
                }
            } else {
                return Err("vsprintf(): Argument #2 ($values) must be of type array".to_string());
            }

            let res = format_sprintf(Some(ctx), fmt, &rest)?;
            let boxed = crate::into_raw(Box::new(res));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        } else {
            Err("vsprintf() expects at least 1 parameter".to_string())
        }
    }
}

lazy_static::lazy_static! {
    static ref STRTOK_STATE: std::sync::Mutex<Option<(String, usize)>> = std::sync::Mutex::new(None);
}

pub fn native_strtok(args: &[Value], _ctx: &mut dyn hyperion_core::types::function::NativeContext) -> Result<Value, String> {
    if args.is_empty() {
        return Err("strtok() expects at least 1 parameter".to_string());
    }

    fn to_string_opt(v: &Value) -> Option<String> {
        let val = v.deref();
        if let Some(p) = val.as_string_ptr() {
            Some(unsafe { (*(p as *const String)).clone() })
        } else if let Some(i) = val.as_int() {
            Some(i.to_string())
        } else if let Some(f) = val.as_float() {
            Some(f.to_string())
        } else if let Some(b) = val.as_bool() {
            Some(if b { "1".to_string() } else { "".to_string() })
        } else if val.is_null() {
            Some("".to_string())
        } else {
            None
        }
    }

    let mut state = STRTOK_STATE.lock().unwrap();

    let (delims, mut pos) = if args.len() >= 2 {
        let Some(s) = to_string_opt(&args[0]) else {
            return Ok(Value::new_bool(false));
        };
        let Some(d) = to_string_opt(&args[1]) else {
            return Ok(Value::new_bool(false));
        };
        *state = Some((s, 0));
        (d, 0)
    } else {
        let Some(d) = to_string_opt(&args[0]) else {
            return Ok(Value::new_bool(false));
        };
        if let Some((_, p)) = &*state {
            (d, *p)
        } else {
            return Ok(Value::new_bool(false));
        }
    };

    let Some((s, _)) = &*state else {
        return Ok(Value::new_bool(false));
    };

    let s_chars: Vec<char> = s.chars().collect();
    while pos < s_chars.len() && delims.contains(s_chars[pos]) {
        pos += 1;
    }

    if pos >= s_chars.len() {
        if let Some((_, p)) = &mut *state {
            *p = s_chars.len();
        }
        return Ok(Value::new_bool(false));
    }

    let start = pos;
    while pos < s_chars.len() && !delims.contains(s_chars[pos]) {
        pos += 1;
    }

    let token: String = s_chars[start..pos].iter().collect();

    if pos < s_chars.len() && delims.contains(s_chars[pos]) {
        pos += 1;
    }

    if let Some((_, p)) = &mut *state {
        *p = pos;
    }

    let boxed = crate::into_raw(Box::new(token));
    Ok(Value::new_string_ptr(boxed as *mut ()))
}

php_function! {
    native_strcmp(str1: String, str2: String) {
        if let (Some(s1), Some(s2)) = (str1, str2) {
            let res = match s1.cmp(&s2) {
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Greater => 1,
            };
            Ok(Value::new_int(res))
        } else {
            Err("strcmp() expects exactly 2 parameters".to_string())
        }
    }
}

php_function! {
    native_strcasecmp(str1: String, str2: String) {
        if let (Some(s1), Some(s2)) = (str1, str2) {
            let res = match s1.to_lowercase().cmp(&s2.to_lowercase()) {
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Greater => 1,
            };
            Ok(Value::new_int(res))
        } else {
            Err("strcasecmp() expects exactly 2 parameters".to_string())
        }
    }
}

php_function! {
    native_strncasecmp(str1: String, str2: String, length: Value) {
        if let (Some(s1), Some(s2)) = (str1, str2) {
            let len = length.and_then(|v| v.as_int()).unwrap_or(0);
            if len < 0 {
                return Ok(Value::new_bool(false));
            }
            let len = len as usize;
            let s1_low = s1.to_lowercase();
            let s2_low = s2.to_lowercase();
            let slice1 = if s1_low.len() > len { &s1_low[..len] } else { &s1_low };
            let slice2 = if s2_low.len() > len { &s2_low[..len] } else { &s2_low };
            let res = match slice1.cmp(slice2) {
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Greater => 1,
            };
            Ok(Value::new_int(res))
        } else {
            Err("strncasecmp() expects exactly 3 parameters".to_string())
        }
    }
}

php_function! {
    native_substr_compare(main_str: String, str: String, offset: Value, length: Value, case_insensitive: Value) {
        if let (Some(s1), Some(s2)) = (main_str, str) {
            let off = offset.and_then(|v| v.as_int()).unwrap_or(0);
            let s1_chars: Vec<char> = s1.chars().collect();
            let actual_offset = if off < 0 {
                let o = s1_chars.len() as i32 + off;
                if o < 0 { return Ok(Value::new_bool(false)); }
                o as usize
            } else {
                off as usize
            };

            if actual_offset > s1_chars.len() {
                return Ok(Value::new_bool(false));
            }

            let len_opt = length.and_then(|v| v.as_int());
            let sub_chars = match len_opt {
                Some(l) if l >= 0 => {
                    let end = std::cmp::min(actual_offset + l as usize, s1_chars.len());
                    &s1_chars[actual_offset..end]
                }
                _ => &s1_chars[actual_offset..],
            };

            let sub: String = sub_chars.iter().collect();
            let is_case_ins = case_insensitive.map(|v| v.is_truthy()).unwrap_or(false);

            let res = if is_case_ins {
                match sub.to_lowercase().cmp(&s2.to_lowercase()) {
                    std::cmp::Ordering::Less => -1,
                    std::cmp::Ordering::Equal => 0,
                    std::cmp::Ordering::Greater => 1,
                }
            } else {
                match sub.cmp(&s2) {
                    std::cmp::Ordering::Less => -1,
                    std::cmp::Ordering::Equal => 0,
                    std::cmp::Ordering::Greater => 1,
                }
            };
            Ok(Value::new_int(res))
        } else {
            Err("substr_compare() expects at least 3 parameters".to_string())
        }
    }
}

php_function! {
    native_strcspn(string: String, characters: String, offset: Value, length: Value) {
        if let (Some(s), Some(chars)) = (string, characters) {
            let offset_val = offset.and_then(|v| v.as_int()).unwrap_or(0);
            let s_bytes = s.as_bytes();
            let start = if offset_val < 0 {
                (s_bytes.len() as i32 + offset_val).max(0) as usize
            } else {
                (offset_val as usize).min(s_bytes.len())
            };
            let sub = &s_bytes[start..];
            let len_opt = length.and_then(|v| v.as_int());
            let max_len = if let Some(l) = len_opt {
                if l < 0 {
                    (sub.len() as i32 + l).max(0) as usize
                } else {
                    (l as usize).min(sub.len())
                }
            } else {
                sub.len()
            };
            let target = &sub[..max_len];
            let char_mask: std::collections::HashSet<u8> = chars.as_bytes().iter().copied().collect();
            let mut count = 0;
            for b in target {
                if char_mask.contains(b) {
                    break;
                }
                count += 1;
            }
            Ok(Value::new_int(count as i32))
        } else {
            Err("strcspn() expects at least 2 parameters".to_string())
        }
    }
}

php_function! {
    native_strspn(string: String, characters: String, offset: Value, length: Value) {
        if let (Some(s), Some(chars)) = (string, characters) {
            let offset_val = offset.and_then(|v| v.as_int()).unwrap_or(0);
            let s_bytes = s.as_bytes();
            let start = if offset_val < 0 {
                (s_bytes.len() as i32 + offset_val).max(0) as usize
            } else {
                (offset_val as usize).min(s_bytes.len())
            };
            let sub = &s_bytes[start..];
            let len_opt = length.and_then(|v| v.as_int());
            let max_len = if let Some(l) = len_opt {
                if l < 0 {
                    (sub.len() as i32 + l).max(0) as usize
                } else {
                    (l as usize).min(sub.len())
                }
            } else {
                sub.len()
            };
            let target = &sub[..max_len];
            let char_mask: std::collections::HashSet<u8> = chars.as_bytes().iter().copied().collect();
            let mut count = 0;
            for b in target {
                if !char_mask.contains(b) {
                    break;
                }
                count += 1;
            }
            Ok(Value::new_int(count as i32))
        } else {
            Err("strspn() expects at least 2 parameters".to_string())
        }
    }
}

fn php_url_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 2);
    for b in input.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            _ => {
                out.push_str(&format!("%{:02X}", b));
            }
        }
    }
    out
}

php_function! {
    native_urlencode(string: String) {
        if let Some(s) = string {
            let encoded = php_url_encode(&s);
            let boxed = crate::into_raw(Box::new(encoded));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        } else {
            Err("urlencode() expects exactly 1 parameter".to_string())
        }
    }
}

fn php_raw_url_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 2);
    for b in input.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push_str(&format!("%{:02X}", b));
            }
        }
    }
    out
}

php_function! {
    native_rawurlencode(string: String) {
        if let Some(s) = string {
            let encoded = php_raw_url_encode(&s);
            let boxed = crate::into_raw(Box::new(encoded));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        } else {
            Err("rawurlencode() expects exactly 1 parameter".to_string())
        }
    }
}


php_function! {
    native_http_build_query(data: Value, numeric_prefix: Value, arg_separator: Value, encoding_type: Value) |ctx| {
        if let Some(data_val) = data {
            let prefix = numeric_prefix.and_then(|v| v.as_string_ptr()).map(|p| unsafe { &*(p as *const String) }.clone()).unwrap_or_default();
            let sep = arg_separator.and_then(|v| v.as_string_ptr()).map(|p| unsafe { &*(p as *const String) }.clone()).unwrap_or_else(|| "&".to_string());
            
            let mut pairs = Vec::new();
            if let Some(arr_ptr) = data_val.as_array_ptr() {
                let arr = unsafe { &*(arr_ptr as *const hyperion_core::types::array::PhpArray) };
                for (k, v) in arr.elements.iter() {
                    let k_str = match k {
                        hyperion_core::types::array::ArrayKey::StringId(id) => ctx.lookup_string(*id).unwrap_or_default(),
                        hyperion_core::types::array::ArrayKey::Int(i) => {
                            if !prefix.is_empty() {
                                format!("{}{}", prefix, i)
                            } else {
                                i.to_string()
                            }
                        }
                    };
                    let v_str = if let Some(s_ptr) = v.as_string_ptr() {
                        unsafe { &*(s_ptr as *const String) }.clone()
                    } else if let Some(i) = v.as_int() {
                        i.to_string()
                    } else if let Some(b) = v.as_bool() {
                        if b { "1".to_string() } else { "0".to_string() }
                    } else if let Some(f) = v.as_float() {
                        f.to_string()
                    } else {
                        String::new()
                    };
                    let encoded_k = php_url_encode(&k_str);
                    let encoded_v = php_url_encode(&v_str);
                    pairs.push(format!("{}={}", encoded_k, encoded_v));
                }
            }
            let res = pairs.join(&sep);
            let boxed = crate::into_raw(Box::new(res));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        } else {
            Err("http_build_query() expects at least 1 parameter".to_string())
        }
    }
}

pub fn php_url_decode(s: &str) -> String {
    let mut res = String::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'+' {
            res.push(' ');
            i += 1;
        } else if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(h) = u8::from_str_radix(std::str::from_utf8(&bytes[i+1..i+3]).unwrap_or(""), 16) {
                res.push(h as char);
                i += 3;
            } else {
                res.push(bytes[i] as char);
                i += 1;
            }
        } else {
            res.push(bytes[i] as char);
            i += 1;
        }
    }
    res
}

pub fn native_parse_str(args: &[Value], ctx: &mut dyn hyperion_core::types::function::NativeContext) -> Result<Value, String> {
    if args.is_empty() {
        return Err("parse_str() expects at least 1 parameter".to_string());
    }
    let input_str = crate::pcre::value_to_string(&args[0]).unwrap_or_default();
    let mut out_arr = PhpArray::new();

    for pair in input_str.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (k_raw, v_raw) = if let Some(eq_idx) = pair.find('=') {
            (&pair[..eq_idx], &pair[eq_idx + 1..])
        } else {
            (pair, "")
        };
        let key = php_url_decode(k_raw);
        let val = php_url_decode(v_raw);

        let val_val = Value::new_string_ptr(crate::into_raw(Box::new(val)) as *mut ());
        let key_id = hyperion_core::types::string_table::intern_string(&key);
        out_arr.insert_string_id(key_id, val_val);
    }

    if args.len() > 1 {
        let boxed_arr = crate::into_raw(Box::new(out_arr));
        crate::pcre::write_out_var(&args[1], Value::new_array_ptr(boxed_arr as *mut ()), ctx);
    }
    Ok(Value::null())
}

fn compare_versions(v1: &str, v2: &str) -> std::cmp::Ordering {
    fn canonicalize(s: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current = String::new();
        let mut is_num = None;
        for c in s.chars() {
            if c == '.' || c == '-' || c == '_' || c == '+' {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                    is_num = None;
                }
            } else if c.is_ascii_digit() {
                if is_num == Some(false) && !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
                is_num = Some(true);
                current.push(c);
            } else {
                if is_num == Some(true) && !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
                is_num = Some(false);
                current.push(c.to_ascii_lowercase());
            }
        }
        if !current.is_empty() {
            tokens.push(current);
        }
        tokens
    }

    fn special_rank(s: &str) -> Option<i32> {
        match s {
            "dev" => Some(0),
            "alpha" | "a" => Some(1),
            "beta" | "b" => Some(2),
            "rc" => Some(3),
            "#" => Some(4),
            "pl" | "p" => Some(5),
            _ => None,
        }
    }

    let t1 = canonicalize(v1);
    let t2 = canonicalize(v2);
    let max_len = t1.len().max(t2.len());

    for i in 0..max_len {
        let p1 = t1.get(i).map(|s| s.as_str()).unwrap_or("#");
        let p2 = t2.get(i).map(|s| s.as_str()).unwrap_or("#");

        let n1 = p1.parse::<i64>();
        let n2 = p2.parse::<i64>();

        let ord = match (n1, n2) {
            (Ok(num1), Ok(num2)) => num1.cmp(&num2),
            (Ok(_), Err(_)) => {
                if let Some(r2) = special_rank(p2) {
                    4.cmp(&r2)
                } else {
                    std::cmp::Ordering::Greater
                }
            }
            (Err(_), Ok(_)) => {
                if let Some(r1) = special_rank(p1) {
                    r1.cmp(&4)
                } else {
                    std::cmp::Ordering::Less
                }
            }
            (Err(_), Err(_)) => {
                let r1 = special_rank(p1);
                let r2 = special_rank(p2);
                match (r1, r2) {
                    (Some(a), Some(b)) => a.cmp(&b),
                    (Some(a), None) => a.cmp(&4),
                    (None, Some(b)) => 4.cmp(&b),
                    (None, None) => p1.cmp(p2),
                }
            }
        };

        if ord != std::cmp::Ordering::Equal {
            return ord;
        }
    }

    std::cmp::Ordering::Equal
}

php_function! {
    native_version_compare(version1: Value, version2: Value, operator: Value) {
        let to_str = |v: Option<&Value>| -> String {
            if let Some(val) = v {
                let deref = val.deref();
                if let Some(sp) = deref.as_string_ptr() {
                    unsafe { &*(sp as *const String) }.clone()
                } else if let Some(i) = deref.as_int() {
                    i.to_string()
                } else if let Some(f) = deref.as_float() {
                    f.to_string()
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        };

        if version1.is_none() || version2.is_none() {
            return Err("version_compare() expects at least 2 parameters".to_string());
        }

        let v1 = to_str(version1);
        let v2 = to_str(version2);
        let ord = compare_versions(&v1, &v2);
        let op_str_opt = operator.and_then(|v| {
            if let Some(sp) = v.deref().as_string_ptr() {
                unsafe { &*(sp as *const String) }.as_str().into()
            } else {
                None
            }
        });

        if let Some(op) = op_str_opt {
            let res = match op {
                "<" | "lt" => ord == std::cmp::Ordering::Less,
                "<=" | "le" => ord != std::cmp::Ordering::Greater,
                ">" | "gt" => ord == std::cmp::Ordering::Greater,
                ">=" | "ge" => ord != std::cmp::Ordering::Less,
                "==" | "=" | "eq" => ord == std::cmp::Ordering::Equal,
                "!=" | "<>" | "ne" => ord != std::cmp::Ordering::Equal,
                _ => false,
            };
            Ok(Value::new_bool(res))
        } else {
            let num = match ord {
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Greater => 1,
            };
            Ok(Value::new_int(num))
        }
    }
}

php_function! {
    native_str_getcsv(string: String, separator: Value, enclosure: Value, escape: Value) |ctx| {
        let input = string.map(|s| s.as_str()).unwrap_or("");
        let sep = separator.and_then(|v| {
            if let Some(sp) = v.deref().as_string_ptr() {
                unsafe { &*(sp as *const String) }.chars().next()
            } else {
                None
            }
        }).unwrap_or(',');

        let enc = enclosure.and_then(|v| {
            if let Some(sp) = v.deref().as_string_ptr() {
                unsafe { &*(sp as *const String) }.chars().next()
            } else {
                None
            }
        }).unwrap_or('"');

        let esc = escape.and_then(|v| {
            if let Some(sp) = v.deref().as_string_ptr() {
                unsafe { &*(sp as *const String) }.chars().next()
            } else {
                None
            }
        }).unwrap_or('\\');

        let mut fields = Vec::new();
        let mut cur_field = String::new();
        let mut in_enclosure = false;
        let mut chars = input.chars().peekable();

        while let Some(c) = chars.next() {
            if c == esc {
                if let Some(&next_c) = chars.peek() {
                    if next_c == enc || next_c == esc || next_c == sep {
                        cur_field.push(chars.next().unwrap());
                        continue;
                    }
                }
                cur_field.push(c);
            } else if c == enc {
                if in_enclosure {
                    if let Some(&next_c) = chars.peek() {
                        if next_c == enc {
                            cur_field.push(enc);
                            chars.next();
                            continue;
                        }
                    }
                    in_enclosure = false;
                } else {
                    in_enclosure = true;
                }
            } else if c == sep && !in_enclosure {
                fields.push(std::mem::take(&mut cur_field));
            } else {
                cur_field.push(c);
            }
        }
        fields.push(cur_field);

        let mut arr = hyperion_core::types::array::PhpArray::new();
        for field in fields {
            let str_ptr = crate::into_raw(Box::new(field));
            arr.push(Value::new_string_ptr(str_ptr as *mut ()));
        }

        let arr_ptr = ctx.get_arena().alloc(arr);
        Ok(Value::new_array_ptr(arr_ptr as *mut ()))
    }
}

php_function! {
    native_uniqid(prefix: Value, more_entropy: Value) |ctx| {
        let pfx = prefix.and_then(|v| v.deref().as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() }).unwrap_or_default();
        let more = more_entropy.and_then(|v| v.deref().as_bool()).unwrap_or(false);

        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
        let sec = now.as_secs();
        let usec = now.subsec_micros();

        let uid = if more {
            format!("{}{:08x}{:05x}.{:08}", pfx, sec, usec, rand::random::<u32>())
        } else {
            format!("{}{:08x}{:05x}", pfx, sec, usec)
        };

        let ptr = ctx.get_arena().alloc_and_track(uid);
        Ok(Value::new_string_ptr(ptr as *mut ()))
    }
}php_function! {
    native_addslashes(str_val: Value) |ctx| {
        let s = if let Some(v) = str_val {
            let deref = v.deref();
            if let Some(sp) = deref.as_string_ptr() {
                unsafe { (*(sp as *const String)).clone() }
            } else if let Some(i) = deref.as_int() {
                i.to_string()
            } else if let Some(f) = deref.as_float() {
                f.to_string()
            } else if let Some(b) = deref.as_bool() {
                if b { "1".to_string() } else { "".to_string() }
            } else {
                "".to_string()
            }
        } else {
            "".to_string()
        };
        let mut out = String::with_capacity(s.len() + 8);
        for c in s.chars() {
            if c == '\'' || c == '"' || c == '\\' || c == '\0' {
                out.push('\\');
            }
            out.push(c);
        }
        let ptr = ctx.get_arena().alloc_and_track(out);
        Ok(Value::new_string_ptr(ptr as *mut ()))
    }
}

php_function! {
    native_stripslashes(str_val: Value) |ctx| {
        let s = if let Some(v) = str_val {
            let deref = v.deref();
            if let Some(sp) = deref.as_string_ptr() {
                unsafe { (*(sp as *const String)).clone() }
            } else if let Some(i) = deref.as_int() {
                i.to_string()
            } else if let Some(f) = deref.as_float() {
                f.to_string()
            } else if let Some(b) = deref.as_bool() {
                if b { "1".to_string() } else { "".to_string() }
            } else {
                "".to_string()
            }
        } else {
            "".to_string()
        };
        let mut out = String::with_capacity(s.len());
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\\' {
                if let Some(next_c) = chars.next() {
                    out.push(next_c);
                }
            } else {
                out.push(c);
            }
        }
        let ptr = ctx.get_arena().alloc_and_track(out);
        Ok(Value::new_string_ptr(ptr as *mut ()))
    }
}

fn build_char_mask(charlist: &str) -> [bool; 256] {
    let mut mask = [false; 256];
    let bytes = charlist.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if i + 3 < bytes.len() && bytes[i + 1] == b'.' && bytes[i + 2] == b'.' {
            let start = bytes[i];
            let end = bytes[i + 3];
            let (min, max) = if start <= end { (start, end) } else { (end, start) };
            for b in min..=max {
                mask[b as usize] = true;
            }
            i += 4;
        } else if bytes[i] == b'\\' && i + 1 < bytes.len() {
            let escaped = match bytes[i + 1] {
                b'n' => b'\n',
                b'r' => b'\r',
                b't' => b'\t',
                b'v' => 0x0B,
                b'f' => 0x0C,
                b'0'..=b'7' => {
                    let mut oct = (bytes[i + 1] - b'0') as u8;
                    let mut consumed = 1;
                    if i + 2 < bytes.len() && (b'0'..=b'7').contains(&bytes[i + 2]) {
                        oct = oct * 8 + (bytes[i + 2] - b'0');
                        consumed += 1;
                        if i + 3 < bytes.len() && (b'0'..=b'7').contains(&bytes[i + 3]) {
                            oct = oct * 8 + (bytes[i + 3] - b'0');
                            consumed += 1;
                        }
                    }
                    i += consumed + 1;
                    mask[oct as usize] = true;
                    continue;
                }
                c => c,
            };
            mask[escaped as usize] = true;
            i += 2;
        } else {
            mask[bytes[i] as usize] = true;
            i += 1;
        }
    }
    mask
}

php_function! {
    native_addcslashes(str_val: Value, charlist_val: Value) |ctx| {
        let s = str_val.and_then(|v| v.deref().as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() }).unwrap_or_default();
        let charlist = charlist_val.and_then(|v| v.deref().as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() }).unwrap_or_default();
        let mask = build_char_mask(&charlist);

        let mut out = Vec::with_capacity(s.len() + 16);
        for &b in s.as_bytes() {
            if mask[b as usize] {
                match b {
                    b'\n' => { out.extend_from_slice(b"\\n"); }
                    b'\r' => { out.extend_from_slice(b"\\r"); }
                    b'\t' => { out.extend_from_slice(b"\\t"); }
                    0x0B => { out.extend_from_slice(b"\\v"); }
                    0x0C => { out.extend_from_slice(b"\\f"); }
                    0..=31 | 127..=255 => {
                        let oct = format!("\\{:03o}", b);
                        out.extend_from_slice(oct.as_bytes());
                    }
                    _ => {
                        out.push(b'\\');
                        out.push(b);
                    }
                }
            } else {
                out.push(b);
            }
        }
        let res_str = String::from_utf8_lossy(&out).to_string();
        let ptr = ctx.get_arena().alloc_and_track(res_str);
        Ok(Value::new_string_ptr(ptr as *mut ()))
    }
}

php_function! {
    native_stripcslashes(str_val: Value) |ctx| {
        let s = str_val.and_then(|v| v.deref().as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() }).unwrap_or_default();
        let mut out = Vec::with_capacity(s.len());
        let bytes = s.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'\\' && i + 1 < bytes.len() {
                match bytes[i + 1] {
                    b'n' => { out.push(b'\n'); i += 2; }
                    b'r' => { out.push(b'\r'); i += 2; }
                    b't' => { out.push(b'\t'); i += 2; }
                    b'v' => { out.push(0x0B); i += 2; }
                    b'f' => { out.push(0x0C); i += 2; }
                    b'a' => { out.push(0x07); i += 2; }
                    b'b' => { out.push(0x08); i += 2; }
                    b'0'..=b'7' => {
                        let mut oct = (bytes[i + 1] - b'0') as u8;
                        let mut consumed = 1;
                        if i + 2 < bytes.len() && (b'0'..=b'7').contains(&bytes[i + 2]) {
                            oct = oct.wrapping_mul(8).wrapping_add(bytes[i + 2] - b'0');
                            consumed += 1;
                            if i + 3 < bytes.len() && (b'0'..=b'7').contains(&bytes[i + 3]) {
                                oct = oct.wrapping_mul(8).wrapping_add(bytes[i + 3] - b'0');
                                consumed += 1;
                            }
                        }
                        out.push(oct);
                        i += consumed + 1;
                    }
                    b'x' if i + 2 < bytes.len() => {
                        let mut hex_val = 0u8;
                        let mut consumed = 0;
                        for j in 0..2 {
                            if i + 2 + j < bytes.len() {
                                let c = bytes[i + 2 + j];
                                if let Some(digit) = (c as char).to_digit(16) {
                                    hex_val = hex_val.wrapping_mul(16).wrapping_add(digit as u8);
                                    consumed += 1;
                                } else {
                                    break;
                                }
                            }
                        }
                        if consumed > 0 {
                            out.push(hex_val);
                            i += 2 + consumed;
                        } else {
                            out.push(bytes[i + 1]);
                            i += 2;
                        }
                    }
                    other => {
                        out.push(other);
                        i += 2;
                    }
                }
            } else {
                out.push(bytes[i]);
                i += 1;
            }
        }
        let res_str = String::from_utf8_lossy(&out).to_string();
        let ptr = ctx.get_arena().alloc_and_track(res_str);
        Ok(Value::new_string_ptr(ptr as *mut ()))
    }
}

php_function! {
    native_setlocale(_category: Value, ...locales) |ctx| {
        let loc = if let Some(first) = locales.first() {
            let fd = first.deref();
            if let Some(s_ptr) = fd.as_string_ptr() {
                let s = unsafe { &*(s_ptr as *const String) };
                if s == "0" || s.is_empty() {
                    "C".to_string()
                } else {
                    s.clone()
                }
            } else {
                "C".to_string()
            }
        } else {
            "C".to_string()
        };
        let ptr = ctx.get_arena().alloc_and_track(loc);
        Ok(Value::new_string_ptr(ptr as *mut ()))
    }
}

