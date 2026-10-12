use flate2::read::{DeflateDecoder, DeflateEncoder, GzDecoder, GzEncoder, ZlibDecoder, ZlibEncoder};
use flate2::Compression;
use hyperion_core::memory::nan_box::Value;
use hyperion_core::php_function;
use std::io::Read;

pub const FORCE_GZIP: i64 = 31;
pub const FORCE_DEFLATE: i64 = 15;

php_function! {
    native_gzencode(data: Value, level: Value) |ctx| {
        let Some(data_v) = data else {
            return Ok(Value::new_bool(false));
        };
        let Some(sp) = data_v.deref().as_string_ptr() else {
            return Ok(Value::new_bool(false));
        };
        let bytes = unsafe { (*(sp as *const String)).as_bytes() };
        let lvl = level.and_then(|v| v.deref().as_int()).unwrap_or(-1);
        let compression = if lvl < 0 || lvl > 9 {
            Compression::default()
        } else {
            Compression::new(lvl as u32)
        };

        let mut encoder = GzEncoder::new(bytes, compression);
        let mut out = Vec::new();
        if encoder.read_to_end(&mut out).is_ok() {
            let s = unsafe { String::from_utf8_unchecked(out) };
            let ptr = ctx.get_arena().alloc_and_track(s);
            Ok(Value::new_string_ptr(ptr as *mut ()))
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_gzdecode(data: Value) |ctx| {
        let Some(data_v) = data else {
            return Ok(Value::new_bool(false));
        };
        let Some(sp) = data_v.deref().as_string_ptr() else {
            return Ok(Value::new_bool(false));
        };
        let bytes = unsafe { (*(sp as *const String)).as_bytes() };

        let mut decoder = GzDecoder::new(bytes);
        let mut out = Vec::new();
        if decoder.read_to_end(&mut out).is_ok() {
            let s = unsafe { String::from_utf8_unchecked(out) };
            let ptr = ctx.get_arena().alloc_and_track(s);
            Ok(Value::new_string_ptr(ptr as *mut ()))
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_gzcompress(data: Value, level: Value) |ctx| {
        let Some(data_v) = data else {
            return Ok(Value::new_bool(false));
        };
        let Some(sp) = data_v.deref().as_string_ptr() else {
            return Ok(Value::new_bool(false));
        };
        let bytes = unsafe { (*(sp as *const String)).as_bytes() };
        let lvl = level.and_then(|v| v.deref().as_int()).unwrap_or(-1);
        let compression = if lvl < 0 || lvl > 9 {
            Compression::default()
        } else {
            Compression::new(lvl as u32)
        };

        let mut encoder = ZlibEncoder::new(bytes, compression);
        let mut out = Vec::new();
        if encoder.read_to_end(&mut out).is_ok() {
            let s = unsafe { String::from_utf8_unchecked(out) };
            let ptr = ctx.get_arena().alloc_and_track(s);
            Ok(Value::new_string_ptr(ptr as *mut ()))
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_gzuncompress(data: Value) |ctx| {
        let Some(data_v) = data else {
            return Ok(Value::new_bool(false));
        };
        let Some(sp) = data_v.deref().as_string_ptr() else {
            return Ok(Value::new_bool(false));
        };
        let bytes = unsafe { (*(sp as *const String)).as_bytes() };

        let mut decoder = ZlibDecoder::new(bytes);
        let mut out = Vec::new();
        if decoder.read_to_end(&mut out).is_ok() {
            let s = unsafe { String::from_utf8_unchecked(out) };
            let ptr = ctx.get_arena().alloc_and_track(s);
            Ok(Value::new_string_ptr(ptr as *mut ()))
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_gzdeflate(data: Value, level: Value) |ctx| {
        let Some(data_v) = data else {
            return Ok(Value::new_bool(false));
        };
        let Some(sp) = data_v.deref().as_string_ptr() else {
            return Ok(Value::new_bool(false));
        };
        let bytes = unsafe { (*(sp as *const String)).as_bytes() };
        let lvl = level.and_then(|v| v.deref().as_int()).unwrap_or(-1);
        let compression = if lvl < 0 || lvl > 9 {
            Compression::default()
        } else {
            Compression::new(lvl as u32)
        };

        let mut encoder = DeflateEncoder::new(bytes, compression);
        let mut out = Vec::new();
        if encoder.read_to_end(&mut out).is_ok() {
            let s = unsafe { String::from_utf8_unchecked(out) };
            let ptr = ctx.get_arena().alloc_and_track(s);
            Ok(Value::new_string_ptr(ptr as *mut ()))
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_gzinflate(data: Value) |ctx| {
        let Some(data_v) = data else {
            return Ok(Value::new_bool(false));
        };
        let Some(sp) = data_v.deref().as_string_ptr() else {
            return Ok(Value::new_bool(false));
        };
        let bytes = unsafe { (*(sp as *const String)).as_bytes() };

        let mut decoder = DeflateDecoder::new(bytes);
        let mut out = Vec::new();
        if decoder.read_to_end(&mut out).is_ok() {
            let s = unsafe { String::from_utf8_unchecked(out) };
            let ptr = ctx.get_arena().alloc_and_track(s);
            Ok(Value::new_string_ptr(ptr as *mut ()))
        } else {
            Ok(Value::new_bool(false))
        }
    }
}
