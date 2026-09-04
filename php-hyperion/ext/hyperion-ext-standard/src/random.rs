use hyperion_core::memory::nan_box::Value;
use hyperion_core::php_function;
use hyperion_core::types::array::{ArrayKey, PhpArray};
use rand::{Rng, RngCore, rngs::OsRng, seq::SliceRandom};

php_function! {
    native_random_bytes(length: Value) {
        if let Some(length_val) = length {
            let len = match length_val.deref().as_int() {
                Some(l) if l > 0 => (l as usize).min(50_000_000),
                _ => return Err("random_bytes(): Length must be greater than 0".to_string()),
            };
            
            let mut bytes = vec![0u8; len];
            OsRng.fill_bytes(&mut bytes);
            
            let s = unsafe { String::from_utf8_unchecked(bytes) };
            let boxed = crate::into_raw(Box::new(s));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        } else {
            Err("random_bytes() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_random_int(min: Value, max: Value) {
        let min_i = min.and_then(|v| v.deref().as_int()).unwrap_or(0) as i64;
        let max_i = max.and_then(|v| v.deref().as_int()).unwrap_or(0) as i64;
        if min_i > max_i {
            return Err("random_int(): Argument #1 ($min) must be less than or equal to argument #2 ($max)".to_string());
        }
        let val = rand::thread_rng().gen_range(min_i..=max_i);
        Ok(Value::new_int(val as i32))
    }
}

php_function! {
    native_randomizer_construct(_this: Value, _engine: Value) {
        Ok(Value::null())
    }
}

php_function! {
    native_randomizer_pick_array_keys(_this: Value, array: Value, num: Value) |ctx| {
        let arr_val = array.ok_or_else(|| "Randomizer::pickArrayKeys(): Argument #1 ($array) is required".to_string())?;
        let num_val = num.ok_or_else(|| "Randomizer::pickArrayKeys(): Argument #2 ($num) is required".to_string())?;
        
        let arr_ptr = arr_val.as_array_ptr().ok_or_else(|| "Randomizer::pickArrayKeys(): Argument #1 ($array) must be of type array".to_string())?;
        let num_int = num_val.as_int().ok_or_else(|| "Randomizer::pickArrayKeys(): Argument #2 ($num) must be of type int".to_string())?;
        
        let arr = unsafe { &*(arr_ptr as *const PhpArray) };
        let count = arr.elements.len();
        
        if num_int <= 0 || (num_int as usize) > count {
            return Err(format!("Randomizer::pickArrayKeys(): Argument #2 ($num) must be between 1 and the number of elements in argument #1 ($array) ({})", count));
        }
        
        let mut keys: Vec<Value> = Vec::with_capacity(count);
        for (k, _) in arr.elements.iter() {
            match k {
                ArrayKey::Int(i) => keys.push(Value::new_int(*i as i32)),
                ArrayKey::StringId(sid) => {
                    let s = hyperion_core::types::string_table::lookup_string(*sid).unwrap_or_default();
                    let s_ptr = ctx.get_arena().alloc_and_track(s);
                    keys.push(Value::new_string_ptr(s_ptr as *mut ()));
                }
            }
        }
        
        let mut rng = rand::thread_rng();
        keys.shuffle(&mut rng);
        keys.truncate(num_int as usize);
        
        let mut res = PhpArray::new();
        for (i, k_val) in keys.into_iter().enumerate() {
            res.insert_int(i as i64, k_val);
        }
        
        let res_ptr = ctx.get_arena().alloc_and_track(res);
        Ok(Value::new_array_ptr(res_ptr as *mut ()))
    }
}

php_function! {
    native_randomizer_shuffle_array(_this: Value, array: Value) |ctx| {
        let arr_val = array.ok_or_else(|| "Randomizer::shuffleArray(): Argument #1 ($array) is required".to_string())?;
        let arr_ptr = arr_val.as_array_ptr().ok_or_else(|| "Randomizer::shuffleArray(): Argument #1 ($array) must be of type array".to_string())?;
        let arr = unsafe { &*(arr_ptr as *const PhpArray) };
        
        let mut values: Vec<Value> = arr.elements.iter().map(|(_, v)| *v).collect();
        let mut rng = rand::thread_rng();
        values.shuffle(&mut rng);
        
        let mut res = PhpArray::new();
        for (i, v) in values.into_iter().enumerate() {
            res.insert_int(i as i64, v);
        }
        
        let res_ptr = ctx.get_arena().alloc_and_track(res);
        Ok(Value::new_array_ptr(res_ptr as *mut ()))
    }
}

php_function! {
    native_randomizer_shuffle_bytes(_this: Value, bytes: Value) |ctx| {
        let bytes_val = bytes.ok_or_else(|| "Randomizer::shuffleBytes(): Argument #1 ($bytes) is required".to_string())?;
        let s_ptr = bytes_val.as_string_ptr().ok_or_else(|| "Randomizer::shuffleBytes(): Argument #1 ($bytes) must be of type string".to_string())?;
        let s = unsafe { &*(s_ptr as *const String) };
        let mut byte_vec = s.as_bytes().to_vec();
        let mut rng = rand::thread_rng();
        byte_vec.shuffle(&mut rng);
        let res_str = String::from_utf8_lossy(&byte_vec).to_string();
        let res_ptr = ctx.get_arena().alloc_and_track(res_str);
        Ok(Value::new_string_ptr(res_ptr as *mut ()))
    }
}

php_function! {
    native_randomizer_get_int(_this: Value, min: Value, max: Value) {
        let min_i = min.and_then(|v| v.as_int()).unwrap_or(0) as i64;
        let max_i = max.and_then(|v| v.as_int()).unwrap_or(0) as i64;
        if min_i > max_i {
            return Err("Randomizer::getInt(): Argument #1 ($min) must be less than or equal to argument #2 ($max)".to_string());
        }
        let val = rand::thread_rng().gen_range(min_i..=max_i);
        Ok(Value::new_int(val as i32))
    }
}

php_function! {
    native_randomizer_get_bytes(_this: Value, length: Value) |ctx| {
        let len_val = length.ok_or_else(|| "Randomizer::getBytes(): Argument #1 ($length) is required".to_string())?;
        let len = match len_val.deref().as_int() {
            Some(l) if l > 0 => (l as usize).min(50_000_000),
            _ => return Err("Randomizer::getBytes(): Argument #1 ($length) must be greater than 0".to_string()),
        };
        let mut bytes = vec![0u8; len];
        OsRng.fill_bytes(&mut bytes);
        let s = unsafe { String::from_utf8_unchecked(bytes) };
        let s_ptr = ctx.get_arena().alloc_and_track(s);
        Ok(Value::new_string_ptr(s_ptr as *mut ()))
    }
}

php_function! {
    native_randomizer_get_bytes_from_string(_this: Value, string: Value, length: Value) |ctx| {
        let str_val = string.ok_or_else(|| "Randomizer::getBytesFromString(): Argument #1 ($string) is required".to_string())?;
        let len_val = length.ok_or_else(|| "Randomizer::getBytesFromString(): Argument #2 ($length) is required".to_string())?;
        let s_ptr = str_val.deref().as_string_ptr().ok_or_else(|| "Randomizer::getBytesFromString(): Argument #1 ($string) must be of type string".to_string())?;
        let s = unsafe { &*(s_ptr as *const String) };
        if s.is_empty() {
            return Err("Randomizer::getBytesFromString(): Argument #1 ($string) cannot be empty".to_string());
        }
        let len = match len_val.deref().as_int() {
            Some(l) if l > 0 => (l as usize).min(50_000_000),
            _ => return Err("Randomizer::getBytesFromString(): Argument #2 ($length) must be greater than 0".to_string()),
        };
        let bytes = s.as_bytes();
        let mut rng = rand::thread_rng();
        let mut out = Vec::with_capacity(len);
        for _ in 0..len {
            let idx = rng.gen_range(0..bytes.len());
            out.push(bytes[idx]);
        }
        let res_str = String::from_utf8_lossy(&out).to_string();
        let res_ptr = ctx.get_arena().alloc_and_track(res_str);
        Ok(Value::new_string_ptr(res_ptr as *mut ()))
    }
}

php_function! {
    native_randomizer_next_float(_this: Value) {
        let f: f64 = rand::thread_rng().gen_range(0.0..1.0);
        Ok(Value::new_float(f))
    }
}

php_function! {
    native_randomizer_get_float(_this: Value, min: Value, max: Value, _boundary: Value) {
        let min_f = min.and_then(|v| v.as_float().or_else(|| v.as_int().map(|i| i as f64))).unwrap_or(0.0);
        let max_f = max.and_then(|v| v.as_float().or_else(|| v.as_int().map(|i| i as f64))).unwrap_or(1.0);
        let val = rand::thread_rng().gen_range(min_f..=max_f);
        Ok(Value::new_float(val))
    }
}
