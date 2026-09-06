use crate::types::class::PhpClass;
use crate::types::function::FunctionPtr;
use hyperion_core::memory::nan_box::Value;
use mio::net::TcpStream;
use std::collections::HashMap;
use std::sync::Arc;

const STACK_MAX: usize = 65536; // Dynamic stack for deep Laravel pipelines
const FRAMES_MAX: usize = 4096; // Max call depth

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum FibreState {
    Ready,     // Ready to execute (waiting in scheduler queue)
    Running,   // Currently executing on a core
    Suspended, // Suspended (waiting for I/O from reactor or yielded)
    Dead,      // Execution finished, safe to drop
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestProtocol {
    Http,
    FastCgi { request_id: u16, keep_conn: bool },
}

#[derive(Copy, Clone)]
pub struct CallFrame {
    pub function: FunctionPtr,
    pub ip: usize,
    pub stack_window: usize,
    pub called_class_id: usize, // For Late Static Binding (static::)
    pub return_override: Option<Value>, // For __set magic method
    pub eval_parent_stack_window: Option<usize>, // For copying locals back after eval()
    pub arity: usize,
}

// Since CallFrame has raw pointers, implement Send manually (it's safe as long as Fibre handles it safely)
unsafe impl Send for CallFrame {}

pub struct Fibre {
    pub trace_log: Vec<String>,
    pub id: u64,
    pub state: FibreState,
    pub protocol: RequestProtocol,
    pub frames: Vec<CallFrame>,
    pub frame_count: usize,
    pub stack: Vec<Value>,
    pub stack_top: usize,
    pub arena: std::sync::Arc<hyperion_core::gc::arena::GcArena>,
    pub output_buffer: Vec<u8>,
    pub ob_buffers: Vec<Vec<u8>>,
    pub tcp_stream: Option<TcpStream>,
    pub engine_state: Arc<GlobalEngineState>,
    pub superglobals: [Value; 8],
    pub included_files: std::collections::HashSet<String>,
    pub included_file_returns: HashMap<String, Value>,
    pub frame_included_paths: HashMap<usize, String>,
    pub autoload_state: HashMap<String, usize>,


    pub static_properties: std::sync::Mutex<HashMap<(String, String), Value>>,
    pub function_statics: std::sync::Mutex<HashMap<(usize, String), Value>>,
    pub autoload_frame_depths: Vec<usize>,
    pub autoloaders: Vec<Value>,
    pub generator_key: Value,
    pub generator_value: Value,
    pub generator_yield_counter: i64,
    pub generator_started: bool,
    pub unwind_action: Option<UnwindAction>,
    pub generator_valid: bool,
    pub is_generator: bool,
    pub is_keep_alive_idle: bool,
    pub http_parse_buffer: Vec<u8>,
    pub stop_frame_count: Option<usize>,
    pub tracer: Option<hyperion_jit::tracer::Tracer>,
    pub local_heat_map: HashMap<(usize, usize), u32>,
    pub jit_deopt_count: HashMap<(usize, usize), u32>,
    pub headers_sent: bool,
    pub response_status_code: u16,
    pub response_headers: Vec<(String, String)>,
    pub dynamic_locals: HashMap<usize, HashMap<String, Value>>,
    pub frame_args: Vec<Vec<Value>>,
    pub magic_guards: std::collections::HashSet<(usize, String, u8)>,
    pub http_request_method: String,
    pub http_request_uri: String,
    pub http_keep_alive: bool,
    pub http_raw_body: Vec<u8>,
    pub boot_checkpoint: Option<hyperion_core::gc::arena::ArenaCheckpoint>,
    pub boot_static_properties: Option<HashMap<(String, String), Value>>,
    pub boot_function_statics: Option<HashMap<(usize, String), Value>>,
    pub boot_stack_top: usize,
    pub boot_stack_snapshot: Option<Vec<Value>>,
    pub boot_frame_count: usize,
    pub boot_call_frame: Option<CallFrame>,
    pub boot_autoloaders: Option<Vec<Value>>,
    pub boot_included_files: Option<std::collections::HashSet<String>>,
    pub warm_request_count: usize,
    pub entry_func_ptr: FunctionPtr,
    pub reactor_id: usize,
}

impl Clone for Fibre {
    fn clone(&self) -> Self {
        let static_props = self.static_properties.lock().unwrap().clone();
        let fn_statics = self.function_statics.lock().unwrap().clone();
        Self {
            trace_log: self.trace_log.clone(),
            id: self.id,
            state: self.state,
            protocol: self.protocol.clone(),
            frames: self.frames.clone(),
            frame_count: self.frame_count,
            stack: self.stack.clone(),
            stack_top: self.stack_top,
            arena: std::sync::Arc::clone(&self.arena),
            output_buffer: self.output_buffer.clone(),
            ob_buffers: self.ob_buffers.clone(),

            tcp_stream: None,
            engine_state: self.engine_state.clone(),
            superglobals: self.superglobals,
            included_files: self.included_files.clone(),
            included_file_returns: self.included_file_returns.clone(),
            frame_included_paths: self.frame_included_paths.clone(),
            autoload_state: self.autoload_state.clone(),
            static_properties: std::sync::Mutex::new(static_props),
            function_statics: std::sync::Mutex::new(fn_statics),
            autoload_frame_depths: self.autoload_frame_depths.clone(),
            autoloaders: self.autoloaders.clone(),
            generator_key: self.generator_key,
            generator_value: self.generator_value,
            generator_yield_counter: self.generator_yield_counter,
            generator_started: self.generator_started,
            unwind_action: self.unwind_action.clone(),
            generator_valid: self.generator_valid,
            is_generator: self.is_generator,
            is_keep_alive_idle: self.is_keep_alive_idle,
            http_parse_buffer: self.http_parse_buffer.clone(),
            stop_frame_count: self.stop_frame_count,
            tracer: None,
            local_heat_map: self.local_heat_map.clone(),
            jit_deopt_count: self.jit_deopt_count.clone(),
            headers_sent: self.headers_sent,
            response_status_code: self.response_status_code,
            response_headers: self.response_headers.clone(),
            dynamic_locals: self.dynamic_locals.clone(),
            frame_args: self.frame_args.clone(),
            magic_guards: std::collections::HashSet::new(),
            http_request_method: self.http_request_method.clone(),
            http_request_uri: self.http_request_uri.clone(),
            http_keep_alive: self.http_keep_alive,
            http_raw_body: self.http_raw_body.clone(),
            entry_func_ptr: self.entry_func_ptr,
            reactor_id: self.reactor_id,
            boot_checkpoint: self.boot_checkpoint,
            boot_static_properties: self.boot_static_properties.clone(),
            boot_function_statics: self.boot_function_statics.clone(),
            boot_stack_top: self.boot_stack_top,
            boot_stack_snapshot: self.boot_stack_snapshot.clone(),
            boot_frame_count: self.boot_frame_count,
            boot_call_frame: self.boot_call_frame,
            boot_autoloaders: self.boot_autoloaders.clone(),
            boot_included_files: self.boot_included_files.clone(),
            warm_request_count: self.warm_request_count,
        }
    }
}


#[derive(Clone, Debug)]
pub enum UnwindAction {
    Return(Value),
    Throw(Value),
}

pub struct OpCacheEntry {
    pub cached_mtime: std::sync::atomic::AtomicU64,
    pub last_checked_time: std::sync::atomic::AtomicU64,
    pub func_ptr: FunctionPtr,
}

pub struct GlobalEngineState {
    pub constants: dashmap::DashMap<String, Value>,
    pub functions: dashmap::DashMap<usize, FunctionPtr>,
    pub classes: dashmap::DashMap<usize, PhpClass>,
    pub class_map: dashmap::DashMap<String, usize>,
    pub func_map: dashmap::DashMap<String, usize>,
    pub opcache: dashmap::DashMap<String, OpCacheEntry>,
    pub default_statics: dashmap::DashMap<(String, String), Value>,
    pub jit_cache: dashmap::DashMap<(usize, usize), std::sync::Arc<hyperion_jit::assembler::CompiledTrace>>,
    pub global_arena: hyperion_core::gc::arena::GlobalGcArena,
    pub db_pool: std::sync::RwLock<Vec<String>>, // Stub for PDO Database Connection Pooling
    pub parked_db_fibres: dashmap::DashMap<u64, Fibre>,
    pub pending_db_results: dashmap::DashMap<u64, Result<Value, String>>,
    pub db_completion_tx: crossbeam_channel::Sender<(u64, Result<Value, String>)>,
    pub db_completion_rx: crossbeam_channel::Receiver<(u64, Result<Value, String>)>,
    pub unparkers: std::sync::RwLock<Vec<crossbeam_utils::sync::Unparker>>,
    pub autoloaders: std::sync::RwLock<Vec<Value>>,
    pub next_class_id: std::sync::atomic::AtomicUsize,
    pub next_func_id: std::sync::atomic::AtomicUsize,
    pub compilation_lock: std::sync::Mutex<()>,
}

impl GlobalEngineState {
    pub fn new() -> Self {
        let (db_completion_tx, db_completion_rx) = crossbeam_channel::unbounded();
        Self {
            constants: dashmap::DashMap::new(),
            functions: dashmap::DashMap::new(),
            classes: dashmap::DashMap::new(),
            class_map: dashmap::DashMap::new(),
            func_map: dashmap::DashMap::new(),
            opcache: dashmap::DashMap::new(),
            default_statics: dashmap::DashMap::new(),
            jit_cache: dashmap::DashMap::new(),
            global_arena: hyperion_core::gc::arena::GlobalGcArena::new(),
            db_pool: std::sync::RwLock::new(Vec::new()),
            parked_db_fibres: dashmap::DashMap::new(),
            pending_db_results: dashmap::DashMap::new(),
            db_completion_tx,
            db_completion_rx,
            unparkers: std::sync::RwLock::new(Vec::new()),
            autoloaders: std::sync::RwLock::new(Vec::new()),
            next_class_id: std::sync::atomic::AtomicUsize::new(0),
            next_func_id: std::sync::atomic::AtomicUsize::new(0),
            compilation_lock: std::sync::Mutex::new(()),
        }
    }

    pub fn merge_compilation_result(&self, result: &mut hyperion_compiler::compiler::CompilationResult, file_path: &str) {
        let classes = &self.classes;
        let default_statics = &self.default_statics;
        for (iface, const_name, value) in result.interface_constants.drain(..) {
            default_statics
                .entry((crate::vm::normalize_name(&iface), const_name))
                .or_insert(value);
        }
        for mut comp_class in result.classes.drain(..) {
            let norm_cls_name = crate::vm::normalize_name(&comp_class.name);
            if self.class_map.contains_key(&norm_cls_name) {
                continue;
            }
            let runtime_class_id = self.next_class_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let real_path = std::fs::canonicalize(file_path)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| file_path.to_string());
            let static_props_map: indexmap::IndexMap<String, (Value, hyperion_parser::parser::ast::Visibility)> =
                comp_class.static_properties.iter().map(|(n, v, vis)| (n.clone(), (*v, *vis))).collect();
            let mut php_class = crate::types::class::PhpClass {
                id: runtime_class_id,
                name: comp_class.name.clone(),
                methods: std::collections::HashMap::new(),
                extends: comp_class.extends.clone(),
                implements: comp_class.implements.clone(),
                traits: comp_class.traits.clone(),
                default_properties: comp_class.default_properties.into_iter().map(|(n, v, vis)| (n, (v, vis))).collect(),
                static_properties: static_props_map,
                attributes: comp_class.attributes.into_iter().map(|a| crate::types::class::ClassAttribute {
                    name: a.name,
                    args: a.args,
                }).collect(),
                filename: Some(real_path),
                is_interface: comp_class.is_interface,
                is_trait: comp_class.is_trait,
                is_enum: comp_class.is_enum,
                is_abstract: comp_class.is_abstract,
                is_final: comp_class.is_final,
                is_readonly: comp_class.is_readonly,
            };
            for (prop_name, default_val, _vis) in comp_class.static_properties {
                if let Some(obj_ptr) = default_val.as_object_ptr() {
                    let obj = unsafe { &mut *(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
                    if obj.class_name.is_none() {
                        obj.class_id = runtime_class_id;
                        obj.class_name = Some(comp_class.name.clone());
                    } else if let Some(ref cn) = obj.class_name {
                        let norm_cn = crate::vm::normalize_name(cn);
                        if let Some(target_id) = self.class_map.get(&norm_cn).map(|v| *v) {
                            obj.class_id = target_id;
                        } else if norm_cn == norm_cls_name {
                            obj.class_id = runtime_class_id;
                        }
                    }
                }
                default_statics.insert(
                    (norm_cls_name.clone(), prop_name),
                    default_val,
                );
            }
            for comp_func in comp_class.methods.drain(..) {
                let method_name = comp_func.name.clone();
                let class_attrs: Vec<crate::types::class::ClassAttribute> = comp_func.attributes.into_iter().map(|a| crate::types::class::ClassAttribute {
                    name: a.name,
                    args: a.args,
                }).collect();
                let func = Box::new(crate::types::function::PhpFunction::new_with_attributes(
                    comp_func.name,
                    comp_func.arity,
                    comp_func.chunk,
                    comp_func.params,
                    true, // is_method
                    comp_func.is_static,
                    comp_func.visibility,
                    comp_func.num_locals,
                    class_attrs,
                ));
                let fptr = crate::types::function::FunctionPtr(Box::into_raw(func));
                let norm_method = crate::vm::normalize_name(&method_name);
                php_class.methods.insert(method_name.clone(), fptr);
                if norm_method != method_name {
                    php_class.methods.insert(norm_method, fptr);
                }
            }
            let cls_id = php_class.id;
            classes.insert(php_class.id, php_class);
            self.class_map.insert(norm_cls_name, cls_id);
            crate::vm::VM::invalidate_method_cache();
        }

        let functions = &self.functions;
        for comp_func in result.functions.drain(..) {
            let norm_func_name = crate::vm::normalize_name(&comp_func.name);
            if self.func_map.contains_key(&norm_func_name) {
                continue;
            }
            let func_name = comp_func.name.clone();
            let class_attrs: Vec<crate::types::class::ClassAttribute> = comp_func.attributes.into_iter().map(|a| crate::types::class::ClassAttribute {
                name: a.name,
                args: a.args,
            }).collect();
            let func = Box::new(crate::types::function::PhpFunction::new_with_attributes(
                comp_func.name,
                comp_func.arity,
                comp_func.chunk,
                comp_func.params,
                false, // is_method
                comp_func.is_static,
                comp_func.visibility,
                comp_func.num_locals,
                class_attrs,
            ));
            let func_id = self.next_func_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            functions.insert(func_id, crate::types::function::FunctionPtr(Box::into_raw(func)));
            self.func_map.insert(crate::vm::normalize_name(&func_name), func_id);
        }
    }

    pub fn compile_and_load_script(
        self: &Arc<Self>,
        file_path: &str,
    ) -> Result<FunctionPtr, String> {
        // 1. Check OPCache first (fast in-memory path)
        if let Some(entry) = self.opcache.get(file_path) {
            return Ok(entry.func_ptr);
        }

        // 2. Read file
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read file '{}': {}", file_path, e))?;

        self.compile_and_load_source(file_path, &content)
    }

    pub fn compile_and_load_source(
        self: &Arc<Self>,
        file_path: &str,
        content: &str,
    ) -> Result<FunctionPtr, String> {
        let _compile_guard = self.compilation_lock.lock().unwrap();

        if let Some(entry) = self.opcache.get(file_path) {
            return Ok(entry.func_ptr);
        }

        let current_mtime = std::fs::metadata(file_path).and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);
        // 3. Compile
        let lexer = hyperion_parser::lexer::Lexer::new(content);
        let mut parser = hyperion_parser::parser::Parser::new(lexer);
        let program = parser.parse_program();
        let comp = hyperion_compiler::compiler::Compiler::new(file_path.to_string());
        let mut result = comp.compile(program);

        self.merge_compilation_result(&mut result, file_path);

        // 6. Create main FunctionPtr
        let main_func = Box::new(crate::types::function::PhpFunction::new(
            format!("__main_{}", file_path),
            0,
            result.main_chunk,
        ));
        let ptr = Box::into_raw(main_func);
        let current_mtime_secs = current_mtime.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
        let now_secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
        self.opcache.insert(file_path.to_string(), OpCacheEntry {
            cached_mtime: std::sync::atomic::AtomicU64::new(current_mtime_secs),
            last_checked_time: std::sync::atomic::AtomicU64::new(now_secs),
            func_ptr: FunctionPtr(ptr),
        });
        Ok(FunctionPtr(ptr))
    }

    pub fn inject_core_classes(self: &Arc<Self>) {
        let classes = &self.classes;
        let mut current_id = classes.len();

        // Inject Constants
        {
            let constants = &self.constants;
            constants.insert(
                "PHP_VERSION_ID".to_string(),
                hyperion_core::memory::nan_box::Value::new_int(80401),
            );

            let php_version_str = Box::new("8.4.1".to_string());
            let php_version_ptr = Box::into_raw(php_version_str) as *mut ();
            constants.insert(
                "PHP_VERSION".to_string(),
                hyperion_core::memory::nan_box::Value::new_string_ptr(php_version_ptr),
            );

            constants.insert(
                "ZEND_THREAD_SAFE".to_string(),
                hyperion_core::memory::nan_box::Value::new_bool(true),
            );

            let php_sapi_str = Box::new("cli".to_string());
            let php_sapi_ptr = Box::into_raw(php_sapi_str) as *mut ();
            constants.insert(
                "PHP_SAPI".to_string(),
                hyperion_core::memory::nan_box::Value::new_string_ptr(php_sapi_ptr),
            );

            let current_exe = std::env::current_exe()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| "php".to_string());
            let php_binary_str = Box::new(current_exe);
            let php_binary_ptr = Box::into_raw(php_binary_str) as *mut ();
            constants.insert(
                "PHP_BINARY".to_string(),
                hyperion_core::memory::nan_box::Value::new_string_ptr(php_binary_ptr),
            );

            let os_name = match std::env::consts::OS {
                "macos" => "Darwin",
                "windows" => "WINNT",
                "linux" => "Linux",
                other => other,
            };
            let php_os_str = Box::new(os_name.to_string());
            let php_os_ptr = Box::into_raw(php_os_str) as *mut ();
            constants.insert(
                "PHP_OS".to_string(),
                hyperion_core::memory::nan_box::Value::new_string_ptr(php_os_ptr),
            );

            let current_exe_buf = std::env::current_exe().unwrap_or_default();
            let bindir = current_exe_buf.parent().unwrap_or(std::path::Path::new("")).to_string_lossy().to_string();
            let php_bindir_str = Box::new(bindir);
            let php_bindir_ptr = Box::into_raw(php_bindir_str) as *mut ();
            constants.insert(
                "PHP_BINDIR".to_string(),
                hyperion_core::memory::nan_box::Value::new_string_ptr(php_bindir_ptr),
            );

            let shlib_suffix = match std::env::consts::OS {
                "macos" => "dylib",
                "windows" => "dll",
                _ => "so",
            };
            let php_shlib_str = Box::new(shlib_suffix.to_string());
            let php_shlib_ptr = Box::into_raw(php_shlib_str) as *mut ();
            constants.insert(
                "PHP_SHLIB_SUFFIX".to_string(),
                hyperion_core::memory::nan_box::Value::new_string_ptr(php_shlib_ptr),
            );

            let dir_sep_str = Box::new("/".to_string());
            let dir_sep_ptr = Box::into_raw(dir_sep_str) as *mut ();
            constants.insert(
                "DIRECTORY_SEPARATOR".to_string(),
                hyperion_core::memory::nan_box::Value::new_string_ptr(dir_sep_ptr),
            );

            let eol_str = Box::new("\n".to_string());
            let eol_ptr = Box::into_raw(eol_str) as *mut ();
            constants.insert(
                "PHP_EOL".to_string(),
                hyperion_core::memory::nan_box::Value::new_string_ptr(eol_ptr),
            );

            let os_family = match std::env::consts::OS {
                "windows" => "Windows",
                "macos" => "Darwin",
                "linux" => "Linux",
                _ => "Unknown",
            };
            let php_os_family_str = Box::new(os_family.to_string());
            let php_os_family_ptr = Box::into_raw(php_os_family_str) as *mut ();
            constants.insert(
                "PHP_OS_FAMILY".to_string(),
                hyperion_core::memory::nan_box::Value::new_string_ptr(php_os_family_ptr),
            );

            constants.insert(
                "PHP_MAXPATHLEN".to_string(),
                hyperion_core::memory::nan_box::Value::new_int(4096),
            );

            // URL component constants
            constants.insert("PHP_URL_SCHEME".to_string(), hyperion_core::memory::nan_box::Value::new_int(0));
            constants.insert("PHP_URL_HOST".to_string(), hyperion_core::memory::nan_box::Value::new_int(1));
            constants.insert("PHP_URL_PORT".to_string(), hyperion_core::memory::nan_box::Value::new_int(2));
            constants.insert("PHP_URL_USER".to_string(), hyperion_core::memory::nan_box::Value::new_int(3));
            constants.insert("PHP_URL_PASS".to_string(), hyperion_core::memory::nan_box::Value::new_int(4));
            constants.insert("PHP_URL_PATH".to_string(), hyperion_core::memory::nan_box::Value::new_int(5));
            constants.insert("PHP_URL_QUERY".to_string(), hyperion_core::memory::nan_box::Value::new_int(6));
            constants.insert("PHP_URL_FRAGMENT".to_string(), hyperion_core::memory::nan_box::Value::new_int(7));

            constants.insert("PHP_MAXPATHLEN".to_string(), hyperion_core::memory::nan_box::Value::new_int(4096));
            constants.insert("PHP_INT_MAX".to_string(), hyperion_core::memory::nan_box::Value::new_int(i32::MAX));
            constants.insert("PHP_INT_MIN".to_string(), hyperion_core::memory::nan_box::Value::new_int(i32::MIN));
            constants.insert("PHP_INT_SIZE".to_string(), hyperion_core::memory::nan_box::Value::new_int(8));

            // Filter flags
            constants.insert("FILTER_DEFAULT".to_string(), hyperion_core::memory::nan_box::Value::new_int(516));
            constants.insert("FILTER_UNSAFE_RAW".to_string(), hyperion_core::memory::nan_box::Value::new_int(516));
            constants.insert("FILTER_VALIDATE_INT".to_string(), hyperion_core::memory::nan_box::Value::new_int(257));
            constants.insert("FILTER_VALIDATE_BOOLEAN".to_string(), hyperion_core::memory::nan_box::Value::new_int(258));
            constants.insert("FILTER_VALIDATE_BOOL".to_string(), hyperion_core::memory::nan_box::Value::new_int(258));
            constants.insert("FILTER_VALIDATE_FLOAT".to_string(), hyperion_core::memory::nan_box::Value::new_int(259));
            constants.insert("FILTER_VALIDATE_REGEXP".to_string(), hyperion_core::memory::nan_box::Value::new_int(272));
            constants.insert("FILTER_VALIDATE_URL".to_string(), hyperion_core::memory::nan_box::Value::new_int(273));
            constants.insert("FILTER_VALIDATE_EMAIL".to_string(), hyperion_core::memory::nan_box::Value::new_int(274));
            constants.insert("FILTER_VALIDATE_IP".to_string(), hyperion_core::memory::nan_box::Value::new_int(275));
            constants.insert("FILTER_VALIDATE_MAC".to_string(), hyperion_core::memory::nan_box::Value::new_int(276));
            constants.insert("FILTER_VALIDATE_DOMAIN".to_string(), hyperion_core::memory::nan_box::Value::new_int(277));
            constants.insert("FILTER_CALLBACK".to_string(), hyperion_core::memory::nan_box::Value::new_int(1024));
            constants.insert("FILTER_FLAG_IPV4".to_string(), hyperion_core::memory::nan_box::Value::new_int(1048576));
            constants.insert("FILTER_FLAG_IPV6".to_string(), hyperion_core::memory::nan_box::Value::new_int(2097152));
            constants.insert("FILTER_NULL_ON_FAILURE".to_string(), hyperion_core::memory::nan_box::Value::new_int(134217728));
            constants.insert("FILTER_REQUIRE_ARRAY".to_string(), hyperion_core::memory::nan_box::Value::new_int(16777216));
            constants.insert("FILTER_REQUIRE_SCALAR".to_string(), hyperion_core::memory::nan_box::Value::new_int(33554432));
            constants.insert("AF_INET".to_string(), hyperion_core::memory::nan_box::Value::new_int(2));
            let af_inet6 = match std::env::consts::OS {
                "macos" => 30,
                _ => 10,
            };
            constants.insert("AF_INET6".to_string(), hyperion_core::memory::nan_box::Value::new_int(af_inet6));

            // Fileinfo constants
            constants.insert("FILEINFO_NONE".to_string(), hyperion_core::memory::nan_box::Value::new_int(0));
            constants.insert("FILEINFO_SYMLINK".to_string(), hyperion_core::memory::nan_box::Value::new_int(2));
            constants.insert("FILEINFO_MIME_TYPE".to_string(), hyperion_core::memory::nan_box::Value::new_int(16));
            constants.insert("FILEINFO_CONTINUE".to_string(), hyperion_core::memory::nan_box::Value::new_int(32));
            constants.insert("FILEINFO_PRESERVE_ATIME".to_string(), hyperion_core::memory::nan_box::Value::new_int(128));
            constants.insert("FILEINFO_RAW".to_string(), hyperion_core::memory::nan_box::Value::new_int(256));
            constants.insert("FILEINFO_MIME_ENCODING".to_string(), hyperion_core::memory::nan_box::Value::new_int(1024));
            constants.insert("FILEINFO_MIME".to_string(), hyperion_core::memory::nan_box::Value::new_int(1040));
            constants.insert("FILEINFO_EXTENSION".to_string(), hyperion_core::memory::nan_box::Value::new_int(2097152));

            // Output handler constants
            constants.insert("PHP_OUTPUT_HANDLER_START".to_string(), hyperion_core::memory::nan_box::Value::new_int(1));
            constants.insert("PHP_OUTPUT_HANDLER_WRITE".to_string(), hyperion_core::memory::nan_box::Value::new_int(0));
            constants.insert("PHP_OUTPUT_HANDLER_FLUSH".to_string(), hyperion_core::memory::nan_box::Value::new_int(4));
            constants.insert("PHP_OUTPUT_HANDLER_CLEAN".to_string(), hyperion_core::memory::nan_box::Value::new_int(2));
            constants.insert("PHP_OUTPUT_HANDLER_FINAL".to_string(), hyperion_core::memory::nan_box::Value::new_int(8));
            constants.insert("PHP_OUTPUT_HANDLER_CONT".to_string(), hyperion_core::memory::nan_box::Value::new_int(0));
            constants.insert("PHP_OUTPUT_HANDLER_END".to_string(), hyperion_core::memory::nan_box::Value::new_int(8));
            constants.insert("PHP_OUTPUT_HANDLER_CLEANABLE".to_string(), hyperion_core::memory::nan_box::Value::new_int(16));
            constants.insert("PHP_OUTPUT_HANDLER_FLUSHABLE".to_string(), hyperion_core::memory::nan_box::Value::new_int(32));
            constants.insert("PHP_OUTPUT_HANDLER_REMOVABLE".to_string(), hyperion_core::memory::nan_box::Value::new_int(64));
            constants.insert("PHP_OUTPUT_HANDLER_STDFLAGS".to_string(), hyperion_core::memory::nan_box::Value::new_int(112));
            constants.insert("PHP_OUTPUT_HANDLER_STARTED".to_string(), hyperion_core::memory::nan_box::Value::new_int(4096));
            constants.insert("PHP_OUTPUT_HANDLER_DISABLED".to_string(), hyperion_core::memory::nan_box::Value::new_int(8192));

            // Tokenizer constants
            for (name, val) in [
                ("T_LNUMBER", 260),
                ("T_DNUMBER", 261),
                ("T_STRING", 262),
                ("T_NAME_FULLY_QUALIFIED", 263),
                ("T_NAME_RELATIVE", 264),
                ("T_NAME_QUALIFIED", 265),
                ("T_VARIABLE", 266),
                ("T_INLINE_HTML", 267),
                ("T_ENCAPSED_AND_WHITESPACE", 268),
                ("T_CONSTANT_ENCAPSED_STRING", 269),
                ("T_STRING_VARNAME", 270),
                ("T_NUM_STRING", 271),
                ("T_INCLUDE", 272),
                ("T_INCLUDE_ONCE", 273),
                ("T_EVAL", 274),
                ("T_REQUIRE", 275),
                ("T_REQUIRE_ONCE", 276),
                ("T_LOGICAL_OR", 277),
                ("T_LOGICAL_XOR", 278),
                ("T_LOGICAL_AND", 279),
                ("T_PRINT", 280),
                ("T_YIELD", 281),
                ("T_YIELD_FROM", 282),
                ("T_INSTANCEOF", 283),
                ("T_NEW", 284),
                ("T_CLONE", 285),
                ("T_EXIT", 286),
                ("T_IF", 287),
                ("T_ELSEIF", 288),
                ("T_ELSE", 289),
                ("T_ENDIF", 290),
                ("T_ECHO", 291),
                ("T_DO", 292),
                ("T_WHILE", 293),
                ("T_ENDWHILE", 294),
                ("T_FOR", 295),
                ("T_ENDFOR", 296),
                ("T_FOREACH", 297),
                ("T_ENDFOREACH", 298),
                ("T_DECLARE", 299),
                ("T_ENDDECLARE", 300),
                ("T_AS", 301),
                ("T_SWITCH", 302),
                ("T_ENDSWITCH", 303),
                ("T_CASE", 304),
                ("T_DEFAULT", 305),
                ("T_MATCH", 306),
                ("T_BREAK", 307),
                ("T_CONTINUE", 308),
                ("T_GOTO", 309),
                ("T_FUNCTION", 310),
                ("T_FN", 311),
                ("T_CONST", 312),
                ("T_RETURN", 313),
                ("T_TRY", 314),
                ("T_CATCH", 315),
                ("T_FINALLY", 316),
                ("T_THROW", 317),
                ("T_USE", 318),
                ("T_INSTEADOF", 319),
                ("T_GLOBAL", 320),
                ("T_STATIC", 321),
                ("T_ABSTRACT", 322),
                ("T_FINAL", 323),
                ("T_PRIVATE", 324),
                ("T_PROTECTED", 325),
                ("T_PUBLIC", 326),
                ("T_PRIVATE_SET", 327),
                ("T_PROTECTED_SET", 328),
                ("T_PUBLIC_SET", 329),
                ("T_READONLY", 330),
                ("T_VAR", 331),
                ("T_UNSET", 332),
                ("T_ISSET", 333),
                ("T_EMPTY", 334),
                ("T_HALT_COMPILER", 335),
                ("T_CLASS", 336),
                ("T_TRAIT", 337),
                ("T_INTERFACE", 338),
                ("T_ENUM", 339),
                ("T_EXTENDS", 340),
                ("T_IMPLEMENTS", 341),
                ("T_NAMESPACE", 342),
                ("T_LIST", 343),
                ("T_ARRAY", 344),
                ("T_CALLABLE", 345),
                ("T_LINE", 346),
                ("T_FILE", 347),
                ("T_DIR", 348),
                ("T_CLASS_C", 349),
                ("T_TRAIT_C", 350),
                ("T_METHOD_C", 351),
                ("T_FUNC_C", 352),
                ("T_PROPERTY_C", 353),
                ("T_NS_C", 354),
                ("T_ATTRIBUTE", 355),
                ("T_PLUS_EQUAL", 356),
                ("T_MINUS_EQUAL", 357),
                ("T_MUL_EQUAL", 358),
                ("T_DIV_EQUAL", 359),
                ("T_CONCAT_EQUAL", 360),
                ("T_MOD_EQUAL", 361),
                ("T_AND_EQUAL", 362),
                ("T_OR_EQUAL", 363),
                ("T_XOR_EQUAL", 364),
                ("T_SL_EQUAL", 365),
                ("T_SR_EQUAL", 366),
                ("T_COALESCE_EQUAL", 367),
                ("T_BOOLEAN_OR", 368),
                ("T_BOOLEAN_AND", 369),
                ("T_IS_EQUAL", 370),
                ("T_IS_NOT_EQUAL", 371),
                ("T_IS_IDENTICAL", 372),
                ("T_IS_NOT_IDENTICAL", 373),
                ("T_IS_SMALLER_OR_EQUAL", 374),
                ("T_IS_GREATER_OR_EQUAL", 375),
                ("T_SPACESHIP", 376),
                ("T_SL", 377),
                ("T_SR", 378),
                ("T_INC", 379),
                ("T_DEC", 380),
                ("T_INT_CAST", 381),
                ("T_DOUBLE_CAST", 382),
                ("T_STRING_CAST", 383),
                ("T_ARRAY_CAST", 384),
                ("T_OBJECT_CAST", 385),
                ("T_BOOL_CAST", 386),
                ("T_UNSET_CAST", 387),
                ("T_OBJECT_OPERATOR", 388),
                ("T_NULLSAFE_OBJECT_OPERATOR", 389),
                ("T_DOUBLE_ARROW", 390),
                ("T_COMMENT", 391),
                ("T_DOC_COMMENT", 392),
                ("T_OPEN_TAG", 393),
                ("T_OPEN_TAG_WITH_ECHO", 394),
                ("T_CLOSE_TAG", 395),
                ("T_WHITESPACE", 396),
                ("T_START_HEREDOC", 397),
                ("T_END_HEREDOC", 398),
                ("T_DOLLAR_OPEN_CURLY_BRACES", 399),
                ("T_CURLY_OPEN", 400),
                ("T_PAAMAYIM_NEKUDOTAYIM", 401),
                ("T_DOUBLE_COLON", 401),
                ("T_NS_SEPARATOR", 402),
                ("T_ELLIPSIS", 403),
                ("T_COALESCE", 404),
                ("T_POW", 405),
                ("T_POW_EQUAL", 406),
                ("T_AMPERSAND_FOLLOWED_BY_VAR_OR_VARARG", 407),
                ("T_AMPERSAND_NOT_FOLLOWED_BY_VAR_OR_VARARG", 408),
                ("T_BAD_CHARACTER", 409),
                ("TOKEN_PARSE", 1),
                ("LIBXML_NOERROR", 32),
                ("LIBXML_COMPACT", 65536),
                ("LIBXML_HTML_NODEFDTD", 4),
                ("LIBXML_NOBLANKS", 256),
                ("LIBXML_NOXMLDECL", 2),
                ("LIBXML_HTML_NOIMPLIED", 8192),
                ("LIBXML_NOCDATA", 16384),
                ("XML_ELEMENT_NODE", 1),
                ("XML_ATTRIBUTE_NODE", 2),
                ("XML_TEXT_NODE", 3),
                ("XML_CDATA_SECTION_NODE", 4),
                ("XML_ENTITY_REF_NODE", 5),
                ("XML_ENTITY_NODE", 6),
                ("XML_PI_NODE", 7),
                ("XML_COMMENT_NODE", 8),
                ("XML_DOCUMENT_NODE", 9),
                ("XML_DOCUMENT_TYPE_NODE", 10),
                ("XML_DOCUMENT_FRAG_NODE", 11),
                ("XML_NOTATION_NODE", 12),
                ("XML_HTML_DOCUMENT_NODE", 13),
            ] {
                constants.insert(name.to_string(), hyperion_core::memory::nan_box::Value::new_int(val));
            }

            // Integer constants, seeded in bulk. Without them
            // `Opcode::FetchConstant` falls back to the bareword as a string, so
            // `array_filter($a, $cb, ARRAY_FILTER_USE_BOTH)` would pass the mode
            // as `"ARRAY_FILTER_USE_BOTH"` and silently select the default mode.
            // Values match upstream PHP 8.4.
            for (name, val) in [
                ("ARRAY_FILTER_USE_KEY", 2),
                ("ARRAY_FILTER_USE_BOTH", 1),
                ("COUNT_NORMAL", 0),
                ("COUNT_RECURSIVE", 1),
                ("SORT_REGULAR", 0),
                ("SORT_NUMERIC", 1),
                ("SORT_STRING", 2),
                ("SORT_DESC", 3),
                ("SORT_ASC", 4),
                ("SORT_LOCALE_STRING", 5),
                ("SORT_NATURAL", 6),
                ("SORT_FLAG_CASE", 8),
                ("STR_PAD_RIGHT", 1),
                ("STR_PAD_LEFT", 0),
                ("STR_PAD_BOTH", 2),
                ("PREG_PATTERN_ORDER", 1),
                ("PREG_SET_ORDER", 2),
                ("PREG_OFFSET_CAPTURE", 256),
                ("PREG_UNMATCHED_AS_NULL", 512),
                ("PREG_SPLIT_NO_EMPTY", 1),
                ("PREG_SPLIT_DELIM_CAPTURE", 2),
                ("PREG_SPLIT_OFFSET_CAPTURE", 4),
                ("PREG_GREP_INVERT", 1),
                ("PREG_NO_ERROR", 0),
                ("PREG_INTERNAL_ERROR", 1),
                ("PREG_BACKTRACK_LIMIT_ERROR", 2),
                ("PREG_RECURSION_LIMIT_ERROR", 3),
                ("PREG_BAD_UTF8_ERROR", 4),
                ("PREG_BAD_UTF8_OFFSET_ERROR", 5),
                ("PREG_JIT_STACKLIMIT_ERROR", 6),
                ("PATHINFO_DIRNAME", 1),
                ("PATHINFO_BASENAME", 2),
                ("PATHINFO_EXTENSION", 4),
                ("PATHINFO_FILENAME", 8),
                ("PATHINFO_ALL", 15),
                ("SCANDIR_SORT_ASCENDING", 0),
                ("SCANDIR_SORT_DESCENDING", 1),
                ("SCANDIR_SORT_NONE", 2),
                ("ENT_QUOTES", 3),
                ("ENT_COMPAT", 2),
                ("ENT_NOQUOTES", 0),
                ("ENT_HTML5", 48),
                ("ENT_XML1", 16),
                ("ENT_SUBSTITUTE", 8),
                ("JSON_HEX_TAG", 1),
                ("JSON_HEX_AMP", 2),
                ("JSON_HEX_APOS", 4),
                ("JSON_HEX_QUOT", 8),
                ("JSON_FORCE_OBJECT", 16),
                ("JSON_NUMERIC_CHECK", 32),
                ("JSON_UNESCAPED_SLASHES", 64),
                ("JSON_PRETTY_PRINT", 128),
                ("JSON_UNESCAPED_UNICODE", 256),
                ("JSON_PARTIAL_OUTPUT_ON_ERROR", 512),
                ("JSON_PRESERVE_ZERO_FRACTION", 1024),
                ("JSON_INVALID_UTF8_IGNORE", 1048576),
                ("JSON_INVALID_UTF8_SUBSTITUTE", 2097152),
                ("JSON_THROW_ON_ERROR", 4194304),
                ("JSON_ERROR_NONE", 0),
                ("JSON_OBJECT_AS_ARRAY", 1),
                ("JSON_BIGINT_AS_STRING", 2),
                ("E_ERROR", 1),
                ("E_WARNING", 2),
                ("E_PARSE", 4),
                ("E_NOTICE", 8),
                ("E_CORE_ERROR", 16),
                ("E_CORE_WARNING", 32),
                ("E_COMPILE_ERROR", 64),
                ("E_COMPILE_WARNING", 128),
                ("E_USER_ERROR", 256),
                ("E_USER_WARNING", 512),
                ("E_USER_NOTICE", 1024),
                ("E_STRICT", 2048),
                ("E_RECOVERABLE_ERROR", 4096),
                ("E_DEPRECATED", 8192),
                ("E_USER_DEPRECATED", 16384),
                ("E_ALL", 32767),
                ("PHP_ROUND_HALF_UP", 1),
                ("PHP_ROUND_HALF_DOWN", 2),
                ("PHP_ROUND_HALF_EVEN", 3),
                ("PHP_ROUND_HALF_ODD", 4),
                ("LC_ALL", 6),
                ("LC_NUMERIC", 4),
                ("SEEK_SET", 0),
                ("SEEK_CUR", 1),
                ("SEEK_END", 2),
                ("LOCK_SH", 1),
                ("LOCK_EX", 2),
                ("LOCK_UN", 3),
                ("PATHINFO_DIRNAME", 1),
                ("PATHINFO_BASENAME", 2),
                ("PATHINFO_EXTENSION", 4),
                ("PATHINFO_FILENAME", 8),
                ("PHP_INT_SIZE", 8),
                ("PHP_FLOAT_DIG", 15),
                ("PHP_MAJOR_VERSION", 8),
                ("PHP_MINOR_VERSION", 4),
                ("PHP_RELEASE_VERSION", 1),
                ("PDO::ATTR_AUTOCOMMIT", 0),
                ("PDO::ATTR_TIMEOUT", 2),
                ("PDO::ATTR_ERRMODE", 3),
                ("PDO::ATTR_SERVER_VERSION", 4),
                ("PDO::ATTR_CLIENT_VERSION", 5),
                ("PDO::ATTR_CASE", 8),
                ("PDO::ATTR_CURSOR", 10),
                ("PDO::ATTR_STATEMENT_CLASS", 11),
                ("PDO::ATTR_PERSISTENT", 12),
                ("PDO::ATTR_DRIVER_NAME", 16),
                ("PDO::ATTR_DEFAULT_FETCH_MODE", 19),
                ("PDO::ATTR_EMULATE_PREPARES", 20),
                ("PDO::ERRMODE_SILENT", 0),
                ("PDO::ERRMODE_WARNING", 1),
                ("PDO::ERRMODE_EXCEPTION", 2),
                ("PDO::FETCH_LAZY", 1),
                ("PDO::FETCH_ASSOC", 2),
                ("PDO::FETCH_NUM", 3),
                ("PDO::FETCH_BOTH", 4),
                ("PDO::FETCH_OBJ", 5),
                ("PDO::FETCH_BOUND", 6),
                ("PDO::FETCH_COLUMN", 7),
                ("PDO::FETCH_CLASS", 8),
                ("PDO::FETCH_INTO", 9),
                ("PDO::FETCH_FUNC", 10),
                ("PDO::PARAM_NULL", 0),
                ("PDO::PARAM_INT", 1),
                ("PDO::PARAM_STR", 2),
                ("PDO::PARAM_LOB", 3),
                ("PDO::PARAM_STMT", 4),
                ("PDO::PARAM_BOOL", 5),
                // ReflectionProperty constants
                ("ReflectionProperty::IS_STATIC", 16),
                ("ReflectionProperty::IS_PUBLIC", 1),
                ("ReflectionProperty::IS_PROTECTED", 2),
                ("ReflectionProperty::IS_PRIVATE", 4),
                ("ReflectionProperty::IS_READONLY", 128),
                ("ReflectionProperty::IS_ABSTRACT", 64),
                ("ReflectionProperty::IS_FINAL", 32),
                ("ReflectionProperty::IS_VIRTUAL", 512),
                ("ReflectionProperty::IS_PROTECTED_SET", 2048),
                ("ReflectionProperty::IS_PRIVATE_SET", 4096),
                // ReflectionMethod constants
                ("ReflectionMethod::IS_STATIC", 16),
                ("ReflectionMethod::IS_PUBLIC", 1),
                ("ReflectionMethod::IS_PROTECTED", 2),
                ("ReflectionMethod::IS_PRIVATE", 4),
                ("ReflectionMethod::IS_ABSTRACT", 64),
                ("ReflectionMethod::IS_FINAL", 32),
                // ReflectionClass constants
                ("ReflectionClass::IS_IMPLICIT_ABSTRACT", 16),
                ("ReflectionClass::IS_EXPLICIT_ABSTRACT", 64),
                ("ReflectionClass::IS_FINAL", 32),
                ("ReflectionClass::IS_READONLY", 65536),
                // ReflectionAttribute constants
                ("ReflectionAttribute::IS_INSTANCEOF", 2),
                // ReflectionClassConstant constants
                ("ReflectionClassConstant::IS_PUBLIC", 1),
                ("ReflectionClassConstant::IS_PROTECTED", 2),
                ("ReflectionClassConstant::IS_PRIVATE", 4),
                ("ReflectionClassConstant::IS_FINAL", 32),
                // cURL constants
                ("CURLOPT_URL", 10002),
                ("CURLOPT_PORT", 3),
                ("CURLOPT_TIMEOUT", 13),
                ("CURLOPT_TIMEOUT_MS", 155),
                ("CURLOPT_RETURNTRANSFER", 19913),
                ("CURLOPT_HEADER", 42),
                ("CURLOPT_POST", 47),
                ("CURLOPT_POSTFIELDS", 10015),
                ("CURLOPT_HTTPHEADER", 10023),
                ("CURLOPT_CUSTOMREQUEST", 10036),
                ("CURLOPT_FOLLOWLOCATION", 52),
                ("CURLOPT_MAXREDIRS", 68),
                ("CURLOPT_USERAGENT", 10018),
                ("CURLOPT_REFERER", 10016),
                ("CURLOPT_ENCODING", 10102),
                ("CURLOPT_COOKIE", 10022),
                ("CURLOPT_COOKIEFILE", 10031),
                ("CURLOPT_COOKIEJAR", 10082),
                ("CURLOPT_SSL_VERIFYPEER", 64),
                ("CURLOPT_SSL_VERIFYHOST", 81),
                ("CURLOPT_NOBODY", 44),
                ("CURLOPT_HTTPGET", 80),
                ("CURLINFO_EFFECTIVE_URL", 1048577),
                ("CURLINFO_HTTP_CODE", 2097154),
                ("CURLINFO_RESPONSE_CODE", 2097154),
                ("CURLINFO_HEADER_SIZE", 2097163),
                ("CURLINFO_SIZE_DOWNLOAD", 3145736),
                ("CURLINFO_TOTAL_TIME", 3145731),
                ("CURLINFO_CONTENT_TYPE", 1048594),
                ("CURLINFO_SPEED_DOWNLOAD", 3145737),
                ("CURLE_OK", 0),
                ("CURLM_OK", 0),
                ("CURLM_CALL_MULTI_PERFORM", -1),
                ("CURLMSG_DONE", 1),
                // File upload error constants
                ("UPLOAD_ERR_OK", 0),
                ("UPLOAD_ERR_INI_SIZE", 1),
                ("UPLOAD_ERR_FORM_SIZE", 2),
                ("UPLOAD_ERR_PARTIAL", 3),
                ("UPLOAD_ERR_NO_FILE", 4),
                ("UPLOAD_ERR_NO_TMP_DIR", 6),
                ("UPLOAD_ERR_CANT_WRITE", 7),
                ("UPLOAD_ERR_EXTENSION", 8),
                // ZipArchive constants
                ("ZipArchive::CREATE", 1),
                ("ZipArchive::OVERWRITE", 8),
                ("ZipArchive::EXCL", 2),
                ("ZipArchive::CHECKCONS", 4),
                ("ZipArchive::RDONLY", 16),
                ("ZipArchive::ER_OK", 0),
                ("ZipArchive::ER_NOZIP", 19),
                ("FORCE_GZIP", 31),
                ("FORCE_DEFLATE", 15),
                // Signals
                ("SIGHUP", 1),
                ("SIGINT", 2),
                ("SIGQUIT", 3),
                ("SIGILL", 4),
                ("SIGTRAP", 5),
                ("SIGABRT", 6),
                ("SIGFPE", 8),
                ("SIGKILL", 9),
                ("SIGUSR1", 10),
                ("SIGSEGV", 11),
                ("SIGUSR2", 12),
                ("SIGPIPE", 13),
                ("SIGALRM", 14),
                ("SIGTERM", 15),
                ("SIGCHLD", 17),
                ("SIGCONT", 18),
                ("SIGSTOP", 19),
                ("SIGTSTP", 20),
                ("SIGTTIN", 21),
                ("SIGTTOU", 22),
                ("SIG_DFL", 0),
                ("SIG_IGN", 1),
                // MySQLi constants
                ("MYSQLI_ASSOC", 1),
                ("MYSQLI_NUM", 2),
                ("MYSQLI_BOTH", 3),
                ("MYSQLI_STORE_RESULT", 0),
                ("MYSQLI_USE_RESULT", 1),
                ("MYSQLI_REPORT_OFF", 0),
                ("MYSQLI_REPORT_ERROR", 1),
                ("MYSQLI_REPORT_STRICT", 2),
                ("MYSQLI_REPORT_INDEX", 4),
                ("MYSQLI_REPORT_ALL", 255),
                ("MYSQLI_CLIENT_SSL", 2048),
                ("MYSQLI_CLIENT_COMPRESS", 32),
                ("MYSQLI_CLIENT_INTERACTIVE", 1024),
                ("MYSQLI_CLIENT_IGNORE_SPACE", 256),
                ("MYSQLI_OPT_CONNECT_TIMEOUT", 0),
                ("MYSQLI_INIT_COMMAND", 3),
            ] {
                constants.insert(
                    name.to_string(),
                    hyperion_core::memory::nan_box::Value::new_int(val),
                );
                if let Some((cls, prop)) = name.split_once("::") {
                    let norm = crate::vm::normalize_name(cls);
                    self.default_statics.insert(
                        (norm, prop.to_string()),
                        hyperion_core::memory::nan_box::Value::new_int(val),
                    );
                }
            }

            // The 64-bit integer limits do not fit this engine's 32-bit int tag,
            // so they are seeded as floats. Code that treats them as "a very
            // large bound" still compares correctly; exact bit patterns do not
            // survive, which is the tradeoff the value model forces.
            for (name, val) in [
                ("PHP_INT_MAX", 9223372036854775807f64),
                ("PHP_INT_MIN", -9223372036854775808f64),
                ("PHP_FLOAT_EPSILON", f64::EPSILON),
                ("PHP_FLOAT_MAX", f64::MAX),
                ("PHP_FLOAT_MIN", f64::MIN_POSITIVE),
                ("NAN", f64::NAN),
                ("INF", f64::INFINITY),
                ("M_PI", std::f64::consts::PI),
                ("M_E", std::f64::consts::E),
                ("M_SQRT2", std::f64::consts::SQRT_2),
                ("M_LN2", std::f64::consts::LN_2),
                ("M_LN10", std::f64::consts::LN_10),
                ("M_LOG2E", std::f64::consts::LOG2_E),
                ("M_LOG10E", std::f64::consts::LOG10_E),
                ("M_PI_2", std::f64::consts::FRAC_PI_2),
                ("M_PI_4", std::f64::consts::FRAC_PI_4),
            ] {
                constants.insert(
                    name.to_string(),
                    hyperion_core::memory::nan_box::Value::new_float(val),
                );
            }

            // String constants. Each leaks one `String` for the life of the
            // engine, matching how PHP_VERSION and friends above are seeded.
            let os_family = match std::env::consts::OS {
                "macos" | "ios" => "Darwin",
                "windows" => "Windows",
                "linux" | "android" => "Linux",
                "freebsd" | "openbsd" | "netbsd" | "dragonfly" => "BSD",
                "solaris" => "Solaris",
                _ => "Unknown",
            };
            let path_sep = if std::env::consts::OS == "windows" { ";" } else { ":" };
            for (name, val) in [
                ("PHP_EOL", "\n"),
                ("PATH_SEPARATOR", path_sep),
                ("PHP_OS_FAMILY", os_family),
                ("PHP_EXTRA_VERSION", ""),
                // Callers identify the iconv backend to decide which charset
                // spellings and flags to trust; naming the engine is more
                // honest than claiming glibc's or libiconv's quirks.
                ("ICONV_IMPL", "hyperion"),
                ("ICONV_VERSION", "1.0"),
            ] {
                let boxed = Box::new(val.to_string());
                let ptr = Box::into_raw(boxed) as *mut ();
                constants.insert(
                    name.to_string(),
                    hyperion_core::memory::nan_box::Value::new_string_ptr(ptr),
                );
            }

            // The three standard streams. These are resources, not strings, and
            // they must be created exactly once so that `STDOUT === STDOUT`
            // holds and a handle stored by one call site is the same stream the
            // next one writes to.
            for (name, handle) in hyperion_ext_standard::stream::make_standard_streams() {
                constants.insert(name.to_string(), handle);
            }
        }

        let builtins = vec![
            "stdClass",
            "Generator",
            "UnitEnum",
            "BackedEnum",
            "ReflectionFunction",
            "ReflectionMethod",
            "ReflectionClass",
            "ReflectionObject",
            "ReflectionProperty",
            "ReflectionParameter",
            "ReflectionType",
            "ReflectionNamedType",
            "ReflectionUnionType",
            "ReflectionIntersectionType",
            "ReflectionAttribute",
            "ReflectionReference",
            "ReflectionClassConstant",
            "ReflectionEnum",
            "ReflectionEnumUnitCase",
            "ReflectionEnumBackedCase",
            "ReflectionFiber",
        ];

        for name in builtins {
            classes.insert(current_id, crate::types::class::PhpClass::new(
                current_id,
                name.to_string(),
            ));
            current_id += 1;
        }

        crate::core_classes::register_core_classes(&classes, &mut current_id);

        let stdlib_classes = hyperion_ext_standard::get_stdlib_classes();
        for (class_name, methods) in stdlib_classes {
            // Check if class is already defined (e.g. via builtins or core_classes)
            let existing_id = {
                classes.iter().find(|c| c.value().name.eq_ignore_ascii_case(&class_name)).map(|c| *c.key())
            };
            if let Some(id) = existing_id {
                if let Some(mut existing_class) = classes.get_mut(&id) {
                    for (method_name, arity, func_name) in methods {
                        existing_class.add_native_method(method_name, arity, func_name);
                    }
                }
            } else {
                let mut cls = crate::types::class::PhpClass::new(current_id, class_name);
                for (method_name, arity, func_name) in methods {
                    cls.add_native_method(method_name, arity, func_name);
                }
                classes.insert(cls.id, cls);
                current_id += 1;
            }
        }

        for c in classes.iter() {
            self.class_map.insert(crate::vm::normalize_name(&c.name), c.id);
        }

        self.next_class_id.store(current_id, std::sync::atomic::Ordering::SeqCst);
        self.next_func_id.store(self.functions.len(), std::sync::atomic::Ordering::SeqCst);

        // Drop the classes lock before compiling, since compile_and_load_source acquires it.
        drop(classes);

        // SPL classes that are plain PHP. Compiling is enough to register them:
        // the compiler hands back class definitions and `compile_and_load_source`
        // merges them, so the prelude body never has to run. It is compiled after
        // `register_core_classes` so the interfaces it implements already exist.
        const PRELUDE: &str = include_str!("prelude.php");
        if let Err(e) = self.compile_and_load_source("<prelude>", PRELUDE) {
            eprintln!("Warning: failed to compile the SPL prelude: {}", e);
        }
    }
}

