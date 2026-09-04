use hyperion_core::memory::nan_box::Value;
use hyperion_core::php_function;
use hyperion_core::types::array::PhpArray;
use hyperion_core::types::resource::{PhpResource, ResourceKind};
use image::{imageops::FilterType, DynamicImage, ImageBuffer, Rgba, RgbaImage};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

#[derive(Clone)]
pub struct GdImage {
    pub img: RgbaImage,
}

lazy_static::lazy_static! {
    static ref GD_IMAGES: Mutex<HashMap<usize, GdImage>> = Mutex::new(HashMap::new());
    static ref NEXT_GD_ID: Mutex<usize> = Mutex::new(1);
}

fn next_gd_id() -> usize {
    let mut g = NEXT_GD_ID.lock().unwrap();
    let id = *g;
    *g += 1;
    id
}

fn get_gd_id(val: Option<&Value>) -> Option<usize> {
    let val = val?.deref();
    if let Some(res_ptr) = val.as_resource_ptr() {
        let res = unsafe { &*(res_ptr as *const PhpResource) };
        if let ResourceKind::Curl(id) = res.kind {
            return Some(id);
        }
    }
    None
}

php_function! {
    native_imagecreatetruecolor(width: Value, height: Value) {
        let (Some(w_v), Some(h_v)) = (width, height) else {
            return Ok(Value::new_bool(false));
        };
        let w = w_v.deref().as_int().unwrap_or(0).max(1) as u32;
        let h = h_v.deref().as_int().unwrap_or(0).max(1) as u32;

        let id = next_gd_id();
        let img = ImageBuffer::from_pixel(w, h, Rgba([0, 0, 0, 255]));

        let mut g = GD_IMAGES.lock().unwrap();
        g.insert(id, GdImage { img });

        let res = PhpResource::new(ResourceKind::Curl(id), "gd");
        let ptr = crate::into_raw(Box::new(res));
        Ok(Value::new_resource_ptr(ptr as *mut ()))
    }
}

php_function! {
    native_imagecreate(width: Value, height: Value) |ctx| {
        let w_val = width.copied().unwrap_or_else(Value::null);
        let h_val = height.copied().unwrap_or_else(Value::null);
        let args = [w_val, h_val];
        native_imagecreatetruecolor(&args, ctx)
    }
}

php_function! {
    native_imagecolorallocate(im: Value, red: Value, green: Value, blue: Value) {
        let _ = im;
        let r = red.and_then(|v| v.deref().as_int()).unwrap_or(0) as u8;
        let g = green.and_then(|v| v.deref().as_int()).unwrap_or(0) as u8;
        let b = blue.and_then(|v| v.deref().as_int()).unwrap_or(0) as u8;

        let packed = ((r as i32) << 16) | ((g as i32) << 8) | (b as i32);
        Ok(Value::new_int(packed))
    }
}

php_function! {
    native_imagesx(im: Value) {
        let Some(id) = get_gd_id(im) else {
            return Ok(Value::new_int(0));
        };
        let g = GD_IMAGES.lock().unwrap();
        let w = g.get(&id).map(|i| i.img.width()).unwrap_or(0);
        Ok(Value::new_int(w as i32))
    }
}

php_function! {
    native_imagesy(im: Value) {
        let Some(id) = get_gd_id(im) else {
            return Ok(Value::new_int(0));
        };
        let g = GD_IMAGES.lock().unwrap();
        let h = g.get(&id).map(|i| i.img.height()).unwrap_or(0);
        Ok(Value::new_int(h as i32))
    }
}

