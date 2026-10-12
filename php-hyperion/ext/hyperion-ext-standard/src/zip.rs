use hyperion_core::memory::nan_box::Value;
use hyperion_core::php_function;
use hyperion_core::types::object::PhpObject;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Mutex;
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

pub struct OpenZipArchive {
    pub filename: String,
    pub entries: Vec<(String, Vec<u8>)>, // (local_name, bytes)
    pub is_new: bool,
}

lazy_static::lazy_static! {
    static ref ZIP_ARCHIVES: Mutex<HashMap<usize, OpenZipArchive>> = Mutex::new(HashMap::new());
    static ref NEXT_ZIP_ID: Mutex<usize> = Mutex::new(1);
}

fn next_zip_id() -> usize {
    let mut g = NEXT_ZIP_ID.lock().unwrap();
    let id = *g;
    *g += 1;
    id
}

php_function! {
    native_ziparchive_open(this: Value, filename: Value, flags: Value) {
        let (Some(this_v), Some(fname_v)) = (this, filename) else {
            return Ok(Value::new_bool(false));
        };
        let Some(fname_ptr) = fname_v.deref().as_string_ptr() else {
            return Ok(Value::new_bool(false));
        };
        let fname = unsafe { (*(fname_ptr as *const String)).clone() };
        let _ = flags;

        let id = next_zip_id();
        let mut entries = Vec::new();
        let path = Path::new(&fname);

        if path.exists() {
            if let Ok(file) = File::open(path) {
                if let Ok(mut zip) = ZipArchive::new(file) {
                    for i in 0..zip.len() {
                        if let Ok(mut file_entry) = zip.by_index(i) {
                            let name = file_entry.name().to_string();
                            let mut buf = Vec::new();
                            let _ = file_entry.read_to_end(&mut buf);
                            entries.push((name, buf));
                        }
                    }
                }
            }
        }

        let num_files = entries.len();
        {
            let mut g = ZIP_ARCHIVES.lock().unwrap();
            g.insert(id, OpenZipArchive {
                filename: fname.clone(),
                entries,
                is_new: !path.exists(),
            });
        }

        // Store zip_id and numFiles on $this object properties
        if let Some(obj_ptr) = this_v.deref().as_object_ptr() {
            let obj = unsafe { &mut *(obj_ptr as *mut PhpObject) };
            obj.properties.insert("__zip_id".to_string(), Value::new_int(id as i32));
            obj.properties.insert("numFiles".to_string(), Value::new_int(num_files as i32));
            obj.properties.insert("status".to_string(), Value::new_int(0));

            let name_str_ptr = crate::into_raw(Box::new(fname));
            obj.properties.insert("filename".to_string(), Value::new_string_ptr(name_str_ptr as *mut ()));
        }

        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_ziparchive_extract_to(this: Value, destination: Value) {
        let (Some(this_v), Some(dest_v)) = (this, destination) else {
            return Ok(Value::new_bool(false));
        };
        let Some(dest_ptr) = dest_v.deref().as_string_ptr() else {
            return Ok(Value::new_bool(false));
        };
        let dest_path = Path::new(unsafe { &*(dest_ptr as *const String) });

        let Some(zip_id) = get_zip_id(this_v) else {
            return Ok(Value::new_bool(false));
        };

        let g = ZIP_ARCHIVES.lock().unwrap();
        let Some(archive) = g.get(&zip_id) else {
            return Ok(Value::new_bool(false));
        };

        for (name, bytes) in &archive.entries {
            let target_path = dest_path.join(name);
            if let Some(parent) = target_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(&target_path, bytes);
        }

        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_ziparchive_add_file(this: Value, filename: Value, localname: Value) {
        let (Some(this_v), Some(fname_v)) = (this, filename) else {
            return Ok(Value::new_bool(false));
        };
        let Some(fname_ptr) = fname_v.deref().as_string_ptr() else {
            return Ok(Value::new_bool(false));
        };
        let fname = unsafe { &*(fname_ptr as *const String) };

        let loc_name = if let Some(l) = localname {
            if let Some(lp) = l.deref().as_string_ptr() {
                unsafe { (*(lp as *const String)).clone() }
            } else {
                Path::new(fname).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
            }
        } else {
            Path::new(fname).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
        };

        let Some(zip_id) = get_zip_id(this_v) else {
            return Ok(Value::new_bool(false));
        };

        let Ok(bytes) = std::fs::read(fname) else {
            return Ok(Value::new_bool(false));
        };

        let mut g = ZIP_ARCHIVES.lock().unwrap();
        if let Some(archive) = g.get_mut(&zip_id) {
            archive.entries.push((loc_name, bytes));
            if let Some(obj_ptr) = this_v.deref().as_object_ptr() {
                let obj = unsafe { &mut *(obj_ptr as *mut PhpObject) };
                obj.properties.insert("numFiles".to_string(), Value::new_int(archive.entries.len() as i32));
            }
            Ok(Value::new_bool(true))
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_ziparchive_add_from_string(this: Value, localname: Value, contents: Value) {
        let (Some(this_v), Some(loc_v), Some(cont_v)) = (this, localname, contents) else {
            return Ok(Value::new_bool(false));
        };
        let (Some(loc_ptr), Some(cont_ptr)) = (loc_v.deref().as_string_ptr(), cont_v.deref().as_string_ptr()) else {
            return Ok(Value::new_bool(false));
        };
        let loc_name = unsafe { (*(loc_ptr as *const String)).clone() };
        let bytes = unsafe { (*(cont_ptr as *const String)).as_bytes().to_vec() };

        let Some(zip_id) = get_zip_id(this_v) else {
            return Ok(Value::new_bool(false));
        };

        let mut g = ZIP_ARCHIVES.lock().unwrap();
        if let Some(archive) = g.get_mut(&zip_id) {
            archive.entries.push((loc_name, bytes));
            if let Some(obj_ptr) = this_v.deref().as_object_ptr() {
                let obj = unsafe { &mut *(obj_ptr as *mut PhpObject) };
                obj.properties.insert("numFiles".to_string(), Value::new_int(archive.entries.len() as i32));
            }
            Ok(Value::new_bool(true))
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_ziparchive_close(this: Value) {
        let Some(this_v) = this else {
            return Ok(Value::new_bool(false));
        };
        let Some(zip_id) = get_zip_id(this_v) else {
            return Ok(Value::new_bool(false));
        };

        let archive_opt = {
            let mut g = ZIP_ARCHIVES.lock().unwrap();
            g.remove(&zip_id)
        };

        let Some(archive) = archive_opt else {
            return Ok(Value::new_bool(false));
        };

        if let Some(parent) = Path::new(&archive.filename).parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        if let Ok(file) = File::create(&archive.filename) {
            let mut writer = ZipWriter::new(file);
            let options = SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);

            for (name, bytes) in archive.entries {
                let _ = writer.start_file(name, options);
                let _ = writer.write_all(&bytes);
            }
            let _ = writer.finish();
        }

        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_ziparchive_get_name_index(this: Value, index: Value) |ctx| {
        let (Some(this_v), Some(idx_v)) = (this, index) else {
            return Ok(Value::new_bool(false));
        };
        let idx = idx_v.deref().as_int().unwrap_or(0) as usize;
        let Some(zip_id) = get_zip_id(this_v) else {
            return Ok(Value::new_bool(false));
        };

        let g = ZIP_ARCHIVES.lock().unwrap();
        let Some(archive) = g.get(&zip_id) else {
            return Ok(Value::new_bool(false));
        };

        if let Some((name, _)) = archive.entries.get(idx) {
            let ptr = ctx.get_arena().alloc_and_track(name.clone());
            Ok(Value::new_string_ptr(ptr as *mut ()))
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

fn get_zip_id(this: &Value) -> Option<usize> {
    let obj_ptr = this.deref().as_object_ptr()?;
    let obj = unsafe { &*(obj_ptr as *const PhpObject) };
    let val = obj.properties.get("__zip_id")?;
    val.deref().as_int().map(|i| i as usize)
}