// Fibre must be Send to allow it to be transferred between worker threads via crossbeam-deque.
// We guarantee computationally that a single Fibre is only mutated by one thread at a time.
unsafe impl Send for Fibre {}

struct HttpServerGlobalsCache {
    pub key_argv: usize,
    pub key_argc: usize,
    pub key_script_filename: usize,
    pub key_script_name: usize,
    pub key_php_self: usize,
    pub key_document_root: usize,
    pub key_request_uri: usize,
    pub key_path_info: usize,
    pub key_query_string: usize,
    pub key_request_method: usize,
    pub key_server_name: usize,
    pub key_server_port: usize,
    pub key_server_protocol: usize,
    pub key_remote_addr: usize,
    pub key_host: usize,
    pub key_user_agent: usize,
    pub key_accept: usize,
    pub key_connection: usize,
    pub key_content_type: usize,
    pub key_content_length: usize,
    pub key_cookie: usize,
    pub key_authorization: usize,
    pub key_accept_encoding: usize,
    pub key_accept_language: usize,
}

impl HttpServerGlobalsCache {
    #[inline(always)]
    pub fn resolve_header_key(&self, name: &str) -> usize {
        if name.eq_ignore_ascii_case("host") {
            self.key_host
        } else if name.eq_ignore_ascii_case("user-agent") {
            self.key_user_agent
        } else if name.eq_ignore_ascii_case("accept") {
            self.key_accept
        } else if name.eq_ignore_ascii_case("connection") {
            self.key_connection
        } else if name.eq_ignore_ascii_case("content-type") {
            self.key_content_type
        } else if name.eq_ignore_ascii_case("content-length") {
            self.key_content_length
        } else if name.eq_ignore_ascii_case("cookie") {
            self.key_cookie
        } else if name.eq_ignore_ascii_case("authorization") {
            self.key_authorization
        } else if name.eq_ignore_ascii_case("accept-encoding") {
            self.key_accept_encoding
        } else if name.eq_ignore_ascii_case("accept-language") {
            self.key_accept_language
        } else {
            let name_upper = name.to_ascii_uppercase().replace('-', "_");
            let key = if name_upper == "CONTENT_TYPE" || name_upper == "CONTENT_LENGTH" {
                name_upper
            } else {
                format!("HTTP_{}", name_upper)
            };
            hyperion_core::types::string_table::intern_string(&key)
        }
    }
}

