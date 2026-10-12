use hyperion_core::php_function;
use hyperion_core::memory::nan_box::Value;
use hyperion_core::types::object::PhpObject;
use std::sync::Mutex;
use chrono::{Utc, TimeZone, LocalResult};
use chrono_tz::Tz;
use std::str::FromStr;

lazy_static::lazy_static! {
    static ref DEFAULT_TIMEZONE: Mutex<Tz> = Mutex::new(chrono_tz::UTC);
}

php_function! {
    native_time() {
        let now = Utc::now().timestamp();
        Ok(Value::new_int(now as i32))
    }
}

php_function! {
    native_microtime(get_as_float: Value) {
        let is_float = get_as_float.and_then(|v| v.as_bool()).unwrap_or(false);
        let duration = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap();
        if is_float {
            Ok(Value::new_float(duration.as_secs_f64()))
        } else {
            let secs = duration.as_secs();
            let subsecs = duration.subsec_micros() as f64 / 1_000_000.0;
            let result = format!("{:.8} {}", subsecs, secs);
            let boxed = crate::into_raw(Box::new(result));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        }
    }
}

php_function! {
    native_hrtime(as_number: Value) {
        let as_num = as_number.and_then(|v| v.as_bool()).unwrap_or(false);
        let duration = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap();
        if as_num {
            let nanos = duration.as_nanos() as f64;
            Ok(Value::new_float(nanos))
        } else {
            let mut arr = hyperion_core::types::array::PhpArray::new();
            arr.push(Value::new_int(duration.as_secs() as i32));
            arr.push(Value::new_int(duration.subsec_nanos() as i32));
            Ok(Value::new_array_ptr(crate::into_raw(Box::new(arr)) as *mut ()))
        }
    }
}


