use hyperion_core::memory::nan_box::Value;
use hyperion_core::php_function;

fn get_ctype_string(val: &Value) -> Option<String> {
    if let Some(i) = val.as_int() {
        if (0..=255).contains(&i) {
            // Treat as ASCII character
            return Some((i as u8 as char).to_string());
        }
        if (-128..0).contains(&i) {
            // PHP quirk: negative integers between -128 and -1 are mapped to 256 + i
            return Some(((256 + i) as u8 as char).to_string());
        }
        return None;
    }
    
    if val.is_string()
        && let Some(ptr) = val.as_string_ptr() {
            let s = unsafe { &*(ptr as *const String) };
            if s.is_empty() {
                return None; // Empty string is false in ctype
            }
            return Some(s.clone());
        }
    
    None
}

php_function! {
    native_ctype_alnum(val: Value) {
        if let Some(v) = val {
            if let Some(s) = get_ctype_string(v) {
                Ok(Value::new_bool(s.chars().all(|c| c.is_ascii_alphanumeric())))
            } else {
                Ok(Value::new_bool(false))
            }
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_ctype_alpha(val: Value) {
        if let Some(v) = val {
            if let Some(s) = get_ctype_string(v) {
                Ok(Value::new_bool(s.chars().all(|c| c.is_ascii_alphabetic())))
            } else {
                Ok(Value::new_bool(false))
            }
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_ctype_cntrl(val: Value) {
        if let Some(v) = val {
            if let Some(s) = get_ctype_string(v) {
                Ok(Value::new_bool(s.chars().all(|c| c.is_ascii_control())))
            } else {
                Ok(Value::new_bool(false))
            }
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_ctype_digit(val: Value) {
        if let Some(v) = val {
            if let Some(s) = get_ctype_string(v) {
                Ok(Value::new_bool(s.chars().all(|c| c.is_ascii_digit())))
            } else {
                Ok(Value::new_bool(false))
            }
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_ctype_graph(val: Value) {
        if let Some(v) = val {
            if let Some(s) = get_ctype_string(v) {
                Ok(Value::new_bool(s.chars().all(|c| c.is_ascii_graphic())))
            } else {
                Ok(Value::new_bool(false))
            }
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_ctype_lower(val: Value) {
        if let Some(v) = val {
            if let Some(s) = get_ctype_string(v) {
                Ok(Value::new_bool(s.chars().all(|c| c.is_ascii_lowercase())))
            } else {
                Ok(Value::new_bool(false))
            }
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_ctype_print(val: Value) {
        if let Some(v) = val {
            if let Some(s) = get_ctype_string(v) {
                Ok(Value::new_bool(s.chars().all(|c| c.is_ascii_graphic() || c == ' ')))
            } else {
                Ok(Value::new_bool(false))
            }
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_ctype_punct(val: Value) {
        if let Some(v) = val {
            if let Some(s) = get_ctype_string(v) {
                Ok(Value::new_bool(s.chars().all(|c| c.is_ascii_punctuation())))
            } else {
                Ok(Value::new_bool(false))
            }
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_ctype_space(val: Value) {
        if let Some(v) = val {
            if let Some(s) = get_ctype_string(v) {
                Ok(Value::new_bool(s.chars().all(|c| c.is_ascii_whitespace())))
            } else {
                Ok(Value::new_bool(false))
            }
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_ctype_upper(val: Value) {
        if let Some(v) = val {
            if let Some(s) = get_ctype_string(v) {
                Ok(Value::new_bool(s.chars().all(|c| c.is_ascii_uppercase())))
            } else {
                Ok(Value::new_bool(false))
            }
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_ctype_xdigit(val: Value) {
        if let Some(v) = val {
            if let Some(s) = get_ctype_string(v) {
                Ok(Value::new_bool(s.chars().all(|c| c.is_ascii_hexdigit())))
            } else {
                Ok(Value::new_bool(false))
            }
        } else {
            Ok(Value::new_bool(false))
        }
    }
}
