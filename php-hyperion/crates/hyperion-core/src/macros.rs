/// Diagnostic print, silent unless `HYPERION_DEBUG` is set.
///
/// Engine tracing must never reach a script's stdout: PHP output *is* the
/// program's result, and a stray `DEBUG PARSER:` line corrupts it (and any
/// HTTP response built from it). Goes to stderr so it stays separable even
/// when enabled.
#[macro_export]
macro_rules! hyp_debug {
    ($($arg:tt)*) => {
        if std::env::var("HYPERION_DEBUG").is_ok() {
            eprintln!($($arg)*);
        }
    };
}

#[macro_export]
macro_rules! extract_arg {
    ($args:expr, $idx:expr, String, $func:expr, $ctx:expr) => {
        if $idx < $args.len() {
            let val = $args[$idx].deref();
            if let Some(ptr) = val.as_string_ptr() {
                Some(unsafe { &*(ptr as *const String) })
            } else if val.is_null() {
                static EMPTY_STR: std::sync::OnceLock<String> = std::sync::OnceLock::new();
                Some(EMPTY_STR.get_or_init(String::new))
            } else if let Some(i) = val.as_int() {
                let boxed = Box::leak(Box::new(i.to_string()));
                Some(&*boxed)
            } else if let Some(f) = val.as_float() {
                let boxed = Box::leak(Box::new(f.to_string()));
                Some(&*boxed)
            } else if let Some(b) = val.as_bool() {
                static TRUE_STR: std::sync::OnceLock<String> = std::sync::OnceLock::new();
                static FALSE_STR: std::sync::OnceLock<String> = std::sync::OnceLock::new();
                if b {
                    Some(TRUE_STR.get_or_init(|| "1".to_string()))
                } else {
                    Some(FALSE_STR.get_or_init(String::new))
                }
            } else if val.is_object() {
                if let Ok(res) = $ctx.call_method_synchronously(val, "__toString", vec![]) {
                    let res = res.deref();
                    if let Some(ptr) = res.as_string_ptr() {
                        Some(unsafe { &*(ptr as *const String) })
                    } else if res.is_null() {
                        static EMPTY_STR: std::sync::OnceLock<String> = std::sync::OnceLock::new();
                        Some(EMPTY_STR.get_or_init(String::new))
                    } else if let Some(i) = res.as_int() {
                        let boxed = Box::leak(Box::new(i.to_string()));
                        Some(&*boxed)
                    } else if let Some(f) = res.as_float() {
                        let boxed = Box::leak(Box::new(f.to_string()));
                        Some(&*boxed)
                    } else if let Some(b) = res.as_bool() {
                        static TRUE_STR: std::sync::OnceLock<String> = std::sync::OnceLock::new();
                        static FALSE_STR: std::sync::OnceLock<String> = std::sync::OnceLock::new();
                        if b {
                            Some(TRUE_STR.get_or_init(|| "1".to_string()))
                        } else {
                            Some(FALSE_STR.get_or_init(String::new))
                        }
                    } else {
                        return Err(format!("{}(): Method __toString() must return a string value", $func));
                    }
                } else {
                    return Err(format!("{}() expects parameter {} to be string, object given", $func, $idx + 1));
                }
            } else {
                return Err(format!("{}() expects parameter {} to be string", $func, $idx + 1));
            }
        } else {
            None
        }
    };
    ($args:expr, $idx:expr, bool, $func:expr, $ctx:expr) => {
        $crate::extract_arg!($args, $idx, bool, $func)
    };
    ($args:expr, $idx:expr, Object, $func:expr, $ctx:expr) => {
        $crate::extract_arg!($args, $idx, Object, $func)
    };
    ($args:expr, $idx:expr, Value, $func:expr, $ctx:expr) => {
        $crate::extract_arg!($args, $idx, Value, $func)
    };
    ($args:expr, $idx:expr, String, $func:expr) => {
        if $idx < $args.len() {
            let val = $args[$idx].deref();
            if let Some(ptr) = val.as_string_ptr() {
                Some(unsafe { &*(ptr as *const String) })
            } else if val.is_null() {
                static EMPTY_STR: std::sync::OnceLock<String> = std::sync::OnceLock::new();
                Some(EMPTY_STR.get_or_init(String::new))
            } else if let Some(i) = val.as_int() {
                let boxed = Box::leak(Box::new(i.to_string()));
                Some(&*boxed)
            } else if let Some(f) = val.as_float() {
                let boxed = Box::leak(Box::new(f.to_string()));
                Some(&*boxed)
            } else if let Some(b) = val.as_bool() {
                static TRUE_STR: std::sync::OnceLock<String> = std::sync::OnceLock::new();
                static FALSE_STR: std::sync::OnceLock<String> = std::sync::OnceLock::new();
                if b {
                    Some(TRUE_STR.get_or_init(|| "1".to_string()))
                } else {
                    Some(FALSE_STR.get_or_init(String::new))
                }
            } else {
                return Err(format!("{}() expects parameter {} to be string", $func, $idx + 1));
            }
        } else {
            None
        }
    };
    ($args:expr, $idx:expr, bool, $func:expr) => {
        if $idx < $args.len() {
            let val = $args[$idx].deref();
            if let Some(b) = val.as_bool() {
                Some(b)
            } else {
                return Err(format!("{}() expects parameter {} to be bool", $func, $idx + 1));
            }
        } else {
            None
        }
    };
    ($args:expr, $idx:expr, Object, $func:expr) => {
        if $idx < $args.len() {
            let val = $args[$idx].deref();
            if let Some(ptr) = val.as_object_ptr() {
                Some(unsafe { &*(ptr as *const $crate::types::object::PhpObject) })
            } else {
                return Err(format!("{}() expects parameter {} to be object", $func, $idx + 1));
            }
        } else {
            None
        }
    };
    ($args:expr, $idx:expr, Value, $func:expr) => {
        if $idx < $args.len() {
            Some(&$args[$idx])
        } else {
            None
        }
    };
}

