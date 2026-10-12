use hyperion_core::php_function;
use hyperion_core::memory::nan_box::Value;
use hyperion_core::types::array::PhpArray;

php_function! {
    native_spl_autoload_register(autoload_function: Value, _throw: Value, prepend: Value) |ctx| {
        println!("DEBUG spl_autoload_register: func={:?} type={:?}", autoload_function, autoload_function.map(|f| f.get_type()));
        let _arena = ctx.get_arena();
        if let Some(func) = autoload_function {
            let prepend_bool = prepend.map(|v| v.is_truthy()).unwrap_or(false);
            ctx.register_autoloader(*func, prepend_bool);
            Ok(Value::new_bool(true))
        } else {
            Err("spl_autoload_register() expects at least 1 parameter".to_string())
        }
    }
}

php_function! {
    native_spl_autoload_call(class_name: String) |ctx| {
        if let Some(name) = class_name {
            ctx.trigger_autoload_sync(&name);
            Ok(Value::null())
        } else {
            Err("spl_autoload_call() expects exactly 1 parameter".to_string())
        }
    }
}


php_function! {
    native_spl_autoload_unregister(autoload_function: Value) |ctx| {
        let _arena = ctx.get_arena();
        if let Some(func) = autoload_function {
            ctx.unregister_autoloader(*func);
            Ok(Value::new_bool(true))
        } else {
            Err("spl_autoload_unregister() expects at least 1 parameter".to_string())
        }
    }
}

php_function! {
    native_spl_autoload_functions() |ctx| {
        let loaders = ctx.get_autoloaders();
        
        let mut arr = PhpArray::new();
        for (i, loader) in loaders.iter().enumerate() {
            arr.insert_int(i as i64, *loader);
        }
        let ptr = ctx.get_arena().alloc(arr);
        Ok(Value::new_array_ptr(ptr as *mut ()))
    }
}

php_function! {
    native_curl_version() |ctx| { 
        let mut arr = PhpArray::new();
        
        let version_str = crate::into_raw(Box::new("7.88.1".to_string()));
        arr.insert_string_id(ctx.intern_string("version"), Value::new_string_ptr(version_str as *mut ()));
        
        arr.insert_string_id(ctx.intern_string("version_number"), Value::new_int(0x075800));
        arr.insert_string_id(ctx.intern_string("features"), Value::new_int(-1));
        
        let ptr = ctx.get_arena().alloc(arr);
        Ok(Value::new_array_ptr(ptr as *mut ()))
    }
}

php_function! {
    native_register_shutdown_function(_callback: Value) {
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_spl_object_hash(obj: Value) |ctx| {
        if let Some(obj_val) = obj {
            let derefed = obj_val.deref();
            if let Some(ptr) = derefed.as_object_ptr() {
                let hash = format!("{:p}", ptr);
                let str_ptr = crate::into_raw(Box::new(hash));
                Ok(Value::new_string_ptr(str_ptr as *mut ()))
            } else if let Some(ptr) = derefed.as_closure_ptr() {
                let hash = format!("{:p}", ptr);
                let str_ptr = crate::into_raw(Box::new(hash));
                Ok(Value::new_string_ptr(str_ptr as *mut ()))
            } else {
                Err("spl_object_hash() expects parameter 1 to be object".to_string())
            }
        } else {
            Err("spl_object_hash() expects parameter 1 to be object".to_string())
        }
    }
}

php_function! {
    native_spl_object_id(obj: Value) {
        if let Some(obj_val) = obj {
            let derefed = obj_val.deref();
            if let Some(ptr) = derefed.as_object_ptr() {
                Ok(Value::new_int(ptr as usize as i32))
            } else if let Some(ptr) = derefed.as_closure_ptr() {
                Ok(Value::new_int(ptr as usize as i32))
            } else {
                Err("spl_object_id() expects parameter 1 to be object".to_string())
            }
        } else {
            Err("spl_object_id() expects parameter 1 to be object".to_string())
        }
    }
}
