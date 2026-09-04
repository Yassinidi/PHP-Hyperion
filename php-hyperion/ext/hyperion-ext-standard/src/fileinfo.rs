use hyperion_core::memory::nan_box::Value;
use hyperion_core::php_function;
use hyperion_core::types::object::PhpObject;
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub fn guess_mime_from_path(path: &str) -> &'static str {
    let p = Path::new(path);
    if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
        match ext.to_ascii_lowercase().as_str() {
            "html" | "htm" => "text/html",
            "css" => "text/css",
            "js" | "mjs" => "text/javascript",
            "json" => "application/json",
            "xml" => "application/xml",
            "txt" | "md" | "log" | "env" => "text/plain",
            "csv" => "text/csv",
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "svg" => "image/svg+xml",
            "ico" => "image/vnd.microsoft.icon",
            "bmp" => "image/bmp",
            "pdf" => "application/pdf",
            "zip" => "application/zip",
            "tar" => "application/x-tar",
            "gz" => "application/gzip",
            "mp4" => "video/mp4",
            "webm" => "video/webm",
            "ogv" => "video/ogg",
            "mp3" => "audio/mpeg",
            "wav" => "audio/wav",
            "ogg" => "audio/ogg",
            "woff" => "font/woff",
            "woff2" => "font/woff2",
            "ttf" => "font/ttf",
            "otf" => "font/otf",
            "php" => "text/x-php",
            _ => "application/octet-stream",
        }
    } else {
        "application/octet-stream"
    }
}

pub fn guess_mime_from_buffer(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        "image/png"
    } else if bytes.starts_with(b"\xff\xd8\xff") {
        "image/jpeg"
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        "image/gif"
    } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        "image/webp"
    } else if bytes.starts_with(b"%PDF-") {
        "application/pdf"
    } else if bytes.starts_with(b"PK\x03\x04") {
        "application/zip"
    } else if bytes.starts_with(b"<!DOCTYPE html")
        || bytes.starts_with(b"<!doctype html")
        || bytes.starts_with(b"<html")
    {
        "text/html"
    } else if bytes.starts_with(b"<?xml") {
        "application/xml"
    } else if (bytes.starts_with(b"{") && bytes.ends_with(b"}"))
        || (bytes.starts_with(b"[") && bytes.ends_with(b"]"))
    {
        "application/json"
    } else if std::str::from_utf8(bytes).is_ok() {
        "text/plain"
    } else {
        "application/octet-stream"
    }
}

pub fn guess_mime_from_file_path(path: &str) -> String {
    if let Ok(mut f) = File::open(path) {
        let mut buf = [0u8; 512];
        if let Ok(n) = f.read(&mut buf) {
            if n > 0 {
                let detected = guess_mime_from_buffer(&buf[..n]);
                if detected != "text/plain" && detected != "application/octet-stream" {
                    return detected.to_string();
                }
            }
        }
    }
    guess_mime_from_path(path).to_string()
}

php_function! {
    native_finfo_construct(this: Value, flags: Value, _magic_database: Value) {
        if let Some(obj_ptr) = this.and_then(|v| v.deref().as_object_ptr()) {
            let obj = unsafe { &mut *(obj_ptr as *mut PhpObject) };
            let f = flags.and_then(|v| v.deref().as_int()).unwrap_or(16); // 16 = FILEINFO_MIME_TYPE
            obj.properties.insert("flags".to_string(), Value::new_int(f));
        }
        Ok(Value::null())
    }
}

php_function! {
    native_finfo_file(this: Value, filename: Value, _flags: Value, _context: Value) |ctx| {
        let Some(fn_val) = filename else { return Ok(Value::new_bool(false)) };
        let Some(sp) = fn_val.deref().as_string_ptr() else { return Ok(Value::new_bool(false)) };
        let s = unsafe { &*(sp as *const String) };
        let mime = guess_mime_from_file_path(s);
        let s_ptr = crate::into_raw(Box::new(mime));
        Ok(Value::new_string_ptr(s_ptr as *mut ()))
    }
}

php_function! {
    native_finfo_buffer(this: Value, string: Value, _flags: Value, _context: Value) |ctx| {
        let Some(str_val) = string else { return Ok(Value::new_bool(false)) };
        let Some(sp) = str_val.deref().as_string_ptr() else { return Ok(Value::new_bool(false)) };
        let s = unsafe { &*(sp as *const String) };
        let mime = guess_mime_from_buffer(s.as_bytes()).to_string();
        let s_ptr = crate::into_raw(Box::new(mime));
        Ok(Value::new_string_ptr(s_ptr as *mut ()))
    }
}

php_function! {
    native_finfo_set_flags(this: Value, flags: Value) {
        if let Some(obj_ptr) = this.and_then(|v| v.deref().as_object_ptr()) {
            let obj = unsafe { &mut *(obj_ptr as *mut PhpObject) };
            let f = flags.and_then(|v| v.deref().as_int()).unwrap_or(0);
            obj.properties.insert("flags".to_string(), Value::new_int(f));
        }
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_finfo_open(flags: Value, _magic_database: Value) |ctx| {
        let class_id = ctx.get_class_id("finfo").unwrap_or(0);
        let mut obj = PhpObject::new_with_name(class_id, "finfo".to_string());
        let f = flags.and_then(|v| v.deref().as_int()).unwrap_or(16);
        obj.properties.insert("flags".to_string(), Value::new_int(f));
        let ptr = ctx.get_arena().alloc(obj);
        Ok(Value::new_object_ptr(ptr as *mut ()))
    }
}

php_function! {
    native_finfo_close(_this: Value) {
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_mime_content_type(filename: Value) |ctx| {
        let Some(fn_val) = filename else { return Ok(Value::new_bool(false)) };
        let Some(sp) = fn_val.deref().as_string_ptr() else { return Ok(Value::new_bool(false)) };
        let s = unsafe { &*(sp as *const String) };
        let mime = guess_mime_from_file_path(s);
        let s_ptr = crate::into_raw(Box::new(mime));
        Ok(Value::new_string_ptr(s_ptr as *mut ()))
    }
}