#[macro_export]
macro_rules! php_function {
    // With Context, Variadic
    (
        $func_name:ident ( $( $arg_name:ident : $arg_type:ident ),* , ...$rest:ident ) |$ctx:ident| $body:expr
    ) => {
        pub fn $func_name(args: &[$crate::memory::nan_box::Value], $ctx: &mut dyn $crate::types::function::NativeContext) -> Result<$crate::memory::nan_box::Value, String> {
            let mut _arg_idx = 0;
            $(
                let $arg_name = $crate::extract_arg!(args, _arg_idx, $arg_type, stringify!($func_name), $ctx);
                _arg_idx += 1;
            )*
            let $rest = if _arg_idx < args.len() { &args[_arg_idx..] } else { &[] };
            
            $body
        }
    };
    // Without Context, Variadic
    (
        $func_name:ident ( $( $arg_name:ident : $arg_type:ident ),* , ...$rest:ident ) $body:expr
    ) => {
        pub fn $func_name(args: &[$crate::memory::nan_box::Value], _ctx: &mut dyn $crate::types::function::NativeContext) -> Result<$crate::memory::nan_box::Value, String> {
            let mut _arg_idx = 0;
            $(
                let $arg_name = $crate::extract_arg!(args, _arg_idx, $arg_type, stringify!($func_name), _ctx);
                _arg_idx += 1;
            )*
            let $rest = if _arg_idx < args.len() { &args[_arg_idx..] } else { &[] };
            
            $body
        }
    };
    // With Context, Fixed args
    (
        $func_name:ident ( $( $arg_name:ident : $arg_type:ident ),* ) |$ctx:ident| $body:expr
    ) => {
        pub fn $func_name(args: &[$crate::memory::nan_box::Value], $ctx: &mut dyn $crate::types::function::NativeContext) -> Result<$crate::memory::nan_box::Value, String> {
            let mut _arg_idx = 0;
            $(
                let $arg_name = $crate::extract_arg!(args, _arg_idx, $arg_type, stringify!($func_name), $ctx);
                _arg_idx += 1;
            )*
            
            $body
        }
    };
    // Without Context, Fixed args
    (
        $func_name:ident ( $( $arg_name:ident : $arg_type:ident ),* ) $body:expr
    ) => {
        pub fn $func_name(args: &[$crate::memory::nan_box::Value], _ctx: &mut dyn $crate::types::function::NativeContext) -> Result<$crate::memory::nan_box::Value, String> {
            let mut _arg_idx = 0;
            $(
                let $arg_name = $crate::extract_arg!(args, _arg_idx, $arg_type, stringify!($func_name), _ctx);
                _arg_idx += 1;
            )*
            
            $body
        }
    };
}
