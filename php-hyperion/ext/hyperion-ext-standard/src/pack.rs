use hyperion_core::memory::nan_box::Value;
use hyperion_core::types::array::PhpArray;
use hyperion_core::types::function::NativeContext;
use hyperion_core::php_function;

php_function! {
    native_pack(format: Value, ...args) {
        let Some(fmt_ptr) = format.and_then(|v| v.deref().as_string_ptr()) else {
            return Err("pack() expects parameter 1 to be string".to_string());
        };
        let fmt = unsafe { &*(fmt_ptr as *const String) };
        let mut out: Vec<u8> = Vec::new();
        let mut arg_idx = 0;

        let chars: Vec<char> = fmt.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let code = chars[i];
            i += 1;
            // Read optional quantifier (* or digits)
            let mut is_star = false;
            let mut count: usize = 1;
            if i < chars.len() && chars[i] == '*' {
                is_star = true;
                i += 1;
            } else {
                let mut num_str = String::new();
                while i < chars.len() && chars[i].is_ascii_digit() {
                    num_str.push(chars[i]);
                    i += 1;
                }
                if !num_str.is_empty() {
                    count = num_str.parse::<usize>().unwrap_or(1);
                }
            }

            match code {
                'C' | 'c' => {
                    let total = if is_star { args.len().saturating_sub(arg_idx) } else { count };
                    for _ in 0..total {
                        if arg_idx < args.len() {
                            let val = args[arg_idx].as_int().unwrap_or(0) as u8;
                            out.push(val);
                            arg_idx += 1;
                        }
                    }
                }
                'n' => { // 16-bit big-endian
                    let total = if is_star { args.len().saturating_sub(arg_idx) } else { count };
                    for _ in 0..total {
                        if arg_idx < args.len() {
                            let val = args[arg_idx].as_int().unwrap_or(0) as u16;
                            out.extend_from_slice(&val.to_be_bytes());
                            arg_idx += 1;
                        }
                    }
                }
                'v' => { // 16-bit little-endian
                    let total = if is_star { args.len().saturating_sub(arg_idx) } else { count };
                    for _ in 0..total {
                        if arg_idx < args.len() {
                            let val = args[arg_idx].as_int().unwrap_or(0) as u16;
                            out.extend_from_slice(&val.to_le_bytes());
                            arg_idx += 1;
                        }
                    }
                }
                'N' => { // 32-bit big-endian
                    let total = if is_star { args.len().saturating_sub(arg_idx) } else { count };
                    for _ in 0..total {
                        if arg_idx < args.len() {
                            let val = args[arg_idx].as_int().unwrap_or(0) as u32;
                            out.extend_from_slice(&val.to_be_bytes());
                            arg_idx += 1;
                        }
                    }
                }
                'V' => { // 32-bit little-endian
                    let total = if is_star { args.len().saturating_sub(arg_idx) } else { count };
                    for _ in 0..total {
                        if arg_idx < args.len() {
                            let val = args[arg_idx].as_int().unwrap_or(0) as u32;
                            out.extend_from_slice(&val.to_le_bytes());
                            arg_idx += 1;
                        }
                    }
                }
                'J' => { // 64-bit big-endian
                    let total = if is_star { args.len().saturating_sub(arg_idx) } else { count };
                    for _ in 0..total {
                        if arg_idx < args.len() {
                            let val = args[arg_idx].as_int().unwrap_or(0) as u64;
                            out.extend_from_slice(&val.to_be_bytes());
                            arg_idx += 1;
                        }
                    }
                }
                'P' => { // 64-bit little-endian
                    let total = if is_star { args.len().saturating_sub(arg_idx) } else { count };
                    for _ in 0..total {
                        if arg_idx < args.len() {
                            let val = args[arg_idx].as_int().unwrap_or(0) as u64;
                            out.extend_from_slice(&val.to_le_bytes());
                            arg_idx += 1;
                        }
                    }
                }
                'H' | 'h' => { // Hex string
                    if arg_idx < args.len() {
                        let hex_str = if let Some(s_ptr) = args[arg_idx].deref().as_string_ptr() {
                            unsafe { &*(s_ptr as *const String) }.clone()
                        } else {
                            String::new()
                        };
                        let bytes = hex_str.as_bytes();
                        let max_len = if is_star { bytes.len() } else { count };
                        let mut j = 0;
                        while j < bytes.len() && j < max_len {
                            let high = hex_val(bytes[j]);
                            let low = if j + 1 < bytes.len() && j + 1 < max_len { hex_val(bytes[j + 1]) } else { 0 };
                            let byte = if code == 'H' { (high << 4) | low } else { (low << 4) | high };
                            out.push(byte);
                            j += 2;
                        }
                        arg_idx += 1;
                    }
                }
                'a' | 'A' => { // Null/space padded string
                    if arg_idx < args.len() {
                        let s = if let Some(s_ptr) = args[arg_idx].deref().as_string_ptr() {
                            unsafe { &*(s_ptr as *const String) }.clone()
                        } else {
                            String::new()
                        };
                        let bytes = s.as_bytes();
                        let target_len = if is_star { bytes.len() } else { count };
                        for k in 0..target_len {
                            if k < bytes.len() {
                                out.push(bytes[k]);
                            } else {
                                out.push(if code == 'a' { 0 } else { b' ' });
                            }
                        }
                        arg_idx += 1;
                    }
                }
                _ => {}
            }
        }

        let out_str = unsafe { String::from_utf8_unchecked(out) };
        let ptr = crate::into_raw(Box::new(out_str));
        Ok(Value::new_string_ptr(ptr as *mut ()))
    }
}

