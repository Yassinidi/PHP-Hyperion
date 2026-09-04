use hyperion_core::php_function;
use hyperion_core::memory::nan_box::Value;

php_function! {
    native_php_sapi_name() {
        let boxed = crate::into_raw(Box::new("cli".to_string()));
        Ok(Value::new_string_ptr(boxed as *mut ()))
    }
}

php_function! {
    native_exception_construct(this: Value, message: Value, code: Value, previous: Value) {
        if let Some(obj_ptr) = this.and_then(|v| v.as_object_ptr()) {
            let obj = unsafe { &mut *(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
            if let Some(m) = message {
                let msg_str = if let Some(s_ptr) = m.as_string_ptr() {
                    unsafe { (*(s_ptr as *const String)).clone() }
                } else if let Some(i) = m.as_int() {
                    i.to_string()
                } else if let Some(f) = m.as_float() {
                    f.to_string()
                } else if let Some(b) = m.as_bool() {
                    if b { "1".to_string() } else { "".to_string() }
                } else {
                    "".to_string()
                };
                let boxed = crate::into_raw(Box::new(msg_str));
                obj.properties.insert("message".to_string(), Value::new_string_ptr(boxed as *mut ()));
            } else {
                let boxed = crate::into_raw(Box::new("".to_string()));
                obj.properties.insert("message".to_string(), Value::new_string_ptr(boxed as *mut ()));
            }
            if let Some(c) = code {
                obj.properties.insert("code".to_string(), *c);
            } else {
                obj.properties.insert("code".to_string(), Value::new_int(0));
            }
            if let Some(p) = previous {
                obj.properties.insert("previous".to_string(), *p);
            } else {
                obj.properties.insert("previous".to_string(), Value::null());
            }
        }
        Ok(Value::null())
    }
}

php_function! {
    native_exception_get_message(this: Value) {
        if let Some(obj_ptr) = this.and_then(|v| v.as_object_ptr()) {
            let obj = unsafe { &*(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
            if let Some(val) = obj.properties.get("message") {
                return Ok(*val);
            }
        }
        let boxed = crate::into_raw(Box::new("".to_string()));
        Ok(Value::new_string_ptr(boxed as *mut ()))
    }
}

php_function! {
    native_exception_get_code(this: Value) {
        if let Some(obj_ptr) = this.and_then(|v| v.as_object_ptr()) {
            let obj = unsafe { &*(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
            if let Some(val) = obj.properties.get("code") {
                return Ok(*val);
            }
        }
        Ok(Value::new_int(0))
    }
}

php_function! {
    native_exception_get_file(this: Value) {
        if let Some(obj_ptr) = this.and_then(|v| v.as_object_ptr()) {
            let obj = unsafe { &*(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
            if let Some(val) = obj.properties.get("file") {
                return Ok(*val);
            }
        }
        let boxed = crate::into_raw(Box::new("".to_string()));
        Ok(Value::new_string_ptr(boxed as *mut ()))
    }
}

php_function! {
    native_exception_get_line(this: Value) {
        if let Some(obj_ptr) = this.and_then(|v| v.as_object_ptr()) {
            let obj = unsafe { &*(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
            if let Some(val) = obj.properties.get("line") {
                return Ok(*val);
            }
        }
        Ok(Value::new_int(0))
    }
}

php_function! {
    native_exception_get_trace(this: Value) |ctx| {
        // Return empty array for now
        let arr = hyperion_core::types::array::PhpArray::new();
        let arr_ptr = ctx.get_arena().alloc(arr);
        Ok(Value::new_array_ptr(arr_ptr as *mut ()))
    }
}

php_function! {
    native_exception_get_trace_as_string(this: Value) {
        let boxed = crate::into_raw(Box::new("".to_string()));
        Ok(Value::new_string_ptr(boxed as *mut ()))
    }
}

php_function! {
    native_exception_get_previous(this: Value) {
        if let Some(obj_ptr) = this.and_then(|v| v.as_object_ptr()) {
            let obj = unsafe { &*(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
            if let Some(val) = obj.properties.get("previous") {
                return Ok(*val);
            }
        }
        Ok(Value::null())
    }
}

php_function! {
    native_exception_to_string(this: Value) {
        if let Some(obj_ptr) = this.and_then(|v| v.as_object_ptr()) {
            let obj = unsafe { &*(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
            let mut msg = String::new();
            if let Some(val) = obj.properties.get("message") {
                if let Some(ptr) = val.as_string_ptr() {
                    let s = unsafe { &*(ptr as *const String) };
                    msg = s.clone();
                }
            }
            let res = format!("Exception: {}", msg);
            let boxed = crate::into_raw(Box::new(res));
            return Ok(Value::new_string_ptr(boxed as *mut ()));
        }
        let boxed = crate::into_raw(Box::new("Exception".to_string()));
        Ok(Value::new_string_ptr(boxed as *mut ()))
    }
}

php_function! {
    native_fiber_construct(this: Value, callback: Value) {
        Ok(Value::null())
    }
}

php_function! {
    native_fiber_start(this: Value, ...rest) {
        Ok(Value::null())
    }
}

php_function! {
    native_fiber_resume(this: Value, value: Value) {
        Ok(Value::null())
    }
}

php_function! {
    native_fiber_throw(this: Value, exception: Value) {
        Ok(Value::null())
    }
}

php_function! {
    native_fiber_get_return(this: Value) {
        Ok(Value::null())
    }
}

php_function! {
    native_fiber_is_started(this: Value) {
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_fiber_is_suspended(this: Value) {
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_fiber_is_running(this: Value) {
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_fiber_is_terminated(this: Value) {
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_fiber_get_current() {
        Ok(Value::null())
    }
}

php_function! {
    native_fiber_suspend(value: Value) {
        Ok(Value::null())
    }
}

