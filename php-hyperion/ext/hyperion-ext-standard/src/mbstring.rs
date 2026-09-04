use hyperion_core::php_function;
use hyperion_core::memory::nan_box::Value;

php_function! {
    native_mb_strlen(string: Value, encoding: Value) {
        if let Some(val) = string {
            if let Some(ptr) = val.deref().as_string_ptr() {
                let s = unsafe { &*(ptr as *const String) };
                let enc_str = encoding.and_then(|e| e.deref().as_string_ptr()).map(|p| unsafe { (*(p as *const String)).to_ascii_lowercase() });
                if let Some(ref enc) = enc_str {
                    if enc == "8bit" || enc == "binary" || enc == "iso-8859-1" || enc == "latin1" || enc == "ascii" {
                        return Ok(Value::new_int(s.len() as i32));
                    }
                }
                let len = s.chars().count();
                Ok(Value::new_int(len as i32))
            } else {
                Ok(Value::new_int(0))
            }
        } else {
            Err("mb_strlen() expects at least 1 parameter".to_string())
        }
    }
}

php_function! {
    native_mb_substr(string: Value, start: Value, length: Value, encoding: Value) {
        if let (Some(val), Some(st)) = (string, start) {
            if let Some(ptr) = val.deref().as_string_ptr() {
                let s = unsafe { &*(ptr as *const String) };
                let start_idx = st.as_int().unwrap_or(0);
                let len_opt = length.and_then(|v| v.as_int());
                
                let enc_str = encoding.and_then(|e| e.deref().as_string_ptr()).map(|p| unsafe { (*(p as *const String)).to_ascii_lowercase() });
                let is_8bit = if let Some(ref enc) = enc_str {
                    enc == "8bit" || enc == "binary" || enc == "iso-8859-1" || enc == "latin1" || enc == "ascii"
                } else {
                    false
                };

                if is_8bit {
                    let bytes = s.as_bytes();
                    let byte_count = bytes.len() as i32;
                    let actual_start = if start_idx < 0 {
                        std::cmp::max(0, byte_count + start_idx)
                    } else {
                        std::cmp::min(start_idx, byte_count)
                    } as usize;

                    let actual_len = if let Some(l) = len_opt {
                        if l < 0 {
                            std::cmp::max(0, byte_count - (actual_start as i32) + l)
                        } else {
                            l
                        }
                    } else {
                        byte_count - (actual_start as i32)
                    } as usize;

                    let sub_bytes = &bytes[actual_start..std::cmp::min(actual_start + actual_len, bytes.len())];
                    let sub = unsafe { String::from_utf8_unchecked(sub_bytes.to_vec()) };
                    let boxed = crate::into_raw(Box::new(sub));
                    return Ok(Value::new_string_ptr(boxed as *mut ()));
                }

                let chars_count = s.chars().count() as i32;
                let actual_start = if start_idx < 0 {
                    std::cmp::max(0, chars_count + start_idx)
                } else {
                    std::cmp::min(start_idx, chars_count)
                };

                let actual_len = if let Some(l) = len_opt {
                    if l < 0 {
                        std::cmp::max(0, chars_count - actual_start + l)
                    } else {
                        l
                    }
                } else {
                    chars_count - actual_start
                };

                let sub: String = s.chars().skip(actual_start as usize).take(actual_len as usize).collect();
                let boxed = crate::into_raw(Box::new(sub));
                Ok(Value::new_string_ptr(boxed as *mut ()))
            } else {
                let boxed = crate::into_raw(Box::new(String::new()));
                Ok(Value::new_string_ptr(boxed as *mut ()))
            }
        } else {
            Err("mb_substr() expects at least 2 parameters".to_string())
        }
    }
}

php_function! {
    native_mb_strpos(haystack: String, needle: String, offset: Value) {
        if let (Some(h), Some(n)) = (haystack, needle) {
            let off = offset.and_then(|v| v.as_int()).unwrap_or(0);
            let h_chars: Vec<char> = h.chars().collect();
            let n_chars: Vec<char> = n.chars().collect();
            
            if n_chars.is_empty() {
                return Ok(Value::new_int(0));
            }
            if h_chars.is_empty() || off < 0 || off as usize >= h_chars.len() {
                return Ok(Value::new_bool(false));
            }
            
            for i in (off as usize)..=(h_chars.len().saturating_sub(n_chars.len())) {
                if h_chars[i..i+n_chars.len()] == n_chars[..] {
                    return Ok(Value::new_int(i as i32));
                }
            }
            Ok(Value::new_bool(false))
        } else {
            Err("mb_strpos() expects at least 2 parameters".to_string())
        }
    }
}

php_function! {
    native_mb_strtolower(string: String, _encoding: Value) {
        if let Some(s) = string {
            let lower = s.to_lowercase();
            let boxed = crate::into_raw(Box::new(lower));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        } else {
            Err("mb_strtolower() expects at least 1 parameter".to_string())
        }
    }
}

php_function! {
    native_mb_strtoupper(string: String, _encoding: Value) {
        if let Some(s) = string {
            let upper = s.to_uppercase();
            let boxed = crate::into_raw(Box::new(upper));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        } else {
            Err("mb_strtoupper() expects at least 1 parameter".to_string())
        }
    }
}