php_function! {
    native_unpack(format: Value, string: Value, ...rest) |ctx| {
        let Some(fmt_ptr) = format.and_then(|v| v.deref().as_string_ptr()) else {
            return Err("unpack() expects parameter 1 to be string".to_string());
        };
        let Some(str_ptr) = string.and_then(|v| v.deref().as_string_ptr()) else {
            return Err("unpack() expects parameter 2 to be string".to_string());
        };
        let fmt = unsafe { &*(fmt_ptr as *const String) };
        let raw_bytes = unsafe { &*(str_ptr as *const String) }.as_bytes();

        let start_offset = if !rest.is_empty() {
            rest[0].as_int().unwrap_or(0).max(0) as usize
        } else {
            0
        };
        let mut byte_idx = start_offset;
        let mut result_arr = PhpArray::new();
        let mut default_idx: i64 = 1;

        // Formats can be separated by '/' e.g. "n1time/n1seq"
        let parts: Vec<&str> = fmt.split('/').collect();
        for part in parts {
            let part = part.trim();
            if part.is_empty() { continue; }

            let chars: Vec<char> = part.chars().collect();
            if chars.is_empty() { continue; }
            let code = chars[0];
            let mut i = 1;

            let mut is_star = false;
            let mut count: usize = 1;
            if i < chars.len() && chars[i] == '*' {
                is_star = true;
                i += 1;
            } else {
                let mut num_str = String::new();
                while i < chars.len() && chars[i].is_ascii_digit() {
                    num_str.push(chars[i]);
                    i += 1;
                }
                if !num_str.is_empty() {
                    count = num_str.parse::<usize>().unwrap_or(1);
                }
            }

            let field_name = if i < chars.len() {
                chars[i..].iter().collect::<String>()
            } else {
                String::new()
            };

            match code {
                'C' | 'c' => {
                    let total = if is_star { raw_bytes.len().saturating_sub(byte_idx) } else { count };
                    for k in 0..total {
                        if byte_idx < raw_bytes.len() {
                            let val = if code == 'c' { raw_bytes[byte_idx] as i8 as i32 } else { raw_bytes[byte_idx] as i32 };
                            byte_idx += 1;
                            insert_result(ctx, &mut result_arr, &field_name, k, is_star, &mut default_idx, Value::new_int(val));
                        }
                    }
                }
                'n' => { // 16-bit big endian
                    let total = if is_star { (raw_bytes.len().saturating_sub(byte_idx)) / 2 } else { count };
                    for k in 0..total {
                        if byte_idx + 2 <= raw_bytes.len() {
                            let val = u16::from_be_bytes([raw_bytes[byte_idx], raw_bytes[byte_idx + 1]]) as i32;
                            byte_idx += 2;
                            insert_result(ctx, &mut result_arr, &field_name, k, is_star, &mut default_idx, Value::new_int(val));
                        }
                    }
                }
                'v' => { // 16-bit little endian
                    let total = if is_star { (raw_bytes.len().saturating_sub(byte_idx)) / 2 } else { count };
                    for k in 0..total {
                        if byte_idx + 2 <= raw_bytes.len() {
                            let val = u16::from_le_bytes([raw_bytes[byte_idx], raw_bytes[byte_idx + 1]]) as i32;
                            byte_idx += 2;
                            insert_result(ctx, &mut result_arr, &field_name, k, is_star, &mut default_idx, Value::new_int(val));
                        }
                    }
                }
                'N' => { // 32-bit big endian
                    let total = if is_star { (raw_bytes.len().saturating_sub(byte_idx)) / 4 } else { count };
                    for k in 0..total {
                        if byte_idx + 4 <= raw_bytes.len() {
                            let val = u32::from_be_bytes([
                                raw_bytes[byte_idx], raw_bytes[byte_idx + 1],
                                raw_bytes[byte_idx + 2], raw_bytes[byte_idx + 3]
                            ]) as i32;
                            byte_idx += 4;
                            insert_result(ctx, &mut result_arr, &field_name, k, is_star, &mut default_idx, Value::new_int(val));
                        }
                    }
                }
                'V' => { // 32-bit little endian
                    let total = if is_star { (raw_bytes.len().saturating_sub(byte_idx)) / 4 } else { count };
                    for k in 0..total {
                        if byte_idx + 4 <= raw_bytes.len() {
                            let val = u32::from_le_bytes([
                                raw_bytes[byte_idx], raw_bytes[byte_idx + 1],
                                raw_bytes[byte_idx + 2], raw_bytes[byte_idx + 3]
                            ]) as i32;
                            byte_idx += 4;
                            insert_result(ctx, &mut result_arr, &field_name, k, is_star, &mut default_idx, Value::new_int(val));
                        }
                    }
                }
                'H' | 'h' => { // Hex string
                    let total_bytes = if is_star { raw_bytes.len().saturating_sub(byte_idx) } else { (count + 1) / 2 };
                    let slice = &raw_bytes[byte_idx..byte_idx + total_bytes.min(raw_bytes.len().saturating_sub(byte_idx))];
                    byte_idx += slice.len();
                    let hex_s: String = slice.iter().map(|b| format!("{:02x}", b)).collect();
                    let hex_ptr = crate::into_raw(Box::new(hex_s));
                    let val = Value::new_string_ptr(hex_ptr as *mut ());
                    insert_result(ctx, &mut result_arr, &field_name, 0, false, &mut default_idx, val);
                }
                'a' | 'A' => { // String
                    let total = if is_star { raw_bytes.len().saturating_sub(byte_idx) } else { count };
                    let slice = &raw_bytes[byte_idx..byte_idx + total.min(raw_bytes.len().saturating_sub(byte_idx))];
                    byte_idx += slice.len();
                    let s = String::from_utf8_lossy(slice).to_string();
                    let s_ptr = crate::into_raw(Box::new(s));
                    let val = Value::new_string_ptr(s_ptr as *mut ());
                    insert_result(ctx, &mut result_arr, &field_name, 0, false, &mut default_idx, val);
                }
                _ => {}
            }
        }

        let arr_ptr = crate::into_raw(Box::new(result_arr));
        Ok(Value::new_array_ptr(arr_ptr as *mut ()))
    }
}

fn insert_result(ctx: &mut dyn NativeContext, arr: &mut PhpArray, field_name: &str, idx: usize, is_star: bool, default_idx: &mut i64, val: Value) {
    if !field_name.is_empty() {
        let key = if is_star || idx > 0 { format!("{}{}", field_name, idx + 1) } else { field_name.to_string() };
        let sid = ctx.intern_string(&key);
        arr.insert_string_id(sid, val);
    } else {
        arr.insert_int(*default_idx, val);
        *default_idx += 1;
    }
}

fn hex_val(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => 0,
    }
}