static SERVER_KEY_CACHE: std::sync::OnceLock<HttpServerGlobalsCache> = std::sync::OnceLock::new();

fn get_server_key_cache() -> &'static HttpServerGlobalsCache {
    SERVER_KEY_CACHE.get_or_init(|| {
        use hyperion_core::types::string_table::intern_string;
        HttpServerGlobalsCache {
            key_argv: intern_string("argv"),
            key_argc: intern_string("argc"),
            key_script_filename: intern_string("SCRIPT_FILENAME"),
            key_script_name: intern_string("SCRIPT_NAME"),
            key_php_self: intern_string("PHP_SELF"),
            key_document_root: intern_string("DOCUMENT_ROOT"),
            key_request_uri: intern_string("REQUEST_URI"),
            key_path_info: intern_string("PATH_INFO"),
            key_query_string: intern_string("QUERY_STRING"),
            key_request_method: intern_string("REQUEST_METHOD"),
            key_server_name: intern_string("SERVER_NAME"),
            key_server_port: intern_string("SERVER_PORT"),
            key_server_protocol: intern_string("SERVER_PROTOCOL"),
            key_remote_addr: intern_string("REMOTE_ADDR"),
            key_host: intern_string("HTTP_HOST"),
            key_user_agent: intern_string("HTTP_USER_AGENT"),
            key_accept: intern_string("HTTP_ACCEPT"),
            key_connection: intern_string("HTTP_CONNECTION"),
            key_content_type: intern_string("CONTENT_TYPE"),
            key_content_length: intern_string("CONTENT_LENGTH"),
            key_cookie: intern_string("HTTP_COOKIE"),
            key_authorization: intern_string("HTTP_AUTHORIZATION"),
            key_accept_encoding: intern_string("HTTP_ACCEPT_ENCODING"),
            key_accept_language: intern_string("HTTP_ACCEPT_LANGUAGE"),
        }
    })
}

impl Fibre {

    pub fn new(id: u64, main_function: FunctionPtr, engine_state: Arc<GlobalEngineState>) -> Self {
        // Dummy frame initialized with null ptr, we'll overwrite the first one
        let dummy_frame = CallFrame {
            function: FunctionPtr(std::ptr::null()),
            ip: 0,
            stack_window: 0,
            called_class_id: 0,
            return_override: None,
            eval_parent_stack_window: None,
            arity: 0,
        };

        let mut frames = vec![dummy_frame; FRAMES_MAX];
        frames[0] = CallFrame {
            function: main_function,
            ip: 0,
            stack_window: 0,
            called_class_id: 0,
            return_override: None,
            eval_parent_stack_window: None,
            arity: 0,
        };

        let autoloaders = engine_state.autoloaders.read().unwrap().clone();

        let mut f = Self {
            id,
            state: FibreState::Ready,
            protocol: RequestProtocol::Http,
            frames,
            frame_count: 1,
            stack: vec![Value::null(); STACK_MAX],
            stack_top: 0,
            arena: std::sync::Arc::new(hyperion_core::gc::arena::GcArena::new()),
            output_buffer: Vec::with_capacity(1024),
            ob_buffers: Vec::new(),
            tcp_stream: None,
            trace_log: Vec::new(),
            autoloaders,
            engine_state,
            superglobals: [Value::null(); 8],
            included_files: std::collections::HashSet::new(),
            included_file_returns: HashMap::new(),
            frame_included_paths: HashMap::new(),
            autoload_state: HashMap::new(),


            static_properties: std::sync::Mutex::new(HashMap::new()),
            function_statics: std::sync::Mutex::new(HashMap::new()),
            autoload_frame_depths: Vec::new(),
            generator_key: Value::null(),
            generator_value: Value::null(),
            generator_yield_counter: 0,
            generator_started: false,
            unwind_action: None,
            generator_valid: true,
            is_generator: false,
            is_keep_alive_idle: false,
            http_parse_buffer: Vec::new(),
            stop_frame_count: None,
            tracer: None,
            local_heat_map: HashMap::new(),
            jit_deopt_count: HashMap::new(),
            headers_sent: false,
            response_status_code: 200,
            response_headers: Vec::new(),
            dynamic_locals: HashMap::new(),
            frame_args: vec![Vec::new(); 1024],
            magic_guards: std::collections::HashSet::new(),
            http_request_method: "GET".to_string(),
            http_request_uri: "/".to_string(),
            http_keep_alive: true,
            http_raw_body: Vec::new(),
            entry_func_ptr: main_function,
            reactor_id: 0,
            boot_checkpoint: None,
            boot_static_properties: None,
            boot_function_statics: None,
            boot_stack_top: 0,
            boot_stack_snapshot: None,
            boot_frame_count: 0,
            boot_call_frame: None,
            boot_autoloaders: None,
            boot_included_files: None,
            warm_request_count: 0,
        };

        // Initialize all 8 superglobals as empty arrays in the local GC arena
        for i in 0..8 {
            let arr = hyperion_core::types::array::PhpArray::new();
            let arr_ref = f.arena.alloc_and_track(arr);
            f.superglobals[i] = Value::new_array_ptr(arr_ref as *mut () as *mut _);
        }

        f
    }

