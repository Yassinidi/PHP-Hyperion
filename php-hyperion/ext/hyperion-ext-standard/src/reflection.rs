use hyperion_core::memory::nan_box::Value;
use hyperion_core::php_function;
use hyperion_core::types::array::PhpArray;

php_function! {
    // `this` is args[0]; args[1] is the closure or function name being reflected.
    native_reflection_function_construct(this: Value, target: Value) |ctx| {
        let (Some(this), Some(target)) = (this, target) else {
            return Ok(Value::null());
        };
        if !this.is_object() {
            return Ok(Value::null());
        }
        let obj_ptr = this.as_object_ptr().unwrap() as *mut hyperion_core::types::object::PhpObject;

        // Closures have no user-visible name in PHP; they report {closure}.
        let target = *target;
        let is_closure = target.is_closure();
        let (name, is_static, scope_class) = match ctx.get_callable_info(target) {
            Some((name, is_static, scope, _)) => (name, is_static, scope),
            None => (String::new(), false, None),
        };
        let display_name = if is_closure { "{closure}".to_string() } else { name };

        unsafe {
            (*obj_ptr).properties.insert("name".to_string(), php_string(display_name));
            (*obj_ptr).properties.insert("anonymous".to_string(), Value::new_bool(is_closure));
            (*obj_ptr).properties.insert("static".to_string(), Value::new_bool(is_static));
            (*obj_ptr).properties.insert("callable".to_string(), target);
            if let Some(scope) = scope_class {
                (*obj_ptr).properties.insert("scope_class".to_string(), php_string(scope));
            }
        }
        Ok(Value::null())
    }
}

php_function! {
    native_reflection_function_get_name(this: Value) {
        Ok(php_string(obj_string_prop(this, "name").unwrap_or_default()))
    }
}

php_function! {
    native_reflection_function_is_anonymous(this: Value) {
        Ok(Value::new_bool(obj_bool_prop(this, "anonymous").unwrap_or(false)))
    }
}

php_function! {
    native_reflection_function_is_static(this: Value) {
        Ok(Value::new_bool(obj_bool_prop(this, "static").unwrap_or(false)))
    }
}

php_function! {
    native_reflection_function_get_closure_scope_class(this: Value) |ctx| {
        // PHP returns a ReflectionClass for the closure's bound scope, or null
        // when the closure was declared outside any class.
        let (Some(scope), Some(class_id)) =
            (obj_string_prop(this, "scope_class"), ctx.get_class_id("ReflectionClass"))
        else {
            return Ok(Value::null());
        };
        let mut obj = hyperion_core::types::object::PhpObject::new(class_id);
        obj.properties.insert("name".to_string(), php_string(scope));
        let obj_ptr = ctx.get_arena().alloc_and_track(obj);
        Ok(Value::new_object_ptr(obj_ptr as *mut ()))
    }
}

php_function! {
    native_reflection_function_get_closure_called_class(this: Value) |ctx| {
        let (Some(scope), Some(class_id)) =
            (obj_string_prop(this, "scope_class"), ctx.get_class_id("ReflectionClass"))
        else {
            return Ok(Value::null());
        };
        let mut obj = hyperion_core::types::object::PhpObject::new(class_id);
        obj.properties.insert("name".to_string(), php_string(scope));
        let obj_ptr = ctx.get_arena().alloc_and_track(obj);
        Ok(Value::new_object_ptr(obj_ptr as *mut ()))
    }
}

php_function! {
    native_reflection_function_get_closure_this(this: Value) {
        Ok(Value::null())
    }
}

php_function! {
    native_reflection_function_get_file_name(this: Value) {
        Ok(php_string("unknown.php".to_string()))
    }
}

php_function! {
    native_reflection_function_get_start_line(this: Value) {
        Ok(Value::new_int(1))
    }
}

php_function! {
    native_reflection_function_get_end_line(this: Value) {
        Ok(Value::new_int(1))
    }
}

php_function! {
    native_reflection_function_get_closure_used_variables(this: Value) |ctx| {
        let mut arr = PhpArray::new();
        let Some(this) = this else {
            let arr_ptr = ctx.get_arena().alloc_and_track(arr);
            return Ok(Value::new_array_ptr(arr_ptr as *mut ()));
        };
        let Some(obj_ptr) = this.as_object_ptr() else {
            let arr_ptr = ctx.get_arena().alloc_and_track(arr);
            return Ok(Value::new_array_ptr(arr_ptr as *mut ()));
        };
        let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
        if let Some(target) = obj.properties.get("callable") {
            let used = ctx.get_closure_used_variables(*target);
            for (name, val) in used {
                let sid = ctx.intern_string(&name);
                arr.insert_string_id(sid, val);
            }
        }
        let arr_ptr = ctx.get_arena().alloc_and_track(arr);
        Ok(Value::new_array_ptr(arr_ptr as *mut ()))
    }
}

php_function! {
    native_reflection_function_invoke(this: Value, ...rest) |ctx| {
        let Some(this) = this else { return Ok(Value::null()) };
        if !this.is_object() {
            return Ok(Value::null());
        }
        let obj_ptr = this.as_object_ptr().unwrap() as *const hyperion_core::types::object::PhpObject;
        let Some(callable) = (unsafe { (*obj_ptr).properties.get("callable") }).copied() else {
            return Ok(Value::null());
        };
        ctx.call_callable_synchronously(callable, rest.to_vec())
    }
}

php_function! {
    native_reflection_function_get_parameters(this: Value) |ctx| {
        let mut arr = PhpArray::new();
        let callable = this
            .filter(|t| t.is_object())
            .and_then(|t| {
                let obj_ptr = t.as_object_ptr()? as *const hyperion_core::types::object::PhpObject;
                unsafe { (*obj_ptr).properties.get("callable") }.copied()
            });

        if let (Some(callable), Some(param_class_id)) =
            (callable, ctx.get_class_id("ReflectionParameter"))
        {
            let name = obj_string_prop(this, "name").unwrap_or_default();
            if let Some((_, _, _, params)) = ctx.get_callable_info(callable) {
                for (i, param) in params.iter().enumerate() {
                    let mut p = hyperion_core::types::object::PhpObject::new(param_class_id);
                    p.properties.insert("name".to_string(), php_string(param.0.clone()));
                    p.properties.insert("has_default".to_string(), Value::new_bool(param.2));
                    p.properties.insert("by_ref".to_string(), Value::new_bool(param.3));
                    p.properties.insert("variadic".to_string(), Value::new_bool(param.4));
                    p.properties.insert("position".to_string(), Value::new_int(i as i32));
                    p.properties.insert("declaring_function".to_string(), php_string(name.clone()));
                    if let Some(ref def_val) = param.5 {
                        p.properties.insert("default".to_string(), *def_val);
                    }
                    if let Some(type_hint) = &param.1 {
                        p.properties.insert("type".to_string(), php_string(type_hint.clone()));
                    }
                    let ptr = ctx.get_arena().alloc_and_track(p);
                    arr.insert_int(i as i64, Value::new_object_ptr(ptr as *mut ()));
                }
            }
        }
        let arr_ptr = ctx.get_arena().alloc_and_track(arr);
        Ok(Value::new_array_ptr(arr_ptr as *mut ()))
    }
}

php_function! {
    native_reflection_function_is_variadic(this: Value) |ctx| {
        let callable = this
            .filter(|t| t.is_object())
            .and_then(|t| {
                let obj_ptr = t.as_object_ptr()? as *const hyperion_core::types::object::PhpObject;
                unsafe { (*obj_ptr).properties.get("callable") }.copied()
            });
        if let Some(callable) = callable {
            if let Some((_, _, _, params)) = ctx.get_callable_info(callable) {
                for param in params {
                    if param.4 {
                        return Ok(Value::new_bool(true));
                    }
                }
            }
        }
        Ok(Value::new_bool(false))
    }
}

/// Read the subject class name stored on a Reflection* object by its constructor.
fn reflected_class_name(this: Option<&Value>) -> Option<String> {
    let this = this?;
    if !this.is_object() {
        return None;
    }
    let obj_ptr = this.as_object_ptr()? as *const hyperion_core::types::object::PhpObject;
    let name_val = unsafe { (*obj_ptr).properties.get("name") }?;
    let name_ptr = name_val.as_string_ptr()?;
    Some(unsafe { (*(name_ptr as *const String)).clone() })
}

/// Read a string property off a Reflection* object.
fn obj_string_prop(this: Option<&Value>, key: &str) -> Option<String> {
    let this = this?;
    if !this.is_object() {
        return None;
    }
    let obj_ptr = this.as_object_ptr()? as *const hyperion_core::types::object::PhpObject;
    let val = unsafe { (*obj_ptr).properties.get(key) }?;
    let ptr = val.as_string_ptr()?;
    Some(unsafe { (*(ptr as *const String)).clone() })
}

/// Read an int property off a Reflection* object.
fn obj_int_prop(this: Option<&Value>, key: &str) -> Option<i32> {
    let this = this?;
    if !this.is_object() {
        return None;
    }
    let obj_ptr = this.as_object_ptr()? as *const hyperion_core::types::object::PhpObject;
    unsafe { (*obj_ptr).properties.get(key) }?.as_int()
}

/// Read a bool property off a Reflection* object.
fn obj_bool_prop(this: Option<&Value>, key: &str) -> Option<bool> {
    let this = this?;
    if !this.is_object() {
        return None;
    }
    let obj_ptr = this.as_object_ptr()? as *const hyperion_core::types::object::PhpObject;
    unsafe { (*obj_ptr).properties.get(key) }?.as_bool()
}

/// Allocate a PHP string Value.
fn php_string(s: String) -> Value {
    Value::new_string_ptr(crate::into_raw(Box::new(s)) as *mut ())
}

/// PHP's builtin (non-class) type names. Everything else is a class/interface,
/// which is what Laravel's container keys auto-wiring off of.
fn is_builtin_type(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "int" | "float" | "string" | "bool" | "array" | "object" | "mixed"
            | "callable" | "iterable" | "void" | "null" | "never" | "false"
            | "true" | "self" | "static" | "parent"
    )
}

php_function! {
    // `this` is args[0]: add_native_method pushes $this ahead of the declared args.
    native_reflection_class_construct(this: Value, name: Value) |ctx| {
        let this = match this {
            Some(this) if this.is_object() => this,
            _ => return Ok(Value::null()),
        };
        let obj_ptr = this.as_object_ptr().unwrap() as *mut hyperion_core::types::object::PhpObject;

        // Accept both `new ReflectionClass('Foo')` and `new ReflectionClass($obj)`.
        let deref_name = name.map(|n| n.deref());
        let class_name = match deref_name {
            Some(val) if val.is_object() => {
                let arg_ptr = val.as_object_ptr().unwrap()
                    as *const hyperion_core::types::object::PhpObject;
                let php_obj = unsafe { &*arg_ptr };
                if let Some(ref custom_name) = php_obj.class_name {
                    Some(custom_name.clone())
                } else {
                    ctx.get_class_name(php_obj.class_id)
                }
            }
            Some(val) => val
                .as_string_ptr()
                .map(|ptr| unsafe { (*(ptr as *const String)).clone() }),
            None => None,
        };

        let Some(class_name) = class_name else {
            return Err("ReflectionException: ReflectionClass::__construct() expects a class name or object".to_string());
        };

        let mut to_load = Some(class_name.clone());
        while let Some(current) = to_load {
            if !ctx.class_exists(&current) {
                ctx.trigger_autoload_sync(&current);
            }
            to_load = ctx.get_parent_class(&current);
        }

        if !ctx.class_exists(&class_name) {
            return Err(format!("ReflectionException: Class \"{}\" does not exist", class_name));
        }

        let boxed = crate::into_raw(Box::new(class_name));
        unsafe {
            (*obj_ptr)
                .properties
                .insert("name".to_string(), Value::new_string_ptr(boxed as *mut ()));
        }
        Ok(Value::null())
    }
}

php_function! {
    native_reflection_class_get_name(this: Value) {
        let name = reflected_class_name(this).unwrap_or_default();
        Ok(Value::new_string_ptr(crate::into_raw(Box::new(name)) as *mut ()))
    }
}

php_function! {
    native_reflection_class_get_parent_class(this: Value) |ctx| {
        // PHP returns a ReflectionClass for the parent, or false when there is none.
        let Some(parent) = reflected_class_name(this).and_then(|n| ctx.get_parent_class(&n)) else {
            return Ok(Value::new_bool(false));
        };
        let Some(class_id) = ctx.get_class_id("ReflectionClass") else {
            return Ok(Value::new_bool(false));
        };
        let mut obj = hyperion_core::types::object::PhpObject::new(class_id);
        let boxed = crate::into_raw(Box::new(parent));
        obj.properties
            .insert("name".to_string(), Value::new_string_ptr(boxed as *mut ()));
        let obj_ptr = ctx.get_arena().alloc_and_track(obj);
        Ok(Value::new_object_ptr(obj_ptr as *mut ()))
    }
}

