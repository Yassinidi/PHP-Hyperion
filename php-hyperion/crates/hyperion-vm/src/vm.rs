#[macro_export]
macro_rules! debug_println {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        {
            if std::env::var("HYPERION_DEBUG").is_ok() {
                eprintln!($($arg)*);
            }
        }
    };
}

#[macro_export]
macro_rules! debug_eprintln {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        {
            if std::env::var("HYPERION_DEBUG").is_ok() {
                eprintln!($($arg)*);
            }
        }
    };
}

use crate::fibre::{CallFrame, Fibre};
use crate::stdlib::StdlibRegistry;
use crate::types::closure::PhpClosure;
use hyperion_bytecode::Opcode;
use hyperion_core::memory::nan_box::{Value, ValueType};
use hyperion_core::types::function::NativeContext;

pub fn is_truthy(val: &Value) -> bool {
    if val.is_bool() {
        val.as_bool().unwrap_or(false)
    } else if let Some(i) = val.as_int() {
        i != 0
    } else if val.is_null() {
        false
    } else if let Some(ptr) = val.as_array_ptr() {
        let arr = unsafe { &*(ptr as *const hyperion_core::types::array::PhpArray) };
        arr.len() > 0
    } else if let Some(ptr) = val.as_string_ptr() {
        let s = unsafe { &*(ptr as *const String) };
        !s.is_empty() && s != "0"
    } else {
        true
    }
}

pub fn normalize_name(name: &str) -> String {
    let name = if let Some(stripped) = name.strip_prefix('\\') {
        stripped
    } else {
        name
    };
    if name.bytes().all(|b| !b.is_ascii_uppercase()) {
        name.to_string()
    } else {
        name.to_ascii_lowercase()
    }
}

/// Resolve a constant name against the global table the way PHP does.
///
/// Constant names are case-sensitive — unlike function and class names, which
/// `normalize_name` lowercases — so only the namespace part is interpreted here.
/// A leading `\` marks a fully-qualified name and is not part of the name
/// itself, and an unqualified name used inside a namespace arrives already
/// prefixed, with PHP falling back to the global constant when no namespaced one
/// exists. Both reduce to: try the name as written, then its last segment.
///
/// Symfony writes `\STDOUT`. Without this, the exact-match lookup missed and the
/// bareword fallback pushed the *string* `"\STDOUT"`, so `is_resource()` on it
/// was false and `ConsoleOutput` could not be constructed.
pub fn resolve_constant(
    constants: &dashmap::DashMap<String, Value>,
    name: &str,
) -> Option<Value> {
    if let Some(v) = constants.get(name) {
        return Some(*v);
    }
    let short = name.rsplit('\\').next().unwrap_or(name);
    if short != name {
        return constants.get(short).map(|v| *v);
    }
    None
}

pub struct TraceRecord {
    pub opcode: Opcode,
    pub operand_types: Vec<ValueType>,
}

#[derive(Debug)]
pub enum ExecutionResult {
    Finished,
    Yielded(YieldReason),
    Preempted,
    Error(String),
    UncaughtException(String),
}

#[derive(Debug)]
pub enum YieldReason {
    Network,
    Disk,
    AsyncInclude(String),
    Database(u64),
    GeneratorYield,
    OutputFlush,
    WaitForHttpRequest,
}

// JIT FFI Callouts
#[no_mangle]
pub extern "C" fn jit_fetch_array_element(array_val_raw: u64, key_val_raw: u64) -> u64 {
    let array_val = Value(array_val_raw);
    let key_val = Value(key_val_raw);

    // Safety check: Is it actually an array? (The JIT GuardType should ensure this)
    if let Some(arr_ptr) = array_val.as_array_ptr() {
        let arr = unsafe { &*(arr_ptr as *const hyperion_core::types::array::PhpArray) };

        let key = if let Some(i) = key_val.as_int() {
            hyperion_core::types::array::ArrayKey::Int(i as i64)
        } else if let Some(s_ptr) = key_val.as_string_ptr() {
            let s = unsafe { &*(s_ptr as *const String) };
            hyperion_core::types::array::ArrayKey::StringId(hyperion_core::types::string_table::intern_string(s))
        } else {
            return Value::null().0;
        };

        if let Some(val) = arr.elements.get(&key) {
            return val.0;
        } 
    } 

    Value::null().0
}



const METHOD_CACHE_SIZE: usize = 16384;
const METHOD_CACHE_MASK: usize = METHOD_CACHE_SIZE - 1;

#[derive(Clone, Copy)]
pub struct MethodCacheSlot {
    pub class_id: usize,
    pub name_hash: u64,
    pub func_ptr: crate::types::function::FunctionPtr,
}

#[inline(always)]
pub fn hash_method_name(name: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in name.bytes() {
        h ^= (b.to_ascii_lowercase()) as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

const STATIC_PROP_CACHE_SIZE: usize = 16384;
const STATIC_PROP_CACHE_MASK: usize = STATIC_PROP_CACHE_SIZE - 1;

#[derive(Clone, Copy)]
pub struct StaticPropCacheSlot {
    pub class_hash: u64,
    pub prop_hash: u64,
    pub val: Value,
    pub is_set: bool,
}

thread_local! {
    static FAST_METHOD_CACHE: std::cell::RefCell<[MethodCacheSlot; METHOD_CACHE_SIZE]> = std::cell::RefCell::new([MethodCacheSlot {
        class_id: usize::MAX,
        name_hash: 0,
        func_ptr: crate::types::function::FunctionPtr(std::ptr::null_mut()),
    }; METHOD_CACHE_SIZE]);
    static FAST_STATIC_PROP_CACHE: std::cell::RefCell<[StaticPropCacheSlot; STATIC_PROP_CACHE_SIZE]> = std::cell::RefCell::new([StaticPropCacheSlot {
        class_hash: u64::MAX,
        prop_hash: 0,
        val: Value::null(),
        is_set: false,
    }; STATIC_PROP_CACHE_SIZE]);
    static METHOD_L1_CACHE: std::cell::RefCell<std::collections::HashMap<(usize, String), Option<crate::types::function::FunctionPtr>>> = std::cell::RefCell::new(std::collections::HashMap::with_capacity(4096));
    static PROPERTY_L1_CACHE: std::cell::RefCell<std::collections::HashMap<(usize, String), bool>> = std::cell::RefCell::new(std::collections::HashMap::with_capacity(4096));
    static INSTANCE_OF_L1_CACHE: std::cell::RefCell<std::collections::HashMap<(usize, String), bool>> = std::cell::RefCell::new(std::collections::HashMap::with_capacity(4096));
    static DEFAULT_PROPERTIES_L1_CACHE: std::cell::RefCell<std::collections::HashMap<usize, Vec<(String, Value)>>> = std::cell::RefCell::new(std::collections::HashMap::with_capacity(4096));
    static STATIC_PROP_L1_CACHE: std::cell::RefCell<std::collections::HashMap<(String, String), Option<Value>>> = std::cell::RefCell::new(std::collections::HashMap::with_capacity(4096));
}

static JIT_THRESHOLD: std::sync::OnceLock<u32> = std::sync::OnceLock::new();

#[inline(always)]
fn get_jit_threshold() -> u32 {
    *JIT_THRESHOLD.get_or_init(|| {
        std::env::var("HYPERION_JIT_THRESHOLD")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(u32::MAX)
    })
}

pub struct VM;

impl VM {
    pub fn is_inlinable_leaf_function(func: &crate::types::function::PhpFunction) -> bool {
        if func.chunk.code.is_empty() || func.chunk.code.len() > 128 {
            return false;
        }
        if !func.chunk.finally_handlers.is_empty() {
            return false;
        }
        let code = &func.chunk.code;
        let mut i = 0;
        while i < code.len() {
            let byte = code[i];
            match Opcode::try_from(byte) {
                Ok(Opcode::Call)
                | Ok(Opcode::CallUnpacked)
                | Ok(Opcode::CallNative)
                | Ok(Opcode::CallConstruct)
                | Ok(Opcode::CallConstructUnpacked)
                | Ok(Opcode::CallNamed)
                | Ok(Opcode::CallIntrinsic)
                | Ok(Opcode::MethodCall)
                | Ok(Opcode::MethodCallUnpacked)
                | Ok(Opcode::StaticMethodCall)
                | Ok(Opcode::StaticMethodCallUnpacked)
                | Ok(Opcode::Yield)
                | Ok(Opcode::CreateGenerator)
                | Ok(Opcode::Eval)
                | Ok(Opcode::Include)
                | Ok(Opcode::IncludeOnce)
                | Ok(Opcode::Throw)
                | Ok(Opcode::Loop) => {
                    return false;
                }
                _ => {}
            }
            i += 1;
        }
        true
    }

    #[inline(always)]
    pub fn invalidate_method_cache() {
        FAST_METHOD_CACHE.with(|c| {
            let mut cache = c.borrow_mut();
            for slot in cache.iter_mut() {
                slot.class_id = usize::MAX;
                slot.name_hash = 0;
                slot.func_ptr = crate::types::function::FunctionPtr(std::ptr::null_mut());
            }
        });
        METHOD_L1_CACHE.with(|c| c.borrow_mut().clear());

        PROPERTY_L1_CACHE.with(|c| c.borrow_mut().clear());
        INSTANCE_OF_L1_CACHE.with(|c| c.borrow_mut().clear());
        DEFAULT_PROPERTIES_L1_CACHE.with(|c| c.borrow_mut().clear());
        STATIC_PROP_L1_CACHE.with(|c| c.borrow_mut().clear());
        Self::invalidate_static_prop_cache();
    }

    #[inline(always)]
    pub fn get_cached_static_prop(class_hash: u64, prop_hash: u64) -> Option<Value> {
        let hash = (class_hash ^ prop_hash.rotate_left(17)) as usize & STATIC_PROP_CACHE_MASK;
        FAST_STATIC_PROP_CACHE.with(|c| {
            let cache = c.borrow();
            let slot = cache[hash];
            if slot.is_set && slot.class_hash == class_hash && slot.prop_hash == prop_hash {
                Some(slot.val)
            } else {
                None
            }
        })
    }

    #[inline(always)]
    pub fn set_cached_static_prop(class_hash: u64, prop_hash: u64, val: Value) {
        let hash = (class_hash ^ prop_hash.rotate_left(17)) as usize & STATIC_PROP_CACHE_MASK;
        FAST_STATIC_PROP_CACHE.with(|c| {
            let mut cache = c.borrow_mut();
            cache[hash] = StaticPropCacheSlot {
                class_hash,
                prop_hash,
                val,
                is_set: true,
            };
        });
    }

    #[inline(always)]
    pub fn invalidate_static_prop_cache() {
        FAST_STATIC_PROP_CACHE.with(|c| {
            let mut cache = c.borrow_mut();
            for slot in cache.iter_mut() {
                slot.is_set = false;
            }
        });
    }


    #[inline(always)]
    pub fn find_class_id(fibre: &Fibre, class_name: &str) -> Option<usize> {
        let norm = normalize_name(class_name);
        fibre.engine_state.class_map.get(&norm).map(|v| *v)
    }

    #[inline(always)]
    pub fn find_func_id(fibre: &Fibre, func_name: &str) -> Option<usize> {
        let norm = normalize_name(func_name);
        fibre.engine_state.func_map.get(&norm).map(|v| *v)
    }

    pub fn trigger_autoload(fibre: &mut Fibre, class_name: &str) -> Result<bool, String> {
        if class_name.eq_ignore_ascii_case("self")
            || class_name.eq_ignore_ascii_case("parent")
            || class_name.eq_ignore_ascii_case("static")
        {
            return Ok(false);
        }
        let norm_class_name = normalize_name(class_name);
        if fibre.engine_state.class_map.contains_key(&norm_class_name) {
            return Ok(false);
        }
        let mut current_idx = *fibre.autoload_state.get(&norm_class_name).unwrap_or(&0);
        let loaders = if !fibre.autoloaders.is_empty() {
            fibre.autoloaders.clone()
        } else {
            let g = fibre.engine_state.autoloaders.read().unwrap().clone();
            fibre.autoloaders = g.clone();
            g
        };
        debug_println!("trigger_autoload: class {} has {} loaders", class_name, loaders.len());

        let mut pushed_autoloader = false;
        while current_idx < loaders.len() {
            let loader = &loaders[current_idx];
            current_idx += 1;
            fibre
                .autoload_state
                .insert(norm_class_name.clone(), current_idx);

            let mut target_func_ptr = None;

            if let Some(loader_closure) = loader.as_closure_ptr() {
                let closure =
                    unsafe { &*(loader_closure as *mut crate::types::closure::PhpClosure) };
                target_func_ptr = Some(closure.function_ptr);
            } else if let Some(l_str_ptr) = loader.as_string_ptr() {
                let l_name = unsafe { &*(l_str_ptr as *const String) };
                let l_name_lower = l_name.to_lowercase();
                if let Some(loader_func) = fibre.engine_state.functions
                    .iter()
                    .find(|f| unsafe { &*f.0 }.name.to_lowercase() == l_name_lower)
                {
                    target_func_ptr = Some(*loader_func);
                }
            } else if let Some(arr_ptr) = loader.as_array_ptr() {
                let arr = unsafe { &*(arr_ptr as *const hyperion_core::types::array::PhpArray) };
                if let (Some(c), Some(m)) = (arr.get_int(0), arr.get_int(1)) {
                    if let (Some(c_str_ptr), Some(m_str_ptr)) =
                        (c.as_string_ptr(), m.as_string_ptr())
                    {
                        let c_str = unsafe { &*(c_str_ptr as *const String) };
                        let m_str = unsafe { &*(m_str_ptr as *const String) };
                        let norm_c_str = normalize_name(c_str);
                        let norm_m_str = m_str.to_lowercase();
                        debug_println!("autoload loader array checking: {}::{}", norm_c_str, norm_m_str);
                        let classes = &fibre.engine_state.classes;
                        let mut target_class_id = 0;
                        let mut found_class = false;
                        if let Some(id) = fibre.engine_state.class_map.get(&norm_c_str).map(|v| *v) {
                            target_class_id = id;
                            found_class = true;
                        } else if let Some((i, _)) = classes
                            .iter()
                            .enumerate()
                            .find(|(_, cls)| normalize_name(&cls.name) == norm_c_str)
                        {
                            target_class_id = i;
                            found_class = true;
                            debug_println!("autoload: found class {} at {}", norm_c_str, i);
                        } else {
                            debug_println!("autoload: class {} not found in engine_state", norm_c_str);
                        }
                        if found_class {
                            target_func_ptr =
                                Self::find_method(&fibre.engine_state, target_class_id, &norm_m_str)
                                    .unwrap_or(None);
                        } 
                    } else if let (Some(obj_ptr), Some(m_str_ptr)) =
                        (c.as_object_ptr(), m.as_string_ptr())
                    {
                        let m_str = unsafe { &*(m_str_ptr as *const String) };
                        let norm_m_str = m_str.to_lowercase();
                        let obj = unsafe {
                            &*(obj_ptr as *const hyperion_core::types::object::PhpObject)
                        };
                        let obj_class_id = Self::resolve_obj_class_id(fibre, obj);
                        target_func_ptr =
                            Self::find_method(&fibre.engine_state, obj_class_id, &norm_m_str).unwrap_or(None);
                    }

                }
            }

            if let Some(func_ptr) = target_func_ptr {
                if fibre.frame_count >= fibre.frames.len() {
                    return Err("Stack overflow: max call depth exceeded".to_string());
                }

                let stack_window = fibre.stack_top;

                let mut call_class_id = 0;
                let mut context_pushed = false;
                if let Some(arr_ptr) = loader.as_array_ptr() {
                    let arr =
                        unsafe { &*(arr_ptr as *const hyperion_core::types::array::PhpArray) };
                    if let Some(first) = arr.get_int(0) {
                        if first.is_object() {
                            let obj = unsafe {
                                &*(first.as_object_ptr().unwrap()
                                    as *const hyperion_core::types::object::PhpObject)
                            };
                            call_class_id = Self::resolve_obj_class_id(fibre, obj);
                            Self::push(fibre, *first);
                            context_pushed = true;
                        } else if first.is_string() {
                            let c_str =
                                unsafe { &*(first.as_string_ptr().unwrap() as *const String) };
                            let norm_c_str = normalize_name(c_str);
                            if let Some(id) = fibre.engine_state.class_map.get(&norm_c_str).map(|v| *v) {
                                call_class_id = id;
                            } else {
                                let classes = &fibre.engine_state.classes;
                                if let Some((i, _)) = classes
                                    .iter()
                                    .enumerate()
                                    .find(|(_, cls)| normalize_name(&cls.name) == norm_c_str)
                                {
                                    call_class_id = i;
                                }
                            }
                        }
                    }
                }

                if !context_pushed {
                    if let Some(closure_ptr) = loader.as_closure_ptr() {
                        let closure = unsafe { &*(closure_ptr as *const crate::types::closure::PhpClosure) };
                        call_class_id = closure.called_class_id;
                        if let Some(this_val) = closure.this_val {
                            Self::push(fibre, this_val);
                        } else {
                            Self::push(fibre, *loader);
                        }
                    } else {
                        Self::push(fibre, Value::new_int(0));
                    }
                }

                let trimmed_class_name = class_name.trim_start_matches('\\');
                let arg_val = Value::new_string_ptr(
                    fibre.arena.alloc_and_track(trimmed_class_name.to_string()) as *mut (),
                );
                Self::push(fibre, arg_val);

                fibre.frames[fibre.frame_count] = CallFrame {
                    function: func_ptr,
                    ip: 0,
                    stack_window,
                    called_class_id: call_class_id,
                    return_override: None, eval_parent_stack_window: None,
                    arity: 1,
                };
                fibre.frame_count += 1;
                let upvalues = if let Some(loader_closure) = loader.as_closure_ptr() {
                    let closure = unsafe { &*(loader_closure as *mut crate::types::closure::PhpClosure) };
                    closure.upvalues.as_slice()
                } else {
                    &[]
                };
                Self::pad_missing_arguments_and_locals(fibre, 1, unsafe { &*func_ptr.0 }, upvalues, false);
                fibre.autoload_frame_depths.push(fibre.frame_count - 1);

                pushed_autoloader = true;
                break;
            }
        }
        if fibre.engine_state.class_map.contains_key(&norm_class_name) {
            return Ok(true);
        }
        Ok(pushed_autoloader)
    }
    /// Wrap a native function in a one-shot bytecode thunk so it can be reached
    /// through the ordinary call opcodes (callable strings, `array_map`, etc.).
    ///
    /// The thunk declares the native's by-reference positions. Without them the
    /// argument-binding funnel would treat every parameter as by-value and
    /// deep-copy array arguments, so `sort($a)` would sort a copy and leave
    /// `$a` untouched.
    pub(crate) fn make_native_thunk(
        normalized: &str,
        arity: usize,
    ) -> crate::types::function::FunctionPtr {
        static NATIVE_THUNKS: std::sync::LazyLock<dashmap::DashMap<(String, usize), crate::types::function::FunctionPtr>> =
            std::sync::LazyLock::new(dashmap::DashMap::new);

        let key = (normalized.to_string(), arity);
        if let Some(thunk) = NATIVE_THUNKS.get(&key) {
            return *thunk;
        }

        let mut chunk = hyperion_bytecode::Chunk::new();
        for i in 1..=arity {
            // `GetLocal` derefs, which is what natives want for ordinary
            // arguments. A by-ref position must instead hand over the cell
            // itself, or an out-param that is still null (`preg_match`'s
            // `$m`) has no way to publish the array it fills back to the
            // caller. `MakeRefLocal` reuses the cell the binding funnel
            // already put in the slot; the ref-collapse loop in `CallNative`
            // then resolves it for the native.
            if hyperion_core::types::native_signatures::is_by_ref_param(normalized, i - 1) {
                chunk.write_opcode(hyperion_bytecode::Opcode::MakeRefLocal);
            } else {
                chunk.write_opcode(hyperion_bytecode::Opcode::GetLocal);
            }
            chunk.write_byte(i as u8);
        }
        chunk.write_opcode(hyperion_bytecode::Opcode::CallNative);
        chunk.write_byte(normalized.len() as u8);
        for b in normalized.bytes() {
            chunk.write_byte(b);
        }
        chunk.write_byte(arity as u8);
        chunk.write_opcode(hyperion_bytecode::Opcode::ReturnValue);

        let params = (0..arity)
            .map(|i| hyperion_compiler::compiler::CompiledParam {
                name: format!("arg{}", i),
                type_hint: None,
                has_default: false,
                default_value: None,
                by_ref: hyperion_core::types::native_signatures::is_by_ref_param(normalized, i),
                is_variadic: false,
            })
            .collect();

        let func = crate::types::function::PhpFunction::new_with_params(
            normalized.to_string(),
            arity,
            chunk,
            params,
            false,
            false,
            hyperion_parser::parser::ast::Visibility::Public,
            arity,
        );
        let ptr = crate::types::function::FunctionPtr(Box::into_raw(Box::new(func)));
        NATIVE_THUNKS.insert(key, ptr);
        ptr
    }

    pub(crate) fn make_native_thunk_forward(
        normalized: &str,
    ) -> crate::types::function::FunctionPtr {
        static NATIVE_FORWARD_THUNKS: std::sync::LazyLock<dashmap::DashMap<String, crate::types::function::FunctionPtr>> =
            std::sync::LazyLock::new(dashmap::DashMap::new);

        if let Some(thunk) = NATIVE_FORWARD_THUNKS.get(normalized) {
            return *thunk;
        }

        let mut chunk = hyperion_bytecode::Chunk::new();
        chunk.write_opcode(hyperion_bytecode::Opcode::CallNativeForward);
        chunk.write_byte(normalized.len() as u8);
        for b in normalized.bytes() {
            chunk.write_byte(b);
        }

        let func = crate::types::function::PhpFunction::new(
            normalized.to_string(),
            0,
            chunk,
        );
        let ptr = crate::types::function::FunctionPtr(Box::into_raw(Box::new(func)));
        NATIVE_FORWARD_THUNKS.insert(normalized.to_string(), ptr);
        ptr
    }

    pub(crate) fn default_val_to_value(
        engine_state: &std::sync::Arc<crate::fibre::GlobalEngineState>,
        def: &hyperion_compiler::compiler::DefaultValue,
        arena: &hyperion_core::gc::arena::GcArena,
        mut fibre: Option<&mut Fibre>,
    ) -> Value {
        match def {
            hyperion_compiler::compiler::DefaultValue::Int(val) => Value::new_int(*val as i32),
            hyperion_compiler::compiler::DefaultValue::Float(val) => Value::new_float(*val),
            hyperion_compiler::compiler::DefaultValue::String(val) => {
                let boxed = arena.alloc_and_track(val.clone());
                Value::new_string_ptr(boxed as *mut ())
            }
            hyperion_compiler::compiler::DefaultValue::Bool(val) => Value::new_bool(*val),
            hyperion_compiler::compiler::DefaultValue::Null => Value::null(),
            hyperion_compiler::compiler::DefaultValue::Constant(name) => {
                let constants = &engine_state.constants;
                resolve_constant(&constants, name).unwrap_or(Value::null())
            }
            hyperion_compiler::compiler::DefaultValue::ClassConstant(class_name, const_name) => {
                if const_name.eq_ignore_ascii_case("class") {
                    let boxed = arena.alloc_and_track(class_name.clone());
                    return Value::new_string_ptr(boxed as *mut ());
                }
                let full = format!("{}::{}", class_name, const_name);
                let constants = &engine_state.constants;
                if let Some(v) = resolve_constant(&constants, &full) {
                    v
                } else {
                    let default_statics = &engine_state.default_statics;
                    let key = (normalize_name(class_name), const_name.clone());
                    if let Some(v) = default_statics.get(&key) {
                        *v
                    } else {
                        let classes = &engine_state.classes;
                        let mut curr_name = Some(class_name.clone());
                        let mut found = None;
                        while let Some(cls_name) = curr_name {
                            let k = (normalize_name(&cls_name), const_name.clone());
                            if let Some(v) = default_statics.get(&k) {
                                found = Some(*v);
                                break;
                            }
                            let norm_name = normalize_name(&cls_name);
                            if let Some(id) = engine_state.class_map.get(&norm_name).map(|v| *v) {
                                if let Some(cls) = classes.get(&id) {
                                    curr_name = cls.extends.clone();
                                } else {
                                    break;
                                }
                            } else if let Some(cls) = classes.iter().find(|c| normalize_name(&c.name) == norm_name) {
                                curr_name = cls.extends.clone();
                            } else {
                                break;
                            }
                        }
                        if let Some(v) = found {
                            v
                        } else if let Some(ref mut f) = fibre {
                            f.trigger_autoload_sync(class_name);
                            let full = format!("{}::{}", class_name, const_name);
                            let constants = &f.engine_state.constants;
                            if let Some(v) = resolve_constant(&constants, &full) {
                                v
                            } else {
                                let default_statics = &f.engine_state.default_statics;
                                let key = (normalize_name(class_name), const_name.clone());
                                if let Some(v) = default_statics.get(&key) {
                                    *v
                                } else {
                                    let classes = &f.engine_state.classes;
                                    let mut curr_name = Some(class_name.clone());
                                    let mut f_found = None;
                                    while let Some(cls_name) = curr_name {
                                        let k = (normalize_name(&cls_name), const_name.clone());
                                        if let Some(v) = default_statics.get(&k) {
                                            f_found = Some(*v);
                                            break;
                                        }
                                        let norm_name = normalize_name(&cls_name);
                                        if let Some(id) = f.engine_state.class_map.get(&norm_name).map(|v| *v) {
                                            if let Some(cls) = classes.get(&id) {
                                                curr_name = cls.extends.clone();
                                            } else {
                                                break;
                                            }
                                        } else if let Some(cls) = classes.iter().find(|c| normalize_name(&c.name) == norm_name) {
                                            curr_name = cls.extends.clone();
                                        } else {
                                            break;
                                        }
                                    }
                                    f_found.unwrap_or(Value::null())
                                }
                            }
                        } else {
                            Value::null()
                        }
                    }
                }
            }
            hyperion_compiler::compiler::DefaultValue::Array(items) => {
                let mut arr = hyperion_core::types::array::PhpArray::new();
                for (k_opt, v_def) in items {
                    let v = Self::default_val_to_value(engine_state, v_def, arena, None);
                    match k_opt {
                        Some(hyperion_compiler::compiler::DefaultValue::String(s)) => {
                            let s_id = hyperion_core::types::string_table::intern_string(s);
                            arr.insert_string_id(s_id, v);
                        }

                        Some(hyperion_compiler::compiler::DefaultValue::Int(i)) => {
                            arr.insert_int(*i, v);
                        }
                        _ => {
                            let next_k = arr.next_free_int_key();
                            arr.insert_int(next_k, v);
                        }
                    }
                }
                let arr_ptr = arena.alloc_and_track(arr);
                Value::new_array_ptr(arr_ptr as *mut ())
            }
            hyperion_compiler::compiler::DefaultValue::New(class_name, args) => {
                if let Some(ref mut f) = fibre {
                    if !f.class_exists(class_name) {
                        f.trigger_autoload_sync(class_name);
                    }
                    let class_id = f.get_class_id(class_name).unwrap_or(0);
                    let obj = f.instantiate_class(class_id);
                    let obj_ptr = f.arena.alloc_and_track(obj);
                    let obj_val = Value::new_object_ptr(obj_ptr as *mut ());
                    
                    let arg_values: Vec<Value> = args.iter().map(|a| {
                        Self::default_val_to_value(engine_state, a, arena, None)
                    }).collect();

                    if Self::class_has_method(f, class_id, "__construct") {
                        let _ = Self::call_method_synchronously(f, obj_val, "__construct", arg_values);
                    }
                    obj_val
                } else {
                    let norm = normalize_name(class_name);
                    let class_id = engine_state.class_map.get(&norm).map(|v| *v).unwrap_or(0);
                    let class_name_opt = engine_state.classes.get(&class_id).map(|c| c.name.clone());
                    let obj = if let Some(ref name) = class_name_opt {
                        hyperion_core::types::object::PhpObject::new_with_name(class_id, name.clone())
                    } else {
                        hyperion_core::types::object::PhpObject::new(class_id)
                    };
                    let obj_ptr = arena.alloc_and_track(obj);
                    Value::new_object_ptr(obj_ptr as *mut ())
                }
            }
        }
    }


    fn pad_missing_arguments_and_locals(
        fibre: &mut Fibre,
        mut arity: usize,
        func: &crate::types::function::PhpFunction,
        upvalues: &[Value],
        variadic_already_packed: bool,
    ) {
        let window = fibre.frames[fibre.frame_count - 1].stack_window;
        let variadic_idx = func.params.iter().position(|p| p.is_variadic);

        if let Some(var_idx) = variadic_idx {
            if variadic_already_packed {
                for i in 0..var_idx {
                    let slot = window + 1 + i;
                    if slot >= fibre.stack_top {
                        break;
                    }
                    let by_ref = func.params.get(i).map(|p| p.by_ref).unwrap_or(false);
                    let raw = fibre.stack[slot];
                    if by_ref {
                        continue;
                    }
                    let plain = raw.deref();
                    let bound = if plain.is_array() {
                        Self::deep_copy_value(fibre, plain)
                    } else {
                        plain
                    };
                    fibre.stack[slot] = bound;
                }
                arity = var_idx + 1;
            } else {
                // 1. Bind arguments by value
                for i in 0..arity {
                    let slot = window + 1 + i;
                    if slot >= fibre.stack_top {
                        break;
                    }
                    let by_ref = func.params.get(i).map(|p| p.by_ref).unwrap_or(false);
                    let raw = fibre.stack[slot];
                    if by_ref {
                        continue;
                    }
                    let plain = raw.deref();
                    let bound = if plain.is_array() {
                        Self::deep_copy_value(fibre, plain)
                    } else {
                        plain
                    };
                    fibre.stack[slot] = bound;
                }

                // 2. Pad missing non-variadic parameters if arity < var_idx
                if arity < var_idx {
                    let engine_state = fibre.engine_state.clone();
                    for i in arity..var_idx {
                        let default_val = if let Some(param) = func.params.get(i) {
                            if let Some(ref def) = param.default_value {
                                let arena = fibre.arena.clone();
                                Self::default_val_to_value(&engine_state, def, &arena, Some(fibre))
                            } else {
                                Value::null()
                            }
                        } else {
                            Value::null()
                        };
                        Self::push(fibre, default_val);
                    }
                    arity = var_idx;
                }

                // 3. Pack variadic arguments into an array at slot `window + 1 + var_idx`
                let mut var_arr = hyperion_core::types::array::PhpArray::new();
                if arity > var_idx {
                    for i in var_idx..arity {
                        let slot = window + 1 + i;
                        if slot < fibre.stack_top {
                            var_arr.insert_int((i - var_idx) as i64, fibre.stack[slot]);
                        }
                    }
                }
                let arr_ref = fibre.arena.alloc_and_track(var_arr);
                let arr_val = Value::new_array_ptr(arr_ref as *mut () as *mut _);
                let var_slot = window + 1 + var_idx;
                if var_slot < fibre.stack.len() {
                    fibre.stack[var_slot] = arr_val;
                }
                arity = var_idx + 1;
            }
        } else {
            // Non-variadic function
            for i in 0..arity {
                let slot = window + 1 + i;
                if slot >= fibre.stack_top {
                    break;
                }
                let by_ref = func.params.get(i).map(|p| p.by_ref).unwrap_or(false);
                let raw = fibre.stack[slot];
                if by_ref {
                    continue;
                }
                let plain = raw.deref();
                let bound = if plain.is_array() {
                    Self::deep_copy_value(fibre, plain)
                } else {
                    plain
                };
                fibre.stack[slot] = bound;
            }

            let expected_arity = func.arity.max(func.params.len());
            if arity < expected_arity {
                let engine_state = fibre.engine_state.clone();
                for i in arity..expected_arity {
                    let default_val = if let Some(param) = func.params.get(i) {
                        if let Some(ref def) = param.default_value {
                            let arena = fibre.arena.clone();
                            Self::default_val_to_value(&engine_state, def, &arena, Some(fibre))
                        } else {
                            Value::null()
                        }
                    } else {
                        Value::null()
                    };
                    Self::push(fibre, default_val);
                }
                arity = expected_arity;
            }
        }

        // Fill remaining local slots up to num_locals with unassigned (or null)
        let num_locals = func.num_locals;
        let locals_start = window + 1 + arity;
        let locals_end = (window + 1 + arity).max(window + 1 + num_locals);
        for slot in locals_start..locals_end {
            if slot < fibre.stack.len() {
                fibre.stack[slot] = Value::null();
            }
        }
        fibre.stack_top = locals_end;

        // Propagate captured lexical variables (upvalues) from the closure to
        // the frame's local variable slots by copying into the slot assigned to
        // the variable name in the closure's chunk.
        if !upvalues.is_empty() {
            let num_params = func.params.len();
            for (idx, upval) in upvalues.iter().enumerate() {
                let slot = window + 1 + num_params + idx;
                if slot < fibre.stack_top {
                    fibre.stack[slot] = *upval;
                }
            }
        }
    }

    pub fn bind_call_arguments(
        engine_state: &std::sync::Arc<crate::fibre::GlobalEngineState>,
        func: &crate::types::function::PhpFunction,
        positional_args: &[Value],
        named_pairs: &[(Value, Value)],
        arena: &hyperion_core::gc::arena::GcArena,
    ) -> (Vec<Value>, bool) {
        if named_pairs.is_empty() || func.params.is_empty() {
            let mut res = positional_args.to_vec();
            for (_name, val) in named_pairs {
                res.push(*val);
            }
            return (res, false);
        }

        let variadic_idx = func.params.iter().position(|p| p.is_variadic);
        if let Some(var_idx) = variadic_idx {
            let mut bound: Vec<Option<Value>> = vec![None; var_idx + 1];
            let mut extra_named: Vec<(String, Value)> = Vec::new();

            // 1. Fill positional arguments for non-variadic parameters (0..var_idx)
            for (i, val) in positional_args.iter().enumerate() {
                if i < var_idx {
                    bound[i] = Some(*val);
                }
            }

            // 2. Fill named arguments
            for (name_val, val) in named_pairs {
                if let Some(s_ptr) = name_val.as_string_ptr() {
                    let name = unsafe { &*(s_ptr as *const String) };
                    if let Some(p_idx) = func.params[..var_idx].iter().position(|p| p.name == *name) {
                        bound[p_idx] = Some(*val);
                    } else {
                        extra_named.push((name.clone(), *val));
                    }
                } else {
                    extra_named.push((String::new(), *val));
                }
            }

            // 3. Fill defaults for missing non-variadic parameters (0..var_idx)
            for i in 0..var_idx {
                if bound[i].is_none() {
                    let def_val = if let Some(param) = func.params.get(i) {
                        if let Some(ref def) = param.default_value {
                            Self::default_val_to_value(engine_state, def, arena, None)
                        } else {
                            Value::null()
                        }
                    } else {
                        Value::null()
                    };
                    bound[i] = Some(def_val);
                }
            }

            // 4. Pack variadic array at bound[var_idx]
            let mut var_arr = hyperion_core::types::array::PhpArray::new();
            if positional_args.len() > var_idx {
                for i in var_idx..positional_args.len() {
                    var_arr.insert_int((i - var_idx) as i64, positional_args[i]);
                }
            }
            for (name, val) in extra_named {
                var_arr.insert_string_id(hyperion_core::types::string_table::intern_string(&name), val);
            }

            let arr_ref = arena.alloc_and_track(var_arr);
            let arr_val = Value::new_array_ptr(arr_ref as *mut () as *mut _);
            bound[var_idx] = Some(arr_val);

            let res: Vec<Value> = bound.into_iter().map(|v| v.unwrap()).collect();
            return (res, true);
        }

        let mut bound: Vec<Option<Value>> = vec![None; func.params.len()];
        let mut extra_positional: Vec<Value> = Vec::new();

        // 1. Fill positional arguments
        for (i, val) in positional_args.iter().enumerate() {
            if i < bound.len() {
                bound[i] = Some(*val);
            } else {
                extra_positional.push(*val);
            }
        }

        // 2. Fill named arguments
        for (name_val, val) in named_pairs {
            if let Some(s_ptr) = name_val.as_string_ptr() {
                let name = unsafe { &*(s_ptr as *const String) };
                if let Some(p_idx) = func.params.iter().position(|p| p.name == *name) {
                    bound[p_idx] = Some(*val);
                } else {
                    extra_positional.push(*val);
                }
            } else {
                extra_positional.push(*val);
            }
        }

        // 3. Fill defaults for missing parameters
        let mut result = Vec::with_capacity(bound.len() + extra_positional.len());
        for (i, opt_val) in bound.into_iter().enumerate() {
            if let Some(v) = opt_val {
                result.push(v);
            } else {
                let def_val = if let Some(param) = func.params.get(i) {
                    if let Some(ref def) = param.default_value {
                        Self::default_val_to_value(engine_state, def, arena, None)
                    } else {
                        Value::null()
                    }
                } else {
                    Value::null()
                };
                result.push(def_val);
            }
        }
        result.extend(extra_positional);
        (result, false)
    }

    #[inline(always)]
    fn read_byte(fibre: &mut Fibre) -> u8 {
        let frame = fibre.frames[fibre.frame_count - 1];
        let func = unsafe { &*frame.function.0 };
        let byte = func.chunk.code[frame.ip];
        fibre.frames[fibre.frame_count - 1].ip += 1;
        byte
    }

    pub fn find_missing_parent(
        classes: &[crate::types::class::PhpClass],
        class_id: usize,
    ) -> Option<String> {
        let mut current_id = class_id;
        loop {
            if current_id >= classes.len() {
                return None;
            }
            let c_ref = classes.get(current_id).unwrap();
            let php_class = &*c_ref;
            if let Some(parent_name) = &php_class.extends {
                if let Some(pid) = classes
                    .iter()
                    .position(|c| normalize_name(&c.name) == normalize_name(parent_name))
                {
                    current_id = pid;
                } else {
                    return Some(parent_name.to_string());
                }
            } else {
                return None;
            }
        }
    }

    #[inline(always)]
    fn read_short(fibre: &mut Fibre) -> u16 {
        let hi = Self::read_byte(fibre) as u16;
        let lo = Self::read_byte(fibre) as u16;
        (hi << 8) | lo
    }

    #[inline(always)]
    pub fn push(fibre: &mut Fibre, value: Value) {
        if fibre.stack_top >= fibre.stack.len() {
            fibre.stack.resize(fibre.stack.len().max(64) * 2, Value::null());
        }
        fibre.stack[fibre.stack_top] = value;
        fibre.stack_top += 1;
    }

    #[inline(always)]
    pub fn pop(fibre: &mut Fibre) -> Value {
        if fibre.stack_top == 0 {
            return Value::null();
        }
        fibre.stack_top -= 1;
        fibre.stack[fibre.stack_top]
    }

    #[inline(always)]
    pub fn resolve_obj_class_id(fibre: &mut Fibre, obj: &hyperion_core::types::object::PhpObject) -> usize {
        if let Some(ref name) = obj.class_name {
            if obj.class_id != 0 {
                if let Some(c) = fibre.engine_state.classes.get(&obj.class_id) {
                    if c.name.eq_ignore_ascii_case(name) {
                        return obj.class_id;
                    }
                }
            }
            if let Some(id) = fibre.get_class_id(name) {
                return id;
            }
            fibre.trigger_autoload_sync(name);
            if let Some(id) = fibre.get_class_id(name) {
                return id;
            }
        }
        obj.class_id
    }

    #[inline(always)]
    pub fn find_method_fast(
        fibre: &mut Fibre,
        class_id: usize,
        method_name: &str,
    ) -> Result<Option<crate::types::function::FunctionPtr>, String> {
        let name_hash = hash_method_name(method_name);
        let hash = ((class_id as u64) ^ name_hash.rotate_left(13)) as usize & METHOD_CACHE_MASK;
        if let Some(func_ptr) = FAST_METHOD_CACHE.with(|c| {
            let cache = c.borrow();
            let slot = cache[hash];
            if slot.class_id == class_id && slot.name_hash == name_hash && !slot.func_ptr.0.is_null() {
                Some(slot.func_ptr)
            } else {
                None
            }
        }) {
            return Ok(Some(func_ptr));
        }

        let res = Self::find_method_with_autoload(fibre, class_id, method_name)?;
        if let Some(func_ptr) = res {
            FAST_METHOD_CACHE.with(|c| {
                let mut cache = c.borrow_mut();
                cache[hash] = MethodCacheSlot {
                    class_id,
                    name_hash,
                    func_ptr,
                };
            });
        }
        Ok(res)
    }

    pub fn find_method_with_autoload(
        fibre: &mut Fibre,
        class_id: usize,
        method_name: &str,
    ) -> Result<Option<crate::types::function::FunctionPtr>, String> {
        let mut attempts = 0;
        loop {
            let res = Self::find_method(&fibre.engine_state, class_id, method_name);
            match res {
                Ok(found) => return Ok(found),
                Err(missing_parent) => {
                    attempts += 1;
                    if attempts > 10 {
                        return Ok(None);
                    }
                    fibre.trigger_autoload_sync(&missing_parent);
                    let norm_p = normalize_name(&missing_parent);
                    if !fibre.engine_state.class_map.contains_key(&norm_p) && fibre.engine_state.classes.iter().find(|c| normalize_name(&c.value().name) == norm_p).is_none() {
                        return Ok(None);
                    }
                }
            }
        }
    }

    pub fn find_method(
        engine_state: &crate::fibre::GlobalEngineState,
        class_id: usize,
        method_name: &str,
    ) -> Result<Option<crate::types::function::FunctionPtr>, String> {
        let norm_method = method_name.to_ascii_lowercase();
        if let Some(cached) = METHOD_L1_CACHE.with(|c| c.borrow().get(&(class_id, norm_method.clone())).copied()) {
            return Ok(cached);
        }

        let classes = &engine_state.classes;
        let mut current_id = Some(class_id);

        while let Some(id) = current_id {
            let (method, parent_name) = if let Some(c) = classes.get(&id) {
                let m = c.methods.get(method_name)
                    .or_else(|| {
                        let norm = normalize_name(method_name);
                        c.methods.get(&norm).or_else(|| {
                            c.methods.iter().find(|(k, _)| k.eq_ignore_ascii_case(method_name)).map(|(_, v)| v)
                        })
                    })
                    .copied();
                (m, c.extends.clone())
            } else {
                break;
            };

            if let Some(m) = method {
                METHOD_L1_CACHE.with(|c| c.borrow_mut().insert((class_id, norm_method), Some(m)));
                return Ok(Some(m));
            }

            if let Some(parent_name) = parent_name {
                let norm_parent = normalize_name(&parent_name);
                let parent_key = engine_state.class_map.get(&norm_parent).map(|v| *v)
                    .or_else(|| classes.iter().find(|c| normalize_name(&c.value().name) == norm_parent).map(|c| *c.key()));
                if parent_key.is_none() {
                    return Err(parent_name);
                }
                current_id = parent_key;
            } else {
                current_id = None;
            }
        }
        METHOD_L1_CACHE.with(|c| c.borrow_mut().insert((class_id, norm_method), None));
        Ok(None)
    }


    pub fn is_method_accessible(
        fibre: &Fibre,
        target_class_id: usize,
        func: &crate::types::function::PhpFunction,
        caller_frame_idx: usize,
    ) -> bool {
        match func.visibility {
            hyperion_parser::parser::ast::Visibility::Public => true,
            hyperion_parser::parser::ast::Visibility::Protected => {
                if caller_frame_idx >= fibre.frame_count {
                    return false;
                }
                let caller_frame = &fibre.frames[caller_frame_idx];
                if caller_frame.called_class_id == 0 {
                    return false;
                }
                let caller_class_id = caller_frame.called_class_id;
                let classes = &fibre.engine_state.classes;
                let is_sub = |child_id: usize, parent_id: usize| -> bool {
                    let mut curr_id = Some(child_id);
                    while let Some(id) = curr_id {
                        if id == parent_id {
                            return true;
                        }
                        let parent_name = classes.get(&id).and_then(|c| c.extends.clone());
                        curr_id = parent_name.and_then(|pn| {
                            let norm_p = normalize_name(&pn);
                            fibre.engine_state.class_map.get(&norm_p).map(|v| *v)
                                .or_else(|| classes.iter().find(|c| normalize_name(&c.value().name) == norm_p).map(|c| *c.key()))
                        });
                    }
                    false
                };

                let declaring_class_id = {
                    let mut curr_id = Some(target_class_id);
                    let mut found_id = target_class_id;
                    while let Some(id) = curr_id {
                        let (has_method, parent_name) = if let Some(c) = classes.get(&id) {
                            (c.methods.values().any(|m| m.0 == func as *const _), c.extends.clone())
                        } else {
                            (false, None)
                        };
                        if has_method {
                            found_id = id;
                            break;
                        }
                        curr_id = parent_name.and_then(|p_name| {
                            let norm_p = normalize_name(&p_name);
                            fibre.engine_state.class_map.get(&norm_p).map(|v| *v)
                                .or_else(|| classes.iter().find(|c| normalize_name(&c.value().name) == norm_p).map(|c| *c.key()))
                        });
                    }
                    found_id
                };

                caller_class_id == declaring_class_id
                    || caller_class_id == target_class_id
                    || is_sub(caller_class_id, declaring_class_id)
                    || is_sub(declaring_class_id, caller_class_id)
                    || is_sub(caller_class_id, target_class_id)
                    || is_sub(target_class_id, caller_class_id)
            }
            hyperion_parser::parser::ast::Visibility::Private => {
                if caller_frame_idx >= fibre.frame_count {
                    return false;
                }
                let caller_frame = &fibre.frames[caller_frame_idx];
                if caller_frame.called_class_id == 0 {
                    return false;
                }
                let caller_class_id = caller_frame.called_class_id;
                let classes = &fibre.engine_state.classes;

                let is_sub = |child_id: usize, parent_id: usize| -> bool {
                    let mut curr_id = Some(child_id);
                    while let Some(id) = curr_id {
                        if id == parent_id {
                            return true;
                        }
                        let parent_name = classes.get(&id).and_then(|c| c.extends.clone());
                        curr_id = parent_name.and_then(|pn| {
                            let norm_p = normalize_name(&pn);
                            fibre.engine_state.class_map.get(&norm_p).map(|v| *v)
                                .or_else(|| classes.iter().find(|c| normalize_name(&c.value().name) == norm_p).map(|c| *c.key()))
                        });
                    }
                    false
                };

                let declaring_class_id = {
                    let mut curr_id = Some(target_class_id);
                    let mut found_id = target_class_id;
                    while let Some(id) = curr_id {
                        let (has_method, parent_name) = if let Some(c) = classes.get(&id) {
                            (c.methods.values().any(|m| m.0 == func as *const _), c.extends.clone())
                        } else {
                            (false, None)
                        };
                        if has_method {
                            found_id = id;
                            break;
                        }
                        curr_id = parent_name.and_then(|p_name| {
                            let norm_p = normalize_name(&p_name);
                            fibre.engine_state.class_map.get(&norm_p).map(|v| *v)
                                .or_else(|| classes.iter().find(|c| normalize_name(&c.value().name) == norm_p).map(|c| *c.key()))
                        });
                    }
                    found_id
                };

                caller_class_id == declaring_class_id
                    || caller_class_id == target_class_id
                    || is_sub(caller_class_id, declaring_class_id)
                    || is_sub(declaring_class_id, caller_class_id)
                    || is_sub(caller_class_id, target_class_id)
                    || is_sub(target_class_id, caller_class_id)
            }
        }
    }

    pub fn get_current_called_class_id(fibre: &Fibre) -> usize {
        if fibre.frame_count == 0 {
            return 0;
        }
        let frame = &fibre.frames[fibre.frame_count - 1];
        if frame.called_class_id != 0 {
            return frame.called_class_id;
        }
        if frame.stack_window < fibre.stack_top {
            let this_val = fibre.stack[frame.stack_window];
            if let Some(obj_ptr) = this_val.as_object_ptr() {
                let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
                if obj.class_id != 0 {
                    return obj.class_id;
                }
            } else if let Some(c_ptr) = this_val.as_closure_ptr() {
                let closure = unsafe { &*(c_ptr as *const crate::types::closure::PhpClosure) };
                if closure.called_class_id != 0 {
                    return closure.called_class_id;
                }
                if let Some(bound_this) = closure.this_val {
                    if let Some(b_ptr) = bound_this.as_object_ptr() {
                        let obj = unsafe { &*(b_ptr as *const hyperion_core::types::object::PhpObject) };
                        if obj.class_id != 0 {
                            return obj.class_id;
                        }
                    }
                }
            }
        }
        for i in (0..fibre.frame_count - 1).rev() {
            let f = &fibre.frames[i];
            if f.called_class_id != 0 {
                return f.called_class_id;
            }
            if f.stack_window < fibre.stack_top {
                let this_val = fibre.stack[f.stack_window];
                if let Some(obj_ptr) = this_val.as_object_ptr() {
                    let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
                    if obj.class_id != 0 {
                        return obj.class_id;
                    }
                }
            }
        }
        let func_ptr = frame.function;
        let classes = &fibre.engine_state.classes;
        if let Some(c) = classes.iter().find(|c| c.value().methods.values().any(|m| m.0 == func_ptr.0)) {
            return *c.key();
        }
        0
    }

    pub fn resolve_dynamic_class_name(fibre: &Fibre, class_name: &str) -> String {
        let class_name = if class_name.starts_with("self\0") {
            &class_name[5..]
        } else if class_name.starts_with("parent\0") {
            &class_name[7..]
        } else {
            class_name
        };
        let var_candidate = if let Some(pos) = class_name.rfind('$') {
            Some(&class_name[pos + 1..])
        } else {
            None
        };
        if let Some(var_name) = var_candidate {
            if fibre.frame_count > 0 {
                let frame = &fibre.frames[fibre.frame_count - 1];
                let func = unsafe { &*frame.function.0 };
                if let Some(pos) = func.chunk.local_names.iter().position(|n| n == var_name) {
                    let val = fibre.stack[frame.stack_window + pos].deref();
                    if val.is_object() {
                        if let Some(ptr) = val.as_object_ptr() {
                            let obj = unsafe { &*(ptr as *const hyperion_core::types::object::PhpObject) };
                            if let Some(ref name) = obj.class_name {
                                return name.clone();
                            } else {
                                let classes = &fibre.engine_state.classes;
                                if let Some(c) = classes.get(&obj.class_id) {
                                    return c.name.clone();
                                }
                            }
                        }
                    } else if let Some(s_ptr) = val.as_string_ptr() {
                        return unsafe { &*(s_ptr as *const String) }.clone();
                    }
                }
            }
        }
        if class_name.eq_ignore_ascii_case("self") {
            let current_func_ptr = fibre.frames[fibre.frame_count - 1].function;
            
            let classes = &fibre.engine_state.classes;
            if let Some(c) = classes
                .iter()
                .find(|c| c.value().methods.values().any(|m| m.0 == current_func_ptr.0))
            {
                return c.value().name.clone();
            } else {
                let class_id = fibre.frames[fibre.frame_count - 1].called_class_id;
                if let Some(c) = classes.get(&class_id) {
                    return c.name.clone();
                }
            }
        } else if class_name.eq_ignore_ascii_case("parent") {
            let classes = &fibre.engine_state.classes;
            
            // Find the active method in the call stack (skipping closures)
            let mut active_func_ptr = fibre.frames[fibre.frame_count - 1].function;
            let mut called_class_id = fibre.frames[fibre.frame_count - 1].called_class_id;
            for i in (0..fibre.frame_count).rev() {
                let f_ptr = fibre.frames[i].function;
                let f_name = unsafe { &*f_ptr.0 }.name.as_str();
                if !f_name.starts_with("closure#") {
                    active_func_ptr = f_ptr;
                    if fibre.frames[i].called_class_id > 0 {
                        called_class_id = fibre.frames[i].called_class_id;
                    }
                    break;
                }
            }

            // Find which class in the inheritance chain of called_class_id declared active_func_ptr
            let mut search_id = if let Some(_) = classes.get(&called_class_id) {
                Some(called_class_id)
            } else {
                classes.iter().find(|c| c.value().methods.values().any(|m| m.0 == active_func_ptr.0)).map(|c| *c.key())
            };
            let mut declaring_class: Option<crate::types::class::PhpClass> = None;

            while let Some(id) = search_id {
                let c = classes.get(&id).unwrap().clone();
                if c.methods.values().any(|m| m.0 == active_func_ptr.0) {
                    declaring_class = Some(c.clone());
                }
                search_id = c.extends.as_ref().and_then(|p_name| {
                    let norm = normalize_name(p_name);
                    fibre.engine_state.class_map.get(&norm).map(|v| *v).or_else(|| {
                        classes.iter().find(|cl| normalize_name(&cl.value().name) == norm).map(|c| *c.key())
                    })
                });
            }

            if let Some(c) = declaring_class {
                if let Some(ref parent) = c.extends {
                    return parent.clone();
                }
            }

            // Fallback: any class containing the method
            if let Some(c) = classes.iter().find(|c| c.value().methods.values().any(|m| m.0 == active_func_ptr.0)) {
                if let Some(ref parent) = c.value().extends {
                    return parent.clone();
                }
            }
        } else if class_name.eq_ignore_ascii_case("static") {
            let class_id = Self::get_current_called_class_id(fibre);
            
            let classes = &fibre.engine_state.classes;
            if let Some(c) = classes.get(&class_id) {
                return c.name.clone();
            }
        }
        let mut s = class_name.to_string();
        if s.starts_with('\\') {
            s.remove(0);
        }
        s
    }

    /// PHP 8 loose equality (`==`) semantics.
    ///
    /// Key PHP 8 rules:
    /// - null == false == 0 == "" == "0" are all equal (PHP 7 compat preserved here)
    /// - PHP 8 BREAKING CHANGE: `0 == "foo"` → **false** (not true as in PHP 7).
    ///   Numbers are not cast from non-numeric strings. Only numeric strings (e.g. "42") are
    ///   compared numerically to integers/floats.
    /// - bool wins: if either side is bool, both are cast to bool.
    /// - null == null, null == false, null == 0, null == "" → true.
    /// - string vs string: try numeric comparison first, else lexicographic.
    /// - int vs float: promote int to float.
    #[inline]
    fn php8_loose_equals(_fibre: &Fibre, a: &Value, b: &Value) -> bool {
        // --- both same type fast path ---
        if a.is_int() && b.is_int() {
            return a.as_int() == b.as_int();
        }
        if a.is_float() && b.is_float() {
            return (a.as_float().unwrap() - b.as_float().unwrap()).abs() < f64::EPSILON;
        }
        if a.is_null() && b.is_null() {
            return true;
        }
        if a.is_bool() && b.is_bool() {
            return a.as_bool() == b.as_bool();
        }
        if a.is_string() && b.is_string() {
            let ls = unsafe { &*(a.as_string_ptr().unwrap() as *const String) };
            let rs = unsafe { &*(b.as_string_ptr().unwrap() as *const String) };
            // Both numeric strings → numeric compare
            if let (Ok(lf), Ok(rf)) = (ls.trim().parse::<f64>(), rs.trim().parse::<f64>()) {
                return (lf - rf).abs() < f64::EPSILON;
            }
            return ls == rs;
        }

        // --- null comparisons ---
        // null == false/0/"" → true; null == non-null-non-false-non-zero → false
        if a.is_null() {
            return Self::php8_null_eq(b);
        }
        if b.is_null() {
            return Self::php8_null_eq(a);
        }

        // --- bool: if either is bool, cast both to bool ---
        if a.is_bool() || b.is_bool() {
            return is_truthy(a) == is_truthy(b);
        }

        // --- int vs float ---
        if a.is_int() && b.is_float() {
            return (a.as_int().unwrap() as f64 - b.as_float().unwrap()).abs() < f64::EPSILON;
        }
        if a.is_float() && b.is_int() {
            return (a.as_float().unwrap() - b.as_int().unwrap() as f64).abs() < f64::EPSILON;
        }

        // --- int/float vs string (PHP 8: only numeric strings compare numerically) ---
        if (a.is_int() || a.is_float()) && b.is_string() {
            let bs = unsafe { &*(b.as_string_ptr().unwrap() as *const String) };
            if let Ok(bf) = bs.trim().parse::<f64>() {
                let af = if a.is_int() {
                    a.as_int().unwrap() as f64
                } else {
                    a.as_float().unwrap()
                };
                return (af - bf).abs() < f64::EPSILON;
            }
            // PHP 8: non-numeric string compared to number → false
            return false;
        }
        if a.is_string() && (b.is_int() || b.is_float()) {
            let as_str = unsafe { &*(a.as_string_ptr().unwrap() as *const String) };
            if let Ok(af) = as_str.trim().parse::<f64>() {
                let bf = if b.is_int() {
                    b.as_int().unwrap() as f64
                } else {
                    b.as_float().unwrap()
                };
                return (af - bf).abs() < f64::EPSILON;
            }
            // PHP 8: non-numeric string → false
            return false;
        }

        // --- objects ---
        // `==` is same-class-and-loosely-equal-properties; `===` (elsewhere) is
        // identity. An identical handle short-circuits either way.
        if a.is_object() && b.is_object() {
            if a.as_object_ptr() == b.as_object_ptr() {
                return true;
            }
            return Self::php_object_loose_equals(_fibre, a, b, 0);
        }

        // --- arrays: same pairs, order irrelevant ---
        if a.is_array() && b.is_array() {
            return Self::php_array_equals(_fibre, a, b, 0);
        }

        false
    }

    /// Depth cap for recursive comparison. PHP raises a fatal error on a
    /// self-referential compare; we bail out instead of overflowing the stack.
    const MAX_COMPARE_DEPTH: usize = 64;

    /// PHP's ordering relation, shared by `<`, `>`, `<=`, `>=` and `<=>`.
    ///
    /// PHP 8 rules: arrays outrank scalars and compare by size then per-key;
    /// bool/null comparisons cast both sides to bool; a number against a
    /// *numeric* string compares numerically, but against a non-numeric string
    /// the number is cast to string (the PHP 8 change). Objects are not
    /// meaningfully ordered — they compare equal here rather than erroring.
    pub fn php_compare(a: &Value, b: &Value) -> std::cmp::Ordering {
        Self::php_compare_depth(a, b, 0)
    }

    fn php_compare_depth(a: &Value, b: &Value, depth: usize) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        if depth > Self::MAX_COMPARE_DEPTH {
            return Ordering::Equal;
        }
        let a = a.deref();
        let b = b.deref();

        // --- arrays ---
        if a.is_array() && b.is_array() {
            let (Some(ap), Some(bp)) = (a.as_array_ptr(), b.as_array_ptr()) else {
                return Ordering::Equal;
            };
            if ap == bp {
                return Ordering::Equal;
            }
            let (aa, ba) = unsafe {
                (
                    &*(ap as *const hyperion_core::types::array::PhpArray),
                    &*(bp as *const hyperion_core::types::array::PhpArray),
                )
            };
            // Fewer elements is unconditionally "less" in PHP.
            match aa.elements.len().cmp(&ba.elements.len()) {
                Ordering::Equal => {}
                other => return other,
            }
            for (ak, av) in aa.elements.iter() {
                // A key missing on the right makes the left side greater.
                let Some(bv) = ba.elements.get(ak) else {
                    return Ordering::Greater;
                };
                match Self::php_compare_depth(av, bv, depth + 1) {
                    Ordering::Equal => {}
                    other => return other,
                }
            }
            return Ordering::Equal;
        }
        if a.is_array() {
            return Ordering::Greater;
        }
        if b.is_array() {
            return Ordering::Less;
        }

        // --- bool / null: compare truthiness ---
        // `null` against a string is the one exception: it compares as "".
        if a.is_null() && b.is_string() {
            let bs = unsafe { &*(b.as_string_ptr().unwrap() as *const String) };
            return "".cmp(bs.as_str());
        }
        if b.is_null() && a.is_string() {
            let as_ = unsafe { &*(a.as_string_ptr().unwrap() as *const String) };
            return as_.as_str().cmp("");
        }
        if a.is_bool() || b.is_bool() || a.is_null() || b.is_null() {
            return is_truthy(&a).cmp(&is_truthy(&b));
        }

        // --- strings ---
        if a.is_string() && b.is_string() {
            let as_ = unsafe { &*(a.as_string_ptr().unwrap() as *const String) };
            let bs = unsafe { &*(b.as_string_ptr().unwrap() as *const String) };
            if let (Ok(af), Ok(bf)) = (as_.trim().parse::<f64>(), bs.trim().parse::<f64>()) {
                return af.partial_cmp(&bf).unwrap_or(Ordering::Equal);
            }
            return as_.as_str().cmp(bs.as_str());
        }

        // --- number vs string ---
        let num_of = |v: &Value| -> Option<f64> {
            v.as_int()
                .map(|i| i as f64)
                .or_else(|| v.as_float())
        };
        if let (Some(af), true) = (num_of(&a), b.is_string()) {
            let bs = unsafe { &*(b.as_string_ptr().unwrap() as *const String) };
            if let Ok(bf) = bs.trim().parse::<f64>() {
                return af.partial_cmp(&bf).unwrap_or(Ordering::Equal);
            }
            // PHP 8: non-numeric string → compare as strings.
            return Self::number_to_string(&a).cmp(&bs.to_string());
        }
        if let (true, Some(bf)) = (a.is_string(), num_of(&b)) {
            let as_ = unsafe { &*(a.as_string_ptr().unwrap() as *const String) };
            if let Ok(af) = as_.trim().parse::<f64>() {
                return af.partial_cmp(&bf).unwrap_or(Ordering::Equal);
            }
            return as_.to_string().cmp(&Self::number_to_string(&b));
        }

        // --- numbers ---
        if let (Some(af), Some(bf)) = (num_of(&a), num_of(&b)) {
            return af.partial_cmp(&bf).unwrap_or(Ordering::Equal);
        }

        // Objects and anything else: no meaningful order.
        Ordering::Equal
    }

    fn number_to_string(v: &Value) -> String {
        if let Some(i) = v.as_int() {
            i.to_string()
        } else if let Some(f) = v.as_float() {
            f.to_string()
        } else {
            String::new()
        }
    }

    /// Array comparison for `==`: same pairs, order irrelevant. (`===` is
    /// order-sensitive and lives on `Value::strict_equals`.)
    fn php_array_equals(fibre: &Fibre, a: &Value, b: &Value, depth: usize) -> bool {
        if depth > Self::MAX_COMPARE_DEPTH {
            return false;
        }
        let (Some(ap), Some(bp)) = (a.as_array_ptr(), b.as_array_ptr()) else {
            return false;
        };
        if ap == bp {
            return true;
        }
        let (aa, ba) = unsafe {
            (
                &*(ap as *const hyperion_core::types::array::PhpArray),
                &*(bp as *const hyperion_core::types::array::PhpArray),
            )
        };
        if aa.elements.len() != ba.elements.len() {
            return false;
        }
        for (ak, av) in aa.elements.iter() {
            let Some(bv) = ba.elements.get(ak) else {
                return false;
            };
            let eq = if av.is_array() && bv.is_array() {
                Self::php_array_equals(fibre, av, bv, depth + 1)
            } else {
                Self::php8_loose_equals(fibre, av, bv)
            };
            if !eq {
                return false;
            }
        }
        true
    }

    /// `$a == $b` for two distinct object handles: same class, and every
    /// property loosely equal.
    fn php_object_loose_equals(fibre: &Fibre, a: &Value, b: &Value, depth: usize) -> bool {
        if depth > Self::MAX_COMPARE_DEPTH {
            return false;
        }
        let (Some(ap), Some(bp)) = (a.as_object_ptr(), b.as_object_ptr()) else {
            return false;
        };
        let (ao, bo) = unsafe {
            (
                &*(ap as *const hyperion_core::types::object::PhpObject),
                &*(bp as *const hyperion_core::types::object::PhpObject),
            )
        };
        if ao.class_id != bo.class_id || ao.properties.len() != bo.properties.len() {
            return false;
        }
        for (name, av) in ao.properties.iter() {
            let Some(bv) = bo.properties.get(name) else {
                return false;
            };
            let eq = if av.is_array() && bv.is_array() {
                Self::php_array_equals(fibre, av, bv, depth + 1)
            } else if av.is_object() && bv.is_object() {
                av.as_object_ptr() == bv.as_object_ptr()
                    || Self::php_object_loose_equals(fibre, av, bv, depth + 1)
            } else {
                Self::php8_loose_equals(fibre, av, bv)
            };
            if !eq {
                return false;
            }
        }
        true
    }

    /// `$a === $b`. Identical types and contents; arrays recurse in order and
    /// objects must be the very same instance. The comparison needs no VM
    /// state, so it lives on `Value` — this is the VM-side name for it.
    #[inline]
    pub fn php_strict_equals(_fibre: &Fibre, a: &Value, b: &Value) -> bool {
        a.strict_equals(b)
    }

    #[inline]
    fn php8_null_eq(other: &Value) -> bool {
        if other.is_null() {
            return true;
        }
        if other.is_bool() {
            return other.as_bool() == Some(false);
        }
        if let Some(i) = other.as_int() {
            return i == 0;
        }
        if let Some(f) = other.as_float() {
            return f == 0.0;
        }
        if let Some(ptr) = other.as_string_ptr() {
            let s = unsafe { &*(ptr as *const String) };
            return s.is_empty();
        }
        if let Some(ptr) = other.as_array_ptr() {
            let arr = unsafe { &*(ptr as *const hyperion_core::types::array::PhpArray) };
            return arr.len() == 0;
        }
        false
    }

    /// PHP array-by-value semantics: copy an array into a fresh GC allocation.
    ///
    /// Objects are intentionally NOT deep-copied — PHP passes objects by
    /// handle (reference-by-default), so they share their pointer.
    /// Scalars (int/float/bool/null/string) are NaN-box values and are
    /// trivially copied by the `Value` copy semantics already.
    ///
    /// This function only needs to allocate a new `PhpArray` for array
    /// values; nested arrays are deep-copied recursively.
    /// Convert a Value used as an array subscript into an `ArrayKey`, matching
    /// `Opcode::ArraySet`: numeric strings normalise to int keys, everything
    /// else interns. Non-scalars fall back to int 0, as PHP does for null-ish
    /// keys it cannot coerce.
    fn value_to_array_key(key: Value) -> hyperion_core::types::array::ArrayKey {
        use hyperion_core::types::array::ArrayKey;
        if let Some(key_str) = key.as_string_ptr() {
            let key_s = unsafe { &*(key_str as *const String) };
            match key_s.parse::<i64>() {
                Ok(i) => ArrayKey::Int(i),
                Err(_) => ArrayKey::StringId(
                    hyperion_core::types::string_table::intern_string(key_s),
                ),
            }
        } else if let Some(i) = key.as_int() {
            ArrayKey::Int(i as i64)
        } else if let Some(b) = key.as_bool() {
            ArrayKey::Int(b as i64)
        } else {
            ArrayKey::Int(0)
        }
    }

    /// Resolve a container Value to a mutable `PhpArray` that is safe to write
    /// through, autovivifying when it is null.
    ///
    /// The container arrives as a *ref* whenever it came from `compile_make_ref`,
    /// which is what lets `$cur = &$arr['a']['b']` build missing levels: writing
    /// the fresh array back into the ref cell makes the parent observe it. A
    /// plain (non-ref) null container has nowhere to store the new array, so it
    /// cannot be autovivified and is reported as an error.
    fn array_ptr_for_write(
        fibre: &mut Fibre,
        container: Value,
    ) -> Result<*mut hyperion_core::types::array::PhpArray, String> {
        let inner = container.deref();
        if let Some(ptr) = inner.as_array_ptr() {
            return Ok(ptr as *mut hyperion_core::types::array::PhpArray);
        }
        if inner.is_null() {
            let arr = hyperion_core::types::array::PhpArray::new();
            let arr_ptr = fibre.arena.alloc_and_track(arr);
            if let Some(cell) = container.as_ref_ptr() {
                let v = Value::new_array_ptr(arr_ptr as *mut ());
                unsafe {
                    (*(cell as *mut hyperion_core::types::reference::PhpRef)).set(v);
                }
                return Ok(arr_ptr);
            }
            return Err("Cannot auto-vivify array on a non-reference value".to_string());
        }
        Err(format!(
            "Cannot use a scalar of type {:?} as an array",
            inner.get_type()
        ))
    }

    fn deep_copy_value(fibre: &mut Fibre, val: Value) -> Value {
        Self::deep_copy_value_depth(fibre, val, 0)
    }

    fn deep_copy_value_depth(fibre: &mut Fibre, val: Value, depth: usize) -> Value {
        if depth > 32 {
            return val;
        }
        if let Some(arr_ptr) = val.as_array_ptr() {
            let src = unsafe { &*(arr_ptr as *const hyperion_core::types::array::PhpArray) };
            let mut new_arr = hyperion_core::types::array::PhpArray::new();
            for (k, v) in src.elements.iter() {
                let new_v = if v.is_array() {
                    Self::deep_copy_value_depth(fibre, *v, depth + 1)
                } else {
                    *v
                };
                match k {
                    hyperion_core::types::array::ArrayKey::Int(i) => new_arr.insert_int(*i, new_v),
                    hyperion_core::types::array::ArrayKey::StringId(s) => new_arr.insert_string_id(*s, new_v),
                }
            }
            let new_ptr = fibre.arena.alloc_and_track(new_arr) as *mut ();
            Value::new_array_ptr(new_ptr)
        } else {
            val
        }
    }

    pub fn call_callable_synchronously(
        fibre: &mut Fibre,
        callable: Value,
        args: Vec<Value>,
    ) -> Result<Value, String> {
        static CALL_WRAPPERS: std::sync::LazyLock<[crate::types::function::FunctionPtr; 16]> = std::sync::LazyLock::new(|| {
            std::array::from_fn(|arity| {
                let mut chunk = hyperion_bytecode::Chunk::new();
                chunk.write_opcode(hyperion_bytecode::Opcode::Call);
                chunk.write_byte(arity as u8);
                chunk.write_opcode(hyperion_bytecode::Opcode::ReturnValue);
                let func = Box::new(crate::types::function::PhpFunction::new(
                    "__internal_call_wrapper".to_string(),
                    0,
                    chunk,
                ));
                crate::types::function::FunctionPtr(Box::into_raw(func))
            })
        });

        let prev_stack_top = fibre.stack_top;
        let prev_frame_count = fibre.frame_count;

        let stack_window = fibre.stack_top;
        Self::push(fibre, callable);
        for arg in &args {
            Self::push(fibre, *arg);
        }

        let (func_ptr, is_static_wrapper) = if args.len() < 16 {
            (CALL_WRAPPERS[args.len()], true)
        } else {
            let mut chunk = hyperion_bytecode::Chunk::new();
            chunk.write_opcode(hyperion_bytecode::Opcode::Call);
            chunk.write_byte(args.len() as u8);
            chunk.write_opcode(hyperion_bytecode::Opcode::ReturnValue);

            let func = crate::types::function::PhpFunction::new(
                "__internal_call_wrapper".to_string(),
                0,
                chunk,
            );
            (crate::types::function::FunctionPtr(Box::into_raw(Box::new(func))), false)
        };

        if fibre.frame_count >= fibre.frames.len() {
            return Err("Stack overflow: max call depth exceeded".to_string());
        }

        fibre.frames[fibre.frame_count] = CallFrame {
            function: func_ptr,
            ip: 0,
            stack_window,
            called_class_id: 0,
            return_override: None, eval_parent_stack_window: None,
            arity: 0,
        };
        fibre.frame_count += 1;

        let old_stop = fibre.stop_frame_count;
        fibre.stop_frame_count = Some(prev_frame_count);

        let res = Self::drive_fibre_synchronously(fibre);

        fibre.stop_frame_count = old_stop;

        // Clean up the temporary function if dynamically allocated
        if !is_static_wrapper {
            unsafe {
                let _ = Box::from_raw(func_ptr.0 as *mut crate::types::function::PhpFunction);
            }
        }

        match res {
            ExecutionResult::Finished => {
                let ret_val = Self::pop(fibre);
                fibre.stack_top = prev_stack_top;
                Ok(ret_val)
            }
            ExecutionResult::Error(err) => {
                fibre.stack_top = prev_stack_top;
                fibre.frame_count = prev_frame_count;
                Err(err)
            }
            ExecutionResult::Yielded(_) => {
                fibre.stack_top = prev_stack_top;
                fibre.frame_count = prev_frame_count;
                Err("Yielded during synchronous call for unsupported reason".to_string())
            }
            ExecutionResult::Preempted => {
                fibre.stack_top = prev_stack_top;
                fibre.frame_count = prev_frame_count;
                Err("Preempted during synchronous call".to_string())
            }
            ExecutionResult::UncaughtException(err) => {
                fibre.stack_top = prev_stack_top;
                fibre.frame_count = prev_frame_count;
                Err(format!("Uncaught exception: {}", err))
            }
        }
    }

    pub fn drive_fibre_synchronously(fibre: &mut Fibre) -> ExecutionResult {
        let mut res = Self::run_fibre(fibre);
        while matches!(
            res,
            ExecutionResult::Yielded(YieldReason::AsyncInclude(_))
                | ExecutionResult::Yielded(YieldReason::Database(_))
                | ExecutionResult::Yielded(YieldReason::OutputFlush)
                | ExecutionResult::Preempted
        ) {
            match res {
                ExecutionResult::Yielded(YieldReason::AsyncInclude(ref path)) => {
                    if let Err(e) = fibre.engine_state.compile_and_load_script(path) {
                        return ExecutionResult::Error(format!("Fatal error: Failed to require '{}': {}", path, e));
                    }
                    fibre.resume();
                }
                ExecutionResult::Yielded(YieldReason::OutputFlush) => {
                    fibre.resume();
                }
                ExecutionResult::Yielded(YieldReason::Database(target_task_id)) => {
                    loop {
                        if let Some((_, result)) = fibre.engine_state.pending_db_results.remove(&target_task_id) {
                            match result {
                                Ok(val) => {
                                    Self::push(fibre, val);
                                }
                                Err(err_msg) => {
                                    let _ = Self::create_and_throw_native_error(fibre, &err_msg);
                                }
                            }
                            fibre.resume();
                            break;
                        }
                        match fibre.engine_state.db_completion_rx.recv_timeout(std::time::Duration::from_millis(5)) {
                            Ok((task_id, result)) => {
                                if task_id == target_task_id {
                                    match result {
                                        Ok(val) => {
                                            Self::push(fibre, val);
                                        }
                                        Err(err_msg) => {
                                            let _ = Self::create_and_throw_native_error(fibre, &err_msg);
                                        }
                                    }
                                    fibre.resume();
                                    break;
                                } else if let Some((_, mut other_fibre)) = fibre.engine_state.parked_db_fibres.remove(&task_id) {
                                    match result {
                                        Ok(val) => {
                                            Self::push(&mut other_fibre, val);
                                        }
                                        Err(err_msg) => {
                                            let _ = Self::create_and_throw_native_error(&mut other_fibre, &err_msg);
                                        }
                                    }
                                    other_fibre.resume();
                                } else {
                                    fibre.engine_state.pending_db_results.insert(task_id, result);
                                }
                            }
                            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                                continue;
                            }
                            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                                return ExecutionResult::Error("Fatal error: DB channel disconnected".to_string());
                            }
                        }
                    }
                }
                ExecutionResult::Preempted => {
                    fibre.resume();
                }
                _ => break,
            }
            res = Self::run_fibre(fibre);
        }
        res
    }

    pub fn try_execute_enum_builtin(
        fibre: &mut Fibre,
        class_id: usize,
        method_name: &str,
        args: &[Value],
    ) -> Option<Result<Value, String>> {
        let norm_method = method_name.to_lowercase();
        let is_cases = norm_method == "cases";
        let is_try_from = norm_method == "tryfrom";
        let is_from = norm_method == "from";

        if !is_cases && !is_try_from && !is_from {
            return None;
        }

        let classes = &fibre.engine_state.classes;
        let c = match classes.get(&class_id) {
            Some(c) => c,
            None => return None,
        };
        let is_enum = c.extends.as_deref() == Some("BackedEnum")
            || c.extends.as_deref() == Some("UnitEnum")
            || c.implements.iter().any(|i| i == "BackedEnum" || i == "UnitEnum");

        if !is_enum {
            return None;
        }

        let class_name = c.name.clone();
        drop(classes);

        let norm_class = normalize_name(&class_name);
        let mut static_props: Vec<(String, Value)> = Vec::new();
        let default_statics = &fibre.engine_state.default_statics;
        for kv in default_statics.iter() {
            let (c_name, prop_name) = kv.key();
            let val = kv.value();
            if c_name == &norm_class {
                let actual_val = fibre.static_properties.lock().unwrap().get(&(c_name.clone(), prop_name.clone())).map(|v| *v).unwrap_or(*val);
                static_props.push((prop_name.clone(), actual_val));
            }
        }
        drop(default_statics);
        let locked_statics = fibre.static_properties.lock().unwrap();
        for ((c_name, prop_name), val) in locked_statics.iter() {
            if c_name == &norm_class && !static_props.iter().any(|(p, _)| p == prop_name) {
                static_props.push((prop_name.clone(), *val));
            }
        }

        if is_cases {
            let mut arr = hyperion_core::types::array::PhpArray::new();
            for (idx, (_, val)) in static_props.iter().enumerate() {
                arr.insert_int(idx as i64, *val);
            }
            let arr_ptr = fibre.arena.alloc_and_track(arr);
            return Some(Ok(Value::new_array_ptr(arr_ptr as *mut ())));
        }

        if is_try_from || is_from {
            let target_val = args.get(0).map(|v| *v).unwrap_or(Value::null());
            for (_, case_val) in &static_props {
                if let Some(obj_ptr) = case_val.as_object_ptr() {
                    let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
                    if let Some(val) = obj.properties.get("value") {
                        let matched = match (val.deref().get_type(), target_val.deref().get_type()) {
                            (hyperion_core::memory::nan_box::ValueType::Int, hyperion_core::memory::nan_box::ValueType::Int) => {
                                val.deref().as_int() == target_val.deref().as_int()
                            }
                            (hyperion_core::memory::nan_box::ValueType::String, hyperion_core::memory::nan_box::ValueType::String) => {
                                let s1 = unsafe { &*(val.deref().as_string_ptr().unwrap() as *const String) };
                                let s2 = unsafe { &*(target_val.deref().as_string_ptr().unwrap() as *const String) };
                                s1 == s2
                            }
                            (hyperion_core::memory::nan_box::ValueType::Int, hyperion_core::memory::nan_box::ValueType::String) => {
                                let s2 = unsafe { &*(target_val.deref().as_string_ptr().unwrap() as *const String) };
                                if let Ok(num) = s2.parse::<i32>() {
                                    val.deref().as_int() == Some(num)
                                } else {
                                    false
                                }
                            }
                            (hyperion_core::memory::nan_box::ValueType::String, hyperion_core::memory::nan_box::ValueType::Int) => {
                                let s1 = unsafe { &*(val.deref().as_string_ptr().unwrap() as *const String) };
                                let num = target_val.deref().as_int().unwrap();
                                s1 == &num.to_string()
                            }
                            _ => val.deref().equals(&target_val.deref()),
                        };

                        if matched {
                            return Some(Ok(*case_val));
                        }
                    }
                }
            }

            if is_try_from {
                return Some(Ok(Value::null()));
            } else {
                return Some(Err(format!(
                    "Uncaught ValueError: {:?} is not a valid backing value for enum {}",
                    target_val, class_name
                )));
            }
        }

        None
    }


    /// Resolve an object to something `foreach` can walk.
    ///
    /// `IteratorAggregate::getIterator()` may itself hand back another aggregate
    /// (Laravel's collections do exactly this), so unwrap in a loop rather than
    /// once — bounded, because a cycle would otherwise hang the engine. The
    /// result is whatever the last call produced: an `Iterator` object, a
    /// `Generator`, or an array.
    fn resolve_iterable(fibre: &mut Fibre, val: Value) -> Result<Value, String> {
        let mut current = val;
        for _ in 0..16 {
            let Some(obj_ptr) = current.as_object_ptr() else {
                return Ok(current);
            };
            let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
            let class_id = Self::resolve_obj_class_id(fibre, obj);

            // A Generator drives itself; an Iterator is already walkable.
            let is_generator = {
                let classes = &fibre.engine_state.classes;
                classes.get(&class_id).map(|c| c.name == "Generator").unwrap_or(false)
            };
            if is_generator || Self::class_has_method(fibre, class_id, "current") {
                return Ok(current);
            }

            if Self::class_has_method(fibre, class_id, "getIterator") {
                current = Self::call_method_synchronously(fibre, current, "getIterator", vec![])?;
                continue;
            }

            // Plain object: `foreach` walks its public properties.
            return Ok(current);
        }
        Err("getIterator() nested too deeply while resolving a foreach source".to_string())
    }

    /// Snapshot a plain object's properties as an array, so `foreach ($obj as
    /// $k => $v)` over a non-Traversable walks them the way PHP does.
    fn object_properties_as_array(fibre: &mut Fibre, obj_val: Value) -> Value {
        let mut arr = hyperion_core::types::array::PhpArray::new();
        if let Some(obj_ptr) = obj_val.as_object_ptr() {
            let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
            for (name, v) in &obj.properties {
                let id = hyperion_core::types::string_table::intern_string(name);
                arr.insert_string_id(id, v.deref());
            }
        }
        let ptr = fibre.arena.alloc_and_track(arr) as *mut ();
        Value::new_array_ptr(ptr)
    }

    pub fn step_generator_fibre(gen_fibre: &mut Fibre) -> ExecutionResult {
        Self::drive_fibre_synchronously(gen_fibre)
    }

    /// Whether an `Iterator` object still has an element, after positioning it.
    /// `first` rewinds; later steps advance. Either way `valid()` decides.
    fn iterator_step(fibre: &mut Fibre, obj_val: Value, first: bool) -> Result<bool, String> {
        let method = if first { "rewind" } else { "next" };
        let class_id = {
            let obj_ptr = obj_val
                .as_object_ptr()
                .ok_or_else(|| "foreach source is not an object".to_string())?;
            let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
            Self::resolve_obj_class_id(fibre, obj)
        };
        // rewind()/next() are optional in practice — some iterators only
        // implement valid()/current()/key() — so a missing one is a no-op
        // rather than a fatal error.
        if Self::class_has_method(fibre, class_id, method) {
            Self::call_method_synchronously(fibre, obj_val, method, vec![])?;
        }
        if !Self::class_has_method(fibre, class_id, "valid") {
            return Ok(false);
        }
        let valid = Self::call_method_synchronously(fibre, obj_val, "valid", vec![])?;
        Ok(is_truthy(&valid))
    }

    #[inline(always)]
    pub fn check_and_run_destructor(fibre: &mut Fibre, obj_ptr: *mut ()) -> Result<(), String> {
        let obj = unsafe { &mut *(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
        if obj.destructed {
            return Ok(());
        }
        let class_id = Self::resolve_obj_class_id(fibre, obj);
        if let Ok(Some(_)) = Self::find_method_fast(fibre, class_id, "__destruct") {
            // Check if obj_ptr is still referenced on the current execution stack
            let is_on_stack = fibre.stack[0..fibre.stack_top].iter().any(|v| v.as_object_ptr() == Some(obj_ptr));
            if !is_on_stack {
                obj.destructed = true;
                let obj_val = Value::new_object_ptr(obj_ptr);
                let _ = Self::call_method_synchronously(fibre, obj_val, "__destruct", vec![])?;
            }
        }
        Ok(())
    }

    pub fn call_method_synchronously(
        fibre: &mut Fibre,
        obj_val: Value,
        method_name: &str,
        args: Vec<Value>,
    ) -> Result<Value, String> {
        let prev_stack_top = fibre.stack_top;
        let prev_frame_count = fibre.frame_count;

        let obj_ptr = obj_val
            .as_object_ptr()
            .ok_or_else(|| "Not an object".to_string())?
            as *mut hyperion_core::types::object::PhpObject;
        let obj = unsafe { &*obj_ptr };
        let class_id = Self::resolve_obj_class_id(fibre, obj);

        let mut func_ptr_opt = Self::find_method_fast(fibre, class_id, method_name).unwrap_or(None);
        if let Some(func_ptr) = func_ptr_opt {
            let func = unsafe { &*func_ptr.0 };
            if fibre.frame_count > 0 && !Self::is_method_accessible(fibre, class_id, func, fibre.frame_count - 1) {
                func_ptr_opt = None;
            }
        }

        let call_func_ptr_opt = if func_ptr_opt.is_none() {
            Self::find_method_fast(fibre, class_id, "__call").unwrap_or(None)
        } else {
            None
        };

        if let Some(func_ptr) = func_ptr_opt {
            let func = unsafe { &*func_ptr.0 }; debug_println!("autoload: pushed function {}", func.name);
                let stack_window = fibre.stack_top;
            Self::push(fibre, obj_val);
            for arg in &args {
                Self::push(fibre, *arg);
            }

            if fibre.frame_count >= fibre.frames.len() {
                return Err("Stack overflow: max call depth exceeded".to_string());
            }

            fibre.frames[fibre.frame_count] = CallFrame {
                function: func_ptr,
                ip: 0,
                stack_window,
                called_class_id: class_id,
                return_override: None, eval_parent_stack_window: None,
                arity: args.len(),
            };
            fibre.set_frame_args(fibre.frame_count, &args);
            fibre.frame_count += 1;
            Self::pad_missing_arguments_and_locals(fibre, args.len(), unsafe { &*func_ptr.0 }, &[], false);
        } else if let Some(func_ptr) = call_func_ptr_opt {
            let func = unsafe { &*func_ptr.0 }; debug_println!("autoload: pushed function {}", func.name);
                let stack_window = fibre.stack_top;
            Self::push(fibre, obj_val);

            // For __call, we pass two args: method_name (string) and arguments (array)
            let mut arr = hyperion_core::types::array::PhpArray::new();
            for (i, arg) in args.iter().enumerate() {
                arr.insert_int(i as i64, *arg);
            }
            let arr_ptr = fibre.arena.alloc_and_track(arr) as *mut ();
            let arr_val = Value::new_array_ptr(arr_ptr);

            let name_ptr = fibre.arena.alloc_and_track(method_name.to_string()) as *mut String;
            let name_val = Value::new_string_ptr(name_ptr as *mut ());

            Self::push(fibre, name_val);
            Self::push(fibre, arr_val);

            if fibre.frame_count >= fibre.frames.len() {
                return Err("Stack overflow: max call depth exceeded".to_string());
            }

            fibre.frames[fibre.frame_count] = CallFrame {
                function: func_ptr,
                ip: 0,
                stack_window,
                called_class_id: class_id,
                return_override: None, eval_parent_stack_window: None,
                arity: 2,
            };
            fibre.frame_count += 1;
            Self::pad_missing_arguments_and_locals(fibre, 2, unsafe { &*func_ptr.0 }, &[], false);
        } else {
            let class_name = if let Some(ref name) = obj.class_name {
                name.clone()
            } else {
                let classes = &fibre.engine_state.classes;
                classes.get(&class_id).map(|c| c.name.clone()).unwrap_or_else(|| "Unknown".to_string())
            };
            return Err(format!(
                "Method {}::{} not found",
                class_name,
                method_name
            ));
        }

        // Set stop frame count to stop executing when this frame finishes/returns
        let old_stop = fibre.stop_frame_count;
        fibre.stop_frame_count = Some(prev_frame_count);

        let res = Self::drive_fibre_synchronously(fibre);

        // Restore stop frame count
        fibre.stop_frame_count = old_stop;

        match res {
            ExecutionResult::Finished => {
                let ret_val = Self::pop(fibre);
                fibre.stack_top = prev_stack_top;
                Ok(ret_val)
            }
            ExecutionResult::Error(err) => {
                fibre.stack_top = prev_stack_top;
                fibre.frame_count = prev_frame_count;
                Err(err)
            }
            ExecutionResult::UncaughtException(err) => {
                fibre.stack_top = prev_stack_top;
                fibre.frame_count = prev_frame_count;
                Err(err)
            }

            _ => {
                fibre.stack_top = prev_stack_top;
                fibre.frame_count = prev_frame_count;
                Err(format!("Unexpected execution result in synchronous call: {:?}", res))
            }
        }
    }

    /// Whether `class_id` (or any ancestor) declares `method_name`.
    fn class_has_method(fibre: &Fibre, class_id: usize, method_name: &str) -> bool {
        Self::find_method(&fibre.engine_state, class_id, method_name)
            .ok()
            .flatten()
            .is_some()
    }

    /// Whether `class_id` (or any ancestor) declares `prop_name`.
    fn class_has_declared_property(fibre: &Fibre, class_id: usize, prop_name: &str) -> bool {
        if let Some(cached) = PROPERTY_L1_CACHE.with(|c| c.borrow().get(&(class_id, prop_name.to_string())).copied()) {
            return cached;
        }

        let classes = &fibre.engine_state.classes;
        let mut curr_id = Some(class_id);
        while let Some(cid) = curr_id {
            if let Some(cls) = classes.get(&cid) {
                if cls.default_properties.contains_key(prop_name) {
                    PROPERTY_L1_CACHE.with(|c| c.borrow_mut().insert((class_id, prop_name.to_string()), true));
                    return true;
                }
                curr_id = cls.extends.as_ref().and_then(|parent_name| {
                    let norm = crate::vm::normalize_name(parent_name);
                    fibre.engine_state.class_map.get(&norm).map(|v| *v)
                });
            } else {
                break;
            }
        }
        PROPERTY_L1_CACHE.with(|c| c.borrow_mut().insert((class_id, prop_name.to_string()), false));
        false
    }

    pub fn is_instance_of(fibre: &mut Fibre, obj_val: Value, target_class_name: &str) -> bool {
        let resolved_target = if target_class_name.eq_ignore_ascii_case("static")
            || target_class_name.eq_ignore_ascii_case("self")
            || target_class_name.eq_ignore_ascii_case("parent")
        {
            Self::resolve_dynamic_class_name(fibre, target_class_name)
        } else {
            target_class_name.to_string()
        };

        let target_norm = normalize_name(&resolved_target);
        if target_norm == "closure" && obj_val.is_closure() {
            return true;
        }
        if obj_val.is_object() {
            if let Some(obj_ptr) = obj_val.as_object_ptr() {
                let obj = unsafe {
                    &*(obj_ptr as *const hyperion_core::types::object::PhpObject)
                };
                let class_id = obj.class_id;
                if let Some(cached) = INSTANCE_OF_L1_CACHE.with(|c| c.borrow().get(&(class_id, target_norm.clone())).copied()) {
                    return cached;
                }
                
                let initial_class_name = if let Some(ref cn) = obj.class_name {
                    Some(cn.clone())
                } else {
                    fibre.engine_state.classes.get(&obj.class_id).map(|c| c.name.clone())
                };

                if let Some(c_name) = initial_class_name {
                    let mut seen: Vec<String> = Vec::new();
                    let mut work: Vec<String> = vec![c_name];
                    let mut is_instance = false;
                    while let Some(name) = work.pop() {
                        let norm = normalize_name(&name);
                        if norm == target_norm {
                            is_instance = true;
                            break;
                        }
                        if seen.iter().any(|s| normalize_name(s) == norm) {
                            continue;
                        }
                        seen.push(name.clone());
                        let cid_opt = fibre.engine_state.class_map.get(&norm).map(|v| *v);
                        let cid = if let Some(cid) = cid_opt {
                            Some(cid)
                        } else {
                            fibre.trigger_autoload_sync(&name);
                            fibre.engine_state.class_map.get(&norm).map(|v| *v)
                        };
                        if let Some(cid) = cid {
                            if let Some(c) = fibre.engine_state.classes.get(&cid) {
                                if let Some(parent) = &c.extends {
                                    work.push(parent.clone());
                                }
                                for i in &c.implements {
                                    work.push(i.clone());
                                }
                            }
                        }
                    }
                    if !is_instance && target_norm == "closure"
                        && fibre.engine_state.classes.get(&obj.class_id).map(|c| c.name.starts_with("closure#")).unwrap_or(false)
                    {
                        is_instance = true;
                    }
                    INSTANCE_OF_L1_CACHE.with(|c| c.borrow_mut().insert((class_id, target_norm), is_instance));
                    return is_instance;
                }
            }
        } else if obj_val.is_closure() && target_norm == "closure" {
            return true;
        }
        false
    }

    pub fn cast_value_to_string(fibre: &mut Fibre, val: Value) -> Result<String, String> {
        let val = val.deref();
        if let Some(i) = val.as_int() {
            Ok(i.to_string())
        } else if let Some(f) = val.as_float() {
            Ok(f.to_string())
        } else if let Some(ptr) = val.as_string_ptr() {
            let s = unsafe { &*(ptr as *const String) };
            Ok(s.clone())
        } else if val.is_bool() {
            Ok(if val.as_bool().unwrap_or(false) {
                "1".to_string()
            } else {
                "".to_string()
            })
        } else if val.is_null() {
            Ok("".to_string())
        } else if val.is_array() {
            Ok(format!("Array (hex: {:X})", val.0))
        } else if val.is_object() {
            let obj_ptr =
                val.as_object_ptr().unwrap() as *mut hyperion_core::types::object::PhpObject;
            let obj = unsafe { &*obj_ptr };
            let class_id = Self::resolve_obj_class_id(fibre, obj);
            let has_to_string = Self::find_method_with_autoload(fibre, class_id, "__toString");
            if has_to_string.unwrap_or(None).is_some() {
                let res = Self::call_method_synchronously(fibre, val, "__toString", vec![])?;
                let res = res.deref();
                if let Some(s_ptr) = res.as_string_ptr() {
                    let s = unsafe { &*(s_ptr as *const String) };
                    Ok(s.clone())
                } else if res.is_null() {
                    Ok("".to_string())
                } else if let Some(i) = res.as_int() {
                    Ok(i.to_string())
                } else if let Some(f) = res.as_float() {
                    Ok(f.to_string())
                } else {
                    Err("Method __toString() must return a string value".to_string())
                }
            } else {
                let class_name = if let Some(ref name) = obj.class_name {
                    name.clone()
                } else {
                    let classes = &fibre.engine_state.classes;
                    classes.get(&class_id).map(|c| c.name.clone()).unwrap_or_else(|| "Object".to_string())
                };
                Err(format!(
                    "Object of class {} could not be converted to string",
                    class_name
                ))
            }
        } else {
            Err("Cannot convert value to string".to_string())
        }
    }

    pub fn find_static_property_key(
        classes: &[crate::types::class::PhpClass],
        class_name: &str,
        prop_name: &str,
        default_statics: &std::collections::HashMap<(String, String), Value>,
    ) -> (String, String) {
        let mut current_class_name = normalize_name(class_name);
        loop {
            let key = (current_class_name.clone(), prop_name.to_string());
            if default_statics.contains_key(&key) {
                return key;
            }
            if let Some(cls) = classes
                .iter()
                .find(|c| normalize_name(&c.name) == current_class_name)
            {
                if let Some(parent_name) = &cls.extends {
                    current_class_name = normalize_name(parent_name);
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        (normalize_name(class_name), prop_name.to_string())
    }

    pub fn handle_throw(fibre: &mut Fibre, exception_val: Value) -> Option<ExecutionResult> {
        hyperion_core::hyp_debug!("Exception Thrown! Stack trace:");
        for i in (0..fibre.frame_count).rev() {
            let frame = fibre.frames[i];
            let func = unsafe { &*frame.function.0 };
            hyperion_core::hyp_debug!("  #{} {} at ip: {}", i, func.name, frame.ip);
        }

        let mut current_frame = fibre.frame_count;
        let mut found_catch = false;

        while current_frame > 0 {
            let frame = fibre.frames[current_frame - 1];
            let func = unsafe { &*frame.function.0 };
            let ip = frame.ip;
            let chunk = &func.chunk;

            let mut innermost_catch = None;
            for handler in chunk.exception_handlers.iter() {
                if ip >= handler.start_ip && ip <= handler.end_ip {
                    // Check if the thrown exception matches the catch block's class type
                    if handler.catch_class.is_empty() || Self::is_instance_of(fibre, exception_val, &handler.catch_class) {
                        innermost_catch = Some(handler);
                        break;
                    }
                }
            }

            let mut innermost_finally = None;
            for handler in chunk.finally_handlers.iter() {
                if ip >= handler.start_ip && ip <= handler.end_ip {
                    innermost_finally = Some(handler);
                    break;
                }
            }

            let mut target_ip = None;
            let mut is_finally = false;

            if let Some(catch) = innermost_catch {
                if let Some(finally) = innermost_finally {
                    if catch.start_ip >= finally.start_ip && catch.end_ip <= finally.end_ip {
                        target_ip = Some(catch.catch_ip);
                    } else {
                        target_ip = Some(finally.finally_ip);
                        is_finally = true;
                    }
                } else {
                    target_ip = Some(catch.catch_ip);
                }
            } else if let Some(finally) = innermost_finally {
                target_ip = Some(finally.finally_ip);
                is_finally = true;
            }

            if let Some(tip) = target_ip {
                let num_locals = func.num_locals;
                let frame_locals = 1 + num_locals.max(frame.arity);
                let stack_depth = if let Some(catch) = innermost_catch {
                    catch.stack_depth
                } else if let Some(finally) = innermost_finally {
                    finally.stack_depth
                } else {
                    0
                };
                fibre.stack_top = frame.stack_window + frame_locals + stack_depth;

                if is_finally {
                    fibre.unwind_action =
                        Some(crate::fibre::UnwindAction::Throw(exception_val));
                    fibre.frames[current_frame - 1].ip = tip;
                } else {
                    Self::push(fibre, exception_val);
                    fibre.frames[current_frame - 1].ip = tip;
                    fibre.unwind_action = None;
                }

                fibre.frame_count = current_frame;
                fibre.dynamic_locals.retain(|&k, _| k < current_frame);
                found_catch = true;
                break;
            }

            current_frame -= 1;
        }

        if !found_catch {
            let msg = if let Some(obj_ptr) = exception_val.as_object_ptr() {
                let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
                let class_name = if let Some(ref name) = obj.class_name {
                    name.clone()
                } else {
                    let classes = &fibre.engine_state.classes;
                    classes.get(&obj.class_id).map(|c| c.name.clone()).unwrap_or_else(|| "Exception".to_string())
                };
                if let Some(val) = obj.properties.get("message") {
                    if let Some(msg_ptr) = val.as_string_ptr() {
                        let m = unsafe { (*(msg_ptr as *const String)).clone() };
                        format!("{}: {}", class_name, m)
                    } else {
                        format!("{}: (non-string message property)", class_name)
                    }
                } else {
                    format!("{}: (no message property, properties={:?})", class_name, obj.properties.keys().collect::<Vec<_>>())
                }
            } else if let Some(msg_ptr) = exception_val.as_string_ptr() {
                let m = unsafe { (*(msg_ptr as *const String)).clone() };
                format!("Uncaught Exception: {}", m)
            } else {
                format!("Uncaught Exception (not an object: val={:?}, type={:?})", exception_val, exception_val.get_type())
            };

            let mut trace = Vec::new();
            for i in (0..fibre.frame_count).rev() {
                let f = &fibre.frames[i];
                if !f.function.0.is_null() {
                    let func = unsafe { &*f.function.0 };
                    trace.push(format!("  #{} {} (ip={})", i, func.name, f.ip));
                }
            }
            let full_msg = if trace.is_empty() {
                msg
            } else {
                format!("{}\nStack trace:\n{}", msg, trace.join("\n"))
            };
            return Some(ExecutionResult::UncaughtException(full_msg));
        }

        None
    }

    pub fn create_and_throw_native_error(fibre: &mut Fibre, err_str: &str) -> Option<ExecutionResult> {
        let (class_name, message) = if let Some(pos) = err_str.find(": ") {
            let prefix = &err_str[..pos];
            let msg = &err_str[pos + 2..];
            if prefix == "PDOException"
                || prefix == "InvalidArgumentException"
                || prefix == "RuntimeException"
                || prefix == "LogicException"
                || prefix == "TypeError"
                || prefix == "Error"
                || prefix == "Exception"
                || prefix == "BadMethodCallException"
                || prefix == "OutOfBoundsException"
                || prefix == "UnexpectedValueException"
                || prefix == "ReflectionException"
                || prefix == "ValueError"
                || prefix == "DivisionByZeroError"
            {
                (prefix, msg)
            } else {
                ("Exception", err_str)
            }
        } else {
            ("Exception", err_str)
        };

        let class_id = Self::find_class_id(fibre, class_name).unwrap_or(0);

        let mut obj = hyperion_core::types::object::PhpObject::new_with_name(class_id, class_name.to_string());
        let msg_ptr = fibre.arena.alloc_and_track(message.to_string());
        obj.properties.insert("message".to_string(), Value::new_string_ptr(msg_ptr as *mut ()));
        obj.properties.insert("code".to_string(), Value::new_int(0));

        let file_line = if fibre.frame_count > 0 {
            let caller_frame = &fibre.frames[fibre.frame_count - 1];
            let func = unsafe { &*caller_frame.function.0 };
            (func.name.clone(), 0)
        } else {
            ("unknown".to_string(), 0)
        };
        let file_ptr = fibre.arena.alloc_and_track(file_line.0);
        obj.properties.insert("file".to_string(), Value::new_string_ptr(file_ptr as *mut ()));
        obj.properties.insert("line".to_string(), Value::new_int(file_line.1));
        obj.properties.insert("previous".to_string(), Value::null());

        let obj_ptr = fibre.arena.alloc_and_track(obj) as *mut ();
        let exception_val = Value::new_object_ptr(obj_ptr);

        Self::handle_throw(fibre, exception_val)
    }

    pub fn handle_return(fibre: &mut Fibre, ret_val: Value) -> Option<ExecutionResult> {
        let frame = fibre.frames[fibre.frame_count - 1];
        let old_stack_top = frame.stack_window;

        let mut is_autoload = false;
        if let Some(&depth) = fibre.autoload_frame_depths.last() {
            if depth == fibre.frame_count - 1 {
                is_autoload = true;
                fibre.autoload_frame_depths.pop();
            }
        }

        fibre.frame_count -= 1;
        fibre.stack_top = old_stack_top;

        if !is_autoload {
            Self::push(fibre, ret_val);
        }
        if fibre.frame_count == 0 {
            return Some(ExecutionResult::Finished);
        }
        None
    }

    pub fn run_fibre(fibre: &mut Fibre) -> ExecutionResult {
        let _arena_scope = fibre.arena.enter();
        let gas_limit: usize = 1_000_000;
        let mut gas = 0;

        'vm_loop: loop {
            if let Some(stop) = fibre.stop_frame_count {
                if fibre.frame_count <= stop {
                    return ExecutionResult::Finished;
                }
            }

            if fibre.frame_count == 0 {
                return ExecutionResult::Finished;
            }

            let frame_idx = fibre.frame_count - 1;
            let start_ip = fibre.frames[frame_idx].ip;
            if start_ip == 0 {
                let func = unsafe { &*fibre.frames[frame_idx].function.0 };
                if let Some(ref jit) = func.jit_entry {
                    let mut ret = hyperion_jit::assembler::JitReturn {
                        tag: 0,
                        payload_ip: 0,
                        stack_top: fibre.stack_top as u64,
                    };
                    (jit.execute_fn)(fibre.stack.as_mut_ptr(), &mut ret);
                    if ret.tag == 0 {
                        let old_stack_top = fibre.frames[frame_idx].stack_window;
                        fibre.frame_count -= 1;
                        fibre.stack_top = old_stack_top + 1;
                        continue 'vm_loop;
                    } else {
                        fibre.frames[frame_idx].ip = ret.payload_ip as usize;
                        fibre.stack_top = ret.stack_top as usize;
                    }
                }
            }

            let start_ip = fibre.frames[fibre.frame_count - 1].ip;
            let current_ip = start_ip;
            let byte = Self::read_byte(fibre);
            let op = match Opcode::try_from(byte) {
                Ok(op) => op,
                Err(_) => return ExecutionResult::Error(format!("Unknown opcode: {}", byte)),
            };

            #[cfg(debug_assertions)]
            if std::env::var("HYPERION_TRACE").is_ok() {
                if fibre.trace_log.len() == 20 {
                    fibre.trace_log.remove(0);
                }
                fibre.trace_log.push(format!(
                    "IP: {}, OP: {:?}",
                    fibre.frames[fibre.frame_count - 1].ip,
                    op
                ));
            }


            match op {
                Opcode::LessThan
                | Opcode::GreaterThan
                | Opcode::LessThanOrEqual
                | Opcode::GreaterThanOrEqual
                | Opcode::NotEquals
                | Opcode::StrictNotEquals
                | Opcode::Modulo
                | Opcode::BitwiseOr
                | Opcode::BitwiseAnd
                | Opcode::BitwiseXor
                | Opcode::ShiftLeft
                | Opcode::ShiftRight
                | Opcode::Power
                | Opcode::LogicalAnd
                | Opcode::LogicalOr => {
                    if let Some(tracer) = &mut fibre.tracer {
                        if tracer.is_recording {
                            let a = fibre.stack[fibre.stack_top - 2];
                            let b = fibre.stack[fibre.stack_top - 1];

                            if a.is_int() && b.is_int() {
                                let a_tag = a.0 & 0xFFFF000000000000;
                                let b_tag = b.0 & 0xFFFF000000000000;

                                match op {
                                    Opcode::LessThan => {
                                        tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardType {
                                            src: fibre.stack_top - 2,
                                            expected_type_tag: a_tag,
                                            bailout_ip: current_ip,
                                            bailout_stack_top: fibre.stack_top,
                                        });
                                        tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardType {
                                            src: fibre.stack_top - 1,
                                            expected_type_tag: b_tag,
                                            bailout_ip: current_ip,
                                            bailout_stack_top: fibre.stack_top,
                                        });
                                        tracer.trace.push(hyperion_jit::tracer::TraceIR::LessThan {
                                            dest: fibre.stack_top - 2,
                                            left: fibre.stack_top - 2,
                                            right: fibre.stack_top - 1,
                                        });
                                    }
                                    Opcode::GreaterThan => {
                                        tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardType {
                                            src: fibre.stack_top - 2,
                                            expected_type_tag: a_tag,
                                            bailout_ip: current_ip,
                                            bailout_stack_top: fibre.stack_top,
                                        });
                                        tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardType {
                                            src: fibre.stack_top - 1,
                                            expected_type_tag: b_tag,
                                            bailout_ip: current_ip,
                                            bailout_stack_top: fibre.stack_top,
                                        });
                                        tracer.trace.push(hyperion_jit::tracer::TraceIR::GreaterThan {
                                            dest: fibre.stack_top - 2,
                                            left: fibre.stack_top - 2,
                                            right: fibre.stack_top - 1,
                                        });
                                    }
                                    Opcode::LessThanOrEqual => {
                                        tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardType {
                                            src: fibre.stack_top - 2,
                                            expected_type_tag: a_tag,
                                            bailout_ip: current_ip,
                                            bailout_stack_top: fibre.stack_top,
                                        });
                                        tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardType {
                                            src: fibre.stack_top - 1,
                                            expected_type_tag: b_tag,
                                            bailout_ip: current_ip,
                                            bailout_stack_top: fibre.stack_top,
                                        });
                                        tracer.trace.push(hyperion_jit::tracer::TraceIR::LessThanOrEqual {
                                            dest: fibre.stack_top - 2,
                                            left: fibre.stack_top - 2,
                                            right: fibre.stack_top - 1,
                                        });
                                    }
                                    Opcode::GreaterThanOrEqual => {
                                        tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardType {
                                            src: fibre.stack_top - 2,
                                            expected_type_tag: a_tag,
                                            bailout_ip: current_ip,
                                            bailout_stack_top: fibre.stack_top,
                                        });
                                        tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardType {
                                            src: fibre.stack_top - 1,
                                            expected_type_tag: b_tag,
                                            bailout_ip: current_ip,
                                            bailout_stack_top: fibre.stack_top,
                                        });
                                        tracer.trace.push(hyperion_jit::tracer::TraceIR::GreaterThanOrEqual {
                                            dest: fibre.stack_top - 2,
                                            left: fibre.stack_top - 2,
                                            right: fibre.stack_top - 1,
                                        });
                                    }
                                    Opcode::NotEquals => {
                                        tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardType {
                                            src: fibre.stack_top - 2,
                                            expected_type_tag: a_tag,
                                            bailout_ip: current_ip,
                                            bailout_stack_top: fibre.stack_top,
                                        });
                                        tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardType {
                                            src: fibre.stack_top - 1,
                                            expected_type_tag: b_tag,
                                            bailout_ip: current_ip,
                                            bailout_stack_top: fibre.stack_top,
                                        });
                                        tracer.trace.push(hyperion_jit::tracer::TraceIR::NotEquals {
                                            dest: fibre.stack_top - 2,
                                            left: fibre.stack_top - 2,
                                            right: fibre.stack_top - 1,
                                        });
                                    }
                                    _ => tracer.stop(),
                                }
                            } else if a.is_float() && b.is_float() {
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardFloat {
                                    src: fibre.stack_top - 2,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardFloat {
                                    src: fibre.stack_top - 1,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top,
                                });

                                match op {
                                    Opcode::LessThan => tracer.trace.push(hyperion_jit::tracer::TraceIR::FloatLessThan {
                                        dest: fibre.stack_top - 2,
                                        left: fibre.stack_top - 2,
                                        right: fibre.stack_top - 1,
                                    }),
                                    Opcode::GreaterThan => tracer.trace.push(hyperion_jit::tracer::TraceIR::FloatGreaterThan {
                                        dest: fibre.stack_top - 2,
                                        left: fibre.stack_top - 2,
                                        right: fibre.stack_top - 1,
                                    }),
                                    Opcode::LessThanOrEqual => tracer.trace.push(hyperion_jit::tracer::TraceIR::FloatLessThanOrEqual {
                                        dest: fibre.stack_top - 2,
                                        left: fibre.stack_top - 2,
                                        right: fibre.stack_top - 1,
                                    }),
                                    Opcode::GreaterThanOrEqual => tracer.trace.push(hyperion_jit::tracer::TraceIR::FloatGreaterThanOrEqual {
                                        dest: fibre.stack_top - 2,
                                        left: fibre.stack_top - 2,
                                        right: fibre.stack_top - 1,
                                    }),
                                    Opcode::NotEquals => tracer.trace.push(hyperion_jit::tracer::TraceIR::FloatNotEquals {
                                        dest: fibre.stack_top - 2,
                                        left: fibre.stack_top - 2,
                                        right: fibre.stack_top - 1,
                                    }),
                                    _ => fibre.tracer = None,
                                }
                            } else if a.is_int() && b.is_float() {
                                let a_tag = a.0 & 0xFFFF000000000000;
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardType {
                                    src: fibre.stack_top - 2,
                                    expected_type_tag: a_tag,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardFloat {
                                    src: fibre.stack_top - 1,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::IntToFloat {
                                    dest: fibre.stack_top - 2,
                                    src: fibre.stack_top - 2,
                                });
                                match op {
                                    Opcode::LessThan => tracer.trace.push(hyperion_jit::tracer::TraceIR::FloatLessThan {
                                        dest: fibre.stack_top - 2,
                                        left: fibre.stack_top - 2,
                                        right: fibre.stack_top - 1,
                                    }),
                                    Opcode::GreaterThan => tracer.trace.push(hyperion_jit::tracer::TraceIR::FloatGreaterThan {
                                        dest: fibre.stack_top - 2,
                                        left: fibre.stack_top - 2,
                                        right: fibre.stack_top - 1,
                                    }),
                                    Opcode::LessThanOrEqual => tracer.trace.push(hyperion_jit::tracer::TraceIR::FloatLessThanOrEqual {
                                        dest: fibre.stack_top - 2,
                                        left: fibre.stack_top - 2,
                                        right: fibre.stack_top - 1,
                                    }),
                                    Opcode::GreaterThanOrEqual => tracer.trace.push(hyperion_jit::tracer::TraceIR::FloatGreaterThanOrEqual {
                                        dest: fibre.stack_top - 2,
                                        left: fibre.stack_top - 2,
                                        right: fibre.stack_top - 1,
                                    }),
                                    Opcode::NotEquals => tracer.trace.push(hyperion_jit::tracer::TraceIR::FloatNotEquals {
                                        dest: fibre.stack_top - 2,
                                        left: fibre.stack_top - 2,
                                        right: fibre.stack_top - 1,
                                    }),
                                    _ => fibre.tracer = None,
                                }
                            } else if a.is_float() && b.is_int() {
                                let b_tag = b.0 & 0xFFFF000000000000;
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardFloat {
                                    src: fibre.stack_top - 2,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardType {
                                    src: fibre.stack_top - 1,
                                    expected_type_tag: b_tag,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::IntToFloat {
                                    dest: fibre.stack_top - 1,
                                    src: fibre.stack_top - 1,
                                });
                                match op {
                                    Opcode::LessThan => tracer.trace.push(hyperion_jit::tracer::TraceIR::FloatLessThan {
                                        dest: fibre.stack_top - 2,
                                        left: fibre.stack_top - 2,
                                        right: fibre.stack_top - 1,
                                    }),
                                    Opcode::GreaterThan => tracer.trace.push(hyperion_jit::tracer::TraceIR::FloatGreaterThan {
                                        dest: fibre.stack_top - 2,
                                        left: fibre.stack_top - 2,
                                        right: fibre.stack_top - 1,
                                    }),
                                    Opcode::LessThanOrEqual => tracer.trace.push(hyperion_jit::tracer::TraceIR::FloatLessThanOrEqual {
                                        dest: fibre.stack_top - 2,
                                        left: fibre.stack_top - 2,
                                        right: fibre.stack_top - 1,
                                    }),
                                    Opcode::GreaterThanOrEqual => tracer.trace.push(hyperion_jit::tracer::TraceIR::FloatGreaterThanOrEqual {
                                        dest: fibre.stack_top - 2,
                                        left: fibre.stack_top - 2,
                                        right: fibre.stack_top - 1,
                                    }),
                                    Opcode::NotEquals => tracer.trace.push(hyperion_jit::tracer::TraceIR::FloatNotEquals {
                                        dest: fibre.stack_top - 2,
                                        left: fibre.stack_top - 2,
                                        right: fibre.stack_top - 1,
                                    }),
                                    _ => fibre.tracer = None,
                                }
                            } else {
                                tracer.stop();
                            }
                        }
                    }

                    let b = Self::pop(fibre);
                    let a = Self::pop(fibre);
                    match op {
                        // All four relational operators are the same comparison
                        // read differently, and the int/float-only versions that
                        // used to live here answered `false` for every string,
                        // array or bool operand.
                        Opcode::LessThan
                        | Opcode::GreaterThan
                        | Opcode::LessThanOrEqual
                        | Opcode::GreaterThanOrEqual => {
                            let ord = Self::php_compare(&a, &b);
                            let result = match op {
                                Opcode::LessThan => ord.is_lt(),
                                Opcode::GreaterThan => ord.is_gt(),
                                Opcode::LessThanOrEqual => ord.is_le(),
                                _ => ord.is_ge(),
                            };
                            Self::push(fibre, Value::new_bool(result));
                        }
                        Opcode::NotEquals => {
                            let result = Self::php8_loose_equals(fibre, &a, &b);
                            Self::push(fibre, Value::new_bool(!result));
                        }
                        Opcode::StrictNotEquals => {
                            // `Value::strict_equals` compares handles, so it is
                            // wrong for arrays; share the opcode's own logic.
                            let result = Self::php_strict_equals(fibre, &a, &b);
                            Self::push(fibre, Value::new_bool(!result));
                        }
                        Opcode::Modulo => {
                            if let (Some(ai), Some(bi)) = (a.as_int(), b.as_int()) {
                                if bi == 0 {
                                    return ExecutionResult::Error("Division by zero".to_string());
                                }
                                Self::push(fibre, Value::new_int(ai % bi));
                            } else {
                                Self::push(fibre, Value::new_bool(false));
                            }
                        }
                        Opcode::LogicalAnd => {
                            Self::push(fibre, Value::new_bool(a.is_truthy() && b.is_truthy()));
                        }
                        Opcode::LogicalOr => {
                            Self::push(fibre, Value::new_bool(a.is_truthy() || b.is_truthy()));
                        }
                        Opcode::BitwiseOr => {
                            if a.is_string() && b.is_string() {
                                let as_ptr = a.as_string_ptr().unwrap();
                                let bs_ptr = b.as_string_ptr().unwrap();
                                let as_str = unsafe { &*(as_ptr as *const String) };
                                let bs_str = unsafe { &*(bs_ptr as *const String) };
                                let a_bytes: Vec<u8> = as_str.chars().map(|c| c as u8).collect();
                                let b_bytes: Vec<u8> = bs_str.chars().map(|c| c as u8).collect();
                                let max_len = a_bytes.len().max(b_bytes.len());
                                let mut res = String::with_capacity(max_len);
                                for i in 0..max_len {
                                    let ab = if i < a_bytes.len() { a_bytes[i] } else { 0 };
                                    let bb = if i < b_bytes.len() { b_bytes[i] } else { 0 };
                                    res.push((ab | bb) as char);
                                }
                                let s_ptr = fibre.arena.alloc_and_track(res) as *mut String;
                                Self::push(fibre, Value::new_string_ptr(s_ptr as *mut ()));
                            } else {
                                let ai = a.to_int_coerced();
                                let bi = b.to_int_coerced();
                                Self::push(fibre, Value::new_int((ai | bi) as i32));
                            }
                        }
                        Opcode::BitwiseAnd => {
                            if a.is_string() && b.is_string() {
                                let as_ptr = a.as_string_ptr().unwrap();
                                let bs_ptr = b.as_string_ptr().unwrap();
                                let as_str = unsafe { &*(as_ptr as *const String) };
                                let bs_str = unsafe { &*(bs_ptr as *const String) };
                                let a_bytes: Vec<u8> = as_str.chars().map(|c| c as u8).collect();
                                let b_bytes: Vec<u8> = bs_str.chars().map(|c| c as u8).collect();
                                let min_len = a_bytes.len().min(b_bytes.len());
                                let mut res = String::with_capacity(min_len);
                                for i in 0..min_len {
                                    res.push((a_bytes[i] & b_bytes[i]) as char);
                                }
                                let s_ptr = fibre.arena.alloc_and_track(res) as *mut String;
                                Self::push(fibre, Value::new_string_ptr(s_ptr as *mut ()));
                            } else {
                                let ai = a.to_int_coerced();
                                let bi = b.to_int_coerced();
                                Self::push(fibre, Value::new_int((ai & bi) as i32));
                            }
                        }
                        Opcode::BitwiseXor => {
                            if a.is_string() && b.is_string() {
                                let as_ptr = a.as_string_ptr().unwrap();
                                let bs_ptr = b.as_string_ptr().unwrap();
                                let as_str = unsafe { &*(as_ptr as *const String) };
                                let bs_str = unsafe { &*(bs_ptr as *const String) };
                                let a_bytes: Vec<u8> = as_str.chars().map(|c| c as u8).collect();
                                let b_bytes: Vec<u8> = bs_str.chars().map(|c| c as u8).collect();
                                let min_len = a_bytes.len().min(b_bytes.len());
                                let mut res = String::with_capacity(min_len);
                                for i in 0..min_len {
                                    res.push((a_bytes[i] ^ b_bytes[i]) as char);
                                }
                                let s_ptr = fibre.arena.alloc_and_track(res) as *mut String;
                                Self::push(fibre, Value::new_string_ptr(s_ptr as *mut ()));
                            } else {
                                let ai = a.to_int_coerced();
                                let bi = b.to_int_coerced();
                                Self::push(fibre, Value::new_int((ai ^ bi) as i32));
                            }
                        }
                        Opcode::ShiftLeft => {
                            let ai = a.to_int_coerced();
                            let bi = b.to_int_coerced();
                            Self::push(fibre, Value::new_int((ai.wrapping_shl(bi as u32)) as i32));
                        }
                        Opcode::ShiftRight => {
                            let ai = a.to_int_coerced();
                            let bi = b.to_int_coerced();
                            Self::push(fibre, Value::new_int((ai.wrapping_shr(bi as u32)) as i32));
                        }
                        Opcode::Power => {
                            if let (Some(ai), Some(bi)) = (a.as_int(), b.as_int()) {
                                Self::push(fibre, Value::new_int(ai.pow(bi as u32)));
                            } else if let (Some(af), Some(bf)) = (a.as_float(), b.as_float()) {
                                Self::push(fibre, Value::new_float(af.powf(bf)));
                            } else {
                                Self::push(fibre, Value::new_int(0));
                            }
                        }
                        _ => unreachable!(),
                    }
                }

                Opcode::LateStaticPropertyGet => {
                    let prop_idx = Self::read_short(fibre);
                    let called_class_id = Self::get_current_called_class_id(fibre);
                    let frame = fibre.frames[fibre.frame_count - 1];
                    let func = unsafe { &*frame.function.0 };
                    let prop_val = func.chunk.constants[prop_idx as usize];

                    let prop_name =
                        unsafe { &*(prop_val.as_string_ptr().unwrap() as *const String) };

                    let classes = &fibre.engine_state.classes;
                    let class_name = classes.get(&called_class_id).map(|c| c.name.clone()).unwrap_or_default();
                    drop(classes);

                    if prop_name.eq_ignore_ascii_case("class") {
                        let str_ptr = fibre.arena.alloc_and_track(class_name);
                        let val = Value::new_string_ptr(str_ptr as *mut ());
                        Self::push(fibre, val);
                        continue;
                    }

                    let val = fibre.get_static_property(&class_name, prop_name).unwrap_or(Value::null());
                    Self::push(fibre, val);
                }
                Opcode::New => {
                    let name_idx = Self::read_short(fibre);
                    let frame = fibre.frames[fibre.frame_count - 1];
                    let func = unsafe { &*frame.function.0 };
                    let name_val = func.chunk.constants[name_idx as usize];
                    let name_ptr = name_val.as_string_ptr().unwrap() as *mut String;
                    let raw_class_name = unsafe { (*name_ptr).clone() };
                    let class_name = Self::resolve_dynamic_class_name(fibre, &raw_class_name);

                    
                    let class_id_opt = Self::find_class_id(fibre, &class_name);

                    let class_id = match class_id_opt {
                        Some(id) => id,
                        None => {
                            if let Some(id) = Self::find_class_id(fibre, &class_name) {
                                id
                            } else {
                                match Self::trigger_autoload(fibre, &class_name) {
                                    Ok(true) => {
                                        let caller_idx = fibre.frame_count - 2;
                                        fibre.frames[caller_idx].ip = start_ip;
                                        continue 'vm_loop;
                                    }
                                    Ok(false) => {
                                        if let Some(id) = Self::find_class_id(fibre, &class_name) {
                                            id
                                        } else {
                                            return ExecutionResult::Error(format!(
                                                "Class '{}' not found",
                                                class_name
                                            ));
                                        }
                                    }
                                    Err(e) => return ExecutionResult::Error(e),
                                }
                            }
                        }
                    };

                    let mut obj = hyperion_core::types::object::PhpObject::new_with_name(class_id, class_name.clone());
                    
                    let mut missing_parent = None;
                    
                    let classes = &fibre.engine_state.classes;
                    let mut current_class_id = class_id;
                    let mut property_overrides = std::collections::HashSet::new();
                    loop {
                        if let Some(c) = classes.get(&current_class_id) {
                            for (k, (val, _vis)) in &c.default_properties {
                                if !property_overrides.contains(k) {
                                    let cloned_val = Fibre::clone_default_property(&mut fibre.arena, val);
                                    obj.properties.insert(k.clone(), cloned_val);
                                    property_overrides.insert(k.clone());
                                }
                            }

                            if let Some(ref ext) = c.extends {
                                if let Some(parent_idx) = fibre.engine_state.class_map.get(&normalize_name(ext)).map(|v| *v) {
                                    current_class_id = parent_idx;
                                    continue;
                                } else {
                                    missing_parent = Some(ext.clone());
                                }
                            }
                        }
                        break;
                    }

                    if let Some(parent_name) = missing_parent {
                        match Self::trigger_autoload(fibre, &parent_name) {
                            Ok(true) => {
                                let caller_idx = fibre.frame_count - 2;
                                fibre.frames[caller_idx].ip = start_ip;
                                continue 'vm_loop;
                            }
                            Ok(false) => {}
                            Err(e) => return ExecutionResult::Error(e),
                        }
                    }

                    let obj_ptr = fibre.arena.alloc_and_track(obj);
                    Self::push(fibre, Value::new_object_ptr(obj_ptr as *mut ()));
                }
                Opcode::NewDynamic => {
                    let class_name_val = Self::pop(fibre).deref();
                    let mut class_name = if let Some(s_ptr) = class_name_val.as_string_ptr() {
                        unsafe { (*(s_ptr as *const String)).clone() }
                    } else if let Some(obj_ptr) = class_name_val.as_object_ptr() {
                        let obj = unsafe { &*(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
                        let cid = Self::resolve_obj_class_id(fibre, obj);
                        let classes = &fibre.engine_state.classes;
                        if let Some(c) = classes.get(&cid) {
                            c.name.clone()
                        } else {
                            return ExecutionResult::Error("Class name must be a valid string or object".to_string());
                        }
                    } else {
                        let val_str = if class_name_val.is_null() {
                            "null"
                        } else if class_name_val.is_bool() {
                            "bool"
                        } else if class_name_val.is_int() {
                            "int"
                        } else if class_name_val.is_float() {
                            "float"
                        } else if class_name_val.is_array() {
                            "array"
                        } else if class_name_val.is_closure() {
                            "closure"
                        } else {
                            "unknown"
                        };
                        let func_name = if fibre.frame_count > 0 {
                            let frame = &fibre.frames[fibre.frame_count - 1];
                            let func = unsafe { &*frame.function.0 };
                            format!("{} (ip={})", func.name, frame.ip)
                        } else {
                            "unknown".to_string()
                        };
                        return ExecutionResult::Error(
                            format!("Class name must be a valid object or a string, got {} in {}", val_str, func_name),
                        );
                    };

                    if class_name.eq_ignore_ascii_case("static")
                        || class_name.eq_ignore_ascii_case("self")
                    {
                        let frame = fibre.frames[fibre.frame_count - 1];
                        let called_class_id = frame.called_class_id;
                        
                        let classes = &fibre.engine_state.classes;
                        if let Some(c) = classes.get(&called_class_id) {
                            class_name = c.name.clone();
                        }
                    }
                    
                    let class_id_opt = Self::find_class_id(fibre, &class_name);

                    let class_id = match class_id_opt {
                        Some(id) => id,
                        None => {
                            if let Some(id) = Self::find_class_id(fibre, &class_name) {
                                id
                            } else {
                                Self::push(fibre, class_name_val);
                                match Self::trigger_autoload(fibre, &class_name) {
                                    Ok(true) => {
                                        let caller_idx = fibre.frame_count - 2;
                                        fibre.frames[caller_idx].ip = start_ip;
                                        continue 'vm_loop;
                                    }
                                    Ok(false) => {
                                        Self::pop(fibre);
                                        if let Some(id) = Self::find_class_id(fibre, &class_name) {
                                            id
                                        } else {
                                            return ExecutionResult::Error(format!(
                                                "Class '{}' not found",
                                                class_name
                                            ));
                                        }
                                    }
                                    Err(e) => return ExecutionResult::Error(e),
                                }
                            }
                        }
                    };

                    let mut obj = hyperion_core::types::object::PhpObject::new_with_name(class_id, class_name.clone());
                    
                    let cached_props = DEFAULT_PROPERTIES_L1_CACHE.with(|c| c.borrow().get(&class_id).cloned());
                    if let Some(props) = cached_props {
                        for (k, val) in props {
                            let cloned_val = Fibre::clone_default_property(&mut fibre.arena, &val);
                            obj.properties.insert(k, cloned_val);
                        }
                    } else {
                        let mut resolved_props = Vec::new();
                        let mut property_overrides = std::collections::HashSet::new();
                        let classes = &fibre.engine_state.classes;
                        let mut current_class_id = class_id;
                        let mut missing_parent = None;
                        loop {
                            if let Some(c) = classes.get(&current_class_id) {
                                for (k, (val, _vis)) in &c.default_properties {
                                    if !property_overrides.contains(k) {
                                        resolved_props.push((k.clone(), *val));
                                        property_overrides.insert(k.clone());
                                    }
                                }
                                if let Some(ref ext) = c.extends {
                                    if let Some(parent_idx) = fibre.engine_state.class_map.get(&normalize_name(ext)).map(|v| *v) {
                                        current_class_id = parent_idx;
                                        continue;
                                    } else {
                                        missing_parent = Some(ext.clone());
                                    }
                                }
                            }
                            break;
                        }

                        if let Some(parent_name) = missing_parent {
                            Self::push(fibre, class_name_val);
                            match Self::trigger_autoload(fibre, &parent_name) {
                                Ok(true) => {
                                    let caller_idx = fibre.frame_count - 2;
                                    fibre.frames[caller_idx].ip = start_ip;
                                    continue 'vm_loop;
                                }
                                Ok(false) => {
                                    Self::pop(fibre);
                                }
                                Err(e) => return ExecutionResult::Error(e),
                            }
                        }

                        for (k, val) in &resolved_props {
                            let cloned_val = Fibre::clone_default_property(&mut fibre.arena, val);
                            obj.properties.insert(k.clone(), cloned_val);
                        }
                        DEFAULT_PROPERTIES_L1_CACHE.with(|c| c.borrow_mut().insert(class_id, resolved_props));
                    }

                    let obj_ptr = fibre.arena.alloc_and_track(obj);
                    Self::push(fibre, Value::new_object_ptr(obj_ptr as *mut ()));
                }
                Opcode::PushUnpackMarker => {
                    Self::push(fibre, Value::new_unpack_marker());

                }
                Opcode::UnpackArray => {
                    let val = Self::pop(fibre);
                    let push_array_elements = |fibre: &mut Fibre, arr: &hyperion_core::types::array::PhpArray| {
                        if arr.is_packed {
                            for val in &arr.packed {
                                Self::push(fibre, *val);
                            }
                        } else {
                            for (key, val) in arr.elements.iter() {
                                match key {
                                    hyperion_core::types::array::ArrayKey::Int(_) => {
                                        Self::push(fibre, *val);
                                    }
                                    hyperion_core::types::array::ArrayKey::StringId(id) => {
                                        let s = hyperion_core::types::string_table::get_string_table()
                                            .get_string(*id)
                                            .unwrap_or_default();
                                        let s_ptr = fibre.arena.alloc_and_track(s) as *mut String;
                                        let name_val = Value::new_string_ptr(s_ptr as *mut ());
                                        Self::push(fibre, name_val);
                                        Self::push(fibre, *val);
                                        Self::push(fibre, Value::new_named_arg_marker());
                                    }
                                }
                            }
                        }
                    };
                    if val.is_array() {
                        let arr_ptr = val.as_array_ptr().unwrap() as *mut hyperion_core::types::array::PhpArray;
                        let arr = unsafe { &*arr_ptr };
                        push_array_elements(fibre, arr);
                    } else if val.is_object() {
                        let iter_res = Self::resolve_iterable(fibre, val);
                        match iter_res {
                            Ok(resolved) => {
                                if resolved.is_array() {
                                    let arr_ptr = resolved.as_array_ptr().unwrap() as *mut hyperion_core::types::array::PhpArray;
                                    let arr = unsafe { &*arr_ptr };
                                    push_array_elements(fibre, arr);
                                } else if resolved.is_object() {
                                    let obj_ptr = resolved.as_object_ptr().unwrap() as *mut hyperion_core::types::object::PhpObject;
                                    let obj = unsafe { &*obj_ptr };
                                    let class_id = Self::resolve_obj_class_id(fibre, obj);
                                    let is_gen = {
                                        let classes = &fibre.engine_state.classes;
                                        classes.get(&class_id).map(|c| c.name == "Generator").unwrap_or(false)
                                    };
                                    if is_gen {
                                        if let Some(gen_val) = obj.properties.get("__generator") {
                                            let gen_fibre_ptr = gen_val.as_object_ptr().unwrap() as *mut Fibre;
                                            let gen_fibre = unsafe { &mut *gen_fibre_ptr };
                                            if !gen_fibre.generator_started {
                                                gen_fibre.generator_started = true;
                                                let res = Self::step_generator_fibre(gen_fibre);
                                                match res {
                                                    ExecutionResult::Yielded(YieldReason::GeneratorYield) => {
                                                        gen_fibre.generator_valid = true;
                                                    }
                                                    ExecutionResult::Finished => {
                                                        gen_fibre.generator_valid = false;
                                                    }
                                                    ExecutionResult::Error(err) => return ExecutionResult::Error(err),
                                                    ExecutionResult::UncaughtException(err) => return ExecutionResult::UncaughtException(err),
                                                    _ => gen_fibre.generator_valid = false,
                                                }
                                            }
                                            while gen_fibre.generator_valid {
                                                let current_val = gen_fibre.generator_value;
                                                Self::push(fibre, current_val);
                                                let res = Self::step_generator_fibre(gen_fibre);
                                                match res {
                                                    ExecutionResult::Yielded(YieldReason::GeneratorYield) => {
                                                        gen_fibre.generator_valid = true;
                                                    }
                                                    ExecutionResult::Finished => {
                                                        gen_fibre.generator_valid = false;
                                                    }
                                                    ExecutionResult::Error(err) => return ExecutionResult::Error(err),
                                                    ExecutionResult::UncaughtException(err) => return ExecutionResult::UncaughtException(err),
                                                    _ => gen_fibre.generator_valid = false,
                                                }
                                            }
                                        }
                                    } else if Self::class_has_method(fibre, class_id, "current") {
                                        let mut valid = match Self::iterator_step(fibre, resolved, true) {
                                            Ok(v) => v,
                                            Err(e) => return ExecutionResult::Error(e),
                                        };
                                        while valid {
                                            let current_val = match Self::call_method_synchronously(fibre, resolved, "current", vec![]) {
                                                Ok(v) => v,
                                                Err(e) => return ExecutionResult::Error(e),
                                            };
                                            Self::push(fibre, current_val);
                                            valid = match Self::iterator_step(fibre, resolved, false) {
                                                Ok(v) => v,
                                                Err(e) => return ExecutionResult::Error(e),
                                            };
                                        }
                                    } else {
                                        for (_, v) in &obj.properties {
                                            Self::push(fibre, v.deref());
                                        }
                                    }
                                }
                            }
                            Err(e) => return ExecutionResult::Error(e),
                        }
                    } else {
                        return ExecutionResult::Error(format!("Only arrays and Traversables can be unpacked, got {:?}", val.get_type()));
                    }
                }
                Opcode::CallConstructUnpacked => {
                    if let Some(tracer) = &mut fibre.tracer {
                        if tracer.is_recording {
                            tracer.stop();
                        }
                    }
                    let mut positional_args = Vec::new();
                    let mut named_pairs = Vec::new();
                    while fibre.stack_top > 0 {
                        let val = Self::pop(fibre);
                        if val.is_unpack_marker() {
                            break;
                        }
                        if val.is_named_arg_marker() {
                            let arg_val = Self::pop(fibre);
                            let name_val = Self::pop(fibre);
                            named_pairs.push((name_val, arg_val));
                        } else {
                            positional_args.push(val);
                        }
                    }
                    let obj = Self::pop(fibre);

                    if let Some(obj_ptr) = obj.as_object_ptr() {
                        let obj_ref =
                            unsafe { &*(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
                        let class_id = obj_ref.class_id;
                        let func_ptr_opt =
                            Self::find_method_fast(fibre, class_id, "__construct");

                        let current_frame_idx = fibre.frame_count - 1;
                        match func_ptr_opt {
                            Ok(Some(func_ptr)) => {
                                let func = unsafe { &*func_ptr.0 };
                                let pos_forward: Vec<Value> = positional_args.iter().rev().cloned().collect();
                                let named_forward: Vec<(Value, Value)> = named_pairs.iter().rev().cloned().collect();
                                let (bound_args, var_packed) = Self::bind_call_arguments(&fibre.engine_state, func, &pos_forward, &named_forward, &fibre.arena);
                                let arity = bound_args.len();

                                let stack_window = fibre.stack_top;
                                Self::push(fibre, obj);

                                for arg in &bound_args {
                                    Self::push(fibre, *arg);
                                }

                                if fibre.frame_count >= fibre.frames.len() {
                                    return ExecutionResult::Error("Stack overflow".to_string());
                                }

                                fibre.frames[fibre.frame_count] = CallFrame {
                                    function: func_ptr,
                                    ip: 0,
                                    stack_window,
                                    called_class_id: class_id,
                                    return_override: Some(obj), eval_parent_stack_window: None,
                                    arity,
                                };
                                fibre.set_frame_args(fibre.frame_count, &bound_args);
                                fibre.frame_count += 1;
                                // Reserve slots for defaulted params and locals, exactly as
                                // CallConstruct does; skipping this lets the constructor's
                                // locals overwrite the caller's live stack.
                                Self::pad_missing_arguments_and_locals(fibre, arity, unsafe {
                                    &*func_ptr.0
                                }, &[], var_packed);
                                continue;
                            }
                            Ok(None) => {
                                Self::push(fibre, obj);
                            }
                            Err(parent_name) => {
                                Self::push(fibre, obj);
                                Self::push(fibre, Value::new_unpack_marker());
                                for arg in positional_args.iter().rev() {
                                    Self::push(fibre, *arg);
                                }
                                for (name, val) in named_pairs.iter().rev() {
                                    Self::push(fibre, *name);
                                    Self::push(fibre, *val);
                                    Self::push(fibre, Value::new_named_arg_marker());
                                }
                                fibre.frames[current_frame_idx].ip = start_ip;
                                match Self::trigger_autoload(fibre, &parent_name) {
                                    Ok(true) => continue 'vm_loop,
                                    Ok(false) => {
                                        if Self::find_class_id(fibre, &parent_name).is_some() {
                                            continue 'vm_loop;
                                        }
                                        return ExecutionResult::Error(format!(
                                            "Class '{}' not found",
                                            parent_name
                                        ))
                                    }
                                    Err(e) => return ExecutionResult::Error(e),
                                }
                            }
                        }
                    } else {
                        Self::push(fibre, obj);
                    }
                }
                Opcode::CallConstruct => {
                    if let Some(tracer) = &mut fibre.tracer {
                        if tracer.is_recording {
                            tracer.stop();
                        }
                    }
                    let arity = Self::read_byte(fibre) as usize;
                    let num_named = Self::read_byte(fibre) as usize;
                    let mut named_pairs: Vec<(Value, Value)> = Vec::new();
                    for _ in 0..num_named {
                        let val = Self::pop(fibre);
                        let name = Self::pop(fibre);
                        named_pairs.push((name, val));
                    }
                    let mut positional_args = Vec::new();
                    for _ in 0..(arity - num_named) {
                        positional_args.push(Self::pop(fibre));
                    }
                    let obj = Self::pop(fibre);

                    if let Some(obj_ptr) = obj.as_object_ptr() {
                        let obj_ref =
                            unsafe { &*(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
                        let class_id = obj_ref.class_id;
                        let func_ptr_opt =
                            Self::find_method_fast(fibre, class_id, "__construct");

                        let current_frame_idx = fibre.frame_count - 1;
                        match func_ptr_opt {
                            Ok(Some(func_ptr)) => {
                                let func = unsafe { &*func_ptr.0 }; debug_println!("autoload: pushed function {}", func.name);
                                let pos_forward: Vec<Value> = positional_args.iter().rev().cloned().collect();
                                let named_forward: Vec<(Value, Value)> = named_pairs.iter().rev().cloned().collect();
                                let (bound_args, var_packed) = Self::bind_call_arguments(&fibre.engine_state, func, &pos_forward, &named_forward, &fibre.arena);
                                let actual_arity = bound_args.len();
                                let stack_window = fibre.stack_top;
                                Self::push(fibre, obj);

                                for arg in bound_args {
                                    Self::push(fibre, arg);
                                }

                                if fibre.frame_count >= fibre.frames.len() {
                                    return ExecutionResult::Error("Stack overflow".to_string());
                                }

                                fibre.frames[fibre.frame_count] = CallFrame {
                                    function: func_ptr,
                                    ip: 0,
                                    stack_window,
                                    called_class_id: class_id,
                                    return_override: Some(obj),
                                    eval_parent_stack_window: None,
                                    arity: actual_arity,
                                };
                                fibre.set_frame_args(fibre.frame_count, &pos_forward);
                                fibre.frame_count += 1;
                                Self::pad_missing_arguments_and_locals(fibre, actual_arity, unsafe {
                                    &*func_ptr.0
                                }, &[], var_packed);
                                continue; // Wait for frame to finish
                            }
                            Ok(None) => {
                                // Missing __construct, PHP just returns the object (pushed at line 2945)
                            }
                            Err(parent_name) => {
                                Self::push(fibre, obj);
                                for arg in positional_args.iter().rev() {
                                    Self::push(fibre, *arg);
                                }
                                for (name, val) in named_pairs.iter().rev() {
                                    Self::push(fibre, *name);
                                    Self::push(fibre, *val);
                                }
                                fibre.frames[current_frame_idx].ip = start_ip;
                                match Self::trigger_autoload(fibre, &parent_name) {
                                    Ok(true) => continue 'vm_loop,
                                    Ok(false) => {
                                        if Self::find_class_id(fibre, &parent_name).is_some() {
                                            continue 'vm_loop;
                                        }
                                        return ExecutionResult::Error(format!(
                                            "Class '{}' not found",
                                            parent_name
                                        ))
                                    }
                                    Err(e) => return ExecutionResult::Error(e),
                                }
                            }
                        }
                    } 
                    // No __construct or not an object, just push the object back
                    Self::push(fibre, obj);
                }
                Opcode::GetProperty => {
                    let name_idx = Self::read_short(fibre);
                    let obj = Self::pop(fibre).deref();

                    if let Some(tracer) = fibre.tracer.as_mut() {
                        if tracer.is_recording {
                            let obj_idx = fibre.stack_top;
                            let dest_idx = fibre.stack_top;
                            let frame = fibre.frames[fibre.frame_count - 1];
                            let func = unsafe { &*frame.function.0 };
                            let chunk_ptr = &func.chunk as *const hyperion_bytecode::Chunk as u64;

                            tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardType {
                                src: obj_idx,
                                expected_type_tag: hyperion_core::memory::nan_box::QNAN
                                    | hyperion_core::memory::nan_box::TAG_OBJ,
                                bailout_ip: fibre.frames[fibre.frame_count - 1].ip - 1,
                                bailout_stack_top: fibre.stack_top + 1,
                            });

                            tracer
                                .trace
                                .push(hyperion_jit::tracer::TraceIR::FetchObjectProperty {
                                    dest: dest_idx,
                                    obj: obj_idx,
                                    name_idx,
                                    chunk_ptr,
                                    fn_ptr: jit_fetch_object_property as *const () as u64,
                                });
                        }
                    }

                    if let Some(obj_ptr) = obj.as_object_ptr() {
                        let php_obj = unsafe {
                            &*(obj_ptr as *const hyperion_core::types::object::PhpObject)
                        };
                        let frame = fibre.frames[fibre.frame_count - 1];
                        let func = unsafe { &*frame.function.0 };
                        let name_val = func.chunk.constants[name_idx as usize];

                        if let Some(name_str_ptr) = name_val.as_string_ptr() {
                            let name_str = unsafe { &*(name_str_ptr as *const String) };
                            if let Some(val) = php_obj.properties.get(name_str) {
                                Self::push(fibre, *val);
                            } else {
                                let class_id = Self::resolve_obj_class_id(fibre, php_obj);
                                if let Ok(Some(_)) = Self::find_method_fast(fibre, class_id, "__get") {
                                    let guard_key = (obj_ptr as usize, name_str.clone(), 1u8);
                                    if fibre.magic_guards.contains(&guard_key) {
                                        Self::push(fibre, Value::null());
                                    } else {
                                        fibre.magic_guards.insert(guard_key.clone());
                                        let call_res = Self::call_method_synchronously(fibre, obj, "__get", vec![name_val]);
                                        fibre.magic_guards.remove(&guard_key);
                                        match call_res {
                                            Ok(res) => Self::push(fibre, res),
                                            Err(e) => return ExecutionResult::Error(e),
                                        }
                                    }
                                } else {
                                    Self::push(fibre, Value::null());
                                }
                            }
                        } else {
                            Self::push(fibre, Value::null());
                        }
                    } else {
                        Self::push(fibre, Value::null());
                    }
                }
                Opcode::GetPropertyDynamic => {
                    let prop_name = Self::pop(fibre);
                    let obj = Self::pop(fibre);
                    
                    if let Some(obj_ptr) = obj.as_object_ptr() {
                        let php_obj = unsafe {
                            &*(obj_ptr as *const hyperion_core::types::object::PhpObject)
                        };
                        let class_id = Self::resolve_obj_class_id(fibre, php_obj);
                        if let Some(name_str_ptr) = prop_name.as_string_ptr() {
                            let name_str = unsafe { &*(name_str_ptr as *const String) };
                            if let Some(val) = php_obj.properties.get(name_str) {
                                Self::push(fibre, *val);
                            } else if Self::class_has_method(fibre, class_id, "__get") {
                                let guard_key = (obj_ptr as usize, name_str.clone(), 1u8);
                                if fibre.magic_guards.contains(&guard_key) {
                                    Self::push(fibre, Value::null());
                                } else {
                                    fibre.magic_guards.insert(guard_key.clone());
                                    let call_res = Self::call_method_synchronously(fibre, obj, "__get", vec![prop_name]);
                                    fibre.magic_guards.remove(&guard_key);
                                    match call_res {
                                        Ok(res) => Self::push(fibre, res),
                                        Err(e) => return ExecutionResult::Error(e),
                                    }
                                }
                            } else {
                                let class_name = if let Some(ref name) = php_obj.class_name {
                                    name.clone()
                                } else {
                                    fibre.engine_state.classes.get(&class_id).map(|c| c.name.clone()).unwrap_or_default()
                                };
                                debug_println!("DEBUG: GetPropertyDynamic missing property '{}' on class '{}'", name_str, class_name);
                                Self::push(fibre, Value::null());
                            }
                        } else {
                            Self::push(fibre, Value::null());
                        }
                    } else {
                        Self::push(fibre, Value::null());
                    }
                }
                Opcode::SetProperty => {
                    let name_idx = Self::read_short(fibre);
                    let raw_val = Self::pop(fibre);
                    let obj = Self::pop(fibre).deref();

                    let val = if raw_val.deref().is_array() {
                        Self::deep_copy_value(fibre, raw_val.deref())
                    } else {
                        raw_val.deref()
                    };

                    if let Some(tracer) = fibre.tracer.as_mut() {
                        if tracer.is_recording {
                            let obj_idx = fibre.stack_top;
                            let val_idx = fibre.stack_top + 1;
                            let dest_idx = fibre.stack_top; // The push(val) will land here

                            let frame = fibre.frames[fibre.frame_count - 1];
                            let func = unsafe { &*frame.function.0 };
                            let chunk_ptr = &func.chunk as *const hyperion_bytecode::Chunk as u64;

                            tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardType {
                                src: obj_idx,
                                expected_type_tag: hyperion_core::memory::nan_box::QNAN
                                    | hyperion_core::memory::nan_box::TAG_OBJ,
                                bailout_ip: fibre.frames[fibre.frame_count - 1].ip - 1,
                                bailout_stack_top: fibre.stack_top + 2, // Restore stack to before both pops
                            });

                            tracer
                                .trace
                                .push(hyperion_jit::tracer::TraceIR::SetObjectProperty {
                                    dest: dest_idx,
                                    obj: obj_idx,
                                    val: val_idx,
                                    name_idx,
                                    chunk_ptr,
                                    fn_ptr: jit_set_object_property as *const () as u64,
                                });
                        }
                    }

                    if let Some(obj_ptr) = obj.as_object_ptr() {
                        let php_obj = unsafe {
                            &mut *(obj_ptr as *mut hyperion_core::types::object::PhpObject)
                        };
                        let frame = fibre.frames[fibre.frame_count - 1];
                        let func = unsafe { &*frame.function.0 };
                        let name_val = func.chunk.constants[name_idx as usize];

                        if let Some(name_str_ptr) = name_val.as_string_ptr() {
                            let name_str = unsafe { &*(name_str_ptr as *const String) };
                            if php_obj.properties.contains_key(name_str) {
                                php_obj.properties.insert(name_str.clone(), val);
                            } else {
                                let class_id = Self::resolve_obj_class_id(fibre, php_obj);
                                if Self::class_has_declared_property(fibre, class_id, name_str) {
                                    php_obj.properties.insert(name_str.clone(), val);
                                } else if let Ok(Some(_)) = Self::find_method_fast(fibre, class_id, "__set") {
                                    let guard_key = (obj_ptr as usize, name_str.clone(), 2u8);
                                    if !fibre.magic_guards.contains(&guard_key) {
                                        fibre.magic_guards.insert(guard_key.clone());
                                        let call_res = Self::call_method_synchronously(fibre, obj, "__set", vec![name_val, val]);
                                        fibre.magic_guards.remove(&guard_key);
                                        match call_res {
                                            Ok(_) => {},
                                            Err(e) => return ExecutionResult::Error(e),
                                        }
                                    } else {
                                        php_obj.properties.insert(name_str.clone(), val);
                                    }
                                } else {
                                    php_obj.properties.insert(name_str.clone(), val);
                                }
                            }
                        }
                    }

                    // SetProperty pushes the value back
                    Self::push(fibre, val);
                }
                Opcode::SetPropertyDynamic => {
                    let raw_val = Self::pop(fibre);
                    let prop_name = Self::pop(fibre);
                    let obj = Self::pop(fibre).deref();

                    let val = if raw_val.deref().is_array() {
                        Self::deep_copy_value(fibre, raw_val.deref())
                    } else {
                        raw_val.deref()
                    };

                    if let Some(obj_ptr) = obj.as_object_ptr() {
                        let php_obj = unsafe {
                            &mut *(obj_ptr as *mut hyperion_core::types::object::PhpObject)
                        };
                        let class_id = Self::resolve_obj_class_id(fibre, php_obj);
                        let name_str = match prop_name.as_string_ptr() {
                            Some(s_ptr) => unsafe { (*(s_ptr as *const String)).clone() },
                            None => format!("{:?}", prop_name),
                        };
                        if php_obj.properties.contains_key(&name_str) || Self::class_has_declared_property(fibre, class_id, &name_str) {
                            php_obj.properties.insert(name_str, val);
                        } else if Self::class_has_method(fibre, class_id, "__set") {
                            let guard_key = (obj_ptr as usize, name_str.clone(), 2u8);
                            if !fibre.magic_guards.contains(&guard_key) {
                                fibre.magic_guards.insert(guard_key.clone());
                                let call_res = Self::call_method_synchronously(fibre, obj, "__set", vec![prop_name, val]);
                                fibre.magic_guards.remove(&guard_key);
                                match call_res {
                                    Ok(_) => {},
                                    Err(e) => return ExecutionResult::Error(e),
                                }
                            } else {
                                php_obj.properties.insert(name_str, val);
                            }
                        } else {
                            php_obj.properties.insert(name_str, val);
                        }
                    }

                    Self::push(fibre, val);
                }
                Opcode::GetStatic => {
                    let class_idx = Self::read_short(fibre);
                    let prop_idx = Self::read_short(fibre);

                    let frame = fibre.frames[fibre.frame_count - 1];
                    let func = unsafe { &*frame.function.0 };
                    let class_val = func.chunk.constants[class_idx as usize];
                    let prop_val = func.chunk.constants[prop_idx as usize];
                    let raw_class_name =
                        unsafe { &*(class_val.as_string_ptr().unwrap() as *const String) };
                    let class_name = Self::resolve_dynamic_class_name(fibre, raw_class_name);
                    let prop_name =
                        unsafe { &*(prop_val.as_string_ptr().unwrap() as *const String) };

                    let mut val = fibre.get_static_property(&class_name, prop_name)
                        .or_else(|| {
                            let full_const = format!("{}::{}", class_name, prop_name);
                            let constants = &fibre.engine_state.constants;
                            resolve_constant(&constants, &full_const)
                        });
                    if val.is_none() {
                        fibre.trigger_autoload_sync(&class_name);
                        val = fibre.get_static_property(&class_name, prop_name)
                            .or_else(|| {
                                let full_const = format!("{}::{}", class_name, prop_name);
                                let constants = &fibre.engine_state.constants;
                                resolve_constant(&constants, &full_const)
                            });
                    }
                    let val = val.unwrap_or(Value::null());
                    Self::push(fibre, val);

                }
                Opcode::SetStatic => {
                    let class_idx = Self::read_short(fibre);
                    let prop_idx = Self::read_short(fibre);
                    let raw_val = Self::pop(fibre);
                    let val = if raw_val.deref().is_array() {
                        Self::deep_copy_value(fibre, raw_val.deref())
                    } else {
                        raw_val.deref()
                    };

                    let frame = fibre.frames[fibre.frame_count - 1];
                    let func = unsafe { &*frame.function.0 };
                    let class_val = func.chunk.constants[class_idx as usize];
                    let prop_val = func.chunk.constants[prop_idx as usize];
                    let raw_class_name =
                        unsafe { &*(class_val.as_string_ptr().unwrap() as *const String) };
                    let class_name = Self::resolve_dynamic_class_name(fibre, raw_class_name);
                    let prop_name =
                        unsafe { &*(prop_val.as_string_ptr().unwrap() as *const String) };
                    fibre.set_static_property(&class_name, prop_name, val);
                    Self::push(fibre, val);
                }
                Opcode::Not => {
                    let val = Self::pop(fibre);
                    Self::push(fibre, Value::new_bool(!val.is_truthy()));
                }


                Opcode::CreateGenerator => {
                    let parent_frame =
                        fibre.frames[fibre.frame_count - 1];
                    let func = unsafe { &*parent_frame.function.0 };
                    let num_locals = func.chunk.code[1] as usize;

                    let child_id = fibre.id ^ 0x5555_5555_5555_5555;
                    let mut child = Box::new(Fibre::new(
                        child_id,
                        parent_frame.function,
                        fibre.engine_state.clone(),
                    ));
                    child.autoloaders = fibre.autoloaders.clone();
                    child.static_properties = std::sync::Mutex::new(fibre.static_properties.lock().unwrap().clone());

                    for i in 0..num_locals {
                        child.stack[i] = fibre.stack[parent_frame.stack_window + i];
                    }
                    child.stack_top = num_locals;

                    child.frames[0].ip = parent_frame.ip;
                    child.frames[0].arity = parent_frame.arity;
                    child.frames[0].called_class_id = parent_frame.called_class_id;

                    child.generator_started = false;
                    child.generator_valid = true;
                    child.generator_yield_counter = 0;
                    child.is_generator = true;

                    let generator_class_id = Self::find_class_id(fibre, "Generator")
                        .unwrap_or_else(|| {
                            fibre.engine_state.classes.iter()
                                .find(|c| c.name == "Generator")
                                .map(|c| *c.key())
                                .expect("Generator class not registered")
                        });

                    let obj_ptr = fibre.arena.alloc_object(generator_class_id);
                    let obj = unsafe { &mut *obj_ptr };

                    let child_ptr: *mut Fibre = Box::into_raw(child);
                    // Track for cleanup when arena resets — reconstructs the Box on drop
                    fibre.arena.track_boxed(child_ptr);
                    obj.properties.insert(
                        "__generator".to_string(),
                        Value::new_object_ptr(child_ptr as *mut ()),
                    );

                    fibre.frame_count -= 1;
                    fibre.stack_top = parent_frame.stack_window;

                    let obj_val = Value::new_object_ptr(obj_ptr as *mut ());
                    Self::push(fibre, obj_val);

                    continue;
                }
                Opcode::Yield => {
                    if fibre.is_generator {
                        let val = Self::pop(fibre);
                        let mut key = Self::pop(fibre);
                        if key.is_null() {
                            key = Value::new_int(fibre.generator_yield_counter as i32);
                            fibre.generator_yield_counter += 1;
                        }
                        fibre.generator_key = key;
                        fibre.generator_value = val;

                        Self::push(fibre, Value::null());

                        return ExecutionResult::Yielded(YieldReason::GeneratorYield);
                    } else {
                        fibre.suspend();
                        return ExecutionResult::Yielded(YieldReason::Network);
                    }
                }
                Opcode::Constant => {
                    let idx = Self::read_short(fibre) as usize;
                    let frame = fibre.frames[fibre.frame_count - 1];
                    let func = unsafe { &*frame.function.0 };
                    if idx >= func.chunk.constants.len() {
                        for t in &fibre.trace_log {
                            debug_eprintln!("{}", t);
                        }
                        debug_eprintln!(
                            "PANIC IMMINENT: func={}, constants_len={}, requested_idx={}",
                            func.name,
                            func.chunk.constants.len(),
                            idx
                        );
                    }
                    let val = func.chunk.constants[idx];

                    if let Some(tracer) = &mut fibre.tracer {
                        if tracer.is_recording {
                            tracer.trace.push(hyperion_jit::tracer::TraceIR::Constant {
                                dest: fibre.stack_top,
                                value: val.0,
                            });
                        }
                    }

                    let val_to_push = if val.deref().is_array() {
                        Self::deep_copy_value(fibre, val.deref())
                    } else {
                        val
                    };
                    Self::push(fibre, val_to_push);
                }
                Opcode::FetchConstant => {
                    let idx = Self::read_short(fibre) as usize;
                    let frame = fibre.frames[fibre.frame_count - 1];
                    let func = unsafe { &*frame.function.0 };
                    let val = func.chunk.constants[idx];

                    if let Some(name_ptr) = val.as_string_ptr() {
                        let name_str = unsafe { &*(name_ptr as *const String) };
                        let const_val_opt = {
                            let constants = &fibre.engine_state.constants;
                            resolve_constant(&constants, name_str)
                        };
                        if let Some(const_val) = const_val_opt {
                            let const_val_to_push = if const_val.deref().is_array() {
                                Self::deep_copy_value(fibre, const_val.deref())
                            } else {
                                const_val
                            };
                            Self::push(fibre, const_val_to_push);
                        } else {
                            // Fallback for barewords in PHP < 8.0: treat as string.
                            // The leading `\` is namespace syntax rather than part
                            // of the word, so it is dropped here too.
                            match name_str.strip_prefix('\\') {
                                Some(bare) => {
                                    let s = fibre.arena.alloc_and_track(bare.to_string());
                                    Self::push(fibre, Value::new_string_ptr(s as *mut ()));
                                }
                                None => Self::push(fibre, val),
                            }
                        }
                    } else {
                        Self::push(fibre, val);
                    }
                }
                Opcode::DeclareConst => {
                    let idx = Self::read_short(fibre) as usize;
                    let frame = fibre.frames[fibre.frame_count - 1];
                    let func = unsafe { &*frame.function.0 };
                    let name_val = func.chunk.constants[idx];
                    let value = Self::pop(fibre);
                    if let Some(name_ptr) = name_val.as_string_ptr() {
                        let name = unsafe { &*(name_ptr as *const String) };
                        let persistent_val = Fibre::clone_value_to_global(&fibre.engine_state, &value);
                        let constants = &fibre.engine_state.constants;
                        // A redeclaration is ignored rather than overwriting, the
                        // same rule `define()` follows. A file included twice
                        // would otherwise clobber the first value.
                        constants.entry(name.clone()).or_insert(persistent_val);
                    }
                }
                Opcode::Add => {
                    if let Some(tracer) = &mut fibre.tracer {
                        if tracer.is_recording {
                            let a = fibre.stack[fibre.stack_top - 2];
                            let b = fibre.stack[fibre.stack_top - 1];
                            if a.is_int() && b.is_int() {
                                let a_tag = a.0 & 0xFFFF000000000000;
                                let b_tag = b.0 & 0xFFFF000000000000;

                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardType {
                                    src: fibre.stack_top - 2,
                                    expected_type_tag: a_tag,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardType {
                                    src: fibre.stack_top - 1,
                                    expected_type_tag: b_tag,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::Add {
                                    dest: fibre.stack_top - 2,
                                    left: fibre.stack_top - 2,
                                    right: fibre.stack_top - 1,
                                });
                            } else if a.is_float() && b.is_float() {
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardFloat {
                                    src: fibre.stack_top - 2,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardFloat {
                                    src: fibre.stack_top - 1,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::FloatAdd {
                                    dest: fibre.stack_top - 2,
                                    left: fibre.stack_top - 2,
                                    right: fibre.stack_top - 1,
                                });
                            } else if a.is_int() && b.is_float() {
                                let a_tag = a.0 & 0xFFFF000000000000;
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardType {
                                    src: fibre.stack_top - 2,
                                    expected_type_tag: a_tag,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardFloat {
                                    src: fibre.stack_top - 1,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::IntToFloat {
                                    dest: fibre.stack_top - 2,
                                    src: fibre.stack_top - 2,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::FloatAdd {
                                    dest: fibre.stack_top - 2,
                                    left: fibre.stack_top - 2,
                                    right: fibre.stack_top - 1,
                                });
                            } else if a.is_float() && b.is_int() {
                                let b_tag = b.0 & 0xFFFF000000000000;
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardFloat {
                                    src: fibre.stack_top - 2,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardType {
                                    src: fibre.stack_top - 1,
                                    expected_type_tag: b_tag,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::IntToFloat {
                                    dest: fibre.stack_top - 1,
                                    src: fibre.stack_top - 1,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::FloatAdd {
                                    dest: fibre.stack_top - 2,
                                    left: fibre.stack_top - 2,
                                    right: fibre.stack_top - 1,
                                });
                            } else {
                                tracer.stop();
                            }
                        }
                    }

                    let b = Self::pop(fibre).deref();
                    let a = Self::pop(fibre).deref();
                    let (a_int, a_flt) = if let Some(i) = a.as_int() {
                        (Some(i), None)
                    } else if let Some(f) = a.as_float() {
                        (None, Some(f))
                    } else if a.is_null() {
                        (Some(0), None)
                    } else if let Some(b_val) = a.as_bool() {
                        (Some(if b_val { 1 } else { 0 }), None)
                    } else if let Some(str_ptr) = a.as_string_ptr() {
                        let s = unsafe { &*(str_ptr as *const String) };
                        if let Ok(i) = s.parse::<i32>() {
                            (Some(i), None)
                        } else if let Ok(f) = s.parse::<f64>() {
                            (None, Some(f))
                        } else {
                            (Some(0), None)
                        }
                    } else {
                        (None, None)
                    };
                    let (b_int, b_flt) = if let Some(i) = b.as_int() {
                        (Some(i), None)
                    } else if let Some(f) = b.as_float() {
                        (None, Some(f))
                    } else if b.is_null() {
                        (Some(0), None)
                    } else if let Some(b_val) = b.as_bool() {
                        (Some(if b_val { 1 } else { 0 }), None)
                    } else if let Some(str_ptr) = b.as_string_ptr() {
                        let s = unsafe { &*(str_ptr as *const String) };
                        if let Ok(i) = s.parse::<i32>() {
                            (Some(i), None)
                        } else if let Ok(f) = s.parse::<f64>() {
                            (None, Some(f))
                        } else {
                            (Some(0), None)
                        }
                    } else {
                        (None, None)
                    };

                    if let (Some(ai), Some(bi)) = (a_int, b_int) {
                        Self::push(fibre, Value::new_int(ai + bi));
                    } else if let (Some(af), Some(bf)) = (a_flt.or(a_int.map(|i| i as f64)), b_flt.or(b_int.map(|i| i as f64))) {
                        Self::push(fibre, Value::new_float(af + bf));
                    } else if let (Some(arr_a_ptr), Some(arr_b_ptr)) =
                        (a.as_array_ptr(), b.as_array_ptr())
                    {
                        let arr_a = unsafe {
                            &*(arr_a_ptr as *const hyperion_core::types::array::PhpArray)
                        };
                        let arr_b = unsafe {
                            &*(arr_b_ptr as *const hyperion_core::types::array::PhpArray)
                        };
                        let mut new_arr = hyperion_core::types::array::PhpArray::new();
                        for (k, v) in arr_a.elements.iter() {
                            new_arr.insert_key(k.clone(), *v);
                        }
                        for (k, v) in arr_b.elements.iter() {
                            if !new_arr.elements.contains_key(k) {
                                new_arr.insert_key(k.clone(), *v);
                            }
                        }
                        let new_arr_ptr = fibre.arena.alloc_and_track(new_arr);
                        Self::push(fibre, Value::new_array_ptr(new_arr_ptr as *mut ()));
                    } else {
                        Self::push(fibre, Value::null());
                    }
                }
                Opcode::Subtract => {
                    if let Some(tracer) = &mut fibre.tracer {
                        if tracer.is_recording {
                            let a = fibre.stack[fibre.stack_top - 2];
                            let b = fibre.stack[fibre.stack_top - 1];
                            if a.is_int() && b.is_int() {
                                let a_tag = a.0 & 0xFFFF000000000000;
                                let b_tag = b.0 & 0xFFFF000000000000;

                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardType {
                                    src: fibre.stack_top - 2,
                                    expected_type_tag: a_tag,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardType {
                                    src: fibre.stack_top - 1,
                                    expected_type_tag: b_tag,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::Subtract {
                                    dest: fibre.stack_top - 2,
                                    left: fibre.stack_top - 2,
                                    right: fibre.stack_top - 1,
                                });
                            } else if a.is_float() && b.is_float() {
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardFloat {
                                    src: fibre.stack_top - 2,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardFloat {
                                    src: fibre.stack_top - 1,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::FloatSubtract {
                                    dest: fibre.stack_top - 2,
                                    left: fibre.stack_top - 2,
                                    right: fibre.stack_top - 1,
                                });
                            } else if a.is_int() && b.is_float() {
                                let a_tag = a.0 & 0xFFFF000000000000;
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardType {
                                    src: fibre.stack_top - 2,
                                    expected_type_tag: a_tag,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardFloat {
                                    src: fibre.stack_top - 1,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::IntToFloat {
                                    dest: fibre.stack_top - 2,
                                    src: fibre.stack_top - 2,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::FloatSubtract {
                                    dest: fibre.stack_top - 2,
                                    left: fibre.stack_top - 2,
                                    right: fibre.stack_top - 1,
                                });
                            } else if a.is_float() && b.is_int() {
                                let b_tag = b.0 & 0xFFFF000000000000;
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardFloat {
                                    src: fibre.stack_top - 2,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardType {
                                    src: fibre.stack_top - 1,
                                    expected_type_tag: b_tag,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::IntToFloat {
                                    dest: fibre.stack_top - 1,
                                    src: fibre.stack_top - 1,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::FloatSubtract {
                                    dest: fibre.stack_top - 2,
                                    left: fibre.stack_top - 2,
                                    right: fibre.stack_top - 1,
                                });
                            } else {
                                tracer.stop();
                            }
                        }
                    }
                    let b = Self::pop(fibre).deref();
                    let a = Self::pop(fibre).deref();
                    let (a_int, a_flt) = if let Some(i) = a.as_int() {
                        (Some(i), None)
                    } else if let Some(f) = a.as_float() {
                        (None, Some(f))
                    } else if a.is_null() {
                        (Some(0), None)
                    } else if let Some(b_val) = a.as_bool() {
                        (Some(if b_val { 1 } else { 0 }), None)
                    } else if let Some(str_ptr) = a.as_string_ptr() {
                        let s = unsafe { &*(str_ptr as *const String) };
                        if let Ok(i) = s.parse::<i32>() {
                            (Some(i), None)
                        } else if let Ok(f) = s.parse::<f64>() {
                            (None, Some(f))
                        } else {
                            (Some(0), None)
                        }
                    } else {
                        (None, None)
                    };
                    let (b_int, b_flt) = if let Some(i) = b.as_int() {
                        (Some(i), None)
                    } else if let Some(f) = b.as_float() {
                        (None, Some(f))
                    } else if b.is_null() {
                        (Some(0), None)
                    } else if let Some(b_val) = b.as_bool() {
                        (Some(if b_val { 1 } else { 0 }), None)
                    } else if let Some(str_ptr) = b.as_string_ptr() {
                        let s = unsafe { &*(str_ptr as *const String) };
                        if let Ok(i) = s.parse::<i32>() {
                            (Some(i), None)
                        } else if let Ok(f) = s.parse::<f64>() {
                            (None, Some(f))
                        } else {
                            (Some(0), None)
                        }
                    } else {
                        (None, None)
                    };

                    if let (Some(ai), Some(bi)) = (a_int, b_int) {
                        Self::push(fibre, Value::new_int(ai - bi));
                    } else if let (Some(af), Some(bf)) = (a_flt.or(a_int.map(|i| i as f64)), b_flt.or(b_int.map(|i| i as f64))) {
                        Self::push(fibre, Value::new_float(af - bf));
                    } else {
                        Self::push(fibre, Value::null());
                    }
                }
                Opcode::Multiply => {
                    if let Some(tracer) = &mut fibre.tracer {
                        if tracer.is_recording {
                            let a = fibre.stack[fibre.stack_top - 2];
                            let b = fibre.stack[fibre.stack_top - 1];
                            if a.is_int() && b.is_int() {
                                let a_tag = a.0 & 0xFFFF000000000000;
                                let b_tag = b.0 & 0xFFFF000000000000;

                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardType {
                                    src: fibre.stack_top - 2,
                                    expected_type_tag: a_tag,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardType {
                                    src: fibre.stack_top - 1,
                                    expected_type_tag: b_tag,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::Multiply {
                                    dest: fibre.stack_top - 2,
                                    left: fibre.stack_top - 2,
                                    right: fibre.stack_top - 1,
                                });
                            } else if a.is_float() && b.is_float() {
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardFloat {
                                    src: fibre.stack_top - 2,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardFloat {
                                    src: fibre.stack_top - 1,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::FloatMultiply {
                                    dest: fibre.stack_top - 2,
                                    left: fibre.stack_top - 2,
                                    right: fibre.stack_top - 1,
                                });
                            } else if a.is_int() && b.is_float() {
                                let a_tag = a.0 & 0xFFFF000000000000;
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardType {
                                    src: fibre.stack_top - 2,
                                    expected_type_tag: a_tag,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardFloat {
                                    src: fibre.stack_top - 1,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::IntToFloat {
                                    dest: fibre.stack_top - 2,
                                    src: fibre.stack_top - 2,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::FloatMultiply {
                                    dest: fibre.stack_top - 2,
                                    left: fibre.stack_top - 2,
                                    right: fibre.stack_top - 1,
                                });
                            } else if a.is_float() && b.is_int() {
                                let b_tag = b.0 & 0xFFFF000000000000;
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardFloat {
                                    src: fibre.stack_top - 2,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardType {
                                    src: fibre.stack_top - 1,
                                    expected_type_tag: b_tag,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::IntToFloat {
                                    dest: fibre.stack_top - 1,
                                    src: fibre.stack_top - 1,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::FloatMultiply {
                                    dest: fibre.stack_top - 2,
                                    left: fibre.stack_top - 2,
                                    right: fibre.stack_top - 1,
                                });
                            } else {
                                tracer.stop();
                            }
                        }
                    }
                    let b = Self::pop(fibre).deref();
                    let a = Self::pop(fibre).deref();
                    let (a_int, a_flt) = if let Some(i) = a.as_int() {
                        (Some(i), None)
                    } else if let Some(f) = a.as_float() {
                        (None, Some(f))
                    } else if a.is_null() {
                        (Some(0), None)
                    } else if let Some(b_val) = a.as_bool() {
                        (Some(if b_val { 1 } else { 0 }), None)
                    } else if let Some(str_ptr) = a.as_string_ptr() {
                        let s = unsafe { &*(str_ptr as *const String) };
                        if let Ok(i) = s.parse::<i32>() {
                            (Some(i), None)
                        } else if let Ok(f) = s.parse::<f64>() {
                            (None, Some(f))
                        } else {
                            (Some(0), None)
                        }
                    } else {
                        (None, None)
                    };
                    let (b_int, b_flt) = if let Some(i) = b.as_int() {
                        (Some(i), None)
                    } else if let Some(f) = b.as_float() {
                        (None, Some(f))
                    } else if b.is_null() {
                        (Some(0), None)
                    } else if let Some(b_val) = b.as_bool() {
                        (Some(if b_val { 1 } else { 0 }), None)
                    } else if let Some(str_ptr) = b.as_string_ptr() {
                        let s = unsafe { &*(str_ptr as *const String) };
                        if let Ok(i) = s.parse::<i32>() {
                            (Some(i), None)
                        } else if let Ok(f) = s.parse::<f64>() {
                            (None, Some(f))
                        } else {
                            (Some(0), None)
                        }
                    } else {
                        (None, None)
                    };

                    if let (Some(ai), Some(bi)) = (a_int, b_int) {
                        Self::push(fibre, Value::new_int(ai * bi));
                    } else if let (Some(af), Some(bf)) = (a_flt.or(a_int.map(|i| i as f64)), b_flt.or(b_int.map(|i| i as f64))) {
                        Self::push(fibre, Value::new_float(af * bf));
                    } else {
                        Self::push(fibre, Value::null());
                    }
                }
                Opcode::Divide => {
                    if let Some(tracer) = &mut fibre.tracer {
                        if tracer.is_recording {
                            let a = fibre.stack[fibre.stack_top - 2];
                            let b = fibre.stack[fibre.stack_top - 1];
                            if a.is_int() && b.is_int() {
                                let a_tag = a.0 & 0xFFFF000000000000;
                                let b_tag = b.0 & 0xFFFF000000000000;

                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardType {
                                    src: fibre.stack_top - 2,
                                    expected_type_tag: a_tag,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardType {
                                    src: fibre.stack_top - 1,
                                    expected_type_tag: b_tag,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::Divide {
                                    dest: fibre.stack_top - 2,
                                    left: fibre.stack_top - 2,
                                    right: fibre.stack_top - 1,
                                });
                            } else if a.is_float() && b.is_float() {
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardFloat {
                                    src: fibre.stack_top - 2,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardFloat {
                                    src: fibre.stack_top - 1,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::FloatDivide {
                                    dest: fibre.stack_top - 2,
                                    left: fibre.stack_top - 2,
                                    right: fibre.stack_top - 1,
                                });
                            } else if a.is_int() && b.is_float() {
                                let a_tag = a.0 & 0xFFFF000000000000;
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardType {
                                    src: fibre.stack_top - 2,
                                    expected_type_tag: a_tag,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardFloat {
                                    src: fibre.stack_top - 1,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::IntToFloat {
                                    dest: fibre.stack_top - 2,
                                    src: fibre.stack_top - 2,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::FloatDivide {
                                    dest: fibre.stack_top - 2,
                                    left: fibre.stack_top - 2,
                                    right: fibre.stack_top - 1,
                                });
                            } else if a.is_float() && b.is_int() {
                                let b_tag = b.0 & 0xFFFF000000000000;
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardFloat {
                                    src: fibre.stack_top - 2,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardType {
                                    src: fibre.stack_top - 1,
                                    expected_type_tag: b_tag,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::IntToFloat {
                                    dest: fibre.stack_top - 1,
                                    src: fibre.stack_top - 1,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::FloatDivide {
                                    dest: fibre.stack_top - 2,
                                    left: fibre.stack_top - 2,
                                    right: fibre.stack_top - 1,
                                });
                            } else {
                                tracer.stop();
                            }
                        }
                    }
                    let b = Self::pop(fibre).deref();
                    let a = Self::pop(fibre).deref();
                    let (a_f, b_f) = match (a.as_int(), a.as_float(), b.as_int(), b.as_float()) {
                        (Some(ai), _, Some(bi), _) => (ai as f64, bi as f64),
                        (_, Some(af), _, Some(bf)) => (af, bf),
                        (Some(ai), _, _, Some(bf)) => (ai as f64, bf),
                        (_, Some(af), Some(bi), _) => (af, bi as f64),
                        _ => {
                            let af = if let Some(i) = a.as_int() { i as f64 } else if let Some(f) = a.as_float() { f } else if a.is_null() { 0.0 } else if let Some(b_val) = a.as_bool() { if b_val { 1.0 } else { 0.0 } } else { 0.0 };
                            let bf = if let Some(i) = b.as_int() { i as f64 } else if let Some(f) = b.as_float() { f } else if b.is_null() { 0.0 } else if let Some(b_val) = b.as_bool() { if b_val { 1.0 } else { 0.0 } } else { 0.0 };
                            (af, bf)
                        }
                    };
                    if b_f == 0.0 {
                        return ExecutionResult::Error("Division by zero".to_string());
                    }
                    if a.is_int() && b.is_int() && a.as_int().unwrap() % b.as_int().unwrap() == 0 {
                        Self::push(
                            fibre,
                            Value::new_int(a.as_int().unwrap() / b.as_int().unwrap()),
                        );
                    } else {
                        Self::push(fibre, Value::new_float(a_f / b_f));
                    }
                }
                Opcode::Concat => {
                    if let Some(tracer) = &mut fibre.tracer {
                        if tracer.is_recording {
                            tracer.trace.push(hyperion_jit::tracer::TraceIR::Concat {
                                dest: fibre.stack_top - 2,
                                left: fibre.stack_top - 2,
                                right: fibre.stack_top - 1,
                                fn_ptr: 0, // We'll populate this later if we need FFI in Assembler
                            });
                        }
                    }
                    let b = Self::pop(fibre);
                    let a = Self::pop(fibre);
                    let a_val = a.deref();
                    let b_val = b.deref();

                    if let (Some(a_ptr), Some(b_ptr)) = (a_val.as_string_ptr(), b_val.as_string_ptr()) {
                        let a_str = unsafe { &*(a_ptr as *const String) };
                        let b_str = unsafe { &*(b_ptr as *const String) };
                        let mut s = String::with_capacity(a_str.len() + b_str.len());
                        s.push_str(a_str);
                        s.push_str(b_str);
                        let str_ptr = fibre.arena.alloc_and_track(s) as *mut String;
                        Self::push(fibre, Value::new_string_ptr(str_ptr as *mut ()));
                    } else {
                        let a_str = match Self::cast_value_to_string(fibre, a_val) {
                            Ok(s) => s,
                            Err(e) => return ExecutionResult::Error(e),
                        };
                        let b_str = match Self::cast_value_to_string(fibre, b_val) {
                            Ok(s) => s,
                            Err(e) => return ExecutionResult::Error(e),
                        };
                        let mut s = String::with_capacity(a_str.len() + b_str.len());
                        s.push_str(&a_str);
                        s.push_str(&b_str);
                        let str_ptr = fibre.arena.alloc_and_track(s) as *mut String;
                        Self::push(fibre, Value::new_string_ptr(str_ptr as *mut ()));
                    }

                }
                Opcode::Echo => {
                    let val = Self::pop(fibre);
                    match Self::cast_value_to_string(fibre, val) {
                        Ok(s) => {
                            if let Some(buf) = fibre.ob_buffers.last_mut() {
                                buf.extend_from_slice(s.as_bytes());
                            } else {
                                fibre.output_buffer.extend_from_slice(s.as_bytes());
                            }
                        }
                        Err(e) => return ExecutionResult::Error(e),
                    }
                }
                Opcode::Reserve => {
                    let num_locals = Self::read_byte(fibre) as usize;
                    let frame = fibre.frames[fibre.frame_count - 1];
                    let current_count = fibre.stack_top - frame.stack_window;
                    if num_locals > current_count {
                        for _ in 0..(num_locals - current_count) {
                            Self::push(fibre, Value::null());
                        }
                    }
                }
                Opcode::Pop => {
                    let val = Self::pop(fibre);
                    if let Some(obj_ptr) = val.as_object_ptr() {
                        if let Err(e) = Self::check_and_run_destructor(fibre, obj_ptr) {
                            return ExecutionResult::Error(e);
                        }
                    }
                }
                Opcode::Clone => {
                    let obj_val = Self::pop(fibre);
                    if !obj_val.is_object() {
                        return ExecutionResult::Error("Cannot clone non-object".to_string());
                    }
                    let obj_ptr = obj_val.as_object_ptr().unwrap()
                        as *mut hyperion_core::types::object::PhpObject;
                    let obj = unsafe { &*obj_ptr };
                    let class_id = Self::resolve_obj_class_id(fibre, obj);

                    let new_obj_ptr = fibre.arena.alloc_object(class_id);
                    let new_obj = unsafe { &mut *new_obj_ptr };
                    new_obj.class_name = obj.class_name.clone();
                    new_obj.properties = obj.properties.clone();
                    let new_obj_val = Value::new_object_ptr(new_obj_ptr as *mut ());

                    let has_clone =
                        Self::find_method_with_autoload(fibre, class_id, "__clone").unwrap_or(None);
                    if has_clone.is_some() {
                        if let Err(e) =
                            Self::call_method_synchronously(fibre, new_obj_val, "__clone", vec![])
                        {
                            return ExecutionResult::Error(e);
                        }
                    }
                    Self::push(fibre, new_obj_val);
                }
                Opcode::MakeClosure => {
                    let name_idx = Self::read_short(fibre) as usize;
                    let upvalue_count = Self::read_byte(fibre) as usize;

                    let frame = fibre.frames[fibre.frame_count - 1];
                    let func = unsafe { &*frame.function.0 };
                    let name_val = func.chunk.constants[name_idx];
                    let name_ptr = name_val.as_string_ptr().unwrap() as *const String;
                    let closure_name = unsafe { &*name_ptr };
                    let _called_class_id = frame.called_class_id;

                    let mut upvalues = Vec::with_capacity(upvalue_count);
                    // Pop in reverse order to preserve original order
                    for _ in 0..upvalue_count {
                        upvalues.push(Value::null());
                    }
                    for i in (0..upvalue_count).rev() {
                        upvalues[i] = Self::pop(fibre);
                    }

                    let func_ptr = {
                        let norm = normalize_name(closure_name);
                        if let Some(id) = fibre.engine_state.func_map.get(&norm).map(|v| *v) {
                            let funcs = &fibre.engine_state.functions;
                            if let Some(f) = funcs.get(&id) {
                                *f
                            } else {
                                panic!("Closure function index out of bounds: {}", closure_name);
                            }
                        } else {
                            let funcs = &fibre.engine_state.functions;
                            if let Some(f) = funcs.iter().find(|f| unsafe { &*f.0 }.name == *closure_name) {
                                *f
                            } else {
                                panic!(
                                    "Closure function not found in global registry: {}",
                                    closure_name
                                );
                            }
                        }
                    };

                    let caller_this = fibre.stack[frame.stack_window];
                    let this_val = if caller_this.is_object() {
                        Some(caller_this)
                    } else if let Some(closure_ptr) = caller_this.as_closure_ptr() {
                        let parent_closure = unsafe { &*(closure_ptr as *const crate::types::closure::PhpClosure) };
                        parent_closure.this_val
                    } else {
                        None
                    };
                    let closure = PhpClosure::new(func_ptr, upvalues, frame.called_class_id, this_val);
                    let closure_ptr = fibre.arena.alloc_and_track(closure) as *mut ();
                    Self::push(fibre, Value::new_closure_ptr(closure_ptr));
                }
                Opcode::Dup => {
                    if let Some(tracer) = &mut fibre.tracer {
                        if tracer.is_recording {
                            tracer.trace.push(hyperion_jit::tracer::TraceIR::GetLocal {
                                dest: fibre.stack_top,
                                src: fibre.stack_top - 1,
                            });
                        }
                    }
                    let val = fibre.stack[fibre.stack_top - 1];
                    Self::push(fibre, val);
                }
                Opcode::Dup2 => {
                    let a = fibre.stack[fibre.stack_top - 2];
                    let b = fibre.stack[fibre.stack_top - 1];
                    Self::push(fibre, a);
                    Self::push(fibre, b);
                }
                Opcode::Swap => {
                    let a = fibre.stack[fibre.stack_top - 2];
                    let b = fibre.stack[fibre.stack_top - 1];
                    fibre.stack[fibre.stack_top - 2] = b;
                    fibre.stack[fibre.stack_top - 1] = a;
                }
                Opcode::CastInt => {
                    let val = Self::pop(fibre).deref();
                    if let Some(i) = val.as_int() {
                        Self::push(fibre, Value::new_int(i));
                    } else if let Some(f) = val.as_float() {
                        Self::push(fibre, Value::new_int(f as i32));
                    } else if let Some(str_ptr) = val.as_string_ptr() {
                        let s = unsafe { &*(str_ptr as *const String) };
                        if let Ok(i) = s.parse::<i32>() {
                            Self::push(fibre, Value::new_int(i));
                        } else {
                            Self::push(fibre, Value::new_int(0));
                        }
                    } else if val.is_bool() {
                        let b = val.as_bool().unwrap();
                        Self::push(fibre, Value::new_int(if b { 1 } else { 0 }));
                    } else {
                        Self::push(fibre, Value::new_int(0));
                    }
                }
                Opcode::CastFloat => {
                    let val = Self::pop(fibre).deref();
                    if let Some(i) = val.as_int() {
                        Self::push(fibre, Value::new_float(i as f64));
                    } else if let Some(f) = val.as_float() {
                        Self::push(fibre, Value::new_float(f));
                    } else if let Some(str_ptr) = val.as_string_ptr() {
                        let s = unsafe { &*(str_ptr as *const String) };
                        if let Ok(f) = s.parse::<f64>() {
                            Self::push(fibre, Value::new_float(f));
                        } else {
                            Self::push(fibre, Value::new_float(0.0));
                        }
                    } else if val.is_bool() {
                        let b = val.as_bool().unwrap();
                        Self::push(fibre, Value::new_float(if b { 1.0 } else { 0.0 }));
                    } else {
                        Self::push(fibre, Value::new_float(0.0));
                    }
                }
                Opcode::CastString => {
                    let val = Self::pop(fibre).deref();
                    match Self::cast_value_to_string(fibre, val) {
                        Ok(s) => {
                            let str_ptr = fibre.arena.alloc_and_track(s) as *mut String;
                            Self::push(fibre, Value::new_string_ptr(str_ptr as *mut ()));
                        }
                        Err(e) => return ExecutionResult::Error(e),
                    }
                }
                Opcode::CastBool => {
                    let val = Self::pop(fibre).deref();
                    Self::push(fibre, Value::new_bool(val.is_truthy()));
                }
                Opcode::CastArray => {
                    let val = Self::pop(fibre).deref();
                    if val.is_array() {
                        Self::push(fibre, val);
                    } else if let Some(obj_ptr) = val.as_object_ptr() {
                        let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
                        let mut arr = hyperion_core::types::array::PhpArray::new();
                        for (k, v) in &obj.properties {
                            let str_id = hyperion_core::types::string_table::intern_string(k);
                            arr.insert_string_id(str_id, *v);
                        }
                        let arr_ptr = fibre.arena.alloc_and_track(arr);
                        Self::push(fibre, Value::new_array_ptr(arr_ptr as *mut ()));
                    } else {
                        let mut arr = hyperion_core::types::array::PhpArray::new();
                        if !val.is_null() {
                            arr.insert_int(0, val);
                        }
                        let arr_ptr = fibre.arena.alloc_and_track(arr);
                        Self::push(fibre, Value::new_array_ptr(arr_ptr as *mut ()));
                    }
                }
                Opcode::CastObject => {
                    let val = Self::pop(fibre).deref();
                    if val.is_object() {
                        Self::push(fibre, val);
                    } else {
                        let stdclass_id = Self::find_class_id(fibre, "stdClass").unwrap_or(0);
                        let mut obj = hyperion_core::types::object::PhpObject::new(stdclass_id);
                        if let Some(arr_ptr) = val.as_array_ptr() {
                            let arr = unsafe { &*(arr_ptr as *const hyperion_core::types::array::PhpArray) };
                            for (k, v) in &arr.elements {
                                match k {
                                    hyperion_core::types::array::ArrayKey::StringId(id) => {
                                        if let Some(name) = hyperion_core::types::string_table::lookup_string(*id) {
                                            obj.properties.insert(name, *v);
                                        }
                                    }
                                    hyperion_core::types::array::ArrayKey::Int(i) => {
                                        obj.properties.insert(i.to_string(), *v);
                                    }
                                }
                            }
                        } else if !val.is_null() {
                            obj.properties.insert("scalar".to_string(), val);
                        }
                        let obj_ptr = fibre.arena.alloc_and_track(obj);
                        Self::push(fibre, Value::new_object_ptr(obj_ptr as *mut ()));
                    }
                }
                Opcode::EndFinally => {
                    if let Some(action) = fibre.unwind_action.clone() {
                        match action {
                            crate::fibre::UnwindAction::Return(val) => {
                                fibre.unwind_action = None;
                                if let Some(res) = Self::handle_return(fibre, val) {
                                    return res;
                                }
                            }
                            crate::fibre::UnwindAction::Throw(val) => {
                                fibre.unwind_action = None;
                                if let Some(res) = Self::handle_throw(fibre, val) {
                                    return res;
                                }
                            }
                        }
                    }
                }

                Opcode::IterInit => {
                    let offset = Self::read_short(fibre) as usize;
                    let is_by_ref = fibre.stack[fibre.stack_top - 1].is_ref();
                    let raw = fibre.stack[fibre.stack_top - 1].deref();

                    // An `IteratorAggregate` is replaced on the stack by whatever
                    // `getIterator()` returned, so `IterNext` sees a source it can
                    // walk directly. A plain object becomes a snapshot array of
                    // its properties.
                    let mut iterable = match Self::resolve_iterable(fibre, raw) {
                        Ok(v) => v,
                        Err(e) => return ExecutionResult::Error(e),
                    };
                    if iterable.is_object() {
                        let obj_ptr = iterable.as_object_ptr().unwrap()
                            as *const hyperion_core::types::object::PhpObject;
                        let obj = unsafe { &*obj_ptr };
                        let class_id = Self::resolve_obj_class_id(fibre, obj);
                        let is_gen = {
                            let classes = &fibre.engine_state.classes;
                            classes.get(&class_id).map(|c| c.name == "Generator").unwrap_or(false)
                        };
                        if !is_gen && !Self::class_has_method(fibre, class_id, "current") {
                            iterable = Self::object_properties_as_array(fibre, iterable);
                        }
                    }
                    if !is_by_ref && iterable.is_array() {
                        let array_ptr = iterable.as_array_ptr().unwrap()
                            as *const hyperion_core::types::array::PhpArray;
                        let cloned_arr = unsafe { (*array_ptr).clone() };
                        let new_ptr = fibre.arena.alloc_and_track(cloned_arr);
                        iterable = Value::new_array_ptr(new_ptr as *mut ());
                    }
                    unsafe {
                        fibre.stack[fibre.stack_top - 1] = iterable;
                    }

                    fibre.stack[fibre.stack_top] = Value::new_int(0);
                    fibre.stack_top += 1;

                    let mut is_generator = false;
                    let mut gen_valid = false;

                    if iterable.is_object() {
                        let obj_ptr = iterable.as_object_ptr().unwrap()
                            as *mut hyperion_core::types::object::PhpObject;
                        let obj = unsafe { &*obj_ptr };
                        let class_id = Self::resolve_obj_class_id(fibre, obj);
                        let is_gen = {
                            let classes = &fibre.engine_state.classes;
                            classes.get(&class_id).map(|c| c.name == "Generator").unwrap_or(false)
                        };
                        if is_gen {
                            is_generator = true;
                            if let Some(gen_val) = obj.properties.get("__generator") {
                                let gen_fibre_ptr = gen_val.as_object_ptr().unwrap() as *mut Fibre;
                                let gen_fibre = unsafe { &mut *gen_fibre_ptr };

                                if !gen_fibre.generator_started {
                                    gen_fibre.generator_started = true;
                                    let res = Self::step_generator_fibre(gen_fibre);
                                    match res {
                                        ExecutionResult::Yielded(YieldReason::GeneratorYield) => {
                                            gen_fibre.generator_valid = true;
                                        }
                                        ExecutionResult::Finished => {
                                            gen_fibre.generator_valid = false;
                                        }
                                        ExecutionResult::Error(err) => {
                                            return ExecutionResult::Error(err);
                                        }
                                        ExecutionResult::UncaughtException(err) => {
                                            return ExecutionResult::UncaughtException(err);
                                        }

                                        _ => {
                                            gen_fibre.generator_valid = false;
                                        }
                                    }
                                }
                                gen_valid = gen_fibre.generator_valid;
                            }
                        }
                    }

                    if is_generator {
                        if !gen_valid {
                            fibre.frames[fibre.frame_count - 1].ip += offset;
                        }
                    } else if iterable.is_array() {
                        let array_ptr = iterable.as_array_ptr().unwrap()
                            as *mut hyperion_core::types::array::PhpArray;
                        let arr = unsafe { &*array_ptr };
                        if arr.len() == 0 {
                            fibre.frames[fibre.frame_count - 1].ip += offset;
                        }
                    } else if iterable.is_object() {
                        // A user-land `Iterator`. Rewinding here rather than in
                        // `IterNext` keeps the "first step rewinds, later steps
                        // advance" rule in one place, and an empty iterator
                        // skips the body just like an empty array.
                        match Self::iterator_step(fibre, iterable, true) {
                            Ok(true) => {}
                            Ok(false) => {
                                fibre.frames[fibre.frame_count - 1].ip += offset;
                            }
                            Err(e) => return ExecutionResult::Error(e),
                        }
                    } else {
                        fibre.frames[fibre.frame_count - 1].ip += offset;
                    }
                }
                Opcode::IterNext => {
                    let key_local = Self::read_byte(fibre);
                    let value_local = Self::read_byte(fibre);
                    // `foreach ($a as &$v)` — the loop variable aliases the
                    // element instead of receiving a copy.
                    let value_by_ref = Self::read_byte(fibre) != 0;
                    let offset = Self::read_short(fibre) as usize;

                    let index_val = fibre.stack[fibre.stack_top - 1];
                    let iterable = fibre.stack[fibre.stack_top - 2];

                    if index_val.as_int().is_none() {
                        return ExecutionResult::Error(format!(
                            "IterNext: expected int index at stack_top - 1, got type {:?} (val {:?})",
                            index_val.get_type(), index_val
                        ));
                    }
                    let idx = index_val.as_int().unwrap() as usize;

                    let mut is_generator = false;
                    if iterable.is_object() {
                        let obj_ptr = iterable.as_object_ptr().unwrap()
                            as *mut hyperion_core::types::object::PhpObject;
                        let obj = unsafe { &*obj_ptr };
                        let class_id = Self::resolve_obj_class_id(fibre, obj);
                        let is_gen = {
                            let classes = &fibre.engine_state.classes;
                            classes.get(&class_id).map(|c| c.name == "Generator").unwrap_or(false)
                        };
                        if is_gen {
                            is_generator = true;
                            if let Some(gen_val) = obj.properties.get("__generator") {
                                let gen_fibre_ptr = gen_val.as_object_ptr().unwrap() as *mut Fibre;
                                let gen_fibre = unsafe { &mut *gen_fibre_ptr };

                                let has_next = if idx == 0 {
                                    gen_fibre.generator_valid
                                } else {
                                    let res = Self::step_generator_fibre(gen_fibre);
                                    match res {
                                        ExecutionResult::Yielded(YieldReason::GeneratorYield) => {
                                            gen_fibre.generator_valid = true;
                                            true
                                        }
                                        ExecutionResult::Finished => {
                                            gen_fibre.generator_valid = false;
                                            false
                                        }
                                        ExecutionResult::Error(err) => {
                                            return ExecutionResult::Error(err);
                                        }
                                        ExecutionResult::UncaughtException(err) => {
                                            return ExecutionResult::UncaughtException(err);
                                        }

                                        _ => {
                                            gen_fibre.generator_valid = false;
                                            false
                                        }
                                    }
                                };


                                if has_next {
                                    let frame_idx = fibre.frame_count - 1;
                                    let stack_window = unsafe {
                                        fibre.frames.get_unchecked(frame_idx).stack_window
                                    };
                                    if key_local != 255 {
                                        unsafe {
                                            *fibre.stack.get_unchecked_mut(
                                                stack_window + key_local as usize,
                                            ) = gen_fibre.generator_key;
                                        }
                                    }
                                    unsafe {
                                        *fibre.stack.get_unchecked_mut(
                                            stack_window + value_local as usize,
                                        ) = gen_fibre.generator_value;
                                    }
                                    fibre.stack[fibre.stack_top - 1] =
                                        Value::new_int((idx + 1) as i32);
                                } else {
                                    fibre.frames[fibre.frame_count - 1].ip += offset;
                                }
                            }
                        }
                    }

                    if !is_generator {
                        if iterable.is_array() {
                            let array_ptr = iterable.as_array_ptr().unwrap()
                                as *mut hyperion_core::types::array::PhpArray;

                            // Flush the previous iteration's loop variable back
                            // into the element it came from. Doing it here also
                            // covers the last element, since IterNext always runs
                            // once more than the body does.
                            if value_by_ref && idx > 0 {
                                let frame_idx = fibre.frame_count - 1;
                                let stack_window = unsafe {
                                    fibre.frames.get_unchecked(frame_idx).stack_window
                                };
                                let cur = fibre.stack[stack_window + value_local as usize].deref();
                                let arr_mut = unsafe { &mut *array_ptr };
                                if let Some((_, slot)) = arr_mut.elements.get_index_mut(idx - 1) {
                                    *slot = cur;
                                }
                            }

                            let arr = unsafe { &*array_ptr };

                            if let Some((key, val)) = arr.elements.get_index(idx) {
                                let frame_idx = fibre.frame_count - 1;
                                let stack_window =
                                    unsafe { fibre.frames.get_unchecked(frame_idx).stack_window };
                                let _k_val = if key_local != 255 {
                                    let k = match key {
                                        hyperion_core::types::array::ArrayKey::Int(i) => {
                                            Value::new_int(*i as i32)
                                        }
                                        hyperion_core::types::array::ArrayKey::StringId(s) => {
                                            if let Some(s_str) = hyperion_core::types::string_table::lookup_string(*s) {
                                                let str_ptr = fibre.arena.alloc_and_track(s_str.clone());
                                                Value::new_string_ptr(str_ptr as *mut ())
                                            } else {
                                                let str_ptr = fibre.arena.alloc_and_track("".to_string());
                                                Value::new_string_ptr(str_ptr as *mut ())
                                            }
                                        }
                                    };
                                    unsafe {
                                        *fibre
                                            .stack
                                            .get_unchecked_mut(stack_window + key_local as usize) =
                                            k
                                    };
                                    Some(k)
                                } else {
                                    None
                                };
                                // By-ref iteration: the loop variable receives a
                                // plain copy, and the *previous* element is
                                // written back from it just before we advance
                                // (plus once more on the exit path). That gives
                                // `foreach ($a as &$v) { $v = ... }` its
                                // mutate-the-source behaviour without ever
                                // storing a ref cell inside the array — which
                                // would leak into every ext function that reads
                                // elements directly.
                                unsafe {
                                    *fibre
                                        .stack
                                        .get_unchecked_mut(stack_window + value_local as usize) =
                                        val.deref()
                                };
                                fibre.stack[fibre.stack_top - 1] =
                                    Value::new_int((idx + 1) as i32);
                            } else {
                                fibre.frames[fibre.frame_count - 1].ip += offset;
                            }
                        } else if iterable.is_object() {
                            // User-land `Iterator`. `IterInit` already rewound and
                            // proved the first element valid, so index 0 reads the
                            // current position without advancing; every later step
                            // calls next() first.
                            let has_next = if idx == 0 {
                                Ok(true)
                            } else {
                                Self::iterator_step(fibre, iterable, false)
                            };
                            match has_next {
                                Ok(true) => {
                                    let val =
                                        match Self::call_method_synchronously(
                                            fibre, iterable, "current", vec![],
                                        ) {
                                            Ok(v) => v,
                                            Err(e) => return ExecutionResult::Error(e),
                                        };
                                    let key_val = if key_local != 255 {
                                        let class_id = unsafe {
                                            &*(iterable.as_object_ptr().unwrap()
                                                as *const hyperion_core::types::object::PhpObject)
                                        }
                                        .class_id;
                                        if Self::class_has_method(fibre, class_id, "key") {
                                            match Self::call_method_synchronously(
                                                fibre, iterable, "key", vec![],
                                            ) {
                                                Ok(v) => v,
                                                Err(e) => return ExecutionResult::Error(e),
                                            }
                                        } else {
                                            Value::new_int(idx as i32)
                                        }
                                    } else {
                                        Value::null()
                                    };

                                    let frame_idx = fibre.frame_count - 1;
                                    let stack_window = unsafe {
                                        fibre.frames.get_unchecked(frame_idx).stack_window
                                    };
                                    if key_local != 255 {
                                        unsafe {
                                            *fibre.stack.get_unchecked_mut(
                                                stack_window + key_local as usize,
                                            ) = key_val;
                                        }
                                    }
                                    unsafe {
                                        *fibre.stack.get_unchecked_mut(
                                            stack_window + value_local as usize,
                                        ) = val.deref();
                                    }
                                    fibre.stack[fibre.stack_top - 1] =
                                        Value::new_int((idx + 1) as i32);
                                }
                                Ok(false) => {
                                    fibre.frames[fibre.frame_count - 1].ip += offset;
                                }
                                Err(e) => return ExecutionResult::Error(e),
                            }
                        } else {
                            fibre.frames[fibre.frame_count - 1].ip += offset;
                        }
                    }
                }
                Opcode::MakeRefLocal => {
                    // Convert the local into a shared cell *in place* so the
                    // original variable and this new alias observe the same
                    // storage from here on. Re-referencing an already-ref slot
                    // reuses its cell rather than nesting.
                    let idx = Self::read_byte(fibre) as usize;
                    let frame = fibre.frames[fibre.frame_count - 1];
                    let slot = frame.stack_window + idx;
                    let existing = fibre.stack[slot];
                    let ref_val = if existing.is_ref() {
                        existing
                    } else {
                        let cell = hyperion_core::types::reference::PhpRef::new(existing);
                        let cell_ptr = fibre.arena.alloc_and_track(cell);
                        let v = Value::new_ref_ptr(cell_ptr as *mut ());
                        fibre.stack[slot] = v;
                        v
                    };
                    Self::push(fibre, ref_val);
                }
                Opcode::MakeRefElement => {
                    // Stack: [container_ref, key] -> [element_ref]
                    // PHP autovivifies on reference, so a missing element (or a
                    // null/non-array container) is materialised rather than
                    // faulting.
                    let key = Self::pop(fibre);
                    let container = Self::pop(fibre);

                    let arr_ptr = match Self::array_ptr_for_write(fibre, container) {
                        Ok(p) => p,
                        Err(e) => return ExecutionResult::Error(e),
                    };

                    let arr = unsafe { &mut *arr_ptr };
                    let arr_key = if key.deref().is_null() {
                        // `$a[] = &...` appends.
                        hyperion_core::types::array::ArrayKey::Int(arr.next_free_int_key())
                    } else {
                        Self::value_to_array_key(key.deref())
                    };

                    let existing = arr.elements.get(&arr_key).map(|v| *v);

                    // Arrays already alias by handle in this engine, so an
                    // array-valued element needs no cell — pushing the handle
                    // gives the same shared-storage semantics without making
                    // every array reader deref. This is the case Laravel's
                    // `Arr::set` hits on every `$array = &$array[$key]` hop.
                    if let Some(v) = existing {
                        if v.deref().is_array() {
                            Self::push(fibre, v.deref());
                            continue;
                        }
                    }

                    let ref_val = match existing {
                        Some(v) if v.is_ref() => v,
                        other => {
                            let cell = hyperion_core::types::reference::PhpRef::new(
                                other.unwrap_or_else(Value::null),
                            );
                            let cell_ptr = fibre.arena.alloc_and_track(cell);
                            let v = Value::new_ref_ptr(cell_ptr as *mut ());
                            let arr = unsafe { &mut *arr_ptr };
                            arr.insert_key(arr_key, v);
                            v
                        }
                    };
                    Self::push(fibre, ref_val);
                }
                Opcode::BindRefLocal => {
                    // `$a = &$b`: store the ref itself in the slot. Unlike
                    // SetLocal this must NOT write through an existing ref —
                    // the point is to rebind the variable to a new cell.
                    let idx = Self::read_byte(fibre) as usize;
                    let val = Self::pop(fibre);
                    let frame = fibre.frames[fibre.frame_count - 1];
                    let slot = frame.stack_window + idx;
                    fibre.stack[slot] = val;
                    Self::push(fibre, val);
                }
                Opcode::SetLocal => {
                    let idx = Self::read_byte(fibre) as usize;
                    let frame = fibre.frames[fibre.frame_count - 1];
                    let slot = frame.stack_window + idx;

                    if let Some(tracer) = &mut fibre.tracer {
                        if tracer.is_recording {
                            tracer.trace.push(hyperion_jit::tracer::TraceIR::SetLocal {
                                dest: slot,
                                src: fibre.stack_top - 1,
                            });
                        }
                    }

                    let raw_val = Self::pop(fibre);
                    // PHP arrays are by-value: deep-copy when assigning to a local slot
                    let val = if raw_val.deref().is_array() {
                        Self::deep_copy_value(fibre, raw_val.deref())
                    } else {
                        raw_val.deref()
                    };
                    let frame = fibre.frames[fibre.frame_count - 1];
                    let slot = frame.stack_window + idx;
                    // If the variable is a reference, the assignment updates the
                    // shared cell so every alias observes it. Overwriting the
                    // slot instead would silently break the reference.
                    let existing = fibre.stack[slot];
                    if let Some(cell) = existing.as_ref_ptr() {
                        unsafe {
                            (*(cell as *mut hyperion_core::types::reference::PhpRef)).set(val)
                        };
                    } else {
                        fibre.stack[slot] = val;
                    }
                    Self::push(fibre, val);
                }
                Opcode::GetLocal | Opcode::GetLocalQuiet => {
                    let idx = Self::read_byte(fibre) as usize;
                    let frame = fibre.frames[fibre.frame_count - 1];
                    let slot = frame.stack_window + idx;

                    if let Some(tracer) = &mut fibre.tracer {
                        if tracer.is_recording {
                            tracer.trace.push(hyperion_jit::tracer::TraceIR::GetLocal {
                                dest: fibre.stack_top,
                                src: slot,
                            });
                        }
                    }

                    // A ref slot reads as its pointee: references are invisible
                    // to every consumer except the write path.
                    let val = fibre.stack[slot].deref();
                    Self::push(fibre, val);
                }

                Opcode::GetSuperglobal => {
                    let id = Self::read_byte(fibre) as usize;
                    let val = if id < 8 {
                        fibre.superglobals[id]
                    } else {
                        Value::null()
                    };
                    Self::push(fibre, val);
                }
                Opcode::SetSuperglobal => {
                    let id = Self::read_byte(fibre) as usize;
                    let val = fibre.stack[fibre.stack_top - 1];
                    if id < 8 {
                        fibre.superglobals[id] = val;
                    }
                }
                Opcode::ArrayGet => {
                    let key = Self::pop(fibre).deref();
                    let array_val = Self::pop(fibre).deref();

                    if let Some(tracer) = fibre.tracer.as_mut() {
                        if tracer.is_recording {
                            let array_idx = fibre.stack_top;
                            let key_idx = fibre.stack_top + 1;
                            let dest_idx = fibre.stack_top;

                            if array_val.is_array() && key.is_int() {
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardType {
                                    src: array_idx,
                                    expected_type_tag: hyperion_core::memory::nan_box::QNAN
                                        | hyperion_core::memory::nan_box::TAG_ARR,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top + 2,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardType {
                                    src: key_idx,
                                    expected_type_tag: hyperion_core::memory::nan_box::QNAN
                                        | hyperion_core::memory::nan_box::TAG_INT,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top + 2,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::ArrayGetPacked {
                                    dest: dest_idx,
                                    array: array_idx,
                                    key: key_idx,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top + 2,
                                });
                            } else {
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardType {
                                    src: array_idx,
                                    expected_type_tag: hyperion_core::memory::nan_box::QNAN
                                        | hyperion_core::memory::nan_box::TAG_ARR,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top + 2,
                                });
                                tracer
                                    .trace
                                    .push(hyperion_jit::tracer::TraceIR::FetchArrayElement {
                                        dest: dest_idx,
                                        array: array_idx,
                                        key: key_idx,
                                        fn_ptr: jit_fetch_array_element as *const () as u64,
                                    });
                            }
                        }
                    }

                    if let Some(arr_ptr) = array_val.as_array_ptr() {
                        let arr =
                            unsafe { &*(arr_ptr as *const hyperion_core::types::array::PhpArray) };
                        if let Some(key_str) = key.as_string_ptr() {
                            let key_s = unsafe { &*(key_str as *const String) };
                            let first_byte = key_s.as_bytes().first();
                            let is_possible_int = first_byte.map_or(false, |&b| b.is_ascii_digit() || b == b'-' || b == b'+');
                            if is_possible_int {
                                if let Ok(key_int) = key_s.parse::<i64>() {
                                    if let Some(val) = arr.get_int(key_int) {
                                        Self::push(fibre, *val);
                                    } else {
                                        Self::push(fibre, Value::null());
                                    }
                                    continue;
                                }
                            }
                            if let Some(val) = arr.get_string_id(hyperion_core::types::string_table::intern_string(key_s)) {
                                Self::push(fibre, *val);
                            } else {
                                Self::push(fibre, Value::null());
                            }
                        }
 else if let Some(key_int) = key.as_int() {
                            if let Some(val) = arr.get_int(key_int as i64) {
                                Self::push(fibre, *val);
                            } else if let Some(val) = arr.get_string_id(hyperion_core::types::string_table::intern_string(&key_int.to_string())) {
                                Self::push(fibre, *val);
                            } else {
                                Self::push(fibre, Value::null());
                            }
                        } else {
                            Self::push(fibre, Value::null());
                        }
                    } else if let Some(str_ptr) = array_val.as_string_ptr() {
                        // String character access: $str[$idx] (supports PHP 7.1+ negative indexing)
                        let s = unsafe { &*(str_ptr as *const String) };
                        let s_bytes = s.as_bytes();
                        let idx_opt = if let Some(ki) = key.as_int() {
                            if ki < 0 {
                                let positive = s_bytes.len() as i32 + ki;
                                if positive >= 0 { Some(positive as usize) } else { None }
                            } else {
                                Some(ki as usize)
                            }
                        } else if let Some(ks_ptr) = key.as_string_ptr() {
                            let ks = unsafe { &*(ks_ptr as *const String) };
                            if let Ok(ki) = ks.parse::<i32>() {
                                if ki < 0 {
                                    let positive = s_bytes.len() as i32 + ki;
                                    if positive >= 0 { Some(positive as usize) } else { None }
                                } else {
                                    Some(ki as usize)
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        if let Some(i) = idx_opt {
                            if i < s_bytes.len() {
                                let ch = &s_bytes[i..i + 1];
                                let boxed = fibre.arena.alloc_and_track(String::from_utf8_lossy(ch).into_owned());
                                Self::push(fibre, Value::new_string_ptr(boxed as *mut ()));
                            } else {
                                Self::push(fibre, Value::null());
                            }
                        } else {
                        }
                    } else if let Some(obj_ptr) = array_val.as_object_ptr() {
                        let obj = unsafe { &*(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
                        let func_ptr_opt = Self::find_method_fast(fibre, obj.class_id, "offsetGet");


                        match func_ptr_opt {
                            Ok(Some(func_ptr)) => {
                                let stack_window = fibre.stack_top;
                                Self::push(fibre, array_val);
                                Self::push(fibre, key);

                                if fibre.frame_count >= fibre.frames.len() {
                                    return ExecutionResult::Error("Stack overflow".to_string());
                                }

                                let passed_args = vec![key];
                                fibre.frames[fibre.frame_count] = CallFrame {
                                    function: func_ptr,
                                    ip: 0,
                                    stack_window,
                                    called_class_id: obj.class_id,
                                    return_override: None, eval_parent_stack_window: None,
                                    arity: 1,
                                };
                                fibre.set_frame_args(fibre.frame_count, &passed_args);
                                fibre.frame_count += 1;
                                Self::pad_missing_arguments_and_locals(fibre, 1, unsafe {
                                    &*func_ptr.0
                                }, &[], false);
                                continue;
                            }
                            Ok(None) => {
                                let class_name = if let Some(ref name) = obj.class_name {
                                    name.clone()
                                } else if let Some(c) = fibre.engine_state.classes.get(&obj.class_id) {
                                    c.name.clone()
                                } else {
                                    "Unknown".to_string()
                                };
                                return ExecutionResult::Error(format!("[ArrayGet] Cannot use object of type {} as array", class_name));
                            }
                            Err(e) => return ExecutionResult::Error(e),
                        }
                    } else {
                        Self::push(fibre, Value::null());
                    }
                }
                // ArrayGetForWrite: fetch array[key] for writing.
                // If the key is missing or null, create an empty PhpArray,
                // store it in the parent array at that key, then push the
                // new array handle so a subsequent ArraySet writes into it.
                Opcode::ArrayGetForWrite => {
                    let key = Self::pop(fibre).deref();
                    let array_val = Self::pop(fibre).deref();

                    if let Some(arr_ptr) = array_val.as_array_ptr() {
                        let arr_key = Self::value_to_array_key(key);
                        let existing = {
                            let arr = unsafe { &*(arr_ptr as *const hyperion_core::types::array::PhpArray) };
                            arr.elements.get(&arr_key).map(|v| *v)
                        };

                        match existing {
                            Some(v) if v.deref().is_array() => {
                                // Existing array element — push handle as-is.
                                Self::push(fibre, v.deref());
                            }
                            Some(v) if v.deref().as_object_ptr().is_some() => {
                                // Existing object — push as-is for property/method chains.
                                Self::push(fibre, v.deref());
                            }
                            _ => {
                                // Missing or scalar — create a new empty array,
                                // store it back into the parent, and push the handle.
                                let new_inner = hyperion_core::types::array::PhpArray::new();
                                let new_ptr = fibre.arena.alloc_and_track(new_inner) as *mut ();
                                let new_val = Value::new_array_ptr(new_ptr);
                                let arr = unsafe { &mut *(arr_ptr as *mut hyperion_core::types::array::PhpArray) };
                                arr.insert_key(arr_key, new_val);
                                Self::push(fibre, new_val);
                            }
                        }
                    } else if let Some(obj_ptr) = array_val.as_object_ptr() {
                        let obj = unsafe { &*(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
                        let func_ptr_opt = Self::find_method_fast(fibre, obj.class_id, "offsetGet");
                        match func_ptr_opt {
                            Ok(Some(func_ptr)) => {
                                let stack_window = fibre.stack_top;
                                Self::push(fibre, array_val);
                                Self::push(fibre, key);

                                if fibre.frame_count >= fibre.frames.len() {
                                    return ExecutionResult::Error("Stack overflow".to_string());
                                }

                                let passed_args = vec![key];
                                fibre.frames[fibre.frame_count] = CallFrame {
                                    function: func_ptr,
                                    ip: 0,
                                    stack_window,
                                    called_class_id: obj.class_id,
                                    return_override: None, eval_parent_stack_window: None,
                                    arity: 1,
                                };
                                fibre.set_frame_args(fibre.frame_count, &passed_args);
                                fibre.frame_count += 1;
                                Self::pad_missing_arguments_and_locals(fibre, 1, unsafe {
                                    &*func_ptr.0
                                }, &[], false);
                                continue;
                            }
                            Ok(None) => {
                                let class_name = if let Some(ref name) = obj.class_name {
                                    name.clone()
                                } else if let Some(c) = fibre.engine_state.classes.get(&obj.class_id) {
                                    c.name.clone()
                                } else {
                                    "Unknown".to_string()
                                };
                                return ExecutionResult::Error(format!("[ArrayGetForWrite] Cannot use object of type {} as array", class_name));
                            }
                            Err(e) => return ExecutionResult::Error(e),
                        }
                    } else {
                        // Not an array (null, scalar, object) — push null so
                        // the subsequent ArraySet sees something it can handle.
                        Self::push(fibre, Value::null());
                    }
                }
                Opcode::EnsureLocalArray => {
                    let idx = Self::read_byte(fibre) as usize;
                    let frame = fibre.frames[fibre.frame_count - 1];
                    let slot = frame.stack_window + idx;
                    let existing = fibre.stack[slot];
                    let derefed = existing.deref();
                    if derefed.is_array() || derefed.as_object_ptr().is_some() || derefed.is_string() {
                        Self::push(fibre, derefed);
                    } else {
                        let new_inner = hyperion_core::types::array::PhpArray::new();
                        let new_ptr = fibre.arena.alloc_and_track(new_inner) as *mut ();
                        let new_val = Value::new_array_ptr(new_ptr);
                        if let Some(cell) = existing.as_ref_ptr() {
                            unsafe {
                                (*(cell as *mut hyperion_core::types::reference::PhpRef)).set(new_val);
                            }
                        } else {
                            fibre.stack[slot] = new_val;
                        }
                        Self::push(fibre, new_val);
                    }
                }
                Opcode::GetPropertyForWrite => {
                    let name_idx = Self::read_short(fibre);
                    let obj = Self::pop(fibre).deref();

                    if let Some(obj_ptr) = obj.as_object_ptr() {
                        let php_obj = unsafe { &mut *(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
                        let frame = fibre.frames[fibre.frame_count - 1];
                        let func = unsafe { &*frame.function.0 };
                        let name_val = func.chunk.constants[name_idx as usize];

                        if let Some(name_str_ptr) = name_val.as_string_ptr() {
                            let name_str = unsafe { &*(name_str_ptr as *const String) };
                            if let Some(val) = php_obj.properties.get(name_str) {
                                let derefed = val.deref();
                                if derefed.is_array() || derefed.as_object_ptr().is_some() || derefed.is_string() {
                                    Self::push(fibre, derefed);
                                } else {
                                    let new_inner = hyperion_core::types::array::PhpArray::new();
                                    let new_ptr = fibre.arena.alloc_and_track(new_inner) as *mut ();
                                    let new_val = Value::new_array_ptr(new_ptr);
                                    php_obj.properties.insert(name_str.clone(), new_val);
                                    Self::push(fibre, new_val);
                                }
                            } else {
                                let new_inner = hyperion_core::types::array::PhpArray::new();
                                let new_ptr = fibre.arena.alloc_and_track(new_inner) as *mut ();
                                let new_val = Value::new_array_ptr(new_ptr);
                                php_obj.properties.insert(name_str.clone(), new_val);
                                Self::push(fibre, new_val);
                            }
                        } else {
                            Self::push(fibre, Value::null());
                        }
                    } else {
                        Self::push(fibre, Value::null());
                    }
                }
                Opcode::GetPropertyDynamicForWrite => {
                    let name_val = Self::pop(fibre).deref();
                    let obj = Self::pop(fibre).deref();

                    if let Some(obj_ptr) = obj.as_object_ptr() {
                        let php_obj = unsafe { &mut *(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
                        let name_str = if let Some(s_ptr) = name_val.as_string_ptr() {
                            unsafe { (*(s_ptr as *const String)).clone() }
                        } else if let Some(i) = name_val.as_int() {
                            i.to_string()
                        } else {
                            "unsupported".to_string()
                        };

                        if let Some(val) = php_obj.properties.get(&name_str) {
                            let derefed = val.deref();
                            if derefed.is_array() || derefed.as_object_ptr().is_some() || derefed.is_string() {
                                Self::push(fibre, derefed);
                            } else {
                                let new_inner = hyperion_core::types::array::PhpArray::new();
                                let new_ptr = fibre.arena.alloc_and_track(new_inner) as *mut ();
                                let new_val = Value::new_array_ptr(new_ptr);
                                php_obj.properties.insert(name_str, new_val);
                                Self::push(fibre, new_val);
                            }
                        } else {
                            let new_inner = hyperion_core::types::array::PhpArray::new();
                            let new_ptr = fibre.arena.alloc_and_track(new_inner) as *mut ();
                            let new_val = Value::new_array_ptr(new_ptr);
                            php_obj.properties.insert(name_str, new_val);
                            Self::push(fibre, new_val);
                        }
                    } else {
                        Self::push(fibre, Value::null());
                    }
                }
                Opcode::ArraySet => {
                    let raw_value = Self::pop(fibre);
                    let value = if raw_value.deref().is_array() {
                        Self::deep_copy_value(fibre, raw_value.deref())
                    } else {
                        raw_value.deref()
                    };
                    let key = Self::pop(fibre);
                    let array_val = Self::pop(fibre);

                    if let Some(tracer) = fibre.tracer.as_mut() {
                        if tracer.is_recording {
                            let array_idx = fibre.stack_top;
                            let key_idx = fibre.stack_top + 1;
                            let val_idx = fibre.stack_top + 2;

                            if array_val.deref().is_array() && key.deref().is_int() {
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardType {
                                    src: array_idx,
                                    expected_type_tag: hyperion_core::memory::nan_box::QNAN
                                        | hyperion_core::memory::nan_box::TAG_ARR,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top + 3,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardType {
                                    src: key_idx,
                                    expected_type_tag: hyperion_core::memory::nan_box::QNAN
                                        | hyperion_core::memory::nan_box::TAG_INT,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top + 3,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::ArraySetPacked {
                                    array: array_idx,
                                    key: key_idx,
                                    val: val_idx,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top + 3,
                                });
                            } else {
                                tracer.stop();
                            }
                        }
                    }

                    if let Some(arr_ptr) = array_val.as_array_ptr() {
                        let arr = unsafe {
                            &mut *(arr_ptr as *mut hyperion_core::types::array::PhpArray)
                        };
                        if let Some(key_str) = key.as_string_ptr() {
                            let key_s = unsafe { &*(key_str as *const String) };
                            let first_byte = key_s.as_bytes().first();
                            let is_possible_int = first_byte.map_or(false, |&b| b.is_ascii_digit() || b == b'-' || b == b'+');
                            if is_possible_int {
                                if let Ok(key_int) = key_s.parse::<i64>() {
                                    arr.insert_int(key_int, value);
                                } else {
                                    arr.insert_string_id(hyperion_core::types::string_table::intern_string(key_s), value);
                                }
                            } else {
                                arr.insert_string_id(hyperion_core::types::string_table::intern_string(key_s), value);
                            }
                        }
 else if let Some(key_int) = key.as_int() {
                            arr.insert_int(key_int as i64, value);
                        } else if key.is_null() {
                            arr.push(value);
                        }

                    } else if let Some(obj_ptr) = array_val.as_object_ptr() {
                        let obj = unsafe { &*(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
                        let func_ptr_opt = Self::find_method_fast(fibre, obj.class_id, "offsetSet");

                        match func_ptr_opt {
                            Ok(Some(func_ptr)) => {
                                let stack_window = fibre.stack_top;
                                Self::push(fibre, array_val);
                                Self::push(fibre, key);
                                Self::push(fibre, value);

                                if fibre.frame_count >= fibre.frames.len() {
                                    return ExecutionResult::Error("Stack overflow".to_string());
                                }

                                let passed_args = vec![key, value];
                                fibre.frames[fibre.frame_count] = CallFrame {
                                    function: func_ptr,
                                    ip: 0,
                                    stack_window,
                                    called_class_id: obj.class_id,
                                    return_override: Some(value), eval_parent_stack_window: None,
                                    arity: 2,
                                };
                                fibre.set_frame_args(fibre.frame_count, &passed_args);
                                fibre.frame_count += 1;
                                Self::pad_missing_arguments_and_locals(fibre, 2, unsafe {
                                    &*func_ptr.0
                                }, &[], false);
                                continue;
                            }
                            Ok(None) => {
                                let class_name = if let Some(ref name) = obj.class_name {
                                    name.clone()
                                } else if let Some(c) = fibre.engine_state.classes.get(&obj.class_id) {
                                    c.name.clone()
                                } else {
                                    "Unknown".to_string()
                                };
                                return ExecutionResult::Error(format!("Cannot use object of type {} as array", class_name));
                            }
                            Err(e) => return ExecutionResult::Error(e),
                        }
                    } else if let Some(str_ptr) = array_val.as_string_ptr() {
                        let s = unsafe { &mut *(str_ptr as *mut String) };
                        let idx_opt = if let Some(ki) = key.as_int() {
                            if ki < 0 {
                                let positive = s.len() as i32 + ki;
                                if positive >= 0 { Some(positive as usize) } else { None }
                            } else {
                                Some(ki as usize)
                            }
                        } else if let Some(ks_ptr) = key.as_string_ptr() {
                            let ks = unsafe { &*(ks_ptr as *const String) };
                            if let Ok(ki) = ks.parse::<i32>() {
                                if ki < 0 {
                                    let positive = s.len() as i32 + ki;
                                    if positive >= 0 { Some(positive as usize) } else { None }
                                } else {
                                    Some(ki as usize)
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        if let Some(idx) = idx_opt {
                            let char_byte = if let Some(vs_ptr) = value.as_string_ptr() {
                                let vs = unsafe { &*(vs_ptr as *const String) };
                                vs.as_bytes().first().map(|v| *v)
                            } else if let Some(vi) = value.as_int() {
                                Some(vi as u8)
                            } else {
                                None
                            };
                            if let Some(b) = char_byte {
                                if idx < s.len() {
                                    unsafe {
                                        s.as_bytes_mut()[idx] = b;
                                    }
                                } else if idx == s.len() {
                                    s.push(b as char);
                                } else {
                                    while s.len() < idx {
                                        s.push(' ');
                                    }
                                    s.push(b as char);
                                }
                            }
                        }
                    }
                    // Return the assigned value (assignment is an expression)
                    Self::push(fibre, value);
                }
                Opcode::ArrayIsset => {
                    let key = Self::pop(fibre).deref();
                    let array_val = Self::pop(fibre).deref();

                    if let Some(arr_ptr) = array_val.as_array_ptr() {
                        let arr = unsafe {
                            &*(arr_ptr as *const hyperion_core::types::array::PhpArray)
                        };
                        let is_set = if let Some(key_str) = key.as_string_ptr() {
                            let key_s = unsafe { &*(key_str as *const String) };
                            let first_byte = key_s.as_bytes().first();
                            let is_possible_int = first_byte.map_or(false, |&b| b.is_ascii_digit() || b == b'-' || b == b'+');
                            if is_possible_int {
                                if let Ok(key_int) = key_s.parse::<i64>() {
                                    arr.get_int(key_int).map(|v| !v.deref().is_null()).unwrap_or(false)
                                } else {
                                    arr.get_string_id(hyperion_core::types::string_table::intern_string(key_s))
                                        .map(|v| !v.deref().is_null())
                                        .unwrap_or(false)
                                }
                            } else {
                                arr.get_string_id(hyperion_core::types::string_table::intern_string(key_s))
                                    .map(|v| !v.deref().is_null())
                                    .unwrap_or(false)
                            }
                        } else if let Some(key_int) = key.as_int() {
                            arr.get_int(key_int as i64)
                                .map(|v| !v.deref().is_null())
                                .unwrap_or(false)
                        } else {
                            false
                        };
                        Self::push(fibre, Value::new_bool(is_set));
                    } else if let Some(str_ptr) = array_val.as_string_ptr() {
                        let s = unsafe { &*(str_ptr as *const String) };
                        let idx_opt = if let Some(ki) = key.as_int() {
                            if ki < 0 {
                                let positive = s.len() as i32 + ki;
                                if positive >= 0 { Some(positive as usize) } else { None }
                            } else {
                                Some(ki as usize)
                            }
                        } else if let Some(ks_ptr) = key.as_string_ptr() {
                            let ks = unsafe { &*(ks_ptr as *const String) };
                            if let Ok(ki) = ks.parse::<i32>() {
                                if ki < 0 {
                                    let positive = s.len() as i32 + ki;
                                    if positive >= 0 { Some(positive as usize) } else { None }
                                } else {
                                    Some(ki as usize)
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        let is_set = idx_opt.map(|i| i < s.len()).unwrap_or(false);
                        Self::push(fibre, Value::new_bool(is_set));
                    } else if array_val.is_object() {
                        let class_id = unsafe {
                            &*(array_val.as_object_ptr().unwrap()
                                as *const hyperion_core::types::object::PhpObject)
                        }
                        .class_id;
                        if Self::class_has_method(fibre, class_id, "offsetExists") {
                            let exists_res = Self::call_method_synchronously(
                                fibre, array_val, "offsetExists", vec![key],
                            );
                            match exists_res {
                                Ok(exists_val) => {
                                    if exists_val.as_bool() == Some(true) {
                                        if Self::class_has_method(fibre, class_id, "offsetGet") {
                                            let get_res = Self::call_method_synchronously(
                                                fibre, array_val, "offsetGet", vec![key],
                                            );
                                            match get_res {
                                                Ok(val) => {
                                                    Self::push(fibre, Value::new_bool(!val.deref().is_null()));
                                                }
                                                Err(_) => {
                                                    Self::push(fibre, Value::new_bool(false));
                                                }
                                            }
                                        } else {
                                            Self::push(fibre, Value::new_bool(true));
                                        }
                                    } else {
                                        Self::push(fibre, Value::new_bool(false));
                                    }
                                }
                                Err(_) => {
                                    Self::push(fibre, Value::new_bool(false));
                                }
                            }
                        } else {
                            Self::push(fibre, Value::new_bool(false));
                        }
                    } else {
                        Self::push(fibre, Value::new_bool(false));
                    }
                }
                Opcode::PropertyIsset => {
                    let name_idx = Self::read_short(fibre);
                    let obj = Self::pop(fibre).deref();

                    if let Some(obj_ptr) = obj.as_object_ptr() {
                        let php_obj = unsafe {
                            &*(obj_ptr as *const hyperion_core::types::object::PhpObject)
                        };
                        let frame = fibre.frames[fibre.frame_count - 1];
                        let func = unsafe { &*frame.function.0 };
                        let name_val = func.chunk.constants[name_idx as usize];

                        if let Some(name_str_ptr) = name_val.as_string_ptr() {
                            let name_str = unsafe { &*(name_str_ptr as *const String) };
                            if let Some(val) = php_obj.properties.get(name_str) {
                                Self::push(fibre, Value::new_bool(!val.deref().is_null()));
                            } else {
                                let class_id = Self::resolve_obj_class_id(fibre, php_obj);
                                if Self::class_has_method(fibre, class_id, "__isset") {
                                    let guard_key = (obj_ptr as usize, name_str.clone(), 3u8);
                                    if fibre.magic_guards.contains(&guard_key) {
                                        Self::push(fibre, Value::new_bool(false));
                                    } else {
                                        fibre.magic_guards.insert(guard_key.clone());
                                        let call_res = Self::call_method_synchronously(fibre, obj, "__isset", vec![name_val]);
                                        fibre.magic_guards.remove(&guard_key);
                                        match call_res {
                                            Ok(res) => {
                                                Self::push(fibre, Value::new_bool(res.is_truthy()));
                                            }
                                            Err(e) => return ExecutionResult::Error(e),
                                        }
                                    }
                                } else {
                                    Self::push(fibre, Value::new_bool(false));
                                }
                            }
                        } else {
                            Self::push(fibre, Value::new_bool(false));
                        }
                    } else {
                        Self::push(fibre, Value::new_bool(false));
                    }
                }
                Opcode::PropertyIssetDynamic => {
                    let prop_name = Self::pop(fibre).deref();
                    let obj = Self::pop(fibre).deref();

                    if let Some(obj_ptr) = obj.as_object_ptr() {
                        let php_obj = unsafe {
                            &*(obj_ptr as *const hyperion_core::types::object::PhpObject)
                        };
                        let class_id = Self::resolve_obj_class_id(fibre, php_obj);
                        if let Some(name_str_ptr) = prop_name.as_string_ptr() {
                            let name_str = unsafe { &*(name_str_ptr as *const String) };
                            if let Some(val) = php_obj.properties.get(name_str) {
                                Self::push(fibre, Value::new_bool(!val.deref().is_null()));
                            } else if Self::class_has_method(fibre, class_id, "__isset") {
                                let guard_key = (obj_ptr as usize, name_str.clone(), 3u8);
                                if fibre.magic_guards.contains(&guard_key) {
                                    Self::push(fibre, Value::new_bool(false));
                                } else {
                                    fibre.magic_guards.insert(guard_key.clone());
                                    let call_res = Self::call_method_synchronously(fibre, obj, "__isset", vec![prop_name]);
                                    fibre.magic_guards.remove(&guard_key);
                                    match call_res {
                                        Ok(res) => {
                                            Self::push(fibre, Value::new_bool(res.is_truthy()));
                                        }
                                        Err(e) => return ExecutionResult::Error(e),
                                    }
                                }
                            } else {
                                Self::push(fibre, Value::new_bool(false));
                            }
                        } else {
                            Self::push(fibre, Value::new_bool(false));
                        }
                    } else {
                        Self::push(fibre, Value::new_bool(false));
                    }
                }
                Opcode::ArrayDelete => {
                    let key = Self::pop(fibre);
                    let array_val = Self::pop(fibre);

                    if let Some(arr_ptr) = array_val.as_array_ptr() {
                        let arr = unsafe {
                            &mut *(arr_ptr as *mut hyperion_core::types::array::PhpArray)
                        };
                        if let Some(key_str) = key.as_string_ptr() {
                            let key_s = unsafe { (*(key_str as *const String)).clone() };
                            if let Ok(key_int) = key_s.parse::<i64>() {
                                arr.remove_int(key_int);
                            } else {
                                arr.remove_string_id(hyperion_core::types::string_table::intern_string(&key_s));
                            }
                        } else if let Some(key_int) = key.as_int() {
                            arr.remove_int(key_int as i64);
                        }
                    } else if array_val.is_object() {
                        // `unset($obj[$k])` on an ArrayAccess implementation.
                        let class_id = unsafe {
                            &*(array_val.as_object_ptr().unwrap()
                                as *const hyperion_core::types::object::PhpObject)
                        }
                        .class_id;
                        if Self::class_has_method(fibre, class_id, "offsetUnset") {
                            if let Err(e) = Self::call_method_synchronously(
                                fibre, array_val, "offsetUnset", vec![key],
                            ) {
                                return ExecutionResult::Error(e);
                            }
                        }
                    }
                }
                Opcode::PropertyDelete => {
                    // `unset($obj->prop)`. PHP removes the property outright, so
                    // a later isset() is false and __get/__set take over.
                    let name = Self::pop(fibre);
                    let obj_val = Self::pop(fibre);
                    if let (Some(obj_ptr), Some(name_ptr)) =
                        (obj_val.as_object_ptr(), name.as_string_ptr())
                    {
                        let obj = unsafe {
                            &mut *(obj_ptr as *mut hyperion_core::types::object::PhpObject)
                        };
                        let name_s = unsafe { &*(name_ptr as *const String) };
                        obj.properties.shift_remove(name_s.as_str());
                    }
                }
                Opcode::Inc => {
                    let val = Self::pop(fibre);
                    if let Some(tracer) = &mut fibre.tracer {
                        if tracer.is_recording {
                            let src_idx = fibre.stack_top;
                            if val.is_int() {
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardType {
                                    src: src_idx,
                                    expected_type_tag: hyperion_core::memory::nan_box::QNAN
                                        | hyperion_core::memory::nan_box::TAG_INT,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top + 1,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::Constant {
                                    dest: src_idx + 1,
                                    value: 0x7FF8000000000000 | 0x0004000000000000 | 1, // QNAN | TAG_INT | 1
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::Add {
                                    dest: src_idx,
                                    left: src_idx,
                                    right: src_idx + 1,
                                });
                            } else if val.is_float() {
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardFloat {
                                    src: src_idx,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top + 1,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::Constant {
                                    dest: src_idx + 1,
                                    value: 1.0f64.to_bits(),
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::FloatAdd {
                                    dest: src_idx,
                                    left: src_idx,
                                    right: src_idx + 1,
                                });
                            } else {
                                fibre.tracer = None;
                            }
                        }
                    }
                    if let Some(i) = val.as_int() {
                        Self::push(fibre, Value::new_int(i + 1));
                    } else if let Some(f) = val.as_float() {
                        Self::push(fibre, Value::new_float(f + 1.0));
                    } else {
                        Self::push(fibre, Value::new_int(1));
                    }
                }
                Opcode::Dec => {
                    let val = Self::pop(fibre);
                    if let Some(tracer) = &mut fibre.tracer {
                        if tracer.is_recording {
                            let src_idx = fibre.stack_top;
                            if val.is_int() {
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardType {
                                    src: src_idx,
                                    expected_type_tag: hyperion_core::memory::nan_box::QNAN
                                        | hyperion_core::memory::nan_box::TAG_INT,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top + 1,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::Constant {
                                    dest: src_idx + 1,
                                    value: 0x7FF8000000000000 | 0x0004000000000000 | 1,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::Subtract {
                                    dest: src_idx,
                                    left: src_idx,
                                    right: src_idx + 1,
                                });
                            } else if val.is_float() {
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardFloat {
                                    src: src_idx,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top + 1,
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::Constant {
                                    dest: src_idx + 1,
                                    value: 1.0f64.to_bits(),
                                });
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::FloatSubtract {
                                    dest: src_idx,
                                    left: src_idx,
                                    right: src_idx + 1,
                                });
                            } else {
                                fibre.tracer = None;
                            }
                        }
                    }
                    if let Some(i) = val.as_int() {
                        Self::push(fibre, Value::new_int(i - 1));
                    } else if let Some(f) = val.as_float() {
                        Self::push(fibre, Value::new_float(f - 1.0));
                    } else {
                        Self::push(fibre, Value::new_int(-1));
                    }
                }
                Opcode::ReturnValue => {
                    let mut ret_val = Self::pop(fibre);
                    let frame = fibre.frames[fibre.frame_count - 1];
                    let chunk = &unsafe { &*frame.function.0 }.chunk;
                    let old_stack_top = frame.stack_window;
                    if let Some(override_val) = frame.return_override {
                        ret_val = override_val;
                    }

                    let mut handled_by_finally = false;
                    for fh in &chunk.finally_handlers {
                        if frame.ip > fh.start_ip && frame.ip - 1 < fh.end_ip {
                            fibre.unwind_action = Some(crate::fibre::UnwindAction::Return(ret_val));
                            fibre.frames[fibre.frame_count - 1].ip = fh.finally_ip;
                            handled_by_finally = true;
                            break;
                        }
                    }
                    if handled_by_finally {
                        continue;
                    }

                    let mut is_autoload = false;
                    if let Some(&depth) = fibre.autoload_frame_depths.last() {
                        if depth == fibre.frame_count - 1 {
                            is_autoload = true;
                            fibre.autoload_frame_depths.pop();
                        }
                    }

                    if let Some(parent_window) = frame.eval_parent_stack_window {
                        if fibre.frame_count >= 2 {
                            let parent_frame = &fibre.frames[fibre.frame_count - 2];
                            let parent_chunk = &unsafe { &*parent_frame.function.0 }.chunk;
                            let curr_chunk = &unsafe { &*frame.function.0 }.chunk;
                            let parent_frame_idx = fibre.frame_count - 2;
                            for (curr_idx, curr_name) in curr_chunk.local_names.iter().enumerate() {
                                let val = fibre.stack[frame.stack_window + curr_idx];
                                if let Some(p_idx) = parent_chunk.local_names.iter().position(|n| n == curr_name) {
                                    fibre.stack[parent_window + p_idx] = val;
                                }
                                if let Some(map) = fibre.dynamic_locals.get_mut(&parent_frame_idx) {
                                    map.insert(curr_name.clone(), val);
                                }
                            }
                        }
                    }

                    let returning_frame_idx = fibre.frame_count - 1;
                    if let Some(path) = fibre.frame_included_paths.remove(&returning_frame_idx) {
                        fibre.included_file_returns.insert(path, ret_val);
                    }

                    fibre.dynamic_locals.remove(&(fibre.frame_count - 1));
                    fibre.frame_count -= 1;
                    fibre.stack_top = old_stack_top;

                    if let Some(tracer) = &mut fibre.tracer {
                        if tracer.is_recording && tracer.inline_depth > 0 {
                            tracer.inline_depth -= 1;
                        }
                    }

                    if !is_autoload {
                        Self::push(fibre, ret_val);
                    }
                    if fibre.frame_count == 0 {
                        return ExecutionResult::Finished;
                    }
                }
                Opcode::Return => {
                    let frame = fibre.frames[fibre.frame_count - 1];
                    let chunk = &unsafe { &*frame.function.0 }.chunk;
                    let old_stack_top = frame.stack_window;

                    let mut handled_by_finally = false;
                    for fh in &chunk.finally_handlers {
                        if frame.ip > fh.start_ip && frame.ip - 1 < fh.end_ip {
                            fibre.unwind_action = Some(crate::fibre::UnwindAction::Return(
                                frame.return_override.unwrap_or_else(Value::null),
                            ));
                            fibre.frames[fibre.frame_count - 1].ip = fh.finally_ip;
                            handled_by_finally = true;
                            break;
                        }
                    }
                    if handled_by_finally {
                        continue;
                    }

                    let mut is_autoload = false;
                    if let Some(&depth) = fibre.autoload_frame_depths.last() {
                        if depth == fibre.frame_count - 1 {
                            is_autoload = true;
                            fibre.autoload_frame_depths.pop();
                        }
                    }

                    let returning_frame_idx = fibre.frame_count - 1;
                    if let Some(path) = fibre.frame_included_paths.remove(&returning_frame_idx) {
                        fibre.included_file_returns.insert(path, Value::null());
                    }



                    if let Some(parent_window) = frame.eval_parent_stack_window {
                        if fibre.frame_count >= 2 {
                            let parent_frame = &fibre.frames[fibre.frame_count - 2];
                            let parent_chunk = &unsafe { &*parent_frame.function.0 }.chunk;
                            let curr_chunk = &unsafe { &*frame.function.0 }.chunk;
                            let parent_frame_idx = fibre.frame_count - 2;
                            for (curr_idx, curr_name) in curr_chunk.local_names.iter().enumerate() {
                                let val = fibre.stack[frame.stack_window + curr_idx];
                                if let Some(p_idx) = parent_chunk.local_names.iter().position(|n| n == curr_name) {
                                    fibre.stack[parent_window + p_idx] = val;
                                }
                                if let Some(map) = fibre.dynamic_locals.get_mut(&parent_frame_idx) {
                                    map.insert(curr_name.clone(), val);
                                }
                            }
                        }
                    }

                    fibre.dynamic_locals.remove(&(fibre.frame_count - 1));
                    fibre.frame_count -= 1;
                    fibre.stack_top = old_stack_top;

                    if let Some(tracer) = &mut fibre.tracer {
                        if tracer.is_recording && tracer.inline_depth > 0 {
                            tracer.inline_depth -= 1;
                        }
                    }

                    if !is_autoload {
                        // A constructor frame falls off its end without an explicit
                        // `return`; return_override carries the new object, which must
                        // be produced here instead of null.
                        Self::push(fibre, frame.return_override.unwrap_or_else(Value::null));
                    }
                    if fibre.frame_count == 0 {
                        return ExecutionResult::Finished;
                    }
                }
                Opcode::Eval => {
                    let code_val = Self::pop(fibre);
                    let mut code_str = Self::cast_value_to_string(fibre, code_val).unwrap_or_else(|_| "".to_string());
                    if !code_str.trim_start().starts_with("<?php") && !code_str.trim_start().starts_with("<?=") {
                        code_str = format!("<?php\n{}", code_str);
                    }
                    
                    let lexer = hyperion_parser::lexer::Lexer::new(&code_str);
                    let mut parser = hyperion_parser::parser::Parser::new(lexer);
                    let ast = parser.parse_program();
                    
                    let frame = fibre.frames[fibre.frame_count - 1];
                    let chunk = unsafe { &(*frame.function.0).chunk };
                    
                    let mut compiler = hyperion_compiler::compiler::Compiler::new("eval_code".to_string());
                    compiler.prefill_locals(&chunk.local_names);
                    
                    let mut eval_result = compiler.compile(ast);
                    fibre.engine_state.merge_compilation_result(&mut eval_result, "eval");

                    let eval_chunk = eval_result.main_chunk;
                    let num_eval_locals = eval_chunk.local_names.len();
                    let parent_stack_window = frame.stack_window;
                    let parent_locals_count = chunk.local_names.len();
                    
                    let func = Box::new(crate::types::function::PhpFunction::new(
                        "eval".to_string(),
                        0,
                        eval_chunk,
                    ));
                    let f_ptr = crate::types::function::FunctionPtr(Box::into_raw(func));
                    fibre.arena.track_boxed(f_ptr.0 as *mut crate::types::function::PhpFunction);
                    
                    let new_stack_window = fibre.stack_top;
                    
                    for i in 0..parent_locals_count {
                        let val = fibre.stack[parent_stack_window + i];
                        Self::push(fibre, val);
                    }
                    
                    for _ in parent_locals_count..num_eval_locals {
                        Self::push(fibre, Value::null());
                    }
                    
                    let new_frame = crate::fibre::CallFrame {
                        function: f_ptr,
                        ip: 0,
                        stack_window: new_stack_window,
                        called_class_id: 0,
                        return_override: None,
                        eval_parent_stack_window: Some(parent_stack_window),
                        arity: 0,
                    };
                    
                    fibre.frames[fibre.frame_count] = new_frame;
                    fibre.frame_count += 1;
                    continue 'vm_loop;
                }
                Opcode::GetLocalDynamic => {
                    let var_name = Self::pop(fibre);
                    let name_str = Self::cast_value_to_string(fibre, var_name).unwrap_or_else(|_| "".to_string());
                    
                    let frame_idx = fibre.frame_count - 1;
                    let frame = &fibre.frames[frame_idx];
                    let chunk = unsafe { &(*frame.function.0).chunk };
                    
                    if let Some(local_idx) = chunk.local_names.iter().position(|n| n == &name_str) {
                        let val = fibre.stack[frame.stack_window + local_idx];
                        Self::push(fibre, val);
                    } else if let Some(val) = fibre.dynamic_locals.get(&frame_idx).and_then(|m| m.get(&name_str)) {
                        Self::push(fibre, *val);
                    } else {
                        // In PHP, undefined dynamic variable results in Warning and null.
                        // For simplicity, we just push null.
                        Self::push(fibre, Value::null());
                    }
                }
                Opcode::SetLocalDynamic => {
                    let raw_val = Self::pop(fibre);
                    let val = if raw_val.deref().is_array() {
                        Self::deep_copy_value(fibre, raw_val.deref())
                    } else {
                        raw_val.deref()
                    };
                    let var_name = Self::pop(fibre);
                    let name_str = Self::cast_value_to_string(fibre, var_name).unwrap_or_else(|_| "".to_string());

                    let frame_idx = fibre.frame_count - 1;
                    let frame = &fibre.frames[frame_idx];
                    let chunk = unsafe { &(*frame.function.0).chunk };

                    if let Some(local_idx) = chunk.local_names.iter().position(|n| n == &name_str) {
                        let slot = frame.stack_window + local_idx;
                        let existing = fibre.stack[slot];
                        if let Some(cell) = existing.as_ref_ptr() {
                            unsafe {
                                (*(cell as *mut hyperion_core::types::reference::PhpRef)).set(val)
                            };
                        } else {
                            fibre.stack[slot] = val;
                        }
                    } else {
                        fibre.dynamic_locals.entry(frame_idx).or_default().insert(name_str, val);
                    }
                    Self::push(fibre, val);
                }
                Opcode::JumpBack => {
                    // Legacy opcode (unused with Loop), kept for compat
                    let _offset = Self::read_short(fibre);
                }
                Opcode::CallNativeMethodForward => {
                    let name_len = Self::read_byte(fibre) as usize;
                    let mut name_bytes = Vec::with_capacity(name_len);
                    for _ in 0..name_len {
                        name_bytes.push(Self::read_byte(fibre));
                    }
                    let raw_func_name = String::from_utf8(name_bytes).unwrap_or_default();
                    let frame = fibre.frames[fibre.frame_count - 1];
                    let old_stack_top = frame.stack_window;
                    let arity = frame.arity;
                    let args = fibre.stack[old_stack_top..old_stack_top + 1 + arity].to_vec();

                    if raw_func_name.starts_with("native_generator_") {
                        let this_val = args[0].deref();
                        if let Some(obj_ptr) = this_val.as_object_ptr() {
                            let obj = unsafe { &mut *(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
                            if let Some(gen_val) = obj.properties.get("__generator") {
                                let gen_fibre_ptr = gen_val.as_object_ptr().unwrap() as *mut Fibre;
                                let gen_fibre = unsafe { &mut *gen_fibre_ptr };
                                let ret_val = match raw_func_name.as_str() {
                                    "native_generator_rewind" => {
                                        if !gen_fibre.generator_started {
                                            gen_fibre.generator_started = true;
                                            let res = Self::step_generator_fibre(gen_fibre);
                                            match res {
                                                ExecutionResult::Yielded(YieldReason::GeneratorYield) => {
                                                    gen_fibre.generator_valid = true;
                                                }
                                                ExecutionResult::Finished => {
                                                    gen_fibre.generator_valid = false;
                                                }
                                                ExecutionResult::Error(err) => return ExecutionResult::Error(err),
                                                ExecutionResult::UncaughtException(err) => return ExecutionResult::UncaughtException(err),
                                                _ => gen_fibre.generator_valid = false,
                                            }
                                        }
                                        Value::null()
                                    }
                                    "native_generator_valid" => {
                                        Value::new_bool(gen_fibre.generator_valid)
                                    }
                                    "native_generator_current" => {
                                        if gen_fibre.generator_valid {
                                            gen_fibre.generator_value
                                        } else {
                                            Value::null()
                                        }
                                    }
                                    "native_generator_key" => {
                                        if gen_fibre.generator_valid {
                                            gen_fibre.generator_key
                                        } else {
                                            Value::null()
                                        }
                                    }
                                    "native_generator_next" => {
                                        if !gen_fibre.generator_started {
                                            gen_fibre.generator_started = true;
                                        }
                                        let res = Self::step_generator_fibre(gen_fibre);
                                        match res {
                                            ExecutionResult::Yielded(YieldReason::GeneratorYield) => {
                                                gen_fibre.generator_valid = true;
                                            }
                                            ExecutionResult::Finished => {
                                                gen_fibre.generator_valid = false;
                                            }
                                            ExecutionResult::Error(err) => return ExecutionResult::Error(err),
                                            ExecutionResult::UncaughtException(err) => return ExecutionResult::UncaughtException(err),
                                            _ => gen_fibre.generator_valid = false,
                                        }
                                        Value::null()
                                    }
                                    "native_generator_send" => {
                                        let send_val = args.get(1).copied().unwrap_or(Value::null());
                                        if !gen_fibre.generator_started {
                                            gen_fibre.generator_started = true;
                                        } else {
                                            gen_fibre.generator_value = send_val;
                                        }
                                        let res = Self::step_generator_fibre(gen_fibre);
                                        match res {
                                            ExecutionResult::Yielded(YieldReason::GeneratorYield) => {
                                                gen_fibre.generator_valid = true;
                                            }
                                            ExecutionResult::Finished => {
                                                gen_fibre.generator_valid = false;
                                            }
                                            ExecutionResult::Error(err) => return ExecutionResult::Error(err),
                                            ExecutionResult::UncaughtException(err) => return ExecutionResult::UncaughtException(err),
                                            _ => gen_fibre.generator_valid = false,
                                        }
                                        if gen_fibre.generator_valid {
                                            gen_fibre.generator_value
                                        } else {
                                            Value::null()
                                        }
                                    }
                                    "native_generator_throw" => {
                                        let ex_val = args.get(1).copied().unwrap_or(Value::null());
                                        let msg = if let Some(ptr) = ex_val.as_object_ptr() {
                                            let obj = unsafe { &*(ptr as *const hyperion_core::types::object::PhpObject) };
                                            if let Some(msg_val) = obj.properties.get("message") {
                                                if let Some(msg_ptr) = msg_val.as_string_ptr() {
                                                    unsafe { (*(msg_ptr as *const String)).clone() }
                                                } else {
                                                    "Uncaught exception".to_string()
                                                }
                                            } else {
                                                "Uncaught exception".to_string()
                                            }
                                        } else {
                                            "Uncaught exception".to_string()
                                        };
                                        return ExecutionResult::UncaughtException(msg);
                                    }
                                    "native_generator_get_return" => {
                                        if gen_fibre.generator_valid {
                                            return ExecutionResult::Error("Cannot get return value of a generator that hasn't returned".to_string());
                                        }
                                        gen_fibre.generator_value
                                    }
                                    _ => Value::null(),
                                };

                                fibre.dynamic_locals.remove(&(fibre.frame_count - 1));
                                fibre.frame_count -= 1;
                                fibre.stack_top = old_stack_top;
                                Self::push(fibre, ret_val);
                                if fibre.frame_count == 0 {
                                    return ExecutionResult::Finished;
                                }
                                continue 'vm_loop;
                            }
                        }
                    }

                    let registry = crate::stdlib::StdlibRegistry::global();
                    let native_fn = registry.functions.get(&raw_func_name);
                    if let Some(f) = native_fn {
                        match f(&args, fibre) {
                            Ok(ret_val) => {
                                fibre.dynamic_locals.remove(&(fibre.frame_count - 1));
                                fibre.frame_count -= 1;
                                fibre.stack_top = old_stack_top;
                                Self::push(fibre, ret_val);
                                if fibre.frame_count == 0 {
                                    return ExecutionResult::Finished;
                                }
                            }
                            Err(e) => {
                                if e == "__HYPERION_EXIT__" {
                                    fibre.frame_count = 0;
                                    return ExecutionResult::Finished;
                                }
                                if let Some(res) = Self::create_and_throw_native_error(fibre, &e) {
                                    return res;
                                }
                            }
                        }
                    } else {
                        return ExecutionResult::Error(format!("Native method {} not found", raw_func_name));
                    }
                }
                Opcode::CallNativeForward => {
                    let name_len = Self::read_byte(fibre) as usize;
                    let mut name_bytes = Vec::with_capacity(name_len);
                    for _ in 0..name_len {
                        name_bytes.push(Self::read_byte(fibre));
                    }
                    let raw_func_name = String::from_utf8(name_bytes).unwrap_or_default();
                    let frame = fibre.frames[fibre.frame_count - 1];
                    let old_stack_top = frame.stack_window;
                    let arity = frame.arity;
                    let args = fibre.stack[old_stack_top + 1..old_stack_top + 1 + arity].to_vec();

                    let registry = crate::stdlib::StdlibRegistry::global();
                    let norm = normalize_name(&raw_func_name);
                    let native_fn = registry.functions.get(&norm).or_else(|| {
                        if norm.contains('\\') {
                            let local = norm.split('\\').next_back().unwrap();
                            registry.functions.get(local)
                        } else {
                            None
                        }
                    });
                    if let Some(f) = native_fn {
                        match f(&args, fibre) {
                            Ok(ret_val) => {
                                fibre.dynamic_locals.remove(&(fibre.frame_count - 1));
                                fibre.frame_count -= 1;
                                fibre.stack_top = old_stack_top;
                                Self::push(fibre, ret_val);
                                if fibre.frame_count == 0 {
                                    return ExecutionResult::Finished;
                                }
                            }
                            Err(e) => {
                                if e == "__HYPERION_EXIT__" {
                                    fibre.frame_count = 0;
                                    return ExecutionResult::Finished;
                                }
                                if let Some(res) = Self::create_and_throw_native_error(fibre, &e) {
                                    return res;
                                }
                            }
                        }
                    } else {
                        return ExecutionResult::Error(format!("Native function {} not found", raw_func_name));
                    }
                }
                Opcode::CallNative => {
                    if let Some(tracer) = &mut fibre.tracer {
                        if tracer.is_recording {
                            tracer.stop();
                        }
                    }
                    let name_len = Self::read_byte(fibre) as usize;
                    let mut name_bytes = Vec::with_capacity(name_len);
                    for _ in 0..name_len {
                        name_bytes.push(Self::read_byte(fibre));
                    }
                    let arity = Self::read_byte(fibre) as usize;
                    let raw_func_name = String::from_utf8(name_bytes).unwrap_or_default();

                    

                    let fq_func_name = normalize_name(&raw_func_name);
                    let func_name = if let Some(last_slash) = fq_func_name.rfind('\\') {
                        fq_func_name[last_slash + 1..].to_string()
                    } else {
                        fq_func_name.clone()
                    };

                    let mut args = Vec::with_capacity(arity);
                    for _ in 0..arity {
                        args.push(Self::pop(fibre));
                    }
                    args.reverse();
                    
                    if func_name == "closure_bind" {
                        debug_println!("DEBUG CallNative closure_bind: popped args = {:?}", args);
                    }

                    // --- Language constructs ---

                    if func_name == "exit" || func_name == "die" {
                        if let Some(msg_val) = args.first() {
                            let deref = msg_val.deref();
                            if let Some(s_ptr) = deref.as_string_ptr() {
                                let s = unsafe { &*(s_ptr as *const String) };
                                fibre.output_buffer.extend_from_slice(s.as_bytes());
                            }
                        }
                        fibre.frame_count = 0;
                        return ExecutionResult::Finished;
                    }

                    if func_name == "extract" {
                        let mut count = 0;
                        if let Some(arr_val) = args.first() {
                            let deref_arr = arr_val.deref();
                            if let Some(arr_ptr) = deref_arr.as_array_ptr() {
                                let arr = unsafe { &*(arr_ptr as *const hyperion_core::types::array::PhpArray) };
                                let top_func_name = unsafe { &*fibre.frames[fibre.frame_count - 1].function.0 }.name.clone();
                                let frame_idx = if fibre.frame_count >= 2 && (top_func_name == "extract" || top_func_name.is_empty()) {
                                    fibre.frame_count - 2
                                } else {
                                    fibre.frame_count - 1
                                };
                                let frame = &fibre.frames[frame_idx];
                                let chunk = unsafe { &(*frame.function.0).chunk };
                                let stack_window = frame.stack_window;
                                for (k, v) in arr.elements.iter() {
                                    let key_str = match k {
                                        hyperion_core::types::array::ArrayKey::StringId(id) => {
                                            hyperion_core::types::string_table::lookup_string(*id).unwrap_or_default()
                                        }
                                        hyperion_core::types::array::ArrayKey::Int(i) => i.to_string(),
                                    };
                                    if !key_str.is_empty() && (key_str.chars().next().unwrap().is_alphabetic() || key_str.starts_with('_')) {
                                        if let Some(pos) = chunk.local_names.iter().position(|n| n == &key_str) {
                                            fibre.stack[stack_window + pos] = *v;
                                        }
                                        fibre.dynamic_locals.entry(frame_idx).or_default().insert(key_str, *v);
                                        count += 1;
                                    }
                                }
                            }
                        }
                        Self::push(fibre, Value::new_int(count));
                        continue;
                    }

                    if func_name == "compact" {
                        fn flatten_compact_args(val: &Value, names: &mut Vec<String>) {
                            let deref_val = val.deref();
                            if let Some(s_ptr) = deref_val.as_string_ptr() {
                                let s = unsafe { &*(s_ptr as *const String) };
                                names.push(s.clone());
                            } else if let Some(arr_ptr) = deref_val.as_array_ptr() {
                                let arr = unsafe { &*(arr_ptr as *const hyperion_core::types::array::PhpArray) };
                                for (_, elem) in arr.elements.iter() {
                                    flatten_compact_args(elem, names);
                                }
                            }
                        }
                        let mut names = Vec::new();
                        for arg in &args {
                            flatten_compact_args(arg, &mut names);
                        }
                        let frame_idx = fibre.frame_count - 1;
                        let frame = &fibre.frames[frame_idx];
                        let chunk = unsafe { &(*frame.function.0).chunk };
                        let stack_window = frame.stack_window;
                        
                        let mut res_arr = hyperion_core::types::array::PhpArray::new();
                        for name in names {
                            let val_opt = if let Some(pos) = chunk.local_names.iter().position(|n| n == &name) {
                                let val = fibre.stack[stack_window + pos].deref();
                                if !val.is_null() { Some(val) } else { None }
                            } else if let Some(dyn_val) = fibre.dynamic_locals.get(&frame_idx).and_then(|m| m.get(&name)) {
                                let val = dyn_val.deref();
                                if !val.is_null() { Some(val) } else { None }
                            } else {
                                None
                            };
                            if let Some(val) = val_opt {
                                let id = hyperion_core::types::string_table::intern_string(&name);
                                res_arr.insert_string_id(id, val);
                            }
                        }
                        let ptr = fibre.arena.alloc_and_track(res_arr);
                        Self::push(fibre, Value::new_array_ptr(ptr as *mut ()));
                        continue;
                    }

                    if func_name == "define" {
                        if args.len() >= 2 {
                            if let Some(name_ptr) = args[0].as_string_ptr() {
                                let name_str = unsafe { &*(name_ptr as *const String) };
                                let persistent_val = Fibre::clone_value_to_global(&fibre.engine_state, &args[1]);
                                fibre.engine_state.constants
                                    .insert(name_str.clone(), persistent_val);
                                Self::push(fibre, Value::new_bool(true));
                                continue;
                            }
                        }
                        Self::push(fibre, Value::new_bool(false));
                        continue;
                    }

                    if func_name == "class_exists"
                        || func_name == "interface_exists"
                        || func_name == "trait_exists"
                        || func_name == "enum_exists"
                    {
                        let mut name_str = String::new();
                        let check_kind = |engine_state: &crate::fibre::GlobalEngineState, norm_class: &str| -> bool {
                            if let Some(cid) = engine_state.class_map.get(norm_class).map(|v| *v) {
                                if let Some(c) = engine_state.classes.get(&cid) {
                                    match func_name.as_str() {
                                        "class_exists" => !c.is_interface && !c.is_trait,
                                        "interface_exists" => c.is_interface,
                                        "trait_exists" => c.is_trait,
                                        "enum_exists" => c.is_enum,
                                        _ => false,
                                    }
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        };

                        let mut exists = false;
                        if let Some(args0) = args.first() {
                            let val = args0.deref();
                            if let Some(name_ptr) = val.as_string_ptr() {
                                name_str = unsafe { &*(name_ptr as *const String) }.clone();
                                let norm_class = normalize_name(&name_str);
                                exists = check_kind(&fibre.engine_state, &norm_class);
                            }
                        }

                        let should_autoload = if args.len() >= 2 {
                            let a1 = args[1].deref();
                            if let Some(b) = a1.as_bool() {
                                b
                            } else if let Some(i) = a1.as_int() {
                                i != 0
                            } else {
                                true
                            }
                        } else {
                            true
                        };

                        if !exists && should_autoload && !name_str.is_empty() {
                            fibre.trigger_autoload_sync(&name_str);
                            let norm_class = normalize_name(&name_str);
                            exists = check_kind(&fibre.engine_state, &norm_class);
                        }

                        Self::push(fibre, Value::new_bool(exists));
                        continue;
                    }

                    if func_name == "function_exists" {
                        let exists = if let Some(args0) = args.first() {
                            if let Some(name_ptr) = args0.as_string_ptr() {
                                let name_str = unsafe { &*(name_ptr as *const String) };
                                let norm_func = normalize_name(name_str);
                                let registry = StdlibRegistry::global();
                                if registry.functions.contains_key(&norm_func) {
                                    true
                                } else if fibre.engine_state.func_map.contains_key(&norm_func) {
                                    true
                                } else if norm_func.contains('\\') {
                                    let local_name = norm_func.split('\\').next_back().unwrap();
                                    fibre.engine_state.func_map.contains_key(local_name) || registry.functions.contains_key(local_name)
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        } else {
                            false
                        };
                        Self::push(fibre, Value::new_bool(exists));
                        continue;
                    }

                    if func_name == "method_exists" {
                        let exists = if let (Some(args0), Some(args1)) = (args.first(), args.get(1))
                        {
                            if let Some(method_ptr) = args1.as_string_ptr() {
                                let method_str = unsafe { &*(method_ptr as *const String) };
                                if let Some(obj_ptr) = args0.as_object_ptr() {
                                    let obj = unsafe {
                                        &*(obj_ptr
                                            as *const hyperion_core::types::object::PhpObject)
                                    };
                                    let class_name = fibre.get_class_name(obj.class_id).unwrap_or_default();
                                    fibre.has_method(&class_name, method_str)
                                } else if let Some(class_ptr) = args0.as_string_ptr() {
                                    let class_str = unsafe { &*(class_ptr as *const String) };
                                    fibre.has_method(class_str, method_str)
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        } else {
                            false
                        };
                        Self::push(fibre, Value::new_bool(exists));
                        continue;
                    }


                    let registry = StdlibRegistry::global();
                    let mut resolved_native_fn = registry.functions.get(&fq_func_name);
                    if resolved_native_fn.is_none() && fq_func_name.contains('\\') {
                        resolved_native_fn = registry.functions.get(&func_name);
                    }

                    if let Some(native_fn) = resolved_native_fn {
                        // Natives read their arguments as plain values — none of
                        // the ~100 `.as_array_ptr()` call sites in ext/ know about
                        // ref cells. Collapse the cells here, at the one boundary
                        // they all cross. By-ref natives still mutate the caller's
                        // data because arrays alias by handle; an out-param that
                        // is still null gets a fresh array published *through* the
                        // cell first, so `preg_match($re, $s, $m)` on an undefined
                        // `$m` has something to fill.
                        //
                        // A scalar out-param (`preg_replace`'s `&$count`) has no
                        // handle to alias, so those few positions keep their cell
                        // and the native assigns through it.
                        for (i, arg) in args.iter_mut().enumerate() {
                            if hyperion_core::types::native_signatures::keeps_ref_cell(
                                &func_name, i,
                            ) {
                                continue;
                            }
                            let Some(cell) = arg.as_ref_ptr() else { continue };
                            let cell = cell as *mut hyperion_core::types::reference::PhpRef;
                            let inner = unsafe { (*cell).get() };
                            let resolved = if inner.is_null() {
                                let arr = hyperion_core::types::array::PhpArray::new();
                                let arr_ptr = fibre.arena.alloc_and_track(arr);
                                let v = Value::new_array_ptr(arr_ptr as *mut ());
                                unsafe { (*cell).set(v) };
                                v
                            } else {
                                inner
                            };
                            *arg = resolved;
                        }

                        match native_fn(&args, fibre) {
                            Ok(result) => {
                                if result.is_yield() {
                                    let yield_id = result.as_yield_id().unwrap_or(0);
                                    fibre.suspend();
                                    if yield_id == 0xFFFFFFFFFFFF {
                                        return ExecutionResult::Yielded(YieldReason::WaitForHttpRequest);
                                    } else {
                                        return ExecutionResult::Yielded(YieldReason::Database(yield_id));
                                    }
                                } else {
                                    Self::push(fibre, result);
                                }
                            }
                            Err(e) => {
                                if e == "__HYPERION_EXIT__" {
                                    fibre.frame_count = 0;
                                    return ExecutionResult::Finished;
                                }
                                if let Some(res) = Self::create_and_throw_native_error(fibre, &e) {
                                    return res;
                                }
                            }
                        }
                    } else {
                        return ExecutionResult::Error(format!(
                            "CallNative: Call to undefined function: {}",
                            fq_func_name
                        ));
                    }
                }
                Opcode::JumpIfFalse => {
                    let offset = Self::read_short(fibre) as usize;
                    let condition = Self::pop(fibre);

                    // Extra debug for arrays
                    if let Some(arr_ptr) = condition.as_array_ptr() {
                        let _arr =
                            unsafe { &*(arr_ptr as *const hyperion_core::types::array::PhpArray) };
                    }

                    let is_falsy = !is_truthy(&condition);

                    if let Some(tracer) = &mut fibre.tracer {
                        if tracer.is_recording {
                            tracer
                                .trace
                                .push(hyperion_jit::tracer::TraceIR::GuardCondition {
                                    src: fibre.stack_top,
                                    expected_bool: !is_falsy,
                                    bailout_ip: current_ip,
                                    bailout_stack_top: fibre.stack_top + 1, // condition was popped, restore it
                                });
                        }
                    }

                    debug_println!(
                        "[DEBUG JUMP] JumpIfFalse: condition={:?}, is_falsy={}, offset={}",
                        condition,
                        is_falsy,
                        offset
                    );
                    if is_falsy {
                        fibre.frames[fibre.frame_count - 1].ip += offset;
                    }
                }
                Opcode::JumpIfNotNull => {
                    let offset = Self::read_short(fibre) as usize;
                    let val = fibre.stack[fibre.stack_top - 1].deref();
                    if !val.is_null() {
                        fibre.frames[fibre.frame_count - 1].ip += offset;
                    }
                }
                Opcode::JumpIfNull => {
                    let offset = Self::read_short(fibre) as usize;
                    let val = fibre.stack[fibre.stack_top - 1].deref();
                    if val.is_null() {
                        fibre.frames[fibre.frame_count - 1].ip += offset;
                    }
                }
                Opcode::PackVariadic => {
                    let fixed_arity = Self::read_byte(fibre) as usize;
                    let frame = fibre.frames[fibre.frame_count - 1];
                    let stack_window = frame.stack_window;
                    let offset = 1; // 1 because stack_window + 0 is the receiver/func_val
                    let existing = fibre.stack[stack_window + offset + fixed_arity];
                    if existing.is_array() {
                        continue;
                    }
                    let current_args = frame.arity;
                    let func = unsafe { &*frame.function.0 };
                    let num_locals = func.num_locals;

                    let mut variadic_args = Vec::new();

                    if current_args > fixed_arity {
                        for i in fixed_arity..current_args {
                            variadic_args.push(fibre.stack[stack_window + offset + i]);
                        }
                    }

                    let arr = hyperion_core::types::array::PhpArray::new();
                    let arr_ref = fibre.arena.alloc_and_track(arr);
                    let mut next_idx = 0;
                    for arg in variadic_args {
                        unsafe { (*arr_ref).insert_int(next_idx, arg) };
                        next_idx += 1;
                    }

                    let arr_val = Value::new_array_ptr(arr_ref as *mut () as *mut _);
                    // Place the array at `stack_window + offset + fixed_arity`
                    unsafe {
                        *fibre
                            .stack
                            .get_unchecked_mut(stack_window + offset + fixed_arity) = arr_val;
                    }

                    // Clear any caller-supplied arguments that exceeded the fixed arity + variadic parameter
                    if current_args > fixed_arity + 1 {
                        for i in (fixed_arity + 1)..current_args {
                            fibre.stack[stack_window + offset + i] = Value::null();
                        }
                    }

                    // Adjust stack_top to preserve all reserved local variables
                    fibre.stack_top = stack_window + 1 + std::cmp::max(fixed_arity + 1, num_locals);
                }
                Opcode::Jump => {
                    let offset = Self::read_short(fibre) as usize;
                    fibre.frames[fibre.frame_count - 1].ip += offset;
                }
                Opcode::SwitchTable => {
                    let table_idx = Self::read_byte(fibre) as usize;
                    let default_offset = Self::read_short(fibre) as usize;
                    let condition = Self::pop(fibre);

                    let frame = fibre.frames[fibre.frame_count - 1];
                    let func = unsafe { &*frame.function.0 };
                    let chunk = &func.chunk;

                    if let Some(table) = chunk.jump_tables.get(table_idx) {
                        if let Some(&jump_offset) = table.get(&condition) {
                            fibre.frames[fibre.frame_count - 1].ip += jump_offset as usize;
                        } else {
                            fibre.frames[fibre.frame_count - 1].ip += default_offset;
                        }
                    } else {
                        // Should not happen if compiled correctly
                        fibre.frames[fibre.frame_count - 1].ip += default_offset;
                    }
                }
                Opcode::Loop => {
                    let offset = Self::read_short(fibre) as usize;
                    let ip = fibre.frames[fibre.frame_count - 1].ip;
                    if offset > ip {
                        hyperion_core::hyp_debug!("PANIC AVERTED: offset {} > ip {}", offset, ip);
                        let func_name = unsafe { &*fibre.frames[fibre.frame_count - 1].function.0 }.name.clone();
                        hyperion_core::hyp_debug!("Function: {}", func_name);
                        return ExecutionResult::Error(format!("Loop offset {} > ip {}", offset, ip));
                    }
                    let loop_header_ip = ip - offset;
                    let func_addr = fibre.frames[fibre.frame_count - 1].function.0 as usize;

                    // 1. Stop tracing if we are currently tracing and have reached the loop header again
                    let mut compile_trace = false;
                    if let Some(tracer) = &mut fibre.tracer {
                        if tracer.is_recording {
                            if tracer.start_ip == loop_header_ip {
                                tracer.trace.push(hyperion_jit::tracer::TraceIR::Loop {
                                    target_ip: loop_header_ip,
                                });
                                tracer.stop();
                                compile_trace = true;
                            } else {
                                // Nested loop encountered during tracing; abort outer trace to trace inner loops only.
                                fibre.tracer = None;
                            }
                        }
                    }

                    if compile_trace {
                        if let Some(tracer) = fibre.tracer.take() {
                            let optimizer = hyperion_jit::optimizer::Optimizer::new();
                            let optimized_trace = optimizer.optimize(tracer.trace);
                            if std::env::var("HYPERION_JIT_DEBUG").is_ok() {
                                eprintln!(
                                    "[JIT COMPILE] Trace for func {:x} at {}:\n{:#?}",
                                    func_addr,
                                    loop_header_ip,
                                    optimized_trace
                                );
                            }
                            let compiled = hyperion_jit::assembler::compile_trace(&optimized_trace);
                            fibre
                                .engine_state
                                .jit_cache
                                .insert((func_addr, loop_header_ip), std::sync::Arc::new(compiled));
                        }
                    }

                    // 2. Check if a compiled trace exists for this loop
                    // First check if it's blacklisted locally
                    let deopts = *fibre.jit_deopt_count.get(&(func_addr, loop_header_ip)).unwrap_or(&0);
                    if deopts < 10 {
                        if let Some(compiled_trace) = fibre
                            .engine_state
                            .jit_cache
                            .get(&(func_addr, loop_header_ip))
                        {
                            let mut ret = hyperion_jit::assembler::JitReturn {
                                tag: 0,
                                payload_ip: 0,
                                stack_top: 0,
                            };
                            let stack_ptr = fibre.stack.as_mut_ptr();
                            (compiled_trace.execute_fn)(stack_ptr, &mut ret);
                            if ret.tag == 1 {
                                if std::env::var("HYPERION_JIT_DEBUG").is_ok() {
                                    eprintln!("[DEBUG JIT] loop_ip={}, ret.payload_ip={}, ret.stack_top={}", loop_header_ip, ret.payload_ip, ret.stack_top);
                                }
                                if ret.payload_ip > 0 {
                                    fibre.frames[fibre.frame_count - 1].ip = ret.payload_ip as usize;
                                }
                                if ret.stack_top > 0 {
                                    fibre.stack_top = ret.stack_top as usize;
                                }
                                continue;
                            }

                        }
                    }

                    // 3. Otherwise, track heat and start tracing if hot (and not already compiled/blacklisted)
                    let already_compiled = fibre.engine_state.jit_cache.contains_key(&(func_addr, loop_header_ip));
                    if !already_compiled && deopts < 10 {
                        let heat = fibre
                            .local_heat_map
                            .entry((func_addr, loop_header_ip))
                            .or_insert(0);
                        *heat += 1;

                        let jit_threshold = get_jit_threshold();

                        if *heat > jit_threshold && fibre.tracer.is_none() {
                            let mut tracer = hyperion_jit::tracer::Tracer::new();
                            let window = fibre.frames[fibre.frame_count - 1].stack_window;
                            let rel_stack_top = fibre.stack_top.saturating_sub(window);
                            tracer.start(loop_header_ip, rel_stack_top);
                            fibre.tracer = Some(tracer);
                        }
                    }

                    fibre.frames[fibre.frame_count - 1].ip = loop_header_ip;

                    gas += 1;
                    if gas >= gas_limit {
                        return ExecutionResult::Preempted;
                    }
                }
                Opcode::Include | Opcode::IncludeOnce => {
                    let is_once = op == Opcode::IncludeOnce;
                    let path_val = Self::pop(fibre);
                    let path_str = match Self::cast_value_to_string(fibre, path_val) {
                        Ok(s) => std::path::Path::new(&s).canonicalize().map(|p| p.to_string_lossy().into_owned()).unwrap_or(s),
                        Err(e) => return ExecutionResult::Error(format!("include(): {}", e)),
                    };

                    if is_once && fibre.included_files.contains(&path_str) {
                        if let Some(cached_ret) = fibre.included_file_returns.get(&path_str) {
                            Self::push(fibre, *cached_ret);
                        } else {
                            // Already included, push 1 (true)
                            Self::push(fibre, Value::new_int(1));
                        }
                    } else {
                        // Check OPcache or compile and load immediately
                        let func_ptr = if let Some(entry) = fibre.engine_state.opcache.get(&path_str) {
                            entry.value().func_ptr
                        } else {
                            match fibre.engine_state.compile_and_load_script(&path_str) {
                                Ok(fptr) => fptr,
                                Err(e) => return ExecutionResult::Error(format!("include('{}'): {}", path_str, e)),
                            }
                        };

                        if is_once {
                            fibre.included_files.insert(path_str.clone());
                        }
                        fibre.frame_included_paths.insert(fibre.frame_count, path_str.clone());
                        if fibre.frame_count >= fibre.frames.len() {

                            return ExecutionResult::Error(
                                "Stack overflow: max call depth exceeded".to_string(),
                            );
                        }
                        let parent_frame = &fibre.frames[fibre.frame_count - 1];

                        let parent_chunk = &unsafe { &*parent_frame.function.0 }.chunk;
                        let parent_window = parent_frame.stack_window;
                        let parent_frame_idx = fibre.frame_count - 1;
                        let called_class_id = parent_frame.called_class_id;

                        let inc_func = unsafe { &*func_ptr.0 };
                        let inc_chunk = &inc_func.chunk;

                        let new_stack_window = fibre.stack_top;

                        for inc_local_name in &inc_chunk.local_names {
                            let val = if let Some(dyn_val) = fibre.dynamic_locals.get(&parent_frame_idx).and_then(|m| m.get(inc_local_name)) {
                                *dyn_val
                            } else if let Some(p_idx) = parent_chunk.local_names.iter().position(|n| n == inc_local_name) {
                                fibre.stack[parent_window + p_idx]
                            } else {
                                Value::null()
                            };
                            Self::push(fibre, val);
                        }

                        if let Some(parent_dyn) = fibre.dynamic_locals.get(&parent_frame_idx).cloned() {
                            fibre.dynamic_locals.insert(fibre.frame_count, parent_dyn);
                        }

                        fibre.frames[fibre.frame_count] = CallFrame {
                            function: func_ptr,
                            ip: 0,
                            stack_window: new_stack_window,
                            called_class_id,
                            return_override: None,
                            eval_parent_stack_window: Some(parent_window),
                            arity: 0,
                        };
                        fibre.frame_count += 1;
                    }
                }
                Opcode::Throw => {
                    let exception_val = Self::pop(fibre);
                    if let Some(res) = Self::handle_throw(fibre, exception_val) {
                        return res;
                    }
                }
                Opcode::Equals => {
                    if let Some(tracer) = &mut fibre.tracer {
                        if tracer.is_recording {
                            let a = fibre.stack[fibre.stack_top - 2];
                            let b = fibre.stack[fibre.stack_top - 1];
                            let a_tag = a.0 & 0xFFFF000000000000;
                            let b_tag = b.0 & 0xFFFF000000000000;

                            tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardType {
                                src: fibre.stack_top - 2,
                                expected_type_tag: a_tag,
                                bailout_ip: current_ip,
                                bailout_stack_top: fibre.stack_top,
                            });
                            tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardType {
                                src: fibre.stack_top - 1,
                                expected_type_tag: b_tag,
                                bailout_ip: current_ip,
                                bailout_stack_top: fibre.stack_top,
                            });
                            tracer.trace.push(hyperion_jit::tracer::TraceIR::Equals {
                                dest: fibre.stack_top - 2,
                                left: fibre.stack_top - 2,
                                right: fibre.stack_top - 1,
                            });
                        }
                    }
                    let b = Self::pop(fibre);
                    let a = Self::pop(fibre);
                    // PHP 8 loose equality semantics
                    let result = Self::php8_loose_equals(fibre, &a, &b);
                    Self::push(fibre, Value::new_bool(result));
                }
                Opcode::StrictEquals => {
                    let b = Self::pop(fibre);
                    let a = Self::pop(fibre);
                    let result = Self::php_strict_equals(fibre, &a, &b);
                    Self::push(fibre, Value::new_bool(result));
                }
                Opcode::InstanceOf => {
                    let right = Self::pop(fibre);
                    let left = Self::pop(fibre);

                    let mut result = false;
                    if let Some(class_name_ptr) = right.as_string_ptr() {
                        let class_name = unsafe { &*(class_name_ptr as *const String) };
                        result = Self::is_instance_of(fibre, left, class_name);
                    }
                    Self::push(fibre, Value::new_bool(result));
                }
                op @ Opcode::Call | op @ Opcode::CallUnpacked => {
                    let mut unpacked_named_pairs: Vec<(Value, Value)> = Vec::new();
                    let (mut arity, stack_window) = if op == Opcode::CallUnpacked {
                        let mut positional_args = Vec::new();
                        while fibre.stack_top > 0 {
                            let val = Self::pop(fibre);
                            if val.is_unpack_marker() {
                                break;
                            }
                            if val.is_named_arg_marker() {
                                let arg_val = Self::pop(fibre);
                                let name_val = Self::pop(fibre);
                                unpacked_named_pairs.push((name_val, arg_val));
                            } else {
                                positional_args.push(val);
                            }
                        }
                        let pos_forward: Vec<Value> = positional_args.iter().rev().cloned().collect();
                        let a = pos_forward.len();
                        let func_val = Self::pop(fibre);
                        let sw = fibre.stack_top;
                        Self::push(fibre, func_val);
                        for arg in &pos_forward {
                            Self::push(fibre, *arg);
                        }
                        (a, sw)
                    } else {
                        let a = Self::read_byte(fibre) as usize;
                        (a, fibre.stack_top - a - 1)
                    };
                    
                    let func_val = fibre.stack[stack_window].deref();
                    fibre.stack[stack_window] = func_val;
                    let _args_slice = &fibre.stack[stack_window + 1..fibre.stack_top];

                    if func_val.is_closure() {
                        let closure_ptr = func_val.as_closure_ptr().unwrap();
                        let closure =
                            unsafe { &*(closure_ptr as *const crate::types::closure::PhpClosure) };
                        let func = unsafe {
                            &*(closure.function_ptr.0 as *const crate::types::function::PhpFunction)
                        };
                        if func.name.starts_with("closure#") {
                            debug_println!("--- Dynamic Disassembly of {} ---", func.name);
                            debug_println!("{:?}", func.chunk.code);
                        }
                    }

                    let func_ptr = if let Some(closure_ptr) = func_val.as_closure_ptr() {
                        let closure =
                            unsafe { &*(closure_ptr as *const crate::types::closure::PhpClosure) };
                        closure.function_ptr
                    } else if let Some(s_ptr) = func_val.as_string_ptr() {
                        let func_name = unsafe { &*(s_ptr as *const String) };
                        let engine = &fibre.engine_state;
                        let normalized = normalize_name(func_name);
                        let funcs = &engine.functions;
                        let found = if let Some(id) = engine.func_map.get(&normalized).map(|v| *v) {
                            funcs.get(&id).map(|v| *v)
                        } else if normalized.contains('\\') {
                            let local_name = normalized.split('\\').next_back().unwrap();
                            engine.func_map.get(local_name).and_then(|id| funcs.get(&*id).map(|v| *v))
                        } else {
                            None
                        };
                        if let Some(f) = found {
                            f
                        } else {
                            // Check stdlib native functions
                            let registry = crate::stdlib::StdlibRegistry::global();
                            let mut resolved_native_fn = registry.functions.get(&normalized);
                            let mut resolved_key = normalized.clone();
                            let mut local_name = normalized.clone();
                            if resolved_native_fn.is_none() && normalized.contains('\\') {
                                local_name = normalized.split('\\').next_back().unwrap().to_string();
                                if let Some(native_fn) = registry.functions.get(&local_name) {
                                    resolved_native_fn = Some(native_fn);
                                    resolved_key = local_name.clone();
                                }
                            }
                            if resolved_native_fn.is_none() {
                                if let Some(native_fn) = registry.functions.get(func_name) {
                                    resolved_native_fn = Some(native_fn);
                                    resolved_key = func_name.clone();
                                }
                            }

                            if resolved_native_fn.is_some() {
                                Self::make_native_thunk(&resolved_key, arity)
                            } else {
                                return ExecutionResult::Error(format!(
                                    "Function {} not found",
                                    func_name
                                ));
                            }
                }
            } else if let Some(obj_ptr) = func_val.as_object_ptr() {
                let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
                let class_id = obj.class_id;
                let find_res = Self::find_method_fast(fibre, class_id, "__invoke");

                if let Ok(Some(f_ptr)) = find_res {
                    fibre.stack[stack_window] = func_val;
                    f_ptr
                } else {
                    let class_name = {
                        let classes = &fibre.engine_state.classes;
                        classes.get(&class_id).map(|c| c.name.clone()).unwrap_or_else(|| "Unknown".to_string())
                    };
                    return ExecutionResult::Error(format!("Call to undefined method {}::__invoke", class_name));
                }
            } else if let Some(arr_ptr) = func_val.as_array_ptr() {
                let arr = unsafe { &*(arr_ptr as *const hyperion_core::types::array::PhpArray) };
                let (class_val, method_val) = if let (Some(c), Some(m)) = (arr.get_int(0), arr.get_int(1)) {
                    (*c, *m)
                } else if arr.elements.len() >= 2 {
                    let mut iter = arr.elements.values();
                    (*iter.next().unwrap(), *iter.next().unwrap())
                } else {
                    (Value::null(), Value::null())
                };

                let class_val = class_val.deref();
                let method_val = method_val.deref();

                let method_name = if let Some(m_ptr) = method_val.as_string_ptr() {
                    unsafe { &*(m_ptr as *const String) }.clone()
                } else {
                    return ExecutionResult::Error(format!("Invalid method name in callable array: {:?}", method_val));
                };

                let is_static;
                let class_id = if let Some(c_ptr) = class_val.as_string_ptr() {
                    let raw_class_name = unsafe { &*(c_ptr as *const String) };
                    let class_name = Self::resolve_dynamic_class_name(fibre, &raw_class_name);
                    is_static = true;
                    match Self::find_class_id(fibre, &class_name) {
                        Some(id) => id,
                        None => {
                            fibre.trigger_autoload_sync(&class_name);
                            match Self::find_class_id(fibre, &class_name) {
                                Some(id) => id,
                                None => {
                                    return ExecutionResult::Error(format!("Class {} not found", class_name));
                                }
                            }
                        }
                    }
                } else if let Some(obj_ptr) = class_val.as_object_ptr() {
                    let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
                    is_static = false;
                    Self::resolve_obj_class_id(fibre, obj)
                } else {
                    return ExecutionResult::Error(format!("Invalid class/object in callable array: class_val={:?}", class_val));
                };

                let mut find_res = Self::find_method_fast(fibre, class_id, &method_name);
                let mut is_magic_call = false;
                if let Ok(None) = find_res {
                    if !is_static {
                        if let Ok(Some(magic_ptr)) = Self::find_method_fast(fibre, class_id, "__call") {
                            find_res = Ok(Some(magic_ptr));
                            is_magic_call = true;
                        }
                    } else {
                        if let Ok(Some(magic_ptr)) = Self::find_method_fast(fibre, class_id, "__callStatic") {
                            find_res = Ok(Some(magic_ptr));
                            is_magic_call = true;
                        }
                    }
                }


                if let Ok(Some(f_ptr)) = find_res {
                    if is_static {
                        fibre.stack[stack_window] = Value::new_int(0);
                    } else {
                        fibre.stack[stack_window] = class_val;
                    }
                    if is_magic_call {
                        let mut args_arr = hyperion_core::types::array::PhpArray::new();
                        let num_args = fibre.stack_top.saturating_sub(stack_window + 1);
                        for i in 0..num_args {
                            args_arr.insert_int(i as i64, fibre.stack[stack_window + 1 + i]);
                        }
                        for (name, val) in unpacked_named_pairs.iter().rev() {
                            if let Some(s) = name.as_string_ptr() {
                                let key_s = unsafe { (*(s as *const String)).clone() };
                                args_arr.insert_string_id(hyperion_core::types::string_table::intern_string(&key_s), *val);
                            }
                        }
                        let args_arr_ptr = fibre.arena.alloc_and_track(args_arr) as *mut ();
                        let args_val = Value::new_array_ptr(args_arr_ptr);

                        let m_name_ptr = fibre.arena.alloc_and_track(method_name.clone()) as *mut String;
                        let m_name_val = Value::new_string_ptr(m_name_ptr as *mut ());

                        fibre.stack[stack_window + 1] = m_name_val;
                        fibre.stack[stack_window + 2] = args_val;
                        fibre.stack_top = stack_window + 3;
                    }
                    f_ptr
                } else if let Some(enum_res) = Self::try_execute_enum_builtin(fibre, class_id, &method_name, &fibre.stack[stack_window + 1..fibre.stack_top].to_vec()) {
                    match enum_res {
                        Ok(res_val) => {
                            fibre.stack[stack_window] = res_val;
                            fibre.stack_top = stack_window + 1;
                            continue 'vm_loop;
                        }
                        Err(err) => return ExecutionResult::Error(err),
                    }
                } else {
                    return ExecutionResult::Error(format!("Method {} not found in callable array", method_name));
                }
            } else {
                let frame = fibre.frames[fibre.frame_count - 1];
                let current_fn = unsafe { &*frame.function.0 };
                let mut stack_frames = Vec::new();
                for (idx, f) in fibre.frames[0..fibre.frame_count].iter().enumerate() {
                    let fn_ref = unsafe { &*f.function.0 };
                    stack_frames.push(format!("#{} {} (ip={}, sw={})", idx, fn_ref.name, f.ip, f.stack_window));
                }
                eprintln!("DEBUG Call error: func_val={:?} at stack_window={} in fn={}, stack_top={}", func_val, stack_window, current_fn.name, fibre.stack_top);
                eprintln!("Current fn bytecode (ip={}): {:?}", frame.ip, current_fn.chunk.code);
                for (i, c) in current_fn.chunk.constants.iter().enumerate() {
                    if let Some(s_ptr) = c.as_string_ptr() {
                        let s = unsafe { &*(s_ptr as *const String) };
                        eprintln!("CONST[{}]: String(\"{}\")", i, s);
                    } else {
                        eprintln!("CONST[{}]: {:?}", i, c);
                    }
                }
                for i in frame.stack_window..fibre.stack_top {
                    eprintln!("STACK[{}]: {:?}", i, fibre.stack[i]);
                }
                eprintln!("Frames:\n{}", stack_frames.join("\n"));
                return ExecutionResult::Error(format!(
                    "Call to undefined function, val type: {:?} in {} (ip={})",
                    func_val.get_type(), current_fn.name, frame.ip
                ));
            };

                    let mut called_class_id = 0;
                    let upvalues = if let Some(closure_ptr) = func_val.as_closure_ptr() {
                        let closure =
                            unsafe { &*(closure_ptr as *const crate::types::closure::PhpClosure) };
                        called_class_id = closure.called_class_id;
                        if let Some(this_val) = closure.this_val {
                            fibre.stack[stack_window] = this_val;
                        }
                        closure.upvalues.as_slice()
                    } else if let Some(obj_ptr) = func_val.as_object_ptr() {
                        let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
                        called_class_id = obj.class_id;
                        &[]
                    } else {
                        if func_val.is_array() {
                            let arr = unsafe { &*(func_val.as_array_ptr().unwrap() as *const hyperion_core::types::array::PhpArray) };
                            let class_val = arr.get_int(0).map(|v| *v).unwrap_or(Value::null());
                            if let Some(c_ptr) = class_val.as_string_ptr() {
                                let raw_class_name = unsafe { &*(c_ptr as *const String) };
                                let class_name = Self::resolve_dynamic_class_name(fibre, &raw_class_name);
                                if let Some(id) = Self::find_class_id(fibre, &class_name) {
                                    called_class_id = id;
                                }
                            } else if let Some(obj_ptr) = class_val.as_object_ptr() {
                                let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
                                called_class_id = obj.class_id;
                            }
                        }
                        &[]
                    };

                    if fibre.frame_count >= fibre.frames.len() {
                        return ExecutionResult::Error("Stack overflow".to_string());
                    }

                    let target_func = unsafe { &*func_ptr.0 };
                    if let Some(tracer) = &mut fibre.tracer {
                        if tracer.is_recording {
                            if tracer.inline_depth < 2 && Self::is_inlinable_leaf_function(target_func) {
                                if func_val.is_closure() {
                                    tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardClosure {
                                        src: stack_window,
                                        expected_closure_fn_ptr: func_ptr.0 as u64,
                                        bailout_ip: current_ip,
                                        bailout_stack_top: fibre.stack_top,
                                    });
                                } else {
                                    tracer.trace.push(hyperion_jit::tracer::TraceIR::GuardFunction {
                                        src: stack_window,
                                        expected_fn_ptr: func_val.0,
                                        bailout_ip: current_ip,
                                        bailout_stack_top: fibre.stack_top,
                                    });
                                }
                                tracer.inline_depth += 1;
                            } else {
                                tracer.stop();
                            }
                        }
                    }

                    let mut var_packed = false;
                    if !unpacked_named_pairs.is_empty() {
                        let pos_args = fibre.stack[stack_window + 1..fibre.stack_top].to_vec();
                        let named_forward: Vec<(Value, Value)> = unpacked_named_pairs.iter().rev().cloned().collect();
                        let (bound, packed) = Self::bind_call_arguments(&fibre.engine_state, target_func, &pos_args, &named_forward, &fibre.arena);
                        fibre.stack_top = stack_window + 1;
                        for arg in &bound {
                            Self::push(fibre, *arg);
                        }
                        arity = bound.len();
                        var_packed = packed;
                    }

                    let passed_args = fibre.stack[stack_window + 1..fibre.stack_top].to_vec();
                    fibre.frames[fibre.frame_count] = CallFrame {
                        function: func_ptr,
                        ip: 0,
                        stack_window,
                        called_class_id,
                        return_override: None, eval_parent_stack_window: None,
                        arity,
                    };
                    fibre.set_frame_args(fibre.frame_count, &passed_args);
                    fibre.frame_count += 1;
                    Self::pad_missing_arguments_and_locals(fibre, arity, unsafe {
                        &*func_ptr.0
                    }, upvalues, var_packed);
                }
                Opcode::CallNamed => {
                    if let Some(tracer) = &mut fibre.tracer {
                        if tracer.is_recording {
                            tracer.stop();
                        }
                    }
                    let arity = Self::read_byte(fibre) as usize;
                    let num_named = Self::read_byte(fibre) as usize;
                    let mut named_pairs: Vec<(Value, Value)> = Vec::new();
                    for _ in 0..num_named {
                        let val = Self::pop(fibre);
                        let name = Self::pop(fibre);
                        named_pairs.push((name, val));
                    }
                    let mut positional_args = Vec::new();
                    for _ in 0..(arity - num_named) {
                        positional_args.push(Self::pop(fibre));
                    }
                    let func_val = Self::pop(fibre);

                    let func_ptr = if let Some(closure_ptr) = func_val.as_closure_ptr() {
                        let closure =
                            unsafe { &*(closure_ptr as *const crate::types::closure::PhpClosure) };
                        closure.function_ptr
                    } else if let Some(obj_ptr) = func_val.as_object_ptr() {
                        let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
                        let class_id = obj.class_id;
                        let find_res = Self::find_method_fast(fibre, class_id, "__invoke");

                        if let Ok(Some(f_ptr)) = find_res {
                            f_ptr
                        } else {
                            let class_name = {
                                let classes = &fibre.engine_state.classes;
                                classes.get(&class_id).map(|c| c.name.clone()).unwrap_or_else(|| "Unknown".to_string())
                            };
                            return ExecutionResult::Error(format!("Call to undefined method {}::__invoke", class_name));
                        }
                    } else if let Some(s_ptr) = func_val.as_string_ptr() {
                        let func_name = unsafe { &*(s_ptr as *const String) };
                        let engine = &fibre.engine_state;
                        let normalized = normalize_name(func_name);
                        let funcs = &engine.functions;
                        let found = if let Some(id) = engine.func_map.get(&normalized).map(|v| *v) {
                            funcs.get(&id).map(|v| *v)
                        } else if normalized.contains('\\') {
                            let local_name = normalized.split('\\').next_back().unwrap();
                            engine.func_map.get(local_name).and_then(|id| funcs.get(&*id).map(|v| *v))
                        } else {
                            None
                        };
                        if let Some(f) = found {
                            f
                        } else {
                            // Check stdlib native functions
                            let registry = crate::stdlib::StdlibRegistry::global();
                            let mut resolved_native_fn = registry.functions.get(&normalized);
                            let mut resolved_key = normalized.clone();
                            let mut local_name = normalized.clone();
                            if resolved_native_fn.is_none() && normalized.contains('\\') {
                                local_name = normalized.split('\\').next_back().unwrap().to_string();
                                if let Some(native_fn) = registry.functions.get(&local_name) {
                                    resolved_native_fn = Some(native_fn);
                                    resolved_key = local_name.clone();
                                }
                            }
                            if resolved_native_fn.is_none() {
                                if let Some(native_fn) = registry.functions.get(func_name) {
                                    resolved_native_fn = Some(native_fn);
                                    resolved_key = func_name.clone();
                                }
                            }

                            if resolved_native_fn.is_some() {
                                Self::make_native_thunk(&resolved_key, arity)
                            } else {
                                let keys: Vec<String> = registry.functions.keys().cloned().collect();
                                return ExecutionResult::Error(format!(
                                    "Function {} not found (normalized: '{}', local_name: '{}'). Registry has {} keys. Contains filter_var: {}",
                                    func_name, normalized, local_name, keys.len(), registry.functions.contains_key("filter_var")
                                ));
                            }
                        }
                    } else {
                        return ExecutionResult::Error(format!(
                            "Call to undefined function, val type: {:?}",
                            func_val.get_type()
                        ));
                    };

                    let func = unsafe { &*func_ptr.0 }; debug_println!("autoload: pushed function {}", func.name);
                    let pos_forward: Vec<Value> = positional_args.iter().rev().cloned().collect();
                    let named_forward: Vec<(Value, Value)> = named_pairs.iter().rev().cloned().collect();
                    let (args, var_packed) = Self::bind_call_arguments(&fibre.engine_state, func, &pos_forward, &named_forward, &fibre.arena);
                    let actual_arity = args.len();
                    
                    let mut upvalues_vec = Vec::new();
                    let (called_class_id, stack_this) = if let Some(closure_ptr) = func_val.as_closure_ptr() {
                        let closure = unsafe { &*(closure_ptr as *const crate::types::closure::PhpClosure) };
                        upvalues_vec = closure.upvalues.clone();
                        (closure.called_class_id, closure.this_val.unwrap_or(func_val))
                    } else {
                        (0, func_val)
                    };

                    let stack_window = fibre.stack_top;
                    Self::push(fibre, stack_this);
                    for arg in &args {
                        Self::push(fibre, *arg);
                    }

                    let passed_args = pos_forward.clone();
                    fibre.frames[fibre.frame_count] = CallFrame {
                        function: func_ptr,
                        ip: 0,
                        stack_window,
                        called_class_id,
                        return_override: None, eval_parent_stack_window: None,
                        arity: actual_arity,
                    };
                    fibre.set_frame_args(fibre.frame_count, &passed_args);
                    fibre.frame_count += 1;
                    Self::pad_missing_arguments_and_locals(fibre, actual_arity, unsafe {
                        &*func_ptr.0
                    }, &upvalues_vec, var_packed);
                }

                op @ Opcode::MethodCall | op @ Opcode::MethodCallUnpacked => {
                    if fibre.tracer.is_some() {
                        fibre.tracer = None;
                    }

                    let (method_name, _arity, _num_named, positional_args, named_pairs, unpacked_name_val) = if op == Opcode::MethodCallUnpacked {
                        let name_val = Self::pop(fibre).deref();
                        let method_name = name_val.as_string_ptr().map(|p| unsafe { (*(p as *mut String)).clone() }).unwrap_or_default();
                        let mut positional_args = Vec::new();
                        let mut named_pairs = Vec::new();
                        while fibre.stack_top > 0 {
                            let val = Self::pop(fibre);
                            if val.is_unpack_marker() {
                                break;
                            }
                            if val.is_named_arg_marker() {
                                let arg_val = Self::pop(fibre);
                                let name_val = Self::pop(fibre);
                                named_pairs.push((name_val, arg_val));
                            } else {
                                positional_args.push(val);
                            }
                        }
                        let arity = positional_args.len() + named_pairs.len();
                        let num_named = named_pairs.len();
                        (method_name, arity, num_named, positional_args, named_pairs, Some(name_val))
                    } else {
                        let name_idx = Self::read_short(fibre) as usize;
                        let arity = Self::read_byte(fibre) as usize;
                        let num_named = Self::read_byte(fibre) as usize;

                        if num_named == 0 && fibre.stack_top >= arity + 1 {
                            let obj_idx = fibre.stack_top - arity - 1;
                            let obj_val = fibre.stack[obj_idx].deref();
                            if let Some(obj_ptr) = obj_val.as_object_ptr() {
                                let obj = unsafe { &*(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
                                let class_id = Self::resolve_obj_class_id(fibre, obj);
                                let frame = fibre.frames[fibre.frame_count - 1];
                                let func_consts = &unsafe { &*frame.function.0 }.chunk.constants;
                                let name_val = func_consts[name_idx];
                                if let Some(s_ptr) = name_val.as_string_ptr() {
                                    let method_name = unsafe { &*(s_ptr as *const String) };
                                    if let Ok(Some(func_ptr)) = Self::find_method_fast(fibre, class_id, method_name) {


                                        let target_fn = unsafe { &*func_ptr.0 };
                                        if !target_fn.name.eq_ignore_ascii_case("__call") && Self::is_method_accessible(fibre, class_id, target_fn, fibre.frame_count - 1) {
                                            let stack_window = obj_idx;
                                            if fibre.frame_count >= fibre.frames.len() {
                                                return ExecutionResult::Error("Stack overflow".to_string());
                                            }
                                            fibre.frames[fibre.frame_count] = CallFrame {
                                                function: func_ptr,
                                                ip: 0,
                                                stack_window,
                                                called_class_id: class_id,
                                                return_override: None,
                                                eval_parent_stack_window: None,
                                                arity,
                                            };
                                            fibre.set_frame_args_from_stack(fibre.frame_count, obj_idx + 1, obj_idx + 1 + arity);
                                            fibre.frame_count += 1;
                                            Self::pad_missing_arguments_and_locals(fibre, arity, target_fn, &[], false);
                                            continue 'vm_loop;


                                        }
                                    }
                                }
                            }
                        }

                        let mut named_pairs: Vec<(Value, Value)> = Vec::new();
                        let mut positional_args = Vec::new();
                        for _ in 0..num_named {
                            let val = Self::pop(fibre);
                            let name = Self::pop(fibre);
                            named_pairs.push((name, val));
                        }
                        for _ in 0..(arity - num_named) {
                            positional_args.push(Self::pop(fibre));
                        }
                        
                        let frame = fibre.frames[fibre.frame_count - 1];
                        let func = unsafe { &*frame.function.0 };
                        let name_val = func.chunk.constants[name_idx];
                        let name_ptr = name_val.as_string_ptr().unwrap() as *mut String;
                        let method_name = unsafe { (*name_ptr).clone() };
                        
                        (method_name, arity, num_named, positional_args, named_pairs, None)
                    };

                    let obj_val = Self::pop(fibre).deref();
                    let pos_forward: Vec<Value> = positional_args.iter().rev().cloned().collect();
                    let named_forward: Vec<(Value, Value)> = named_pairs.iter().rev().cloned().collect();

                    if method_name == "causedByConcurrencyError" || method_name == "handleTransactionException" {
                        eprintln!("[DEBUG MethodCall dispatch] method: {}, obj: {:?}, args: {:?}", method_name, obj_val, pos_forward);
                    }

                    if obj_val.is_closure() {
                        let closure_ptr = obj_val.as_closure_ptr().unwrap();
                        let closure = unsafe { &*(closure_ptr as *const crate::types::closure::PhpClosure) };
                        if method_name.eq_ignore_ascii_case("bindTo") {
                            let new_this = pos_forward.first().map(|v| *v).unwrap_or(Value::null());
                            let new_scope = pos_forward.get(1).map(|v| *v);
                            let mut called_class_id = closure.called_class_id;
                            if let Some(scope_val) = new_scope {
                                if let Some(s_ptr) = scope_val.as_string_ptr() {
                                    let s = unsafe { &*(s_ptr as *const String) };
                                    if !s.eq_ignore_ascii_case("static") {
                                        if let Some(id) = Self::find_class_id(fibre, s) {
                                            called_class_id = id;
                                        }
                                    }
                                } else if let Some(obj_ptr) = scope_val.as_object_ptr() {
                                    let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
                                    called_class_id = Self::resolve_obj_class_id(fibre, obj);
                                } else if scope_val.is_null() {
                                    called_class_id = 0;
                                }
                            } else if new_this.is_object() {
                                let obj = unsafe { &*(new_this.as_object_ptr().unwrap() as *const hyperion_core::types::object::PhpObject) };
                                called_class_id = Self::resolve_obj_class_id(fibre, obj);
                            }
                            let this_val = if new_this.is_object() { Some(new_this) } else { None };
                            let mut upvalues = closure.upvalues.clone();
                            let func = unsafe { &*closure.function_ptr.0 };
                            let num_params = func.params.len();
                            for (i, name) in func.chunk.local_names.iter().enumerate() {
                                if name == "this" {
                                    if i >= 1 + num_params {
                                        let upvalue_idx = i - (1 + num_params);
                                        if upvalue_idx < upvalues.len() {
                                            upvalues[upvalue_idx] = new_this;
                                        }
                                    } else if i >= num_params {
                                        let upvalue_idx = i - num_params;
                                        if upvalue_idx < upvalues.len() {
                                            upvalues[upvalue_idx] = new_this;
                                        }
                                    }
                                }
                            }
                            let new_closure = PhpClosure::new(closure.function_ptr, upvalues, called_class_id, this_val);
                            let new_ptr = fibre.arena.alloc_and_track(new_closure) as *mut ();
                            Self::push(fibre, Value::new_closure_ptr(new_ptr));
                            continue 'vm_loop;
                        } else if method_name.eq_ignore_ascii_case("call") {
                            let new_this = pos_forward.first().map(|v| *v).unwrap_or(Value::null());
                            let call_args: Vec<Value> = pos_forward.iter().skip(1).map(|v| *v).collect();
                            let mut called_class_id = closure.called_class_id;
                            if new_this.is_object() {
                                let obj = unsafe { &*(new_this.as_object_ptr().unwrap() as *const hyperion_core::types::object::PhpObject) };
                                called_class_id = Self::resolve_obj_class_id(fibre, obj);
                            }
                            let func_ptr = closure.function_ptr;
                            let func = unsafe { &*func_ptr.0 };
                            let stack_window = fibre.stack_top;
                            if new_this.is_object() {
                                Self::push(fibre, new_this);
                            } else {
                                Self::push(fibre, Value::new_int(0));
                            }
                            let (args, var_packed) = Self::bind_call_arguments(&fibre.engine_state, func, &call_args, &[], &fibre.arena);
                            let actual_arity = args.len();
                            for arg in &args {
                                Self::push(fibre, *arg);
                            }
                            if fibre.frame_count >= fibre.frames.len() {
                                return ExecutionResult::Error("Stack overflow".to_string());
                            }
                            fibre.frames[fibre.frame_count] = CallFrame {
                                function: func_ptr,
                                ip: 0,
                                stack_window,
                                called_class_id,
                                return_override: None,
                                eval_parent_stack_window: None,
                                arity: actual_arity,
                            };
                            fibre.frame_count += 1;
                            let mut upvalues = closure.upvalues.clone();
                            let num_params = func.params.len();
                            for (i, name) in func.chunk.local_names.iter().enumerate() {
                                if name == "this" {
                                    if i >= 1 + num_params {
                                        let upvalue_idx = i - (1 + num_params);
                                        if upvalue_idx < upvalues.len() {
                                            upvalues[upvalue_idx] = new_this;
                                        }
                                    }
                                }
                            }
                            Self::pad_missing_arguments_and_locals(fibre, actual_arity, func, &upvalues, var_packed);
                            continue 'vm_loop;
                        } else if method_name.eq_ignore_ascii_case("__invoke") {
                            let func_ptr = closure.function_ptr;
                            let func = unsafe { &*func_ptr.0 };
                            let stack_window = fibre.stack_top;
                            if let Some(this_val) = closure.this_val {
                                Self::push(fibre, this_val);
                            } else {
                                Self::push(fibre, Value::new_int(0));
                            }
                            let (args, var_packed) = Self::bind_call_arguments(&fibre.engine_state, func, &pos_forward, &named_forward, &fibre.arena);
                            let actual_arity = args.len();
                            for arg in &args {
                                Self::push(fibre, *arg);
                            }
                            if fibre.frame_count >= fibre.frames.len() {
                                return ExecutionResult::Error("Stack overflow".to_string());
                            }
                            fibre.frames[fibre.frame_count] = CallFrame {
                                function: func_ptr,
                                ip: 0,
                                stack_window,
                                called_class_id: closure.called_class_id,
                                return_override: None,
                                eval_parent_stack_window: None,
                                arity: actual_arity,
                            };
                            fibre.frame_count += 1;
                            Self::pad_missing_arguments_and_locals(fibre, actual_arity, func, &closure.upvalues, var_packed);
                            continue 'vm_loop;
                        } else {
                            return ExecutionResult::Error(format!("Call to undefined method Closure::{}", method_name));
                        }
                    }

                    let obj_ptr = match obj_val.as_object_ptr() {
                        Some(ptr) => ptr as *mut hyperion_core::types::object::PhpObject,
                        None => {
                            let caller_func = if fibre.frame_count > 0 {
                                unsafe { (&*fibre.frames[fibre.frame_count - 1].function.0).name.clone() }
                            } else {
                                "<unknown>".to_string()
                            };
                            let err_msg = format!(
                                "Error: Call to a member function {}() on {:?}",
                                method_name,
                                obj_val.get_type()
                            );

                            if let Some(res) = Self::create_and_throw_native_error(fibre, &err_msg) {
                                return res;
                            } else {
                                continue;
                            }
                        }
                    };
                    let obj = unsafe { &*obj_ptr };
                    let class_id = Self::resolve_obj_class_id(fibre, obj);

                    let current_frame_idx = fibre.frame_count - 1;
                    let restore_stack = |fibre: &mut Fibre| {
                        Self::push(fibre, obj_val);
                        // The unpacked form consumed a marker and a trailing method
                        // name; both must come back or the replay's pop-until-marker
                        // loop runs off into the caller's live stack.
                        if unpacked_name_val.is_some() {
                            Self::push(fibre, Value::new_unpack_marker());
                            for arg in positional_args.iter().rev() {
                                Self::push(fibre, *arg);
                            }
                            for (name, val) in named_pairs.iter().rev() {
                                Self::push(fibre, *name);
                                Self::push(fibre, *val);
                                Self::push(fibre, Value::new_named_arg_marker());
                            }
                        } else {
                            for arg in positional_args.iter().rev() {
                                Self::push(fibre, *arg);
                            }
                            for (name, val) in named_pairs.iter().rev() {
                                Self::push(fibre, *name);
                                Self::push(fibre, *val);
                            }
                        }
                        if let Some(name_val) = unpacked_name_val {
                            Self::push(fibre, name_val);
                        }
                        fibre.frames[current_frame_idx].ip = start_ip;
                    };

                    let mut find_res =
                        Self::find_method_fast(fibre, class_id, &method_name);
                    if let Ok(Some(ptr)) = find_res {
                        let func = unsafe { &*ptr.0 };
                        if !Self::is_method_accessible(fibre, class_id, func, current_frame_idx) {
                            find_res = Self::find_method_fast(fibre, class_id, "__call");
                        }
                    } else if let Ok(None) = find_res {
                        find_res = Self::find_method_fast(fibre, class_id, "__call");
                    }


                    let func_ptr_opt = match find_res {
                        Ok(Some(ptr)) => Some(ptr),
                        Ok(None) => None,
                        Err(parent_name) => {
                            restore_stack(fibre);
                            match Self::trigger_autoload(fibre, &parent_name) {
                                Ok(true) => continue 'vm_loop,
                                Ok(false) => {
                                    if Self::find_class_id(fibre, &parent_name).is_some() {
                                        continue 'vm_loop;
                                    }
                                    return ExecutionResult::Error(format!(
                                        "Class '{}' not found",
                                        parent_name
                                    ))
                                }
                                Err(e) => return ExecutionResult::Error(e),
                            }
                        }
                    };

                    let func_ptr = match func_ptr_opt {
                        Some(ptr) => ptr,
                        None => {
                            let class_name = if let Some(ref name) = obj.class_name {
                                name.clone()
                            } else {
                                let classes = &fibre.engine_state.classes;
                                classes.get(&class_id).map(|c| c.name.clone()).unwrap_or_else(|| "Unknown".to_string())
                            };
                            return ExecutionResult::Error(format!(
                                "Method {}::{} not found",
                                class_name, method_name
                            ));
                        }
                    };

                    let func = unsafe { &*func_ptr.0 }; debug_println!("autoload: pushed function {}", func.name);
                    let stack_window = fibre.stack_top;
                    Self::push(fibre, obj_val);

                    let mut var_packed = false;
                    let actual_arity;
                    let func_ptr_name = unsafe { &*func_ptr.0 }.name.to_lowercase();
                    if func_ptr_name == "__call" && method_name.to_lowercase() != "__call" {
                        let mut arr = hyperion_core::types::array::PhpArray::new();
                        for (i, arg) in pos_forward.iter().enumerate() {
                            arr.insert_int(i as i64, *arg);
                        }
                        for (name, val) in &named_forward {
                            if let Some(s) = name.as_string_ptr() {
                                let key_s = unsafe { (*(s as *const String)).clone() };
                                arr.insert_string_id(hyperion_core::types::string_table::intern_string(&key_s), *val);
                            }
                        }
                        let arr_ptr = fibre.arena.alloc_and_track(arr) as *mut ();
                        let arr_val = Value::new_array_ptr(arr_ptr);

                        let m_name_ptr =
                            fibre.arena.alloc_and_track(method_name.to_string()) as *mut String;
                        let m_name_val = Value::new_string_ptr(m_name_ptr as *mut ());

                        Self::push(fibre, m_name_val);
                        Self::push(fibre, arr_val);
                        actual_arity = 2;
                    } else {
                        let (args, vp) = Self::bind_call_arguments(&fibre.engine_state, func, &pos_forward, &named_forward, &fibre.arena);
                        var_packed = vp;
                        actual_arity = args.len();
                        for arg in &args {
                            Self::push(fibre, *arg);
                        }
                    }

                    if fibre.frame_count >= fibre.frames.len() {
                        return ExecutionResult::Error("Stack overflow".to_string());
                    }

                    let passed_args = pos_forward.clone();
                    fibre.frames[fibre.frame_count] = CallFrame {
                        function: func_ptr,
                        ip: 0,
                        stack_window,
                        called_class_id: class_id,
                        return_override: None, eval_parent_stack_window: None,
                        arity: actual_arity,
                    };
                    fibre.set_frame_args(fibre.frame_count, &passed_args);
                    fibre.frame_count += 1;
                    Self::pad_missing_arguments_and_locals(fibre, actual_arity, unsafe {
                        &*func_ptr.0
                    }, &[], var_packed);
                }
                op @ Opcode::StaticMethodCall | op @ Opcode::StaticMethodCallUnpacked => {
                    let class_idx = Self::read_short(fibre) as usize;
                    let method_idx = Self::read_short(fibre) as usize;

                    let (_arity, _num_named, positional_args, named_pairs, was_unpacked) = if op == Opcode::StaticMethodCallUnpacked {
                        let mut positional_args = Vec::new();
                        let mut named_pairs = Vec::new();
                        while fibre.stack_top > 0 {
                            let val = Self::pop(fibre);
                            if val.is_unpack_marker() {
                                break;
                            }
                            if val.is_named_arg_marker() {
                                let arg_val = Self::pop(fibre);
                                let name_val = Self::pop(fibre);
                                named_pairs.push((name_val, arg_val));
                            } else {
                                positional_args.push(val);
                            }
                        }
                        (positional_args.len() + named_pairs.len(), named_pairs.len(), positional_args, named_pairs, true)
                    } else {
                        let arity = Self::read_byte(fibre) as usize;
                        let num_named = Self::read_byte(fibre) as usize;

                        if num_named == 0 && fibre.stack_top >= arity {
                            let frame = fibre.frames[fibre.frame_count - 1];
                            let func_consts = &unsafe { &*frame.function.0 }.chunk.constants;
                            let class_val = func_consts[class_idx];
                            let method_val = func_consts[method_idx];
                            if let (Some(c_ptr), Some(m_ptr)) = (class_val.as_string_ptr(), method_val.as_string_ptr()) {
                                let raw_class_name = unsafe { &*(c_ptr as *const String) };
                                let method_name = unsafe { &*(m_ptr as *const String) };
                                let class_name = Self::resolve_dynamic_class_name(fibre, raw_class_name);
                                let clean_class = class_name.trim_start_matches('\\');
                                if clean_class.eq_ignore_ascii_case("Closure") && method_name.eq_ignore_ascii_case("bind") {
                                    let stack_base = fibre.stack_top - arity;
                                    let target_closure_val = fibre.stack[stack_base];
                                    if let Some(closure_ptr) = target_closure_val.as_closure_ptr() {
                                        let closure = unsafe { &*(closure_ptr as *const crate::types::closure::PhpClosure) };
                                        let new_this = if arity >= 2 { fibre.stack[stack_base + 1] } else { Value::null() };
                                        let new_scope = if arity >= 3 { Some(fibre.stack[stack_base + 2]) } else { None };
                                        let mut called_class_id = closure.called_class_id;
                                        if let Some(scope_val) = new_scope {
                                            if let Some(s_ptr) = scope_val.as_string_ptr() {
                                                let s = unsafe { &*(s_ptr as *const String) };
                                                if !s.eq_ignore_ascii_case("static") {
                                                    if let Some(id) = Self::find_class_id(fibre, s.trim_start_matches('\\')) {
                                                        called_class_id = id;
                                                    }
                                                }
                                            } else if let Some(obj_ptr) = scope_val.as_object_ptr() {
                                                let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
                                                called_class_id = Self::resolve_obj_class_id(fibre, obj);
                                            } else if scope_val.is_null() {
                                                called_class_id = 0;
                                            }
                                        } else if new_this.is_object() {
                                            let obj = unsafe { &*(new_this.as_object_ptr().unwrap() as *const hyperion_core::types::object::PhpObject) };
                                            called_class_id = Self::resolve_obj_class_id(fibre, obj);
                                        }
                                        let this_val = if new_this.is_object() { Some(new_this) } else { None };
                                        let mut upvalues = closure.upvalues.clone();
                                        let func = unsafe { &*closure.function_ptr.0 };
                                        let num_params = func.params.len();
                                        for (i, name) in func.chunk.local_names.iter().enumerate() {
                                            if name == "this" {
                                                if i >= 1 + num_params {
                                                    let upvalue_idx = i - (1 + num_params);
                                                    if upvalue_idx < upvalues.len() {
                                                        upvalues[upvalue_idx] = new_this;
                                                    }
                                                } else if i >= num_params {
                                                    let upvalue_idx = i - num_params;
                                                    if upvalue_idx < upvalues.len() {
                                                        upvalues[upvalue_idx] = new_this;
                                                    }
                                                }
                                            }
                                        }
                                        let new_closure = PhpClosure::new(closure.function_ptr, upvalues, called_class_id, this_val);
                                        let new_ptr = fibre.arena.alloc_and_track(new_closure) as *mut ();
                                        fibre.stack_top -= arity;
                                        Self::push(fibre, Value::new_closure_ptr(new_ptr));
                                        continue 'vm_loop;
                                    }
                                }
                                if clean_class.eq_ignore_ascii_case("Closure") && method_name.eq_ignore_ascii_case("fromCallable") {
                                    let stack_base = fibre.stack_top - arity;
                                    if arity >= 1 {
                                        let target = fibre.stack[stack_base];
                                        use hyperion_core::types::function::NativeContext;
                                        match fibre.closure_from_callable(target) {
                                            Ok(c) => {
                                                fibre.stack_top = stack_base;
                                                Self::push(fibre, c);
                                                continue 'vm_loop;
                                            }
                                            Err(e) => return ExecutionResult::Error(e),
                                        }
                                    }
                                }
                                if let Some(class_id) = Self::find_class_id(fibre, &class_name) {
                                    if let Ok(Some(func_ptr)) = Self::find_method_fast(fibre, class_id, method_name) {


                                        let target_fn = unsafe { &*func_ptr.0 };
                                        if !target_fn.name.eq_ignore_ascii_case("__callstatic") && Self::is_method_accessible(fibre, class_id, target_fn, fibre.frame_count - 1) {
                                            let stack_window = fibre.stack_top - arity;
                                            if fibre.stack.len() <= fibre.stack_top + 1 {
                                                fibre.stack.resize(fibre.stack.len() * 2, Value::null());
                                            }
                                            fibre.stack.copy_within(stack_window..fibre.stack_top, stack_window + 1);
                                            let mut current_this = fibre.stack[frame.stack_window];
                                            if let Some(closure_ptr) = current_this.as_closure_ptr() {
                                                let closure = unsafe { &*(closure_ptr as *const crate::types::closure::PhpClosure) };
                                                current_this = closure.this_val.unwrap_or(Value::null());
                                            } else if !current_this.is_object() {
                                                current_this = Value::null();
                                            }
                                            fibre.stack[stack_window] = current_this;
                                            fibre.stack_top += 1;
                                            if fibre.frame_count >= fibre.frames.len() {
                                                return ExecutionResult::Error("Stack overflow".to_string());
                                            }
                                            let effective_called_class_id = if raw_class_name.starts_with("self\0") || raw_class_name.starts_with("parent\0") || raw_class_name.eq_ignore_ascii_case("self") || raw_class_name.eq_ignore_ascii_case("parent") {
                                                let caller_called = Self::get_current_called_class_id(fibre);
                                                if caller_called != 0 { caller_called } else { class_id }
                                            } else {
                                                class_id
                                            };
                                            fibre.frames[fibre.frame_count] = CallFrame {
                                                function: func_ptr,
                                                ip: 0,
                                                stack_window,
                                                called_class_id: effective_called_class_id,
                                                return_override: None,
                                                eval_parent_stack_window: None,
                                                arity,
                                            };

                                            fibre.set_frame_args_from_stack(fibre.frame_count, stack_window + 1, stack_window + 1 + arity);
                                            fibre.frame_count += 1;
                                            Self::pad_missing_arguments_and_locals(fibre, arity, target_fn, &[], false);
                                            continue 'vm_loop;


                                        }
                                    }
                                }
                            }
                        }

                        let mut named_pairs: Vec<(Value, Value)> = Vec::new(); // (name, value)
                        let mut positional_args = Vec::new();
                        for _ in 0..num_named {
                            let val = Self::pop(fibre);
                            let name = Self::pop(fibre);
                            named_pairs.push((name, val));
                        }
                        for _ in 0..(arity - num_named) {
                            positional_args.push(Self::pop(fibre));
                        }
                        (arity, num_named, positional_args, named_pairs, false)
                    };

                    let frame = fibre.frames[fibre.frame_count - 1];
                    let func = unsafe { &*frame.function.0 };

                    let method_val = func.chunk.constants[method_idx];
                    let method_ptr = method_val.as_string_ptr().unwrap() as *mut String;
                    let method_name = unsafe { (*method_ptr).clone() };

                    let class_val = func.chunk.constants[class_idx];
                    let class_ptr = class_val.as_string_ptr().unwrap() as *mut String;
                    let raw_class_name = unsafe { (*class_ptr).clone() };
                    let class_name = Self::resolve_dynamic_class_name(fibre, &raw_class_name);

                    let pos_forward: Vec<Value> = positional_args.iter().rev().cloned().collect();
                    let named_forward: Vec<(Value, Value)> = named_pairs.iter().rev().cloned().collect();

                    let class_id_opt = Self::find_class_id(fibre, &class_name);

                    let class_id = match class_id_opt {
                        Some(id) => id,
                        None => {
                            // Restore stack in original order for replay after autoload
                            // The unpacked form consumed a marker; replay needs it back
                            // or the pop-until-marker loop overruns the caller's stack.
                            if was_unpacked {
                                Self::push(fibre, Value::new_unpack_marker());
                                for arg in positional_args.iter().rev() {
                                    Self::push(fibre, *arg);
                                }
                                for (name, val) in named_pairs.iter().rev() {
                                    Self::push(fibre, *name);
                                    Self::push(fibre, *val);
                                    Self::push(fibre, Value::new_named_arg_marker());
                                }
                            } else {
                                for arg in positional_args.iter().rev() {
                                    Self::push(fibre, *arg);
                                }
                                for (name, val) in named_pairs.iter().rev() {
                                    Self::push(fibre, *name);
                                    Self::push(fibre, *val);
                                }
                            }

                            // Calculate ip_rewind: opcode(1) + class_idx(2) + method_idx(2) + arity(1) + num_named(1) = 7
                            match Self::trigger_autoload(fibre, &class_name) {
                                Ok(true) => {
                                    let caller_idx = fibre.frame_count - 2;
                                    fibre.frames[caller_idx].ip = start_ip;
                                    continue 'vm_loop;
                                }
                                Ok(false) => {}
                                Err(e) => return ExecutionResult::Error(e),
                            }
                            return ExecutionResult::Error(format!(
                                "Class {} not found",
                                class_name
                            ));
                        }
                    };

                    let clean_class = class_name.trim_start_matches('\\');
                    if clean_class.eq_ignore_ascii_case("Closure") && method_name.eq_ignore_ascii_case("bind") {
                        if let Some(target_closure_val) = pos_forward.first() {
                            if let Some(closure_ptr) = target_closure_val.as_closure_ptr() {
                                let closure = unsafe { &*(closure_ptr as *const crate::types::closure::PhpClosure) };
                                let new_this = pos_forward.get(1).map(|v| *v).unwrap_or(Value::null());
                                let new_scope = pos_forward.get(2).map(|v| *v);
                                let mut called_class_id = closure.called_class_id;
                                if let Some(scope_val) = new_scope {
                                    if let Some(s_ptr) = scope_val.as_string_ptr() {
                                        let s = unsafe { &*(s_ptr as *const String) };
                                        if !s.eq_ignore_ascii_case("static") {
                                            if let Some(id) = Self::find_class_id(fibre, s.trim_start_matches('\\')) {
                                                called_class_id = id;
                                            }
                                        }
                                    } else if let Some(obj_ptr) = scope_val.as_object_ptr() {
                                        let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
                                        called_class_id = Self::resolve_obj_class_id(fibre, obj);
                                    } else if scope_val.is_null() {
                                        called_class_id = 0;
                                    }
                                } else if new_this.is_object() {
                                    let obj = unsafe { &*(new_this.as_object_ptr().unwrap() as *const hyperion_core::types::object::PhpObject) };
                                    called_class_id = Self::resolve_obj_class_id(fibre, obj);
                                }
                                let this_val = if new_this.is_object() { Some(new_this) } else { None };
                                let mut upvalues = closure.upvalues.clone();
                                let func = unsafe { &*closure.function_ptr.0 };
                                let num_params = func.params.len();
                                for (i, name) in func.chunk.local_names.iter().enumerate() {
                                    if name == "this" {
                                        if i >= 1 + num_params {
                                            let upvalue_idx = i - (1 + num_params);
                                            if upvalue_idx < upvalues.len() {
                                                upvalues[upvalue_idx] = new_this;
                                            }
                                        } else if i >= num_params {
                                            let upvalue_idx = i - num_params;
                                            if upvalue_idx < upvalues.len() {
                                                upvalues[upvalue_idx] = new_this;
                                            }
                                        }
                                    }
                                }
                                let new_closure = PhpClosure::new(closure.function_ptr, upvalues, called_class_id, this_val);
                                let new_ptr = fibre.arena.alloc_and_track(new_closure) as *mut ();
                                Self::push(fibre, Value::new_closure_ptr(new_ptr));
                                continue 'vm_loop;
                            }
                        }
                    }
                    if clean_class.eq_ignore_ascii_case("Closure") && method_name.eq_ignore_ascii_case("fromCallable") {
                        if let Some(target) = pos_forward.first() {
                            use hyperion_core::types::function::NativeContext;
                            match fibre.closure_from_callable(*target) {
                                Ok(c) => {
                                    Self::push(fibre, c);
                                    continue 'vm_loop;
                                }
                                Err(e) => return ExecutionResult::Error(e),
                            }
                        }
                    }

                    let current_frame_idx = fibre.frame_count - 1;
                    let mut find_res =
                        Self::find_method_fast(fibre, class_id, &method_name);
                    if let Ok(Some(ptr)) = find_res {
                        let func = unsafe { &*ptr.0 };
                        if !Self::is_method_accessible(fibre, class_id, func, current_frame_idx) {
                            find_res = Self::find_method_fast(fibre, class_id, "__callStatic");
                        }
                    } else if let Ok(None) = find_res {
                        find_res = Self::find_method_fast(fibre, class_id, "__callStatic");
                    }

                    let func_ptr = match find_res {
                        Ok(Some(ptr)) => ptr,
                        Ok(None) => {
                            if let Some(enum_res) = Self::try_execute_enum_builtin(fibre, class_id, &method_name, &pos_forward) {
                                match enum_res {
                                    Ok(res_val) => {
                                        Self::push(fibre, res_val);
                                        continue 'vm_loop;
                                    }
                                    Err(err) => return ExecutionResult::Error(err),
                                }
                            }
                            return ExecutionResult::Error(format!(
                                "Static method {}::{} not found",
                                class_name, method_name
                            ))
                        }
                        Err(parent_name) => {
                            // Restore the stack for replay: marker (unpacked form only),
                            // then args in their original bottom-to-top order.
                            if was_unpacked {
                                Self::push(fibre, Value::new_unpack_marker());
                                for arg in positional_args.iter().rev() {
                                    Self::push(fibre, *arg);
                                }
                                for (name, val) in named_pairs.iter().rev() {
                                    Self::push(fibre, *name);
                                    Self::push(fibre, *val);
                                    Self::push(fibre, Value::new_named_arg_marker());
                                }
                            } else {
                                for arg in positional_args.iter().rev() {
                                    Self::push(fibre, *arg);
                                }
                                for (name, val) in named_pairs.iter().rev() {
                                    Self::push(fibre, *name);
                                    Self::push(fibre, *val);
                                }
                            }
                            fibre.frames[current_frame_idx].ip = start_ip;

                            match Self::trigger_autoload(fibre, &parent_name) {
                                Ok(true) => continue 'vm_loop,
                                Ok(false) => {
                                    if Self::find_class_id(fibre, &parent_name).is_some() {
                                        continue 'vm_loop;
                                    }
                                    return ExecutionResult::Error(format!(
                                        "Class '{}' not found",
                                        parent_name
                                    ))
                                }
                                Err(e) => return ExecutionResult::Error(e),
                            }
                        }
                    };

                    let func = unsafe { &*func_ptr.0 }; debug_println!("autoload: pushed function {}", func.name);
                    let stack_window = fibre.stack_top;
                    let caller_frame = fibre.frames[fibre.frame_count - 1];
                    let mut current_this = fibre.stack[caller_frame.stack_window];
                    if let Some(closure_ptr) = current_this.as_closure_ptr() {
                        let closure = unsafe { &*(closure_ptr as *const crate::types::closure::PhpClosure) };
                        if let Some(bound_this) = closure.this_val {
                            current_this = bound_this;
                        } else {
                            current_this = Value::null();
                        }
                    } else if !current_this.is_object() {
                        current_this = Value::null();
                    }
                    Self::push(fibre, current_this);

                    let mut var_packed = false;
                    let actual_arity;
                    
                    let func_ptr_name = unsafe { &*func_ptr.0 }.name.to_lowercase();
                    if func_ptr_name == "__callstatic"
                        && method_name.to_lowercase() != "__callstatic"
                    {
                        let mut arr = hyperion_core::types::array::PhpArray::new();
                        for (i, arg) in pos_forward.iter().enumerate() {
                            arr.insert_int(i as i64, *arg);
                        }
                        for (name, val) in &named_forward {
                            if let Some(s) = name.as_string_ptr() {
                                let key_s = unsafe { (*(s as *const String)).clone() };
                                arr.insert_string_id(hyperion_core::types::string_table::intern_string(&key_s), *val);
                            }
                        }
                        let arr_ptr = fibre.arena.alloc_and_track(arr) as *mut ();
                        let arr_val = Value::new_array_ptr(arr_ptr);

                        let m_name_ptr =
                            fibre.arena.alloc_and_track(method_name.to_string()) as *mut String;
                        let m_name_val = Value::new_string_ptr(m_name_ptr as *mut ());

                        Self::push(fibre, m_name_val);
                        Self::push(fibre, arr_val);
                        actual_arity = 2;
                    } else {
                        let (args, vp) = Self::bind_call_arguments(&fibre.engine_state, func, &pos_forward, &named_forward, &fibre.arena);
                        var_packed = vp;
                        actual_arity = args.len();
                        for arg in &args {
                            Self::push(fibre, *arg);
                        }
                    }

                    if fibre.frame_count >= fibre.frames.len() {
                        return ExecutionResult::Error("Stack overflow".to_string());
                    }

                    let effective_called_class_id = if raw_class_name.starts_with("self\0") || raw_class_name.starts_with("parent\0") || raw_class_name.eq_ignore_ascii_case("self") || raw_class_name.eq_ignore_ascii_case("parent") {
                        let caller_called = Self::get_current_called_class_id(fibre);
                        if caller_called != 0 { caller_called } else { class_id }
                    } else {
                        class_id
                    };


                    let passed_args = pos_forward.clone();
                    fibre.frames[fibre.frame_count] = CallFrame {
                        function: func_ptr,
                        ip: 0,
                        stack_window,
                        called_class_id: effective_called_class_id,
                        return_override: None, eval_parent_stack_window: None,
                        arity: actual_arity,
                    };
                    fibre.set_frame_args(fibre.frame_count, &passed_args);
                    fibre.frame_count += 1;
                    
                    if class_name == "Closure" && method_name == "bind" {
                        hyperion_core::hyp_debug!("DEBUG StaticMethodCall Closure::bind before pad: stack_window={}, stack_top={}, actual_arity={}", stack_window, fibre.stack_top, actual_arity);
                    }
                    
                    Self::pad_missing_arguments_and_locals(fibre, actual_arity, unsafe {
                        &*func_ptr.0
                    }, &[], var_packed);
                    
                    if class_name == "Closure" && method_name == "bind" {
                        hyperion_core::hyp_debug!("DEBUG StaticMethodCall Closure::bind after pad: stack_top={}", fibre.stack_top);
                    }
                }
                op @ Opcode::DynamicStaticMethodCall | op @ Opcode::DynamicStaticMethodCallUnpacked => {
                    let (positional_args, named_pairs, args) = if op == Opcode::DynamicStaticMethodCallUnpacked {
                        let mut p_args = Vec::new();
                        let mut n_pairs = Vec::new();
                        while fibre.stack_top > 0 {
                            let val = Self::pop(fibre);
                            if val.is_unpack_marker() {
                                break;
                            }
                            if val.is_named_arg_marker() {
                                let arg_val = Self::pop(fibre);
                                let name_val = Self::pop(fibre);
                                n_pairs.push((name_val, arg_val));
                            } else {
                                p_args.push(val);
                            }
                        }
                        let a: Vec<Value> = p_args.iter().rev().cloned().collect();
                        (p_args, n_pairs, a)
                    } else {
                        let arity = Self::read_byte(fibre) as usize;
                        let num_named = Self::read_byte(fibre) as usize;

                        let mut named_pairs: Vec<(Value, Value)> = Vec::new();
                        let mut positional_args = Vec::new();
                        for _ in 0..num_named {
                            let val = Self::pop(fibre);
                            let name = Self::pop(fibre);
                            named_pairs.push((name, val));
                        }
                        for _ in 0..(arity - num_named) {
                            positional_args.push(Self::pop(fibre));
                        }

                        let mut args: Vec<Value> = positional_args.iter().rev().cloned().collect();
                        for (_name, val) in named_pairs.iter().rev() {
                            args.push(*val);
                        }
                        (positional_args, named_pairs, args)
                    };

                    let _null_val = Self::pop(fibre);
                    let method_name_val = Self::pop(fibre);
                    let class_name_val = Self::pop(fibre);
                    let method_name = method_name_val
                        .as_string_ptr()
                        .map(|p| unsafe { &*(p as *const String) }.clone())
                        .unwrap_or_default();
                    let class_name = if class_name_val.is_object() {
                        let obj_ptr = class_name_val.as_object_ptr().unwrap() as *const hyperion_core::types::object::PhpObject;
                        let obj = unsafe { &*obj_ptr };
                        if let Some(ref name) = obj.class_name {
                            name.clone()
                        } else {
                            let class_id = obj.class_id;
                            let classes = &fibre.engine_state.classes;
                            classes.get(&class_id).map(|c| c.name.clone()).unwrap_or_default()
                        }
                    } else {
                        class_name_val
                            .as_string_ptr()
                            .map(|p| unsafe { &*(p as *const String) }.clone())
                            .unwrap_or_default()
                    };
                    let resolved_class_name = Self::resolve_dynamic_class_name(fibre, &class_name);

                    let class_id = match Self::find_class_id(fibre, &resolved_class_name) {
                        Some(id) => id,
                        None => {
                            Self::push(fibre, class_name_val);
                                Self::push(fibre, method_name_val);
                                Self::push(fibre, _null_val);
                                if op == Opcode::DynamicStaticMethodCallUnpacked {
                                    Self::push(fibre, Value::new_unpack_marker());
                                    for arg in positional_args.iter().rev() {
                                        Self::push(fibre, *arg);
                                    }
                                    for (name, val) in named_pairs.iter().rev() {
                                        Self::push(fibre, *name);
                                        Self::push(fibre, *val);
                                        Self::push(fibre, Value::new_named_arg_marker());
                                    }
                                } else {
                                    for arg in positional_args.iter().rev() {
                                        Self::push(fibre, *arg);
                                    }
                                    for (name, val) in named_pairs.iter().rev() {
                                        Self::push(fibre, *name);
                                        Self::push(fibre, *val);
                                    }
                                }

                                match Self::trigger_autoload(fibre, &resolved_class_name) {
                                    Ok(true) => {
                                        let caller_idx = fibre.frame_count - 2;
                                        fibre.frames[caller_idx].ip = start_ip;
                                        continue 'vm_loop;
                                    }
                                    Ok(false) => {}
                                    Err(e) => return ExecutionResult::Error(e),
                                }
                                return ExecutionResult::Error(format!(
                                    "Class {} not found",
                                    resolved_class_name
                                ));
                            }
                    };

                    let clean_class = resolved_class_name.trim_start_matches('\\');
                    if clean_class.eq_ignore_ascii_case("Closure") && method_name.eq_ignore_ascii_case("bind") {
                        let pos_forward = &args;
                        if let Some(target_closure_val) = pos_forward.first() {
                            if let Some(closure_ptr) = target_closure_val.as_closure_ptr() {
                                let closure = unsafe { &*(closure_ptr as *const crate::types::closure::PhpClosure) };
                                let new_this = pos_forward.get(1).map(|v| *v).unwrap_or(Value::null());
                                let new_scope = pos_forward.get(2).map(|v| *v);
                                let mut called_class_id = closure.called_class_id;
                                if let Some(scope_val) = new_scope {
                                    if let Some(s_ptr) = scope_val.as_string_ptr() {
                                        let s = unsafe { &*(s_ptr as *const String) };
                                        if !s.eq_ignore_ascii_case("static") {
                                            if let Some(id) = Self::find_class_id(fibre, s.trim_start_matches('\\')) {
                                                called_class_id = id;
                                            }
                                        }
                                    } else if let Some(obj_ptr) = scope_val.as_object_ptr() {
                                        let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
                                        called_class_id = Self::resolve_obj_class_id(fibre, obj);
                                    } else if scope_val.is_null() {
                                        called_class_id = 0;
                                    }
                                } else if new_this.is_object() {
                                    let obj = unsafe { &*(new_this.as_object_ptr().unwrap() as *const hyperion_core::types::object::PhpObject) };
                                    called_class_id = Self::resolve_obj_class_id(fibre, obj);
                                }
                                let this_val = if new_this.is_object() { Some(new_this) } else { None };
                                let mut upvalues = closure.upvalues.clone();
                                let func = unsafe { &*closure.function_ptr.0 };
                                let num_params = func.params.len();
                                for (i, name) in func.chunk.local_names.iter().enumerate() {
                                    if name == "this" {
                                        if i >= 1 + num_params {
                                            let upvalue_idx = i - (1 + num_params);
                                            if upvalue_idx < upvalues.len() {
                                                upvalues[upvalue_idx] = new_this;
                                            }
                                        } else if i >= num_params {
                                            let upvalue_idx = i - num_params;
                                            if upvalue_idx < upvalues.len() {
                                                upvalues[upvalue_idx] = new_this;
                                            }
                                        }
                                    }
                                }
                                let new_closure = PhpClosure::new(closure.function_ptr, upvalues, called_class_id, this_val);
                                let new_ptr = fibre.arena.alloc_and_track(new_closure) as *mut ();
                                Self::push(fibre, Value::new_closure_ptr(new_ptr));
                                continue 'vm_loop;
                            }
                        }
                    }

                    let current_frame_idx = fibre.frame_count - 1;
                    let mut find_res =
                        Self::find_method_fast(fibre, class_id, &method_name);
                    if let Ok(Some(ptr)) = find_res {
                        let func = unsafe { &*ptr.0 };
                        if !Self::is_method_accessible(fibre, class_id, func, current_frame_idx) {
                            find_res = Self::find_method_fast(fibre, class_id, "__callStatic");
                        }
                    } else if let Ok(None) = find_res {
                        find_res = Self::find_method_fast(fibre, class_id, "__callStatic");
                    }

                    let func_ptr = match find_res {
                        Ok(Some(ptr)) => ptr,
                        Ok(None) => {
                            if let Some(enum_res) = Self::try_execute_enum_builtin(fibre, class_id, &method_name, &args) {
                                match enum_res {
                                    Ok(res_val) => {
                                        Self::push(fibre, res_val);
                                        continue 'vm_loop;
                                    }
                                    Err(err) => return ExecutionResult::Error(err),
                                }
                            }
                            return ExecutionResult::Error(format!(
                                "Static method {}::{} not found",
                                class_name, method_name
                            ))
                        }
                        Err(parent_name) => {
                            Self::push(fibre, class_name_val);
                            Self::push(fibre, method_name_val);
                            Self::push(fibre, _null_val);
                            if op == Opcode::DynamicStaticMethodCallUnpacked {
                                Self::push(fibre, Value::new_unpack_marker());
                                for arg in positional_args.iter().rev() {
                                    Self::push(fibre, *arg);
                                }
                                for (name, val) in named_pairs.iter().rev() {
                                    Self::push(fibre, *name);
                                    Self::push(fibre, *val);
                                    Self::push(fibre, Value::new_named_arg_marker());
                                }
                            } else {
                                for arg in positional_args.iter().rev() {
                                    Self::push(fibre, *arg);
                                }
                                for (name, val) in named_pairs.iter().rev() {
                                    Self::push(fibre, *name);
                                    Self::push(fibre, *val);
                                }
                            }
                            fibre.frames[current_frame_idx].ip = start_ip;

                            match Self::trigger_autoload(fibre, &parent_name) {
                                Ok(true) => continue 'vm_loop,
                                Ok(false) => {
                                    if Self::find_class_id(fibre, &parent_name).is_some() {
                                        continue 'vm_loop;
                                    }
                                    return ExecutionResult::Error(format!(
                                        "Class '{}' not found",
                                        parent_name
                                    ))
                                }
                                Err(e) => return ExecutionResult::Error(e),
                            }
                        }
                    };

                    let func = unsafe { &*func_ptr.0 }; debug_println!("autoload: pushed function {}", func.name);
                    let stack_window = fibre.stack_top;
                    let caller_frame = fibre.frames[fibre.frame_count - 1];
                    let mut current_this = fibre.stack[caller_frame.stack_window];
                    if let Some(closure_ptr) = current_this.as_closure_ptr() {
                        let closure = unsafe { &*(closure_ptr as *const crate::types::closure::PhpClosure) };
                        if let Some(bound_this) = closure.this_val {
                            current_this = bound_this;
                        } else {
                            current_this = Value::null();
                        }
                    } else if !current_this.is_object() {
                        current_this = Value::null();
                    }
                    Self::push(fibre, current_this);

                    let mut var_packed = false;
                    let mut actual_arity = args.len();
                    let func_ptr_name = unsafe { &*func_ptr.0 }.name.to_lowercase();
                    if func_ptr_name == "__callstatic"
                        && method_name.to_lowercase() != "__callstatic"
                    {
                        let mut arr = hyperion_core::types::array::PhpArray::new();
                        for (i, arg) in positional_args.iter().rev().enumerate() {
                            arr.insert_int(i as i64, *arg);
                        }
                        for (name, val) in named_pairs.iter().rev() {
                            if let Some(s) = name.as_string_ptr() {
                                let key_s = unsafe { (*(s as *const String)).clone() };
                                arr.insert_string_id(hyperion_core::types::string_table::intern_string(&key_s), *val);
                            }
                        }
                        let arr_ptr = fibre.arena.alloc_and_track(arr) as *mut ();
                        let arr_val = Value::new_array_ptr(arr_ptr);

                        let m_name_ptr =
                            fibre.arena.alloc_and_track(method_name.to_string()) as *mut String;
                        let m_name_val = Value::new_string_ptr(m_name_ptr as *mut ());

                        Self::push(fibre, m_name_val);
                        Self::push(fibre, arr_val);
                        actual_arity = 2;
                    } else {
                        let pos_forward: Vec<Value> = positional_args.iter().rev().cloned().collect();
                        let named_forward: Vec<(Value, Value)> = named_pairs.iter().rev().cloned().collect();
                        let (bound, vp) = Self::bind_call_arguments(&fibre.engine_state, func, &pos_forward, &named_forward, &fibre.arena);
                        var_packed = vp;
                        actual_arity = bound.len();
                        for arg in &bound {
                            Self::push(fibre, *arg);
                        }
                    }

                    if fibre.frame_count >= fibre.frames.len() {
                        return ExecutionResult::Error("Stack overflow".to_string());
                    }

                    let effective_called_class_id = if class_name.starts_with("self\0") || class_name.starts_with("parent\0") || class_name.eq_ignore_ascii_case("self") || class_name.eq_ignore_ascii_case("parent") {
                        let caller_called = Self::get_current_called_class_id(fibre);
                        if caller_called != 0 {
                            caller_called
                        } else {
                            class_id
                        }
                    } else {
                        class_id
                    };


                    let passed_args = args.clone();
                    fibre.frames[fibre.frame_count] = CallFrame {
                        function: func_ptr,
                        ip: 0,
                        stack_window,
                        called_class_id: effective_called_class_id,
                        return_override: None, eval_parent_stack_window: None,
                        arity: actual_arity,
                    };

                    fibre.set_frame_args(fibre.frame_count, &passed_args);
                    fibre.frame_count += 1;
                    Self::pad_missing_arguments_and_locals(fibre, actual_arity, unsafe {
                        &*func_ptr.0
                    }, &[], var_packed);
                }
                Opcode::LateStaticMethodCall => {
                    let name_idx = Self::read_short(fibre) as usize;
                    let arity = Self::read_byte(fibre) as usize;
                    let num_named = Self::read_byte(fibre) as usize;

                    let mut named_pairs = Vec::with_capacity(num_named);
                    for _ in 0..num_named {
                        let val = Self::pop(fibre);
                        let name = Self::pop(fibre);
                        named_pairs.push((name, val));
                    }
                    let mut positional_args = Vec::with_capacity(arity.saturating_sub(num_named));
                    for _ in 0..(arity.saturating_sub(num_named)) {
                        positional_args.push(Self::pop(fibre));
                    }

                    let pos_forward: Vec<Value> = positional_args.iter().rev().copied().collect();
                    let named_forward: Vec<(Value, Value)> = named_pairs.iter().rev().copied().collect();

                    let frame = fibre.frames[fibre.frame_count - 1];
                    let func = unsafe { &*frame.function.0 };
                    let name_val = func.chunk.constants[name_idx];
                    let name_ptr = name_val.as_string_ptr().unwrap() as *mut String;
                    let method_name = unsafe { (*name_ptr).clone() };

                    // Get the late static binding class ID (called_class_id)
                    let class_id = Self::get_current_called_class_id(fibre);

                    let class_name = {
                        let classes = &fibre.engine_state.classes;
                        classes.get(&class_id).map(|c| c.name.clone()).unwrap_or_default()
                    };

                    let current_frame_idx = fibre.frame_count - 1;
                    let mut find_res =
                        Self::find_method_fast(fibre, class_id, &method_name);

                    if let Ok(Some(ptr)) = find_res {
                        let func = unsafe { &*ptr.0 };
                        if !Self::is_method_accessible(fibre, class_id, func, current_frame_idx) {
                            find_res = Self::find_method_fast(fibre, class_id, "__callStatic");
                        }
                    } else if let Ok(None) = find_res {
                        find_res = Self::find_method_fast(fibre, class_id, "__callStatic");
                    }

                    let func_ptr = match find_res {
                        Ok(Some(ptr)) => ptr,
                        Ok(None) => {
                            return ExecutionResult::Error(format!(
                                "Static method {}::{} not found",
                                class_name, method_name
                            ))
                        }
                        Err(parent_name) => {
                            for arg in positional_args.iter().rev() {
                                Self::push(fibre, *arg);
                            }
                            for (name, val) in named_pairs.iter().rev() {
                                Self::push(fibre, *name);
                                Self::push(fibre, *val);
                            }
                            fibre.frames[current_frame_idx].ip = start_ip;

                            match Self::trigger_autoload(fibre, &parent_name) {
                                Ok(true) => continue 'vm_loop,
                                Ok(false) => {
                                    if Self::find_class_id(fibre, &parent_name).is_some() {
                                        continue 'vm_loop;
                                    }
                                    return ExecutionResult::Error(format!(
                                        "Class '{}' not found",
                                        parent_name
                                    ))
                                }
                                Err(e) => return ExecutionResult::Error(e),
                            }
                        }
                    };

                    let func = unsafe { &*func_ptr.0 }; debug_println!("autoload: pushed function {}", func.name);
                let stack_window = fibre.stack_top;
                    let caller_frame = fibre.frames[fibre.frame_count - 1];
                    let mut current_this =
                        fibre.stack[caller_frame.stack_window];
                    if let Some(closure_ptr) = current_this.as_closure_ptr() {
                        let closure = unsafe { &*(closure_ptr as *const crate::types::closure::PhpClosure) };
                        current_this = closure.this_val.unwrap_or(Value::null());
                    } else if !current_this.is_object() {
                        current_this = Value::null();
                    }
                    Self::push(fibre, current_this);

                    let mut var_packed = false;
                    let actual_arity;
                    let func_ptr_name = unsafe { &*func_ptr.0 }.name.to_lowercase();
                    if func_ptr_name == "__callstatic"
                        && method_name.to_lowercase() != "__callstatic"
                    {
                        let mut arr = hyperion_core::types::array::PhpArray::new();
                        for (i, arg) in positional_args.iter().rev().enumerate() {
                            arr.insert_int(i as i64, *arg);
                        }
                        for (name, val) in named_pairs.iter().rev() {
                            if let Some(s) = name.as_string_ptr() {
                                let key_s = unsafe { (*(s as *const String)).clone() };
                                arr.insert_string_id(hyperion_core::types::string_table::intern_string(&key_s), *val);
                            }
                        }
                        let arr_ptr = fibre.arena.alloc_and_track(arr) as *mut ();
                        let arr_val = Value::new_array_ptr(arr_ptr);

                        let m_name_ptr =
                            fibre.arena.alloc_and_track(method_name.to_string()) as *mut String;
                        let m_name_val = Value::new_string_ptr(m_name_ptr as *mut ());

                        Self::push(fibre, m_name_val);
                        Self::push(fibre, arr_val);
                        actual_arity = 2;
                    } else {
                        let (args, vp) = Self::bind_call_arguments(&fibre.engine_state, func, &pos_forward, &named_forward, &fibre.arena);
                        var_packed = vp;
                        actual_arity = args.len();
                        for arg in &args {
                            Self::push(fibre, *arg);
                        }
                    }


                    if fibre.frame_count >= fibre.frames.len() {
                        return ExecutionResult::Error("Stack overflow".to_string());
                    }

                    let passed_args = pos_forward.clone();
                    fibre.frames[fibre.frame_count] = CallFrame {
                        function: func_ptr,
                        ip: 0,
                        stack_window,
                        called_class_id: class_id,
                        return_override: None, eval_parent_stack_window: None,
                        arity: actual_arity,
                    };
                    fibre.set_frame_args(fibre.frame_count, &passed_args);
                    fibre.frame_count += 1;
                    Self::pad_missing_arguments_and_locals(fibre, actual_arity, unsafe {
                        &*func_ptr.0
                    }, &[], var_packed);
                }
                Opcode::LateStaticPropertySet => {
                    let prop_idx = Self::read_short(fibre);
                    let raw_val = Self::pop(fibre);
                    let val = if raw_val.deref().is_array() {
                        Self::deep_copy_value(fibre, raw_val.deref())
                    } else {
                        raw_val.deref()
                    };

                    let called_class_id = Self::get_current_called_class_id(fibre);
                    let frame = fibre.frames[fibre.frame_count - 1];
                    let func = unsafe { &*frame.function.0 };
                    let prop_val = func.chunk.constants[prop_idx as usize];
                    let prop_name =
                        unsafe { &*(prop_val.as_string_ptr().unwrap() as *const String) };

                    let classes = &fibre.engine_state.classes;
                    let class_name = classes.get(&called_class_id).map(|c| c.name.clone()).unwrap_or_default();
                    drop(classes);

                    fibre.set_static_property(&class_name, prop_name, val);
                    Self::push(fibre, val);
                }

                Opcode::CallIntrinsic => {
                    let intrinsic_id = Self::read_byte(fibre);
                    let arity = Self::read_byte(fibre);

                    if intrinsic_id == 9 {
                        // Create empty array
                        let arr = hyperion_core::types::array::PhpArray::new();
                        let arr_ptr = fibre.arena.alloc_and_track(arr) as *mut ();
                        let array_val = Value::new_array_ptr(arr_ptr);
                        Self::push(fibre, array_val);
                    } else if intrinsic_id == 10 {
                        // ArraySpread: stack has [target_array, source_iterable]
                        let source_val = Self::pop(fibre);
                        let target_val = Self::pop(fibre);
                        if let Some(target_ptr) = target_val.as_array_ptr() {
                            let target_arr = unsafe { &mut *(target_ptr as *mut hyperion_core::types::array::PhpArray) };
                            if let Some(source_ptr) = source_val.as_array_ptr() {
                                let source_arr = unsafe { &*(source_ptr as *const hyperion_core::types::array::PhpArray) };
                                for (k, v) in source_arr.elements.iter() {
                                    match k {
                                        hyperion_core::types::array::ArrayKey::Int(_) => {
                                            target_arr.push(*v);
                                        }
                                        hyperion_core::types::array::ArrayKey::StringId(s) => {
                                            target_arr.insert_string_id(*s, *v);
                                        }
                                    }
                                }
                            } else if source_val.is_object() {
                                if let Ok(arr_val) = fibre.iterator_to_array(source_val, true) {
                                    if let Some(arr_ptr) = arr_val.as_array_ptr() {
                                        let iter_arr = unsafe { &*(arr_ptr as *const hyperion_core::types::array::PhpArray) };
                                        for (k, v) in iter_arr.elements.iter() {
                                            match k {
                                                hyperion_core::types::array::ArrayKey::Int(_) => {
                                                    target_arr.push(*v);
                                                }
                                                hyperion_core::types::array::ArrayKey::StringId(s) => {
                                                    target_arr.insert_string_id(*s, *v);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Self::push(fibre, target_val);
                        } else {
                            Self::push(fibre, target_val);
                        }
                    } else if intrinsic_id == 13 {
                        // spl_autoload_register(callable, throw=true, prepend=false)
                        let mut args = Vec::new();
                        for _ in 0..arity {
                            args.push(Self::pop(fibre));
                        }
                        args.reverse();
                        if let Some(func) = args.first() {
                            let prepend = args.get(2).map(is_truthy).unwrap_or(false);
                            if prepend {
                                fibre.autoloaders.insert(0, *func);
                            } else {
                                fibre.autoloaders.push(*func);
                            }
                            Self::push(fibre, Value::new_bool(true));
                        } else {
                            Self::push(fibre, Value::new_bool(false));
                        }
                    } else if intrinsic_id == 16 {
                        // spl_autoload_unregister(callable)
                        let mut args = Vec::new();
                        for _ in 0..arity {
                            args.push(Self::pop(fibre));
                        }
                        args.reverse();
                        if let Some(func) = args.first() {
                            let func_clone = *func;
                            fibre.autoloaders.retain(|x| {
                                // Simple pointer equality for closures/strings
                                x.0 != func_clone.0
                            });
                        }
                        Self::push(fibre, Value::new_bool(true));
                    } else if intrinsic_id == 17 {
                        // spl_autoload_functions()
                        for _ in 0..arity {
                            Self::pop(fibre);
                        }
                        let mut arr = hyperion_core::types::array::PhpArray::new();
                        for (i, loader) in fibre.autoloaders.iter().enumerate() {
                            arr.insert_int(i as i64, *loader);
                        }
                        let arr_ptr = fibre.arena.alloc_and_track(arr) as *mut ();
                        Self::push(fibre, Value::new_array_ptr(arr_ptr));
                    } else if intrinsic_id == 30 {
                        // isset
                        let mut args = Vec::new();
                        for _ in 0..arity {
                            args.push(Self::pop(fibre));
                        }
                        let result = args.iter().all(|v| !v.is_null());
                        Self::push(fibre, Value::new_bool(result));
                    } else if intrinsic_id == 31 {
                        // empty
                        let mut args = Vec::new();
                        for _ in 0..arity {
                            args.push(Self::pop(fibre));
                        }
                        let result = args.first().map(|v| !is_truthy(v)).unwrap_or(true);
                        Self::push(fibre, Value::new_bool(result));
                    } else if intrinsic_id == 32 {
                        // unset
                        for _ in 0..arity {
                            Self::pop(fibre);
                        }
                        Self::push(fibre, Value::null());
                    } else {
                        // Other intrinsics can be added here
                        for _ in 0..arity {
                            Self::pop(fibre);
                        }
                        Self::push(fibre, Value::null());
                    }
                }
                Opcode::CompareSpaceship => {
                    let b = Self::pop(fibre);
                    let a = Self::pop(fibre);
                    let ord = Self::php_compare(&a, &b);
                    Self::push(
                        fibre,
                        Value::new_int(match ord {
                            std::cmp::Ordering::Less => -1,
                            std::cmp::Ordering::Equal => 0,
                            std::cmp::Ordering::Greater => 1,
                        }),
                    );
                }
                Opcode::GetStaticVar => {
                    let name_idx = Self::read_short(fibre) as usize;
                    let local_idx = Self::read_byte(fibre) as usize;
                    let offset = Self::read_short(fibre) as usize;

                    let (fn_key, var_name) = {
                        let frame = &fibre.frames[fibre.frame_count - 1];
                        let func = unsafe { &*frame.function.0 };
                        let name_val = func.chunk.constants[name_idx];
                        let name_ptr = name_val.as_string_ptr().unwrap() as *const String;
                        (frame.function.0 as usize, unsafe { (*name_ptr).clone() })
                    };

                    let statics = fibre.function_statics.lock().unwrap();
                    if let Some(&ref_val) = statics.get(&(fn_key, var_name)) {
                        drop(statics);
                        let frame = &mut fibre.frames[fibre.frame_count - 1];
                        let slot = frame.stack_window + local_idx;
                        fibre.stack[slot] = ref_val;
                        frame.ip += offset;
                    }
                }
                Opcode::InitStaticVar => {
                    let name_idx = Self::read_short(fibre) as usize;
                    let local_idx = Self::read_byte(fibre) as usize;

                    let (fn_key, var_name) = {
                        let frame = &fibre.frames[fibre.frame_count - 1];
                        let func = unsafe { &*frame.function.0 };
                        let name_val = func.chunk.constants[name_idx];
                        let name_ptr = name_val.as_string_ptr().unwrap() as *const String;
                        (frame.function.0 as usize, unsafe { (*name_ptr).clone() })
                    };

                    let val = Self::pop(fibre);
                    let cell = hyperion_core::types::reference::PhpRef::new(val);
                    let cell_ptr = fibre.arena.alloc_and_track(cell);
                    let ref_val = Value::new_ref_ptr(cell_ptr as *mut ());

                    {
                        let mut statics = fibre.function_statics.lock().unwrap();
                        statics.insert((fn_key, var_name), ref_val);
                    }
                    let frame = &fibre.frames[fibre.frame_count - 1];
                    let slot = frame.stack_window + local_idx;
                    fibre.stack[slot] = ref_val;
                }
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn jit_fetch_object_property(obj_ptr: u64, name_idx: u16, chunk_ptr: u64) -> u64 {
    if obj_ptr == 0 || chunk_ptr == 0 {
        return hyperion_core::memory::nan_box::Value::null().0;
    }

    let chunk = unsafe { &*(chunk_ptr as *const hyperion_bytecode::Chunk) };
    let name_val = chunk.constants[name_idx as usize];

    if let Some(name_str_ptr) = name_val.as_string_ptr() {
        let name_str = unsafe { &*(name_str_ptr as *const String) };
        let php_obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
        if let Some(val) = php_obj.properties.get(name_str) {
            return val.0;
        }
    }

    hyperion_core::memory::nan_box::Value::null().0
}

#[no_mangle]
pub extern "C" fn jit_set_object_property(
    obj_ptr: u64,
    val_bits: u64,
    name_idx: u16,
    chunk_ptr: u64,
) {
    if obj_ptr == 0 || chunk_ptr == 0 {
        return;
    }

    let chunk = unsafe { &*(chunk_ptr as *const hyperion_bytecode::Chunk) };
    let name_val = chunk.constants[name_idx as usize];
    let val = hyperion_core::memory::nan_box::Value(val_bits);

    if let Some(name_str_ptr) = name_val.as_string_ptr() {
        let name_str = unsafe { &*(name_str_ptr as *const String) };
        let php_obj = unsafe { &mut *(obj_ptr as *mut hyperion_core::types::object::PhpObject) };

        php_obj.properties.insert(name_str.clone(), val);
    }
}