    #[inline(always)]
    pub fn clone_default_property(arena: &hyperion_core::gc::arena::GcArena, val: &Value) -> Value {
        let val_deref = val.deref();
        if let Some(arr_ptr) = val_deref.as_array_ptr() {
            let old_arr = unsafe { &*(arr_ptr as *const hyperion_core::types::array::PhpArray) };
            let mut new_arr = hyperion_core::types::array::PhpArray::new();
            for (k, v) in old_arr.elements.iter() {
                let cloned_v = Self::clone_default_property(arena, v);
                match k {
                    hyperion_core::types::array::ArrayKey::Int(i) => new_arr.insert_int(*i, cloned_v),
                    hyperion_core::types::array::ArrayKey::StringId(s) => new_arr.insert_string_id(*s, cloned_v),
                }
            }
            let new_arr_ptr = arena.alloc_and_track(new_arr);
            Value::new_array_ptr(new_arr_ptr as *mut ())
        } else if let Some(str_ptr) = val_deref.as_string_ptr() {
            let s = unsafe { &*(str_ptr as *const String) };
            let new_str_ptr = arena.alloc_and_track(s.clone());
            Value::new_string_ptr(new_str_ptr as *mut ())
        } else {
            *val
        }
    }

    pub fn clone_value_to_global(engine_state: &Arc<GlobalEngineState>, val: &Value) -> Value {
        Self::clone_value_to_global_depth(engine_state, val, 0)
    }

    fn clone_value_to_global_depth(engine_state: &Arc<GlobalEngineState>, val: &Value, depth: usize) -> Value {
        if depth > 10 {
            return *val;
        }
        let val_deref = val.deref();
        if let Some(arr_ptr) = val_deref.as_array_ptr() {
            let old_arr = unsafe { &*(arr_ptr as *const hyperion_core::types::array::PhpArray) };
            let mut new_arr = hyperion_core::types::array::PhpArray::new();
            for (k, v) in old_arr.elements.iter() {
                let cloned_v = Self::clone_value_to_global_depth(engine_state, v, depth + 1);
                match k {
                    hyperion_core::types::array::ArrayKey::Int(i) => new_arr.insert_int(*i, cloned_v),
                    hyperion_core::types::array::ArrayKey::StringId(s) => new_arr.insert_string_id(*s, cloned_v),
                }
            }
            let new_arr_ptr = engine_state.global_arena.alloc_and_track(new_arr);
            Value::new_array_ptr(new_arr_ptr as *mut ())
        } else if let Some(str_ptr) = val_deref.as_string_ptr() {
            let s = unsafe { &*(str_ptr as *const String) };
            let new_str_ptr = engine_state.global_arena.alloc_and_track(s.clone());
            Value::new_string_ptr(new_str_ptr as *mut ())
        } else if let Some(obj_ptr) = val_deref.as_object_ptr() {
            let old_obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
            let mut new_obj = hyperion_core::types::object::PhpObject::new(old_obj.class_id);
            new_obj.class_name = old_obj.class_name.clone();
            for (k, v) in old_obj.properties.iter() {
                let cloned_v = Self::clone_value_to_global_depth(engine_state, v, depth + 1);
                new_obj.properties.insert(k.clone(), cloned_v);
            }
            let new_obj_ptr = engine_state.global_arena.alloc_and_track(new_obj);
            Value::new_object_ptr(new_obj_ptr as *mut ())
        } else if let Some(closure_ptr) = val_deref.as_closure_ptr() {
            let old_closure = unsafe { &*(closure_ptr as *const crate::types::closure::PhpClosure) };
            let upvalues = old_closure.upvalues.iter().map(|v| Self::clone_value_to_global_depth(engine_state, v, depth + 1)).collect();
            let this_val = old_closure.this_val.map(|v| Self::clone_value_to_global_depth(engine_state, &v, depth + 1));
            let new_closure = crate::types::closure::PhpClosure::new(old_closure.function_ptr, upvalues, old_closure.called_class_id, this_val);
            let new_cl_ptr = engine_state.global_arena.alloc_and_track(new_closure);
            Value::new_closure_ptr(new_cl_ptr as *mut ())
        } else {
            *val
        }
    }

    pub fn clone_value_to_fibre(&self, val: &Value) -> Value {
        Self::clone_value_to_fibre_depth(&self.arena, val, 0)
    }

    fn clone_value_to_fibre_depth(arena: &Arc<hyperion_core::gc::arena::GcArena>, val: &Value, depth: usize) -> Value {
        if depth > 10 {
            return *val;
        }
        let val_deref = val.deref();
        if let Some(arr_ptr) = val_deref.as_array_ptr() {
            let old_arr = unsafe { &*(arr_ptr as *const hyperion_core::types::array::PhpArray) };
            let mut new_arr = hyperion_core::types::array::PhpArray::new();
            for (k, v) in old_arr.elements.iter() {
                let cloned_v = Self::clone_value_to_fibre_depth(arena, v, depth + 1);
                match k {
                    hyperion_core::types::array::ArrayKey::Int(i) => new_arr.insert_int(*i, cloned_v),
                    hyperion_core::types::array::ArrayKey::StringId(s) => new_arr.insert_string_id(*s, cloned_v),
                }
            }
            let new_arr_ptr = arena.alloc_and_track(new_arr);
            Value::new_array_ptr(new_arr_ptr as *mut ())
        } else if let Some(obj_ptr) = val_deref.as_object_ptr() {
            let old_obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
            let mut new_obj = hyperion_core::types::object::PhpObject::new(old_obj.class_id);
            new_obj.class_name = old_obj.class_name.clone();
            for (k, v) in old_obj.properties.iter() {
                let cloned_v = Self::clone_value_to_fibre_depth(arena, v, depth + 1);
                new_obj.properties.insert(k.clone(), cloned_v);
            }
            let new_obj_ptr = arena.alloc_and_track(new_obj);
            Value::new_object_ptr(new_obj_ptr as *mut ())
        } else {
            *val
        }
    }

    #[inline(always)]
    pub fn suspend(&mut self) {

        self.state = FibreState::Suspended;
    }

    #[inline(always)]
    pub fn resume(&mut self) {
        self.state = FibreState::Ready;
    }

    #[inline(always)]
    pub fn kill(&mut self) {
        self.state = FibreState::Dead;
    }

    pub fn clear(&mut self, main_function: FunctionPtr) {
        self.state = FibreState::Ready;
        self.protocol = RequestProtocol::Http;
        self.frame_count = 1;
        self.frames[0] = CallFrame {
            function: main_function,
            ip: 0,
            stack_window: 0,
            called_class_id: 0,
            return_override: None,
            eval_parent_stack_window: None,
            arity: 0,
        };
        for f in &mut self.frames[1..] {
            *f = CallFrame {
                function: FunctionPtr(std::ptr::null_mut()),
                ip: 0,
                stack_window: 0,
                called_class_id: 0,
                return_override: None,
                eval_parent_stack_window: None,
                arity: 0,
            };
        }
        let clean_top = self.stack_top.min(self.stack.len());
        self.stack[..clean_top].fill(Value::null());
        self.stack_top = 0;
        self.arena.reset();
        self.output_buffer.clear();
        self.ob_buffers.clear();
        self.trace_log.clear();
        self.http_raw_body.clear();
        
        self.included_files.clear();
        self.autoload_state.clear();
        self.static_properties.lock().unwrap().clear();
        self.function_statics.lock().unwrap().clear();
        self.autoload_frame_depths.clear();
        self.autoloaders.clear();
        self.generator_key = Value::null();
        self.generator_value = Value::null();
        self.generator_yield_counter = 0;
        self.generator_started = false;
        self.unwind_action = None;
        self.generator_valid = true;
        self.is_generator = false;
        self.stop_frame_count = None;
        self.tracer = None;
        self.local_heat_map.clear();
        self.jit_deopt_count.clear();
        self.headers_sent = false;
        self.response_status_code = 200;
        self.response_headers.clear();
        self.dynamic_locals.clear();
        for args in &mut self.frame_args {
            args.clear();
        }

        self.http_request_method = "GET".to_string();
        self.http_request_uri = "/".to_string();

        for i in 0..8 {
            let arr = hyperion_core::types::array::PhpArray::new();
            let arr_ref = self.arena.alloc_and_track(arr);
            self.superglobals[i] = Value::new_array_ptr(arr_ref as *mut () as *mut _);
        }
    }

    pub fn reset_for_worker_request(&mut self, main_function: FunctionPtr) {
        self.frame_count = 1;
        self.frames[0] = CallFrame {
            function: main_function,
            ip: 0,
            stack_window: 0,
            called_class_id: 0,
            return_override: None,
            eval_parent_stack_window: None,
            arity: 0,
        };
        let used_stack = self.stack_top.min(self.stack.len());
        self.stack[..used_stack].fill(Value::null());
        self.stack_top = 0;
        self.output_buffer.clear();



        self.ob_buffers.clear();
        self.trace_log.clear();
        self.http_raw_body.clear();
        self.headers_sent = false;
        self.response_status_code = 200;
        self.response_headers.clear();
        self.generator_key = Value::null();
        self.generator_value = Value::null();
        self.generator_yield_counter = 0;
        self.generator_started = false;
        self.unwind_action = None;
        self.generator_valid = true;
        self.is_generator = false;
        self.stop_frame_count = None;
        self.tracer = None;
        self.dynamic_locals.clear();
        for args in &mut self.frame_args {
            args.clear();
        }
        self.http_request_method = "GET".to_string();
        self.http_request_uri = "/".to_string();
        self.included_files.clear();
        self.included_file_returns.clear();
        self.frame_included_paths.clear();
        self.autoload_state.clear();
        self.autoload_frame_depths.clear();

        if let Some(cp) = self.boot_checkpoint {
            self.arena.reset_to_checkpoint(cp);
            if let Some(ref boot_statics) = self.boot_static_properties {
                *self.static_properties.lock().unwrap() = boot_statics.clone();
            }
            if let Some(ref boot_fn_statics) = self.boot_function_statics {
                *self.function_statics.lock().unwrap() = boot_fn_statics.clone();
            }
        } else {
            self.arena.reset();
            self.static_properties.lock().unwrap().clear();
            self.function_statics.lock().unwrap().clear();
        }

        self.entry_func_ptr = main_function;
        self.state = FibreState::Ready;
        self.autoloaders = self.engine_state.autoloaders.read().unwrap().clone();

        for i in 0..8 {
            let arr = hyperion_core::types::array::PhpArray::new();
            let arr_ref = self.arena.alloc_and_track(arr);
            self.superglobals[i] = Value::new_array_ptr(arr_ref as *mut () as *mut _);
        }
    }

    #[inline(always)]
    pub fn mark_boot_checkpoint(&mut self) {
        self.boot_checkpoint = Some(self.arena.create_checkpoint());
        self.boot_static_properties = Some(self.static_properties.lock().unwrap().clone());
        self.boot_function_statics = Some(self.function_statics.lock().unwrap().clone());
        self.boot_stack_top = self.stack_top;
        self.boot_stack_snapshot = Some(self.stack[..self.stack_top].to_vec());
        self.boot_frame_count = self.frame_count;
        self.boot_call_frame = self.frames.first().copied();
        self.boot_autoloaders = Some(self.autoloaders.clone());
        self.boot_included_files = Some(self.included_files.clone());
        for i in 0..8 {
            let arr = hyperion_core::types::array::PhpArray::new();
            let arr_ref = self.arena.alloc_and_track(arr);
            self.superglobals[i] = Value::new_array_ptr(arr_ref as *mut () as *mut _);
        }
    }

    #[inline(always)]
    pub fn clone_for_worker(&self, new_id: u64) -> Self {
        let mut f = self.clone();
        f.id = new_id;
        f
    }

    #[inline(always)]
    pub fn reset_to_boot_checkpoint(&mut self) {
        if let Some(cp) = self.boot_checkpoint {
            self.arena.reset_to_checkpoint(cp);
            if let Some(ref boot_statics) = self.boot_static_properties {
                *self.static_properties.lock().unwrap() = boot_statics.clone();
            }
            if let Some(ref boot_fn_statics) = self.boot_function_statics {
                *self.function_statics.lock().unwrap() = boot_fn_statics.clone();
            }
            if let Some(ref boot_stack) = self.boot_stack_snapshot {
                self.stack[..boot_stack.len()].copy_from_slice(boot_stack);
                for slot in self.stack[boot_stack.len()..self.stack_top].iter_mut() {
                    *slot = Value::null();
                }
                self.stack_top = self.boot_stack_top;
                self.frame_count = self.boot_frame_count;
            }
            if let Some(frame) = self.boot_call_frame {
                self.frames[0] = frame;
            }
            self.output_buffer.clear();
            self.ob_buffers.clear();
            self.trace_log.clear();
            self.http_raw_body.clear();
            self.headers_sent = false;
            self.response_status_code = 200;
            self.response_headers.clear();
            self.autoload_state.clear();
            self.autoload_frame_depths.clear();
            self.dynamic_locals.clear();
            for args in &mut self.frame_args {
                args.clear();
            }
            self.magic_guards.clear();
            self.included_file_returns.clear();
            self.frame_included_paths.clear();
            if let Some(ref boot_loaders) = self.boot_autoloaders {
                self.autoloaders = boot_loaders.clone();
            }
            if let Some(ref boot_inc) = self.boot_included_files {
                self.included_files = boot_inc.clone();
            }
            for i in 0..8 {
                let arr = hyperion_core::types::array::PhpArray::new();
                let arr_ref = self.arena.alloc_and_track(arr);
                self.superglobals[i] = Value::new_array_ptr(arr_ref as *mut () as *mut _);
            }
        }
    }

    #[inline(always)]
    pub fn collect_garbage(&mut self) -> usize {
        let from_idx = self.boot_checkpoint.map(|cp| cp.drop_list_len).unwrap_or(0);
        let curr_len = self.arena.drop_list_len();
        if curr_len <= from_idx {
            return 0;
        }

        static INIT_HOOK: std::sync::Once = std::sync::Once::new();
        INIT_HOOK.call_once(|| {
            fn visit_closure_upvalues(ptr: *mut (), callback: &mut dyn FnMut(&Value)) {
                let cl = unsafe { &*(ptr as *const crate::types::closure::PhpClosure) };
                for v in &cl.upvalues {
                    callback(v);
                }
                if let Some(ref tv) = cl.this_val {
                    callback(tv);
                }
            }
            hyperion_core::gc::collector::set_closure_visitor(visit_closure_upvalues);
        });

        let static_vals: Vec<Value> = self.static_properties.lock().unwrap().values().cloned().collect();
        let fn_static_vals: Vec<Value> = self.function_statics.lock().unwrap().values().cloned().collect();
        let inc_returns: Vec<Value> = self.included_file_returns.values().cloned().collect();
        let dyn_locals: Vec<Value> = self.dynamic_locals.values().flat_map(|m| m.values().cloned()).collect();
        let frame_rets: Vec<Value> = self.frames[..self.frame_count].iter().filter_map(|f| f.return_override).collect();
        let frame_arg_vals: Vec<Value> = self.frame_args.iter().flatten().cloned().collect();
        let boot_statics: Vec<Value> = self.boot_static_properties.as_ref().map(|m| m.values().cloned().collect()).unwrap_or_default();
        let boot_fn_statics: Vec<Value> = self.boot_function_statics.as_ref().map(|m| m.values().cloned().collect()).unwrap_or_default();
        let boot_stack: &[Value] = self.boot_stack_snapshot.as_deref().unwrap_or(&[]);
        let constants: Vec<Value> = self.engine_state.constants.iter().map(|kv| *kv.value()).collect();
        let default_statics: Vec<Value> = self.engine_state.default_statics.iter().map(|kv| *kv.value()).collect();

        let roots = [
            &self.stack[..self.stack_top],
            boot_stack,
            &static_vals[..],
            &fn_static_vals[..],
            &boot_statics[..],
            &boot_fn_statics[..],
            &self.superglobals[..],
            &inc_returns[..],
            &dyn_locals[..],
            &frame_rets[..],
            &frame_arg_vals[..],
            &constants[..],
            &default_statics[..],
            &self.autoloaders[..],
            &[self.generator_key, self.generator_value][..],
        ];
        let from_idx = self.boot_checkpoint.map(|cp| cp.drop_list_len).unwrap_or(0);
        hyperion_core::gc::collector::GcCollector::collect(&self.arena, from_idx, &roots)
    }


    pub fn flush_all_ob_buffers(&mut self) {
        while let Some(buf) = self.ob_buffers.pop() {
            if let Some(parent) = self.ob_buffers.last_mut() {
                parent.extend_from_slice(&buf);
            } else {
                self.output_buffer.extend_from_slice(&buf);
            }
        }
    }

    #[inline(always)]
    pub fn set_frame_args(&mut self, frame_idx: usize, args: &[Value]) {
        if self.frame_args.len() <= frame_idx {
            self.frame_args.resize(frame_idx + 16, Vec::new());
        }
        self.frame_args[frame_idx].clear();
        self.frame_args[frame_idx].extend_from_slice(args);
    }

    #[inline(always)]
    pub fn set_frame_args_from_stack(&mut self, frame_idx: usize, start: usize, end: usize) {
        if self.frame_args.len() <= frame_idx {
            self.frame_args.resize(frame_idx + 16, Vec::new());
        }
        self.frame_args[frame_idx].clear();
        self.frame_args[frame_idx].extend_from_slice(&self.stack[start..end]);
    }