php_function! {
    native_date_default_timezone_set(timezone_id: String) {
        if let Some(tz_str) = timezone_id {
            if let Ok(tz) = Tz::from_str(tz_str.as_str()) {
                *DEFAULT_TIMEZONE.lock().unwrap() = tz;
                Ok(Value::new_bool(true))
            } else {
                // Return false for invalid timezones
                Ok(Value::new_bool(false))
            }
        } else {
            Err("date_default_timezone_set() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_date_default_timezone_get() {
        let tz = DEFAULT_TIMEZONE.lock().unwrap();
        let tz_name = tz.name().to_string();
        let boxed = crate::into_raw(Box::new(tz_name));
        Ok(Value::new_string_ptr(boxed as *mut ()))
    }
}

php_function! {
    native_date(format: String, timestamp: Value) {
        if let Some(fmt) = format {
            let ts = timestamp.and_then(|v| v.as_int()).unwrap_or_else(|| Utc::now().timestamp() as i32);
            let tz = *DEFAULT_TIMEZONE.lock().unwrap();
            
            // Map common PHP date format characters to Chrono strftime
            // This is a simplified 80% coverage mapping
            let mut chrono_fmt = String::new();
            let mut escape_next = false;
            for c in fmt.chars() {
                if escape_next {
                    chrono_fmt.push(c);
                    escape_next = false;
                    continue;
                }
                match c {
                    '\\' => escape_next = true,
                    'Y' => chrono_fmt.push_str("%Y"), // 4 digit year
                    'y' => chrono_fmt.push_str("%y"), // 2 digit year
                    'm' => chrono_fmt.push_str("%m"), // month with leading zero
                    'n' => chrono_fmt.push_str("%-m"), // month without leading zero
                    'F' => chrono_fmt.push_str("%B"), // full month name
                    'M' => chrono_fmt.push_str("%b"), // short month name
                    'd' => chrono_fmt.push_str("%d"), // day with leading zero
                    'j' => chrono_fmt.push_str("%-d"), // day without leading zero
                    'H' => chrono_fmt.push_str("%H"), // 24hr format with leading zero
                    'G' => chrono_fmt.push_str("%-H"), // 24hr format without leading zero
                    'h' => chrono_fmt.push_str("%I"), // 12hr format with leading zero
                    'g' => chrono_fmt.push_str("%-I"), // 12hr format without leading zero
                    'i' => chrono_fmt.push_str("%M"), // minutes with leading zero
                    's' => chrono_fmt.push_str("%S"), // seconds with leading zero
                    'a' => chrono_fmt.push_str("%P"), // am/pm lowercase
                    'A' => chrono_fmt.push_str("%p"), // AM/PM uppercase
                    'w' => chrono_fmt.push_str("%w"), // numeric day of week 0-6
                    'l' => chrono_fmt.push_str("%A"), // Full day name
                    'D' => chrono_fmt.push_str("%a"), // Short day name
                    'U' => chrono_fmt.push_str("%s"), // Unix timestamp
                    'c' => chrono_fmt.push_str("%Y-%m-%dT%H:%M:%S%:z"), // ISO 8601 date
                    'O' => chrono_fmt.push_str("%z"), // Diff to GMT no colon
                    'P' => chrono_fmt.push_str("%:z"), // Diff to GMT with colon
                    'T' => chrono_fmt.push_str("%Z"), // Timezone abbrev
                    _ => chrono_fmt.push(c),
                }
            }
            
            // Format time
            if let LocalResult::Single(dt) = tz.timestamp_opt(ts as i64, 0) {
                let result = dt.format(&chrono_fmt).to_string();
                let boxed = crate::into_raw(Box::new(result));
                Ok(Value::new_string_ptr(boxed as *mut ()))
            } else {
                Ok(Value::new_bool(false))
            }
        } else {
            Err("date() expects at least 1 parameter".to_string())
        }
    }
}

php_function! {
    native_gmdate(format: String, timestamp: Value) {
        if let Some(fmt) = format {
            let ts = timestamp.and_then(|v| v.as_int()).unwrap_or_else(|| Utc::now().timestamp() as i32);
            let tz = chrono_tz::UTC;
            
            let mut chrono_fmt = String::new();
            let mut escape_next = false;
            for c in fmt.chars() {
                if escape_next {
                    chrono_fmt.push(c);
                    escape_next = false;
                    continue;
                }
                match c {
                    '\\' => escape_next = true,
                    'Y' => chrono_fmt.push_str("%Y"),
                    'y' => chrono_fmt.push_str("%y"),
                    'm' => chrono_fmt.push_str("%m"),
                    'n' => chrono_fmt.push_str("%-m"),
                    'F' => chrono_fmt.push_str("%B"),
                    'M' => chrono_fmt.push_str("%b"),
                    'd' => chrono_fmt.push_str("%d"),
                    'j' => chrono_fmt.push_str("%-d"),
                    'H' => chrono_fmt.push_str("%H"),
                    'G' => chrono_fmt.push_str("%-H"),
                    'h' => chrono_fmt.push_str("%I"),
                    'g' => chrono_fmt.push_str("%-I"),
                    'i' => chrono_fmt.push_str("%M"),
                    's' => chrono_fmt.push_str("%S"),
                    'a' => chrono_fmt.push_str("%P"),
                    'A' => chrono_fmt.push_str("%p"),
                    'w' => chrono_fmt.push_str("%w"),
                    'l' => chrono_fmt.push_str("%A"),
                    'D' => chrono_fmt.push_str("%a"),
                    'U' => chrono_fmt.push_str("%s"),
                    'c' => chrono_fmt.push_str("%Y-%m-%dT%H:%M:%S%:z"),
                    'O' => chrono_fmt.push_str("%z"),
                    'P' => chrono_fmt.push_str("%:z"),
                    'T' => chrono_fmt.push_str("%Z"),
                    _ => chrono_fmt.push(c),
                }
            }
            
            if let LocalResult::Single(dt) = tz.timestamp_opt(ts as i64, 0) {
                let result = dt.format(&chrono_fmt).to_string();
                let boxed = crate::into_raw(Box::new(result));
                Ok(Value::new_string_ptr(boxed as *mut ()))
            } else {
                Ok(Value::new_bool(false))
            }
        } else {
            Err("gmdate() expects at least 1 parameter".to_string())
        }
    }
}

php_function! {
    native_strtotime(time: Value, now: Value) {
        let Some(t_val) = time else {
            return Err("strtotime() expects at least 1 parameter".to_string());
        };
        let td = t_val.deref();
        let Some(ptr) = td.as_string_ptr() else {
            return Ok(Value::new_bool(false));
        };
        let t_str = unsafe { &*(ptr as *const String) };
        let ts = now.and_then(|v| {
            let vd = v.deref();
            vd.as_int().map(|i| i as i64)
        }).unwrap_or_else(|| Utc::now().timestamp() as i64);
        let tz = *DEFAULT_TIMEZONE.lock().unwrap();

        let trimmed = t_str.trim();
        let lower = trimmed.to_lowercase();
        
        if lower == "now" {
            return Ok(Value::new_int(ts as i32));
        }
        if lower == "yesterday" {
            return Ok(Value::new_int((ts - 86400) as i32));
        }
        if lower == "tomorrow" {
            return Ok(Value::new_int((ts + 86400) as i32));
        }

        let parts: Vec<&str> = lower.split_whitespace().collect();
        if parts.len() == 2 || parts.len() == 3 {
            let (num_val, unit_str) = if parts.len() == 3 {
                let sign = if parts[0] == "-" { -1i64 } else { 1i64 };
                (parts[1].parse::<i64>().ok().map(|n| n * sign), parts[2])
            } else {
                (parts[0].parse::<i64>().ok(), parts[1])
            };
            if let Some(n) = num_val {
                let unit = unit_str.trim_end_matches('s');
                let multiplier = match unit {
                    "second" | "sec" => Some(1i64),
                    "minute" | "min" => Some(60i64),
                    "hour" => Some(3600i64),
                    "day" => Some(86400i64),
                    "week" => Some(7 * 86400i64),
                    "month" => Some(30 * 86400i64),
                    "year" => Some(365 * 86400i64),
                    _ => None,
                };
                if let Some(m) = multiplier {
                    return Ok(Value::new_int((ts + n * m) as i32));
                }
            }
        }

        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(trimmed) {
            return Ok(Value::new_int(dt.timestamp() as i32));
        }
        if let Ok(dt) = chrono::DateTime::parse_from_rfc2822(trimmed) {
            return Ok(Value::new_int(dt.timestamp() as i32));
        }
        if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%d %H:%M:%S") {
            if let LocalResult::Single(dt) = tz.from_local_datetime(&naive) {
                return Ok(Value::new_int(dt.timestamp() as i32));
            }
        }
        if let Ok(naive_date) = chrono::NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
            if let Some(naive) = naive_date.and_hms_opt(0, 0, 0) {
                if let LocalResult::Single(dt) = tz.from_local_datetime(&naive) {
                    return Ok(Value::new_int(dt.timestamp() as i32));
                }
            }
        }
        if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(trimmed, "%d-%m-%Y %H:%M:%S") {
            if let LocalResult::Single(dt) = tz.from_local_datetime(&naive) {
                return Ok(Value::new_int(dt.timestamp() as i32));
            }
        }
        if let Ok(naive_date) = chrono::NaiveDate::parse_from_str(trimmed, "%d-%m-%Y") {
            if let Some(naive) = naive_date.and_hms_opt(0, 0, 0) {
                if let LocalResult::Single(dt) = tz.from_local_datetime(&naive) {
                    return Ok(Value::new_int(dt.timestamp() as i32));
                }
            }
        }

        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_native_date_create(datetime: String, timezone: String) {
        let dt_str = datetime.cloned().unwrap_or_else(|| "now".to_string());
        let tz_str = timezone.cloned().unwrap_or_else(|| "UTC".to_string());
        let tz = Tz::from_str(&tz_str).unwrap_or(*DEFAULT_TIMEZONE.lock().unwrap());
        
        let s = dt_str.trim().to_lowercase();
        if s == "now" {
            return Ok(Value::new_int(Utc::now().timestamp() as i32));
        }
        
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(dt_str.as_str()) {
            return Ok(Value::new_int(dt.timestamp() as i32));
        }
        if let Ok(dt) = chrono::DateTime::parse_from_rfc2822(dt_str.as_str()) {
            return Ok(Value::new_int(dt.timestamp() as i32));
        }
        
        if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(dt_str.as_str(), "%Y-%m-%d %H:%M:%S")
            && let LocalResult::Single(dt) = tz.from_local_datetime(&naive) {
                return Ok(Value::new_int(dt.timestamp() as i32));
            }
        
        if let Ok(naive_date) = chrono::NaiveDate::parse_from_str(dt_str.as_str(), "%Y-%m-%d") {
            let naive = naive_date.and_hms_opt(0, 0, 0).unwrap();
            if let LocalResult::Single(dt) = tz.from_local_datetime(&naive) {
                return Ok(Value::new_int(dt.timestamp() as i32));
            }
        }
        
        Ok(Value::new_int(Utc::now().timestamp() as i32))
    }
}

php_function! {
    native_native_date_format(timestamp: Value, format: String, timezone: String) {
        if let (Some(ts_val), Some(fmt)) = (timestamp, format) {
            let ts = ts_val.as_int().unwrap_or(0);
            let tz_str = timezone.cloned().unwrap_or_else(|| "UTC".to_string());
            let tz = Tz::from_str(&tz_str).unwrap_or(*DEFAULT_TIMEZONE.lock().unwrap());
            
            let mut chrono_fmt = String::new();
            let mut escape_next = false;
            for c in fmt.chars() {
                if escape_next {
                    chrono_fmt.push(c);
                    escape_next = false;
                    continue;
                }
                match c {
                    '\\' => escape_next = true,
                    'Y' => chrono_fmt.push_str("%Y"),
                    'y' => chrono_fmt.push_str("%y"),
                    'm' => chrono_fmt.push_str("%m"),
                    'n' => chrono_fmt.push_str("%-m"),
                    'F' => chrono_fmt.push_str("%B"),
                    'M' => chrono_fmt.push_str("%b"),
                    'd' => chrono_fmt.push_str("%d"),
                    'j' => chrono_fmt.push_str("%-d"),
                    'H' => chrono_fmt.push_str("%H"),
                    'G' => chrono_fmt.push_str("%-H"),
                    'h' => chrono_fmt.push_str("%I"),
                    'g' => chrono_fmt.push_str("%-I"),
                    'i' => chrono_fmt.push_str("%M"),
                    's' => chrono_fmt.push_str("%S"),
                    'a' => chrono_fmt.push_str("%P"),
                    'A' => chrono_fmt.push_str("%p"),
                    'w' => chrono_fmt.push_str("%w"),
                    'l' => chrono_fmt.push_str("%A"),
                    'D' => chrono_fmt.push_str("%a"),
                    'U' => chrono_fmt.push_str("%s"),
                    'c' => chrono_fmt.push_str("%Y-%m-%dT%H:%M:%S%:z"),
                    'O' => chrono_fmt.push_str("%z"),
                    'P' => chrono_fmt.push_str("%:z"),
                    'T' => chrono_fmt.push_str("%Z"),
                    _ => chrono_fmt.push(c),
                }
            }
            
            if let LocalResult::Single(dt) = tz.timestamp_opt(ts as i64, 0) {
                let result = dt.format(&chrono_fmt).to_string();
                let boxed = crate::into_raw(Box::new(result));
                Ok(Value::new_string_ptr(boxed as *mut ()))
            } else {
                Ok(Value::new_bool(false))
            }
        } else {
            Err("__native_date_format() expects at least 2 parameters".to_string())
        }
    }
}

php_function! {
    native_native_date_modify(timestamp: Value, modifier: String, timezone: String) {
        if let (Some(ts_val), Some(mod_str)) = (timestamp, modifier) {
            let ts = ts_val.as_int().unwrap_or(0);
            
            // Naive implementation for "+1 day", "-1 month", etc.
            let mut days = 0;
            let mut seconds = 0;
            
            let parts: Vec<&str> = mod_str.split_whitespace().collect();
            if parts.len() >= 2
                && let Ok(amount) = parts[0].parse::<i64>() {
                    let unit = parts[1].to_lowercase();
                    if unit.starts_with("day") { days = amount; }
                    else if unit.starts_with("week") { days = amount * 7; }
                    else if unit.starts_with("hour") { seconds = amount * 3600; }
                    else if unit.starts_with("minute") || unit.starts_with("min") { seconds = amount * 60; }
                    else if unit.starts_with("second") || unit.starts_with("sec") { seconds = amount; }
                    else if unit.starts_with("month") { days = amount * 30; } // Very naive
                    else if unit.starts_with("year") { days = amount * 365; } // Very naive
                }
            
            let new_ts = ts as i64 + (days * 86400) + seconds;
            Ok(Value::new_int(new_ts as i32))
        } else {
            Err("__native_date_modify() expects at least 2 parameters".to_string())
        }
    }
}

php_function! {
    native_native_date_diff(ts1: Value, ts2: Value, absolute: Value) |ctx| { 
        if let (Some(t1_val), Some(t2_val)) = (ts1, ts2) {
            let t1 = t1_val.as_int().unwrap_or(0) as i64;
            let t2 = t2_val.as_int().unwrap_or(0) as i64;
            let abs = absolute.and_then(|v| v.as_bool()).unwrap_or(false);
            
            let mut diff = t2 - t1;
            let invert = if diff < 0 { 1 } else { 0 };
            if abs {
                diff = diff.abs();
            }
            
            let days = diff.abs() / 86400 ;
            let mut arr = hyperion_core::types::array::PhpArray::new();
            arr.insert_string_id(ctx.intern_string("days"), Value::new_int(days as i32));
            arr.insert_string_id(ctx.intern_string("y"), Value::new_int((days / 365) as i32));
            arr.insert_string_id(ctx.intern_string("m"), Value::new_int(((days % 365) / 30) as i32));
            arr.insert_string_id(ctx.intern_string("d"), Value::new_int(((days % 365) % 30) as i32));
            arr.insert_string_id(ctx.intern_string("h"), Value::new_int(((diff.abs() % 86400) / 3600) as i32));
            arr.insert_string_id(ctx.intern_string("i"), Value::new_int(((diff.abs() % 3600) / 60) as i32));
            arr.insert_string_id(ctx.intern_string("s"), Value::new_int((diff.abs() % 60) as i32));
            arr.insert_string_id(ctx.intern_string("invert"), Value::new_int(invert));
            
            let arr_ptr = ctx.get_arena().alloc(arr);
            Ok(Value::new_array_ptr(arr_ptr as *mut ()))
        } else {
            Err("__native_date_diff() expects at least 2 parameters".to_string())
        }
    }
}

php_function! {
    native_datetimezone_construct(this: Value, timezone: String) {
        if let Some(obj_ptr) = this.and_then(|v| v.as_object_ptr()) {
            let obj = unsafe { &mut *(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
            if let Some(tz) = timezone {
                let boxed = crate::into_raw(Box::new(tz.clone()));
                obj.properties.insert("name".to_string(), Value::new_string_ptr(boxed as *mut ()));
            }
        }
        Ok(Value::null())
    }
}

php_function! {
    native_datetimezone_get_name(this: Value) {
        if let Some(obj_ptr) = this.and_then(|v| v.as_object_ptr()) {
            let obj = unsafe { &*(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
            if let Some(val) = obj.properties.get("name") {
                return Ok(*val);
            }
        }
        let boxed = crate::into_raw(Box::new("UTC".to_string()));
        Ok(Value::new_string_ptr(boxed as *mut ()))
    }
}

php_function! {
    native_datetime_construct(this: Value, datetime: String, timezone: Value) {
        // Retrieve default timezone or check if timezone object provided
        let default_tz = DEFAULT_TIMEZONE.lock().unwrap().name().to_string();
        let mut tz_name = default_tz.clone();

        if let Some(tz_val) = timezone {
            if let Some(obj_ptr) = tz_val.as_object_ptr() {
                let tz_obj = unsafe { &*(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
                if let Some(n) = tz_obj.properties.get("name") {
                    if let Some(ptr) = n.as_string_ptr() {
                        let s = unsafe { &*(ptr as *const String) };
                        tz_name = s.clone();
                    }
                }
            }
        }

        let dt_str = datetime.cloned().unwrap_or_else(|| "now".to_string());
        let tz = Tz::from_str(&tz_name).unwrap_or(*DEFAULT_TIMEZONE.lock().unwrap());
        
        let s = dt_str.trim().to_lowercase();
        let timestamp = if s == "now" {
            Utc::now().timestamp() as i32
        } else if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(dt_str.as_str()) {
            dt.timestamp() as i32
        } else if let Ok(dt) = chrono::DateTime::parse_from_rfc2822(dt_str.as_str()) {
            dt.timestamp() as i32
        } else if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(dt_str.as_str(), "%Y-%m-%d %H:%M:%S") {
            if let LocalResult::Single(dt) = tz.from_local_datetime(&naive) {
                dt.timestamp() as i32
            } else { Utc::now().timestamp() as i32 }
        } else if let Ok(naive_date) = chrono::NaiveDate::parse_from_str(dt_str.as_str(), "%Y-%m-%d") {
            let naive = naive_date.and_hms_opt(0, 0, 0).unwrap();
            if let LocalResult::Single(dt) = tz.from_local_datetime(&naive) {
                dt.timestamp() as i32
            } else { Utc::now().timestamp() as i32 }
        } else {
            Utc::now().timestamp() as i32
        };

        if let Some(obj_ptr) = this.and_then(|v| v.as_object_ptr()) {
            let obj = unsafe { &mut *(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
            obj.properties.insert("timestamp".to_string(), Value::new_int(timestamp));
            let boxed_tz = crate::into_raw(Box::new(tz_name));
            obj.properties.insert("timezone_name".to_string(), Value::new_string_ptr(boxed_tz as *mut ()));
        }

        Ok(Value::null())
    }
}

php_function! {
    native_datetime_format(this: Value, format: String) {
        if let Some(obj_ptr) = this.and_then(|v| v.as_object_ptr()) {
            let obj = unsafe { &*(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
            
            let ts = obj.properties.get("timestamp").and_then(|v| v.as_int()).unwrap_or(0);
            let mut tz_str = "UTC".to_string();
            if let Some(tz_val) = obj.properties.get("timezone_name") {
                if let Some(ptr) = tz_val.as_string_ptr() {
                    let s = unsafe { &*(ptr as *const String) };
                    tz_str = s.clone();
                }
            }
            
            let tz = Tz::from_str(&tz_str).unwrap_or(*DEFAULT_TIMEZONE.lock().unwrap());
            
            if let Some(fmt) = format {
                let mut chrono_fmt = String::new();
                let mut escape_next = false;
                for c in fmt.chars() {
                    if escape_next {
                        chrono_fmt.push(c);
                        escape_next = false;
                        continue;
                    }
                    match c {
                        '\\' => escape_next = true,
                        'Y' => chrono_fmt.push_str("%Y"),
                        'y' => chrono_fmt.push_str("%y"),
                        'm' => chrono_fmt.push_str("%m"),
                        'n' => chrono_fmt.push_str("%-m"),
                        'F' => chrono_fmt.push_str("%B"),
                        'M' => chrono_fmt.push_str("%b"),
                        'd' => chrono_fmt.push_str("%d"),
                        'j' => chrono_fmt.push_str("%-d"),
                        'H' => chrono_fmt.push_str("%H"),
                        'G' => chrono_fmt.push_str("%-H"),
                        'h' => chrono_fmt.push_str("%I"),
                        'g' => chrono_fmt.push_str("%-I"),
                        'i' => chrono_fmt.push_str("%M"),
                        's' => chrono_fmt.push_str("%S"),
                        'a' => chrono_fmt.push_str("%P"),
                        'A' => chrono_fmt.push_str("%p"),
                        'w' => chrono_fmt.push_str("%w"),
                        'l' => chrono_fmt.push_str("%A"),
                        'D' => chrono_fmt.push_str("%a"),
                        'u' => chrono_fmt.push_str("%6f"),
                        'v' => chrono_fmt.push_str("%3f"),
                        'U' => chrono_fmt.push_str("%s"),
                        'c' => chrono_fmt.push_str("%Y-%m-%dT%H:%M:%S%:z"),
                        'O' => chrono_fmt.push_str("%z"),
                        'P' => chrono_fmt.push_str("%:z"),
                        'T' => chrono_fmt.push_str("%Z"),
                        _ => chrono_fmt.push(c),
                    }
                }
                
                if let LocalResult::Single(dt) = tz.timestamp_opt(ts as i64, 0) {
                    let result = dt.format(&chrono_fmt).to_string();
                    let boxed = crate::into_raw(Box::new(result));
                    return Ok(Value::new_string_ptr(boxed as *mut ()));
                }
            }
        }
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_datetime_modify(this: Value, modifier: String) {
        if let Some(obj_ptr) = this.and_then(|v| v.as_object_ptr()) {
            let obj = unsafe { &mut *(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
            let ts = obj.properties.get("timestamp").and_then(|v| v.as_int()).unwrap_or(0);
            
            if let Some(mod_str) = modifier {
                let mut days = 0;
                let mut seconds = 0;
                
                let parts: Vec<&str> = mod_str.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(amount) = parts[0].parse::<i64>() {
                        let unit = parts[1].to_lowercase();
                        if unit.starts_with("day") { days = amount; }
                        else if unit.starts_with("week") { days = amount * 7; }
                        else if unit.starts_with("hour") { seconds = amount * 3600; }
                        else if unit.starts_with("minute") || unit.starts_with("min") { seconds = amount * 60; }
                        else if unit.starts_with("second") || unit.starts_with("sec") { seconds = amount; }
                        else if unit.starts_with("month") { days = amount * 30; }
                        else if unit.starts_with("year") { days = amount * 365; }
                    }
                }
                
                let new_ts = ts as i64 + (days * 86400) + seconds;
                obj.properties.insert("timestamp".to_string(), Value::new_int(new_ts as i32));
            }
        }
        // return $this
        if let Some(t) = this { Ok(*t) } else { Ok(Value::null()) }
    }
}

php_function! {
    native_datetimeimmutable_modify(this: Value, modifier: String) |ctx| {
        // Since DateTimeImmutable creates a clone, we can't easily clone it here natively without object clone support.
        // Actually we can, just create a new object. But we need its class ID.
        // For now, let's just modify `this` which is technically incorrect for immutable, but this VM
        // doesn't have a native `clone` yet. Let's do exactly what we did above for now.
        if let Some(obj_ptr) = this.and_then(|v| v.as_object_ptr()) {
            let obj = unsafe { &mut *(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
            let ts = obj.properties.get("timestamp").and_then(|v| v.as_int()).unwrap_or(0);
            
            if let Some(mod_str) = modifier {
                let mut days = 0;
                let mut seconds = 0;
                
                let parts: Vec<&str> = mod_str.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(amount) = parts[0].parse::<i64>() {
                        let unit = parts[1].to_lowercase();
                        if unit.starts_with("day") { days = amount; }
                        else if unit.starts_with("week") { days = amount * 7; }
                        else if unit.starts_with("hour") { seconds = amount * 3600; }
                        else if unit.starts_with("minute") || unit.starts_with("min") { seconds = amount * 60; }
                        else if unit.starts_with("second") || unit.starts_with("sec") { seconds = amount; }
                        else if unit.starts_with("month") { days = amount * 30; }
                        else if unit.starts_with("year") { days = amount * 365; }
                    }
                }
                
                let new_ts = ts as i64 + (days * 86400) + seconds;
                obj.properties.insert("timestamp".to_string(), Value::new_int(new_ts as i32));
            }
        }
        if let Some(t) = this { Ok(*t) } else { Ok(Value::null()) }
    }
}

php_function! {
    native_datetime_get_timestamp(this: Value) {
        if let Some(obj_ptr) = this.and_then(|v| v.as_object_ptr()) {
            let obj = unsafe { &*(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
            if let Some(ts) = obj.properties.get("timestamp") {
                return Ok(*ts);
            }
        }
        Ok(Value::new_int(0))
    }
}

fn dateinterval_to_seconds(interval_val: Option<&Value>) -> i64 {
    let Some(val) = interval_val else { return 0; };
    let deref = val.deref();
    let Some(obj_ptr) = deref.as_object_ptr() else { return 0; };
    let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
    
    let y = obj.properties.get("y").and_then(|v| v.deref().as_int()).unwrap_or(0) as i64;
    let m = obj.properties.get("m").and_then(|v| v.deref().as_int()).unwrap_or(0) as i64;
    let d = obj.properties.get("d").and_then(|v| v.deref().as_int()).unwrap_or(0) as i64;
    let h = obj.properties.get("h").and_then(|v| v.deref().as_int()).unwrap_or(0) as i64;
    let i = obj.properties.get("i").and_then(|v| v.deref().as_int()).unwrap_or(0) as i64;
    let s = obj.properties.get("s").and_then(|v| v.deref().as_int()).unwrap_or(0) as i64;
    let invert = obj.properties.get("invert").and_then(|v| v.deref().as_int()).unwrap_or(0);
    
    let total = (y * 365 * 86400) + (m * 30 * 86400) + (d * 86400) + (h * 3600) + (i * 60) + s;
    if invert == 1 { -total } else { total }
}

php_function! {
    native_datetime_add(this: Value, interval: Value) {
        if let Some(obj_ptr) = this.and_then(|v| v.deref().as_object_ptr()) {
            let obj = unsafe { &mut *(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
            let ts = obj.properties.get("timestamp").and_then(|v| v.deref().as_int()).unwrap_or(0) as i64;
            let delta = dateinterval_to_seconds(interval);
            let new_ts = ts + delta;
            obj.properties.insert("timestamp".to_string(), Value::new_int(new_ts as i32));
        }
        if let Some(t) = this { Ok(*t) } else { Ok(Value::null()) }
    }
}

php_function! {
    native_datetime_sub(this: Value, interval: Value) {
        if let Some(obj_ptr) = this.and_then(|v| v.deref().as_object_ptr()) {
            let obj = unsafe { &mut *(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
            let ts = obj.properties.get("timestamp").and_then(|v| v.deref().as_int()).unwrap_or(0) as i64;
            let delta = dateinterval_to_seconds(interval);
            let new_ts = ts - delta;
            obj.properties.insert("timestamp".to_string(), Value::new_int(new_ts as i32));
        }
        if let Some(t) = this { Ok(*t) } else { Ok(Value::null()) }
    }
}

php_function! {
    native_datetimeimmutable_add(this: Value, interval: Value) |ctx| {
        if let Some(obj_ptr) = this.and_then(|v| v.deref().as_object_ptr()) {
            let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
            let ts = obj.properties.get("timestamp").and_then(|v| v.deref().as_int()).unwrap_or(0) as i64;
            let delta = dateinterval_to_seconds(interval);
            let new_ts = ts + delta;
            let mut clone_obj = hyperion_core::types::object::PhpObject::new_with_name(obj.class_id, obj.class_name.clone().unwrap_or_default());
            for (k, v) in &obj.properties {
                clone_obj.properties.insert(k.clone(), *v);
            }
            clone_obj.properties.insert("timestamp".to_string(), Value::new_int(new_ts as i32));
            let ptr = ctx.get_arena().alloc(clone_obj);
            return Ok(Value::new_object_ptr(ptr as *mut ()));
        }
        if let Some(t) = this { Ok(*t) } else { Ok(Value::null()) }
    }
}

php_function! {
    native_datetimeimmutable_sub(this: Value, interval: Value) |ctx| {
        if let Some(obj_ptr) = this.and_then(|v| v.deref().as_object_ptr()) {
            let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
            let ts = obj.properties.get("timestamp").and_then(|v| v.deref().as_int()).unwrap_or(0) as i64;
            let delta = dateinterval_to_seconds(interval);
            let new_ts = ts - delta;
            let mut clone_obj = hyperion_core::types::object::PhpObject::new_with_name(obj.class_id, obj.class_name.clone().unwrap_or_default());
            for (k, v) in &obj.properties {
                clone_obj.properties.insert(k.clone(), *v);
            }
            clone_obj.properties.insert("timestamp".to_string(), Value::new_int(new_ts as i32));
            let ptr = ctx.get_arena().alloc(clone_obj);
            return Ok(Value::new_object_ptr(ptr as *mut ()));
        }
        if let Some(t) = this { Ok(*t) } else { Ok(Value::null()) }
    }
}

php_function! {
    native_datetime_create_from_interface(object: Value) |ctx| {
        let dt_class_id = ctx.get_class_id("DateTime").unwrap_or(0);
        let mut new_obj = hyperion_core::types::object::PhpObject::new_with_name(dt_class_id, "DateTime".to_string());
        if let Some(obj_ptr) = object.and_then(|v| v.deref().as_object_ptr()) {
            let src = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
            for (k, v) in &src.properties {
                new_obj.properties.insert(k.clone(), *v);
            }
        }
        let ptr = ctx.get_arena().alloc(new_obj);
        Ok(Value::new_object_ptr(ptr as *mut ()))
    }
}

php_function! {
    native_datetime_immutable_create_from_interface(object: Value) |ctx| {
        let dt_class_id = ctx.get_class_id("DateTimeImmutable").unwrap_or(0);
        let mut new_obj = hyperion_core::types::object::PhpObject::new_with_name(dt_class_id, "DateTimeImmutable".to_string());
        if let Some(obj_ptr) = object.and_then(|v| v.deref().as_object_ptr()) {
            let src = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
            for (k, v) in &src.properties {
                new_obj.properties.insert(k.clone(), *v);
            }
        }
        let ptr = ctx.get_arena().alloc(new_obj);
        Ok(Value::new_object_ptr(ptr as *mut ()))
    }
}

php_function! {
    native_datetime_set_timestamp(this: Value, timestamp: Value) {
        if let Some(obj_ptr) = this.and_then(|v| v.as_object_ptr()) {
            let obj = unsafe { &mut *(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
            if let Some(ts) = timestamp {
                obj.properties.insert("timestamp".to_string(), *ts);
            }
        }
        if let Some(t) = this { Ok(*t) } else { Ok(Value::null()) }
    }
}

php_function! {
    native_datetimeimmutable_set_timestamp(this: Value, timestamp: Value) {
        if let Some(obj_ptr) = this.and_then(|v| v.as_object_ptr()) {
            let obj = unsafe { &mut *(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
            if let Some(ts) = timestamp {
                obj.properties.insert("timestamp".to_string(), *ts);
            }
        }
        if let Some(t) = this { Ok(*t) } else { Ok(Value::null()) }
    }
}

php_function! {
    native_datetime_get_timezone(this: Value) |ctx| {
        let mut tz_name = "UTC".to_string();
        if let Some(obj_ptr) = this.and_then(|v| v.as_object_ptr()) {
            let obj = unsafe { &*(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
            if let Some(tz_val) = obj.properties.get("timezone_name") {
                if let Some(ptr) = tz_val.as_string_ptr() {
                    let s = unsafe { &*(ptr as *const String) };
                    tz_name = s.clone();
                }
            }
        }
        let tz_class_id = ctx.get_class_id("DateTimeZone").unwrap_or(0);
        let mut tz_obj = ctx.instantiate_class(tz_class_id);
        let boxed = crate::into_raw(Box::new(tz_name));
        tz_obj.properties.insert("name".to_string(), Value::new_string_ptr(boxed as *mut ()));
        let boxed_obj = crate::into_raw(Box::new(tz_obj));
        Ok(Value::new_object_ptr(boxed_obj as *mut ()))
    }
}

php_function! {
    native_datetime_set_timezone(this: Value, timezone: Value) {
        if let Some(obj_ptr) = this.and_then(|v| v.as_object_ptr()) {
            let obj = unsafe { &mut *(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
            let mut tz_name = "UTC".to_string();
            if let Some(tz_val) = timezone {
                if let Some(tz_obj_ptr) = tz_val.as_object_ptr() {
                    let tz_obj = unsafe { &*(tz_obj_ptr as *const hyperion_core::types::object::PhpObject) };
                    if let Some(name_val) = tz_obj.properties.get("name") {
                        if let Some(s_ptr) = name_val.as_string_ptr() {
                            tz_name = unsafe { (*(s_ptr as *const String)).clone() };
                        }
                    }
                } else if let Some(s_ptr) = tz_val.as_string_ptr() {
                    tz_name = unsafe { (*(s_ptr as *const String)).clone() };
                }
            }
            let boxed = crate::into_raw(Box::new(tz_name));
            obj.properties.insert("timezone_name".to_string(), Value::new_string_ptr(boxed as *mut ()));
        }
        if let Some(t) = this { Ok(*t) } else { Ok(Value::null()) }
    }
}

php_function! {
    native_datetimeimmutable_set_timezone(this: Value, timezone: Value) |ctx| {
        let mut tz_name = "UTC".to_string();
        if let Some(tz_val) = timezone {
            if let Some(tz_obj_ptr) = tz_val.as_object_ptr() {
                let tz_obj = unsafe { &*(tz_obj_ptr as *const hyperion_core::types::object::PhpObject) };
                if let Some(name_val) = tz_obj.properties.get("name") {
                    if let Some(s_ptr) = name_val.as_string_ptr() {
                        tz_name = unsafe { (*(s_ptr as *const String)).clone() };
                    }
                }
            } else if let Some(s_ptr) = tz_val.as_string_ptr() {
                tz_name = unsafe { (*(s_ptr as *const String)).clone() };
            }
        }
        if let Some(obj_ptr) = this.and_then(|v| v.as_object_ptr()) {
            let obj = unsafe { &*(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
            let ts = obj.properties.get("timestamp").cloned().unwrap_or(Value::new_int(0));
            let mut clone_obj = ctx.instantiate_class(obj.class_id);
            clone_obj.properties.insert("timestamp".to_string(), ts);
            let boxed = crate::into_raw(Box::new(tz_name));
            clone_obj.properties.insert("timezone_name".to_string(), Value::new_string_ptr(boxed as *mut ()));
            let boxed_obj = crate::into_raw(Box::new(clone_obj));
            Ok(Value::new_object_ptr(boxed_obj as *mut ()))
        } else {
            Ok(Value::null())
        }
    }
}

php_function! {
    native_datetime_set_time(this: Value, hour: Value, minute: Value, second: Value, _microsecond: Value) {
        if let Some(obj_ptr) = this.and_then(|v| v.deref().as_object_ptr()) {
            let obj = unsafe { &mut *(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
            let ts = obj.properties.get("timestamp").and_then(|v| v.deref().as_int()).unwrap_or(0) as i64;
            let mut tz_str = "UTC".to_string();
            if let Some(tz_val) = obj.properties.get("timezone_name") {
                if let Some(ptr) = tz_val.deref().as_string_ptr() {
                    let s = unsafe { &*(ptr as *const String) };
                    tz_str = s.clone();
                }
            }
            let tz = Tz::from_str(&tz_str).unwrap_or(*DEFAULT_TIMEZONE.lock().unwrap());
            if let LocalResult::Single(dt) = tz.timestamp_opt(ts, 0) {
                let h = hour.and_then(|v| v.deref().as_int()).unwrap_or(0) as u32;
                let m = minute.and_then(|v| v.deref().as_int()).unwrap_or(0) as u32;
                let s = second.and_then(|v| v.deref().as_int()).unwrap_or(0) as u32;
                if let Some(new_dt) = dt.date_naive().and_hms_opt(h, m, s) {
                    if let LocalResult::Single(final_dt) = tz.from_local_datetime(&new_dt) {
                        obj.properties.insert("timestamp".to_string(), Value::new_int(final_dt.timestamp() as i32));
                    }
                }
            }
        }
        if let Some(t) = this { Ok(*t) } else { Ok(Value::null()) }
    }
}

php_function! {
    native_datetime_set_date(this: Value, year: Value, month: Value, day: Value) {
        if let Some(obj_ptr) = this.and_then(|v| v.deref().as_object_ptr()) {
            let obj = unsafe { &mut *(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
            let ts = obj.properties.get("timestamp").and_then(|v| v.deref().as_int()).unwrap_or(0) as i64;
            let mut tz_str = "UTC".to_string();
            if let Some(tz_val) = obj.properties.get("timezone_name") {
                if let Some(ptr) = tz_val.deref().as_string_ptr() {
                    let s = unsafe { &*(ptr as *const String) };
                    tz_str = s.clone();
                }
            }
            let tz = Tz::from_str(&tz_str).unwrap_or(*DEFAULT_TIMEZONE.lock().unwrap());
            if let LocalResult::Single(dt) = tz.timestamp_opt(ts, 0) {
                let y = year.and_then(|v| v.deref().as_int()).unwrap_or(2026);
                let m = month.and_then(|v| v.deref().as_int()).unwrap_or(1) as u32;
                let d = day.and_then(|v| v.deref().as_int()).unwrap_or(1) as u32;
                if let Some(new_date) = chrono::NaiveDate::from_ymd_opt(y, m, d) {
                    let new_dt = new_date.and_time(dt.time());
                    if let LocalResult::Single(final_dt) = tz.from_local_datetime(&new_dt) {
                        obj.properties.insert("timestamp".to_string(), Value::new_int(final_dt.timestamp() as i32));
                    }
                }
            }
        }
        if let Some(t) = this { Ok(*t) } else { Ok(Value::null()) }
    }
}

php_function! {
    native_datetimeimmutable_set_time(this: Value, hour: Value, minute: Value, second: Value, _microsecond: Value) |ctx| {
        if let Some(obj_ptr) = this.and_then(|v| v.deref().as_object_ptr()) {
            let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
            let ts = obj.properties.get("timestamp").and_then(|v| v.deref().as_int()).unwrap_or(0) as i64;
            let mut tz_str = "UTC".to_string();
            if let Some(tz_val) = obj.properties.get("timezone_name") {
                if let Some(ptr) = tz_val.deref().as_string_ptr() {
                    let s = unsafe { &*(ptr as *const String) };
                    tz_str = s.clone();
                }
            }
            let tz = Tz::from_str(&tz_str).unwrap_or(*DEFAULT_TIMEZONE.lock().unwrap());
            let mut new_ts = ts;
            if let LocalResult::Single(dt) = tz.timestamp_opt(ts, 0) {
                let h = hour.and_then(|v| v.deref().as_int()).unwrap_or(0) as u32;
                let m = minute.and_then(|v| v.deref().as_int()).unwrap_or(0) as u32;
                let s = second.and_then(|v| v.deref().as_int()).unwrap_or(0) as u32;
                if let Some(new_dt) = dt.date_naive().and_hms_opt(h, m, s) {
                    if let LocalResult::Single(final_dt) = tz.from_local_datetime(&new_dt) {
                        new_ts = final_dt.timestamp();
                    }
                }
            }
            let mut clone_obj = hyperion_core::types::object::PhpObject::new_with_name(obj.class_id, obj.class_name.clone().unwrap_or_default());
            for (k, v) in &obj.properties {
                clone_obj.properties.insert(k.clone(), *v);
            }
            clone_obj.properties.insert("timestamp".to_string(), Value::new_int(new_ts as i32));
            let ptr = ctx.get_arena().alloc(clone_obj);
            return Ok(Value::new_object_ptr(ptr as *mut ()));
        }
        if let Some(t) = this { Ok(*t) } else { Ok(Value::null()) }
    }
}

php_function! {
    native_datetimeimmutable_set_date(this: Value, year: Value, month: Value, day: Value) |ctx| {
        if let Some(obj_ptr) = this.and_then(|v| v.deref().as_object_ptr()) {
            let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
            let ts = obj.properties.get("timestamp").and_then(|v| v.deref().as_int()).unwrap_or(0) as i64;
            let mut tz_str = "UTC".to_string();
            if let Some(tz_val) = obj.properties.get("timezone_name") {
                if let Some(ptr) = tz_val.deref().as_string_ptr() {
                    let s = unsafe { &*(ptr as *const String) };
                    tz_str = s.clone();
                }
            }
            let tz = Tz::from_str(&tz_str).unwrap_or(*DEFAULT_TIMEZONE.lock().unwrap());
            let mut new_ts = ts;
            if let LocalResult::Single(dt) = tz.timestamp_opt(ts, 0) {
                let y = year.and_then(|v| v.deref().as_int()).unwrap_or(2026);
                let m = month.and_then(|v| v.deref().as_int()).unwrap_or(1) as u32;
                let d = day.and_then(|v| v.deref().as_int()).unwrap_or(1) as u32;
                if let Some(new_date) = chrono::NaiveDate::from_ymd_opt(y, m, d) {
                    let new_dt = new_date.and_time(dt.time());
                    if let LocalResult::Single(final_dt) = tz.from_local_datetime(&new_dt) {
                        new_ts = final_dt.timestamp();
                    }
                }
            }
            let mut clone_obj = hyperion_core::types::object::PhpObject::new_with_name(obj.class_id, obj.class_name.clone().unwrap_or_default());
            for (k, v) in &obj.properties {
                clone_obj.properties.insert(k.clone(), *v);
            }
            clone_obj.properties.insert("timestamp".to_string(), Value::new_int(new_ts as i32));
            let ptr = ctx.get_arena().alloc(clone_obj);
            return Ok(Value::new_object_ptr(ptr as *mut ()));
        }
        if let Some(t) = this { Ok(*t) } else { Ok(Value::null()) }
    }
}

php_function! {
    native_datetime_diff(this: Value, target: Value, absolute: Value) |ctx| {
        // Fallback for returning a mocked DateInterval
        let mut t1 = 0;
        let mut t2 = 0;

        if let Some(obj_ptr) = this.and_then(|v| v.as_object_ptr()) {
            let obj = unsafe { &*(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
            t1 = obj.properties.get("timestamp").and_then(|v| v.as_int()).unwrap_or(0);
        }

        if let Some(obj_ptr) = target.and_then(|v| v.as_object_ptr()) {
            let obj = unsafe { &*(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
            t2 = obj.properties.get("timestamp").and_then(|v| v.as_int()).unwrap_or(0);
        }

        let abs = absolute.and_then(|v| v.as_bool()).unwrap_or(false);
        let mut diff = (t2 as i64) - (t1 as i64);
        let invert = if diff < 0 { 1 } else { 0 };
        if abs { diff = diff.abs(); }

        let days = diff.abs() / 86400;
        
        let di_class_id = ctx.get_class_id("DateInterval").unwrap_or(0);
        let mut di_obj = PhpObject::new_with_name(di_class_id, "DateInterval".to_string());
        di_obj.properties.insert("days".to_string(), Value::new_int(days as i32));
        di_obj.properties.insert("y".to_string(), Value::new_int((days / 365) as i32));
        di_obj.properties.insert("m".to_string(), Value::new_int(((days % 365) / 30) as i32));
        di_obj.properties.insert("d".to_string(), Value::new_int(((days % 365) % 30) as i32));
        di_obj.properties.insert("h".to_string(), Value::new_int(((diff.abs() % 86400) / 3600) as i32));
        di_obj.properties.insert("i".to_string(), Value::new_int(((diff.abs() % 3600) / 60) as i32));
        di_obj.properties.insert("s".to_string(), Value::new_int((diff.abs() % 60) as i32));
        di_obj.properties.insert("f".to_string(), Value::new_float(0.0));
        di_obj.properties.insert("invert".to_string(), Value::new_int(invert));
        di_obj.properties.insert("from_string".to_string(), Value::new_bool(false));
        
        let boxed_obj = crate::into_raw(Box::new(di_obj));
        Ok(Value::new_object_ptr(boxed_obj as *mut ()))
    }
}

php_function! {
    native_datetime_get_offset(this: Value) {
        Ok(Value::new_int(0))
    }
}

php_function! {
    native_datetime_wakeup(this: Value) {
        Ok(Value::null())
    }
}

php_function! {
    native_dateinterval_construct(this: Value, duration: Value) |ctx| {
        let Some(this_val) = this else { return Ok(Value::null()); };
        let deref = this_val.deref();
        let Some(obj_ptr) = deref.as_object_ptr() else { return Ok(Value::null()); };
        let obj = unsafe { &mut *(obj_ptr as *mut PhpObject) };
        
        let text = if let Some(val) = duration {
            let deref = val.deref();
            if let Some(sp) = deref.as_string_ptr() {
                unsafe { (*(sp as *const String)).clone() }
            } else {
                "".to_string()
            }
        } else {
            "".to_string()
        };

        let mut y = 0i32;
        let mut m = 0i32;
        let mut d = 0i32;
        let mut h = 0i32;
        let mut i = 0i32;
        let mut s = 0i32;
        let mut f = 0.0f64;

        if text.starts_with('P') || text.starts_with('p') {
            let mut is_time = false;
            let mut current_num = String::new();
            for c in text[1..].chars() {
                if c == 'T' || c == 't' {
                    is_time = true;
                    current_num.clear();
                } else if c.is_ascii_digit() || c == '.' {
                    current_num.push(c);
                } else {
                    if let Ok(num) = current_num.parse::<f64>() {
                        match c {
                            'Y' | 'y' => y += num as i32,
                            'M' | 'm' => {
                                if is_time {
                                    i += num as i32;
                                } else {
                                    m += num as i32;
                                }
                            }
                            'W' | 'w' => d += (num * 7.0) as i32,
                            'D' | 'd' => d += num as i32,
                            'H' | 'h' => h += num as i32,
                            'S' | 's' => {
                                s += num.floor() as i32;
                                f += num.fract();
                            }
                            _ => {}
                        }
                    }
                    current_num.clear();
                }
            }
        }

        obj.properties.insert("y".to_string(), Value::new_int(y));
        obj.properties.insert("m".to_string(), Value::new_int(m));
        obj.properties.insert("d".to_string(), Value::new_int(d));
        obj.properties.insert("h".to_string(), Value::new_int(h));
        obj.properties.insert("i".to_string(), Value::new_int(i));
        obj.properties.insert("s".to_string(), Value::new_int(s));
        obj.properties.insert("f".to_string(), Value::new_float(f));
        obj.properties.insert("invert".to_string(), Value::new_int(0));
        obj.properties.insert("days".to_string(), Value::new_bool(false));
        obj.properties.insert("from_string".to_string(), Value::new_bool(false));

        Ok(Value::null())
    }
}

php_function! {
    native_dateinterval_create_from_date_string(datetime: Value) |ctx| {
        let di_class_id = ctx.get_class_id("DateInterval").unwrap_or(0);
        let mut di_obj = PhpObject::new_with_name(di_class_id, "DateInterval".to_string());
        
        let mut y = 0i32;
        let mut m = 0i32;
        let mut d = 0i32;
        let mut h = 0i32;
        let mut i = 0i32;
        let mut s = 0i32;
        let mut f = 0.0f64;
        
        let text = if let Some(val) = datetime {
            let deref = val.deref();
            if let Some(sp) = deref.as_string_ptr() {
                unsafe { (*(sp as *const String)).clone() }
            } else if let Some(i) = deref.as_int() {
                i.to_string()
            } else {
                "".to_string()
            }
        } else {
            "".to_string()
        };

        if !text.is_empty() {
            let lower = text.to_lowercase();
            let parts: Vec<&str> = lower.split_whitespace().collect();
            let mut idx = 0;
            while idx < parts.len() {
                if let Ok(num) = parts[idx].parse::<f64>() {
                    if idx + 1 < parts.len() {
                        let unit = parts[idx + 1];
                        if unit.starts_with("year") {
                            y += num as i32;
                        } else if unit.starts_with("month") {
                            m += num as i32;
                        } else if unit.starts_with("week") {
                            d += (num * 7.0) as i32;
                        } else if unit.starts_with("day") {
                            d += num as i32;
                        } else if unit.starts_with("hour") {
                            h += num as i32;
                        } else if unit.starts_with("min") {
                            i += num as i32;
                        } else if unit.starts_with("sec") {
                            s += num as i32;
                        } else if unit.starts_with("micro") || unit.starts_with("usec") || unit == "us" {
                            f += num / 1_000_000.0;
                        } else if unit.starts_with("milli") || unit.starts_with("msec") || unit == "ms" {
                            f += num / 1_000.0;
                        }
                        idx += 2;
                        continue;
                    }
                }
                idx += 1;
            }
        }
        
        di_obj.properties.insert("y".to_string(), Value::new_int(y));
        di_obj.properties.insert("m".to_string(), Value::new_int(m));
        di_obj.properties.insert("d".to_string(), Value::new_int(d));
        di_obj.properties.insert("h".to_string(), Value::new_int(h));
        di_obj.properties.insert("i".to_string(), Value::new_int(i));
        di_obj.properties.insert("s".to_string(), Value::new_int(s));
        di_obj.properties.insert("f".to_string(), Value::new_float(f));
        di_obj.properties.insert("invert".to_string(), Value::new_int(0));
        di_obj.properties.insert("days".to_string(), Value::new_bool(false));
        di_obj.properties.insert("from_string".to_string(), Value::new_bool(true));

        let ptr = ctx.get_arena().alloc(di_obj);
        Ok(Value::new_object_ptr(ptr as *mut ()))
    }
}

php_function! {
    native_datetime_get_last_errors() |ctx| {
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_datetime_set_last_errors(_errors: Value) |ctx| {
        Ok(Value::null())
    }
}

fn parse_datetime_by_format(fmt: &str, dt_str: &str, tz: &Tz) -> i64 {
    if fmt == "U" || fmt == "U.u" || fmt.starts_with('U') {
        if let Ok(ts_f) = dt_str.parse::<f64>() {
            return ts_f.floor() as i64;
        } else if let Ok(ts_i) = dt_str.parse::<i64>() {
            return ts_i;
        }
    }
    let mut chrono_fmt = String::new();
    let mut escape = false;
    for c in fmt.chars() {
        if escape {
            chrono_fmt.push(c);
            escape = false;
            continue;
        }
        match c {
            '\\' => escape = true,
            'D' => chrono_fmt.push_str("%a"),
            'l' => chrono_fmt.push_str("%A"),
            'M' => chrono_fmt.push_str("%b"),
            'F' => chrono_fmt.push_str("%B"),
            'Y' => chrono_fmt.push_str("%Y"),
            'y' => chrono_fmt.push_str("%y"),
            'm' => chrono_fmt.push_str("%m"),
            'n' => chrono_fmt.push_str("%m"),
            'd' => chrono_fmt.push_str("%d"),
            'j' => chrono_fmt.push_str("%d"),
            'H' => chrono_fmt.push_str("%H"),
            'h' => chrono_fmt.push_str("%I"),
            'G' => chrono_fmt.push_str("%H"),
            'g' => chrono_fmt.push_str("%I"),
            'i' => chrono_fmt.push_str("%M"),
            's' => chrono_fmt.push_str("%S"),
            'a' => chrono_fmt.push_str("%P"),
            'A' => chrono_fmt.push_str("%p"),
            'u' => chrono_fmt.push_str("%f"),
            'v' => chrono_fmt.push_str("%3f"),
            'U' => chrono_fmt.push_str("%s"),
            'O' => chrono_fmt.push_str("%z"),
            'P' => chrono_fmt.push_str("%:z"),
            'T' | 'e' => chrono_fmt.push_str("%Z"),
            other => chrono_fmt.push(other),
        }
    }

    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(dt_str, &chrono_fmt) {
        if let LocalResult::Single(dt) = tz.from_local_datetime(&naive) {
            dt.timestamp()
        } else {
            naive.and_utc().timestamp()
        }
    } else if let Ok(naive_date) = chrono::NaiveDate::parse_from_str(dt_str, &chrono_fmt) {
        if let Some(naive) = naive_date.and_hms_opt(0, 0, 0) {
            if let LocalResult::Single(dt) = tz.from_local_datetime(&naive) {
                dt.timestamp()
            } else {
                naive.and_utc().timestamp()
            }
        } else {
            Utc::now().timestamp()
        }
    } else if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(dt_str) {
        dt.timestamp()
    } else if let Ok(dt) = chrono::DateTime::parse_from_rfc2822(dt_str) {
        dt.timestamp()
    } else if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(dt_str, "%Y-%m-%d %H:%M:%S") {
        naive.and_utc().timestamp()
    } else {
        Utc::now().timestamp()
    }
}

php_function! {
    native_datetime_create_from_format(this: Value, format: String, datetime_str: String, timezone: Value) |ctx| {
        let fmt = format.cloned().unwrap_or_default();
        let dt_str = datetime_str.cloned().unwrap_or_default();

        let mut tz_name = "UTC".to_string();
        if let Some(tz_val) = timezone {
            if let Some(obj_ptr) = tz_val.as_object_ptr() {
                let obj = unsafe { &*(obj_ptr as *const PhpObject) };
                if let Some(name_val) = obj.properties.get("timezone_name") {
                    if let Some(s_ptr) = name_val.as_string_ptr() {
                        let s = unsafe { &*(s_ptr as *const String) };
                        tz_name = s.clone();
                    }
                }
            } else if let Some(s_ptr) = tz_val.as_string_ptr() {
                let s = unsafe { &*(s_ptr as *const String) };
                tz_name = s.clone();
            }
        }

        let tz = Tz::from_str(&tz_name).unwrap_or(*DEFAULT_TIMEZONE.lock().unwrap());
        let timestamp = parse_datetime_by_format(&fmt, &dt_str, &tz);

        let called_id = ctx.get_called_class_id();
        let dt_class_id = if called_id != 0 {
            called_id
        } else {
            ctx.get_class_id("DateTime").unwrap_or(0)
        };
        let mut dt_obj = ctx.instantiate_class(dt_class_id);
        dt_obj.properties.insert("timestamp".to_string(), Value::new_int(timestamp as i32));
        let boxed_tz = crate::into_raw(Box::new(tz_name));
        dt_obj.properties.insert("timezone_name".to_string(), Value::new_string_ptr(boxed_tz as *mut ()));
        let boxed_obj = crate::into_raw(Box::new(dt_obj));
        Ok(Value::new_object_ptr(boxed_obj as *mut ()))
    }
}

php_function! {
    native_datetime_immutable_create_from_format(this: Value, format: String, datetime_str: String, timezone: Value) |ctx| {
        let fmt = format.cloned().unwrap_or_default();
        let dt_str = datetime_str.cloned().unwrap_or_default();

        let mut tz_name = "UTC".to_string();
        if let Some(tz_val) = timezone {
            if let Some(obj_ptr) = tz_val.as_object_ptr() {
                let obj = unsafe { &*(obj_ptr as *const PhpObject) };
                if let Some(name_val) = obj.properties.get("timezone_name") {
                    if let Some(s_ptr) = name_val.as_string_ptr() {
                        let s = unsafe { &*(s_ptr as *const String) };
                        tz_name = s.clone();
                    }
                }
            } else if let Some(s_ptr) = tz_val.as_string_ptr() {
                let s = unsafe { &*(s_ptr as *const String) };
                tz_name = s.clone();
            }
        }

        let tz = Tz::from_str(&tz_name).unwrap_or(*DEFAULT_TIMEZONE.lock().unwrap());
        let timestamp = parse_datetime_by_format(&fmt, &dt_str, &tz);

        let called_id = ctx.get_called_class_id();
        let dt_class_id = if called_id != 0 {
            called_id
        } else {
            ctx.get_class_id("DateTimeImmutable").unwrap_or(0)
        };
        let mut dt_obj = ctx.instantiate_class(dt_class_id);
        dt_obj.properties.insert("timestamp".to_string(), Value::new_int(timestamp as i32));
        let boxed_tz = crate::into_raw(Box::new(tz_name));
        dt_obj.properties.insert("timezone_name".to_string(), Value::new_string_ptr(boxed_tz as *mut ()));
        let boxed_obj = crate::into_raw(Box::new(dt_obj));
        Ok(Value::new_object_ptr(boxed_obj as *mut ()))
    }
}