php_function! {
    native_mb_convert_case(string: String, mode: Value, _encoding: Value) {
        if let (Some(s), Some(m)) = (string, mode) {
            let mode_val = m.as_int().unwrap_or(0);
            let result = match mode_val {
                1 => s.to_lowercase(), // MB_CASE_LOWER
                2 => { // MB_CASE_TITLE
                    let mut res = String::new();
                    let mut cap_next = true;
                    for c in s.chars() {
                        if c.is_alphanumeric() {
                            if cap_next {
                                for uc in c.to_uppercase() {
                                    res.push(uc);
                                }
                                cap_next = false;
                            } else {
                                for lc in c.to_lowercase() {
                                    res.push(lc);
                                }
                            }
                        } else {
                            res.push(c);
                            cap_next = true;
                        }
                    }
                    res
                }
                _ => s.to_uppercase(), // MB_CASE_UPPER
            };
            let boxed = crate::into_raw(Box::new(result));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        } else {
            Err("mb_convert_case() expects at least 2 parameters".to_string())
        }
    }
}

php_function! {
    native_mb_internal_encoding(encoding: Value) {
        if let Some(enc_val) = encoding {
            if enc_val.is_null() {
                let boxed = crate::into_raw(Box::new("UTF-8".to_string()));
                Ok(Value::new_string_ptr(boxed as *mut ()))
            } else {
                Ok(Value::new_bool(true))
            }
        } else {
            let boxed = crate::into_raw(Box::new("UTF-8".to_string()));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        }
    }
}

php_function! {
    native_mb_detect_encoding(string: String, _encodings: Value, _strict: Value) {
        if let Some(_) = string {
            let boxed = crate::into_raw(Box::new("UTF-8".to_string()));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        } else {
            Err("mb_detect_encoding() expects at least 1 parameter".to_string())
        }
    }
}

php_function! {
    native_mb_check_encoding(_value: Value, _encoding: Value) {
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_mb_ord(string: String, _encoding: Value) {
        if let Some(s) = string {
            if let Some(c) = s.chars().next() {
                Ok(Value::new_int(c as u32 as i32))
            } else {
                Ok(Value::new_bool(false))
            }
        } else {
            Err("mb_ord() expects at least 1 parameter".to_string())
        }
    }
}

php_function! {
    native_mb_chr(code: Value, _encoding: Value) {
        if let Some(c_val) = code {
            let codepoint = c_val.as_int().unwrap_or(0) as u32;
            if let Some(c) = char::from_u32(codepoint) {
                let mut s = String::new();
                s.push(c);
                let boxed = crate::into_raw(Box::new(s));
                Ok(Value::new_string_ptr(boxed as *mut ()))
            } else {
                Ok(Value::new_bool(false))
            }
        } else {
            Err("mb_chr() expects at least 1 parameter".to_string())
        }
    }
}

php_function! {
    native_mb_strimwidth(string: Value, start: Value, width: Value, trim_marker: Value, _encoding: Value) {
        let Some(s_val) = string else {
            return Err("mb_strimwidth() expects at least 3 parameters".to_string());
        };
        let s = if let Some(sp) = s_val.as_string_ptr() {
            unsafe { &*(sp as *const String) }.clone()
        } else {
            String::new()
        };
        let start_idx = start.and_then(|v| v.as_int()).unwrap_or(0).max(0) as usize;
        let width_val = width.and_then(|v| v.as_int()).unwrap_or(s.chars().count() as i32).max(0) as usize;
        let marker = trim_marker.and_then(|v| v.as_string_ptr()).map(|sp| unsafe { &*(sp as *const String) }.clone()).unwrap_or_default();
        let marker_len = marker.chars().count();

        let chars: Vec<char> = s.chars().collect();
        if start_idx >= chars.len() {
            let boxed = crate::into_raw(Box::new(String::new()));
            return Ok(Value::new_string_ptr(boxed as *mut ()));
        }

        let slice = &chars[start_idx..];
        let result = if slice.len() <= width_val {
            slice.iter().collect::<String>()
        } else if width_val > marker_len {
            let take_len = width_val - marker_len;
            let mut res = slice.iter().take(take_len).collect::<String>();
            res.push_str(&marker);
            res
        } else {
            slice.iter().take(width_val).collect::<String>()
        };

        let boxed = crate::into_raw(Box::new(result));
        Ok(Value::new_string_ptr(boxed as *mut ()))
    }
}

php_function! {
    native_mb_split(pattern: Value, string: Value, limit: Value) |ctx| {
        let (Some(pat_val), Some(str_val)) = (pattern, string) else {
            return Err("mb_split() expects at least 2 parameters".to_string());
        };
        let Some(pat_ptr) = pat_val.deref().as_string_ptr() else { return Ok(Value::new_bool(false)); };
        let Some(str_ptr) = str_val.deref().as_string_ptr() else { return Ok(Value::new_bool(false)); };
        let pat = unsafe { &*(pat_ptr as *const String) };
        let s = unsafe { &*(str_ptr as *const String) };
        let lim = limit.and_then(|l| l.as_int()).unwrap_or(-1);

        let re = match regex::Regex::new(pat) {
            Ok(r) => r,
            Err(_) => return Ok(Value::new_bool(false)),
        };

        let mut arr = hyperion_core::types::array::PhpArray::new();
        if lim == 1 {
            let s_ptr = ctx.get_arena().alloc_and_track(s.clone());
            arr.insert_int(0, Value::new_string_ptr(s_ptr as *mut ()));
        } else {
            let splits: Vec<&str> = if lim > 1 {
                re.splitn(s, lim as usize).collect()
            } else {
                re.split(s).collect()
            };
            for (i, part) in splits.into_iter().enumerate() {
                let part_ptr = ctx.get_arena().alloc_and_track(part.to_string());
                arr.insert_int(i as i64, Value::new_string_ptr(part_ptr as *mut ()));
            }
        }
        let arr_ptr = ctx.get_arena().alloc_and_track(arr);
        Ok(Value::new_array_ptr(arr_ptr as *mut ()))
    }
}