    pub fn populate_cli_superglobals(&mut self, args: &[String]) {
        if let Some(server_ptr) = self.superglobals[0].as_array_ptr() {
            let server_arr =
                unsafe { &mut *(server_ptr as *mut hyperion_core::types::array::PhpArray) };

            // 1. argc
            let argc = args.len();
            server_arr.insert_string_id(
                hyperion_core::types::string_table::intern_string("argc"),
                Value::new_int(argc as i32),
            );

            // 2. argv
            let mut argv_arr = hyperion_core::types::array::PhpArray::new();
            for (i, arg) in args.iter().enumerate() {
                let arg_val = self.arena.alloc_and_track(arg.clone());
                argv_arr.insert_int(i as i64, Value::new_string_ptr(arg_val as *mut ()));
            }
            let argv_ptr = self.arena.alloc_and_track(argv_arr);
            server_arr.insert_string_id(
                hyperion_core::types::string_table::intern_string("argv"),
                Value::new_array_ptr(argv_ptr as *mut ()),
            );

            // 3. SCRIPT_FILENAME, SCRIPT_NAME, PHP_SELF
            if let Some(script_path) = args.first() {
                let sf_ptr = self.arena.alloc_and_track(script_path.clone());
                server_arr.insert_string_id(
                    hyperion_core::types::string_table::intern_string("SCRIPT_FILENAME"),
                    Value::new_string_ptr(sf_ptr as *mut ()),
                );

                let sn_ptr = self.arena.alloc_and_track(script_path.clone());
                server_arr.insert_string_id(
                    hyperion_core::types::string_table::intern_string("SCRIPT_NAME"),
                    Value::new_string_ptr(sn_ptr as *mut ()),
                );

                let ps_ptr = self.arena.alloc_and_track(script_path.clone());
                server_arr.insert_string_id(
                    hyperion_core::types::string_table::intern_string("PHP_SELF"),
                    Value::new_string_ptr(ps_ptr as *mut ()),
                );
            }

            if let Ok(cwd) = std::env::current_dir() {
                let pwd_str = cwd.to_string_lossy().to_string();
                let pwd_ptr = self.arena.alloc_and_track(pwd_str);
                server_arr.insert_string_id(
                    hyperion_core::types::string_table::intern_string("PWD"),
                    Value::new_string_ptr(pwd_ptr as *mut ()),
                );
            }
        }

        // Also populate $_ENV
        if let Some(env_ptr) = self.superglobals[5].as_array_ptr() {
            let env_arr = unsafe { &mut *(env_ptr as *mut hyperion_core::types::array::PhpArray) };
            for (key, val) in std::env::vars() {
                let val_ptr = self.arena.alloc_and_track(val);
                env_arr.insert_string_id(hyperion_core::types::string_table::intern_string(&key), Value::new_string_ptr(val_ptr as *mut ()));
            }
        }

        // Also populate $GLOBALS['argv'] and $GLOBALS['argc']
        if let Some(globals_ptr) = self.superglobals[6].as_array_ptr() {
            let globals_arr = unsafe { &mut *(globals_ptr as *mut hyperion_core::types::array::PhpArray) };
            let argc = args.len();
            globals_arr.insert_string_id(hyperion_core::types::string_table::intern_string("argc"), Value::new_int(argc as i32));
            if let Some(server_ptr) = self.superglobals[0].as_array_ptr() {
                let server_arr = unsafe { &*(server_ptr as *const hyperion_core::types::array::PhpArray) };
                if let Some(argv_val) = server_arr.get_string_id(hyperion_core::types::string_table::intern_string("argv")) {
                    globals_arr.insert_string_id(hyperion_core::types::string_table::intern_string("argv"), *argv_val);
                }
            }
        }
    }

    pub fn populate_http_superglobals(&mut self, req: &httparse::Request, body: &[u8], script_path: &str) {

        let keys = get_server_key_cache();
        self.http_raw_body = body.to_vec();
        // 1. Populate $_SERVER
        if let Some(server_ptr) = self.superglobals[0].as_array_ptr() {
            let server_arr =
                unsafe { &mut *(server_ptr as *mut hyperion_core::types::array::PhpArray) };
            server_arr.elements.clear();

            if !body.is_empty() {
                let body_str = String::from_utf8_lossy(body).into_owned();
                let body_ptr = self.arena.alloc_and_track(body_str);
                server_arr.insert_string_id(hyperion_core::types::string_table::intern_string("HTTP_RAW_POST_DATA"), Value::new_string_ptr(body_ptr as *mut ()));
            }

            // Default empty argv/argc
            let argv_arr = hyperion_core::types::array::PhpArray::new();
            let argv_ptr = self.arena.alloc_and_track(argv_arr);
            server_arr.insert_string_id(keys.key_argv, Value::new_array_ptr(argv_ptr as *mut ()));
            server_arr.insert_string_id(keys.key_argc, Value::new_int(0));

            static SCRIPT_INFO: std::sync::OnceLock<(String, String)> = std::sync::OnceLock::new();
            let (script_filename, doc_root) = SCRIPT_INFO.get_or_init(|| {
                let sfn = std::path::Path::new(script_path)
                    .canonicalize()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_else(|_| script_path.to_string());
                let dr = std::path::Path::new(&sfn)
                    .parent()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_else(|| ".".to_string());
                (sfn, dr)
            });

            let sfn_ptr = self.arena.alloc_and_track(script_filename.clone());
            server_arr.insert_string_id(keys.key_script_filename, Value::new_string_ptr(sfn_ptr as *mut ()));

            let sn_ptr = self.arena.alloc_and_track("/index.php".to_string());
            server_arr.insert_string_id(keys.key_script_name, Value::new_string_ptr(sn_ptr as *mut ()));

            let ps_ptr = self.arena.alloc_and_track("/index.php".to_string());
            server_arr.insert_string_id(keys.key_php_self, Value::new_string_ptr(ps_ptr as *mut ()));

            let dr_ptr = self.arena.alloc_and_track(doc_root.clone());
            server_arr.insert_string_id(keys.key_document_root, Value::new_string_ptr(dr_ptr as *mut ()));

            // Request URI & Method
            if let Some(path) = req.path {
                let mut request_uri = path;
                let mut query_string = "";
                if let Some(pos) = path.find('?') {
                    request_uri = &path[..pos];
                    query_string = &path[pos + 1..];
                }

                self.http_request_uri = path.to_string();
                let ru_ptr = self.arena.alloc_and_track(path.to_string());
                server_arr.insert_string_id(keys.key_request_uri, Value::new_string_ptr(ru_ptr as *mut ()));

                let pi_ptr = self.arena.alloc_and_track(request_uri.to_string());
                server_arr.insert_string_id(keys.key_path_info, Value::new_string_ptr(pi_ptr as *mut ()));

                let qs_ptr = self.arena.alloc_and_track(query_string.to_string());
                server_arr.insert_string_id(keys.key_query_string, Value::new_string_ptr(qs_ptr as *mut ()));
            }

            if let Some(method) = req.method {
                self.http_request_method = method.to_string();
                let rm_ptr = self.arena.alloc_and_track(method.to_string());
                server_arr.insert_string_id(keys.key_request_method, Value::new_string_ptr(rm_ptr as *mut ()));
            }


            let version = req.version.unwrap_or(1);
            let mut keep_alive = version == 1;

            // Headers to $_SERVER (HTTP_*)
            for header in req.headers.iter() {
                if header.name.eq_ignore_ascii_case("connection") {
                    if header.value.eq_ignore_ascii_case(b"close") {
                        keep_alive = false;
                    } else if header.value.eq_ignore_ascii_case(b"keep-alive") {
                        keep_alive = true;
                    }
                }
                let key_id = keys.resolve_header_key(header.name);
                let value_str = String::from_utf8_lossy(header.value).into_owned();
                let val_ptr = self.arena.alloc_and_track(value_str);
                server_arr.insert_string_id(key_id, Value::new_string_ptr(val_ptr as *mut ()));
            }

            self.http_keep_alive = keep_alive;

            // Other server properties
            let sn_ptr = self.arena.alloc_and_track("localhost".to_string());
            server_arr.insert_string_id(keys.key_server_name, Value::new_string_ptr(sn_ptr as *mut ()));

            let sp_ptr = self.arena.alloc_and_track("8000".to_string());
            server_arr.insert_string_id(keys.key_server_port, Value::new_string_ptr(sp_ptr as *mut ()));

            let spro_ptr = self.arena.alloc_and_track("HTTP/1.1".to_string());
            server_arr.insert_string_id(keys.key_server_protocol, Value::new_string_ptr(spro_ptr as *mut ()));

            let ra_ptr = self.arena.alloc_and_track("127.0.0.1".to_string());
            server_arr.insert_string_id(keys.key_remote_addr, Value::new_string_ptr(ra_ptr as *mut ()));
        }


        // Helper for URL decoding
        let url_decode = Self::url_decode;

        // 2. Populate $_GET
        let mut get_arr = hyperion_core::types::array::PhpArray::new();
        if let Some(path) = req.path {
            if let Some(pos) = path.find('?') {
                let query = &path[pos + 1..];
                for part in query.split('&') {
                    let mut kv = part.splitn(2, '=');
                    if let Some(k) = kv.next() {
                        let v = kv.next().unwrap_or("");
                        let k_dec = url_decode(k.trim());
                        let v_dec = url_decode(v.trim());
                        let v_ptr = self.arena.alloc_and_track(v_dec);
                        get_arr.insert_string_id(hyperion_core::types::string_table::intern_string(&k_dec), Value::new_string_ptr(v_ptr as *mut ()));
                    }
                }
            }
        }
        if let Some(get_ptr) = self.superglobals[1].as_array_ptr() {
            unsafe { *(get_ptr as *mut hyperion_core::types::array::PhpArray) = get_arr };
        }

        // 3. Populate $_POST and $_FILES
        let mut post_arr = hyperion_core::types::array::PhpArray::new();
        let mut files_arr = hyperion_core::types::array::PhpArray::new();

        let content_type = req.headers.iter()
            .find(|h| h.name.eq_ignore_ascii_case("content-type"))
            .map(|h| String::from_utf8_lossy(h.value).into_owned())
            .unwrap_or_default();

        if req.method == Some("POST") && !body.is_empty() {
            if content_type.to_ascii_lowercase().starts_with("multipart/form-data") {
                Self::parse_multipart_data(&self.arena, &content_type, body, &mut post_arr, &mut files_arr);
            } else {
                let body_str = String::from_utf8_lossy(body);
                for part in body_str.split('&') {
                    let mut kv = part.splitn(2, '=');
                    if let Some(k) = kv.next() {
                        let v = kv.next().unwrap_or("");
                        let k_dec = url_decode(k.trim());
                        let v_dec = url_decode(v.trim());
                        let v_ptr = self.arena.alloc_and_track(v_dec);
                        post_arr.insert_string_id(hyperion_core::types::string_table::intern_string(&k_dec), Value::new_string_ptr(v_ptr as *mut ()));
                    }
                }
            }
        }
        if let Some(post_ptr) = self.superglobals[2].as_array_ptr() {
            unsafe { *(post_ptr as *mut hyperion_core::types::array::PhpArray) = post_arr };
        }
        if let Some(files_ptr) = self.superglobals[4].as_array_ptr() {
            unsafe { *(files_ptr as *mut hyperion_core::types::array::PhpArray) = files_arr };
        }

        // 4. Populate $_COOKIE
        let mut cookie_arr = hyperion_core::types::array::PhpArray::new();
        for header in req.headers.iter() {
            if header.name.eq_ignore_ascii_case("cookie") {
                let val_str = String::from_utf8_lossy(header.value);
                for cookie_pair in val_str.split(';') {
                    let mut kv = cookie_pair.trim().splitn(2, '=');
                    if let Some(k) = kv.next() {
                        if let Some(v) = kv.next() {
                            let k_dec = url_decode(k.trim());
                            let v_dec = url_decode(v.trim());
                            let v_ptr = self.arena.alloc_and_track(v_dec);
                            cookie_arr
                                .insert_string_id(hyperion_core::types::string_table::intern_string(&k_dec), Value::new_string_ptr(v_ptr as *mut ()));
                        }
                    }
                }
            }
        }
        if let Some(cookie_ptr) = self.superglobals[3].as_array_ptr() {
            unsafe { *(cookie_ptr as *mut hyperion_core::types::array::PhpArray) = cookie_arr };
        }

        // 5. Populate $_REQUEST (merge of $_GET, $_POST, and $_COOKIE)
        let mut req_arr = hyperion_core::types::array::PhpArray::new();
        if let Some(get_ptr) = self.superglobals[1].as_array_ptr() {
            let g = unsafe { &*(get_ptr as *const hyperion_core::types::array::PhpArray) };
            for (k, v) in &g.elements {
                match k {
                    hyperion_core::types::array::ArrayKey::Int(i) => req_arr.insert_int(*i, *v),
                    hyperion_core::types::array::ArrayKey::StringId(s) => {
                        req_arr.insert_string_id(*s, *v)
                    }
                }
            }
        }
        if let Some(post_ptr) = self.superglobals[2].as_array_ptr() {
            let p = unsafe { &*(post_ptr as *const hyperion_core::types::array::PhpArray) };
            for (k, v) in &p.elements {
                match k {
                    hyperion_core::types::array::ArrayKey::Int(i) => req_arr.insert_int(*i, *v),
                    hyperion_core::types::array::ArrayKey::StringId(s) => {
                        req_arr.insert_string_id(*s, *v)
                    }
                }
            }
        }
        if let Some(cookie_ptr) = self.superglobals[3].as_array_ptr() {
            let c = unsafe { &*(cookie_ptr as *const hyperion_core::types::array::PhpArray) };
            for (k, v) in &c.elements {
                match k {
                    hyperion_core::types::array::ArrayKey::Int(i) => req_arr.insert_int(*i, *v),
                    hyperion_core::types::array::ArrayKey::StringId(s) => {
                        req_arr.insert_string_id(*s, *v)
                    }
                }
            }
        }
        if let Some(req_ptr) = self.superglobals[6].as_array_ptr() {
            unsafe { *(req_ptr as *mut hyperion_core::types::array::PhpArray) = req_arr };
        }

        // 6. Populate $_ENV
        if let Some(env_ptr) = self.superglobals[5].as_array_ptr() {
            let env_arr = unsafe { &mut *(env_ptr as *mut hyperion_core::types::array::PhpArray) };
            for (key, val) in std::env::vars() {
                let val_ptr = self.arena.alloc_and_track(val);
                env_arr.insert_string_id(hyperion_core::types::string_table::intern_string(&key), Value::new_string_ptr(val_ptr as *mut ()));
            }
        }
    }

    pub fn parse_multipart_data(
        arena: &hyperion_core::gc::arena::GcArena,
        content_type_header: &str,
        body: &[u8],
        post_arr: &mut hyperion_core::types::array::PhpArray,
        files_arr: &mut hyperion_core::types::array::PhpArray,
    ) {
        let Some(boundary_idx) = content_type_header.to_ascii_lowercase().find("boundary=") else {
            return;
        };
        let mut boundary = content_type_header[boundary_idx + "boundary=".len()..].trim();
        if boundary.starts_with('"') && boundary.ends_with('"') && boundary.len() >= 2 {
            boundary = &boundary[1..boundary.len() - 1];
        }
        let delimiter = format!("--{}", boundary).into_bytes();
        let delimiter_len = delimiter.len();

        let mut start = 0;
        while let Some(pos) = Self::find_subsequence(&body[start..], &delimiter) {
            let part_start = start + pos + delimiter_len;
            let part_end = match Self::find_subsequence(&body[part_start..], &delimiter) {
                Some(next_pos) => part_start + next_pos,
                None => break,
            };

            let part_bytes = &body[part_start..part_end];
            let part_bytes = if part_bytes.ends_with(b"\r\n") {
                &part_bytes[..part_bytes.len() - 2]
            } else if part_bytes.ends_with(b"\n") {
                &part_bytes[..part_bytes.len() - 1]
            } else {
                part_bytes
            };

            if let Some(header_end) = Self::find_subsequence(part_bytes, b"\r\n\r\n") {
                let header_raw = String::from_utf8_lossy(&part_bytes[..header_end]);
                let content_bytes = &part_bytes[header_end + 4..];

                let mut field_name = None;
                let mut file_name = None;
                let mut part_content_type = "application/octet-stream".to_string();

                for line in header_raw.lines() {
                    let line = line.trim();
                    if line.to_ascii_lowercase().starts_with("content-disposition:") {
                        for param in line["content-disposition:".len()..].split(';') {
                            let param = param.trim();
                            if let Some((k, v)) = param.split_once('=') {
                                let k = k.trim().to_ascii_lowercase();
                                let mut v = v.trim();
                                if v.starts_with('"') && v.ends_with('"') && v.len() >= 2 {
                                    v = &v[1..v.len() - 1];
                                }
                                if k == "name" {
                                    field_name = Some(v.to_string());
                                } else if k == "filename" {
                                    file_name = Some(v.to_string());
                                }
                            }
                        }
                    } else if line.to_ascii_lowercase().starts_with("content-type:") {
                        part_content_type = line["content-type:".len()..].trim().to_string();
                    }
                }

                if let Some(field) = field_name {
                    let is_array_field = field.contains('[') && field.ends_with(']');
                    let clean_field = if is_array_field {
                        &field[..field.find('[').unwrap()]
                    } else {
                        &field
                    };
                    let clean_field_id = hyperion_core::types::string_table::intern_string(clean_field);

                    if let Some(fname) = file_name {
                        if fname.is_empty() {
                            let mut file_obj = hyperion_core::types::array::PhpArray::new();
                            let empty_str = arena.alloc_and_track(String::new());
                            let empty_val = Value::new_string_ptr(empty_str as *mut ());
                            file_obj.insert_string_id(hyperion_core::types::string_table::intern_string("name"), empty_val);
                            file_obj.insert_string_id(hyperion_core::types::string_table::intern_string("full_path"), empty_val);
                            file_obj.insert_string_id(hyperion_core::types::string_table::intern_string("type"), empty_val);
                            file_obj.insert_string_id(hyperion_core::types::string_table::intern_string("tmp_name"), empty_val);
                            file_obj.insert_string_id(hyperion_core::types::string_table::intern_string("error"), Value::new_int(4)); // UPLOAD_ERR_NO_FILE
                            file_obj.insert_string_id(hyperion_core::types::string_table::intern_string("size"), Value::new_int(0));

                            let file_obj_ptr = arena.alloc_and_track(file_obj);
                            if is_array_field {
                                let target_array_ptr = match files_arr.get_string_id(clean_field_id) {
                                    Some(v) if v.is_array() => v.as_array_ptr().unwrap() as *mut hyperion_core::types::array::PhpArray,
                                    _ => {
                                        let new_arr = hyperion_core::types::array::PhpArray::new();
                                        let new_arr_ptr = arena.alloc_and_track(new_arr);
                                        files_arr.insert_string_id(clean_field_id, Value::new_array_ptr(new_arr_ptr as *mut ()));
                                        new_arr_ptr as *mut hyperion_core::types::array::PhpArray
                                    }
                                };
                                unsafe {
                                    (*target_array_ptr).push(Value::new_array_ptr(file_obj_ptr as *mut ()));
                                }
                            } else {
                                files_arr.insert_string_id(clean_field_id, Value::new_array_ptr(file_obj_ptr as *mut ()));
                            }
                        } else {
                            static UPLOAD_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
                            let uid = UPLOAD_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            let tmp_filename = format!("php_upld_{}_{}", uid, fname.replace('/', "_").replace('\\', "_"));
                            let tmp_path = std::env::temp_dir().join(&tmp_filename);
                            let _ = std::fs::write(&tmp_path, content_bytes);

                            let mut file_obj = hyperion_core::types::array::PhpArray::new();
                            let name_ptr = arena.alloc_and_track(fname.clone());
                            file_obj.insert_string_id(hyperion_core::types::string_table::intern_string("name"), Value::new_string_ptr(name_ptr as *mut ()));

                            let full_path_ptr = arena.alloc_and_track(fname);
                            file_obj.insert_string_id(hyperion_core::types::string_table::intern_string("full_path"), Value::new_string_ptr(full_path_ptr as *mut ()));

                            let type_ptr = arena.alloc_and_track(part_content_type);
                            file_obj.insert_string_id(hyperion_core::types::string_table::intern_string("type"), Value::new_string_ptr(type_ptr as *mut ()));

                            let tmp_str = tmp_path.to_string_lossy().into_owned();
                            let tmp_ptr = arena.alloc_and_track(tmp_str);
                            file_obj.insert_string_id(hyperion_core::types::string_table::intern_string("tmp_name"), Value::new_string_ptr(tmp_ptr as *mut ()));

                            file_obj.insert_string_id(hyperion_core::types::string_table::intern_string("error"), Value::new_int(0));
                            file_obj.insert_string_id(hyperion_core::types::string_table::intern_string("size"), Value::new_int(content_bytes.len() as i32));

                            let file_obj_ptr = arena.alloc_and_track(file_obj);
                            if is_array_field {
                                let target_array_ptr = match files_arr.get_string_id(clean_field_id) {
                                    Some(v) if v.is_array() => v.as_array_ptr().unwrap() as *mut hyperion_core::types::array::PhpArray,
                                    _ => {
                                        let new_arr = hyperion_core::types::array::PhpArray::new();
                                        let new_arr_ptr = arena.alloc_and_track(new_arr);
                                        files_arr.insert_string_id(clean_field_id, Value::new_array_ptr(new_arr_ptr as *mut ()));
                                        new_arr_ptr as *mut hyperion_core::types::array::PhpArray
                                    }
                                };
                                unsafe {
                                    (*target_array_ptr).push(Value::new_array_ptr(file_obj_ptr as *mut ()));
                                }
                            } else {
                                files_arr.insert_string_id(clean_field_id, Value::new_array_ptr(file_obj_ptr as *mut ()));
                            }
                        }
                    } else {
                        let val_str = String::from_utf8_lossy(content_bytes).into_owned();
                        let val_ptr = arena.alloc_and_track(val_str);
                        if is_array_field {
                            let target_array_ptr = match post_arr.get_string_id(clean_field_id) {
                                Some(v) if v.is_array() => v.as_array_ptr().unwrap() as *mut hyperion_core::types::array::PhpArray,
                                _ => {
                                    let new_arr = hyperion_core::types::array::PhpArray::new();
                                    let new_arr_ptr = arena.alloc_and_track(new_arr);
                                    post_arr.insert_string_id(clean_field_id, Value::new_array_ptr(new_arr_ptr as *mut ()));
                                    new_arr_ptr as *mut hyperion_core::types::array::PhpArray
                                }
                            };
                            unsafe {
                                (*target_array_ptr).push(Value::new_string_ptr(val_ptr as *mut ()));
                            }
                        } else {
                            post_arr.insert_string_id(clean_field_id, Value::new_string_ptr(val_ptr as *mut ()));
                        }
                    }
                }
            }

            start = part_start;
        }
    }

    fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack.windows(needle.len()).position(|window| window == needle)
    }

    pub fn url_decode(s: &str) -> String {
        let mut decoded = String::new();
        let mut chars = s.chars();
        while let Some(ch) = chars.next() {
            if ch == '%' {
                let mut hex = String::new();
                if let Some(h1) = chars.next() {
                    hex.push(h1);
                }
                if let Some(h2) = chars.next() {
                    hex.push(h2);
                }
                if let Ok(val) = u8::from_str_radix(&hex, 16) {
                    decoded.push(val as char);
                }
            } else if ch == '+' {
                decoded.push(' ');
            } else {
                decoded.push(ch);
            }
        }
        decoded
    }

    pub fn populate_fastcgi_superglobals(
        &mut self,
        params: &HashMap<String, String>,
        stdin_data: &[u8],
    ) {
        // 1. Populate $_SERVER
        if let Some(server_ptr) = self.superglobals[0].as_array_ptr() {
            let server_arr =
                unsafe { &mut *(server_ptr as *mut hyperion_core::types::array::PhpArray) };
            for (key, val) in params {
                let val_ptr = self.arena.alloc_and_track(val.clone());
                server_arr.insert_string_id(hyperion_core::types::string_table::intern_string(&key.clone()), Value::new_string_ptr(val_ptr as *mut ()));
            }

            // Default empty argv/argc if not provided by FastCGI params
            if server_arr.get_string_id(hyperion_core::types::string_table::intern_string("argv")).is_none() {
                let argv_arr = hyperion_core::types::array::PhpArray::new();
                let argv_ptr = self.arena.alloc_and_track(argv_arr);
                server_arr.insert_string_id(hyperion_core::types::string_table::intern_string("argv"),
                    Value::new_array_ptr(argv_ptr as *mut ()),
                );
                server_arr.insert_string_id(hyperion_core::types::string_table::intern_string("argc"), Value::new_int(0));
            }
        }

        // 2. Populate $_GET
        let mut get_arr = hyperion_core::types::array::PhpArray::new();
        if let Some(query) = params.get("QUERY_STRING") {
            for part in query.split('&') {
                let mut kv = part.splitn(2, '=');
                if let Some(k) = kv.next() {
                    let v = kv.next().unwrap_or("");
                    let k_dec = Self::url_decode(k.trim());
                    let v_dec = Self::url_decode(v.trim());
                    let v_ptr = self.arena.alloc_and_track(v_dec);
                    get_arr.insert_string_id(hyperion_core::types::string_table::intern_string(&k_dec), Value::new_string_ptr(v_ptr as *mut ()));
                }
            }
        }
        if let Some(get_ptr) = self.superglobals[1].as_array_ptr() {
            unsafe { *(get_ptr as *mut hyperion_core::types::array::PhpArray) = get_arr };
        }

        // 3. Populate $_POST
        let mut post_arr = hyperion_core::types::array::PhpArray::new();
        if params.get("REQUEST_METHOD").map(|s| s.as_str()) == Some("POST")
            && !stdin_data.is_empty()
        {
            let body_str = String::from_utf8_lossy(stdin_data);
            for part in body_str.split('&') {
                let mut kv = part.splitn(2, '=');
                if let Some(k) = kv.next() {
                    let v = kv.next().unwrap_or("");
                    let k_dec = Self::url_decode(k.trim());
                    let v_dec = Self::url_decode(v.trim());
                    let v_ptr = self.arena.alloc_and_track(v_dec);
                    post_arr.insert_string_id(hyperion_core::types::string_table::intern_string(&k_dec), Value::new_string_ptr(v_ptr as *mut ()));
                }
            }
        }
        if let Some(post_ptr) = self.superglobals[2].as_array_ptr() {
            unsafe { *(post_ptr as *mut hyperion_core::types::array::PhpArray) = post_arr };
        }

        // 4. Populate $_COOKIE
        let mut cookie_arr = hyperion_core::types::array::PhpArray::new();
        if let Some(val_str) = params.get("HTTP_COOKIE") {
            for cookie_pair in val_str.split(';') {
                let mut kv = cookie_pair.trim().splitn(2, '=');
                if let Some(k) = kv.next() {
                    if let Some(v) = kv.next() {
                        let k_dec = Self::url_decode(k.trim());
                        let v_dec = Self::url_decode(v.trim());
                        let v_ptr = self.arena.alloc_and_track(v_dec);
                        cookie_arr.insert_string_id(hyperion_core::types::string_table::intern_string(&k_dec), Value::new_string_ptr(v_ptr as *mut ()));
                    }
                }
            }
        }
        if let Some(cookie_ptr) = self.superglobals[3].as_array_ptr() {
            unsafe { *(cookie_ptr as *mut hyperion_core::types::array::PhpArray) = cookie_arr };
        }

        // 5. Populate $_REQUEST (merge of $_GET, $_POST, and $_COOKIE)
        let mut req_arr = hyperion_core::types::array::PhpArray::new();
        if let Some(get_ptr) = self.superglobals[1].as_array_ptr() {
            let g = unsafe { &*(get_ptr as *const hyperion_core::types::array::PhpArray) };
            for (k, v) in &g.elements {
                match k {
                    hyperion_core::types::array::ArrayKey::Int(i) => req_arr.insert_int(*i, *v),
                    hyperion_core::types::array::ArrayKey::StringId(s) => {
                        req_arr.insert_string_id(*s, *v)
                    }
                }
            }
        }
        if let Some(post_ptr) = self.superglobals[2].as_array_ptr() {
            let p = unsafe { &*(post_ptr as *const hyperion_core::types::array::PhpArray) };
            for (k, v) in &p.elements {
                match k {
                    hyperion_core::types::array::ArrayKey::Int(i) => req_arr.insert_int(*i, *v),
                    hyperion_core::types::array::ArrayKey::StringId(s) => {
                        req_arr.insert_string_id(*s, *v)
                    }
                }
            }
        }
        if let Some(cookie_ptr) = self.superglobals[3].as_array_ptr() {
            let c = unsafe { &*(cookie_ptr as *const hyperion_core::types::array::PhpArray) };
            for (k, v) in &c.elements {
                match k {
                    hyperion_core::types::array::ArrayKey::Int(i) => req_arr.insert_int(*i, *v),
                    hyperion_core::types::array::ArrayKey::StringId(s) => {
                        req_arr.insert_string_id(*s, *v)
                    }
                }
            }
        }
        if let Some(req_ptr) = self.superglobals[6].as_array_ptr() {
            unsafe { *(req_ptr as *mut hyperion_core::types::array::PhpArray) = req_arr };
        }

        // 6. Populate $_ENV
        if let Some(env_ptr) = self.superglobals[5].as_array_ptr() {
            let env_arr = unsafe { &mut *(env_ptr as *mut hyperion_core::types::array::PhpArray) };
            for (key, val) in std::env::vars() {
                let val_ptr = self.arena.alloc_and_track(val);
                env_arr.insert_string_id(hyperion_core::types::string_table::intern_string(&key), Value::new_string_ptr(val_ptr as *mut ()));
            }
        }
    }
}

pub fn find_class_in_slice(engine_state: &GlobalEngineState, classes: &dashmap::DashMap<usize, PhpClass>, name: &str) -> Option<PhpClass> {
    let norm = crate::vm::normalize_name(name);
    if let Some(id) = engine_state.class_map.get(&norm).map(|v| *v) {
        classes.get(&id).map(|v| v.clone())
    } else {
        classes.iter().find(|c| crate::vm::normalize_name(&c.value().name) == norm).map(|c| c.value().clone())
    }
}

impl hyperion_core::types::function::NativeContext for Fibre {
    fn get_arena(&mut self) -> &hyperion_core::gc::arena::GcArena {
        &self.arena
    }

    fn get_backtrace(&self) -> Vec<hyperion_core::types::function::BacktraceFrame> {
        let mut frames = Vec::new();
        for f in self.frames[0..self.frame_count].iter().rev() {
            let func = unsafe { &*f.function.0 };
            let (file, line, function, class, frame_type, object) = if func.name.starts_with("closure#") {
                let parts: Vec<&str> = func.name.split('#').collect();
                let file = parts.get(1).unwrap_or(&"").to_string();
                let line = parts.get(2).and_then(|l| l.parse::<usize>().ok()).unwrap_or(1);
                let this_val = self.stack.get(f.stack_window).map(|v| *v);
                let obj = this_val.filter(|v| v.is_object());
                (file, line, "{closure}".to_string(), None, None, obj)
            } else if func.name.contains("::") {
                let parts: Vec<&str> = func.name.split("::").collect();
                let class_name = parts[0].to_string();
                let method_name = parts[1].to_string();
                let this_val = self.stack.get(f.stack_window).map(|v| *v);
                let is_obj = this_val.map(|v| v.is_object()).unwrap_or(false);
                let f_type = if is_obj { "->" } else { "::" };
                let obj = if is_obj { this_val } else { None };
                (String::new(), 1, method_name, Some(class_name), Some(f_type.to_string()), obj)
            } else {
                let this_val = self.stack.get(f.stack_window).map(|v| *v);
                let obj = this_val.filter(|v| v.is_object());
                (String::new(), 1, func.name.clone(), None, None, obj)
            };
            frames.push(hyperion_core::types::function::BacktraceFrame {
                file,
                line,
                function,
                class,
                frame_type,
                object,
            });
        }
        frames
    }

    fn call_callable_synchronously(
        &mut self,
        callable: hyperion_core::memory::nan_box::Value,
        args: Vec<hyperion_core::memory::nan_box::Value>,
    ) -> Result<hyperion_core::memory::nan_box::Value, String> {
        crate::vm::VM::call_callable_synchronously(self, callable, args)
    }

    fn call_method_synchronously(
        &mut self,
        obj: hyperion_core::memory::nan_box::Value,
        method: &str,
        args: Vec<hyperion_core::memory::nan_box::Value>,
    ) -> Result<hyperion_core::memory::nan_box::Value, String> {
        crate::vm::VM::call_method_synchronously(self, obj, method, args)
    }

    fn get_class_name(&self, class_id: usize) -> Option<String> {
        let classes = &self.engine_state.classes;
        classes.get(&class_id).map(|c| c.name.clone())
    }

    fn get_class_id(&self, class_name: &str) -> Option<usize> {
        let norm = crate::vm::normalize_name(class_name);
        self.engine_state.class_map.get(&norm).map(|v| *v)
    }

    fn class_exists(&self, name: &str) -> bool {
        let norm = crate::vm::normalize_name(name);
        self.engine_state.class_map.contains_key(&norm)
    }

    fn interface_exists(&self, name: &str) -> bool {
        self.is_interface(name)
    }

    fn trait_exists(&self, name: &str) -> bool {
        self.is_trait(name)
    }

    fn enum_exists(&self, name: &str) -> bool {
        self.is_enum(name)
    }

    fn is_interface(&self, name: &str) -> bool {
        let classes = &self.engine_state.classes;
        if let Some(cls) = find_class_in_slice(&self.engine_state, &classes, name) {
            cls.is_interface
        } else {
            false
        }
    }

    fn is_trait(&self, name: &str) -> bool {
        let classes = &self.engine_state.classes;
        if let Some(cls) = find_class_in_slice(&self.engine_state, &classes, name) {
            cls.is_trait
        } else {
            false
        }
    }

    fn is_enum(&self, name: &str) -> bool {
        let classes = &self.engine_state.classes;
        if let Some(cls) = find_class_in_slice(&self.engine_state, &classes, name) {
            cls.is_enum
        } else {
            false
        }
    }

    fn is_class_abstract(&self, name: &str) -> bool {
        let classes = &self.engine_state.classes;
        if let Some(cls) = find_class_in_slice(&self.engine_state, &classes, name) {
            cls.is_abstract
        } else {
            false
        }
    }

    fn is_class_final(&self, name: &str) -> bool {
        let classes = &self.engine_state.classes;
        if let Some(cls) = find_class_in_slice(&self.engine_state, &classes, name) {
            cls.is_final
        } else {
            false
        }
    }

    fn is_class_readonly(&self, name: &str) -> bool {
        let classes = &self.engine_state.classes;
        if let Some(cls) = find_class_in_slice(&self.engine_state, &classes, name) {
            cls.is_readonly
        } else {
            false
        }
    }

    fn register_class_alias(&mut self, class_name: &str, alias_name: &str) -> bool {
        let norm_target = crate::vm::normalize_name(class_name);
        let target_id_opt = self.engine_state.class_map.get(&norm_target).map(|v| *v)
            .or_else(|| self.engine_state.classes.iter().find(|c| crate::vm::normalize_name(&c.value().name) == norm_target).map(|c| *c.key()));
        if let Some(target_id) = target_id_opt {
            let norm_alias = crate::vm::normalize_name(alias_name);
            self.engine_state.class_map.insert(norm_alias, target_id);
            crate::vm::VM::invalidate_method_cache();
            return true;
        }
        false
    }

    fn function_exists(&self, name: &str) -> bool {
        let clean_name = name.trim_start_matches('\\');
        let normalized = crate::vm::normalize_name(clean_name);
        if self.engine_state.func_map.contains_key(&normalized) {
            return true;
        }
        let registry = crate::stdlib::StdlibRegistry::global();
        if registry.functions.contains_key(&normalized) {
            return true;
        }
        if !clean_name.contains('\\') {
            let local = normalized.rsplit('\\').next().unwrap_or(&normalized);
            if self.engine_state.func_map.contains_key(local) {
                return true;
            }
            if registry.functions.contains_key(local) {
                return true;
            }
        }
        false
    }

    fn trigger_autoload_sync(&mut self, class_name: &str) {
        let clean_name = class_name.trim_start_matches('\\');
        let mut loaders = self.autoloaders.clone();
        if loaders.is_empty() {
            loaders = self.engine_state.autoloaders.read().unwrap().clone();
            self.autoloaders = loaders.clone();
        }
        hyperion_core::hyp_debug!("trigger_autoload_sync: {} loaders for {}", loaders.len(), clean_name);
        let class_name_str = clean_name.to_string();
        for loader in loaders {
            hyperion_core::hyp_debug!("calling loader: {:?}", loader.get_type());
            let class_name_ptr = self.arena.alloc_and_track(class_name_str.clone());
            let val = hyperion_core::memory::nan_box::Value::new_string_ptr(class_name_ptr as *mut ());
            let res = crate::vm::VM::call_callable_synchronously(self, loader, vec![val]);
            if let Err(e) = res {
                eprintln!("Autoloader failed: {}", e);
            } else {
                hyperion_core::hyp_debug!("Autoloader returned: {:?}", res.unwrap().get_type());
            }
            if <Self as hyperion_core::types::function::NativeContext>::class_exists(self, clean_name)
                || <Self as hyperion_core::types::function::NativeContext>::class_exists(self, class_name) {
                break;
            }
        }
    }

    fn register_autoloader(&mut self, loader: hyperion_core::memory::nan_box::Value, prepend: bool) {
        if prepend {
            self.autoloaders.insert(0, loader);
        } else {
            self.autoloaders.push(loader);
        }
    }



    fn unregister_autoloader(&mut self, loader: hyperion_core::memory::nan_box::Value) {
        self.autoloaders.retain(|x| {
            let mut equal = false;
            if loader.is_array() && x.is_array() {
                let arr_l = unsafe { &*(loader.as_array_ptr().unwrap() as *const hyperion_core::types::array::PhpArray) };
                let arr_x = unsafe { &*(x.as_array_ptr().unwrap() as *const hyperion_core::types::array::PhpArray) };
                let l0 = arr_l.get_int(0).map(|v| *v).unwrap_or(hyperion_core::memory::nan_box::Value::null());
                let l1 = arr_l.get_int(1).map(|v| *v).unwrap_or(hyperion_core::memory::nan_box::Value::null());
                let x0 = arr_x.get_int(0).map(|v| *v).unwrap_or(hyperion_core::memory::nan_box::Value::null());
                let x1 = arr_x.get_int(1).map(|v| *v).unwrap_or(hyperion_core::memory::nan_box::Value::null());
                
                let mut p0_eq = false;
                if l0.is_string() && x0.is_string() {
                    let s1 = unsafe { &*(l0.as_string_ptr().unwrap() as *const String) };
                    let s2 = unsafe { &*(x0.as_string_ptr().unwrap() as *const String) };
                    p0_eq = s1 == s2;
                } else if l0.is_object() && x0.is_object() {
                    p0_eq = l0.as_object_ptr() == x0.as_object_ptr();
                }
                
                let mut p1_eq = false;
                if l1.is_string() && x1.is_string() {
                    let s1 = unsafe { &*(l1.as_string_ptr().unwrap() as *const String) };
                    let s2 = unsafe { &*(x1.as_string_ptr().unwrap() as *const String) };
                    p1_eq = s1 == s2;
                }
                equal = p0_eq && p1_eq;
            } else if loader.is_string() && x.is_string() {
                let s1 = unsafe { &*(loader.as_string_ptr().unwrap() as *const String) };
                let s2 = unsafe { &*(x.as_string_ptr().unwrap() as *const String) };
                equal = s1 == s2;
            } else if loader.is_closure() && x.is_closure() {
                equal = loader.as_closure_ptr() == x.as_closure_ptr();
            }
            !equal
        });
    }

    fn get_autoloaders(&self) -> Vec<hyperion_core::memory::nan_box::Value> {
        self.autoloaders.clone()
    }

    fn get_parent_class(&self, class_name: &str) -> Option<String> {
        let classes = &self.engine_state.classes;
        find_class_in_slice(&self.engine_state, &classes, class_name).and_then(|c| c.extends.clone())
    }

    fn is_property_public(&self, class_name: &str, property_name: &str) -> bool {
        let classes = &self.engine_state.classes;
        let mut current_name = Some(class_name.to_string());
        let mut seen = std::collections::HashSet::new();
        while let Some(name) = current_name {
            let norm = crate::vm::normalize_name(&name);
            if !seen.insert(norm.clone()) {
                break;
            }
            if let Some(cls) = find_class_in_slice(&self.engine_state, &classes, &name) {
                if let Some((_, vis)) = cls.default_properties.get(property_name) {
                    return matches!(vis, hyperion_parser::parser::ast::Visibility::Public);
                }
                if let Some((_, vis)) = cls.static_properties.get(property_name) {
                    return matches!(vis, hyperion_parser::parser::ast::Visibility::Public);
                }
                current_name = cls.extends.clone();
            } else {
                break;
            }
        }
        false
    }

    fn is_property_static(&self, class_name: &str, property_name: &str) -> bool {
        let classes = &self.engine_state.classes;
        let mut current_name = Some(class_name.to_string());
        let mut seen = std::collections::HashSet::new();
        while let Some(name) = current_name {
            let norm = crate::vm::normalize_name(&name);
            if !seen.insert(norm.clone()) {
                break;
            }
            if let Some(cls) = find_class_in_slice(&self.engine_state, &classes, &name) {
                if cls.static_properties.contains_key(property_name) {
                    return true;
                }
                if cls.default_properties.contains_key(property_name) {
                    return false;
                }
                current_name = cls.extends.clone();
            } else {
                break;
            }
        }
        let norm = crate::vm::normalize_name(class_name);
        self.engine_state.default_statics.contains_key(&(norm, property_name.to_string()))
    }

    fn is_method_public(&self, class_name: &str, method_name: &str) -> bool {
        let classes = &self.engine_state.classes;
        if let Some(cls) = find_class_in_slice(&self.engine_state, &classes, class_name) {
            if let Some(fp) = cls.methods.get(method_name) {
                let func = unsafe { &*fp.0 };
                matches!(func.visibility, hyperion_parser::parser::ast::Visibility::Public)
            } else {
                false
            }
        } else {
            false
        }
    }

