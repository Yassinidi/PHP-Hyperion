use crate::memory::nan_box::Value;

#[derive(Clone, Debug)]
pub struct BacktraceFrame {
    pub file: String,
    pub line: usize,
    pub function: String,
    pub class: Option<String>,
    pub frame_type: Option<String>,
    pub object: Option<crate::memory::nan_box::Value>,
}

#[derive(Clone, Debug)]
pub struct ReflectedPropertyInfo {
    pub name: String,
    pub declaring_class: String,
    pub is_static: bool,
    pub is_public: bool,
    pub is_protected: bool,
    pub is_private: bool,
    pub is_readonly: bool,
    pub default_value: crate::memory::nan_box::Value,
}

pub trait NativeContext {
    fn get_arena(&mut self) -> &crate::gc::arena::GcArena;
    fn get_backtrace(&self) -> Vec<BacktraceFrame>;
    fn call_callable_synchronously(
        &mut self,
        callable: crate::memory::nan_box::Value,
        args: Vec<crate::memory::nan_box::Value>,
    ) -> Result<crate::memory::nan_box::Value, String>;
    fn call_method_synchronously(
        &mut self,
        obj: crate::memory::nan_box::Value,
        method: &str,
        args: Vec<crate::memory::nan_box::Value>,
    ) -> Result<crate::memory::nan_box::Value, String>;
    fn get_class_name(&self, class_id: usize) -> Option<String>;
    fn get_class_id(&self, class_name: &str) -> Option<usize>;
    fn class_exists(&self, name: &str) -> bool;
    fn interface_exists(&self, name: &str) -> bool;
    fn trait_exists(&self, name: &str) -> bool;
    fn enum_exists(&self, name: &str) -> bool;
    fn is_interface(&self, name: &str) -> bool;
    fn is_trait(&self, name: &str) -> bool;
    fn is_enum(&self, name: &str) -> bool;
    fn is_class_abstract(&self, name: &str) -> bool;
    fn is_class_final(&self, name: &str) -> bool;
    fn is_class_readonly(&self, name: &str) -> bool;
    fn register_class_alias(&mut self, class_name: &str, alias_name: &str) -> bool;
    /// Whether a function of this name can actually be called — a user-defined
    /// one compiled so far, or a registered native.
    ///
    /// `function_exists()` must answer honestly. Returning true for everything
    /// is worse than returning false: Laravel guards optional extensions with
    /// it (`function_exists('pcntl_signal')`), so a blanket yes routes the code
    /// straight into a call that does not resolve.
    fn function_exists(&self, name: &str) -> bool;
    fn trigger_autoload_sync(&mut self, class_name: &str);
    fn register_autoloader(&mut self, loader: crate::memory::nan_box::Value, prepend: bool);
    fn unregister_autoloader(&mut self, loader: crate::memory::nan_box::Value);
    fn get_autoloaders(&self) -> Vec<crate::memory::nan_box::Value>;
    fn get_parent_class(&self, class_name: &str) -> Option<String>;
    fn is_property_public(&self, class_name: &str, property_name: &str) -> bool;
    fn is_method_public(&self, class_name: &str, method_name: &str) -> bool;
    fn has_method(&mut self, class_name: &str, method_name: &str) -> bool;
    fn get_static_property(&mut self, class_name: &str, property_name: &str) -> Option<crate::memory::nan_box::Value>;
    fn set_static_property(&mut self, class_name: &str, property_name: &str, value: crate::memory::nan_box::Value);
    fn implements_interface(&self, class_name: &str, interface_name: &str) -> bool;
    fn get_implemented_interfaces(&self, class_name: &str) -> Vec<String>;
    fn get_parent_classes(&mut self, class_name: &str) -> Vec<String>;
    fn intern_string(&self, s: &str) -> usize;
    fn lookup_string(&self, id: usize) -> Option<String>;
    fn get_method_params(&self, class_name: &str, method_name: &str) -> Option<Vec<(String, Option<String>, bool, bool, bool, Option<crate::memory::nan_box::Value>)>>;
    fn get_class_methods(&mut self, class_name: &str) -> Option<Vec<String>>;
    fn get_declared_classes(&self) -> Vec<String>;
    fn get_declared_interfaces(&self) -> Vec<String>;
    fn get_declared_traits(&self) -> Vec<String>;
    fn get_class_file_name(&self, class_name: &str) -> Option<String>;
    /// Introspect a closure or named-function Value for ReflectionFunction.
    /// Returns (name, is_static, scope_class, params).
    fn get_callable_info(
        &self,
        callable: crate::memory::nan_box::Value,
    ) -> Option<(String, bool, Option<String>, Vec<(String, Option<String>, bool, bool, bool, Option<crate::memory::nan_box::Value>)>)>;
    fn get_closure_used_variables(
        &self,
        callable: crate::memory::nan_box::Value,
    ) -> Vec<(String, crate::memory::nan_box::Value)>;
    fn is_method_static(&self, class_name: &str, method_name: &str) -> bool;
    fn get_method_attributes(&self, class_name: &str, method_name: &str) -> Vec<(String, Vec<(Option<String>, crate::memory::nan_box::Value)>)>;
    fn get_class_attributes(&self, class_name: &str) -> Vec<(String, Vec<(Option<String>, crate::memory::nan_box::Value)>)>;
    fn get_class_traits(&mut self, class_name: &str) -> Vec<String>;
    fn has_class_property(&mut self, class_name: &str, prop_name: &str) -> bool;
    fn get_class_properties(&mut self, class_name: &str) -> Vec<ReflectedPropertyInfo>;
    fn get_class_property_info(&mut self, class_name: &str, prop_name: &str) -> Option<ReflectedPropertyInfo>;
    fn is_property_static(&self, class_name: &str, property_name: &str) -> bool;
    fn get_property_declaring_class(&mut self, class_name: &str, prop_name: &str) -> Option<String>;
    fn get_method_declaring_class(&mut self, class_name: &str, method_name: &str) -> Option<String>;
    fn get_caller_arity(&self) -> usize;
    fn get_caller_args(&self) -> Vec<crate::memory::nan_box::Value>;
    /// Look up a global constant by name, so `defined()` and `constant()` can
    /// answer from the same table `Opcode::FetchConstant` reads.
    fn lookup_constant(&mut self, name: &str) -> Option<crate::memory::nan_box::Value>;
    fn closure_from_callable(
        &mut self,
        callable: crate::memory::nan_box::Value,
    ) -> Result<crate::memory::nan_box::Value, String>;
    /// Define a global constant, for `define()`. Returns false if it already
    /// exists, matching PHP's refusal to redefine.
    fn define_constant(&mut self, name: &str, value: crate::memory::nan_box::Value) -> bool;
    /// Append bytes to the script's output, the same buffer `echo` writes to.
    ///
    /// Natives that produce output (`var_dump`, `print_r`, `printf`, a write to
    /// `php://stdout`) must go through here rather than `print!`. The SAPI
    /// decides when that buffer is flushed, so anything printed directly to fd
    /// 1 jumps ahead of every pending `echo` and scrambles the ordering.
    fn write_output(&mut self, bytes: &[u8]);
    fn flush_output(&mut self);
    fn iterator_to_array(&mut self, iterable: crate::memory::nan_box::Value, preserve_keys: bool) -> Result<crate::memory::nan_box::Value, String>;
    fn ob_start(&mut self);
    fn ob_get_clean(&mut self) -> Option<String>;
    fn ob_get_contents(&self) -> Option<String>;
    fn ob_end_clean(&mut self) -> bool;
    fn ob_end_flush(&mut self) -> bool;
    fn ob_flush(&mut self) -> bool;
    fn ob_get_level(&self) -> usize;
    fn get_called_class_id(&self) -> usize;
    fn instantiate_class(&mut self, class_id: usize) -> crate::types::object::PhpObject;
    fn set_http_response_code(&mut self, code: u16) -> u16;
    fn get_http_response_code(&self) -> u16;
    fn add_response_header(&mut self, header: &str, replace: bool);
    fn remove_response_header(&mut self, name: &str);
    fn get_response_headers(&self) -> Vec<String>;
    fn headers_sent(&self) -> bool;
    fn extract_variables(&mut self, array_val: crate::memory::nan_box::Value) -> usize;
    fn get_defined_vars(&self) -> Vec<(String, crate::memory::nan_box::Value)>;
    fn get_http_raw_body(&self) -> &[u8] { &[] }
    fn get_db_completion_callback(&self) -> std::sync::Arc<dyn Fn(u64, Result<crate::memory::nan_box::Value, String>) + Send + Sync> {
        std::sync::Arc::new(|_, _| {})
    }
}

pub type NativeFn = fn(args: &[Value], ctx: &mut dyn NativeContext) -> Result<Value, String>;
