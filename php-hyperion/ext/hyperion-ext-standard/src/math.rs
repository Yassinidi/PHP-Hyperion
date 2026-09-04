use hyperion_core::php_function;
use hyperion_core::memory::nan_box::Value;

// ============================================================
// 🔢 PHP Math Module — ~25 native functions
// ============================================================

php_function! {
    native_abs(number: Value) {
        if let Some(v) = number {
            if let Some(i) = v.as_int() {
                Ok(Value::new_int(i.abs()))
            } else if let Some(f) = v.as_float() {
                Ok(Value::new_float(f.abs()))
            } else {
                Ok(Value::new_int(0))
            }
        } else {
            Err("abs() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_ceil(number: Value) {
        if let Some(v) = number {
            let f = if let Some(fl) = v.as_float() { fl }
                    else if let Some(i) = v.as_int() { i as f64 }
                    else { 0.0 };
            Ok(Value::new_float(f.ceil()))
        } else {
            Err("ceil() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_floor(number: Value) {
        if let Some(v) = number {
            let f = if let Some(fl) = v.as_float() { fl }
                    else if let Some(i) = v.as_int() { i as f64 }
                    else { 0.0 };
            Ok(Value::new_float(f.floor()))
        } else {
            Err("floor() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_round(number: Value, precision: Value) {
        if let Some(v) = number {
            let f = if let Some(fl) = v.as_float() { fl }
                    else if let Some(i) = v.as_int() { i as f64 }
                    else { 0.0 };
            let prec = precision.and_then(|p| p.as_int()).unwrap_or(0);
            let factor = 10f64.powi(prec);
            Ok(Value::new_float((f * factor).round() / factor))
        } else {
            Err("round() expects at least 1 parameter".to_string())
        }
    }
}

pub fn native_max(args: &[Value], _ctx: &mut dyn hyperion_core::types::function::NativeContext) -> Result<Value, String> {
    if args.is_empty() {
        return Err("max(): Array must contain at least one element".to_string());
    }
    if args.len() == 1 {
        if let Some(arr_ptr) = args[0].as_array_ptr() {
            let php_arr = unsafe { &*(arr_ptr as *const hyperion_core::types::array::PhpArray) };
            if php_arr.elements.is_empty() {
                return Err("max(): Array must contain at least one element".to_string());
            }
            let mut max_val = php_arr.elements.values().next().copied().unwrap();
            let mut max_num = if let Some(f) = max_val.as_float() { f } else if let Some(i) = max_val.as_int() { i as f64 } else { 0.0 };
            for v in php_arr.elements.values().skip(1) {
                let num = if let Some(f) = v.as_float() { f } else if let Some(i) = v.as_int() { i as f64 } else { 0.0 };
                if num > max_num {
                    max_num = num;
                    max_val = *v;
                }
            }
            return Ok(max_val);
        }
        return Ok(args[0]);
    }
    
    let mut max_val = args[0];
    let mut max_num = if let Some(f) = max_val.as_float() { f } else if let Some(i) = max_val.as_int() { i as f64 } else { 0.0 };
    for v in &args[1..] {
        let num = if let Some(f) = v.as_float() { f } else if let Some(i) = v.as_int() { i as f64 } else { 0.0 };
        if num > max_num {
            max_num = num;
            max_val = *v;
        }
    }
    Ok(max_val)
}

pub fn native_min(args: &[Value], _ctx: &mut dyn hyperion_core::types::function::NativeContext) -> Result<Value, String> {
    if args.is_empty() {
        return Err("min(): Array must contain at least one element".to_string());
    }
    if args.len() == 1 {
        if let Some(arr_ptr) = args[0].as_array_ptr() {
            let php_arr = unsafe { &*(arr_ptr as *const hyperion_core::types::array::PhpArray) };
            if php_arr.elements.is_empty() {
                return Err("min(): Array must contain at least one element".to_string());
            }
            let mut min_val = php_arr.elements.values().next().copied().unwrap();
            let mut min_num = if let Some(f) = min_val.as_float() { f } else if let Some(i) = min_val.as_int() { i as f64 } else { 0.0 };
            for v in php_arr.elements.values().skip(1) {
                let num = if let Some(f) = v.as_float() { f } else if let Some(i) = v.as_int() { i as f64 } else { 0.0 };
                if num < min_num {
                    min_num = num;
                    min_val = *v;
                }
            }
            return Ok(min_val);
        }
        return Ok(args[0]);
    }
    
    let mut min_val = args[0];
    let mut min_num = if let Some(f) = min_val.as_float() { f } else if let Some(i) = min_val.as_int() { i as f64 } else { 0.0 };
    for v in &args[1..] {
        let num = if let Some(f) = v.as_float() { f } else if let Some(i) = v.as_int() { i as f64 } else { 0.0 };
        if num < min_num {
            min_num = num;
            min_val = *v;
        }
    }
    Ok(min_val)
}

php_function! {
    native_fmod(x: Value, y: Value) {
        if let (Some(a), Some(b)) = (x, y) {
            let fa = if let Some(f) = a.as_float() { f } else if let Some(i) = a.as_int() { i as f64 } else { 0.0 };
            let fb = if let Some(f) = b.as_float() { f } else if let Some(i) = b.as_int() { i as f64 } else { 1.0 };
            Ok(Value::new_float(fa % fb))
        } else {
            Err("fmod() expects exactly 2 parameters".to_string())
        }
    }
}

php_function! {
    native_intdiv(a: Value, b: Value) {
        if let (Some(a), Some(b)) = (a, b) {
            let ai = a.as_int().unwrap_or(0);
            let bi = b.as_int().unwrap_or(1);
            if bi == 0 { return Err("Division by zero".to_string()); }
            Ok(Value::new_int(ai / bi))
        } else {
            Err("intdiv() expects exactly 2 parameters".to_string())
        }
    }
}

// --- Advanced ---

php_function! {
    native_pow(base: Value, exp: Value) {
        if let (Some(b), Some(e)) = (base, exp) {
            let fb = if let Some(f) = b.as_float() { f } else if let Some(i) = b.as_int() { i as f64 } else { 0.0 };
            let fe = if let Some(f) = e.as_float() { f } else if let Some(i) = e.as_int() { i as f64 } else { 0.0 };
            Ok(Value::new_float(fb.powf(fe)))
        } else {
            Err("pow() expects exactly 2 parameters".to_string())
        }
    }
}

php_function! {
    native_sqrt(number: Value) {
        if let Some(v) = number {
            let f = if let Some(fl) = v.as_float() { fl } else if let Some(i) = v.as_int() { i as f64 } else { 0.0 };
            Ok(Value::new_float(f.sqrt()))
        } else {
            Err("sqrt() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_exp(number: Value) {
        if let Some(v) = number {
            let f = if let Some(fl) = v.as_float() { fl } else if let Some(i) = v.as_int() { i as f64 } else { 0.0 };
            Ok(Value::new_float(f.exp()))
        } else {
            Err("exp() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_log(number: Value) {
        if let Some(v) = number {
            let f = if let Some(fl) = v.as_float() { fl } else if let Some(i) = v.as_int() { i as f64 } else { 0.0 };
            Ok(Value::new_float(f.ln()))
        } else {
            Err("log() expects at least 1 parameter".to_string())
        }
    }
}

php_function! {
    native_log10(number: Value) {
        if let Some(v) = number {
            let f = if let Some(fl) = v.as_float() { fl } else if let Some(i) = v.as_int() { i as f64 } else { 0.0 };
            Ok(Value::new_float(f.log10()))
        } else {
            Err("log10() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_log2(number: Value) {
        if let Some(v) = number {
            let f = if let Some(fl) = v.as_float() { fl } else if let Some(i) = v.as_int() { i as f64 } else { 0.0 };
            Ok(Value::new_float(f.log2()))
        } else {
            Err("log2() expects exactly 1 parameter".to_string())
        }
    }
}

// --- Trigonometry ---

php_function! { native_sin(n: Value) Ok(Value::new_float(extract_float(n).sin()))  }
php_function! { native_cos(n: Value) Ok(Value::new_float(extract_float(n).cos()))  }
php_function! { native_tan(n: Value) Ok(Value::new_float(extract_float(n).tan()))  }
php_function! { native_asin(n: Value) Ok(Value::new_float(extract_float(n).asin()))  }
php_function! { native_acos(n: Value) Ok(Value::new_float(extract_float(n).acos()))  }
php_function! { native_atan(n: Value) Ok(Value::new_float(extract_float(n).atan()))  }

php_function! {
    native_atan2(y: Value, x: Value) {
        let fy = extract_float(y);
        let fx = extract_float(x);
        Ok(Value::new_float(fy.atan2(fx)))
    }
}

php_function! { native_pi() Ok(Value::new_float(std::f64::consts::PI))  }

php_function! {
    native_deg2rad(n: Value) Ok(Value::new_float(extract_float(n).to_radians())) 
}

php_function! {
    native_rad2deg(n: Value) Ok(Value::new_float(extract_float(n).to_degrees())) 
}

// --- Random ---

php_function! {
    native_rand(min_val: Value, max_val: Value) {
        use rand::Rng;
        let min = min_val.and_then(|v| v.as_int()).unwrap_or(0);
        let max = max_val.and_then(|v| v.as_int()).unwrap_or(i32::MAX);
        let val = rand::thread_rng().gen_range(min..=max);
        Ok(Value::new_int(val))
    }
}

php_function! {
    native_mt_rand(min_val: Value, max_val: Value) {
        use rand::Rng;
        let min = min_val.and_then(|v| v.as_int()).unwrap_or(0);
        let max = max_val.and_then(|v| v.as_int()).unwrap_or(i32::MAX);
        let val = rand::thread_rng().gen_range(min..=max);
        Ok(Value::new_int(val))
    }
}

php_function! {
    native_mt_srand(seed_val: Value, _mode: Value) {
        let _ = seed_val;
        Ok(Value::null())
    }
}

php_function! {
    native_srand(seed_val: Value, _mode: Value) {
        let _ = seed_val;
        Ok(Value::null())
    }
}

php_function! {
    native_mt_getrandmax() {
        Ok(Value::new_int(i32::MAX))
    }
}

php_function! {
    native_getrandmax() {
        Ok(Value::new_int(i32::MAX))
    }
}

php_function! {
    native_random_int(min_val: Value, max_val: Value) {
        use rand::Rng;
        if let (Some(mn), Some(mx)) = (min_val, max_val) {
            let min = mn.as_int().unwrap_or(0);
            let max = mx.as_int().unwrap_or(i32::MAX);
            let val = rand::thread_rng().gen_range(min..=max);
            Ok(Value::new_int(val))
        } else {
            Err("random_int() expects exactly 2 parameters".to_string())
        }
    }
}

// Helper: extract float from optional Value
fn extract_float(v: Option<&Value>) -> f64 {
    v.map(|val| {
        if let Some(f) = val.as_float() { f }
        else if let Some(i) = val.as_int() { i as f64 }
        else { 0.0 }
    }).unwrap_or(0.0)
}

php_function! {
    native_hexdec(hex_string: Value) {
        if let Some(v) = hex_string {
            let s_owned: String;
            let s = if let Some(p) = v.as_string_ptr() {
                unsafe { &*(p as *const String) }
            } else if let Some(i) = v.as_int() {
                s_owned = i.to_string();
                &s_owned
            } else {
                ""
            };
            let s = s.trim().trim_start_matches("0x").trim_start_matches("0X").trim_start_matches('#');
            let mut hex_digits = String::new();
            for c in s.chars() {
                if c.is_ascii_hexdigit() {
                    hex_digits.push(c);
                }
            }
            if hex_digits.is_empty() {
                return Ok(Value::new_int(0));
            }
            if let Ok(val) = i64::from_str_radix(&hex_digits, 16) {
                if val <= i32::MAX as i64 && val >= i32::MIN as i64 {
                    Ok(Value::new_int(val as i32))
                } else {
                    Ok(Value::new_float(val as f64))
                }
            } else if let Ok(val) = u64::from_str_radix(&hex_digits, 16) {
                Ok(Value::new_float(val as f64))
            } else {
                Ok(Value::new_int(0))
            }
        } else {
            Err("hexdec() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_dechex(num: Value) |ctx| {
        if let Some(v) = num {
            let n = if let Some(i) = v.as_int() { i as i64 }
            else if let Some(f) = v.as_float() { f as i64 }
            else { 0 };
            let s = format!("{:x}", n);
            let ptr = ctx.get_arena().alloc_and_track(s);
            Ok(Value::new_string_ptr(ptr as *mut ()))
        } else {
            Err("dechex() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_bindec(bin_string: Value) {
        if let Some(v) = bin_string {
            let s = if let Some(p) = v.as_string_ptr() {
                unsafe { (*(p as *const String)).clone() }
            } else if let Some(i) = v.as_int() {
                i.to_string()
            } else {
                String::new()
            };
            let mut bin_digits = String::new();
            for c in s.chars() {
                if c == '0' || c == '1' {
                    bin_digits.push(c);
                }
            }
            if bin_digits.is_empty() {
                return Ok(Value::new_int(0));
            }
            if let Ok(val) = i64::from_str_radix(&bin_digits, 2) {
                if val <= i32::MAX as i64 && val >= i32::MIN as i64 {
                    Ok(Value::new_int(val as i32))
                } else {
                    Ok(Value::new_float(val as f64))
                }
            } else {
                Ok(Value::new_int(0))
            }
        } else {
            Err("bindec() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_decbin(num: Value) |ctx| {
        if let Some(v) = num {
            let n = if let Some(i) = v.as_int() { i as i64 }
            else if let Some(f) = v.as_float() { f as i64 }
            else { 0 };
            let s = format!("{:b}", n);
            let ptr = ctx.get_arena().alloc_and_track(s);
            Ok(Value::new_string_ptr(ptr as *mut ()))
        } else {
            Err("decbin() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_octdec(oct_string: Value) {
        if let Some(v) = oct_string {
            let s = if let Some(p) = v.as_string_ptr() {
                unsafe { (*(p as *const String)).clone() }
            } else if let Some(i) = v.as_int() {
                i.to_string()
            } else {
                String::new()
            };
            let mut oct_digits = String::new();
            for c in s.chars() {
                if ('0'..='7').contains(&c) {
                    oct_digits.push(c);
                }
            }
            if oct_digits.is_empty() {
                return Ok(Value::new_int(0));
            }
            if let Ok(val) = i64::from_str_radix(&oct_digits, 8) {
                if val <= i32::MAX as i64 && val >= i32::MIN as i64 {
                    Ok(Value::new_int(val as i32))
                } else {
                    Ok(Value::new_float(val as f64))
                }
            } else {
                Ok(Value::new_int(0))
            }
        } else {
            Err("octdec() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_decoct(num: Value) |ctx| {
        if let Some(v) = num {
            let n = if let Some(i) = v.as_int() { i as i64 }
            else if let Some(f) = v.as_float() { f as i64 }
            else { 0 };
            let s = format!("{:o}", n);
            let ptr = ctx.get_arena().alloc_and_track(s);
            Ok(Value::new_string_ptr(ptr as *mut ()))
        } else {
            Err("decoct() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_base_convert(num: Value, from_base: Value, to_base: Value) |ctx| {
        let s = num.and_then(|v| v.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() }).unwrap_or_default();
        let from = from_base.and_then(|v| v.as_int()).unwrap_or(10) as u32;
        let to = to_base.and_then(|v| v.as_int()).unwrap_or(10) as u32;
        if from < 2 || from > 36 || to < 2 || to > 36 {
            return Err("Invalid base".to_string());
        }
        let parsed = u64::from_str_radix(&s.trim(), from).unwrap_or(0);
        let digits = b"0123456789abcdefghijklmnopqrstuvwxyz";
        let res = if parsed == 0 {
            "0".to_string()
        } else {
            let mut n = parsed;
            let mut buf = Vec::new();
            while n > 0 {
                buf.push(digits[(n % (to as u64)) as usize]);
                n /= to as u64;
            }
            buf.reverse();
            String::from_utf8(buf).unwrap_or_default()
        };
        let ptr = ctx.get_arena().alloc_and_track(res);
        Ok(Value::new_string_ptr(ptr as *mut ()))
    }
}

php_function! {
    native_is_nan(number: Value) {
        if let Some(v) = number {
            let v = v.deref();
            if let Some(f) = v.as_float() {
                Ok(Value::new_bool(f.is_nan()))
            } else {
                Ok(Value::new_bool(false))
            }
        } else {
            Err("is_nan() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_is_infinite(number: Value) {
        if let Some(v) = number {
            let v = v.deref();
            if let Some(f) = v.as_float() {
                Ok(Value::new_bool(f.is_infinite()))
            } else {
                Ok(Value::new_bool(false))
            }
        } else {
            Err("is_infinite() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_is_finite(number: Value) {
        if let Some(v) = number {
            let v = v.deref();
            if let Some(f) = v.as_float() {
                Ok(Value::new_bool(f.is_finite()))
            } else if v.as_int().is_some() {
                Ok(Value::new_bool(true))
            } else {
                Ok(Value::new_bool(false))
            }
        } else {
            Err("is_finite() expects exactly 1 parameter".to_string())
        }
    }
}