    fn has_method(&mut self, class_name: &str, method_name: &str) -> bool {
        let mut current_name = Some(class_name.to_string());
        while let Some(name) = current_name {
            self.trigger_autoload_sync(&name);
            let classes = &self.engine_state.classes;
            if let Some(cls) = find_class_in_slice(&self.engine_state, &classes, &name) {
                if cls.methods.keys().any(|k| k.eq_ignore_ascii_case(method_name)) {
                    return true;
                }
                current_name = cls.extends.clone();
            } else {
                break;
            }
        }
        false
    }

    fn get_static_property(&mut self, class_name: &str, property_name: &str) -> Option<hyperion_core::memory::nan_box::Value> {
        let prop_clean = property_name.strip_prefix('$').unwrap_or(property_name);
        let prop_variants = [
            prop_clean.to_string(),
            format!("${}", prop_clean),
            prop_clean.to_lowercase(),
            prop_clean.to_uppercase(),
        ];
        let mut seen: Vec<String> = Vec::new();
        let mut work: Vec<String> = vec![class_name.to_string()];
        while let Some(name) = work.pop() {
            let normalized = crate::vm::normalize_name(&name);
            if seen.contains(&normalized) {
                continue;
            }
            seen.push(normalized.clone());
            for p_var in &prop_variants {
                let key = (normalized.clone(), p_var.clone());
                if let Some(v) = self.static_properties.lock().unwrap().get(&key) {
                    return Some(*v);
                }
                if let Some(v) = self
                    .engine_state
                    .default_statics
                    .get(&key)
                {
                    let val = if v.is_array() || v.is_object() {
                        self.clone_value_to_fibre(&v)
                    } else {
                        *v
                    };
                    self.static_properties.lock().unwrap().insert(key, val);
                    return Some(val);
                }
            }
            let mut c = find_class_in_slice(&self.engine_state, &self.engine_state.classes, &normalized);
            if c.is_none() {
                self.trigger_autoload_sync(&name);
                for p_var in &prop_variants {
                    let key = (normalized.clone(), p_var.clone());
                    if let Some(v) = self.engine_state.default_statics.get(&key) {
                        let val = if v.is_array() || v.is_object() {
                            self.clone_value_to_fibre(&v)
                        } else {
                            *v
                        };
                        self.static_properties.lock().unwrap().insert(key, val);
                        return Some(val);
                    }
                }
                c = find_class_in_slice(&self.engine_state, &self.engine_state.classes, &normalized);
            }
            if let Some(c) = c {
                if let Some(parent) = &c.extends {
                    work.push(parent.clone());
                }
                for i in &c.implements {
                    work.push(i.clone());
                }
            }
        }
        None
    }

    fn set_static_property(&mut self, class_name: &str, property_name: &str, value: hyperion_core::memory::nan_box::Value) {
        let mut target_class = class_name.to_string();
        let classes = &self.engine_state.classes;
        let default_statics = &self.engine_state.default_statics;
        let mut current = Some(class_name.to_string());
        while let Some(name) = current {
            let normalized = crate::vm::normalize_name(&name);
            let key = (normalized.clone(), property_name.to_string());
            if default_statics.contains_key(&key) {
                target_class = normalized;
                break;
            }
            current = find_class_in_slice(&self.engine_state, &classes, &normalized)
                .and_then(|c| c.extends.clone());
        }
        drop(classes);
        drop(default_statics);

        let key = (crate::vm::normalize_name(&target_class), property_name.to_string());
        self.static_properties.lock().unwrap().insert(key, value);
    }


    fn implements_interface(&self, class_name: &str, interface_name: &str) -> bool {
        let classes = &self.engine_state.classes;
        let target_norm = crate::vm::normalize_name(interface_name);
        let class_norm = crate::vm::normalize_name(class_name);
        let mut seen: Vec<String> = Vec::new();
        let mut work: Vec<String> = vec![class_name.to_string()];
        while let Some(name) = work.pop() {
            let norm = crate::vm::normalize_name(&name);
            if norm == target_norm && norm != class_norm {
                return true;
            }
            if seen.iter().any(|s| crate::vm::normalize_name(s) == norm) {
                continue;
            }
            seen.push(name.clone());
            if let Some(c) = find_class_in_slice(&self.engine_state, &classes, &norm) {
                if let Some(parent) = c.extends.as_ref() {
                    work.push(parent.clone());
                }
                for i in &c.implements {
                    work.push(i.clone());
                }
            }
        }
        false
    }

    fn get_implemented_interfaces(&self, class_name: &str) -> Vec<String> {
        let classes = &self.engine_state.classes;
        let mut seen: Vec<String> = Vec::new();
        let mut work: Vec<String> = vec![class_name.to_string()];
        let mut interfaces: Vec<String> = Vec::new();
        while let Some(name) = work.pop() {
            if seen.iter().any(|s| s.eq_ignore_ascii_case(&name)) {
                continue;
            }
            seen.push(name.clone());
            if let Some(c) = find_class_in_slice(&self.engine_state, &classes, &name) {
                if let Some(parent) = c.extends.as_ref() {
                    work.push(parent.clone());
                }
                for i in &c.implements {
                    if !interfaces.iter().any(|iface| iface.eq_ignore_ascii_case(i)) {
                        interfaces.push(i.to_string());
                    }
                    work.push(i.to_string());
                }
            }
        }
        interfaces
    }

    fn get_parent_classes(&mut self, class_name: &str) -> Vec<String> {
        let mut parents = Vec::new();
        let mut current = Some(class_name.to_string());
        while let Some(name) = current {
            if !self.class_exists(&name) {
                self.trigger_autoload_sync(&name);
            }
            let classes = &self.engine_state.classes;
            if let Some(c) = find_class_in_slice(&self.engine_state, &classes, &name) {
                if let Some(parent) = c.extends.as_ref() {
                    parents.push(parent.clone());
                    current = Some(parent.clone());
                    continue;
                }
            }
            break;
        }
        parents
    }

    fn intern_string(&self, s: &str) -> usize {
        hyperion_core::types::string_table::intern_string(s)
    }

    fn lookup_string(&self, id: usize) -> Option<String> {
        hyperion_core::types::string_table::lookup_string(id)
    }

    fn get_method_params(&self, class_name: &str, method_name: &str) -> Option<Vec<(String, Option<String>, bool, bool, bool, Option<Value>)>> {
        let classes = &self.engine_state.classes;
        let mut current_name = Some(class_name.to_string());
        while let Some(name) = current_name {
            if let Some(cls) = find_class_in_slice(&self.engine_state, &classes, &name) {
                let key = cls.methods.keys().find(|k| k.to_lowercase() == method_name.to_lowercase());
                if let Some(k) = key {
                    if let Some(func_ptr) = cls.methods.get(k) {
                        let func = unsafe { &*func_ptr.0 };
                        let params: Vec<(String, Option<String>, bool, bool, bool, Option<Value>)> = func.params.iter().map(|p| {
                            let def_val = p.default_value.as_ref().map(|d| {
                                crate::vm::VM::default_val_to_value(&self.engine_state, d, &self.arena, None)
                            });
                            (p.name.clone(), p.type_hint.clone(), p.has_default, p.by_ref, p.is_variadic, def_val)
                        }).collect();
                        return Some(params);
                    }
                }
                current_name = cls.extends.clone();
            } else {
                break;
            }
        }
        None
    }

    fn get_class_methods(&mut self, class_name: &str) -> Option<Vec<String>> {
        if !self.class_exists(class_name) {
            self.trigger_autoload_sync(class_name);
        }
        let classes = &self.engine_state.classes;
        let mut methods = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut current_name = Some(class_name.to_string());
        let mut found_any_class = false;
        while let Some(name) = current_name {
            if let Some(cls) = find_class_in_slice(&self.engine_state, &classes, &name) {
                found_any_class = true;
                for (_, func_ptr) in &cls.methods {
                    let func = unsafe { &*func_ptr.0 };
                    let norm = crate::vm::normalize_name(&func.name);
                    if seen.insert(norm) {
                        methods.push(func.name.clone());
                    }
                }
                current_name = cls.extends.clone();
            } else {
                break;
            }
        }
        if found_any_class {
            Some(methods)
        } else {
            None
        }
    }

    fn get_declared_classes(&self) -> Vec<String> {
        let mut names = Vec::new();
        for entry in self.engine_state.classes.iter() {
            names.push(entry.value().name.clone());
        }
        names
    }

    fn get_declared_interfaces(&self) -> Vec<String> {
        Vec::new()
    }

    fn get_declared_traits(&self) -> Vec<String> {
        Vec::new()
    }

    fn get_class_file_name(&self, class_name: &str) -> Option<String> {
        let classes = &self.engine_state.classes;
        find_class_in_slice(&self.engine_state, &classes, class_name).and_then(|c| c.filename.clone())
    }

    fn is_method_static(&self, class_name: &str, method_name: &str) -> bool {
        let classes = &self.engine_state.classes;
        let mut current_name = Some(class_name.to_string());
        while let Some(name) = current_name {
            if let Some(cls) = find_class_in_slice(&self.engine_state, &classes, &name) {
                let key = cls.methods.keys().find(|k| k.to_lowercase() == method_name.to_lowercase());
                if let Some(k) = key {
                    if let Some(func_ptr) = cls.methods.get(k) {
                        let func = unsafe { &*func_ptr.0 };
                        return func.is_static;
                    }
                }
                current_name = cls.extends.clone();
            } else {
                break;
            }
        }
        false
    }

    fn get_method_attributes(&self, class_name: &str, method_name: &str) -> Vec<(String, Vec<(Option<String>, Value)>)> {
        let classes = &self.engine_state.classes;
        let mut current_name = Some(class_name.to_string());
        while let Some(name) = current_name {
            if let Some(cls) = find_class_in_slice(&self.engine_state, &classes, &name) {
                let norm = crate::vm::normalize_name(method_name);
                if let Some(func_ptr) = cls.methods.get(&norm).or_else(|| cls.methods.get(method_name)) {
                    let func = unsafe { &*func_ptr.0 };
                    return func.attributes.iter().map(|a| (a.name.clone(), a.args.clone())).collect();
                }
                current_name = cls.extends.clone();
            } else {
                break;
            }
        }
        Vec::new()
    }

    fn get_class_attributes(&self, class_name: &str) -> Vec<(String, Vec<(Option<String>, Value)>)> {
        let classes = &self.engine_state.classes;
        if let Some(cls) = find_class_in_slice(&self.engine_state, &classes, class_name) {
            cls.attributes.iter().map(|a| (a.name.clone(), a.args.clone())).collect()
        } else {
            Vec::new()
        }
    }

    fn get_class_traits(&mut self, class_name: &str) -> Vec<String> {
        if !self.class_exists(class_name) {
            self.trigger_autoload_sync(class_name);
        }
        let classes = &self.engine_state.classes;
        if let Some(cls) = find_class_in_slice(&self.engine_state, &classes, class_name) {
            cls.traits.clone()
        } else {
            Vec::new()
        }
    }

    fn has_class_property(&mut self, class_name: &str, prop_name: &str) -> bool {
        if !self.class_exists(class_name) {
            self.trigger_autoload_sync(class_name);
        }
        let mut current_name = Some(class_name.to_string());
        let mut seen = std::collections::HashSet::new();
        while let Some(name) = current_name {
            let norm = crate::vm::normalize_name(&name);
            if !seen.insert(norm.clone()) {
                break;
            }
            if let Some(cls) = find_class_in_slice(&self.engine_state, &self.engine_state.classes, &name) {
                if cls.default_properties.contains_key(prop_name) || cls.static_properties.contains_key(prop_name) {
                    return true;
                }
                current_name = cls.extends.clone();
                if let Some(ref p) = current_name {
                    if !self.class_exists(p) {
                        self.trigger_autoload_sync(p);
                    }
                }
            } else {
                break;
            }
        }
        let norm = crate::vm::normalize_name(class_name);
        self.engine_state.default_statics.contains_key(&(norm, prop_name.to_string()))
    }

    fn get_class_properties(&mut self, class_name: &str) -> Vec<hyperion_core::types::function::ReflectedPropertyInfo> {
        let mut results = Vec::new();
        let mut seen_names = std::collections::HashSet::new();

        if !self.class_exists(class_name) {
            self.trigger_autoload_sync(class_name);
        }

        let mut current_name = Some(class_name.to_string());
        let mut chain = Vec::new();
        let mut visited = std::collections::HashSet::new();
        while let Some(name) = current_name {
            let norm = crate::vm::normalize_name(&name);
            if !visited.insert(norm.clone()) {
                break;
            }
            if let Some(cls) = find_class_in_slice(&self.engine_state, &self.engine_state.classes, &norm) {
                chain.push(cls.clone());
                current_name = cls.extends.clone();
                if let Some(ref p) = current_name {
                    if !self.class_exists(p) {
                        self.trigger_autoload_sync(p);
                    }
                }
            } else {
                break;
            }
        }

        for (idx, cls) in chain.iter().enumerate() {
            let is_child = idx == 0;

            // Instance default properties
            for (prop_name, (default_val, vis)) in &cls.default_properties {
                if !seen_names.insert(prop_name.clone()) {
                    continue;
                }
                if !is_child && matches!(vis, hyperion_parser::parser::ast::Visibility::Private) {
                    continue;
                }
                let is_public = matches!(vis, hyperion_parser::parser::ast::Visibility::Public);
                let is_protected = matches!(vis, hyperion_parser::parser::ast::Visibility::Protected);
                let is_private = matches!(vis, hyperion_parser::parser::ast::Visibility::Private);

                results.push(hyperion_core::types::function::ReflectedPropertyInfo {
                    name: prop_name.clone(),
                    declaring_class: cls.name.clone(),
                    is_static: false,
                    is_public,
                    is_protected,
                    is_private,
                    is_readonly: false,
                    default_value: *default_val,
                });
            }

            // Static properties
            for (prop_name, (default_val, vis)) in &cls.static_properties {
                if !seen_names.insert(prop_name.clone()) {
                    continue;
                }
                if !is_child && matches!(vis, hyperion_parser::parser::ast::Visibility::Private) {
                    continue;
                }
                let is_public = matches!(vis, hyperion_parser::parser::ast::Visibility::Public);
                let is_protected = matches!(vis, hyperion_parser::parser::ast::Visibility::Protected);
                let is_private = matches!(vis, hyperion_parser::parser::ast::Visibility::Private);

                results.push(hyperion_core::types::function::ReflectedPropertyInfo {
                    name: prop_name.clone(),
                    declaring_class: cls.name.clone(),
                    is_static: true,
                    is_public,
                    is_protected,
                    is_private,
                    is_readonly: false,
                    default_value: *default_val,
                });
            }
        }

        // Also check if any properties in default_statics belong to this class
        let norm_class = crate::vm::normalize_name(class_name);
        for entry in self.engine_state.default_statics.iter() {
            let (cls, prop) = entry.key();
            if cls == &norm_class && seen_names.insert(prop.clone()) {
                results.push(hyperion_core::types::function::ReflectedPropertyInfo {
                    name: prop.clone(),
                    declaring_class: class_name.to_string(),
                    is_static: true,
                    is_public: true,
                    is_protected: false,
                    is_private: false,
                    is_readonly: false,
                    default_value: *entry.value(),
                });
            }
        }

        results
    }

    fn get_class_property_info(&mut self, class_name: &str, prop_name: &str) -> Option<hyperion_core::types::function::ReflectedPropertyInfo> {
        let props = self.get_class_properties(class_name);
        props.into_iter().find(|p| p.name == prop_name)
    }

    fn get_property_declaring_class(&mut self, class_name: &str, prop_name: &str) -> Option<String> {
        if let Some(info) = self.get_class_property_info(class_name, prop_name) {
            return Some(info.declaring_class);
        }
        None
    }

    fn get_method_declaring_class(&mut self, class_name: &str, method_name: &str) -> Option<String> {
        if !self.class_exists(class_name) {
            self.trigger_autoload_sync(class_name);
        }
        let mut current_name = Some(class_name.to_string());
        while let Some(name) = current_name {
            if !self.class_exists(&name) {
                self.trigger_autoload_sync(&name);
            }
            if let Some(cls) = find_class_in_slice(&self.engine_state, &self.engine_state.classes, &name) {
                if cls.methods.keys().any(|k| k.eq_ignore_ascii_case(method_name)) {
                    return Some(cls.name.clone());
                }
                current_name = cls.extends.clone();
            } else {
                break;
            }
        }
        None
    }

    fn get_callable_info(
        &self,
        callable: Value,
    ) -> Option<(String, bool, Option<String>, Vec<(String, Option<String>, bool, bool, bool, Option<Value>)>)> {
        // A closure carries its PhpFunction directly; a string names a global
        // function. Anything else has no ReflectionFunction representation.
        let mut scope_class = None;
        let func: &crate::types::function::PhpFunction = if callable.is_closure() {
            let closure_ptr = callable.as_closure_ptr()? as *const crate::types::closure::PhpClosure;
            let called_class_id = unsafe { (*closure_ptr).called_class_id };
            scope_class = self.get_class_name(called_class_id);
            unsafe { &*(*closure_ptr).function_ptr.0 }
        } else if callable.is_string() {
            let name_ptr = callable.as_string_ptr()?;
            let name = unsafe { (*(name_ptr as *const String)).clone() };
            let norm = crate::vm::normalize_name(&name);
            let functions = &self.engine_state.functions;
            let ptr = if let Some(id) = self.engine_state.func_map.get(&norm).map(|v| *v) {
                functions.get(&id).map(|v| *v)
            } else {
                functions.iter().find(|f| {
                    crate::vm::normalize_name(unsafe { &(*f.value().0).name }) == norm
                }).map(|v| *v.value())
            }?;
            unsafe { &*ptr.0 }
        } else {
            return None;
        };

        let params = func
            .params
            .iter()
            .map(|p| {
                let def_val = p.default_value.as_ref().map(|d| {
                    crate::vm::VM::default_val_to_value(&self.engine_state, d, &self.arena, None)
                });
                (p.name.clone(), p.type_hint.clone(), p.has_default, p.by_ref, p.is_variadic, def_val)
            })
            .collect();
        Some((func.name.clone(), func.is_static, scope_class, params))
    }

    fn get_closure_used_variables(
        &self,
        callable: Value,
    ) -> Vec<(String, Value)> {
        if callable.is_closure() {
            if let Some(closure_ptr) = callable.as_closure_ptr() {
                let closure = unsafe { &*(closure_ptr as *const crate::types::closure::PhpClosure) };
                let mut vars = Vec::new();
                for (i, val) in closure.upvalues.iter().enumerate() {
                    vars.push((format!("var_{}", i), *val));
                }
                return vars;
            }
        }
        Vec::new()
    }

    fn get_caller_arity(&self) -> usize {
        if self.frame_count >= 2 {
            let caller_idx = self.frame_count - 2;
            if let Some(args) = self.frame_args.get(caller_idx) {
                if !args.is_empty() {
                    return args.len();
                }
            }
            self.frames[caller_idx].arity
        } else {
            0
        }
    }

    fn get_caller_args(&self) -> Vec<hyperion_core::memory::nan_box::Value> {
        if self.frame_count >= 2 {
            let caller_idx = self.frame_count - 2;
            if let Some(args) = self.frame_args.get(caller_idx) {
                if !args.is_empty() {
                    return args.clone();
                }
            }
            let caller_frame = &self.frames[caller_idx];
            let arity = caller_frame.arity;
            let mut args = Vec::with_capacity(arity);
            for i in 0..arity {
                args.push(self.stack[caller_frame.stack_window + 1 + i]);
            }
            args
        } else {
            vec![]
        }
    }

    fn lookup_constant(&mut self, name: &str) -> Option<hyperion_core::memory::nan_box::Value> {
        let constants = &self.engine_state.constants;
        if let Some(val) = crate::vm::resolve_constant(&constants, name) {
            return Some(val);
        }
        drop(constants);

        if let Some((class_name, member_name)) = name.split_once("::") {
            let class_name = class_name.trim_start_matches('\\');
            if let Some(val) = self.get_static_property(class_name, member_name) {
                return Some(val);
            }
            self.trigger_autoload_sync(class_name);
            if let Some(val) = self.get_static_property(class_name, member_name) {
                return Some(val);
            }
            let constants = &self.engine_state.constants;
            if let Some(val) = crate::vm::resolve_constant(&constants, name) {
                return Some(val);
            }
            if let Some(val) = crate::vm::resolve_constant(&constants, &format!("{}::{}", class_name, member_name)) {
                return Some(val);
            }
        }
        None
    }

