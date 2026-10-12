use hyperion_core::memory::nan_box::Value;
use hyperion_core::php_function;
use hyperion_core::types::array::PhpArray;

const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";

#[inline(always)]
fn fast_hex_encode(bytes: &[u8]) -> String {
    let mut out = Vec::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX_CHARS[(b >> 4) as usize]);
        out.push(HEX_CHARS[(b & 0x0f) as usize]);
    }
    unsafe { String::from_utf8_unchecked(out) }
}

fn php_string_helper(ctx: &mut dyn hyperion_core::types::function::NativeContext, s: &str) -> Value {
    let ptr = ctx.get_arena().alloc_and_track(s.to_string());
    Value::new_string_ptr(ptr as *mut ())
}

// ============================================================
// 🔒 PHP Hash & Password Module — ~10 native functions
// ============================================================

php_function! {
    native_hash(algo: String, data: String, binary: Value) {
        if let (Some(algo), Some(data)) = (algo, data) {
            let is_binary = binary.and_then(|b| b.as_bool()).unwrap_or(false);
            let bytes: Vec<u8> = match algo.to_lowercase().as_str() {
                "md5" => {
                    use md5::Digest;
                    md5::Md5::digest(data.as_bytes()).to_vec()
                }
                "sha1" => {
                    use sha1::Digest;
                    sha1::Sha1::digest(data.as_bytes()).to_vec()
                }
                "sha256" => {
                    use sha2::Digest;
                    sha2::Sha256::digest(data.as_bytes()).to_vec()
                }
                "sha512" => {
                    use sha2::Digest;
                    sha2::Sha512::digest(data.as_bytes()).to_vec()
                }
                "xxh128" | "xxh3" => {
                    let h = xxhash_rust::xxh3::xxh3_128(data.as_bytes());
                    h.to_be_bytes().to_vec()
                }
                "xxh64" => {
                    let h = xxhash_rust::xxh64::xxh64(data.as_bytes(), 0);
                    h.to_be_bytes().to_vec()
                }
                "xxh32" => {
                    let h = xxhash_rust::xxh32::xxh32(data.as_bytes(), 0);
                    h.to_be_bytes().to_vec()
                }
                _ => return Err(format!("hash(): Unknown hashing algorithm: {}", algo)),
            };

            let out_str = if is_binary {
                unsafe { String::from_utf8_unchecked(bytes) }
            } else {
                fast_hex_encode(&bytes)
            };

            let boxed = crate::into_raw(Box::new(out_str));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        } else {
            Err("hash() expects at least 2 parameters".to_string())
        }
    }
}

php_function! {
    native_hash_file(algo: String, filename: String, binary: Value) |ctx| {
        if let (Some(algo), Some(filename)) = (algo, filename) {
            let data = match std::fs::read(&filename) {
                Ok(d) => d,
                Err(_) => return Ok(Value::new_bool(false)),
            };
            let is_binary = binary.and_then(|b| b.as_bool()).unwrap_or(false);
            let bytes: Vec<u8> = match algo.to_lowercase().as_str() {
                "md5" => {
                    use md5::Digest;
                    md5::Md5::digest(&data).to_vec()
                }
                "sha1" => {
                    use sha1::Digest;
                    sha1::Sha1::digest(&data).to_vec()
                }
                "sha256" => {
                    use sha2::Digest;
                    sha2::Sha256::digest(&data).to_vec()
                }
                "sha512" => {
                    use sha2::Digest;
                    sha2::Sha512::digest(&data).to_vec()
                }
                "xxh128" | "xxh3" => {
                    let h = xxhash_rust::xxh3::xxh3_128(&data);
                    h.to_be_bytes().to_vec()
                }
                "xxh64" => {
                    let h = xxhash_rust::xxh64::xxh64(&data, 0);
                    h.to_be_bytes().to_vec()
                }
                "xxh32" => {
                    let h = xxhash_rust::xxh32::xxh32(&data, 0);
                    h.to_be_bytes().to_vec()
                }
                _ => return Err(format!("hash_file(): Unknown hashing algorithm: {}", algo)),
            };

            let out_str = if is_binary {
                unsafe { String::from_utf8_unchecked(bytes) }
            } else {
                fast_hex_encode(&bytes)
            };

            let str_ptr = ctx.get_arena().alloc_and_track(out_str);
            Ok(Value::new_string_ptr(str_ptr as *mut ()))
        } else {
            Err("hash_file() expects at least 2 parameters".to_string())
        }
    }
}