php_function! {
    native_reflection_class_implements_interface(this: Value, interface: Value) |ctx| {
        let (Some(class_name), Some(interface_name)) = (
            reflected_class_name(this),
            interface.and_then(|i| i.as_string_ptr())
                .map(|ptr| unsafe { (*(ptr as *const String)).clone() }),
        ) else {
            return Ok(Value::new_bool(false));
        };
        Ok(Value::new_bool(ctx.implements_interface(&class_name, &interface_name)))
    }
}

php_function! {
    native_reflection_class_is_instantiable(this: Value) |ctx| {
        // Only concrete, existing classes are instantiable.
        let Some(class_name) = reflected_class_name(this) else {
            return Ok(Value::new_bool(false));
        };
        Ok(Value::new_bool(ctx.class_exists(&class_name)))
    }
}

php_function! {
    native_reflection_class_get_constructor(this: Value) |ctx| {
        // PHP returns null when the class declares no constructor.
        let Some(class_name) = reflected_class_name(this) else {
            return Ok(Value::null());
        };
        if !ctx.has_method(&class_name, "__construct") {
            return Ok(Value::null());
        }
        let Some(method_class_id) = ctx.get_class_id("ReflectionMethod") else {
            return Ok(Value::null());
        };

        let mut obj = hyperion_core::types::object::PhpObject::new(method_class_id);
        let class_boxed = crate::into_raw(Box::new(class_name));
        obj.properties.insert(
            "class".to_string(),
            Value::new_string_ptr(class_boxed as *mut ()),
        );
        let method_name = crate::into_raw(Box::new("__construct".to_string()));
        obj.properties.insert(
            "name".to_string(),
            Value::new_string_ptr(method_name as *mut ()),
        );
        let obj_ptr = ctx.get_arena().alloc_and_track(obj);
        Ok(Value::new_object_ptr(obj_ptr as *mut ()))
    }
}

php_function! {
    native_reflection_class_get_methods(this: Value, filter: Value) |ctx| {
        let mut arr = PhpArray::new();
        let Some(class_name) = reflected_class_name(this) else {
            let arr_ptr = ctx.get_arena().alloc_and_track(arr);
            return Ok(Value::new_array_ptr(arr_ptr as *mut ()));
        };

        if let Some(methods) = ctx.get_class_methods(&class_name) {
            if let Some(method_class_id) = ctx.get_class_id("ReflectionMethod") {
                for (idx, method_name) in methods.into_iter().enumerate() {
                    let mut obj = hyperion_core::types::object::PhpObject::new(method_class_id);
                    let declaring_class = ctx.get_method_declaring_class(&class_name, &method_name).unwrap_or_else(|| class_name.clone());
                    let class_boxed = crate::into_raw(Box::new(declaring_class));
                    obj.properties.insert(
                        "class".to_string(),
                        Value::new_string_ptr(class_boxed as *mut ()),
                    );
                    let method_boxed = crate::into_raw(Box::new(method_name));
                    obj.properties.insert(
                        "name".to_string(),
                        Value::new_string_ptr(method_boxed as *mut ()),
                    );
                    let obj_ptr = ctx.get_arena().alloc_and_track(obj);
                    arr.insert_int(idx as i64, Value::new_object_ptr(obj_ptr as *mut ()));
                }
            }
        }

        let arr_ptr = ctx.get_arena().alloc_and_track(arr);
        Ok(Value::new_array_ptr(arr_ptr as *mut ()))
    }
}

php_function! {
    native_reflection_class_get_method(this: Value, name: Value) |ctx| {
        let Some(class_name) = reflected_class_name(this) else {
            return Err("ReflectionException: ReflectionClass::getMethod(): Internal error".to_string());
        };
        let Some(method_name) = name.and_then(|n| n.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() }) else {
            return Err("ReflectionException: ReflectionClass::getMethod() expects parameter 1 to be string".to_string());
        };
        if !ctx.has_method(&class_name, &method_name) {
            return Err(format!("ReflectionException: Method {}::{}() does not exist", class_name, method_name));
        }
        let Some(method_class_id) = ctx.get_class_id("ReflectionMethod") else {
            return Ok(Value::null());
        };

        let mut obj = hyperion_core::types::object::PhpObject::new(method_class_id);
        let declaring_class = ctx.get_method_declaring_class(&class_name, &method_name).unwrap_or_else(|| class_name.clone());
        let class_boxed = crate::into_raw(Box::new(declaring_class));
        obj.properties.insert(
            "class".to_string(),
            Value::new_string_ptr(class_boxed as *mut ()),
        );
        let method_boxed = crate::into_raw(Box::new(method_name));
        obj.properties.insert(
            "name".to_string(),
            Value::new_string_ptr(method_boxed as *mut ()),
        );
        let obj_ptr = ctx.get_arena().alloc_and_track(obj);
        Ok(Value::new_object_ptr(obj_ptr as *mut ()))
    }
}

php_function! {
    native_reflection_class_has_method(this: Value, name: Value) |ctx| {
        let Some(class_name) = reflected_class_name(this) else {
            return Ok(Value::new_bool(false));
        };
        let Some(method_name) = name.and_then(|n| n.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() }) else {
            return Ok(Value::new_bool(false));
        };
        Ok(Value::new_bool(ctx.has_method(&class_name, &method_name)))
    }
}

fn is_instance_of_class(ctx: &mut dyn hyperion_core::types::function::NativeContext, class_name: &str, target: &str) -> bool {
    let norm_class = class_name.trim_start_matches('\\');
    let norm_target = target.trim_start_matches('\\');
    if norm_class.eq_ignore_ascii_case(norm_target) {
        return true;
    }
    let parents = ctx.get_parent_classes(norm_class);
    if parents.iter().any(|p| p.trim_start_matches('\\').eq_ignore_ascii_case(norm_target)) {
        return true;
    }
    if ctx.implements_interface(norm_class, norm_target) {
        return true;
    }
    false
}

php_function! {
    native_reflection_class_get_attributes(this: Value, name: Value, flags: Value) |ctx| {
        let mut arr = PhpArray::new();
        let Some(class_name) = reflected_class_name(this) else {
            let arr_ptr = ctx.get_arena().alloc_and_track(arr);
            return Ok(Value::new_array_ptr(arr_ptr as *mut ()));
        };

        let filter_name = name.and_then(|n| n.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() });
        let is_instanceof = flags.and_then(|f| f.as_int()).map(|i| (i & 2) != 0).unwrap_or(false);

        let attributes = ctx.get_class_attributes(&class_name);
        let attr_class_id = ctx.get_class_id("ReflectionAttribute").unwrap_or(0);

        let mut idx: i64 = 0;
        for (attr_name, args) in attributes {
            if let Some(ref filter) = filter_name {
                let norm_filter = filter.trim_start_matches('\\').to_lowercase();
                let norm_attr = attr_name.trim_start_matches('\\').to_lowercase();
                let short_attr = if let Some(last) = norm_attr.rfind('\\') {
                    &norm_attr[last + 1..]
                } else {
                    &norm_attr
                };
                let short_filter = if let Some(last) = norm_filter.rfind('\\') {
                    &norm_filter[last + 1..]
                } else {
                    &norm_filter
                };

                let exact_match = norm_attr == norm_filter || short_attr == short_filter;
                let instanceof_match = is_instanceof && is_instance_of_class(ctx, &attr_name, filter);
                if !exact_match && !instanceof_match {
                    continue;
                }
            }

            let mut obj = hyperion_core::types::object::PhpObject::new_with_name(attr_class_id, "ReflectionAttribute".to_string());
            let name_str = crate::into_raw(Box::new(attr_name.clone()));
            obj.properties.insert("name".to_string(), Value::new_string_ptr(name_str as *mut ()));

            let mut args_arr = PhpArray::new();
            let mut auto_idx: i64 = 0;
            for (arg_name, val) in &args {
                if let Some(k) = arg_name {
                    let k_id = ctx.intern_string(k);
                    args_arr.insert_string_id(k_id, *val);
                } else {
                    args_arr.insert_int(auto_idx, *val);
                    auto_idx += 1;
                }
            }
            let args_ptr = ctx.get_arena().alloc_and_track(args_arr);
            obj.properties.insert("arguments".to_string(), Value::new_array_ptr(args_ptr as *mut ()));

            let obj_ptr = ctx.get_arena().alloc_and_track(obj);
            arr.insert_int(idx, Value::new_object_ptr(obj_ptr as *mut ()));
            idx += 1;
        }

        let arr_ptr = ctx.get_arena().alloc_and_track(arr);
        Ok(Value::new_array_ptr(arr_ptr as *mut ()))
    }
}

php_function! {
    native_reflection_class_new_instance_args(this: Value, args: Value) |ctx| {
        let (Some(this), Some(args)) = (this, args) else {
            return Err("ReflectionException: ReflectionClass::newInstanceArgs(): Invalid arguments".to_string());
        };
        let Some(class_name) = reflected_class_name(Some(this)) else {
            return Err("ReflectionException: ReflectionClass::newInstanceArgs(): Target class name not found".to_string());
        };
        if !ctx.class_exists(&class_name) {
            ctx.trigger_autoload_sync(&class_name);
        }
        let class_id = ctx.get_class_id(&class_name).unwrap_or(0);
        let obj = ctx.instantiate_class(class_id);
        let obj_ptr = ctx.get_arena().alloc_and_track(obj);
        let instance = Value::new_object_ptr(obj_ptr as *mut ());

        if ctx.has_method(&class_name, "__construct") {
            let mut arr = hyperion_core::types::array::PhpArray::new();
            arr.insert_int(0, instance);
            let m_ptr = ctx.get_arena().alloc_and_track("__construct".to_string());
            arr.insert_int(1, Value::new_string_ptr(m_ptr as *mut ()));
            let arr_ptr = ctx.get_arena().alloc_and_track(arr);
            let callable_arr = Value::new_array_ptr(arr_ptr as *mut ());

            let mut call_args = Vec::new();
            if let Some(arr_ptr) = args.as_array_ptr() {
                let a = unsafe { &*(arr_ptr as *const hyperion_core::types::array::PhpArray) };
                for (_, v) in &a.elements {
                    call_args.push(*v);
                }
            }
            let _ = ctx.call_callable_synchronously(callable_arr, call_args);
        }
        Ok(instance)
    }
}

php_function! {
    native_reflection_class_new_instance(this: Value, ...rest) |ctx| {
        let Some(this) = this else {
            return Err("ReflectionException: ReflectionClass::newInstance(): Invalid target".to_string());
        };
        let Some(class_name) = reflected_class_name(Some(this)) else {
            return Err("ReflectionException: ReflectionClass::newInstance(): Target class name not found".to_string());
        };
        if !ctx.class_exists(&class_name) {
            ctx.trigger_autoload_sync(&class_name);
        }
        let class_id = ctx.get_class_id(&class_name).unwrap_or(0);
        let obj = ctx.instantiate_class(class_id);
        let obj_ptr = ctx.get_arena().alloc_and_track(obj);
        let instance = Value::new_object_ptr(obj_ptr as *mut ());

        if ctx.has_method(&class_name, "__construct") {
            let mut arr = hyperion_core::types::array::PhpArray::new();
            arr.insert_int(0, instance);
            let m_ptr = ctx.get_arena().alloc_and_track("__construct".to_string());
            arr.insert_int(1, Value::new_string_ptr(m_ptr as *mut ()));
            let arr_ptr = ctx.get_arena().alloc_and_track(arr);
            let callable_arr = Value::new_array_ptr(arr_ptr as *mut ());

            let call_args = rest.to_vec();
            let _ = ctx.call_callable_synchronously(callable_arr, call_args);
        }
        Ok(instance)
    }
}

php_function! {
    native_reflection_class_new_instance_without_constructor(this: Value) |ctx| {
        let Some(this) = this else {
            return Err("ReflectionException: ReflectionClass::newInstanceWithoutConstructor(): Invalid target".to_string());
        };
        let Some(class_name) = reflected_class_name(Some(this)) else {
            return Err("ReflectionException: ReflectionClass::newInstanceWithoutConstructor(): Target class name not found".to_string());
        };
        if !ctx.class_exists(&class_name) {
            ctx.trigger_autoload_sync(&class_name);
        }
        let class_id = ctx.get_class_id(&class_name).unwrap_or(0);
        let obj = ctx.instantiate_class(class_id);
        let obj_ptr = ctx.get_arena().alloc_and_track(obj);
        let instance = Value::new_object_ptr(obj_ptr as *mut ());
        Ok(instance)
    }
}