    fn define_constant(&mut self, name: &str, value: hyperion_core::memory::nan_box::Value) -> bool {
        let constants = &self.engine_state.constants;
        if constants.contains_key(name) {
            return false;
        }
        constants.insert(name.to_string(), value);
        true
    }

    fn write_output(&mut self, bytes: &[u8]) {
        // Same buffer `Opcode::Echo` appends to, so a native's output lands in
        // the right place relative to the surrounding echoes.
        self.output_buffer.extend_from_slice(bytes);
    }

    fn flush_output(&mut self) {
        use std::io::Write;
        let _ = std::io::stdout().write_all(&self.output_buffer);
        let _ = std::io::stdout().flush();
        self.output_buffer.clear();
    }

    fn iterator_to_array(&mut self, iterable: hyperion_core::memory::nan_box::Value, preserve_keys: bool) -> Result<hyperion_core::memory::nan_box::Value, String> {
        let mut result_arr = hyperion_core::types::array::PhpArray::new();
        if iterable.is_array() {
            if let Some(arr_ptr) = iterable.as_array_ptr() {
                let php_arr = unsafe { &*(arr_ptr as *const hyperion_core::types::array::PhpArray) };
                let mut int_key = 0;
                for (k, v) in php_arr.elements.iter() {
                    if preserve_keys {
                        match k {
                            hyperion_core::types::array::ArrayKey::Int(n) => {
                                result_arr.insert_int(*n, *v);
                            }
                            hyperion_core::types::array::ArrayKey::StringId(s) => {
                                result_arr.insert_string_id(*s, *v);
                            }
                        }
                    } else {
                        result_arr.insert_int(int_key, *v);
                        int_key += 1;
                    }
                }
            }
            let ptr = self.arena.alloc_and_track(result_arr);
            return Ok(hyperion_core::memory::nan_box::Value::new_array_ptr(ptr as *mut ()));
        } else if iterable.is_object() {
            let obj_ptr = iterable.as_object_ptr().unwrap() as *mut hyperion_core::types::object::PhpObject;
            let obj = unsafe { &*obj_ptr };
            let classes = &self.engine_state.classes;
            let is_gen = classes.get(&obj.class_id).map(|c| c.name == "Generator").unwrap_or(false);
            if is_gen {
                if let Some(gen_val) = obj.properties.get("__generator") {
                    let gen_fibre_ptr = gen_val.as_object_ptr().unwrap() as *mut Fibre;
                    let gen_fibre = unsafe { &mut *gen_fibre_ptr };
                    if !gen_fibre.generator_started {
                        gen_fibre.generator_started = true;
                        let res = crate::vm::VM::step_generator_fibre(gen_fibre);
                        match res {
                            crate::vm::ExecutionResult::Yielded(crate::vm::YieldReason::GeneratorYield) => {
                                gen_fibre.generator_valid = true;
                            }
                            crate::vm::ExecutionResult::Finished => {
                                gen_fibre.generator_valid = false;
                            }
                            crate::vm::ExecutionResult::Error(e) => return Err(e),
                            crate::vm::ExecutionResult::UncaughtException(e) => return Err(e),
                            _ => {
                                gen_fibre.generator_valid = false;
                            }
                        }
                    }
                    let mut int_counter = 0;
                    while gen_fibre.generator_valid {
                        let key_val = gen_fibre.generator_key;
                        let val_val = gen_fibre.generator_value;
                        if preserve_keys {
                            if let Some(k_int) = key_val.as_int() {
                                result_arr.insert_int(k_int as i64, val_val);
                            } else if let Some(k_str_ptr) = key_val.as_string_ptr() {
                                let k_str = unsafe { &*(k_str_ptr as *const String) };
                                let id = hyperion_core::types::string_table::intern_string(k_str);
                                result_arr.insert_string_id(id, val_val);
                            } else {
                                result_arr.insert_int(int_counter, val_val);
                            }
                        } else {
                            result_arr.insert_int(int_counter, val_val);
                        }
                        int_counter += 1;

                        let res = crate::vm::VM::run_fibre(gen_fibre);
                        match res {
                            crate::vm::ExecutionResult::Yielded(crate::vm::YieldReason::GeneratorYield) => {
                                gen_fibre.generator_valid = true;
                            }
                            crate::vm::ExecutionResult::Finished => {
                                gen_fibre.generator_valid = false;
                            }
                            crate::vm::ExecutionResult::Error(e) => return Err(e),
                            crate::vm::ExecutionResult::UncaughtException(e) => return Err(e),
                            _ => {
                                gen_fibre.generator_valid = false;
                            }
                        }
                    }
                    let ptr = self.arena.alloc_and_track(result_arr);
                    return Ok(hyperion_core::memory::nan_box::Value::new_array_ptr(ptr as *mut ()));
                }
            } else {
                if !crate::vm::VM::is_instance_of(self, iterable, "Traversable") {
                    return Err("iterator_to_array(): Argument #1 ($iterator) must be of type Traversable, object given".to_string());
                }
                if let Some(storage_val) = obj.properties.get("storage") {
                    if storage_val.is_array() {
                        return self.iterator_to_array(*storage_val, preserve_keys);
                    }
                }
                // Try getIterator for IteratorAggregate
                if crate::vm::VM::is_instance_of(self, iterable, "IteratorAggregate") {
                    if let Ok(iter) = crate::vm::VM::call_method_synchronously(self, iterable, "getIterator", vec![]) {
                        if (iter.is_object() || iter.is_array()) && iter != iterable {
                            return self.iterator_to_array(iter, preserve_keys);
                        }
                    }
                }
                // Or if it implements Iterator (rewind, valid, current, key, next)
                let _ = crate::vm::VM::call_method_synchronously(self, iterable, "rewind", vec![]);
                let mut int_counter = 0;
                while let Ok(v) = crate::vm::VM::call_method_synchronously(self, iterable, "valid", vec![]) {
                    if !v.as_bool().unwrap_or(false) {
                        break;
                    }
                    let curr_val = crate::vm::VM::call_method_synchronously(self, iterable, "current", vec![]).unwrap_or(hyperion_core::memory::nan_box::Value::null());
                    let key_val = crate::vm::VM::call_method_synchronously(self, iterable, "key", vec![]).unwrap_or(hyperion_core::memory::nan_box::Value::null());
                    if preserve_keys {
                        if let Some(k_int) = key_val.as_int() {
                            result_arr.insert_int(k_int as i64, curr_val);
                        } else if let Some(k_str_ptr) = key_val.as_string_ptr() {
                            let k_str = unsafe { &*(k_str_ptr as *const String) };
                            let id = hyperion_core::types::string_table::intern_string(k_str);
                            result_arr.insert_string_id(id, curr_val);
                        } else {
                            result_arr.insert_int(int_counter, curr_val);
                        }
                    } else {
                        result_arr.insert_int(int_counter, curr_val);
                    }
                    int_counter += 1;
                    let _ = crate::vm::VM::call_method_synchronously(self, iterable, "next", vec![]);
                }
                let ptr = self.arena.alloc_and_track(result_arr);
                return Ok(hyperion_core::memory::nan_box::Value::new_array_ptr(ptr as *mut ()));
            }
        }
        Err("iterator_to_array() expects parameter 1 to be array or Traversable".to_string())
    }

    fn ob_start(&mut self) {
        self.ob_buffers.push(Vec::new());
    }

    fn ob_get_clean(&mut self) -> Option<String> {
        self.ob_buffers.pop().map(|buf| String::from_utf8_lossy(&buf).to_string())
    }

    fn ob_get_contents(&self) -> Option<String> {
        self.ob_buffers.last().map(|buf| String::from_utf8_lossy(buf).to_string())
    }

    fn ob_end_clean(&mut self) -> bool {
        self.ob_buffers.pop().is_some()
    }

    fn ob_end_flush(&mut self) -> bool {
        if let Some(buf) = self.ob_buffers.pop() {
            if let Some(parent) = self.ob_buffers.last_mut() {
                parent.extend_from_slice(&buf);
            } else {
                self.output_buffer.extend_from_slice(&buf);
            }
            true
        } else {
            false
        }
    }

    fn ob_flush(&mut self) -> bool {
        if let Some(buf) = self.ob_buffers.last_mut() {
            let content = std::mem::take(buf);
            let len = self.ob_buffers.len();
            if len >= 2 {
                self.ob_buffers[len - 2].extend_from_slice(&content);
            } else {
                self.output_buffer.extend_from_slice(&content);
            }
            true
        } else {
            false
        }
    }

    fn ob_get_level(&self) -> usize {
        self.ob_buffers.len()
    }

    fn get_called_class_id(&self) -> usize {
        if self.frame_count > 0 {
            self.frames[self.frame_count - 1].called_class_id
        } else {
            0
        }
    }

    fn instantiate_class(&mut self, class_id: usize) -> hyperion_core::types::object::PhpObject {
        let class_name = self.engine_state.classes.get(&class_id).map(|c| c.name.clone());
        let mut obj = if let Some(ref name) = class_name {
            hyperion_core::types::object::PhpObject::new_with_name(class_id, name.clone())
        } else {
            hyperion_core::types::object::PhpObject::new(class_id)
        };
        let classes = &self.engine_state.classes;
        if let Some(_cls) = classes.get(&class_id) {
            let mut current_class_id = class_id;
            let mut property_overrides = std::collections::HashSet::new();
            loop {
                if let Some(c) = classes.get(&current_class_id) {
                    for (k, (val, _vis)) in &c.default_properties {
                        if !property_overrides.contains(k) {
                            let cloned_val = Self::clone_default_property(&mut self.arena, val);
                            obj.properties.insert(k.clone(), cloned_val);
                            property_overrides.insert(k.clone());
                        }
                    }

                    if let Some(ref ext) = c.extends {
                        if let Some(parent_idx) = classes.iter().find(|pc| crate::vm::normalize_name(&pc.value().name) == crate::vm::normalize_name(ext)).map(|pc| *pc.key()) {
                            current_class_id = parent_idx;
                            continue;
                        }
                    }
                }
                break;
            }
        }
        obj
    }

    fn set_http_response_code(&mut self, code: u16) -> u16 {
        let prev = self.response_status_code;
        self.response_status_code = code;
        prev
    }

    fn get_http_response_code(&self) -> u16 {
        self.response_status_code
    }

    fn add_response_header(&mut self, header: &str, replace: bool) {
        if let Some((name, value)) = header.split_once(':') {
            let name_trimmed = name.trim().to_string();
            let value_trimmed = value.trim().to_string();
            if replace {
                self.response_headers.retain(|(n, _)| !n.eq_ignore_ascii_case(&name_trimmed));
            }
            self.response_headers.push((name_trimmed, value_trimmed));
        } else {
            // e.g. "HTTP/1.1 404 Not Found"
            if header.starts_with("HTTP/") {
                let parts: Vec<&str> = header.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(code) = parts[1].parse::<u16>() {
                        self.response_status_code = code;
                    }
                }
            }
        }
    }

    fn remove_response_header(&mut self, name: &str) {
        self.response_headers.retain(|(n, _)| !n.eq_ignore_ascii_case(name));
    }

    fn get_response_headers(&self) -> Vec<String> {
        self.response_headers.iter().map(|(n, v)| format!("{}: {}", n, v)).collect()
    }

    fn headers_sent(&self) -> bool {
        self.headers_sent
    }

    fn extract_variables(&mut self, array_val: hyperion_core::memory::nan_box::Value) -> usize {
        if self.frame_count == 0 {
            return 0;
        }
        let Some(arr_ptr) = array_val.as_array_ptr() else {
            return 0;
        };
        let arr = unsafe { &*(arr_ptr as *const hyperion_core::types::array::PhpArray) };
        let top_func_name = unsafe { &*self.frames[self.frame_count - 1].function.0 }.name.clone();
        let caller_frame_idx = if self.frame_count >= 2 && (top_func_name == "extract" || top_func_name.is_empty()) {
            self.frame_count - 2
        } else {
            self.frame_count - 1
        };
        let caller_frame = self.frames[caller_frame_idx];
        let caller_func = unsafe { &*caller_frame.function.0 };
        let caller_chunk = &caller_func.chunk;
        let caller_window = caller_frame.stack_window;

        let mut count = 0;
        for (key, val) in &arr.elements {
            let key_str = match key {
                hyperion_core::types::array::ArrayKey::StringId(id) => {
                    hyperion_core::types::string_table::lookup_string(*id).unwrap_or_default()
                }
                hyperion_core::types::array::ArrayKey::Int(i) => i.to_string(),
            };
            if let Some(pos) = caller_chunk.local_names.iter().position(|n| n == &key_str) {
                if caller_window + pos < self.stack.len() {
                    self.stack[caller_window + pos] = *val;
                }
            }
            self.dynamic_locals.entry(caller_frame_idx).or_default().insert(key_str, *val);
            count += 1;
        }
        count
    }

    fn get_defined_vars(&self) -> Vec<(String, hyperion_core::memory::nan_box::Value)> {
        let mut target_frame_idx = None;
        for i in (0..self.frame_count).rev() {
            let func = unsafe { &*self.frames[i].function.0 };
            if !func.name.starts_with("native_")
                && func.name != "get_defined_vars"
                && func.name != "__internal_call_wrapper"
            {
                target_frame_idx = Some(i);
                break;
            }
        }
        let frame_idx = match target_frame_idx {
            Some(idx) => idx,
            None => {
                if self.frame_count > 0 {
                    self.frame_count - 1
                } else {
                    return Vec::new();
                }
            }
        };

        let frame = &self.frames[frame_idx];
        let chunk = unsafe { &(*frame.function.0).chunk };
        let stack_window = frame.stack_window;
        
        let mut res = Vec::new();
        for (pos, name) in chunk.local_names.iter().enumerate() {
            if name != "this" && name != "<closure>" {
                let val = self.stack[stack_window + pos].deref();
                res.push((name.clone(), val));
            }
        }
        if let Some(dyn_map) = self.dynamic_locals.get(&frame_idx) {
            for (k, v) in dyn_map {
                res.push((k.clone(), v.deref()));
            }
        }
        res
    }

    fn get_http_raw_body(&self) -> &[u8] {
        &self.http_raw_body
    }

    fn get_db_completion_callback(&self) -> std::sync::Arc<dyn Fn(u64, Result<hyperion_core::memory::nan_box::Value, String>) + Send + Sync> {
        let tx = self.engine_state.db_completion_tx.clone();
        let engine_state = self.engine_state.clone();
        std::sync::Arc::new(move |task_id, res| {
            let _ = tx.send((task_id, res));
            if let Ok(unparkers) = engine_state.unparkers.read() {
                for u in unparkers.iter() {
                    u.unpark();
                }
            }
        })
    }

    fn closure_from_callable(
        &mut self,
        callable: Value,
    ) -> Result<Value, String> {
        let callable = callable.deref();
        if callable.is_closure() {
            return Ok(callable);
        }

        // 1. Array callable: [$obj, $method] or [$class, $method]
        if let Some(arr_ptr) = callable.as_array_ptr() {
            let arr = unsafe { &*(arr_ptr as *const hyperion_core::types::array::PhpArray) };
            let (target_val, method_val) = if let (Some(c), Some(m)) = (arr.get_int(0), arr.get_int(1)) {
                (*c, *m)
            } else if arr.elements.len() >= 2 {
                let mut iter = arr.elements.values();
                (*iter.next().unwrap(), *iter.next().unwrap())
            } else {
                return Err("Closure::fromCallable(): array must contain at least 2 elements".to_string());
            };

            let target_val = target_val.deref();
            let method_val = method_val.deref();

            let method_name = if let Some(m_ptr) = method_val.as_string_ptr() {
                unsafe { &*(m_ptr as *const String) }.clone()
            } else {
                return Err("Closure::fromCallable(): method name must be a string".to_string());
            };

            let (class_id, this_val) = if let Some(obj_ptr) = target_val.as_object_ptr() {
                let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
                (obj.class_id, Some(target_val))
            } else if let Some(c_ptr) = target_val.as_string_ptr() {
                let class_name = unsafe { &*(c_ptr as *const String) };
                let resolved_name = crate::vm::VM::resolve_dynamic_class_name(self, class_name);
                let id = self.get_class_id(&resolved_name)
                    .or_else(|| {
                        self.trigger_autoload_sync(&resolved_name);
                        self.get_class_id(&resolved_name)
                    })
                    .ok_or_else(|| format!("Class \"{}\" not found", resolved_name))?;
                (id, None)
            } else {
                return Err("Closure::fromCallable(): first element must be an object or class name".to_string());
            };

            let method_ptr = match crate::vm::VM::find_method_fast(self, class_id, &method_name) {
                Ok(Some(ptr)) => ptr,
                _ => return Err(format!("Call to undefined method in Closure::fromCallable()")),
            };

            let closure = crate::types::closure::PhpClosure::new(method_ptr, Vec::new(), class_id, this_val);
            let ptr = self.arena.alloc_and_track(closure) as *mut ();
            return Ok(Value::new_closure_ptr(ptr));
        }

        // 2. String callable: function name or "Class::method"
        if let Some(s_ptr) = callable.as_string_ptr() {
            let s = unsafe { &*(s_ptr as *const String) };
            if s.contains("::") {
                let mut parts = s.splitn(2, "::");
                let class_str = parts.next().unwrap();
                let method_str = parts.next().unwrap();
                let resolved_name = crate::vm::VM::resolve_dynamic_class_name(self, class_str);
                let id = self.get_class_id(&resolved_name)
                    .or_else(|| {
                        self.trigger_autoload_sync(&resolved_name);
                        self.get_class_id(&resolved_name)
                    })
                    .ok_or_else(|| format!("Class \"{}\" not found", resolved_name))?;
                let method_ptr = match crate::vm::VM::find_method_fast(self, id, method_str) {
                    Ok(Some(ptr)) => ptr,
                    _ => return Err(format!("Call to undefined method {}::{} in Closure::fromCallable()", resolved_name, method_str)),
                };
                let closure = crate::types::closure::PhpClosure::new(method_ptr, Vec::new(), id, None);
                let ptr = self.arena.alloc_and_track(closure) as *mut ();
                return Ok(Value::new_closure_ptr(ptr));
            }

            // Global function
            let norm = crate::vm::normalize_name(s);
            let func_ptr = if let Some(id) = self.engine_state.func_map.get(&norm).map(|v| *v) {
                self.engine_state.functions.get(&id).map(|v| *v)
            } else {
                self.engine_state.functions.iter().find(|f| {
                    crate::vm::normalize_name(unsafe { &(*f.value().0).name }) == norm
                }).map(|v| *v.value())
            };
            let func_ptr = if let Some(fp) = func_ptr {
                fp
            } else {
                let registry = crate::stdlib::StdlibRegistry::global();
                if registry.functions.contains_key(&norm) {
                    crate::vm::VM::make_native_thunk_forward(&norm)
                } else if !norm.contains('\\') && registry.functions.contains_key(norm.rsplit('\\').next().unwrap_or(&norm)) {
                    let short_name = norm.rsplit('\\').next().unwrap_or(&norm);
                    crate::vm::VM::make_native_thunk_forward(short_name)
                } else {
                    return Err(format!("Function {} not found", s));
                }
            };

            let closure = crate::types::closure::PhpClosure::new(func_ptr, Vec::new(), 0, None);
            let ptr = self.arena.alloc_and_track(closure) as *mut ();
            return Ok(Value::new_closure_ptr(ptr));
        }

        // 3. Invokable object
        if let Some(obj_ptr) = callable.as_object_ptr() {
            let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
            let class_id = obj.class_id;
            let method_ptr = match crate::vm::VM::find_method_fast(self, class_id, "__invoke") {
                Ok(Some(ptr)) => ptr,
                _ => return Err("Object is not invokable in Closure::fromCallable()".to_string()),
            };
            let closure = crate::types::closure::PhpClosure::new(method_ptr, Vec::new(), class_id, Some(callable));
            let ptr = self.arena.alloc_and_track(closure) as *mut ();
            return Ok(Value::new_closure_ptr(ptr));
        }

        Err("Closure::fromCallable(): argument must be a valid callable".to_string())
    }
}