php_function! {
    native_hash_hmac(algo: String, data: String, key: String) {
        if let (Some(algo), Some(data), Some(key)) = (algo, data, key) {
            let hex = match algo.as_str() {
                "sha256" => {
                    use hmac::{Hmac, Mac};
                    type HmacSha256 = Hmac<sha2::Sha256>;
                    let mut mac = HmacSha256::new_from_slice(key.as_bytes())
                        .map_err(|e| format!("hash_hmac error: {}", e))?;
                    mac.update(data.as_bytes());
                    let result = mac.finalize();
                    fast_hex_encode(&result.into_bytes())
                }
                "sha1" => {
                    use hmac::{Hmac, Mac};
                    type HmacSha1 = Hmac<sha1::Sha1>;
                    let mut mac = HmacSha1::new_from_slice(key.as_bytes())
                        .map_err(|e| format!("hash_hmac error: {}", e))?;
                    mac.update(data.as_bytes());
                    let result = mac.finalize();
                    fast_hex_encode(&result.into_bytes())
                }
                "md5" => {
                    use hmac::{Hmac, Mac};
                    type HmacMd5 = Hmac<md5::Md5>;
                    let mut mac = HmacMd5::new_from_slice(key.as_bytes())
                        .map_err(|e| format!("hash_hmac error: {}", e))?;
                    mac.update(data.as_bytes());
                    let result = mac.finalize();
                    fast_hex_encode(&result.into_bytes())
                }
                _ => return Err(format!("hash_hmac(): Unknown hashing algorithm: {}", algo)),
            };
            let boxed = crate::into_raw(Box::new(hex));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        } else {
            Err("hash_hmac() expects exactly 3 parameters".to_string())
        }
    }
}

php_function! {
    native_hash_equals(known: String, user: String) {
        if let (Some(known), Some(user)) = (known, user) {
            // Constant-time comparison to prevent timing attacks
            if known.len() != user.len() {
                return Ok(Value::new_bool(false));
            }
            let mut result = 0u8;
            for (a, b) in known.bytes().zip(user.bytes()) {
                result |= a ^ b;
            }
            Ok(Value::new_bool(result == 0))
        } else {
            Err("hash_equals() expects exactly 2 parameters".to_string())
        }
    }
}

php_function! {
    native_crc32(data: String) {
        if let Some(s) = data {
            let mut crc: u32 = 0xFFFFFFFF;
            for byte in s.bytes() {
                crc ^= byte as u32;
                for _ in 0..8 {
                    if crc & 1 == 1 {
                        crc = (crc >> 1) ^ 0xEDB88320;
                    } else {
                        crc >>= 1;
                    }
                }
            }
            Ok(Value::new_int((crc ^ 0xFFFFFFFF) as i32))
        } else {
            Err("crc32() expects exactly 1 parameter".to_string())
        }
    }
}

// --- Password Functions ---

php_function! {
    native_password_hash(password: String, algo: Value, options: Value) {
        if let Some(pw) = password {
            let mut cost = bcrypt::DEFAULT_COST;
            if let Some(opts_val) = options {
                if let Some(arr_ptr) = opts_val.as_array_ptr() {
                    let arr = unsafe { &*(arr_ptr as *const PhpArray) };
                    if let Some(c_val) = arr.get_by_str("cost") {
                        let parsed_cost = if let Some(c) = c_val.as_int() {
                            Some(c)
                        } else if let Some(s_ptr) = c_val.as_string_ptr() {
                            let s = unsafe { &*(s_ptr as *const String) };
                            s.trim().parse::<i32>().ok()
                        } else {
                            None
                        };
                        if let Some(c) = parsed_cost {
                            cost = (c as u32).clamp(4, 31);
                        }
                    }
                }
            }
            match bcrypt::hash(pw.as_str(), cost) {
                Ok(mut hashed) => {
                    if hashed.starts_with("$2b$") {
                        hashed.replace_range(0..4, "$2y$");
                    }
                    let boxed = crate::into_raw(Box::new(hashed));
                    Ok(Value::new_string_ptr(boxed as *mut ()))
                }
                Err(e) => Err(format!("password_hash() error: {}", e)),
            }
        } else {
            Err("password_hash() expects at least 2 parameters".to_string())
        }
    }
}

php_function! {
    native_password_verify(password: String, hash: String) {
        if let (Some(pw), Some(h)) = (password, hash) {
            let normalized_hash = if h.starts_with("$2y$") || h.starts_with("$2x$") {
                format!("$2b${}", &h[4..])
            } else {
                h.clone()
            };
            match bcrypt::verify(pw.as_str(), normalized_hash.as_str()) {
                Ok(valid) => Ok(Value::new_bool(valid)),
                Err(_) => Ok(Value::new_bool(false)),
            }
        } else {
            Err("password_verify() expects exactly 2 parameters".to_string())
        }
    }
}