php_function! {
    native_reflection_attribute_get_name(this: Value) {
        Ok(php_string(obj_string_prop(this, "name").unwrap_or_default()))
    }
}

php_function! {
    native_reflection_attribute_get_arguments(this: Value) {
        let Some(this) = this else { return Ok(Value::null()) };
        if let Some(obj_ptr) = this.as_object_ptr() {
            let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
            if let Some(args) = obj.properties.get("arguments") {
                return Ok(*args);
            }
        }
        Ok(Value::null())
    }
}

php_function! {
    native_reflection_attribute_new_instance(this: Value) |ctx| {
        let Some(this) = this else { return Ok(Value::null()) };
        let attr_name = obj_string_prop(Some(this), "name").unwrap_or_default();
        if attr_name.is_empty() {
            return Ok(Value::null());
        }

        if !ctx.class_exists(&attr_name) {
            ctx.trigger_autoload_sync(&attr_name);
        }

        let class_id = ctx.get_class_id(&attr_name).unwrap_or(0);
        let mut obj = ctx.instantiate_class(class_id);

        // Populate arguments into properties or construct call
        let mut positional_args: Vec<(i64, Value)> = Vec::new();
        let mut named_args: std::collections::HashMap<String, Value> = std::collections::HashMap::new();

        if let Some(obj_ptr) = this.as_object_ptr() {
            let r_obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
            if let Some(args_val) = r_obj.properties.get("arguments") {
                if let Some(arr_ptr) = args_val.as_array_ptr() {
                    let arr = unsafe { &*(arr_ptr as *const PhpArray) };
                    for (k, v) in &arr.elements {
                        match k {
                            hyperion_core::types::array::ArrayKey::StringId(sid) => {
                                if let Some(key_str) = ctx.lookup_string(*sid) {
                                    let clean_key = key_str.trim_start_matches('$').to_string();
                                    named_args.insert(clean_key.clone(), *v);
                                    obj.properties.insert(clean_key, *v);
                                }
                            }
                            hyperion_core::types::array::ArrayKey::Int(i) => {
                                positional_args.push((*i, *v));
                            }
                        }
                    }
                }
            }
        }

        let gen_context = ctx.get_static_property("OpenApi\\Generator", "context")
            .or_else(|| ctx.get_static_property("OpenApi\\Generator", "$context"));

        fn fixup_nested_objects(val: Value, gen_context: Option<Value>, seen: &mut std::collections::HashSet<usize>) {
            if let Some(arr_ptr) = val.as_array_ptr() {
                let addr = arr_ptr as usize;
                if !seen.insert(addr) {
                    return;
                }
                let arr = unsafe { &mut *(arr_ptr as *mut PhpArray) };
                for (_, v) in arr.elements.iter_mut() {
                    fixup_nested_objects(*v, gen_context, seen);
                }
            } else if let Some(obj_ptr) = val.as_object_ptr() {
                let addr = obj_ptr as usize;
                if !seen.insert(addr) {
                    return;
                }
                let obj = unsafe { &mut *(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
                let is_oa = if let Some(ref cn) = obj.class_name {
                    cn.contains("OpenApi\\")
                } else {
                    false
                };
                if is_oa {
                    if !obj.properties.contains_key("_context") || obj.properties.get("_context").map_or(true, |v| v.is_null()) {
                        if let Some(gc) = gen_context {
                            obj.properties.insert("_context".to_string(), gc);
                        }
                    }
                }
                for (prop_name, prop_val) in obj.properties.iter_mut() {
                    if prop_name != "_context" && prop_name != "_unmerged" {
                        fixup_nested_objects(*prop_val, gen_context, seen);
                    }
                }
            }
        }

        let mut seen = std::collections::HashSet::new();
        for (_, v) in &mut named_args {
            fixup_nested_objects(*v, gen_context, &mut seen);
        }
        for (_, v) in &mut positional_args {
            fixup_nested_objects(*v, gen_context, &mut seen);
        }

        if attr_name.contains("OpenApi\\") {
            if !obj.properties.contains_key("_context") || obj.properties.get("_context").map_or(true, |v| v.is_null()) {
                if let Some(gc) = gen_context {
                    obj.properties.insert("_context".to_string(), gc);
                }
            }
        }

        let obj_ptr = ctx.get_arena().alloc_and_track(obj);
        let instance = Value::new_object_ptr(obj_ptr as *mut ());

        // If class has constructor, call it
        if ctx.has_method(&attr_name, "__construct") {
            let mut arr = hyperion_core::types::array::PhpArray::new();
            arr.insert_int(0, instance);
            let m_ptr = ctx.get_arena().alloc_and_track("__construct".to_string());
            arr.insert_int(1, Value::new_string_ptr(m_ptr as *mut ()));
            let arr_ptr = ctx.get_arena().alloc_and_track(arr);
            let callable_arr = Value::new_array_ptr(arr_ptr as *mut ());

            let mut call_args = Vec::new();
            if let Some(params) = ctx.get_method_params(&attr_name, "__construct") {
                positional_args.sort_by_key(|(i, _)| *i);
                for (i, (param_name, _, _, _, _, def_val)) in params.iter().enumerate() {
                    let clean_param = param_name.trim_start_matches('$');
                    if let Some(val) = named_args.get(clean_param) {
                        call_args.push(*val);
                    } else if let Some((_, val)) = positional_args.iter().find(|(idx, _)| *idx as usize == i) {
                        call_args.push(*val);
                    } else if let Some(d) = def_val {
                        call_args.push(*d);
                    } else {
                        call_args.push(Value::null());
                    }
                }
            } else {
                positional_args.sort_by_key(|(i, _)| *i);
                for (_, v) in positional_args {
                    call_args.push(v);
                }
            }

            let _ = ctx.call_callable_synchronously(
                callable_arr,
                call_args,
            );
        }

        Ok(instance)
    }
}

php_function! {
    native_reflection_class_is_enum(this: Value) |ctx| {
        let Some(class_name) = reflected_class_name(this) else {
            return Ok(Value::new_bool(false));
        };
        Ok(Value::new_bool(ctx.is_enum(&class_name)))
    }
}

php_function! {
    native_reflection_class_is_subclass_of(this: Value, class: Value) |ctx| {
        let Some(class_name) = reflected_class_name(this) else {
            return Ok(Value::new_bool(false));
        };
        let parent_target = match class {
            Some(v) if v.is_string() => v.as_string_ptr().map(|p| unsafe { (*(p as *const String)).clone() }),
            Some(v) if v.is_object() => reflected_class_name(Some(v)),
            _ => None,
        };
        let Some(parent_target) = parent_target else {
            return Ok(Value::new_bool(false));
        };
        let target_clean = parent_target.trim_start_matches('\\');
        let class_clean = class_name.trim_start_matches('\\');

        let parents = ctx.get_parent_classes(class_clean);
        for p in &parents {
            let p_clean = p.trim_start_matches('\\');
            if p_clean.eq_ignore_ascii_case(target_clean) {
                return Ok(Value::new_bool(true));
            }
            if ctx.implements_interface(p_clean, target_clean) {
                return Ok(Value::new_bool(true));
            }
        }
        Ok(Value::new_bool(ctx.implements_interface(class_clean, target_clean)))
    }
}

php_function! {
    native_reflection_class_is_interface(this: Value) |ctx| {
        let Some(class_name) = reflected_class_name(this) else {
            return Ok(Value::new_bool(false));
        };
        Ok(Value::new_bool(ctx.is_interface(&class_name)))
    }
}

php_function! {
    native_reflection_enum_is_backed(this: Value) |ctx| {
        let Some(class_name) = reflected_class_name(this) else {
            return Ok(Value::new_bool(false));
        };
        let is_backed = ctx.implements_interface(&class_name, "BackedEnum")
            || ctx.get_parent_class(&class_name).map(|p| p == "BackedEnum").unwrap_or(false);
        Ok(Value::new_bool(is_backed))
    }
}

php_function! {
    native_reflection_enum_get_backing_type(this: Value) |ctx| {
        let Some(class_name) = reflected_class_name(this) else {
            return Ok(Value::null());
        };
        let is_backed = ctx.implements_interface(&class_name, "BackedEnum")
            || ctx.get_parent_class(&class_name).map(|p| p == "BackedEnum").unwrap_or(false);
        if !is_backed {
            return Ok(Value::null());
        }

        let Some(type_class_id) = ctx.get_class_id("ReflectionNamedType") else {
            return Ok(Value::null());
        };
        let mut obj = hyperion_core::types::object::PhpObject::new(type_class_id);
        obj.properties.insert("name".to_string(), php_string("string".to_string()));
        obj.properties.insert("allows_null".to_string(), Value::new_bool(false));
        let obj_ptr = ctx.get_arena().alloc_and_track(obj);
        Ok(Value::new_object_ptr(obj_ptr as *mut ()))
    }
}

php_function! {
    native_reflection_enum_has_case(this: Value, name: Value) {
        let _ = (this, name);
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_enum_get_cases(this: Value) |ctx| {
        let _ = this;
        let arr = PhpArray::new();
        let arr_ptr = ctx.get_arena().alloc_and_track(arr);
        Ok(Value::new_array_ptr(arr_ptr as *mut ()))
    }
}

php_function! {
    native_reflection_enum_get_case(this: Value, _name: Value) {
        let _ = this;
        Ok(Value::null())
    }
}

php_function! {
    native_reflection_class_is_abstract(this: Value) |ctx| {
        let Some(class_name) = reflected_class_name(this) else {
            return Ok(Value::new_bool(false));
        };
        Ok(Value::new_bool(ctx.is_class_abstract(&class_name) || ctx.is_interface(&class_name)))
    }
}

php_function! {
    native_reflection_class_is_final(this: Value) |ctx| {
        let Some(class_name) = reflected_class_name(this) else {
            return Ok(Value::new_bool(false));
        };
        Ok(Value::new_bool(ctx.is_class_final(&class_name) || ctx.is_enum(&class_name)))
    }
}

php_function! {
    native_reflection_class_is_internal(this: Value) {
        let _ = this;
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_class_is_user_defined(this: Value) {
        let _ = this;
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_reflection_class_is_anonymous(this: Value) {
        let _ = this;
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_class_is_trait(this: Value) |ctx| {
        let Some(class_name) = reflected_class_name(this) else {
            return Ok(Value::new_bool(false));
        };
        Ok(Value::new_bool(ctx.is_trait(&class_name)))
    }
}

php_function! {
    native_reflection_class_is_iterable(this: Value) {
        let _ = this;
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_class_is_readonly(this: Value) |ctx| {
        let Some(class_name) = reflected_class_name(this) else {
            return Ok(Value::new_bool(false));
        };
        Ok(Value::new_bool(ctx.is_class_readonly(&class_name)))
    }
}

php_function! {
    native_reflection_class_is_cloneable(this: Value) {
        let _ = this;
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_reflection_class_has_constant(this: Value, name: Value) |ctx| {
        let Some(class_name) = reflected_class_name(this) else {
            return Ok(Value::new_bool(false));
        };
        let Some(const_name) = name.and_then(|n| n.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() }) else {
            return Ok(Value::new_bool(false));
        };
        let scoped_name = format!("{}::{}", class_name, const_name);
        if ctx.lookup_constant(&scoped_name).is_some() || ctx.get_static_property(&class_name, &const_name).is_some() {
            return Ok(Value::new_bool(true));
        }
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_class_get_constant(this: Value, name: Value) |ctx| {
        let Some(class_name) = reflected_class_name(this) else {
            return Ok(Value::new_bool(false));
        };
        let Some(const_name) = name.and_then(|n| n.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() }) else {
            return Ok(Value::new_bool(false));
        };
        let scoped_name = format!("{}::{}", class_name, const_name);
        if let Some(val) = ctx.lookup_constant(&scoped_name) {
            return Ok(val);
        }
        if let Some(val) = ctx.get_static_property(&class_name, &const_name) {
            return Ok(val);
        }
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_class_get_constants(this: Value, filter: Value) |ctx| {
        let _ = filter;
        let mut arr = PhpArray::new();
        let Some(class_name) = reflected_class_name(this) else {
            let arr_ptr = ctx.get_arena().alloc_and_track(arr);
            return Ok(Value::new_array_ptr(arr_ptr as *mut ()));
        };

        let props = ctx.get_class_properties(&class_name);
        for p in props {
            if p.is_static {
                let val = ctx.get_static_property(&class_name, &p.name).unwrap_or(p.default_value);
                let key_sid = ctx.intern_string(&p.name);
                arr.insert_string_id(key_sid, val);
            }
        }

        let arr_ptr = ctx.get_arena().alloc_and_track(arr);
        Ok(Value::new_array_ptr(arr_ptr as *mut ()))
    }
}

php_function! {
    native_reflection_class_get_reflection_constant(this: Value, name: Value) |ctx| {
        let Some(class_name) = reflected_class_name(this) else {
            return Ok(Value::new_bool(false));
        };
        let Some(const_name) = name.and_then(|n| n.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() }) else {
            return Ok(Value::new_bool(false));
        };
        let scoped_name = format!("{}::{}", class_name, const_name);
        let val = if let Some(val) = ctx.lookup_constant(&scoped_name) {
            Some(val)
        } else if let Some(val) = ctx.get_static_property(&class_name, &const_name) {
            Some(val)
        } else {
            None
        };
        let Some(val) = val else {
            return Ok(Value::new_bool(false));
        };
        let Some(const_class_id) = ctx.get_class_id("ReflectionClassConstant") else {
            return Ok(Value::new_bool(false));
        };
        let mut obj = hyperion_core::types::object::PhpObject::new(const_class_id);
        obj.properties.insert("class".to_string(), php_string(class_name));
        obj.properties.insert("name".to_string(), php_string(const_name));
        obj.properties.insert("value".to_string(), val);
        obj.properties.insert("is_public".to_string(), Value::new_bool(true));
        obj.properties.insert("is_protected".to_string(), Value::new_bool(false));
        obj.properties.insert("is_private".to_string(), Value::new_bool(false));
        let obj_ptr = ctx.get_arena().alloc_and_track(obj);
        Ok(Value::new_object_ptr(obj_ptr as *mut ()))
    }
}

php_function! {
    native_reflection_class_get_reflection_constants(this: Value, filter: Value) |ctx| {
        let _ = filter;
        let mut arr = PhpArray::new();
        let Some(class_name) = reflected_class_name(this) else {
            let arr_ptr = ctx.get_arena().alloc_and_track(arr);
            return Ok(Value::new_array_ptr(arr_ptr as *mut ()));
        };
        let const_class_id = ctx.get_class_id("ReflectionClassConstant").unwrap_or(0);
        if const_class_id == 0 {
            let arr_ptr = ctx.get_arena().alloc_and_track(arr);
            return Ok(Value::new_array_ptr(arr_ptr as *mut ()));
        }

        let props = ctx.get_class_properties(&class_name);
        let mut idx: i64 = 0;
        for p in props {
            if p.is_static {
                let val = ctx.get_static_property(&class_name, &p.name).unwrap_or(p.default_value);
                let mut obj = hyperion_core::types::object::PhpObject::new(const_class_id);
                obj.properties.insert("class".to_string(), php_string(p.declaring_class));
                obj.properties.insert("name".to_string(), php_string(p.name));
                obj.properties.insert("value".to_string(), val);
                obj.properties.insert("is_public".to_string(), Value::new_bool(p.is_public));
                obj.properties.insert("is_protected".to_string(), Value::new_bool(p.is_protected));
                obj.properties.insert("is_private".to_string(), Value::new_bool(p.is_private));
                let obj_ptr = ctx.get_arena().alloc_and_track(obj);
                arr.insert_int(idx, Value::new_object_ptr(obj_ptr as *mut ()));
                idx += 1;
            }
        }

        let arr_ptr = ctx.get_arena().alloc_and_track(arr);
        Ok(Value::new_array_ptr(arr_ptr as *mut ()))
    }
}

php_function! {
    native_reflection_class_get_file_name(this: Value) |ctx| {
        if let Some(name) = reflected_class_name(this) {
            if let Some(filename) = ctx.get_class_file_name(&name) {
                let boxed = crate::into_raw(Box::new(filename));
                return Ok(Value::new_string_ptr(boxed as *mut ()));
            }
        }
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_class_get_start_line(this: Value) {
        let _ = this;
        Ok(Value::new_int(1))
    }
}

php_function! {
    native_reflection_class_get_end_line(this: Value) {
        let _ = this;
        Ok(Value::new_int(100))
    }
}

php_function! {
    native_reflection_class_get_doc_comment(this: Value) {
        let _ = this;
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_class_get_short_name(this: Value) {
        if let Some(name) = reflected_class_name(this) {
            let short = name.split('\\').next_back().unwrap_or(&name).to_string();
            let ptr = crate::into_raw(Box::new(short));
            Ok(Value::new_string_ptr(ptr as *mut ()))
        } else {
            Ok(Value::null())
        }
    }
}

php_function! {
    native_reflection_class_get_namespace_name(this: Value) {
        if let Some(name) = reflected_class_name(this) {
            let ns = if let Some(idx) = name.rfind('\\') {
                name[..idx].to_string()
            } else {
                String::new()
            };
            let ptr = crate::into_raw(Box::new(ns));
            Ok(Value::new_string_ptr(ptr as *mut ()))
        } else {
            let ptr = crate::into_raw(Box::new(String::new()));
            Ok(Value::new_string_ptr(ptr as *mut ()))
        }
    }
}

php_function! {
    native_reflection_class_in_namespace(this: Value) {
        if let Some(name) = reflected_class_name(this) {
            Ok(Value::new_bool(name.contains('\\')))
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_reflection_class_get_properties(this: Value, filter: Value) |ctx| {
        let mut arr = PhpArray::new();
        let Some(class_name) = reflected_class_name(this) else {
            let arr_ptr = ctx.get_arena().alloc_and_track(arr);
            return Ok(Value::new_array_ptr(arr_ptr as *mut ()));
        };

        let filter_mask = filter.and_then(|f| f.as_int()).map(|i| i as i64);
        let prop_class_id = ctx.get_class_id("ReflectionProperty").unwrap_or(0);
        if prop_class_id == 0 {
            let arr_ptr = ctx.get_arena().alloc_and_track(arr);
            return Ok(Value::new_array_ptr(arr_ptr as *mut ()));
        }

        let props = ctx.get_class_properties(&class_name);
        let mut idx = 0;
        for p in props {
            if let Some(mask) = filter_mask {
                let mut matches_filter = false;
                if (mask & 1 != 0) && p.is_public { matches_filter = true; }
                if (mask & 2 != 0) && p.is_protected { matches_filter = true; }
                if (mask & 4 != 0) && p.is_private { matches_filter = true; }
                if (mask & 16 != 0) && p.is_static { matches_filter = true; }
                if (mask & 128 != 0) && p.is_readonly { matches_filter = true; }
                if !matches_filter {
                    continue;
                }
            }

            let mut obj = hyperion_core::types::object::PhpObject::new(prop_class_id);
            let class_boxed = crate::into_raw(Box::new(p.declaring_class));
            obj.properties.insert("class".to_string(), Value::new_string_ptr(class_boxed as *mut ()));
            let name_boxed = crate::into_raw(Box::new(p.name));
            obj.properties.insert("name".to_string(), Value::new_string_ptr(name_boxed as *mut ()));
            obj.properties.insert("is_static".to_string(), Value::new_bool(p.is_static));
            obj.properties.insert("is_public".to_string(), Value::new_bool(p.is_public));
            obj.properties.insert("is_protected".to_string(), Value::new_bool(p.is_protected));
            obj.properties.insert("is_private".to_string(), Value::new_bool(p.is_private));
            obj.properties.insert("is_readonly".to_string(), Value::new_bool(p.is_readonly));
            obj.properties.insert("default_value".to_string(), p.default_value);

            let obj_ptr = ctx.get_arena().alloc_and_track(obj);
            arr.insert_int(idx, Value::new_object_ptr(obj_ptr as *mut ()));
            idx += 1;
        }

        let arr_ptr = ctx.get_arena().alloc_and_track(arr);
        Ok(Value::new_array_ptr(arr_ptr as *mut ()))
    }
}

php_function! {
    native_reflection_class_get_default_properties(this: Value) |ctx| {
        let mut arr = PhpArray::new();
        let Some(class_name) = reflected_class_name(this) else {
            let arr_ptr = ctx.get_arena().alloc_and_track(arr);
            return Ok(Value::new_array_ptr(arr_ptr as *mut ()));
        };
        let props = ctx.get_class_properties(&class_name);
        for p in props {
            let key_sid = ctx.intern_string(&p.name);
            arr.insert_string_id(key_sid, p.default_value);
        }
        let arr_ptr = ctx.get_arena().alloc_and_track(arr);
        Ok(Value::new_array_ptr(arr_ptr as *mut ()))
    }
}

php_function! {
    native_reflection_class_get_static_properties(this: Value) |ctx| {
        let mut arr = PhpArray::new();
        let Some(class_name) = reflected_class_name(this) else {
            let arr_ptr = ctx.get_arena().alloc_and_track(arr);
            return Ok(Value::new_array_ptr(arr_ptr as *mut ()));
        };
        let props = ctx.get_class_properties(&class_name);
        for p in props {
            if p.is_static {
                let val = ctx.get_static_property(&class_name, &p.name).unwrap_or(p.default_value);
                let key_sid = ctx.intern_string(&p.name);
                arr.insert_string_id(key_sid, val);
            }
        }
        let arr_ptr = ctx.get_arena().alloc_and_track(arr);
        Ok(Value::new_array_ptr(arr_ptr as *mut ()))
    }
}

php_function! {
    native_reflection_class_get_interfaces(this: Value) |ctx| {
        let _ = (this, ctx);
        let ptr = crate::into_raw(Box::new(PhpArray::new()));
        Ok(Value::new_array_ptr(ptr as *mut ()))
    }
}

php_function! {
    native_reflection_class_get_interface_names(this: Value) |ctx| {
        let _ = (this, ctx);
        let ptr = crate::into_raw(Box::new(PhpArray::new()));
        Ok(Value::new_array_ptr(ptr as *mut ()))
    }
}

php_function! {
    native_reflection_class_get_traits(this: Value) |ctx| {
        let mut arr = PhpArray::new();
        let Some(class_name) = reflected_class_name(this) else {
            let arr_ptr = ctx.get_arena().alloc_and_track(arr);
            return Ok(Value::new_array_ptr(arr_ptr as *mut ()));
        };

        let rc_id = ctx.get_class_id("ReflectionClass").unwrap_or(0);
        let traits = ctx.get_class_traits(&class_name);
        for trait_name in traits {
            if rc_id > 0 {
                let mut obj = hyperion_core::types::object::PhpObject::new(rc_id);
                let trait_boxed = crate::into_raw(Box::new(trait_name.clone()));
                obj.properties.insert(
                    "name".to_string(),
                    Value::new_string_ptr(trait_boxed as *mut ()),
                );
                let obj_ptr = ctx.get_arena().alloc_and_track(obj);
                let trait_sid = ctx.intern_string(&trait_name);
                arr.insert_string_id(trait_sid, Value::new_object_ptr(obj_ptr as *mut ()));
            }
        }

        let arr_ptr = ctx.get_arena().alloc_and_track(arr);
        Ok(Value::new_array_ptr(arr_ptr as *mut ()))
    }
}

php_function! {
    native_reflection_class_get_trait_names(this: Value) |ctx| {
        let mut arr = PhpArray::new();
        let Some(class_name) = reflected_class_name(this) else {
            let arr_ptr = ctx.get_arena().alloc_and_track(arr);
            return Ok(Value::new_array_ptr(arr_ptr as *mut ()));
        };

        let traits = ctx.get_class_traits(&class_name);
        for (idx, trait_name) in traits.into_iter().enumerate() {
            let trait_boxed = crate::into_raw(Box::new(trait_name));
            arr.insert_int(idx as i64, Value::new_string_ptr(trait_boxed as *mut ()));
        }

        let arr_ptr = ctx.get_arena().alloc_and_track(arr);
        Ok(Value::new_array_ptr(arr_ptr as *mut ()))
    }
}

php_function! {
    native_reflection_class_has_property(this: Value, name: Value) |ctx| {
        let Some(class_name) = reflected_class_name(this) else {
            return Ok(Value::new_bool(false));
        };
        let Some(prop_name) = name.and_then(|n| n.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() }) else {
            return Ok(Value::new_bool(false));
        };
        let has_prop = ctx.has_class_property(&class_name, &prop_name)
            || ctx.get_static_property(&class_name, &prop_name).is_some()
            || ctx.is_property_public(&class_name, &prop_name)
            || ctx.get_class_property_info(&class_name, &prop_name).is_some();
        Ok(Value::new_bool(has_prop))
    }
}

php_function! {
    native_reflection_class_get_property(this: Value, name: Value) |ctx| {
        let Some(class_name) = reflected_class_name(this) else {
            return Err("ReflectionException: ReflectionClass::getProperty(): Internal error".to_string());
        };
        let Some(prop_name) = name.and_then(|n| n.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() }) else {
            return Err("ReflectionException: ReflectionClass::getProperty() expects parameter 1 to be string".to_string());
        };
        let Some(prop_class_id) = ctx.get_class_id("ReflectionProperty") else {
            return Ok(Value::null());
        };

        let prop_info = ctx.get_class_property_info(&class_name, &prop_name);
        let declaring_class = prop_info.as_ref().map(|p| p.declaring_class.clone()).unwrap_or_else(|| class_name.clone());

        let mut obj = hyperion_core::types::object::PhpObject::new(prop_class_id);
        let class_boxed = crate::into_raw(Box::new(declaring_class));
        obj.properties.insert(
            "class".to_string(),
            Value::new_string_ptr(class_boxed as *mut ()),
        );
        let prop_boxed = crate::into_raw(Box::new(prop_name));
        obj.properties.insert(
            "name".to_string(),
            Value::new_string_ptr(prop_boxed as *mut ()),
        );
        if let Some(ref p) = prop_info {
            obj.properties.insert("is_static".to_string(), Value::new_bool(p.is_static));
            obj.properties.insert("is_public".to_string(), Value::new_bool(p.is_public));
            obj.properties.insert("is_protected".to_string(), Value::new_bool(p.is_protected));
            obj.properties.insert("is_private".to_string(), Value::new_bool(p.is_private));
            obj.properties.insert("is_readonly".to_string(), Value::new_bool(p.is_readonly));
            obj.properties.insert("default_value".to_string(), p.default_value);
        }

        let obj_ptr = ctx.get_arena().alloc_and_track(obj);
        Ok(Value::new_object_ptr(obj_ptr as *mut ()))
    }
}

php_function! {
    native_reflection_property_construct(this: Value, class: Value, name: Value) |ctx| {
        if let Some(this) = this {
            if this.is_object() {
                let obj_ptr = this.as_object_ptr().unwrap() as *mut hyperion_core::types::object::PhpObject;
                unsafe {
                    let mut class_name = String::new();
                    if let Some(class) = class {
                        class_name = if let Some(class_str) = class.as_string_ptr() {
                            (*(class_str as *const String)).clone()
                        } else if class.is_object() {
                            let arg_ptr = class.as_object_ptr().unwrap() as *const hyperion_core::types::object::PhpObject;
                            ctx.get_class_name((*arg_ptr).class_id).unwrap_or_default()
                        } else {
                            String::new()
                        };
                        if !ctx.class_exists(&class_name) {
                            ctx.trigger_autoload_sync(&class_name);
                        }
                    }
                    let prop_name = if let Some(name) = name {
                        if let Some(s_ptr) = name.as_string_ptr() {
                            (*(s_ptr as *const String)).clone()
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    };

                    let prop_info = ctx.get_class_property_info(&class_name, &prop_name);
                    let declaring_class = prop_info.as_ref().map(|p| p.declaring_class.clone()).unwrap_or(class_name);

                    (*obj_ptr).properties.insert("class".to_string(), php_string(declaring_class));
                    (*obj_ptr).properties.insert("name".to_string(), php_string(prop_name));

                    if let Some(ref p) = prop_info {
                        (*obj_ptr).properties.insert("is_static".to_string(), Value::new_bool(p.is_static));
                        (*obj_ptr).properties.insert("is_public".to_string(), Value::new_bool(p.is_public));
                        (*obj_ptr).properties.insert("is_protected".to_string(), Value::new_bool(p.is_protected));
                        (*obj_ptr).properties.insert("is_private".to_string(), Value::new_bool(p.is_private));
                        (*obj_ptr).properties.insert("is_readonly".to_string(), Value::new_bool(p.is_readonly));
                        (*obj_ptr).properties.insert("default_value".to_string(), p.default_value);
                    }
                }
            }
        }
        Ok(Value::null())
    }
}

php_function! {
    native_reflection_property_get_declaring_class(this: Value) |ctx| {
        let Some(this) = this else { return Ok(Value::null()) };
        let Some(obj_ptr) = this.as_object_ptr() else { return Ok(Value::null()) };
        let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
        let class_name = obj.properties.get("class").and_then(|v| v.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() });
        let prop_name = obj.properties.get("name").and_then(|v| v.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() });
        let (Some(class_name), Some(prop_name)) = (class_name, prop_name) else { return Ok(Value::null()) };

        let declaring_class = ctx.get_property_declaring_class(&class_name, &prop_name).unwrap_or(class_name);
        let Some(rc_id) = ctx.get_class_id("ReflectionClass") else { return Ok(Value::null()) };

        let mut rc_obj = hyperion_core::types::object::PhpObject::new(rc_id);
        rc_obj.properties.insert("name".to_string(), php_string(declaring_class));
        let rc_ptr = ctx.get_arena().alloc_and_track(rc_obj);
        Ok(Value::new_object_ptr(rc_ptr as *mut ()))
    }
}

php_function! {
    native_reflection_property_get_name(this: Value) {
        if let Some(this) = this {
            if let Some(obj_ptr) = this.as_object_ptr() {
                let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
                if let Some(name_val) = obj.properties.get("name") {
                    return Ok(*name_val);
                }
            }
        }
        Ok(Value::null())
    }
}

php_function! {
    native_reflection_property_is_public(this: Value) |ctx| {
        let Some(this) = this else { return Ok(Value::new_bool(true)) };
        let Some(obj_ptr) = this.as_object_ptr() else { return Ok(Value::new_bool(true)) };
        let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
        if let Some(v) = obj.properties.get("is_public") {
            return Ok(*v);
        }
        let class_name = obj.properties.get("class").and_then(|v| v.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() });
        let prop_name = obj.properties.get("name").and_then(|v| v.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() });
        if let (Some(class_name), Some(prop_name)) = (class_name, prop_name) {
            return Ok(Value::new_bool(ctx.is_property_public(&class_name, &prop_name)));
        }
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_reflection_property_is_static(this: Value) |ctx| {
        let Some(this) = this else { return Ok(Value::new_bool(false)) };
        let Some(obj_ptr) = this.as_object_ptr() else { return Ok(Value::new_bool(false)) };
        let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
        if let Some(v) = obj.properties.get("is_static") {
            return Ok(*v);
        }
        let class_name = obj.properties.get("class").and_then(|v| v.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() });
        let prop_name = obj.properties.get("name").and_then(|v| v.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() });
        if let (Some(class_name), Some(prop_name)) = (class_name, prop_name) {
            return Ok(Value::new_bool(ctx.is_property_static(&class_name, &prop_name)));
        }
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_property_is_protected(this: Value) |ctx| {
        let Some(this) = this else { return Ok(Value::new_bool(false)) };
        let Some(obj_ptr) = this.as_object_ptr() else { return Ok(Value::new_bool(false)) };
        let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
        if let Some(v) = obj.properties.get("is_protected") {
            return Ok(*v);
        }
        let class_name = obj.properties.get("class").and_then(|v| v.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() });
        let prop_name = obj.properties.get("name").and_then(|v| v.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() });
        if let (Some(class_name), Some(prop_name)) = (class_name, prop_name) {
            if let Some(info) = ctx.get_class_property_info(&class_name, &prop_name) {
                return Ok(Value::new_bool(info.is_protected));
            }
        }
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_property_is_private(this: Value) |ctx| {
        let Some(this) = this else { return Ok(Value::new_bool(false)) };
        let Some(obj_ptr) = this.as_object_ptr() else { return Ok(Value::new_bool(false)) };
        let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
        if let Some(v) = obj.properties.get("is_private") {
            return Ok(*v);
        }
        let class_name = obj.properties.get("class").and_then(|v| v.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() });
        let prop_name = obj.properties.get("name").and_then(|v| v.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() });
        if let (Some(class_name), Some(prop_name)) = (class_name, prop_name) {
            if let Some(info) = ctx.get_class_property_info(&class_name, &prop_name) {
                return Ok(Value::new_bool(info.is_private));
            }
        }
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_property_get_modifiers(this: Value) |ctx| {
        let Some(this) = this else { return Ok(Value::new_int(1)) };
        let Some(obj_ptr) = this.as_object_ptr() else { return Ok(Value::new_int(1)) };
        let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };

        let class_name = obj.properties.get("class").and_then(|v| v.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() });
        let prop_name = obj.properties.get("name").and_then(|v| v.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() });

        let (is_pub, is_prot, is_priv, is_stat, is_ro) = if let (Some(cn), Some(pn)) = (class_name, prop_name) {
            if let Some(info) = ctx.get_class_property_info(&cn, &pn) {
                (info.is_public, info.is_protected, info.is_private, info.is_static, info.is_readonly)
            } else {
                let p = obj.properties.get("is_public").map(|v| v.is_truthy()).unwrap_or(true);
                let pr = obj.properties.get("is_protected").map(|v| v.is_truthy()).unwrap_or(false);
                let pv = obj.properties.get("is_private").map(|v| v.is_truthy()).unwrap_or(false);
                let s = obj.properties.get("is_static").map(|v| v.is_truthy()).unwrap_or(false);
                let ro = obj.properties.get("is_readonly").map(|v| v.is_truthy()).unwrap_or(false);
                (p, pr, pv, s, ro)
            }
        } else {
            (true, false, false, false, false)
        };

        let mut modifiers = 0i64;
        if is_pub { modifiers |= 1; }
        if is_prot { modifiers |= 2; }
        if is_priv { modifiers |= 4; }
        if is_stat { modifiers |= 16; }
        if is_ro { modifiers |= 128; }
        Ok(Value::new_int(modifiers as i32))
    }
}

php_function! {
    native_reflection_property_set_accessible(accessible: Value) {
        Ok(Value::null())
    }
}

php_function! {
    native_reflection_property_get_value(this: Value, object: Value) |ctx| {
        let Some(this) = this else { return Ok(Value::null()); };
        let Some(this_ptr) = this.as_object_ptr() else { return Ok(Value::null()); };
        let r_obj = unsafe { &*(this_ptr as *const hyperion_core::types::object::PhpObject) };
        let Some(prop_name_val) = r_obj.properties.get("name") else { return Ok(Value::null()); };
        let Some(prop_name_ptr) = prop_name_val.as_string_ptr() else { return Ok(Value::null()); };
        let prop_name = unsafe { &*(prop_name_ptr as *const String) };

        if let Some(object) = object {
            if let Some(obj_ptr) = object.as_object_ptr() {
                let target_obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
                if let Some(val) = target_obj.properties.get(prop_name) {
                    return Ok(*val);
                }
            }
        } else if let Some(class_val) = r_obj.properties.get("class") {
            if let Some(class_ptr) = class_val.as_string_ptr() {
                let class_name = unsafe { &*(class_ptr as *const String) };
                if let Some(val) = ctx.get_static_property(class_name, prop_name) {
                    return Ok(val);
                }
            }
        }
        Ok(Value::null())
    }
}

php_function! {
    native_reflection_property_set_value(this: Value, object: Value, value: Value) {
        if let Some(this) = this {
            if this.is_object() {
                let this_ptr = this.as_object_ptr().unwrap() as *mut hyperion_core::types::object::PhpObject;
                let prop_name_val = unsafe { (*this_ptr).properties.get("name") };
                if let Some(prop_name_val) = prop_name_val {
                    if let Some(s_ptr) = prop_name_val.as_string_ptr() {
                        let prop_name = unsafe { (*(s_ptr as *const String)).clone() };
                        if let Some(object) = object {
                            if object.is_object() {
                                let obj_ptr = object.as_object_ptr().unwrap() as *mut hyperion_core::types::object::PhpObject;
                                if let Some(value) = value {
                                    unsafe {
                                        (*obj_ptr).properties.insert(prop_name, *value);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(Value::null())
    }
}

php_function! {
    native_reflection_method_construct(this: Value, class: Value, name: Value) |ctx| {
        if let Some(this) = this {
            if this.is_object() {
                let obj_ptr = this.as_object_ptr().unwrap() as *mut hyperion_core::types::object::PhpObject;
                unsafe {
                    if let Some(class) = class {
                        let class_name = if let Some(class_str) = class.as_string_ptr() {
                            let name = (*(class_str as *const String)).clone();
                            if !ctx.class_exists(&name) {
                                ctx.trigger_autoload_sync(&name);
                            }
                            name
                        } else if class.is_object() {
                            let arg_ptr = class.as_object_ptr().unwrap() as *const hyperion_core::types::object::PhpObject;
                            let php_obj = &*arg_ptr;
                            if let Some(ref custom_name) = php_obj.class_name {
                                custom_name.clone()
                            } else {
                                ctx.get_class_name(php_obj.class_id).unwrap_or_default()
                            }
                        } else {
                            String::new()
                        };
                        let method_name = name.and_then(|n| n.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() }).unwrap_or_default();
                        let declaring_class = if !method_name.is_empty() {
                            ctx.get_method_declaring_class(&class_name, &method_name).unwrap_or_else(|| class_name.clone())
                        } else {
                            class_name
                        };
                        (*obj_ptr).properties.insert("class".to_string(), php_string(declaring_class));
                    }
                    if let Some(name) = name {
                        (*obj_ptr).properties.insert("name".to_string(), *name);
                    }
                }
            }
        }
        Ok(Value::null())
    }
}

php_function! {
    native_reflection_method_is_public() {
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_reflection_method_is_protected() {
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_method_is_private() {
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_method_set_accessible(accessible: Value) {
        Ok(Value::null())
    }
}

php_function! {
    native_reflection_method_invoke(this: Value, object: Value, ...rest) |ctx| {
        let Some(this) = this else { return Ok(Value::null()) };
        let Some(obj_ptr) = this.as_object_ptr() else { return Ok(Value::null()) };
        let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
        let class_name = obj.properties.get("class").and_then(|v| v.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() });
        let method_name = obj.properties.get("name").and_then(|v| v.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() });
        let (Some(class_name), Some(method_name)) = (class_name, method_name) else {
            return Ok(Value::null());
        };

        hyperion_core::hyp_debug!("DEBUG invoke: class={:?}, method={:?}, object={:?}, rest={:?}", class_name, method_name, object, rest);

        let target = if let Some(obj_val) = object {
            if obj_val.is_object() {
                *obj_val
            } else {
                php_string(class_name)
            }
        } else {
            php_string(class_name)
        };

        let mut arr = PhpArray::new();
        arr.insert_int(0, target);
        arr.insert_int(1, php_string(method_name));
        let arr_ptr = ctx.get_arena().alloc_and_track(arr);
        let callable = Value::new_array_ptr(arr_ptr as *mut ());

        ctx.call_callable_synchronously(callable, rest.to_vec())
    }
}

php_function! {
    native_reflection_method_invoke_args(this: Value, object: Value, args: Value) |ctx| {
        let arg_vec = if let Some(arr_val) = args {
            if let Some(arr_ptr) = arr_val.as_array_ptr() {
                let arr = unsafe { &*(arr_ptr as *const PhpArray) };
                arr.elements.iter().map(|(_, v)| *v).collect()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        let Some(this) = this else { return Ok(Value::null()) };
        let Some(obj_ptr) = this.as_object_ptr() else { return Ok(Value::null()) };
        let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
        let class_name = obj.properties.get("class").and_then(|v| v.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() });
        let method_name = obj.properties.get("name").and_then(|v| v.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() });
        let (Some(class_name), Some(method_name)) = (class_name, method_name) else {
            return Ok(Value::null());
        };

        let target = if let Some(obj_val) = object {
            if obj_val.is_object() {
                *obj_val
            } else {
                php_string(class_name)
            }
        } else {
            php_string(class_name)
        };

        let mut arr = PhpArray::new();
        arr.insert_int(0, target);
        arr.insert_int(1, php_string(method_name));
        let arr_ptr = ctx.get_arena().alloc_and_track(arr);
        let callable = Value::new_array_ptr(arr_ptr as *mut ());

        ctx.call_callable_synchronously(callable, arg_vec)
    }
}

php_function! {
    native_reflection_method_has_return_type(this: Value) {
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_method_get_return_type(this: Value) {
        Ok(Value::null())
    }
}

php_function! {
    native_reflection_method_has_tentative_return_type(this: Value) {
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_method_get_tentative_return_type(this: Value) {
        Ok(Value::null())
    }
}

php_function! {
    native_reflection_method_is_static(this: Value) |ctx| {
        if let Some(this) = this {
            if let Some(obj_ptr) = this.as_object_ptr() {
                let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
                let class_name = obj.properties.get("class").and_then(|v| v.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() });
                let method_name = obj.properties.get("name").and_then(|v| v.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() });
                if let (Some(class_name), Some(method_name)) = (class_name, method_name) {
                    return Ok(Value::new_bool(ctx.is_method_static(&class_name, &method_name)));
                }
            }
        }
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_method_get_declaring_class(this: Value) |ctx| {
        let Some(this) = this else { return Ok(Value::null()) };
        let Some(obj_ptr) = this.as_object_ptr() else { return Ok(Value::null()) };
        let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
        let class_name = obj.properties.get("class").and_then(|v| v.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() });
        let method_name = obj.properties.get("name").and_then(|v| v.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() });
        let (Some(class_name), Some(method_name)) = (class_name, method_name) else { return Ok(Value::null()) };

        let declaring_class = ctx.get_method_declaring_class(&class_name, &method_name).unwrap_or(class_name);
        let Some(rc_id) = ctx.get_class_id("ReflectionClass") else { return Ok(Value::null()) };

        let mut rc_obj = hyperion_core::types::object::PhpObject::new(rc_id);
        rc_obj.properties.insert("name".to_string(), php_string(declaring_class));
        let rc_ptr = ctx.get_arena().alloc_and_track(rc_obj);
        Ok(Value::new_object_ptr(rc_ptr as *mut ()))
    }
}

php_function! {
    native_reflection_method_get_attributes(this: Value, name: Value, flags: Value) |ctx| {
        let mut arr = PhpArray::new();
        let Some(this) = this else {
            let arr_ptr = ctx.get_arena().alloc_and_track(arr);
            return Ok(Value::new_array_ptr(arr_ptr as *mut ()));
        };
        let Some(obj_ptr) = this.as_object_ptr() else {
            let arr_ptr = ctx.get_arena().alloc_and_track(arr);
            return Ok(Value::new_array_ptr(arr_ptr as *mut ()));
        };
        let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
        let class_name = obj.properties.get("class").and_then(|v| v.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() });
        let method_name = obj.properties.get("name").and_then(|v| v.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() });
        let (Some(class_name), Some(method_name)) = (class_name, method_name) else {
            let arr_ptr = ctx.get_arena().alloc_and_track(arr);
            return Ok(Value::new_array_ptr(arr_ptr as *mut ()));
        };

        let filter_name = name.and_then(|n| n.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() });
        let is_instanceof = flags.and_then(|f| f.as_int()).map(|i| (i & 2) != 0).unwrap_or(false);
        let attributes = ctx.get_method_attributes(&class_name, &method_name);
        let attr_class_id = ctx.get_class_id("ReflectionAttribute").unwrap_or(0);

        let mut idx: i64 = 0;
        for (attr_name, args) in attributes {
            if let Some(ref filter) = filter_name {
                let norm_filter = filter.trim_start_matches('\\').to_lowercase();
                let norm_attr = attr_name.trim_start_matches('\\').to_lowercase();
                let short_attr = if let Some(last) = norm_attr.rfind('\\') {
                    &norm_attr[last + 1..]
                } else {
                    &norm_attr
                };
                let short_filter = if let Some(last) = norm_filter.rfind('\\') {
                    &norm_filter[last + 1..]
                } else {
                    &norm_filter
                };
                let exact_match = norm_attr == norm_filter || short_attr == short_filter;
                let instanceof_match = is_instanceof && is_instance_of_class(ctx, &attr_name, filter);
                if !exact_match && !instanceof_match {
                    continue;
                }
            }

            if attr_class_id > 0 {
                let mut obj = hyperion_core::types::object::PhpObject::new(attr_class_id);
                obj.properties.insert("name".to_string(), php_string(attr_name));

                let mut args_arr = PhpArray::new();
                for (pos, (arg_name, arg_val)) in args.into_iter().enumerate() {
                    if let Some(ref k) = arg_name {
                        let k_id = ctx.intern_string(k);
                        args_arr.insert_string_id(k_id, arg_val);
                    } else {
                        args_arr.insert_int(pos as i64, arg_val);
                    }
                }
                let args_ptr = ctx.get_arena().alloc_and_track(args_arr);
                obj.properties.insert("arguments".to_string(), Value::new_array_ptr(args_ptr as *mut ()));

                let obj_ptr = ctx.get_arena().alloc_and_track(obj);
                arr.insert_int(idx, Value::new_object_ptr(obj_ptr as *mut ()));
                idx += 1;
            }
        }

        let arr_ptr = ctx.get_arena().alloc_and_track(arr);
        Ok(Value::new_array_ptr(arr_ptr as *mut ()))
    }
}

php_function! {
    native_reflection_method_get_name(this: Value) {
        if let Some(this) = this {
            if let Some(obj_ptr) = this.as_object_ptr() {
                let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
                if let Some(name_val) = obj.properties.get("name") {
                    return Ok(*name_val);
                }
            }
        }
        Ok(Value::null())
    }
}

php_function! {
    native_reflection_method_get_number_of_parameters(this: Value) |ctx| {
        let this_slice = this.map(|t| std::slice::from_ref(t)).unwrap_or(&[]);
        let params_val = native_reflection_method_get_parameters(this_slice, ctx)?;
        if let Some(arr_ptr) = params_val.as_array_ptr() {
            let arr = unsafe { &*(arr_ptr as *const hyperion_core::types::array::PhpArray) };
            return Ok(Value::new_int(arr.elements.len() as i32));
        }
        Ok(Value::new_int(0))
    }
}

php_function! {
    native_reflection_method_get_number_of_required_parameters(this: Value) |ctx| {
        let this_slice = this.map(|t| std::slice::from_ref(t)).unwrap_or(&[]);
        native_reflection_method_get_number_of_parameters(this_slice, ctx)
    }
}

php_function! {
    native_reflection_method_get_file_name(this: Value) |ctx| {
        if let Some(class_name) = obj_string_prop(this, "class") {
            if let Some(filename) = ctx.get_class_file_name(&class_name) {
                let boxed = crate::into_raw(Box::new(filename));
                return Ok(Value::new_string_ptr(boxed as *mut ()));
            }
        }
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_method_get_start_line(this: Value) {
        let _ = this;
        Ok(Value::new_int(1))
    }
}

php_function! {
    native_reflection_method_get_end_line(this: Value) {
        let _ = this;
        Ok(Value::new_int(100))
    }
}

php_function! {
    native_reflection_method_get_doc_comment(this: Value) {
        let _ = this;
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_method_get_short_name(this: Value) |ctx| {
        let this_slice = this.map(|t| std::slice::from_ref(t)).unwrap_or(&[]);
        native_reflection_method_get_name(this_slice, ctx)
    }
}

php_function! {
    native_reflection_method_get_namespace_name(this: Value) {
        let _ = this;
        let ptr = crate::into_raw(Box::new(String::new()));
        Ok(Value::new_string_ptr(ptr as *mut ()))
    }
}

php_function! {
    native_reflection_method_in_namespace(this: Value) {
        let _ = this;
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_method_is_abstract(this: Value) {
        let _ = this;
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_method_is_final(this: Value) {
        let _ = this;
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_method_is_constructor(this: Value) |ctx| {
        let this_slice = this.map(|t| std::slice::from_ref(t)).unwrap_or(&[]);
        if let Ok(name_val) = native_reflection_method_get_name(this_slice, ctx) {
            if let Some(sp) = name_val.as_string_ptr() {
                let name = unsafe { &*(sp as *const String) };
                return Ok(Value::new_bool(name == "__construct"));
            }
        }
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_method_is_destructor(this: Value) |ctx| {
        let this_slice = this.map(|t| std::slice::from_ref(t)).unwrap_or(&[]);
        if let Ok(name_val) = native_reflection_method_get_name(this_slice, ctx) {
            if let Some(sp) = name_val.as_string_ptr() {
                let name = unsafe { &*(sp as *const String) };
                return Ok(Value::new_bool(name == "__destruct"));
            }
        }
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_method_is_closure(this: Value) {
        let _ = this;
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_method_is_deprecated(this: Value) {
        let _ = this;
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_method_is_internal(this: Value) {
        let _ = this;
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_method_is_user_defined(this: Value) {
        let _ = this;
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_reflection_method_is_generator(this: Value) {
        let _ = this;
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_method_is_variadic(this: Value) |ctx| {
        if let Some(this) = this {
            if this.is_object() {
                let obj_ptr = this.as_object_ptr().unwrap() as *mut hyperion_core::types::object::PhpObject;
                let class_name_val = unsafe { (*obj_ptr).properties.get("class") };
                let method_name_val = unsafe { (*obj_ptr).properties.get("name") };
                if let (Some(class_name_val), Some(method_name_val)) = (class_name_val, method_name_val) {
                    if let (Some(class_name_ptr), Some(method_name_ptr)) = (class_name_val.as_string_ptr(), method_name_val.as_string_ptr()) {
                        let class_name = unsafe { (*(class_name_ptr as *const String)).clone() };
                        let method_name = unsafe { (*(method_name_ptr as *const String)).clone() };
                        if let Some(params) = ctx.get_method_params(&class_name, &method_name) {
                            for param in params {
                                if param.4 {
                                    return Ok(Value::new_bool(true));
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_method_returns_reference(this: Value) {
        let _ = this;
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_property_get_file_name(this: Value) {
        let _ = this;
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_property_get_doc_comment(this: Value) {
        let _ = this;
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_property_get_type(this: Value) {
        let _ = this;
        Ok(Value::null())
    }
}

php_function! {
    native_reflection_property_has_type(this: Value) {
        let _ = this;
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_property_has_default_value(this: Value) |ctx| {
        let Some(this) = this else { return Ok(Value::new_bool(false)) };
        let Some(obj_ptr) = this.as_object_ptr() else { return Ok(Value::new_bool(false)) };
        let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
        if obj.properties.contains_key("default_value") {
            return Ok(Value::new_bool(true));
        }
        let class_name = obj.properties.get("class").and_then(|v| v.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() });
        let prop_name = obj.properties.get("name").and_then(|v| v.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() });
        if let (Some(class_name), Some(prop_name)) = (class_name, prop_name) {
            if ctx.get_class_property_info(&class_name, &prop_name).is_some() {
                return Ok(Value::new_bool(true));
            }
        }
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_property_get_default_value(this: Value) |ctx| {
        let Some(this) = this else { return Ok(Value::null()) };
        let Some(obj_ptr) = this.as_object_ptr() else { return Ok(Value::null()) };
        let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
        if let Some(v) = obj.properties.get("default_value") {
            return Ok(*v);
        }
        let class_name = obj.properties.get("class").and_then(|v| v.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() });
        let prop_name = obj.properties.get("name").and_then(|v| v.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() });
        if let (Some(class_name), Some(prop_name)) = (class_name, prop_name) {
            if let Some(info) = ctx.get_class_property_info(&class_name, &prop_name) {
                return Ok(info.default_value);
            }
        }
        Ok(Value::null())
    }
}

php_function! {
    native_reflection_property_is_initialized(this: Value, object: Value) {
        let _ = (this, object);
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_reflection_property_is_readonly(this: Value) {
        let Some(this) = this else { return Ok(Value::new_bool(false)) };
        let Some(obj_ptr) = this.as_object_ptr() else { return Ok(Value::new_bool(false)) };
        let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
        if let Some(v) = obj.properties.get("is_readonly") {
            return Ok(*v);
        }
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_property_is_promoted(this: Value) {
        let _ = this;
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_property_is_default(this: Value) {
        let _ = this;
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_reflection_property_is_virtual(this: Value) {
        let _ = this;
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_property_is_dynamic(this: Value) {
        let _ = this;
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_property_is_abstract(this: Value) {
        let _ = this;
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_property_is_final(this: Value) {
        let _ = this;
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_property_is_lazy(this: Value) {
        let _ = this;
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_property_is_protected_set(this: Value) {
        let _ = this;
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_property_is_private_set(this: Value) {
        let _ = this;
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_property_has_hook(this: Value) {
        let _ = this;
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_property_has_hooks(this: Value) {
        let _ = this;
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_property_get_hook(this: Value) {
        let _ = this;
        Ok(Value::null())
    }
}

php_function! {
    native_reflection_property_get_hooks(this: Value) |ctx| {
        let _ = (this, ctx);
        let ptr = crate::into_raw(Box::new(PhpArray::new()));
        Ok(Value::new_array_ptr(ptr as *mut ()))
    }
}

php_function! {
    native_reflection_property_get_settable_type(this: Value) {
        let _ = this;
        Ok(Value::null())
    }
}

php_function! {
    native_reflection_parameter_construct(this: Value, function: Value, param: Value) |ctx| {
        if let Some(this) = this {
            if this.is_object() {
                let obj_ptr = this.as_object_ptr().unwrap() as *mut hyperion_core::types::object::PhpObject;
                unsafe {
                    if let Some(function) = function {
                        (*obj_ptr).properties.insert("function".to_string(), *function);
                    }
                    if let Some(param) = param {
                        (*obj_ptr).properties.insert("name".to_string(), *param);
                    }
                }
            }
        }
        Ok(Value::null())
    }
}

php_function! {
    native_reflection_parameter_get_type(this: Value) |ctx| {
        // `type` is stashed on the ReflectionParameter by getParameters().
        // A leading `?` means nullable; strip it so getName() matches PHP.
        let raw = match obj_string_prop(this, "type") {
            Some(t) => t,
            None => return Ok(Value::null()),
        };
        let nullable = raw.starts_with('?');
        let name = raw.trim_start_matches('?').to_string();
        if name.is_empty() {
            return Ok(Value::null());
        }
        // Union/intersection types are not ReflectionNamedType in PHP; Laravel's
        // Util::getParameterClassName relies on that instanceof check failing.
        let class = if name.contains('|') || name.contains('&') {
            "ReflectionUnionType"
        } else {
            "ReflectionNamedType"
        };
        let class_id = match ctx.get_class_id(class) {
            Some(id) => id,
            None => return Ok(Value::null()),
        };
        let mut t = hyperion_core::types::object::PhpObject::new(class_id);
        let allows_null = nullable || name.to_lowercase() == "mixed" || name.to_lowercase() == "null";
        t.properties.insert("name".to_string(), php_string(name));
        t.properties.insert("nullable".to_string(), Value::new_bool(allows_null));
        let ptr = ctx.get_arena().alloc_and_track(t);
        Ok(Value::new_object_ptr(ptr as *mut ()))
    }
}

php_function! {
    native_reflection_parameter_is_optional(this: Value) {
        Ok(Value::new_bool(obj_bool_prop(this, "has_default").unwrap_or(false)))
    }
}

php_function! {
    native_reflection_parameter_is_default_value_available(this: Value) {
        Ok(Value::new_bool(obj_bool_prop(this, "has_default").unwrap_or(false)))
    }
}

php_function! {
    native_reflection_parameter_get_default_value(this: Value) {
        // The compiled default is materialised onto the parameter object by
        // getParameters(); absent means the caller already checked
        // isDefaultValueAvailable() and this is unreachable in practice.
        if let Some(this) = this {
            if this.is_object() {
                let obj_ptr = this.as_object_ptr().unwrap()
                    as *const hyperion_core::types::object::PhpObject;
                if let Some(v) = unsafe { (*obj_ptr).properties.get("default") } {
                    return Ok(*v);
                }
            }
        }
        Ok(Value::null())
    }
}

php_function! {
    native_reflection_type_allows_null() {
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_reflection_named_type_get_name(this: Value) {
        Ok(php_string(obj_string_prop(this, "name").unwrap_or_else(|| "mixed".to_string())))
    }
}

php_function! {
    native_reflection_named_type_is_builtin(this: Value) {
        let name = obj_string_prop(this, "name").unwrap_or_else(|| "mixed".to_string());
        Ok(Value::new_bool(is_builtin_type(&name)))
    }
}

php_function! {
    native_reflection_method_get_parameters(this: Value) |ctx| {
        let mut arr = PhpArray::new();
        if let Some(this) = this {
            if this.is_object() {
                let obj_ptr = this.as_object_ptr().unwrap() as *mut hyperion_core::types::object::PhpObject;
                let class_name_val = unsafe { (*obj_ptr).properties.get("class") };
                let method_name_val = unsafe { (*obj_ptr).properties.get("name") };
                if let (Some(class_name_val), Some(method_name_val)) = (class_name_val, method_name_val) {
                    if let (Some(class_name_ptr), Some(method_name_ptr)) = (class_name_val.as_string_ptr(), method_name_val.as_string_ptr()) {
                        let class_name = unsafe { (*(class_name_ptr as *const String)).clone() };
                        let method_name = unsafe { (*(method_name_ptr as *const String)).clone() };
                        if let Some(params) = ctx.get_method_params(&class_name, &method_name) {
                            if let Some(param_class_id) = ctx.get_class_id("ReflectionParameter") {
                                for (i, param) in params.iter().enumerate() {
                                    let mut param_obj = hyperion_core::types::object::PhpObject::new(param_class_id);
                                    param_obj.properties.insert("name".to_string(), php_string(param.0.clone()));
                                    param_obj.properties.insert("has_default".to_string(), Value::new_bool(param.2));
                                    param_obj.properties.insert("by_ref".to_string(), Value::new_bool(param.3));
                                    param_obj.properties.insert("variadic".to_string(), Value::new_bool(param.4));
                                    param_obj.properties.insert("position".to_string(), Value::new_int(i as i32));
                                    param_obj.properties.insert("declaring_class".to_string(), php_string(class_name.clone()));
                                    if let Some(ref def_val) = param.5 {
                                        param_obj.properties.insert("default".to_string(), *def_val);
                                    }
                                    if let Some(type_hint) = &param.1 {
                                        param_obj.properties.insert("type".to_string(), php_string(type_hint.clone()));
                                    }
                                    let param_obj_ptr = ctx.get_arena().alloc_and_track(param_obj);
                                    arr.insert_int(i as i64, Value::new_object_ptr(param_obj_ptr as *mut ()));
                                }
                            }
                        }
                    }
                }
            }
        }
        let arr_ptr = ctx.get_arena().alloc_and_track(arr);
        Ok(Value::new_array_ptr(arr_ptr as *mut ()))
    }
}


php_function! {
    native_reflection_parameter_get_declaring_class(this: Value) |ctx| {
        // getParameters() records the class the method was found on.
        let class_name = match obj_string_prop(this, "declaring_class") {
            Some(c) => c,
            None => return Ok(Value::null()),
        };
        let class_id = match ctx.get_class_id("ReflectionClass") {
            Some(id) => id,
            None => return Ok(Value::null()),
        };
        let mut rc = hyperion_core::types::object::PhpObject::new(class_id);
        rc.properties.insert("name".to_string(), php_string(class_name));
        let ptr = ctx.get_arena().alloc_and_track(rc);
        Ok(Value::new_object_ptr(ptr as *mut ()))
    }
}

php_function! {
    native_reflection_parameter_is_variadic(this: Value) {
        Ok(Value::new_bool(obj_bool_prop(this, "variadic").unwrap_or(false)))
    }
}

php_function! {
    native_reflection_parameter_is_passed_by_reference(this: Value) {
        Ok(Value::new_bool(obj_bool_prop(this, "by_ref").unwrap_or(false)))
    }
}

php_function! {
    native_get_object_vars(obj: Value) |ctx| {
        let mut arr = PhpArray::new();
        let Some(obj_val) = obj else {
            let ptr = ctx.get_arena().alloc_and_track(arr);
            return Ok(Value::new_array_ptr(ptr as *mut ()));
        };
        if let Some(obj_ptr) = obj_val.as_object_ptr() {
            let obj_ref = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
            for (k, v) in &obj_ref.properties {
                let k_id = ctx.intern_string(k);
                arr.insert_string_id(k_id, *v);
            }
        }
        let ptr = ctx.get_arena().alloc_and_track(arr);
        Ok(Value::new_array_ptr(ptr as *mut ()))
    }
}

php_function! {
    native_reflection_parameter_has_default_value(this: Value) {
        Ok(Value::new_bool(obj_bool_prop(this, "has_default").unwrap_or(false)))
    }
}

php_function! {
    native_reflection_parameter_get_name(this: Value) {
        Ok(php_string(obj_string_prop(this, "name").unwrap_or_else(|| "param".to_string())))
    }
}

php_function! {
    native_reflection_parameter_get_position(this: Value) {
        if let Some(this) = this {
            if this.is_object() {
                let obj_ptr = this.as_object_ptr().unwrap()
                    as *const hyperion_core::types::object::PhpObject;
                if let Some(v) = unsafe { (*obj_ptr).properties.get("position") } {
                    return Ok(*v);
                }
            }
        }
        Ok(Value::new_int(0))
    }
}

php_function! {
    native_reflection_parameter_has_type(this: Value) {
        Ok(Value::new_bool(obj_string_prop(this, "type").is_some_and(|t| !t.is_empty())))
    }
}

php_function! {
    native_reflection_parameter_allows_null() {
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_reflection_named_type_allows_null(this: Value) {
        // On a ReflectionNamedType this reads the `nullable` flag recorded from
        // the `?T` prefix; on a ReflectionParameter with no type at all, PHP
        // reports true (an untyped parameter accepts null).
        match obj_bool_prop(this, "nullable") {
            Some(b) => Ok(Value::new_bool(b)),
            None => Ok(Value::new_bool(
                !obj_string_prop(this, "type").is_some_and(|t| !t.is_empty()),
            )),
        }
    }
}

php_function! {
    native_class_implements(class_or_obj: Value, autoload: Value) |ctx| {
        let class_name = match class_or_obj {
            Some(v) if v.is_object() => {
                let obj_ptr = v.as_object_ptr().unwrap() as *const hyperion_core::types::object::PhpObject;
                ctx.get_class_name(unsafe { (*obj_ptr).class_id })
            }
            Some(v) => v.as_string_ptr().map(|p| unsafe { (*(p as *const String)).clone() }),
            None => None,
        };
        let Some(class_name) = class_name else {
            return Ok(Value::new_bool(false));
        };
        let do_autoload = autoload.and_then(|v| v.as_bool()).unwrap_or(true);
        if do_autoload && !ctx.class_exists(&class_name) {
            ctx.trigger_autoload_sync(&class_name);
        }
        if !ctx.class_exists(&class_name) {
            return Ok(Value::new_bool(false));
        }
        let ifaces = ctx.get_implemented_interfaces(&class_name);
        let mut arr = PhpArray::new();
        for iface in ifaces {
            let boxed_key = crate::into_raw(Box::new(iface.clone()));
            let boxed_val = crate::into_raw(Box::new(iface));
            let k_id = ctx.intern_string(unsafe { &*boxed_key });
            arr.insert_string_id(k_id, Value::new_string_ptr(boxed_val as *mut ()));
        }
        let ptr = ctx.get_arena().alloc_and_track(arr);
        Ok(Value::new_array_ptr(ptr as *mut ()))
    }
}

php_function! {
    native_class_parents(class_or_obj: Value, autoload: Value) |ctx| {
        let class_name = match class_or_obj {
            Some(v) if v.is_object() => {
                let obj_ptr = v.as_object_ptr().unwrap() as *const hyperion_core::types::object::PhpObject;
                ctx.get_class_name(unsafe { (*obj_ptr).class_id })
            }
            Some(v) => v.as_string_ptr().map(|p| unsafe { (*(p as *const String)).clone() }),
            None => None,
        };
        let Some(class_name) = class_name else {
            return Ok(Value::new_bool(false));
        };
        let do_autoload = autoload.and_then(|v| v.as_bool()).unwrap_or(true);
        if do_autoload && !ctx.class_exists(&class_name) {
            ctx.trigger_autoload_sync(&class_name);
        }
        if !ctx.class_exists(&class_name) {
            return Ok(Value::new_bool(false));
        }
        let parents = ctx.get_parent_classes(&class_name);
        let mut arr = PhpArray::new();
        for parent in parents {
            let boxed_key = crate::into_raw(Box::new(parent.clone()));
            let boxed_val = crate::into_raw(Box::new(parent));
            let k_id = ctx.intern_string(unsafe { &*boxed_key });
            arr.insert_string_id(k_id, Value::new_string_ptr(boxed_val as *mut ()));
        }
        let ptr = ctx.get_arena().alloc_and_track(arr);
        Ok(Value::new_array_ptr(ptr as *mut ()))
    }
}

php_function! {
    native_class_uses(class_or_obj: Value, autoload: Value) |ctx| {
        let class_name = match class_or_obj {
            Some(v) if v.is_object() => {
                let obj_ptr = v.as_object_ptr().unwrap() as *const hyperion_core::types::object::PhpObject;
                ctx.get_class_name(unsafe { (*obj_ptr).class_id })
            }
            Some(v) => v.as_string_ptr().map(|p| unsafe { (*(p as *const String)).clone() }),
            None => None,
        };
        let Some(class_name) = class_name else {
            return Ok(Value::new_bool(false));
        };
        let do_autoload = autoload.and_then(|v| v.as_bool()).unwrap_or(true);
        if do_autoload && !ctx.class_exists(&class_name) {
            ctx.trigger_autoload_sync(&class_name);
        }
        if !ctx.class_exists(&class_name) {
            return Ok(Value::new_bool(false));
        }
        let traits = ctx.get_class_traits(&class_name);
        let mut arr = PhpArray::new();
        for tr in traits {
            let boxed_key = crate::into_raw(Box::new(tr.clone()));
            let boxed_val = crate::into_raw(Box::new(tr));
            let k_id = ctx.intern_string(unsafe { &*boxed_key });
            arr.insert_string_id(k_id, Value::new_string_ptr(boxed_val as *mut ()));
        }
        let ptr = ctx.get_arena().alloc_and_track(arr);
        Ok(Value::new_array_ptr(ptr as *mut ()))
    }
}

php_function! {
    native_get_class_methods(class_or_object: Value) |ctx| {
        let class_name = if let Some(v) = class_or_object {
            if let Some(obj_ptr) = v.as_object_ptr() {
                let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
                obj.class_name.clone().or_else(|| ctx.get_class_name(obj.class_id))
            } else if let Some(str_ptr) = v.as_string_ptr() {
                Some(unsafe { (*(str_ptr as *const String)).clone() })
            } else {
                None
            }
        } else {
            None
        };
        let Some(cname) = class_name else {
            return Ok(Value::null());
        };
        if let Some(methods) = ctx.get_class_methods(&cname) {
            let mut arr = PhpArray::new();
            for (i, m) in methods.into_iter().enumerate() {
                let m_ptr = ctx.get_arena().alloc_and_track(m);
                arr.insert_int(i as i64, Value::new_string_ptr(m_ptr as *mut ()));
            }
            let arr_ptr = ctx.get_arena().alloc_and_track(arr);
            Ok(Value::new_array_ptr(arr_ptr as *mut ()))
        } else {
            Ok(Value::null())
        }
    }
}

php_function! {
    native_reflection_reference_from_array_element(array: Value, key: Value) |ctx| {
        let _ = (array, key, ctx);
        Ok(Value::null())
    }
}

php_function! {
    native_reflection_reference_get_id(this: Value) {
        let _ = this;
        let s = crate::into_raw(Box::new("0".to_string()));
        Ok(Value::new_string_ptr(s as *mut ()))
    }
}

php_function! {
    native_reflection_parameter_to_string(this: Value) |ctx| {
        let name = obj_string_prop(this, "name").unwrap_or_default();
        let pos = obj_int_prop(this, "position").unwrap_or(0);
        let type_str = obj_string_prop(this, "type").map(|t| format!("{} ", t)).unwrap_or_default();
        let s = format!("Parameter #{} [ {}${} ]", pos, type_str, name);
        let ptr = ctx.get_arena().alloc_and_track(s);
        Ok(Value::new_string_ptr(ptr as *mut ()))
    }
}

php_function! {
    native_reflection_type_to_string(this: Value) |ctx| {
        let name = obj_string_prop(this, "name").unwrap_or_default();
        let ptr = ctx.get_arena().alloc_and_track(name);
        Ok(Value::new_string_ptr(ptr as *mut ()))
    }
}

php_function! {
    native_reflection_property_get_attributes(this: Value, name: Value, flags: Value) |ctx| {
        let _ = (this, name, flags);
        let arr = PhpArray::new();
        let arr_ptr = ctx.get_arena().alloc_and_track(arr);
        Ok(Value::new_array_ptr(arr_ptr as *mut ()))
    }
}

php_function! {
    native_reflection_class_constant_construct(this: Value, class: Value, name: Value) |ctx| {
        let Some(this) = this else { return Ok(Value::null()); };
        let Some(this_ptr) = this.as_object_ptr() else { return Ok(Value::null()); };
        let obj = unsafe { &mut *(this_ptr as *mut hyperion_core::types::object::PhpObject) };
        let class_name = if let Some(class) = class {
            if let Some(s) = class.as_string_ptr() {
                unsafe { (*(s as *const String)).clone() }
            } else if class.is_object() {
                let obj_ptr = class.as_object_ptr().unwrap() as *const hyperion_core::types::object::PhpObject;
                let php_obj = unsafe { &*obj_ptr };
                if let Some(ref cn) = php_obj.class_name {
                    cn.clone()
                } else {
                    ctx.get_class_name(php_obj.class_id).unwrap_or_default()
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        let const_name = name.and_then(|n| n.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() }).unwrap_or_default();
        obj.properties.insert("class".to_string(), php_string(class_name));
        obj.properties.insert("name".to_string(), php_string(const_name));
        Ok(Value::null())
    }
}

php_function! {
    native_reflection_class_constant_get_name(this: Value) {
        if let Some(this) = this {
            if let Some(obj_ptr) = this.as_object_ptr() {
                let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
                if let Some(name_val) = obj.properties.get("name") {
                    return Ok(*name_val);
                }
            }
        }
        Ok(Value::null())
    }
}

php_function! {
    native_reflection_class_constant_get_value(this: Value) |ctx| {
        let Some(this) = this else { return Ok(Value::null()); };
        let Some(this_ptr) = this.as_object_ptr() else { return Ok(Value::null()); };
        let obj = unsafe { &*(this_ptr as *const hyperion_core::types::object::PhpObject) };
        if let Some(v) = obj.properties.get("value") {
            return Ok(*v);
        }
        let class_name = obj.properties.get("class").and_then(|v| v.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() }).unwrap_or_default();
        let const_name = obj.properties.get("name").and_then(|v| v.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() }).unwrap_or_default();
        let scoped_name = format!("{}::{}", class_name, const_name);
        if let Some(val) = ctx.lookup_constant(&scoped_name) {
            return Ok(val);
        }
        if let Some(val) = ctx.get_static_property(&class_name, &const_name) {
            return Ok(val);
        }
        Ok(Value::null())
    }
}

php_function! {
    native_reflection_class_constant_get_declaring_class(this: Value) |ctx| {
        let Some(this) = this else { return Ok(Value::null()); };
        let Some(this_ptr) = this.as_object_ptr() else { return Ok(Value::null()); };
        let obj = unsafe { &*(this_ptr as *const hyperion_core::types::object::PhpObject) };
        let class_name = obj.properties.get("class").and_then(|v| v.as_string_ptr()).map(|p| unsafe { (*(p as *const String)).clone() }).unwrap_or_default();
        let Some(rc_id) = ctx.get_class_id("ReflectionClass") else { return Ok(Value::null()); };
        let mut rc_obj = hyperion_core::types::object::PhpObject::new(rc_id);
        rc_obj.properties.insert("name".to_string(), php_string(class_name));
        let rc_ptr = ctx.get_arena().alloc_and_track(rc_obj);
        Ok(Value::new_object_ptr(rc_ptr as *mut ()))
    }
}

php_function! {
    native_reflection_class_constant_is_public(this: Value) {
        if let Some(b) = obj_bool_prop(this, "is_public") {
            Ok(Value::new_bool(b))
        } else {
            Ok(Value::new_bool(true))
        }
    }
}

php_function! {
    native_reflection_class_constant_is_protected(this: Value) {
        if let Some(b) = obj_bool_prop(this, "is_protected") {
            Ok(Value::new_bool(b))
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_reflection_class_constant_is_private(this: Value) {
        if let Some(b) = obj_bool_prop(this, "is_private") {
            Ok(Value::new_bool(b))
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_reflection_class_constant_is_final(this: Value) {
        let _ = this;
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_class_constant_get_modifiers(this: Value) {
        let is_protected = obj_bool_prop(this, "is_protected").unwrap_or(false);
        let is_private = obj_bool_prop(this, "is_private").unwrap_or(false);
        let mut mod_mask = 1; // IS_PUBLIC = 1
        if is_protected {
            mod_mask = 2; // IS_PROTECTED = 2
        } else if is_private {
            mod_mask = 4; // IS_PRIVATE = 4
        }
        Ok(Value::new_int(mod_mask))
    }
}

php_function! {
    native_reflection_class_constant_get_doc_comment(this: Value) {
        let _ = this;
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_reflection_class_constant_get_attributes(this: Value, name: Value, flags: Value) |ctx| {
        let _ = (this, name, flags);
        let arr = PhpArray::new();
        let arr_ptr = ctx.get_arena().alloc_and_track(arr);
        Ok(Value::new_array_ptr(arr_ptr as *mut ()))
    }
}

php_function! {
    native_reflection_class_constant_to_string(this: Value) {
        let name = obj_string_prop(this, "name").unwrap_or_default();
        let class = obj_string_prop(this, "class").unwrap_or_default();
        let s = format!("Constant [ public {}::{} ]", class, name);
        Ok(php_string(s))
    }
}

