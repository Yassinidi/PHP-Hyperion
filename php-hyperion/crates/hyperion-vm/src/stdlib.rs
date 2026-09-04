use std::sync::OnceLock;
use indexmap::IndexMap;
use hyperion_core::types::function::NativeFn;

pub struct StdlibRegistry {
    pub functions: IndexMap<String, NativeFn>,
}

impl StdlibRegistry {
    fn new() -> Self {
        let mut registry = Self {
            functions: IndexMap::new(),
        };
        // Register the standard library functions from stdlib crate
        let funcs = hyperion_ext_standard::get_stdlib_functions();
        for (name, func) in funcs {
            registry.register(name, func);
        }
        
        registry.register("hyperion_accept_request".to_string(), hyperion_accept_request);
        registry.register("hyperion_route_register".to_string(), hyperion_route_register);
        registry.register("hyperion_route_match".to_string(), hyperion_route_match);
        
        registry
    }

    fn register(&mut self, name: String, func: NativeFn) -> usize {
        let (index, _) = self.functions.insert_full(name, func);
        index
    }

    pub fn global() -> &'static Self {
        static REGISTRY: OnceLock<StdlibRegistry> = OnceLock::new();
        REGISTRY.get_or_init(StdlibRegistry::new)
    }
}

fn hyperion_accept_request(
    _args: &[hyperion_core::memory::nan_box::Value],
    _ctx: &mut dyn hyperion_core::types::function::NativeContext,
) -> Result<hyperion_core::memory::nan_box::Value, String> {
    Ok(hyperion_core::memory::nan_box::Value::new_yield(0xFFFFFFFFFFFF))
}

fn hyperion_route_register(
    args: &[hyperion_core::memory::nan_box::Value],
    _ctx: &mut dyn hyperion_core::types::function::NativeContext,
) -> Result<hyperion_core::memory::nan_box::Value, String> {
    if args.len() < 2 {
        return Err("hyperion_route_register expects at least 2 parameters (method, pattern)".to_string());
    }
    let method = match args[0].deref().as_string_ptr() {
        Some(ptr) => unsafe { &*(ptr as *const String) },
        None => return Ok(hyperion_core::memory::nan_box::Value::new_bool(false)),
    };
    let pattern = match args[1].deref().as_string_ptr() {
        Some(ptr) => unsafe { &*(ptr as *const String) },
        None => return Ok(hyperion_core::memory::nan_box::Value::new_bool(false)),
    };
    let route_id = if args.len() >= 3 {
        args[2].deref().as_int().unwrap_or(0) as u32
    } else {
        0
    };
    let metadata = if args.len() >= 4 {
        if let Some(ptr) = args[3].deref().as_string_ptr() {
            unsafe { (*(ptr as *const String)).clone() }
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    crate::route_trie::get_global_route_tree().insert(method, pattern, route_id, &metadata);
    Ok(hyperion_core::memory::nan_box::Value::new_bool(true))
}

fn hyperion_route_match(
    args: &[hyperion_core::memory::nan_box::Value],
    ctx: &mut dyn hyperion_core::types::function::NativeContext,
) -> Result<hyperion_core::memory::nan_box::Value, String> {
    if args.len() < 2 {
        return Err("hyperion_route_match expects 2 parameters (method, uri)".to_string());
    }
    let method = match args[0].deref().as_string_ptr() {
        Some(ptr) => unsafe { &*(ptr as *const String) },
        None => return Ok(hyperion_core::memory::nan_box::Value::null()),
    };
    let uri = match args[1].deref().as_string_ptr() {
        Some(ptr) => unsafe { &*(ptr as *const String) },
        None => return Ok(hyperion_core::memory::nan_box::Value::null()),
    };

    if let Some(m) = crate::route_trie::get_global_route_tree().match_path(method, uri) {
        let k_route_id = ctx.intern_string("route_id");
        let k_meta = ctx.intern_string("metadata");
        let k_params = ctx.intern_string("params");
        let interned_params: Vec<(usize, String)> = m.params.into_iter().map(|(k, v)| {
            (ctx.intern_string(&k), v)
        }).collect();

        let arena = ctx.get_arena();
        let mut arr = hyperion_core::types::array::PhpArray::new();
        arr.insert_string_id(k_route_id, hyperion_core::memory::nan_box::Value::new_int(m.route_id as i32));
        
        let meta_str = arena.alloc_and_track(m.metadata);
        arr.insert_string_id(k_meta, hyperion_core::memory::nan_box::Value::new_string_ptr(meta_str as *mut ()));

        let mut params_arr = hyperion_core::types::array::PhpArray::new();
        for (k_id, v) in interned_params {
            let v_str = arena.alloc_and_track(v);
            params_arr.insert_string_id(k_id, hyperion_core::memory::nan_box::Value::new_string_ptr(v_str as *mut ()));
        }
        let params_ptr = arena.alloc_and_track(params_arr);
        arr.insert_string_id(k_params, hyperion_core::memory::nan_box::Value::new_array_ptr(params_ptr as *mut ()));

        let arr_ptr = arena.alloc_and_track(arr);
        Ok(hyperion_core::memory::nan_box::Value::new_array_ptr(arr_ptr as *mut ()))
    } else {
        Ok(hyperion_core::memory::nan_box::Value::null())
    }
}