php_function! {
    native_password_needs_rehash(hash: String, algo: Value, options: Value) {
        if let Some(h) = hash {
            let mut target_cost = bcrypt::DEFAULT_COST;
            if let Some(opts_val) = options {
                if let Some(arr_ptr) = opts_val.as_array_ptr() {
                    let arr = unsafe { &*(arr_ptr as *const PhpArray) };
                    if let Some(c_val) = arr.get_by_str("cost") {
                        let parsed_cost = if let Some(c) = c_val.as_int() {
                            Some(c)
                        } else if let Some(s_ptr) = c_val.as_string_ptr() {
                            let s = unsafe { &*(s_ptr as *const String) };
                            s.trim().parse::<i32>().ok()
                        } else {
                            None
                        };
                        if let Some(c) = parsed_cost {
                            target_cost = (c as u32).clamp(4, 31);
                        }
                    }
                }
            }
            let h_str = h.as_str();
            let is_bcrypt = h_str.starts_with("$2y$") || h_str.starts_with("$2b$") || h_str.starts_with("$2a$");
            if !is_bcrypt {
                return Ok(Value::new_bool(true));
            }
            let current_cost = if h_str.len() >= 6 && h_str[4..6].chars().all(|c| c.is_ascii_digit()) {
                h_str[4..6].parse::<u32>().unwrap_or(12)
            } else {
                12
            };
            Ok(Value::new_bool(current_cost != target_cost))
        } else {
            Err("password_needs_rehash() expects at least 2 parameters".to_string())
        }
    }
}

php_function! {
    native_password_get_info(hash: String) |ctx| {
        let mut arr = PhpArray::new();
        let h_str = hash.map(|s| s.as_str()).unwrap_or("");
        if h_str.starts_with("$2y$") || h_str.starts_with("$2a$") || h_str.starts_with("$2b$") || h_str.starts_with("$2x$") {
            let algo_sid = ctx.intern_string("algo");
            let s_2y = php_string_helper(ctx, "2y");
            arr.insert_string_id(algo_sid, s_2y);

            let algoname_sid = ctx.intern_string("algoName");
            let s_bcrypt = php_string_helper(ctx, "bcrypt");
            arr.insert_string_id(algoname_sid, s_bcrypt);

            let mut opts = PhpArray::new();
            let cost_sid = ctx.intern_string("cost");
            let cost = if h_str.len() >= 6 && h_str[4..6].chars().all(|c| c.is_ascii_digit()) {
                h_str[4..6].parse::<i64>().unwrap_or(12)
            } else {
                12
            };
            opts.insert_string_id(cost_sid, Value::new_int(cost as i32));
            let opts_ptr = ctx.get_arena().alloc_and_track(opts);
            let opts_val = Value::new_array_ptr(opts_ptr as *mut ());

            let options_sid = ctx.intern_string("options");
            arr.insert_string_id(options_sid, opts_val);
        } else {
            let algo_sid = ctx.intern_string("algo");
            arr.insert_string_id(algo_sid, Value::null());

            let algoname_sid = ctx.intern_string("algoName");
            let s_unknown = php_string_helper(ctx, "unknown");
            arr.insert_string_id(algoname_sid, s_unknown);

            let opts = PhpArray::new();
            let opts_ptr = ctx.get_arena().alloc_and_track(opts);
            let opts_val = Value::new_array_ptr(opts_ptr as *mut ());

            let options_sid = ctx.intern_string("options");
            arr.insert_string_id(options_sid, opts_val);
        }

        let arr_ptr = ctx.get_arena().alloc_and_track(arr);
        Ok(Value::new_array_ptr(arr_ptr as *mut ()))
    }
}

php_function! {
    native_hash_algos() |ctx| {
        let algos = ["md2", "md4", "md5", "sha1", "sha224", "sha256", "sha384", "sha512", "ripemd128", "ripemd160", "ripemd256", "ripemd320", "whirlpool", "tiger128,3", "tiger160,3", "tiger192,3", "tiger128,4", "tiger160,4", "tiger192,4", "snefru", "snefru256", "gost", "gost-crypto", "adler32", "crc32", "crc32b", "crc32c", "fnv132", "fnv1a32", "fnv164", "fnv1a64", "joaat", "murmur3a", "murmur3c", "murmur3f", "xxh32", "xxh64", "xxh3", "xxh128"];
        let mut arr = PhpArray::new();
        for algo in algos {
            let s = php_string_helper(ctx, algo);
            arr.push(s);
        }
        let arr_ptr = ctx.get_arena().alloc_and_track(arr);
        Ok(Value::new_array_ptr(arr_ptr as *mut ()))
    }
}

php_function! {
    native_hash_hmac_algos() |ctx| {
        let algos = ["md2", "md4", "md5", "sha1", "sha224", "sha256", "sha384", "sha512", "ripemd128", "ripemd160", "ripemd256", "ripemd320", "whirlpool", "tiger128,3", "tiger160,3", "tiger192,3", "tiger128,4", "tiger160,4", "tiger192,4", "snefru", "snefru256", "gost", "gost-crypto", "adler32", "crc32", "crc32b", "crc32c", "fnv132", "fnv1a32", "fnv164", "fnv1a64", "joaat", "murmur3a", "murmur3c", "murmur3f", "xxh32", "xxh64", "xxh3", "xxh128"];
        let mut arr = PhpArray::new();
        for algo in algos {
            let s = php_string_helper(ctx, algo);
            arr.push(s);
        }
        let arr_ptr = ctx.get_arena().alloc_and_track(arr);
        Ok(Value::new_array_ptr(arr_ptr as *mut ()))
    }
}