php_function! {
    native_imagepng(im: Value, to: Value) {
        let Some(id) = get_gd_id(im) else {
            return Ok(Value::new_bool(false));
        };
        let g = GD_IMAGES.lock().unwrap();
        let Some(gd) = g.get(&id) else {
            return Ok(Value::new_bool(false));
        };

        if let Some(to_v) = to {
            if let Some(sp) = to_v.deref().as_string_ptr() {
                let path_str = unsafe { &*(sp as *const String) };
                if let Some(parent) = Path::new(path_str).parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                return Ok(Value::new_bool(gd.img.save_with_format(path_str, image::ImageFormat::Png).is_ok()));
            }
        }
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_imagejpeg(im: Value, to: Value, quality: Value) {
        let _ = quality;
        let Some(id) = get_gd_id(im) else {
            return Ok(Value::new_bool(false));
        };
        let g = GD_IMAGES.lock().unwrap();
        let Some(gd) = g.get(&id) else {
            return Ok(Value::new_bool(false));
        };

        if let Some(to_v) = to {
            if let Some(sp) = to_v.deref().as_string_ptr() {
                let path_str = unsafe { &*(sp as *const String) };
                if let Some(parent) = Path::new(path_str).parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let rgb = DynamicImage::ImageRgba8(gd.img.clone()).to_rgb8();
                return Ok(Value::new_bool(rgb.save_with_format(path_str, image::ImageFormat::Jpeg).is_ok()));
            }
        }
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_imagecreatefrompng(filename: Value) {
        let Some(fname_v) = filename else {
            return Ok(Value::new_bool(false));
        };
        let Some(fname_ptr) = fname_v.deref().as_string_ptr() else {
            return Ok(Value::new_bool(false));
        };
        let fname = unsafe { &*(fname_ptr as *const String) };

        if let Ok(img) = image::open(fname) {
            let id = next_gd_id();
            let mut g = GD_IMAGES.lock().unwrap();
            g.insert(id, GdImage { img: img.to_rgba8() });

            let res = PhpResource::new(ResourceKind::Curl(id), "gd");
            let ptr = crate::into_raw(Box::new(res));
            Ok(Value::new_resource_ptr(ptr as *mut ()))
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_imagecreatefromjpeg(filename: Value) {
        let Some(fname_v) = filename else {
            return Ok(Value::new_bool(false));
        };
        let Some(fname_ptr) = fname_v.deref().as_string_ptr() else {
            return Ok(Value::new_bool(false));
        };
        let fname = unsafe { &*(fname_ptr as *const String) };

        if let Ok(img) = image::open(fname) {
            let id = next_gd_id();
            let mut g = GD_IMAGES.lock().unwrap();
            g.insert(id, GdImage { img: img.to_rgba8() });

            let res = PhpResource::new(ResourceKind::Curl(id), "gd");
            let ptr = crate::into_raw(Box::new(res));
            Ok(Value::new_resource_ptr(ptr as *mut ()))
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_imagecopyresampled(
        dst_im: Value,
        src_im: Value,
        dst_x: Value,
        dst_y: Value,
        src_x: Value,
        src_y: Value,
        dst_w: Value,
        dst_h: Value,
        src_w: Value,
        src_h: Value
    ) {
        let (Some(dst_id), Some(src_id)) = (get_gd_id(dst_im), get_gd_id(src_im)) else {
            return Ok(Value::new_bool(false));
        };

        let target_w = dst_w.and_then(|v| v.deref().as_int()).unwrap_or(0).max(1) as u32;
        let target_h = dst_h.and_then(|v| v.deref().as_int()).unwrap_or(0).max(1) as u32;

        let src_img = {
            let g = GD_IMAGES.lock().unwrap();
            match g.get(&src_id) {
                Some(s) => s.img.clone(),
                None => return Ok(Value::new_bool(false)),
            }
        };

        let resized = image::imageops::resize(&src_img, target_w, target_h, FilterType::Lanczos3);

        let mut g = GD_IMAGES.lock().unwrap();
        if let Some(dst) = g.get_mut(&dst_id) {
            let target_dx = dst_x.and_then(|v| v.deref().as_int()).unwrap_or(0) as i64;
            let target_dy = dst_y.and_then(|v| v.deref().as_int()).unwrap_or(0) as i64;
            image::imageops::overlay(&mut dst.img, &resized, target_dx, target_dy);
            Ok(Value::new_bool(true))
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_imagecopyresized(
        dst_im: Value,
        src_im: Value,
        dst_x: Value,
        dst_y: Value,
        src_x: Value,
        src_y: Value,
        dst_w: Value,
        dst_h: Value,
        src_w: Value,
        src_h: Value
    ) |ctx| {
        let args = [
            dst_im.copied().unwrap_or_else(Value::null),
            src_im.copied().unwrap_or_else(Value::null),
            dst_x.copied().unwrap_or_else(Value::null),
            dst_y.copied().unwrap_or_else(Value::null),
            src_x.copied().unwrap_or_else(Value::null),
            src_y.copied().unwrap_or_else(Value::null),
            dst_w.copied().unwrap_or_else(Value::null),
            dst_h.copied().unwrap_or_else(Value::null),
            src_w.copied().unwrap_or_else(Value::null),
            src_h.copied().unwrap_or_else(Value::null),
        ];
        native_imagecopyresampled(&args, ctx)
    }
}

php_function! {
    native_getimagesize(filename: Value) |ctx| {
        let Some(fname_v) = filename else {
            return Ok(Value::new_bool(false));
        };
        let Some(fname_ptr) = fname_v.deref().as_string_ptr() else {
            return Ok(Value::new_bool(false));
        };
        let fname = unsafe { &*(fname_ptr as *const String) };

        if let Ok(reader) = image::ImageReader::open(fname) {
            if let Ok(dim) = reader.into_dimensions() {
                let mut arr = PhpArray::new();
                arr.insert_int(0, Value::new_int(dim.0 as i32));
                arr.insert_int(1, Value::new_int(dim.1 as i32));

                let mime = if fname.ends_with(".png") {
                    "image/png"
                } else if fname.ends_with(".jpg") || fname.ends_with(".jpeg") {
                    "image/jpeg"
                } else if fname.ends_with(".webp") {
                    "image/webp"
                } else {
                    "image/octet-stream"
                };

                let mime_id = ctx.intern_string("mime");
                let mime_ptr = ctx.get_arena().alloc_and_track(mime.to_string());
                arr.insert_string_id(mime_id, Value::new_string_ptr(mime_ptr as *mut ()));

                let ptr = ctx.get_arena().alloc_and_track(arr);
                return Ok(Value::new_array_ptr(ptr as *mut ()));
            }
        }
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_imagedestroy(im: Value) {
        if let Some(id) = get_gd_id(im) {
            let mut g = GD_IMAGES.lock().unwrap();
            g.remove(&id);
        }
        Ok(Value::new_bool(true))
    }
}
