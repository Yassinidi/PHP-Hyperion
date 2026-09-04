use hyperion_core::memory::nan_box::Value;
use hyperion_core::php_function;
use hyperion_core::types::array::PhpArray;

// ============================================================
// 🛡️ PHP Variable Handling Module — ~20 native functions
// ============================================================

// --- Type Checking ---

php_function! { native_is_array(v: Value) Ok(Value::new_bool(v.map(|v| v.deref().is_array()).unwrap_or(false)))  }
php_function! { native_is_bool(v: Value) Ok(Value::new_bool(v.map(|v| v.deref().is_bool()).unwrap_or(false)))  }
php_function! { native_is_int(v: Value) Ok(Value::new_bool(v.map(|v| v.deref().is_int()).unwrap_or(false)))  }
php_function! { native_is_integer(v: Value) Ok(Value::new_bool(v.map(|v| v.deref().is_int()).unwrap_or(false)))  }
php_function! { native_is_long(v: Value) Ok(Value::new_bool(v.map(|v| v.deref().is_int()).unwrap_or(false)))  }
php_function! { native_is_float(v: Value) Ok(Value::new_bool(v.map(|v| v.deref().is_float()).unwrap_or(false)))  }
php_function! { native_is_double(v: Value) Ok(Value::new_bool(v.map(|v| v.deref().is_float()).unwrap_or(false)))  }
php_function! { native_is_string(v: Value) Ok(Value::new_bool(v.map(|v| v.deref().is_string()).unwrap_or(false)))  }
php_function! { native_is_null(v: Value) Ok(Value::new_bool(v.map(|v| v.deref().is_null()).unwrap_or(true)))  }
php_function! { native_is_object(v: Value) Ok(Value::new_bool(v.map(|v| { let d = v.deref(); d.is_object() || d.is_closure() }).unwrap_or(false)))  }
php_function! { native_is_iterable(v: Value) Ok(Value::new_bool(v.map(|v| { let d = v.deref(); d.is_array() || d.is_object() || d.is_closure() }).unwrap_or(false)))  }
php_function! {
    native_is_countable(v: Value) |ctx| {
        let is_cnt = if let Some(val) = v {
            let derefed = val.deref();
            if derefed.is_array() {
                true
            } else if let Some(obj_ptr) = derefed.as_object_ptr() {
                let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
                if let Some(class_name) = ctx.get_class_name(obj.class_id) {
                    ctx.implements_interface(&class_name, "Countable")
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };
        Ok(Value::new_bool(is_cnt))
    }
}

php_function! {
    native_func_num_args() |ctx| {
        Ok(Value::new_int(ctx.get_caller_arity() as i32))
    }
}

php_function! {
    native_func_get_arg(arg_num: Value) |ctx| {
        let n = arg_num.and_then(|v| if v.is_int() { v.as_int() } else { None }).unwrap_or(0);
        let args = ctx.get_caller_args();
        if n >= 0 && n < args.len() as i32 {
            Ok(args[n as usize])
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_func_get_args() |ctx| {
        let args = ctx.get_caller_args();
        let mut arr = PhpArray::new();
        for (i, arg) in args.into_iter().enumerate() {
            arr.insert_int(i as i64, arg);
        }
        let arr_ptr = ctx.get_arena().alloc(arr);
        Ok(Value::new_array_ptr(arr_ptr as *mut ()))
    }
}

php_function! {
    native_is_numeric(v: Value) {
        if let Some(val) = v {
            let vd = val.deref();
            if vd.is_int() || vd.is_float() {
                return Ok(Value::new_bool(true));
            }
            if let Some(sp) = vd.as_string_ptr() {
                let s = unsafe { &*(sp as *const String) };
                let is_num = s.trim().parse::<f64>().is_ok();
                return Ok(Value::new_bool(is_num));
            }
            Ok(Value::new_bool(false))
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_is_scalar(v: Value) {
        if let Some(val) = v {
            let vd = val.deref();
            let scalar = vd.is_int() || vd.is_float() || vd.is_bool() || vd.is_string();
            Ok(Value::new_bool(scalar))
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_is_callable(v: Value, syntax_only: Value, callable_name: Value) |ctx| {
        let _ = (syntax_only, callable_name);
        if let Some(val) = v {
            let deref = val.deref();
            if deref.is_closure() {
                return Ok(Value::new_bool(true));
            }
            if let Some(obj_ptr) = deref.as_object_ptr() {
                let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
                let cls_name = obj.class_name.clone().or_else(|| ctx.get_class_name(obj.class_id));
                if let Some(cls) = cls_name {
                    if ctx.has_method(&cls, "__invoke") {
                        return Ok(Value::new_bool(true));
                    }
                }
                return Ok(Value::new_bool(false));
            }
            if let Some(s_ptr) = deref.as_string_ptr() {
                let s = unsafe { &*(s_ptr as *const String) };
                if s.contains("::") {
                    let parts: Vec<&str> = s.split("::").collect();
                    if parts.len() == 2 {
                        let cls = parts[0];
                        let method = parts[1];
                        return Ok(Value::new_bool(ctx.has_method(cls, method)));
                    }
                }
                return Ok(Value::new_bool(ctx.function_exists(s)));
            }
            if let Some(arr_ptr) = deref.as_array_ptr() {
                let arr = unsafe { &*(arr_ptr as *const PhpArray) };
                if arr.elements.len() == 2 {
                    let mut vals = arr.elements.values();
                    let target = vals.next();
                    let method = vals.next();
                    if let (Some(t_val), Some(m_val)) = (target, method) {
                        let t_deref = t_val.deref();
                        let m_deref = m_val.deref();
                        if let Some(m_ptr) = m_deref.as_string_ptr() {
                            let method_name = unsafe { &*(m_ptr as *const String) };
                            if let Some(obj_ptr) = t_deref.as_object_ptr() {
                                let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
                                let cls_name = obj.class_name.clone().or_else(|| ctx.get_class_name(obj.class_id));
                                if let Some(cls) = cls_name {
                                    return Ok(Value::new_bool(ctx.has_method(&cls, method_name)));
                                }
                            } else if let Some(cls_ptr) = t_deref.as_string_ptr() {
                                let cls = unsafe { &*(cls_ptr as *const String) };
                                return Ok(Value::new_bool(ctx.has_method(cls, method_name)));
                            }
                        }
                    }
                }
            }
            Ok(Value::new_bool(false))
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

pub fn native_is_a(
    args: &[Value],
    ctx: &mut dyn hyperion_core::types::function::NativeContext,
) -> Result<Value, String> {
    if args.len() < 2 {
        return Ok(Value::new_bool(false));
    }

    let object_or_class = args[0];
    let class_name_val = args[1];

    let allow_string = if args.len() >= 3 {
        args[2].as_bool().unwrap_or(false)
    } else {
        false
    };

    let obj_class_name = if let Some(obj_ptr) = object_or_class.as_object_ptr() {
        let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
        obj.class_name.clone().unwrap_or_else(|| ctx.get_class_name(obj.class_id).unwrap_or_default())
    } else if allow_string && object_or_class.is_string() {
        unsafe { &*(object_or_class.as_string_ptr().unwrap() as *const String) }.clone()
    } else {
        return Ok(Value::new_bool(false));
    };

    let target_class_name = if class_name_val.is_string() {
        unsafe { &*(class_name_val.as_string_ptr().unwrap() as *const String) }.clone()
    } else {
        return Ok(Value::new_bool(false));
    };

    if obj_class_name.eq_ignore_ascii_case(&target_class_name) {
        return Ok(Value::new_bool(true));
    }

    let mut is_a = false;
    let mut curr_class = Some(obj_class_name.clone());
    while let Some(cname) = curr_class {
        if cname.eq_ignore_ascii_case(&target_class_name) {
            is_a = true;
            break;
        }
        if ctx.implements_interface(&cname, &target_class_name) {
            is_a = true;
            break;
        }
        curr_class = ctx.get_parent_class(&cname);
    }

    Ok(Value::new_bool(is_a))
}

pub fn native_is_subclass_of(
    args: &[Value],
    ctx: &mut dyn hyperion_core::types::function::NativeContext,
) -> Result<Value, String> {
    if args.len() < 2 {
        return Ok(Value::new_bool(false));
    }

    let object_or_class = args[0];
    let class_name_val = args[1];

    let allow_string = if args.len() >= 3 {
        args[2].as_bool().unwrap_or(true)
    } else {
        true
    };

    let obj_class_name = if let Some(obj_ptr) = object_or_class.as_object_ptr() {
        let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
        obj.class_name.clone().unwrap_or_else(|| ctx.get_class_name(obj.class_id).unwrap_or_default())
    } else if allow_string && object_or_class.is_string() {
        unsafe { &*(object_or_class.as_string_ptr().unwrap() as *const String) }.clone()
    } else {
        return Ok(Value::new_bool(false));
    };

    let target_class_name = if let Some(obj_ptr) = class_name_val.as_object_ptr() {
        let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
        obj.class_name.clone().unwrap_or_else(|| ctx.get_class_name(obj.class_id).unwrap_or_default())
    } else if class_name_val.is_string() {
        unsafe { &*(class_name_val.as_string_ptr().unwrap() as *const String) }.clone()
    } else {
        return Ok(Value::new_bool(false));
    };

    let obj_clean = obj_class_name.trim_start_matches('\\');
    let target_clean = target_class_name.trim_start_matches('\\');

    if obj_clean.eq_ignore_ascii_case(target_clean) {
        return Ok(Value::new_bool(false));
    }

    let parents = ctx.get_parent_classes(obj_clean);
    for p in &parents {
        let p_clean = p.trim_start_matches('\\');
        if p_clean.eq_ignore_ascii_case(target_clean) {
            return Ok(Value::new_bool(true));
        }
        if ctx.implements_interface(p_clean, target_clean) {
            return Ok(Value::new_bool(true));
        }
    }

    if ctx.implements_interface(obj_clean, target_clean) {
        return Ok(Value::new_bool(true));
    }

    Ok(Value::new_bool(false))
}

// --- Type Info ---

php_function! {
    native_gettype(v: Value) {
        if let Some(val) = v {
            // A ref is transparent: gettype() reports the pointee's type.
            let val = val.deref();
            let type_str = if val.is_null() { "NULL" }
                else if val.is_bool() { "boolean" }
                else if val.is_int() { "integer" }
                else if val.is_float() { "double" }
                else if val.is_string() { "string" }
                else if val.is_array() { "array" }
                else if val.is_object() { "object" }
                else if val.is_closure() { "object" }
                else if let Some(p) = val.as_resource_ptr() {
                    // PHP distinguishes a live handle from a closed one here,
                    // and code logging types relies on seeing the difference.
                    let r = unsafe { &*(p as *const hyperion_core::types::resource::PhpResource) };
                    if r.is_open() { "resource" } else { "resource (closed)" }
                }
                else { "unknown type" };
            let boxed = crate::into_raw(Box::new(type_str.to_string()));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        } else {
            let boxed = crate::into_raw(Box::new("NULL".to_string()));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        }
    }
}

// --- Type Conversion ---

php_function! {
    native_intval(v: Value) {
        if let Some(val) = v {
            if let Some(i) = val.as_int() { return Ok(Value::new_int(i)); }
            if let Some(f) = val.as_float() { return Ok(Value::new_int(f as i32)); }
            if let Some(b) = val.as_bool() { return Ok(Value::new_int(if b { 1 } else { 0 })); }
            if val.is_null() { return Ok(Value::new_int(0)); }
            if let Some(sp) = val.as_string_ptr() {
                let s = unsafe { &*(sp as *const String) };
                let parsed = s.trim().parse::<i32>().unwrap_or(0);
                return Ok(Value::new_int(parsed));
            }
            Ok(Value::new_int(0))
        } else {
            Ok(Value::new_int(0))
        }
    }
}

php_function! {
    native_floatval(v: Value) {
        if let Some(val) = v {
            if let Some(f) = val.as_float() { return Ok(Value::new_float(f)); }
            if let Some(i) = val.as_int() { return Ok(Value::new_float(i as f64)); }
            if let Some(b) = val.as_bool() { return Ok(Value::new_float(if b { 1.0 } else { 0.0 })); }
            if val.is_null() { return Ok(Value::new_float(0.0)); }
            if let Some(sp) = val.as_string_ptr() {
                let s = unsafe { &*(sp as *const String) };
                let parsed = s.trim().parse::<f64>().unwrap_or(0.0);
                return Ok(Value::new_float(parsed));
            }
            Ok(Value::new_float(0.0))
        } else {
            Ok(Value::new_float(0.0))
        }
    }
}

php_function! {
    native_strval(v: Value) |ctx| {
        if let Some(val) = v {
            let s = if val.is_object() {
                let obj_ptr = val.as_object_ptr().unwrap() as *const hyperion_core::types::object::PhpObject;
                let obj = unsafe { &*obj_ptr };
                let class_name = if let Some(ref name) = obj.class_name {
                    name.clone()
                } else {
                    ctx.get_class_name(obj.class_id).unwrap_or_default()
                };
                if !class_name.is_empty() && ctx.has_method(&class_name, "__toString") {
                    let mut arr = hyperion_core::types::array::PhpArray::new();
                    arr.insert_int(0, *val);
                    let m_ptr = ctx.get_arena().alloc_and_track("__toString".to_string());
                    arr.insert_int(1, Value::new_string_ptr(m_ptr as *mut ()));
                    let arr_ptr = ctx.get_arena().alloc_and_track(arr);
                    let callable_arr = Value::new_array_ptr(arr_ptr as *mut ());
                    if let Ok(res) = ctx.call_callable_synchronously(callable_arr, vec![]) {
                        if let Some(s_ptr) = res.as_string_ptr() {
                            unsafe { (*(s_ptr as *const String)).clone() }
                        } else {
                            "Object".to_string()
                        }
                    } else {
                        "Object".to_string()
                    }
                } else {
                    "Object".to_string()
                }
            } else {
                value_to_string(val)
            };
            let boxed = ctx.get_arena().alloc_and_track(s);
            Ok(Value::new_string_ptr(boxed as *mut ()))
        } else {
            let boxed = ctx.get_arena().alloc_and_track(String::new());
            Ok(Value::new_string_ptr(boxed as *mut ()))
        }
    }
}

php_function! {
    native_boolval(v: Value) {
        if let Some(val) = v {
            Ok(Value::new_bool(!value_is_falsy(val)))
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_assert(assertion: Value, description: Value) |ctx| {
        if let Some(val) = assertion {
            let derefed = val.deref();
            if !value_is_falsy(&derefed) {
                return Ok(Value::new_bool(true));
            }
            let bt = ctx.get_backtrace();
            let caller_loc = if let Some(frame) = bt.first() {
                format!(" at {}:{}", frame.file, frame.line)
            } else {
                String::new()
            };
            let msg = if let Some(desc_val) = description {
                if let Some(s_ptr) = desc_val.deref().as_string_ptr() {
                    unsafe { &*(s_ptr as *const String) }.clone()
                } else {
                    "Assertion failed".to_string()
                }
            } else {
                "Assertion failed".to_string()
            };
            return Err(format!("AssertionError: {}{}", msg, caller_loc));
        }
        Ok(Value::new_bool(true))
    }
}

// --- Debug / Output ---

php_function! {
    native_var_dump(v: Value, ...rest) |ctx| {
        // PHP's var_dump is variadic: it dumps every argument in order.
        let mut output = String::new();
        match v {
            Some(val) => output.push_str(&var_dump_value(val, 0, ctx)),
            None => output.push_str("NULL\n"),
        }
        for extra in rest {
            output.push_str(&var_dump_value(extra, 0, ctx));
        }
        // Straight to the script's output buffer, not fd 1: `print!` here
        // would jump ahead of every pending `echo`, so a dump would appear
        // before output the script emitted earlier.
        ctx.write_output(output.as_bytes());
        Ok(Value::null())
    }
}

php_function! {
    native_print_r(v: Value, return_val: Value) |ctx| {
        if let Some(val) = v {
            let output = print_r_value(val, 0, ctx);
            let should_return = return_val.and_then(|r| r.as_bool()).unwrap_or(false);
            if should_return {
                let boxed = crate::into_raw(Box::new(output));
                Ok(Value::new_string_ptr(boxed as *mut ()))
            } else {
                ctx.write_output(output.as_bytes());
                Ok(Value::new_bool(true))
            }
        } else {
            Ok(Value::new_bool(true))
        }
    }
}

php_function! {
    native_var_export(v: Value, return_val: Value) |ctx| {
        if let Some(val) = v {
            let output = var_export_value(val, 0, ctx);
            let should_return = return_val.and_then(|r| r.as_bool()).unwrap_or(false);
            if should_return {
                let boxed = crate::into_raw(Box::new(output));
                Ok(Value::new_string_ptr(boxed as *mut ()))
            } else {
                ctx.write_output(output.as_bytes());
                Ok(Value::null())
            }
        } else {
            Ok(Value::null())
        }
    }
}

// --- Existence checks (need GlobalEngineState access — simplified stubs) ---

php_function! {
    native_filter_var(v: Value, filter: Value, flags: Value) {
        let val = if let Some(val) = v { val } else { return Ok(Value::new_bool(false)) };

        let filter_id = filter.and_then(|f| f.as_int()).unwrap_or(516); // 516 = FILTER_DEFAULT
        let flag_val = flags.and_then(|fl| fl.as_int()).unwrap_or(0);

        match filter_id {
            258 => { // FILTER_VALIDATE_BOOLEAN
                let is_true = if let Some(b) = val.as_bool() { b }
                    else if let Some(i) = val.as_int() { i == 1 }
                    else if let Some(sp) = val.as_string_ptr() {
                        let s = unsafe { &*(sp as *const String) }.to_lowercase();
                        s == "1" || s == "true" || s == "on" || s == "yes"
                    } else { false };
                Ok(Value::new_bool(is_true))
            },
            275 => { // FILTER_VALIDATE_IP
                if let Some(sp) = val.as_string_ptr() {
                    let s = unsafe { &*(sp as *const String) };
                    if flag_val == 1048576 { // FILTER_FLAG_IPV4
                        if s.parse::<std::net::Ipv4Addr>().is_ok() {
                            Ok(*val)
                        } else {
                            Ok(Value::new_bool(false))
                        }
                    } else if flag_val == 2097152 { // FILTER_FLAG_IPV6
                        if s.parse::<std::net::Ipv6Addr>().is_ok() {
                            Ok(*val)
                        } else {
                            Ok(Value::new_bool(false))
                        }
                    } else {
                        if s.parse::<std::net::IpAddr>().is_ok() {
                            Ok(*val)
                        } else {
                            Ok(Value::new_bool(false))
                        }
                    }
                } else {
                    Ok(Value::new_bool(false))
                }
            },
            274 => { // FILTER_VALIDATE_EMAIL
                if let Some(sp) = val.as_string_ptr() {
                    let s = unsafe { &*(sp as *const String) };
                    let parts: Vec<&str> = s.split('@').collect();
                    if parts.len() == 2 && !parts[0].is_empty() && parts[1].contains('.') && !parts[1].starts_with('.') && !parts[1].ends_with('.') {
                        Ok(*val)
                    } else {
                        Ok(Value::new_bool(false))
                    }
                } else {
                    Ok(Value::new_bool(false))
                }
            },
            273 => { // FILTER_VALIDATE_URL
                if let Some(sp) = val.as_string_ptr() {
                    let s = unsafe { &*(sp as *const String) };
                    lazy_static::lazy_static! {
                        static ref URL_RE: regex::Regex = regex::Regex::new(
                            r#"^(?i)([a-z][a-z0-9+.-]*):\/\/(?:[^\s:@]+(?::[^\s:@]*)?@)?([^\s/:?#]+)(?::\d+)?(?:\/[^\s?#]*)?(?:\?[^\s#]*)?(?:#[^\s]*)?$"#
                        ).unwrap();
                    }
                    if URL_RE.is_match(s) {
                        Ok(*val)
                    } else {
                        Ok(Value::new_bool(false))
                    }
                } else {
                    Ok(Value::new_bool(false))
                }
            },
            _ => Ok(*val), // Default: return as is
        }
    }
}

php_function! {
    native_class_exists(name: Value, autoload: Value) |ctx| { let _arena = ctx.get_arena();
        if let Some(n) = name {
            let n = n.deref();
            if let Some(sp) = n.as_string_ptr() {
                let raw_name = unsafe { &*(sp as *const String) };
                let class_name = raw_name.trim_start_matches('\\');
                let mut exists = ctx.class_exists(class_name) && !ctx.interface_exists(class_name) && !ctx.trait_exists(class_name);

                let should_autoload = autoload.map(|a| a.deref()).and_then(|a| if let Some(b) = a.as_bool() { Some(b) } else if let Some(i) = a.as_int() { Some(i != 0) } else { None }).unwrap_or(true);
                if !exists && should_autoload {
                    ctx.trigger_autoload_sync(class_name);
                    exists = ctx.class_exists(class_name) && !ctx.interface_exists(class_name) && !ctx.trait_exists(class_name);
                }
                Ok(Value::new_bool(exists))
            } else {
                Ok(Value::new_bool(false))
            }
        } else {
            Err("class_exists() expects at least 1 parameter".to_string())
        }
    }
}

php_function! {
    native_class_alias(class: Value, alias: Value, autoload: Value) |ctx| { let _arena = ctx.get_arena();
        if let (Some(c), Some(a)) = (class, alias) {
            let c = c.deref();
            let a = a.deref();
            if let (Some(cp), Some(ap)) = (c.as_string_ptr(), a.as_string_ptr()) {
                let raw_c = unsafe { &*(cp as *const String) };
                let raw_a = unsafe { &*(ap as *const String) };
                let class_name = raw_c.trim_start_matches('\\');
                let alias_name = raw_a.trim_start_matches('\\');

                let should_autoload = autoload.map(|a| a.deref()).and_then(|a| if let Some(b) = a.as_bool() { Some(b) } else if let Some(i) = a.as_int() { Some(i != 0) } else { None }).unwrap_or(true);
                if !ctx.class_exists(class_name) && should_autoload {
                    ctx.trigger_autoload_sync(class_name);
                }

                let success = ctx.register_class_alias(class_name, alias_name);
                Ok(Value::new_bool(success))
            } else {
                Ok(Value::new_bool(false))
            }
        } else {
            Err("class_alias() expects at least 2 parameters".to_string())
        }
    }
}

php_function! {
    native_interface_exists(name: Value, autoload: Value) |ctx| { let _arena = ctx.get_arena();
        if let Some(n) = name {
            let n = n.deref();
            if let Some(sp) = n.as_string_ptr() {
                let raw_name = unsafe { &*(sp as *const String) };
                let class_name = raw_name.trim_start_matches('\\');
                let mut exists = ctx.interface_exists(class_name);

                let should_autoload = autoload.map(|a| a.deref()).and_then(|a| if let Some(b) = a.as_bool() { Some(b) } else if let Some(i) = a.as_int() { Some(i != 0) } else { None }).unwrap_or(true);
                if !exists && should_autoload {
                    ctx.trigger_autoload_sync(class_name);
                    exists = ctx.interface_exists(class_name);
                }
                Ok(Value::new_bool(exists))
            } else {
                Ok(Value::new_bool(false))
            }
        } else {
            Err("interface_exists() expects at least 1 parameter".to_string())
        }
    }
}

php_function! {
    native_trait_exists(name: Value, autoload: Value) |ctx| {
        if let Some(n) = name {
            let n = n.deref();
            if let Some(sp) = n.as_string_ptr() {
                let raw_name = unsafe { &*(sp as *const String) };
                let class_name = raw_name.trim_start_matches('\\');
                let mut exists = ctx.trait_exists(class_name);

                let should_autoload = autoload.map(|a| a.deref()).and_then(|a| if let Some(b) = a.as_bool() { Some(b) } else if let Some(i) = a.as_int() { Some(i != 0) } else { None }).unwrap_or(true);
                if !exists && should_autoload {
                    ctx.trigger_autoload_sync(class_name);
                    exists = ctx.trait_exists(class_name);
                }
                Ok(Value::new_bool(exists))
            } else {
                Ok(Value::new_bool(false))
            }
        } else {
            Err("trait_exists() expects at least 1 parameter".to_string())
        }
    }
}

php_function! {
    native_enum_exists(name: Value, autoload: Value) |ctx| {
        if let Some(n) = name {
            let n = n.deref();
            if let Some(sp) = n.as_string_ptr() {
                let raw_name = unsafe { &*(sp as *const String) };
                let class_name = raw_name.trim_start_matches('\\');
                let mut exists = ctx.enum_exists(class_name);

                let should_autoload = autoload.map(|a| a.deref()).and_then(|a| if let Some(b) = a.as_bool() { Some(b) } else if let Some(i) = a.as_int() { Some(i != 0) } else { None }).unwrap_or(true);
                if !exists && should_autoload {
                    ctx.trigger_autoload_sync(class_name);
                    exists = ctx.enum_exists(class_name);
                }
                Ok(Value::new_bool(exists))
            } else {
                Ok(Value::new_bool(false))
            }
        } else {
            Err("enum_exists() expects at least 1 parameter".to_string())
        }
    }
}

php_function! {
    native_gc_enabled() {
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_gc_enable() {
        Ok(Value::null())
    }
}

php_function! {
    native_gc_disable() {
        Ok(Value::null())
    }
}

php_function! {
    native_gc_collect_cycles() {
        Ok(Value::new_int(0))
    }
}

php_function! {
    native_gc_mem_caches() {
        Ok(Value::new_int(0))
    }
}

php_function! {
    native_gc_status() |ctx| {
        let mut arr = hyperion_core::types::array::PhpArray::new();
        let k1 = ctx.intern_string("runs");
        arr.insert_string_id(k1, Value::new_int(0));
        let k2 = ctx.intern_string("collected");
        arr.insert_string_id(k2, Value::new_int(0));
        let k3 = ctx.intern_string("threshold");
        arr.insert_string_id(k3, Value::new_int(10000));
        let k4 = ctx.intern_string("roots");
        arr.insert_string_id(k4, Value::new_int(0));
        let k5 = ctx.intern_string("running");
        arr.insert_string_id(k5, Value::new_bool(false));
        let k6 = ctx.intern_string("protected");
        arr.insert_string_id(k6, Value::new_bool(false));
        let k7 = ctx.intern_string("full");
        arr.insert_string_id(k7, Value::new_bool(false));
        let k8 = ctx.intern_string("buffer_size");
        arr.insert_string_id(k8, Value::new_int(10000));
        let ptr = crate::into_raw(Box::new(arr));
        Ok(Value::new_array_ptr(ptr as *mut ()))
    }
}

php_function! {
    native_get_mangled_object_vars(object: Value) |ctx| {
        if let Some(obj_val) = object {
            if let Some(obj_ptr) = obj_val.as_object_ptr() {
                let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
                let mut arr = hyperion_core::types::array::PhpArray::new();
                for (k, v) in &obj.properties {
                    let sid = ctx.intern_string(k);
                    arr.insert_string_id(sid, *v);
                }
                let ptr = crate::into_raw(Box::new(arr));
                return Ok(Value::new_array_ptr(ptr as *mut ()));
            }
        }
        let ptr = crate::into_raw(Box::new(hyperion_core::types::array::PhpArray::new()));
        Ok(Value::new_array_ptr(ptr as *mut ()))
    }
}

php_function! {
    native_memory_reset_peak_usage() {
        Ok(Value::null())
    }
}

php_function! {
    native_function_exists(name: String) {
        if let Some(_name) = name {
            Ok(Value::new_bool(false))
        } else {
            Err("function_exists() expects exactly 1 parameter".to_string())
        }
    }
}

pub fn native_method_exists(
    args: &[Value],
    ctx: &mut dyn hyperion_core::types::function::NativeContext,
) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("method_exists() expects exactly 2 parameters".to_string());
    }
    let obj_val = args[0];
    let method_val = args[1];

    let method_name = if let Some(m_ptr) = method_val.as_string_ptr() {
        unsafe { (*(m_ptr as *const String)).clone() }
    } else {
        return Ok(Value::new_bool(false));
    };

    let class_name = if let Some(obj_ptr) = obj_val.as_object_ptr() {
        let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
        if let Some(ref name) = obj.class_name {
            name.clone()
        } else {
            ctx.get_class_name(obj.class_id).unwrap_or_default()
        }
    } else if let Some(c_ptr) = obj_val.as_string_ptr() {
        unsafe { (*(c_ptr as *const String)).clone() }
    } else {
        return Ok(Value::new_bool(false));
    };

    if class_name.is_empty() {
        return Ok(Value::new_bool(false));
    }

    let res = ctx.has_method(&class_name, &method_name);
    Ok(Value::new_bool(res))
}

php_function! {
    native_property_exists(object_or_class: Value, prop: Value) |ctx| {
        let (Some(val), Some(prop_val)) = (object_or_class, prop) else {
            return Ok(Value::new_bool(false));
        };
        let Some(prop_name) = prop_val.as_string_ptr().map(|p| unsafe { (*(p as *const String)).clone() }) else {
            return Ok(Value::new_bool(false));
        };

        if let Some(obj_ptr) = val.as_object_ptr() {
            let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
            if obj.properties.contains_key(&prop_name) {
                return Ok(Value::new_bool(true));
            }
            let class_name_opt = obj.class_name.clone().or_else(|| ctx.get_class_name(obj.class_id));
            if let Some(class_name) = class_name_opt {
                let has = ctx.has_class_property(&class_name, &prop_name)
                    || ctx.get_static_property(&class_name, &prop_name).is_some()
                    || ctx.is_property_public(&class_name, &prop_name);
                return Ok(Value::new_bool(has));
            }
        } else if let Some(s_ptr) = val.as_string_ptr() {
            let class_name = unsafe { (*(s_ptr as *const String)).clone() };
            let has = ctx.has_class_property(&class_name, &prop_name)
                || ctx.get_static_property(&class_name, &prop_name).is_some()
                || ctx.is_property_public(&class_name, &prop_name);
            return Ok(Value::new_bool(has));
        }

        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_get_parent_class(object_or_class: Value) |ctx| {
        let class_name = if let Some(val) = object_or_class {
            if val.is_null() {
                None
            } else if let Some(obj_ptr) = val.as_object_ptr() {
                let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
                obj.class_name.clone().or_else(|| ctx.get_class_name(obj.class_id))
            } else if let Some(s_ptr) = val.as_string_ptr() {
                Some(unsafe { (*(s_ptr as *const String)).clone() })
            } else {
                None
            }
        } else {
            None
        };

        if let Some(class_name) = class_name {
            if let Some(parent) = ctx.get_parent_class(&class_name) {
                let parent_boxed = crate::into_raw(Box::new(parent));
                return Ok(Value::new_string_ptr(parent_boxed as *mut ()));
            }
        }

        Ok(Value::new_bool(false))
    }
}

// ============================================================
// Helpers
// ============================================================

fn value_to_string(v: &Value) -> String {
    if v.is_null() {
        return String::new();
    }
    if let Some(b) = v.as_bool() {
        return if b { "1".to_string() } else { String::new() };
    }
    if let Some(i) = v.as_int() {
        return i.to_string();
    }
    if let Some(f) = v.as_float() {
        return f.to_string();
    }
    if let Some(sp) = v.as_string_ptr() {
        let s = unsafe { &*(sp as *const String) };
        return s.clone();
    }
    if v.is_array() {
        return "Array".to_string();
    }
    if v.is_object() {
        return "Object".to_string();
    }
    String::new()
}

fn value_is_falsy(v: &Value) -> bool {
    if v.is_null() {
        return true;
    }
    if let Some(b) = v.as_bool() {
        return !b;
    }
    if let Some(i) = v.as_int() {
        return i == 0;
    }
    if let Some(f) = v.as_float() {
        return f == 0.0;
    }
    if let Some(sp) = v.as_string_ptr() {
        let s = unsafe { &*(sp as *const String) };
        return s.is_empty() || s == "0";
    }
    false
}

fn var_dump_value(
    v: &Value,
    indent: usize,
    ctx: &mut dyn hyperion_core::types::function::NativeContext,
) -> String {
    let mut visited = std::collections::HashSet::new();
    var_dump_value_inner(v, indent, ctx, &mut visited)
}

fn var_dump_value_inner(
    v: &Value,
    indent: usize,
    ctx: &mut dyn hyperion_core::types::function::NativeContext,
    visited: &mut std::collections::HashSet<usize>,
) -> String {
    let pad = " ".repeat(indent);
    if v.is_null() {
        return format!("{}NULL\n", pad);
    }
    if let Some(b) = v.as_bool() {
        return format!("{}bool({})\n", pad, b);
    }
    if let Some(i) = v.as_int() {
        return format!("{}int({})\n", pad, i);
    }
    if let Some(f) = v.as_float() {
        return format!("{}float({})\n", pad, f);
    }
    if let Some(sp) = v.as_string_ptr() {
        let s = unsafe { &*(sp as *const String) };
        return format!("{}string({}) \"{}\"\n", pad, s.len(), s);
    }
    if let Some(ap) = v.as_array_ptr() {
        let addr = ap as usize;
        if visited.contains(&addr) {
            return format!("{}array({}) {{ *RECURSION* }}\n", pad, 0);
        }
        visited.insert(addr);
        let arr = unsafe { &*(ap as *const PhpArray) };
        let mut out = format!("{}array({}) {{\n", pad, arr.len());
        for (key, val) in arr.elements.iter() {
            let k = match key {
                hyperion_core::types::array::ArrayKey::Int(i) => format!("[{}]", i),
                hyperion_core::types::array::ArrayKey::StringId(s) => {
                    format!("[\"{}\"]", ctx.lookup_string(*s).unwrap_or_default())
                }
            };
            out.push_str(&format!("{}  {}=>\n", pad, k));
            out.push_str(&var_dump_value_inner(val, indent + 4, ctx, visited));
        }
        visited.remove(&addr);
        out.push_str(&format!("{}}}\n", pad));
        return out;
    }
    if let Some(op) = v.as_object_ptr() {
        let addr = op as usize;
        let obj = unsafe { &*(op as *const hyperion_core::types::object::PhpObject) };
        let class_name = if let Some(ref name) = obj.class_name {
            name.clone()
        } else {
            ctx.get_class_name(obj.class_id).unwrap_or_else(|| "stdClass".to_string())
        };
        if visited.contains(&addr) {
            return format!("{}object({})#{} (0) {{ *RECURSION* }}\n", pad, class_name, (op as usize) % 1000);
        }
        visited.insert(addr);
        let mut out = format!("{}object({})#{} ({}) {{\n", pad, class_name, (op as usize) % 1000, obj.properties.len());
        for (prop_name, prop_val) in &obj.properties {
            out.push_str(&format!("{}  [\"{}\"]=>\n", pad, prop_name));
            out.push_str(&var_dump_value_inner(prop_val, indent + 4, ctx, visited));
        }
        visited.remove(&addr);
        out.push_str(&format!("{}}}\n", pad));
        return out;
    }
    format!("{}unknown\n", pad)
}

fn print_r_value(
    v: &Value,
    indent: usize,
    ctx: &mut dyn hyperion_core::types::function::NativeContext,
) -> String {
    let mut visited = std::collections::HashSet::new();
    print_r_value_inner(v, indent, ctx, &mut visited)
}

fn print_r_value_inner(
    v: &Value,
    indent: usize,
    ctx: &mut dyn hyperion_core::types::function::NativeContext,
    visited: &mut std::collections::HashSet<usize>,
) -> String {
    let pad = " ".repeat(indent);
    if v.is_null() {
        return String::new();
    }
    if let Some(b) = v.as_bool() {
        return if b { "1".to_string() } else { String::new() };
    }
    if let Some(i) = v.as_int() {
        return i.to_string();
    }
    if let Some(f) = v.as_float() {
        return f.to_string();
    }
    if let Some(sp) = v.as_string_ptr() {
        let s = unsafe { &*(sp as *const String) };
        return s.clone();
    }
    if let Some(ap) = v.as_array_ptr() {
        let addr = ap as usize;
        if visited.contains(&addr) {
            return format!("Array\n{}(\n{} *RECURSION*\n{})\n", pad, pad, pad);
        }
        visited.insert(addr);
        let arr = unsafe { &*(ap as *const PhpArray) };
        let mut out = format!("Array\n{}(\n", pad);
        for (key, val) in arr.elements.iter() {
            let k = match key {
                hyperion_core::types::array::ArrayKey::Int(i) => format!("{}", i),
                hyperion_core::types::array::ArrayKey::StringId(s) => {
                    ctx.lookup_string(*s).unwrap_or_default()
                }
            };
            out.push_str(&format!(
                "{}    [{}] => {}\n",
                pad,
                k,
                print_r_value_inner(val, indent + 8, ctx, visited)
            ));
        }
        visited.remove(&addr);
        out.push_str(&format!("{})\n", pad));
        return out;
    }
    if let Some(op) = v.as_object_ptr() {
        let addr = op as usize;
        let obj = unsafe { &*(op as *const hyperion_core::types::object::PhpObject) };
        let class_name = if let Some(ref name) = obj.class_name {
            name.clone()
        } else {
            ctx.get_class_name(obj.class_id).unwrap_or_else(|| "stdClass".to_string())
        };
        if visited.contains(&addr) {
            return format!("{} Object\n{}(\n{} *RECURSION*\n{})\n", class_name, pad, pad, pad);
        }
        visited.insert(addr);
        let mut out = format!("{} Object\n{}(\n", class_name, pad);
        for (prop_name, prop_val) in &obj.properties {
            out.push_str(&format!(
                "{}    [{}] => {}\n",
                pad,
                prop_name,
                print_r_value_inner(prop_val, indent + 8, ctx, visited)
            ));
        }
        visited.remove(&addr);
        out.push_str(&format!("{})\n", pad));
        return out;
    }
    String::new()
}

fn var_export_value(
    v: &Value,
    indent: usize,
    ctx: &mut dyn hyperion_core::types::function::NativeContext,
) -> String {
    let mut visited = std::collections::HashSet::new();
    var_export_value_inner(v, indent, ctx, &mut visited)
}

fn var_export_value_inner(
    v: &Value,
    indent: usize,
    ctx: &mut dyn hyperion_core::types::function::NativeContext,
    visited: &mut std::collections::HashSet<usize>,
) -> String {
    let pad = " ".repeat(indent);
    if v.is_null() {
        return "NULL".to_string();
    }
    if let Some(b) = v.as_bool() {
        return if b {
            "true".to_string()
        } else {
            "false".to_string()
        };
    }
    if let Some(i) = v.as_int() {
        return i.to_string();
    }
    if let Some(f) = v.as_float() {
        return if f.fract() == 0.0 {
            format!("{:.1}", f)
        } else {
            f.to_string()
        };
    }
    if let Some(sp) = v.as_string_ptr() {
        let s = unsafe { &*(sp as *const String) };
        let escaped = s.replace('\\', "\\\\").replace('\'', "\\'");
        return format!("'{}'", escaped);
    }
    if let Some(ap) = v.as_array_ptr() {
        let addr = ap as usize;
        if visited.contains(&addr) {
            return "NULL /* *RECURSION* */".to_string();
        }
        visited.insert(addr);
        let arr = unsafe { &*(ap as *const hyperion_core::types::array::PhpArray) };
        if arr.elements.is_empty() {
            visited.remove(&addr);
            return "array ()".to_string();
        }
        let mut out = "array (\n".to_string();
        for (k, val) in arr.elements.iter() {
            let rendered = var_export_value_inner(val, indent + 2, ctx, visited);
            match k {
                hyperion_core::types::array::ArrayKey::Int(i) => {
                    out.push_str(&format!("{}  {} => {},\n", pad, i, rendered));
                }
                hyperion_core::types::array::ArrayKey::StringId(sid) => {
                    let s = ctx.lookup_string(*sid).unwrap_or_default();
                    let escaped = s.replace('\\', "\\\\").replace('\'', "\\'");
                    out.push_str(&format!("{}  '{}' => {},\n", pad, escaped, rendered));
                }
            }
        }
        visited.remove(&addr);
        out.push_str(&format!("{})", pad));
        return out;
    }
    if let Some(op) = v.as_object_ptr() {
        let addr = op as usize;
        let obj = unsafe { &*(op as *const hyperion_core::types::object::PhpObject) };
        let class_name = obj
            .class_name
            .clone()
            .unwrap_or_else(|| ctx.get_class_name(obj.class_id).unwrap_or_else(|| "stdClass".to_string()));
        if visited.contains(&addr) {
            return "NULL /* *RECURSION* */".to_string();
        }
        visited.insert(addr);
        let mut out = format!("\\{}::__set_state(array(\n", class_name);
        for (name, val) in obj.properties.iter() {
            let rendered = var_export_value_inner(val, indent + 2, ctx, visited);
            out.push_str(&format!("{}   '{}' => {},\n", pad, name, rendered));
        }
        visited.remove(&addr);
        out.push_str(&format!("{}))", pad));
        return out;
    }
    "NULL".to_string()
}

php_function! {
    native_define(name: Value, value: Value) {
        // We intercept this at the VM level anyway for `CallNative`,
        // but we need it here so the compiler assigns it to the stdlib registry!
        Ok(Value::new_bool(true))
    }
}

pub fn native_exit(
    args: &[Value],
    ctx: &mut dyn hyperion_core::types::function::NativeContext,
) -> Result<Value, String> {
    if let Some(v) = args.first() {
        if let Some(sp) = v.as_string_ptr() {
            let s = unsafe { &*(sp as *const String) };
            ctx.write_output(s.as_bytes());
        }
    }
    Err("__HYPERION_EXIT__".to_string())
}

pub fn native_die(
    args: &[Value],
    ctx: &mut dyn hyperion_core::types::function::NativeContext,
) -> Result<Value, String> {
    native_exit(args, ctx)
}

php_function! {
    native_get_class(obj: Value) |ctx| { let _arena = ctx.get_arena();
        if let Some(o) = obj {
            if let Some(obj_ptr) = o.as_object_ptr() {
                let obj_ref = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
                let class_name = obj_ref
                    .class_name
                    .clone()
                    .unwrap_or_else(|| ctx.get_class_name(obj_ref.class_id).unwrap_or_else(|| "stdClass".to_string()));
                Ok(Value::new_string_ptr(crate::into_raw(Box::new(class_name)) as *mut ()))
            } else if let Some(s_ptr) = o.deref().as_string_ptr() {
                let s = unsafe { &*(s_ptr as *const String) }.clone();
                Ok(Value::new_string_ptr(crate::into_raw(Box::new(s)) as *mut ()))
            } else {
                Ok(Value::new_bool(false))
            }
        } else {
            Ok(Value::new_string_ptr(crate::into_raw(Box::new("UnknownClass".to_string())) as *mut ()))
        }
    }
}

php_function! {
    native_get_declared_classes() |ctx| {
        let classes = ctx.get_declared_classes();
        let mut arr = hyperion_core::types::array::PhpArray::new();
        for name in classes {
            let boxed = crate::into_raw(Box::new(name));
            arr.push(Value::new_string_ptr(boxed as *mut ()));
        }
        let arr_ptr = ctx.get_arena().alloc(arr);
        Ok(Value::new_array_ptr(arr_ptr as *mut ()))
    }
}

php_function! {
    native_get_declared_interfaces() |ctx| {
        let interfaces = ctx.get_declared_interfaces();
        let mut arr = hyperion_core::types::array::PhpArray::new();
        for name in interfaces {
            let boxed = crate::into_raw(Box::new(name));
            arr.push(Value::new_string_ptr(boxed as *mut ()));
        }
        let arr_ptr = ctx.get_arena().alloc(arr);
        Ok(Value::new_array_ptr(arr_ptr as *mut ()))
    }
}

php_function! {
    native_get_declared_traits() |ctx| {
        let traits = ctx.get_declared_traits();
        let mut arr = hyperion_core::types::array::PhpArray::new();
        for name in traits {
            let boxed = crate::into_raw(Box::new(name));
            arr.push(Value::new_string_ptr(boxed as *mut ()));
        }
        let arr_ptr = ctx.get_arena().alloc(arr);
        Ok(Value::new_array_ptr(arr_ptr as *mut ()))
    }
}

pub fn native_closure_bind(
    args: &[Value],
    _ctx: &mut dyn hyperion_core::types::function::NativeContext,
) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Closure::bind() expects at least 1 parameter".to_string());
    }

    let closure_val = args[1]; // args[0] is $this, args[1] is the first explicit argument!

    if closure_val.is_closure() {
        Ok(closure_val)
    } else {
        Err(format!(
            "Closure::bind() expects parameter 1 to be Closure, got {:?}",
            closure_val.get_type()
        ))
    }
}

pub fn native_closure_from_callable(
    args: &[Value],
    ctx: &mut dyn hyperion_core::types::function::NativeContext,
) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Closure::fromCallable() expects exactly 1 parameter, 0 given".to_string());
    }
    ctx.closure_from_callable(args[1])
}

pub fn native_call_user_func(
    args: &[Value],
    ctx: &mut dyn hyperion_core::types::function::NativeContext,
) -> Result<Value, String> {
    if args.is_empty() {
        return Err("call_user_func() expects at least 1 parameter".to_string());
    }
    let callable = args[0];
    let fn_args = args[1..].to_vec();
    ctx.call_callable_synchronously(callable, fn_args)
}

pub fn native_call_user_func_array(
    args: &[Value],
    ctx: &mut dyn hyperion_core::types::function::NativeContext,
) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("call_user_func_array() expects exactly 2 parameters".to_string());
    }
    let callable = args[0];
    let arg_array = args[1];

    let mut fn_args = Vec::new();
    if let Some(arr_ptr) = arg_array.as_array_ptr() {
        let arr = unsafe { &*(arr_ptr as *const hyperion_core::types::array::PhpArray) };
        for (_, val) in arr.elements.iter() {
            fn_args.push(*val);
        }
    } else {
        return Err("call_user_func_array() expects parameter 2 to be array".to_string());
    }

    ctx.call_callable_synchronously(callable, fn_args)
}

pub fn native_forward_static_call(
    args: &[Value],
    ctx: &mut dyn hyperion_core::types::function::NativeContext,
) -> Result<Value, String> {
    if args.is_empty() {
        return Err("forward_static_call() expects at least 1 parameter".to_string());
    }
    let callable = args[0];
    let fn_args = args[1..].to_vec();
    ctx.call_callable_synchronously(callable, fn_args)
}

pub fn native_forward_static_call_array(
    args: &[Value],
    ctx: &mut dyn hyperion_core::types::function::NativeContext,
) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("forward_static_call_array() expects exactly 2 parameters".to_string());
    }
    let callable = args[0];
    let arg_array = args[1];

    let mut fn_args = Vec::new();
    if let Some(arr_ptr) = arg_array.as_array_ptr() {
        let arr = unsafe { &*(arr_ptr as *const hyperion_core::types::array::PhpArray) };
        for (_, val) in arr.elements.iter() {
            fn_args.push(*val);
        }
    } else {
        return Err("forward_static_call_array() expects parameter 2 to be array".to_string());
    }

    ctx.call_callable_synchronously(callable, fn_args)
}
php_function! {
    native_extract(v: Value) |ctx| {
        let count = if let Some(val) = v {
            ctx.extract_variables(*val)
        } else {
            0
        };
        Ok(Value::new_int(count as i32))
    }
}

php_function! {
    native_token_get_all(code_val: Value, _flags_val: Value) |ctx| {
        let code = if let Some(cv) = code_val {
            if let Some(sp) = cv.as_string_ptr() {
                unsafe { &*(sp as *const String) }.clone()
            } else {
                return Err("token_get_all(): Argument #1 must be of type string".to_string());
            }
        } else {
            return Err("token_get_all() expects at least 1 parameter".to_string());
        };

        let mut lexer = hyperion_parser::lexer::Lexer::new(&code);
        lexer.preserve_whitespace = true;
        lexer.preserve_comments = true;
        let mut tokens_arr = hyperion_core::types::array::PhpArray::new();
        let mut idx = 0;
        let mut looking_for_property = false;

        loop {
            let record = lexer.next_token();
            if record.token == hyperion_parser::lexer::Token::Eof {
                break;
            }

            if let hyperion_parser::lexer::Token::InterpolatedString(ref parts) = record.token {
                looking_for_property = false;
                let q_ptr = ctx.get_arena().alloc_and_track("\"".to_string());
                tokens_arr.insert_int(idx, Value::new_string_ptr(q_ptr as *mut ()));
                idx += 1;

                for part in parts {
                    match part {
                        hyperion_parser::lexer::InterpolatedPart::Literal(s) => {
                            let mut single_token_arr = hyperion_core::types::array::PhpArray::new();
                            single_token_arr.insert_int(0, Value::new_int(268)); // T_ENCAPSED_AND_WHITESPACE
                            let text_ptr = ctx.get_arena().alloc_and_track(s.clone());
                            single_token_arr.insert_int(1, Value::new_string_ptr(text_ptr as *mut ()));
                            single_token_arr.insert_int(2, Value::new_int(record.line as i32));
                            let token_arr_ptr = ctx.get_arena().alloc_and_track(single_token_arr);
                            tokens_arr.insert_int(idx, Value::new_array_ptr(token_arr_ptr as *mut ()));
                            idx += 1;
                        }
                        hyperion_parser::lexer::InterpolatedPart::Variable(s) => {
                            let mut single_token_arr = hyperion_core::types::array::PhpArray::new();
                            single_token_arr.insert_int(0, Value::new_int(266)); // T_VARIABLE
                            let var_name = if s.starts_with('$') { s.clone() } else { format!("${}", s) };
                            let text_ptr = ctx.get_arena().alloc_and_track(var_name);
                            single_token_arr.insert_int(1, Value::new_string_ptr(text_ptr as *mut ()));
                            single_token_arr.insert_int(2, Value::new_int(record.line as i32));
                            let token_arr_ptr = ctx.get_arena().alloc_and_track(single_token_arr);
                            tokens_arr.insert_int(idx, Value::new_array_ptr(token_arr_ptr as *mut ()));
                            idx += 1;
                        }
                        hyperion_parser::lexer::InterpolatedPart::Expression(s) => {
                            // 1. T_CURLY_OPEN (400) "{"
                            let mut co_arr = hyperion_core::types::array::PhpArray::new();
                            co_arr.insert_int(0, Value::new_int(400));
                            let t_ptr = ctx.get_arena().alloc_and_track("{".to_string());
                            co_arr.insert_int(1, Value::new_string_ptr(t_ptr as *mut ()));
                            co_arr.insert_int(2, Value::new_int(record.line as i32));
                            let co_ptr = ctx.get_arena().alloc_and_track(co_arr);
                            tokens_arr.insert_int(idx, Value::new_array_ptr(co_ptr as *mut ()));
                            idx += 1;

                            // 2. T_VARIABLE or expression token
                            let var_name = if s.starts_with('$') { s.clone() } else { format!("${}", s) };
                            let mut v_arr = hyperion_core::types::array::PhpArray::new();
                            v_arr.insert_int(0, Value::new_int(266)); // T_VARIABLE
                            let vt_ptr = ctx.get_arena().alloc_and_track(var_name);
                            v_arr.insert_int(1, Value::new_string_ptr(vt_ptr as *mut ()));
                            v_arr.insert_int(2, Value::new_int(record.line as i32));
                            let v_ptr = ctx.get_arena().alloc_and_track(v_arr);
                            tokens_arr.insert_int(idx, Value::new_array_ptr(v_ptr as *mut ()));
                            idx += 1;

                            // 3. Literal "}"
                            let cb_ptr = ctx.get_arena().alloc_and_track("}".to_string());
                            tokens_arr.insert_int(idx, Value::new_string_ptr(cb_ptr as *mut ()));
                            idx += 1;
                        }
                    }
                }

                let q_ptr = ctx.get_arena().alloc_and_track("\"".to_string());
                tokens_arr.insert_int(idx, Value::new_string_ptr(q_ptr as *mut ()));
                idx += 1;
                continue;
            }

            let (is_single_char, mut token_id, default_text) = match &record.token {
                // Comments and whitespace
                hyperion_parser::lexer::Token::Whitespace(s) => (false, 396, s.as_str()),
                hyperion_parser::lexer::Token::Comment(s) => (false, 391, s.as_str()),
                hyperion_parser::lexer::Token::DocComment(s) => (false, 392, s.as_str()),

                // Special
                hyperion_parser::lexer::Token::InlineHtml(s) => (false, 267, s.as_str()),
                hyperion_parser::lexer::Token::OpenTag => (false, 393, "<?php"),
                hyperion_parser::lexer::Token::CloseTag => (false, 395, "?>"),

                // Identifiers & Literals
                hyperion_parser::lexer::Token::Variable(_) => (false, 266, ""),
                hyperion_parser::lexer::Token::Identifier(s) => {
                    let token_id = if s.starts_with('\\') {
                        263 // T_NAME_FULLY_QUALIFIED
                    } else if s.starts_with("namespace\\") {
                        264 // T_NAME_RELATIVE
                    } else if s.contains('\\') {
                        265 // T_NAME_QUALIFIED
                    } else {
                        262 // T_STRING
                    };
                    (false, token_id, s.as_str())
                }
                hyperion_parser::lexer::Token::StringLiteral(_) => (false, 269, ""),
                hyperion_parser::lexer::Token::Integer(_) => (false, 260, ""),
                hyperion_parser::lexer::Token::Float(_) => (false, 261, ""),

                // Keywords
                hyperion_parser::lexer::Token::If => (false, 287, "if"),
                hyperion_parser::lexer::Token::Else => (false, 289, "else"),
                hyperion_parser::lexer::Token::Elseif => (false, 288, "elseif"),
                hyperion_parser::lexer::Token::Endif => (false, 290, "endif"),
                hyperion_parser::lexer::Token::Echo => (false, 291, "echo"),
                hyperion_parser::lexer::Token::Function => (false, 310, "function"),
                hyperion_parser::lexer::Token::Fn => (false, 311, "fn"),
                hyperion_parser::lexer::Token::Return => (false, 313, "return"),
                hyperion_parser::lexer::Token::Class => (false, 336, "class"),
                hyperion_parser::lexer::Token::New => (false, 284, "new"),
                hyperion_parser::lexer::Token::While => (false, 293, "while"),
                hyperion_parser::lexer::Token::Endwhile => (false, 294, "endwhile"),
                hyperion_parser::lexer::Token::Do => (false, 292, "do"),
                hyperion_parser::lexer::Token::For => (false, 295, "for"),
                hyperion_parser::lexer::Token::Endfor => (false, 296, "endfor"),
                hyperion_parser::lexer::Token::Foreach => (false, 297, "foreach"),
                hyperion_parser::lexer::Token::Endforeach => (false, 298, "endforeach"),
                hyperion_parser::lexer::Token::As => (false, 301, "as"),
                hyperion_parser::lexer::Token::Yield => (false, 281, "yield"),
                hyperion_parser::lexer::Token::Switch => (false, 302, "switch"),
                hyperion_parser::lexer::Token::Endswitch => (false, 303, "endswitch"),
                hyperion_parser::lexer::Token::Match => (false, 306, "match"),
                hyperion_parser::lexer::Token::Case => (false, 304, "case"),
                hyperion_parser::lexer::Token::Default => (false, 305, "default"),
                hyperion_parser::lexer::Token::Break => (false, 307, "break"),
                hyperion_parser::lexer::Token::Continue => (false, 308, "continue"),
                hyperion_parser::lexer::Token::Use => (false, 318, "use"),
                hyperion_parser::lexer::Token::Require => (false, 275, "require"),
                hyperion_parser::lexer::Token::Include => (false, 272, "include"),
                hyperion_parser::lexer::Token::RequireOnce => (false, 276, "require_once"),
                hyperion_parser::lexer::Token::IncludeOnce => (false, 273, "include_once"),
                hyperion_parser::lexer::Token::Try => (false, 314, "try"),
                hyperion_parser::lexer::Token::Catch => (false, 315, "catch"),
                hyperion_parser::lexer::Token::Finally => (false, 316, "finally"),
                hyperion_parser::lexer::Token::Throw => (false, 317, "throw"),
                hyperion_parser::lexer::Token::Namespace => (false, 342, "namespace"),
                hyperion_parser::lexer::Token::Trait => (false, 337, "trait"),
                hyperion_parser::lexer::Token::Interface => (false, 338, "interface"),
                hyperion_parser::lexer::Token::Implements => (false, 341, "implements"),
                hyperion_parser::lexer::Token::Extends => (false, 340, "extends"),
                hyperion_parser::lexer::Token::Public => (false, 326, "public"),
                hyperion_parser::lexer::Token::Protected => (false, 325, "protected"),
                hyperion_parser::lexer::Token::Private => (false, 324, "private"),
                hyperion_parser::lexer::Token::Readonly => (false, 330, "readonly"),
                hyperion_parser::lexer::Token::Static => (false, 321, "static"),
                hyperion_parser::lexer::Token::Enum => (false, 339, "enum"),
                hyperion_parser::lexer::Token::Clone => (false, 285, "clone"),
                hyperion_parser::lexer::Token::Declare => (false, 299, "declare"),
                hyperion_parser::lexer::Token::Final => (false, 323, "final"),
                hyperion_parser::lexer::Token::Abstract => (false, 322, "abstract"),
                hyperion_parser::lexer::Token::Const => (false, 312, "const"),
                hyperion_parser::lexer::Token::InstanceOf => (false, 283, "instanceof"),
                hyperion_parser::lexer::Token::Isset => (false, 333, "isset"),
                hyperion_parser::lexer::Token::Empty => (false, 334, "empty"),
                hyperion_parser::lexer::Token::Unset => (false, 332, "unset"),
                hyperion_parser::lexer::Token::List => (false, 343, "list"),
                hyperion_parser::lexer::Token::Print => (false, 280, "print"),
                hyperion_parser::lexer::Token::Eval => (false, 274, "eval"),
                hyperion_parser::lexer::Token::Global => (false, 320, "global"),

                // Multi-character Operators
                hyperion_parser::lexer::Token::ObjectOperator => (false, 388, "->"),
                hyperion_parser::lexer::Token::NullsafeObjectOperator => (false, 389, "?->"),
                hyperion_parser::lexer::Token::DoubleColon => (false, 401, "::"),
                hyperion_parser::lexer::Token::FatArrow => (false, 390, "=>"),
                hyperion_parser::lexer::Token::NullCoalesce => (false, 404, "??"),
                hyperion_parser::lexer::Token::NullCoalesceAssign => (false, 367, "??="),
                hyperion_parser::lexer::Token::Ellipsis => (false, 403, "..."),
                hyperion_parser::lexer::Token::Spaceship => (false, 376, "<=>"),
                hyperion_parser::lexer::Token::StrictEquals => (false, 372, "==="),
                hyperion_parser::lexer::Token::Equals => (false, 370, "=="),
                hyperion_parser::lexer::Token::StrictNotEquals => (false, 373, "!=="),
                hyperion_parser::lexer::Token::NotEquals => (false, 371, "!="),
                hyperion_parser::lexer::Token::LessThanOrEqual => (false, 374, "<="),
                hyperion_parser::lexer::Token::GreaterThanOrEqual => (false, 375, ">="),
                hyperion_parser::lexer::Token::LogicalAnd => (false, 369, "&&"),
                hyperion_parser::lexer::Token::LogicalOr => (false, 368, "||"),
                hyperion_parser::lexer::Token::PlusPlus => (false, 379, "++"),
                hyperion_parser::lexer::Token::MinusMinus => (false, 380, "--"),
                hyperion_parser::lexer::Token::PlusAssign => (false, 356, "+="),
                hyperion_parser::lexer::Token::MinusAssign => (false, 357, "-="),
                hyperion_parser::lexer::Token::MultiplyAssign => (false, 358, "*="),
                hyperion_parser::lexer::Token::DivideAssign => (false, 359, "/="),
                hyperion_parser::lexer::Token::ModuloAssign => (false, 361, "%="),
                hyperion_parser::lexer::Token::DotAssign => (false, 360, ".="),
                hyperion_parser::lexer::Token::BitwiseAndAssign => (false, 362, "&="),
                hyperion_parser::lexer::Token::BitwiseOrAssign => (false, 363, "|="),
                hyperion_parser::lexer::Token::BitwiseXorAssign => (false, 364, "^="),
                hyperion_parser::lexer::Token::ShiftLeftAssign => (false, 365, "<<="),
                hyperion_parser::lexer::Token::ShiftRightAssign => (false, 366, ">>="),
                hyperion_parser::lexer::Token::PowerAssign => (false, 406, "**="),
                hyperion_parser::lexer::Token::Power => (false, 405, "**"),
                hyperion_parser::lexer::Token::ShiftLeft => (false, 377, "<<"),
                hyperion_parser::lexer::Token::ShiftRight => (false, 378, ">>"),
                hyperion_parser::lexer::Token::AttributeOpen => (false, 355, "#["),

                // Single Characters (Returned as literal strings in PHP token_get_all)
                hyperion_parser::lexer::Token::Semicolon => (true, 0, ";"),
                hyperion_parser::lexer::Token::Comma => (true, 0, ","),
                hyperion_parser::lexer::Token::OpenParen => (true, 0, "("),
                hyperion_parser::lexer::Token::CloseParen => (true, 0, ")"),
                hyperion_parser::lexer::Token::OpenBrace => (true, 0, "{"),
                hyperion_parser::lexer::Token::CloseBrace => (true, 0, "}"),
                hyperion_parser::lexer::Token::OpenBracket => (true, 0, "["),
                hyperion_parser::lexer::Token::CloseBracket => (true, 0, "]"),
                hyperion_parser::lexer::Token::Assign => (true, 0, "="),
                hyperion_parser::lexer::Token::Colon => (true, 0, ":"),
                hyperion_parser::lexer::Token::Pipe => (true, 0, "|"),
                hyperion_parser::lexer::Token::Ampersand => (true, 0, "&"),
                hyperion_parser::lexer::Token::QuestionMark => (true, 0, "?"),
                hyperion_parser::lexer::Token::At => (true, 0, "@"),
                hyperion_parser::lexer::Token::Plus => (true, 0, "+"),
                hyperion_parser::lexer::Token::Minus => (true, 0, "-"),
                hyperion_parser::lexer::Token::Multiply => (true, 0, "*"),
                hyperion_parser::lexer::Token::Divide => (true, 0, "/"),
                hyperion_parser::lexer::Token::Modulo => (true, 0, "%"),
                hyperion_parser::lexer::Token::Dot => (true, 0, "."),
                hyperion_parser::lexer::Token::Not => (true, 0, "!"),
                hyperion_parser::lexer::Token::BitwiseXor => (true, 0, "^"),
                hyperion_parser::lexer::Token::LessThan => (true, 0, "<"),
                hyperion_parser::lexer::Token::GreaterThan => (true, 0, ">"),

                _ => (true, 0, ""),
            };

            let token_text = if let Some(ref r) = record.raw {
                r.clone()
            } else if !default_text.is_empty() {
                default_text.to_string()
            } else {
                match &record.token {
                    hyperion_parser::lexer::Token::Variable(s) => format!("${}", s),
                    hyperion_parser::lexer::Token::StringLiteral(s) => format!("'{}'", s),
                    hyperion_parser::lexer::Token::Integer(i) => i.to_string(),
                    hyperion_parser::lexer::Token::Float(f) => f.to_string(),
                    _ => format!("{:?}", record.token),
                }
            };

            let is_ws_or_comment = matches!(
                record.token,
                hyperion_parser::lexer::Token::Whitespace(_)
                    | hyperion_parser::lexer::Token::Comment(_)
                    | hyperion_parser::lexer::Token::DocComment(_)
            );

            if !is_ws_or_comment {
                if looking_for_property {
                    if !is_single_char && token_id != 266 && token_id != 267 {
                        token_id = 262; // T_STRING
                    }
                    looking_for_property = false;
                }

                if matches!(
                    record.token,
                    hyperion_parser::lexer::Token::ObjectOperator
                        | hyperion_parser::lexer::Token::NullsafeObjectOperator
                ) {
                    looking_for_property = true;
                }
            }

            if is_single_char {
                let str_ptr = ctx.get_arena().alloc_and_track(token_text);
                tokens_arr.insert_int(idx, Value::new_string_ptr(str_ptr as *mut ()));
            } else {
                let mut single_token_arr = hyperion_core::types::array::PhpArray::new();
                single_token_arr.insert_int(0, Value::new_int(token_id));
                let text_ptr = ctx.get_arena().alloc_and_track(token_text);
                single_token_arr.insert_int(1, Value::new_string_ptr(text_ptr as *mut ()));
                single_token_arr.insert_int(2, Value::new_int(record.line as i32));
                let token_arr_ptr = ctx.get_arena().alloc_and_track(single_token_arr);
                tokens_arr.insert_int(idx, Value::new_array_ptr(token_arr_ptr as *mut ()));
            }
            idx += 1;
        }

        let res_ptr = ctx.get_arena().alloc_and_track(tokens_arr);
        Ok(Value::new_array_ptr(res_ptr as *mut ()))
    }
}

pub fn native_token_name(args: &[Value], _ctx: &mut dyn hyperion_core::types::function::NativeContext) -> Result<Value, String> {
    let id = args.first().and_then(|v| v.as_int()).unwrap_or(0);
    let name = match id {
        260 => "T_LNUMBER",
        261 => "T_DNUMBER",
        262 => "T_STRING",
        263 => "T_NAME_FULLY_QUALIFIED",
        264 => "T_NAME_RELATIVE",
        265 => "T_NAME_QUALIFIED",
        266 => "T_VARIABLE",
        267 => "T_INLINE_HTML",
        268 => "T_ENCAPSED_AND_WHITESPACE",
        269 => "T_CONSTANT_ENCAPSED_STRING",
        270 => "T_STRING_VARNAME",
        271 => "T_NUM_STRING",
        272 => "T_INCLUDE",
        273 => "T_INCLUDE_ONCE",
        274 => "T_EVAL",
        275 => "T_REQUIRE",
        276 => "T_REQUIRE_ONCE",
        277 => "T_LOGICAL_OR",
        278 => "T_LOGICAL_XOR",
        279 => "T_LOGICAL_AND",
        280 => "T_PRINT",
        281 => "T_YIELD",
        282 => "T_YIELD_FROM",
        283 => "T_INSTANCEOF",
        284 => "T_NEW",
        285 => "T_CLONE",
        286 => "T_EXIT",
        287 => "T_IF",
        288 => "T_ELSEIF",
        289 => "T_ELSE",
        290 => "T_ENDIF",
        291 => "T_ECHO",
        292 => "T_DO",
        293 => "T_WHILE",
        294 => "T_ENDWHILE",
        295 => "T_FOR",
        296 => "T_ENDFOR",
        297 => "T_FOREACH",
        298 => "T_ENDFOREACH",
        299 => "T_DECLARE",
        300 => "T_ENDDECLARE",
        301 => "T_AS",
        302 => "T_SWITCH",
        303 => "T_ENDSWITCH",
        304 => "T_CASE",
        305 => "T_DEFAULT",
        306 => "T_MATCH",
        307 => "T_BREAK",
        308 => "T_CONTINUE",
        309 => "T_GOTO",
        310 => "T_FUNCTION",
        311 => "T_FN",
        312 => "T_CONST",
        313 => "T_RETURN",
        314 => "T_TRY",
        315 => "T_CATCH",
        316 => "T_FINALLY",
        317 => "T_THROW",
        318 => "T_USE",
        319 => "T_INSTEADOF",
        320 => "T_GLOBAL",
        321 => "T_STATIC",
        322 => "T_ABSTRACT",
        323 => "T_FINAL",
        324 => "T_PRIVATE",
        325 => "T_PROTECTED",
        326 => "T_PUBLIC",
        327 => "T_PRIVATE_SET",
        328 => "T_PROTECTED_SET",
        329 => "T_PUBLIC_SET",
        330 => "T_READONLY",
        331 => "T_VAR",
        332 => "T_UNSET",
        333 => "T_ISSET",
        334 => "T_EMPTY",
        335 => "T_HALT_COMPILER",
        336 => "T_CLASS",
        337 => "T_TRAIT",
        338 => "T_INTERFACE",
        339 => "T_ENUM",
        340 => "T_EXTENDS",
        341 => "T_IMPLEMENTS",
        342 => "T_NAMESPACE",
        343 => "T_LIST",
        344 => "T_ARRAY",
        345 => "T_CALLABLE",
        346 => "T_LINE",
        347 => "T_FILE",
        348 => "T_DIR",
        349 => "T_CLASS_C",
        350 => "T_TRAIT_C",
        351 => "T_METHOD_C",
        352 => "T_FUNC_C",
        353 => "T_PROPERTY_C",
        354 => "T_NS_C",
        355 => "T_ATTRIBUTE",
        356 => "T_PLUS_EQUAL",
        357 => "T_MINUS_EQUAL",
        358 => "T_MUL_EQUAL",
        359 => "T_DIV_EQUAL",
        360 => "T_CONCAT_EQUAL",
        361 => "T_MOD_EQUAL",
        362 => "T_AND_EQUAL",
        363 => "T_OR_EQUAL",
        364 => "T_XOR_EQUAL",
        365 => "T_SL_EQUAL",
        366 => "T_SR_EQUAL",
        367 => "T_COALESCE_EQUAL",
        368 => "T_BOOLEAN_OR",
        369 => "T_BOOLEAN_AND",
        370 => "T_IS_EQUAL",
        371 => "T_IS_NOT_EQUAL",
        372 => "T_IS_IDENTICAL",
        373 => "T_IS_NOT_IDENTICAL",
        374 => "T_IS_SMALLER_OR_EQUAL",
        375 => "T_IS_GREATER_OR_EQUAL",
        376 => "T_SPACESHIP",
        377 => "T_SL",
        378 => "T_SR",
        379 => "T_INC",
        380 => "T_DEC",
        381 => "T_INT_CAST",
        382 => "T_DOUBLE_CAST",
        383 => "T_STRING_CAST",
        384 => "T_ARRAY_CAST",
        385 => "T_OBJECT_CAST",
        386 => "T_BOOL_CAST",
        387 => "T_UNSET_CAST",
        388 => "T_OBJECT_OPERATOR",
        389 => "T_NULLSAFE_OBJECT_OPERATOR",
        390 => "T_DOUBLE_ARROW",
        391 => "T_COMMENT",
        392 => "T_DOC_COMMENT",
        393 => "T_OPEN_TAG",
        394 => "T_OPEN_TAG_WITH_ECHO",
        395 => "T_CLOSE_TAG",
        396 => "T_WHITESPACE",
        397 => "T_START_HEREDOC",
        398 => "T_END_HEREDOC",
        399 => "T_DOLLAR_OPEN_CURLY_BRACES",
        400 => "T_CURLY_OPEN",
        401 => "T_DOUBLE_COLON",
        402 => "T_NS_SEPARATOR",
        403 => "T_ELLIPSIS",
        404 => "T_COALESCE",
        405 => "T_POW",
        406 => "T_POW_EQUAL",
        407 => "T_AMPERSAND_FOLLOWED_BY_VAR_OR_VARARG",
        408 => "T_AMPERSAND_NOT_FOLLOWED_BY_VAR_OR_VARARG",
        409 => "T_BAD_CHARACTER",
        _ => "UNKNOWN",
    };
    let str_ptr = _ctx.get_arena().alloc_and_track(name.to_string());
    Ok(Value::new_string_ptr(str_ptr as *mut ()))
}


fn serialize_value(val: Value, ctx: &mut dyn hyperion_core::types::function::NativeContext, out: &mut String) {
    if val.is_null() {
        out.push_str("N;");
    } else if let Some(b) = val.as_bool() {
        out.push_str(&format!("b:{};", if b { 1 } else { 0 }));
    } else if let Some(i) = val.as_int() {
        out.push_str(&format!("i:{};", i));
    } else if let Some(f) = val.as_float() {
        out.push_str(&format!("d:{};", f));
    } else if let Some(sp) = val.as_string_ptr() {
        let s = unsafe { &*(sp as *const String) };
        out.push_str(&format!("s:{}:\"{}\";", s.len(), s));
    } else if let Some(ap) = val.as_array_ptr() {
        let arr = unsafe { &*(ap as *const hyperion_core::types::array::PhpArray) };
        out.push_str(&format!("a:{}:{{", arr.elements.len()));
        for (k, v) in &arr.elements {
            match k {
                hyperion_core::types::array::ArrayKey::Int(i) => {
                    out.push_str(&format!("i:{};", i));
                }
                hyperion_core::types::array::ArrayKey::StringId(sid) => {
                    let s = ctx.lookup_string(*sid).unwrap_or_default();
                    out.push_str(&format!("s:{}:\"{}\";", s.len(), s));
                }
            }
            serialize_value(*v, ctx, out);
        }
        out.push('}');
    } else if let Some(op) = val.as_object_ptr() {
        let obj = unsafe { &*(op as *const hyperion_core::types::object::PhpObject) };
        let class_name = if let Some(ref name) = obj.class_name {
            name.clone()
        } else if let Some(name) = ctx.get_class_name(obj.class_id) {
            name
        } else {
            "stdClass".to_string()
        };
        out.push_str(&format!("O:{}:\"{}\":{}:{{", class_name.len(), class_name, obj.properties.len()));
        for (k, v) in &obj.properties {
            out.push_str(&format!("s:{}:\"{}\";", k.len(), k));
            serialize_value(*v, ctx, out);
        }
        out.push('}');
    } else {
        out.push_str("N;");
    }
}

fn parse_serialized(bytes: &[u8], pos: &mut usize, ctx: &mut dyn hyperion_core::types::function::NativeContext) -> Option<Value> {
    if *pos >= bytes.len() {
        return None;
    }
    match bytes[*pos] {
        b'N' => {
            if *pos + 1 < bytes.len() && bytes[*pos + 1] == b';' {
                *pos += 2;
                Some(Value::null())
            } else {
                None
            }
        }
        b'b' => {
            if *pos + 3 < bytes.len() && bytes[*pos + 1] == b':' && bytes[*pos + 3] == b';' {
                let b = bytes[*pos + 2] == b'1';
                *pos += 4;
                Some(Value::new_bool(b))
            } else {
                None
            }
        }
        b'i' => {
            *pos += 2;
            let start = *pos;
            while *pos < bytes.len() && bytes[*pos] != b';' {
                *pos += 1;
            }
            let s = std::str::from_utf8(&bytes[start..*pos]).ok()?;
            *pos += 1;
            let i: i64 = s.parse().ok()?;
            Some(Value::new_int(i as i32))
        }
        b'd' => {
            *pos += 2;
            let start = *pos;
            while *pos < bytes.len() && bytes[*pos] != b';' {
                *pos += 1;
            }
            let s = std::str::from_utf8(&bytes[start..*pos]).ok()?;
            *pos += 1;
            let f: f64 = s.parse().ok()?;
            Some(Value::new_float(f))
        }
        b's' => {
            *pos += 2;
            let start = *pos;
            while *pos < bytes.len() && bytes[*pos] != b':' {
                *pos += 1;
            }
            let len_str = std::str::from_utf8(&bytes[start..*pos]).ok()?;
            let len: usize = len_str.parse().ok()?;
            *pos += 2;
            if *pos + len <= bytes.len() {
                let str_bytes = &bytes[*pos..(*pos + len)];
                *pos += len + 2;
                let s = String::from_utf8_lossy(str_bytes).to_string();
                let s_ptr = crate::into_raw(Box::new(s));
                Some(Value::new_string_ptr(s_ptr as *mut ()))
            } else {
                None
            }
        }
        b'a' => {
            *pos += 2;
            let start = *pos;
            while *pos < bytes.len() && bytes[*pos] != b':' {
                *pos += 1;
            }
            let count_str = std::str::from_utf8(&bytes[start..*pos]).ok()?;
            let count: usize = count_str.parse().ok()?;
            *pos += 2;
            let mut arr = PhpArray::new();
            for _ in 0..count {
                let key_val = parse_serialized(bytes, pos, ctx)?;
                let val = parse_serialized(bytes, pos, ctx)?;
                if let Some(i) = key_val.as_int() {
                    arr.insert_int(i as i64, val);
                } else if let Some(sp) = key_val.as_string_ptr() {
                    let s = unsafe { &*(sp as *const String) };
                    let sid = ctx.intern_string(s);
                    arr.insert_string_id(sid, val);
                }
            }
            if *pos < bytes.len() && bytes[*pos] == b'}' {
                *pos += 1;
            }
            let ptr = ctx.get_arena().alloc_and_track(arr);
            Some(Value::new_array_ptr(ptr as *mut ()))
        }
        b'O' => {
            *pos += 2;
            let start = *pos;
            while *pos < bytes.len() && bytes[*pos] != b':' {
                *pos += 1;
            }
            let len_str = std::str::from_utf8(&bytes[start..*pos]).ok()?;
            let name_len: usize = len_str.parse().ok()?;
            *pos += 2; // skip :\"
            if *pos + name_len > bytes.len() {
                return None;
            }
            let class_name = String::from_utf8_lossy(&bytes[*pos..*pos + name_len]).to_string();
            *pos += name_len + 2; // skip name and \":
            let prop_start = *pos;
            while *pos < bytes.len() && bytes[*pos] != b':' {
                *pos += 1;
            }
            let prop_count_str = std::str::from_utf8(&bytes[prop_start..*pos]).ok()?;
            let prop_count: usize = prop_count_str.parse().ok()?;
            *pos += 2; // skip :{

            let mut class_id = ctx.get_class_id(&class_name).unwrap_or(0);
            if class_id == 0 {
                ctx.trigger_autoload_sync(&class_name);
                class_id = ctx.get_class_id(&class_name).unwrap_or(0);
            }
            let mut obj = hyperion_core::types::object::PhpObject::new(class_id);
            obj.class_name = Some(class_name);

            for _ in 0..prop_count {
                let key_val = parse_serialized(bytes, pos, ctx)?;
                let val = parse_serialized(bytes, pos, ctx)?;
                if let Some(sp) = key_val.as_string_ptr() {
                    let mut prop_name = unsafe { &*(sp as *const String) }.clone();
                    if let Some(last_null) = prop_name.rfind('\0') {
                        prop_name = prop_name[last_null + 1..].to_string();
                    }
                    obj.properties.insert(prop_name, val);
                }
            }
            if *pos < bytes.len() && bytes[*pos] == b'}' {
                *pos += 1;
            }
            let ptr = ctx.get_arena().alloc_and_track(obj);
            Some(Value::new_object_ptr(ptr as *mut ()))
        }
        _ => None,
    }
}

php_function! {
    native_serialize(v: Value) |ctx| {
        let Some(val) = v else { return Ok(Value::null()) };
        let mut out = String::new();
        serialize_value(*val, ctx, &mut out);
        let s_ptr = crate::into_raw(Box::new(out));
        Ok(Value::new_string_ptr(s_ptr as *mut ()))
    }
}

php_function! {
    native_unserialize(data: Value, _options: Value) |ctx| {
        let Some(data_val) = data else { return Ok(Value::new_bool(false)) };
        let Some(sp) = data_val.as_string_ptr() else { return Ok(Value::new_bool(false)) };
        let s = unsafe { &*(sp as *const String) };
        let mut pos = 0;
        match parse_serialized(s.as_bytes(), &mut pos, ctx) {
            Some(v) => Ok(v),
            None => Ok(Value::new_bool(false)),
        }
    }
}

php_function! {
    native_get_defined_vars() |ctx| {
        let vars = ctx.get_defined_vars();
        let mut arr = PhpArray::new();
        for (name, val) in vars {
            let id = ctx.intern_string(&name);
            arr.insert_string_id(id, val);
        }
        let ptr = ctx.get_arena().alloc_and_track(arr);
        Ok(Value::new_array_ptr(ptr as *mut ()))
    }
}

php_function! {
    native_ip2long(ip_val: Value) {
        let Some(ip_val) = ip_val else { return Ok(Value::new_bool(false)) };
        let Some(sp) = ip_val.as_string_ptr() else { return Ok(Value::new_bool(false)) };
        let s = unsafe { &*(sp as *const String) };
        if let Ok(ipv4) = s.parse::<std::net::Ipv4Addr>() {
            let num = u32::from_be_bytes(ipv4.octets()) as i32;
            Ok(Value::new_int(num))
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_long2ip(proper_address: Value) |ctx| {
        let Some(val) = proper_address else { return Ok(Value::new_bool(false)) };
        let num = if let Some(i) = val.as_int() {
            i as u32
        } else if let Some(sp) = val.as_string_ptr() {
            let s = unsafe { &*(sp as *const String) };
            s.parse::<i64>().unwrap_or(0) as u32
        } else {
            0u32
        };
        let ip = std::net::Ipv4Addr::from(num.to_be_bytes());
        let out = ip.to_string();
        let s_ptr = crate::into_raw(Box::new(out));
        Ok(Value::new_string_ptr(s_ptr as *mut ()))
    }
}

php_function! {
    native_inet_pton(family: Value, address: Value) |ctx| {
        let Some(addr_val) = address else { return Ok(Value::new_bool(false)) };
        let Some(sp) = addr_val.as_string_ptr() else { return Ok(Value::new_bool(false)) };
        let s = unsafe { &*(sp as *const String) };
        
        let fam = family.and_then(|f| f.as_int()).unwrap_or(2); // 2 = AF_INET
        if fam == 2 {
            if let Ok(ipv4) = s.parse::<std::net::Ipv4Addr>() {
                let bytes = ipv4.octets();
                let out = unsafe { String::from_utf8_unchecked(bytes.to_vec()) };
                let s_ptr = crate::into_raw(Box::new(out));
                return Ok(Value::new_string_ptr(s_ptr as *mut ()));
            }
        } else {
            if let Ok(ipv6) = s.parse::<std::net::Ipv6Addr>() {
                let bytes = ipv6.octets();
                let out = unsafe { String::from_utf8_unchecked(bytes.to_vec()) };
                let s_ptr = crate::into_raw(Box::new(out));
                return Ok(Value::new_string_ptr(s_ptr as *mut ()));
            }
        }
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_inet_ntop(family: Value, address: Value) |ctx| {
        let Some(addr_val) = address else { return Ok(Value::new_bool(false)) };
        let Some(sp) = addr_val.as_string_ptr() else { return Ok(Value::new_bool(false)) };
        let s = unsafe { &*(sp as *const String) };
        let bytes = s.as_bytes();
        if bytes.len() == 4 {
            let mut arr = [0u8; 4];
            arr.copy_from_slice(bytes);
            let ip = std::net::Ipv4Addr::from(arr);
            let out = ip.to_string();
            let s_ptr = crate::into_raw(Box::new(out));
            return Ok(Value::new_string_ptr(s_ptr as *mut ()));
        } else if bytes.len() == 16 {
            let mut arr = [0u8; 16];
            arr.copy_from_slice(bytes);
            let ip = std::net::Ipv6Addr::from(arr);
            let out = ip.to_string();
            let s_ptr = crate::into_raw(Box::new(out));
            return Ok(Value::new_string_ptr(s_ptr as *mut ()));
        }
        Ok(Value::new_bool(false))
    }
}


