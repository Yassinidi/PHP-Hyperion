use std::sync::Arc;
use fancy_regex::{Captures, Regex, RegexBuilder};
use hyperion_core::memory::nan_box::Value;
use hyperion_core::types::array::{ArrayKey, PhpArray};
use hyperion_core::types::function::NativeContext;
use hyperion_core::types::reference::PhpRef;

// ============================================================
// 🔍 PHP PCRE Module — Full native regex implementation on fancy-regex
// ============================================================

// Error codes matching PHP PCRE constants:
pub const PREG_NO_ERROR: i32 = 0;
pub const PREG_INTERNAL_ERROR: i32 = 1;
pub const PREG_BACKTRACK_LIMIT_ERROR: i32 = 2;
pub const PREG_RECURSION_LIMIT_ERROR: i32 = 3;
pub const PREG_BAD_UTF8_ERROR: i32 = 4;
pub const PREG_BAD_UTF8_OFFSET_ERROR: i32 = 5;
pub const PREG_JIT_STACKLIMIT_ERROR: i32 = 6;

// Flags matching PHP constants:
pub const PREG_PATTERN_ORDER: i64 = 1;
pub const PREG_SET_ORDER: i64 = 2;
pub const PREG_OFFSET_CAPTURE: i64 = 256;
pub const PREG_UNMATCHED_AS_NULL: i64 = 512;

pub const PREG_SPLIT_NO_EMPTY: i64 = 1;
pub const PREG_SPLIT_DELIM_CAPTURE: i64 = 2;
pub const PREG_SPLIT_OFFSET_CAPTURE: i64 = 4;

pub const PREG_GREP_INVERT: i64 = 1;

thread_local! {
    static LAST_PREG_ERROR: std::cell::Cell<i32> = std::cell::Cell::new(PREG_NO_ERROR);
    static LAST_PREG_ERROR_MSG: std::cell::RefCell<String> = std::cell::RefCell::new(String::from("No error"));
    static THREAD_REGEX_L1: std::cell::RefCell<std::collections::HashMap<String, Arc<Regex>>> = std::cell::RefCell::new(std::collections::HashMap::with_capacity(512));
}

#[inline(always)]
pub fn with_value_str<F, R>(val: &Value, f: F) -> Option<R>
where
    F: FnOnce(&str) -> R,
{
    let v = val.deref();
    if let Some(p) = v.as_string_ptr() {
        let s = unsafe { &*(p as *const String) };
        Some(f(s.as_str()))
    } else if let Some(i) = v.as_int() {
        let s = i.to_string();
        Some(f(&s))
    } else if let Some(b) = v.as_bool() {
        Some(f(if b { "1" } else { "" }))
    } else if v.is_null() {
        Some(f(""))
    } else if v.is_object() {
        if let Some(obj_ptr) = v.as_object_ptr() {
            let obj = unsafe { &*(obj_ptr as *const hyperion_core::types::object::PhpObject) };
            if let Some(prop_val) = obj.properties.get("value") {
                if let Some(s_ptr) = prop_val.deref().as_string_ptr() {
                    let s = unsafe { &*(s_ptr as *const String) };
                    return Some(f(s.as_str()));
                }
            }
        }
        None
    } else {
        None
    }
}



lazy_static::lazy_static! {
    static ref REGEX_CACHE: dashmap::DashMap<String, Arc<Regex>> = dashmap::DashMap::new();
}

pub fn set_preg_error(code: i32, msg: &str) {
    LAST_PREG_ERROR.with(|e| e.set(code));
    LAST_PREG_ERROR_MSG.with(|m| *m.borrow_mut() = msg.to_string());
}

pub fn clear_preg_error() {
    LAST_PREG_ERROR.with(|e| e.set(PREG_NO_ERROR));
    LAST_PREG_ERROR_MSG.with(|m| *m.borrow_mut() = "No error".to_string());
}

fn error_to_code(err: &fancy_regex::Error) -> (i32, &'static str) {
    match err {
        fancy_regex::Error::RuntimeError(fancy_regex::RuntimeError::BacktrackLimitExceeded) => {
            (PREG_BACKTRACK_LIMIT_ERROR, "Backtrack limit exhausted")
        }
        _ => (PREG_INTERNAL_ERROR, "Internal error"),
    }
}

/// Strip delimiters, parse modifier flags, and prepare pattern for fancy-regex.
pub fn parse_php_pattern(pattern: &str) -> Result<(String, bool, bool, bool, bool, bool, bool), String> {
    let pattern = pattern.trim_start();
    if pattern.is_empty() {
        return Err("Empty regular expression".to_string());
    }

    let first_char = pattern.chars().next().unwrap();
    if first_char.is_alphanumeric() || first_char == '\\' || first_char.is_whitespace() {
        return Err("Delimiter must not be alphanumeric, backslash, or whitespace".to_string());
    }

    let (delim_close, is_bracket) = match first_char {
        '(' => (')', true),
        '[' => (']', true),
        '{' => ('}', true),
        '<' => ('>', true),
        c => (c, false),
    };

    let mut depth: usize = if is_bracket { 1 } else { 0 };
    let mut escaped = false;
    let mut close_pos = None;

    let chars: Vec<(usize, char)> = pattern.char_indices().collect();
    let mut in_char_class = false;
    for i in 1..chars.len() {
        let (byte_idx, c) = chars[i];
        if escaped {
            escaped = false;
            continue;
        }
        if c == '\\' {
            escaped = true;
            continue;
        }
        if c == '[' && first_char != '[' {
            in_char_class = true;
            continue;
        }
        if c == ']' && in_char_class {
            in_char_class = false;
            continue;
        }
        if in_char_class && is_bracket {
            continue;
        }
        if is_bracket {
            if c == first_char {
                depth += 1;
            } else if c == delim_close {
                depth -= 1;
                if depth == 0 {
                    close_pos = Some((byte_idx, c.len_utf8()));
                    break;
                }
            }
        } else if c == delim_close {
            close_pos = Some((byte_idx, c.len_utf8()));
            break;
        }
    }

    let (close_byte_idx, close_char_len) = close_pos
        .ok_or_else(|| format!("No ending delimiter '{}' found", delim_close))?;

    let first_char_len = first_char.len_utf8();
    let raw_body = &pattern[first_char_len..close_byte_idx];
    let flags_str = &pattern[close_byte_idx + close_char_len..];

    let mut case_insensitive = false;
    let mut multiline = false;
    let mut dotall = false;
    let mut extended = false;
    let mut ungreedy = false;
    let mut anchored = false;

    for f in flags_str.chars() {
        match f {
            'i' => case_insensitive = true,
            'm' => multiline = true,
            's' => dotall = true,
            'x' => extended = true,
            'U' => ungreedy = true,
            'A' => anchored = true,
            'u' | 'n' | 'J' | 'S' | 'X' | 'D' => {}, // recognized modifiers
            ' ' | '\t' | '\r' | '\n' => {},
            _ => {}, // tolerate unknown modifiers
        }
    }

    // Strip common PCRE control verbs at the start of raw_body
    let mut body = raw_body;
    let verbs = ["(*UTF8)", "(*UTF)", "(*UCP)", "(*CRLF)", "(*ANYCRLF)", "(*ANY)", "(*NO_AUTO_CAPTURE)"];
    for verb in &verbs {
        if body.starts_with(verb) {
            body = &body[verb.len()..];
        }
    }

    let mut body_str = body.to_string();
    if body_str.contains("(?") || body_str.contains("(?>") {
        body_str = body_str.replace(
            r#"\{(?-1)\}"#,
            r#"\{[^{}]*(?:\{[^{}]*\}[^{}]*)*\}"#,
        );
        body_str = body_str.replace(
            r#"\{(?:(?-1))\}"#,
            r#"\{[^{}]*(?:\{[^{}]*\}[^{}]*)*\}"#,
        );
        body_str = body_str.replace(
            "(?: (?>[^()]+) | (?-1) )*",
            r#"(?:[^()]+|\((?:[^()]+|\([^()]*\))*\))*"#
        );
        body_str = body_str.replace(
            "(?:(?>[^()]+)|(?-1))*",
            r#"(?:[^()]+|\((?:[^()]+|\([^()]*\))*\))*"#
        );
        body_str = body_str.replace(
            "( (?>[^()]+) | (?2) )*",
            r#"(?:[^()]+|\((?:[^()]+|\([^()]*\))*\))*"#
        );
        body_str = body_str.replace(
            "((?>[^()]+)|(?2))*",
            r#"(?:[^()]+|\((?:[^()]+|\([^()]*\))*\))*"#
        );
        body_str = body_str.replace(
            "( (?>[^()]+) | (?1) )*",
            r#"(?:[^()]+|\((?:[^()]+|\([^()]*\))*\))*"#
        );
        body_str = body_str.replace(
            "((?>[^()]+)|(?1))*",
            r#"(?:[^()]+|\((?:[^()]+|\([^()]*\))*\))*"#
        );
        for num in 0..=9 {
            body_str = body_str.replace(&format!("(?{})", num), r#"(?:\((?:[^()]+|\([^()]*\))*\)|[^()]+)*"#);
        }
        body_str = body_str.replace("(?-1)", r#"(?:\((?:[^()]+|\([^()]*\))*\)|[^()]+)*"#);
        body_str = body_str.replace("(?R)", r#"(?:[^()]+|\((?:[^()]+|\([^()]*\))*\))*"#);
        body_str = body_str.replace("(?>", "(?:");
        body_str = body_str.replace("(?|", "(?:");
    }

    if body_str.contains("(*SKIP)") {
        body_str = body_str.replace("(*SKIP)(*FAIL)", "(*SKIP)(*F)");
        body_str = body_str.replace("(*FAIL)", "(*F)");
        if let Some(pos) = body_str.find("(*SKIP)(*F)") {
            let left = &body_str[..pos];
            let after = &body_str[pos + "(*SKIP)(*F)".len()..];
            let right = after.trim_start().strip_prefix('|').unwrap_or(after);
            body_str = format!("(?P<__skip__>{})|(?P<__match__>{})", left.trim(), right.trim());
        }
    }

    body_str = translate_pcre_shorthands(&body_str);

    Ok((body_str, case_insensitive, multiline, dotall, extended, ungreedy, anchored))
}

fn translate_pcre_shorthands(pattern: &str) -> String {
    let mut out = String::with_capacity(pattern.len() + 16);
    let mut in_class = false;
    let mut escaped = false;

    for c in pattern.chars() {
        if escaped {
            escaped = false;
            match c {
                'h' => {
                    if in_class {
                        out.push_str(" \t");
                    } else {
                        out.push_str("[ \t]");
                    }
                }
                'H' => {
                    if in_class {
                        out.push_str("^ \t");
                    } else {
                        out.push_str("[^ \t]");
                    }
                }
                'v' => {
                    if in_class {
                        out.push_str("\r\n\x0B\x0C");
                    } else {
                        out.push_str("[\r\n\x0B\x0C]");
                    }
                }
                'V' => {
                    if in_class {
                        out.push_str("^\r\n\x0B\x0C");
                    } else {
                        out.push_str("[^\r\n\x0B\x0C]");
                    }
                }
                other => {
                    out.push('\\');
                    out.push(other);
                }
            }
            continue;
        }

        if c == '\\' {
            escaped = true;
            continue;
        }

        if c == '[' && !in_class {
            in_class = true;
            out.push('[');
            continue;
        }

        if c == ']' && in_class {
            in_class = false;
            out.push(']');
            continue;
        }

        out.push(c);
    }

    if escaped {
        out.push('\\');
    }

    out
}

pub fn get_or_compile_regex(pattern_str: &str) -> Result<Arc<Regex>, (i32, String)> {
    if let Some(re) = THREAD_REGEX_L1.with(|c| c.borrow().get(pattern_str).cloned()) {
        return Ok(re);
    }

    if let Some(re) = REGEX_CACHE.get(pattern_str) {
        let arc_re = Arc::clone(&re);
        THREAD_REGEX_L1.with(|c| c.borrow_mut().insert(pattern_str.to_string(), Arc::clone(&arc_re)));
        return Ok(arc_re);
    }

    let (body, case_insensitive, multiline, dotall, extended, ungreedy, anchored) =
        parse_php_pattern(pattern_str).map_err(|msg| (PREG_INTERNAL_ERROR, msg))?;

    let mut final_body = body;
    if ungreedy {
        final_body = format!("(?U){}", final_body);
    }
    if anchored {
        final_body = format!(r"\A(?:{})", final_body);
    }

    let mut builder = RegexBuilder::new(&final_body);
    builder.case_insensitive(case_insensitive);
    builder.multi_line(multiline);
    builder.dot_matches_new_line(dotall);
    builder.ignore_whitespace(extended);

    let re = builder.build().map_err(|e| {
        let code = match &e {
            fancy_regex::Error::RuntimeError(fancy_regex::RuntimeError::BacktrackLimitExceeded) => PREG_BACKTRACK_LIMIT_ERROR,
            _ => PREG_INTERNAL_ERROR,
        };
        (code, format!("Compilation failed: {}", e))
    })?;

    let arc_re = Arc::new(re);
    REGEX_CACHE.insert(pattern_str.to_string(), Arc::clone(&arc_re));
    THREAD_REGEX_L1.with(|c| c.borrow_mut().insert(pattern_str.to_string(), Arc::clone(&arc_re)));
    Ok(arc_re)
}


fn next_char_boundary(s: &str, mut pos: usize) -> usize {
    while pos < s.len() && !s.is_char_boundary(pos) {
        pos += 1;
    }
    pos
}

pub fn write_out_var(arg: &Value, new_val: Value, _ctx: &mut dyn NativeContext) {
    if let Some(ref_ptr) = arg.as_ref_ptr() {
        let r = unsafe { &mut *(ref_ptr as *mut PhpRef) };
        r.set(new_val);
    } else if let Some(target_arr_ptr) = arg.as_array_ptr() {
        if let Some(new_arr_ptr) = new_val.as_array_ptr() {
            let target = unsafe { &mut *(target_arr_ptr as *mut PhpArray) };
            let src = unsafe { &*(new_arr_ptr as *const PhpArray) };
            target.elements = src.elements.clone();
        }
    }
}

pub fn value_to_string(val: &Value) -> Option<String> {
    let v = val.deref();
    if let Some(p) = v.as_string_ptr() {
        Some(unsafe { (*(p as *const String)).clone() })
    } else if let Some(i) = v.as_int() {
        Some(i.to_string())
    } else if let Some(f) = v.as_float() {
        Some(f.to_string())
    } else if let Some(b) = v.as_bool() {
        Some(if b { "1".to_string() } else { "".to_string() })
    } else if v.is_null() {
        Some("".to_string())
    } else {
        None
    }
}

fn expand_replacement(template: &str, caps: &Captures<'_, str>) -> String {
    let mut result = String::with_capacity(template.len());
    let chars: Vec<char> = template.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if (c == '\\' || c == '$') && i + 1 < chars.len() {
            let next = chars[i + 1];
            if next == '\\' || next == '$' {
                result.push(next);
                i += 2;
                continue;
            }
            if next.is_ascii_digit() {
                let mut num = (next as u32 - '0' as u32) as usize;
                let mut j = i + 2;
                if j < chars.len() && chars[j].is_ascii_digit() {
                    let num2 = num * 10 + (chars[j] as u32 - '0' as u32) as usize;
                    if num2 < caps.len() {
                        num = num2;
                        j += 1;
                    }
                }
                if let Some(m) = caps.get(num) {
                    result.push_str(m.as_str());
                }
                i = j;
                continue;
            }
            if next == '{' {
                if let Some(end_brace) = chars[i + 2..].iter().position(|&ch| ch == '}') {
                    let inner: String = chars[i + 2..i + 2 + end_brace].iter().collect();
                    if let Ok(group_idx) = inner.parse::<usize>() {
                        if let Some(m) = caps.get(group_idx) {
                            result.push_str(m.as_str());
                        }
                    } else if let Some(m) = caps.name(&inner) {
                        result.push_str(m.as_str());
                    }
                    i = i + 2 + end_brace + 1;
                    continue;
                }
            }
            if next == 'g' && i + 2 < chars.len() {
                if chars[i + 2] == '{' || chars[i + 2] == '<' {
                    let close = if chars[i + 2] == '{' { '}' } else { '>' };
                    if let Some(end_pos) = chars[i + 3..].iter().position(|&ch| ch == close) {
                        let inner: String = chars[i + 3..i + 3 + end_pos].iter().collect();
                        if let Ok(group_idx) = inner.parse::<usize>() {
                            if let Some(m) = caps.get(group_idx) {
                                result.push_str(m.as_str());
                            }
                        } else if let Some(m) = caps.name(&inner) {
                            result.push_str(m.as_str());
                        }
                        i = i + 3 + end_pos + 1;
                    }
                }
            }
        }
        result.push(c);
        i += 1;
    }
    result
}

fn build_single_matches_array(
    caps: &Captures<'_, str>,
    re: &Regex,
    flags: i64,
    base_offset: usize,
    ctx: &mut dyn NativeContext,
) -> Value {
    let offset_capture = (flags & PREG_OFFSET_CAPTURE) != 0;
    let unmatched_null = (flags & PREG_UNMATCHED_AS_NULL) != 0;
    let num_groups = re.captures_len();
    let mut arr = PhpArray::with_capacity(num_groups * 2);
    let capture_names: Vec<Option<&str>> = re.capture_names().collect();

    let max_matched_group = (0..num_groups).rev().find(|&i| caps.get(i).is_some()).unwrap_or(0);

    for i in 0..num_groups {
        if let Some(m) = caps.get(i) {
            let val = if offset_capture {
                let mut pair = PhpArray::with_capacity(2);
                let s_boxed = crate::into_raw(Box::new(m.as_str().to_string()));
                pair.insert_int(0, Value::new_string_ptr(s_boxed as *mut ()));
                pair.insert_int(1, Value::new_int((m.start() + base_offset) as i32));
                Value::new_array_ptr(ctx.get_arena().alloc(pair) as *mut ())
            } else {
                let s_boxed = crate::into_raw(Box::new(m.as_str().to_string()));
                Value::new_string_ptr(s_boxed as *mut ())
            };
            arr.insert_int(i as i64, val);
            if let Some(Some(name)) = capture_names.get(i) {
                let id = ctx.intern_string(name);
                arr.insert_string_id(id, val);
            }
        } else if unmatched_null {
            let val = if offset_capture {
                let mut pair = PhpArray::with_capacity(2);
                pair.insert_int(0, Value::null());
                pair.insert_int(1, Value::new_int(-1));
                Value::new_array_ptr(ctx.get_arena().alloc(pair) as *mut ())
            } else {
                Value::null()
            };
            arr.insert_int(i as i64, val);
            if let Some(Some(name)) = capture_names.get(i) {
                let id = ctx.intern_string(name);
                arr.insert_string_id(id, val);
            }
        } else if i <= max_matched_group {
            let val = if offset_capture {
                let mut pair = PhpArray::with_capacity(2);
                let s_boxed = crate::into_raw(Box::new(String::new()));
                pair.insert_int(0, Value::new_string_ptr(s_boxed as *mut ()));
                pair.insert_int(1, Value::new_int(-1));
                Value::new_array_ptr(ctx.get_arena().alloc(pair) as *mut ())
            } else {
                let s_boxed = crate::into_raw(Box::new(String::new()));
                Value::new_string_ptr(s_boxed as *mut ())
            };
            arr.insert_int(i as i64, val);
            if let Some(Some(name)) = capture_names.get(i) {
                let id = ctx.intern_string(name);
                arr.insert_string_id(id, val);
            }
        }
    }

    let ptr = ctx.get_arena().alloc(arr);
    Value::new_array_ptr(ptr as *mut ())
}

// -------------------------------------------------------------
// Native Function Handlers
// -------------------------------------------------------------

pub fn native_preg_match(args: &[Value], ctx: &mut dyn NativeContext) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("preg_match() expects at least 2 parameters".to_string());
    }

    let flags = if args.len() > 3 {
        args[3].deref().as_int().map(|i| i as i64).unwrap_or(0)
    } else {
        0
    };

    let offset = if args.len() > 4 {
        args[4].deref().as_int().map(|i| i as usize).unwrap_or(0)
    } else {
        0
    };

    let res = with_value_str(&args[0], |pat_str| {
        with_value_str(&args[1], |subj_str| {
            if offset > subj_str.len() {
                clear_preg_error();
                if args.len() > 2 {
                    let empty_arr = ctx.get_arena().alloc(PhpArray::new());
                    write_out_var(&args[2], Value::new_array_ptr(empty_arr as *mut ()), ctx);
                }
                return Ok(Value::new_int(0));
            }

            if !subj_str.is_char_boundary(offset) {
                set_preg_error(PREG_BAD_UTF8_OFFSET_ERROR, "Offset does not correspond to the beginning of a valid UTF-8 code point");
                return Ok(Value::new_bool(false));
            }

            let re = match get_or_compile_regex(pat_str) {
                Ok(re) => re,
                Err((code, msg)) => {
                    set_preg_error(code, &msg);
                    return Ok(Value::new_bool(false));
                }
            };

            let (match_target, base_offset) = if offset > 0 {
                (&subj_str[offset..], offset)
            } else {
                (subj_str, 0)
            };

            let mut search_pos = 0;
            let mut found_caps = None;
            while search_pos <= match_target.len() {
                match re.captures_from_pos(match_target, search_pos) {
                    Ok(Some(caps)) => {
                        if let Some(skip_m) = caps.name("__skip__") {
                            search_pos = if skip_m.end() > search_pos { skip_m.end() } else { next_char_boundary(match_target, search_pos + 1) };
                            if search_pos > match_target.len() {
                                break;
                            }
                            continue;
                        }
                        found_caps = Some(caps);
                        break;
                    }
                    Ok(None) => break,
                    Err(e) => {
                        let (code, msg) = error_to_code(&e);
                        set_preg_error(code, msg);
                        return Ok(Value::new_bool(false));
                    }
                }
            }

            if let Some(caps) = found_caps {
                clear_preg_error();
                if args.len() > 2 {
                    let matches_val = build_single_matches_array(&caps, &re, flags, base_offset, ctx);
                    write_out_var(&args[2], matches_val, ctx);
                }
                Ok(Value::new_int(1))
            } else {
                clear_preg_error();
                if args.len() > 2 {
                    let empty_arr = ctx.get_arena().alloc(PhpArray::new());
                    write_out_var(&args[2], Value::new_array_ptr(empty_arr as *mut ()), ctx);
                }
                Ok(Value::new_int(0))
            }
        })
    });

    match res {
        Some(Some(val)) => val,
        _ => {
            set_preg_error(PREG_INTERNAL_ERROR, "Invalid pattern or subject type");
            Ok(Value::new_bool(false))
        }
    }
}


pub fn native_preg_match_all(args: &[Value], ctx: &mut dyn NativeContext) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("preg_match_all() expects at least 2 parameters".to_string());
    }

    let Some(pat_str) = value_to_string(&args[0]) else {
        set_preg_error(PREG_INTERNAL_ERROR, "Invalid pattern type");
        return Ok(Value::new_bool(false));
    };

    let Some(subj_str) = value_to_string(&args[1]) else {
        set_preg_error(PREG_INTERNAL_ERROR, "Invalid subject type");
        return Ok(Value::new_bool(false));
    };

    let flags = if args.len() > 3 {
        args[3].deref().as_int().map(|i| i as i64).unwrap_or(PREG_PATTERN_ORDER)
    } else {
        PREG_PATTERN_ORDER
    };

    let offset = if args.len() > 4 {
        args[4].deref().as_int().map(|i| i as usize).unwrap_or(0)
    } else {
        0
    };

    if offset > subj_str.len() {
        clear_preg_error();
        if args.len() > 2 {
            let empty_arr = ctx.get_arena().alloc(PhpArray::new());
            write_out_var(&args[2], Value::new_array_ptr(empty_arr as *mut ()), ctx);
        }
        return Ok(Value::new_int(0));
    }

    if !subj_str.is_char_boundary(offset) {
        set_preg_error(PREG_BAD_UTF8_OFFSET_ERROR, "Offset does not correspond to the beginning of a valid UTF-8 code point");
        return Ok(Value::new_bool(false));
    }

    let re = match get_or_compile_regex(&pat_str) {
        Ok(re) => re,
        Err((code, msg)) => {
            set_preg_error(code, &msg);
            return Ok(Value::new_bool(false));
        }
    };

    let mut all_caps = Vec::new();
    let mut current_pos = offset;

    loop {
        match re.captures_from_pos(subj_str.as_str(), current_pos) {
            Ok(Some(caps)) => {
                if let Some(skip_m) = caps.name("__skip__") {
                    current_pos = if skip_m.end() > current_pos { skip_m.end() } else { next_char_boundary(subj_str.as_str(), current_pos + 1) };
                    if current_pos > subj_str.len() {
                        break;
                    }
                    continue;
                }
                let m0 = caps.name("__match__").or_else(|| caps.get(0)).unwrap();
                let m_start = m0.start();
                let m_end = m0.end();
                all_caps.push(caps);
                if m_end > m_start {
                    current_pos = m_end;
                } else {
                    current_pos = next_char_boundary(subj_str.as_str(), m_start + 1);
                }
                if current_pos > subj_str.len() {
                    break;
                }
            }
            Ok(None) => break,
            Err(e) => {
                let (code, msg) = error_to_code(&e);
                set_preg_error(code, msg);
                return Ok(Value::new_bool(false));
            }
        }
    }

    clear_preg_error();
    let count = all_caps.len();

    if args.len() > 2 {
        let is_set_order = (flags & PREG_SET_ORDER) != 0;
        let offset_capture = (flags & PREG_OFFSET_CAPTURE) != 0;
        let unmatched_null = (flags & PREG_UNMATCHED_AS_NULL) != 0;
        let num_groups = re.captures_len();
        let capture_names: Vec<Option<&str>> = re.capture_names().collect();

        if is_set_order {
            let mut outer_arr = PhpArray::new();
            for (match_idx, caps) in all_caps.iter().enumerate() {
                let single_match_val = build_single_matches_array(caps, &re, flags, 0, ctx);
                outer_arr.insert_int(match_idx as i64, single_match_val);
            }
            let res_val = Value::new_array_ptr(ctx.get_arena().alloc(outer_arr) as *mut ());
            write_out_var(&args[2], res_val, ctx);
        } else {
            // PREG_PATTERN_ORDER
            let mut outer_arr = PhpArray::new();
            for group_idx in 0..num_groups {
                let mut group_arr = PhpArray::new();
                for (match_idx, caps) in all_caps.iter().enumerate() {
                    let val = if let Some(cap) = caps.get(group_idx) {
                        if offset_capture {
                            let mut pair = PhpArray::new();
                            let s_boxed = crate::into_raw(Box::new(cap.as_str().to_string()));
                            pair.insert_int(0, Value::new_string_ptr(s_boxed as *mut ()));
                            pair.insert_int(1, Value::new_int(cap.start() as i32));
                            Value::new_array_ptr(ctx.get_arena().alloc(pair) as *mut ())
                        } else {
                            let s_boxed = crate::into_raw(Box::new(cap.as_str().to_string()));
                            Value::new_string_ptr(s_boxed as *mut ())
                        }
                    } else if unmatched_null {
                        if offset_capture {
                            let mut pair = PhpArray::new();
                            pair.insert_int(0, Value::null());
                            pair.insert_int(1, Value::new_int(-1));
                            Value::new_array_ptr(ctx.get_arena().alloc(pair) as *mut ())
                        } else {
                            Value::null()
                        }
                    } else {
                        if offset_capture {
                            let mut pair = PhpArray::new();
                            let s_boxed = crate::into_raw(Box::new(String::new()));
                            pair.insert_int(0, Value::new_string_ptr(s_boxed as *mut ()));
                            pair.insert_int(1, Value::new_int(-1));
                            Value::new_array_ptr(ctx.get_arena().alloc(pair) as *mut ())
                        } else {
                            let s_boxed = crate::into_raw(Box::new(String::new()));
                            Value::new_string_ptr(s_boxed as *mut ())
                        }
                    };
                    group_arr.insert_int(match_idx as i64, val);
                }
                let group_val = Value::new_array_ptr(ctx.get_arena().alloc(group_arr) as *mut ());
                outer_arr.insert_int(group_idx as i64, group_val);
                if let Some(Some(name)) = capture_names.get(group_idx) {
                    let id = ctx.intern_string(name);
                    outer_arr.insert_string_id(id, group_val);
                }
            }
            let res_val = Value::new_array_ptr(ctx.get_arena().alloc(outer_arr) as *mut ());
            write_out_var(&args[2], res_val, ctx);
        }
    }

    Ok(Value::new_int(count as i32))
}

fn replace_in_string(
    re: &Regex,
    replacement: &str,
    subject: &str,
    limit: i64,
    count: &mut i64,
) -> Result<String, (i32, String)> {
    let mut result = String::with_capacity(subject.len());
    let mut last_end = 0;
    let mut current_pos = 0;
    let mut replaces_done = 0;

    loop {
        if limit >= 0 && replaces_done >= limit {
            break;
        }

        match re.captures_from_pos(subject, current_pos) {
            Ok(Some(caps)) => {
                let m0 = caps.get(0).unwrap();
                let m_start = m0.start();
                let m_end = m0.end();

                result.push_str(&subject[last_end..m_start]);
                let expanded = expand_replacement(replacement, &caps);
                result.push_str(&expanded);

                last_end = m_end;
                replaces_done += 1;
                *count += 1;

                if m_end > m_start {
                    current_pos = m_end;
                } else {
                    current_pos = next_char_boundary(subject, m_start + 1);
                }
                if current_pos > subject.len() {
                    break;
                }
            }
            Ok(None) => break,
            Err(e) => {
                let (code, msg) = error_to_code(&e);
                return Err((code, msg.to_string()));
            }
        }
    }

    result.push_str(&subject[last_end..]);
    Ok(result)
}

fn apply_replacements_to_string(
    pairs: &[(Arc<Regex>, String)],
    subject: &str,
    limit: i64,
    count: &mut i64,
) -> Result<String, (i32, String)> {
    let mut cur = subject.to_string();
    for (re, rep) in pairs {
        cur = replace_in_string(re, rep, &cur, limit, count)?;
    }
    Ok(cur)
}

pub fn native_preg_replace(args: &[Value], ctx: &mut dyn NativeContext) -> Result<Value, String> {
    if args.len() < 3 {
        return Err("preg_replace() expects at least 3 parameters".to_string());
    }

    let pat_val = args[0].deref();
    let rep_val = args[1].deref();
    let subj_val = args[2].deref();

    let limit = if args.len() > 3 {
        args[3].deref().as_int().map(|i| i as i64).unwrap_or(-1)
    } else {
        -1
    };

    let mut pairs = Vec::new();
    if let Some(pat_arr_ptr) = pat_val.as_array_ptr() {
        let pat_arr = unsafe { &*(pat_arr_ptr as *const PhpArray) };
        let rep_arr_opt = rep_val.as_array_ptr().map(|p| unsafe { &*(p as *const PhpArray) });

        for (idx, (_k, p_val)) in pat_arr.elements.iter().enumerate() {
            let Some(p_str) = value_to_string(p_val) else { continue };
            let re = match get_or_compile_regex(&p_str) {
                Ok(re) => re,
                Err((code, msg)) => {
                    set_preg_error(code, &msg);
                    return Ok(Value::null());
                }
            };
            let rep_str = if let Some(rep_arr) = rep_arr_opt {
                rep_arr.get_int(idx as i64).and_then(value_to_string).unwrap_or_default()
            } else {
                value_to_string(&rep_val).unwrap_or_default()
            };
            pairs.push((re, rep_str));
        }
    } else if let Some(p_str) = value_to_string(&pat_val) {
        let re = match get_or_compile_regex(&p_str) {
            Ok(re) => re,
            Err((code, msg)) => {
                set_preg_error(code, &msg);
                return Ok(Value::null());
            }
        };
        let rep_str = value_to_string(&rep_val).unwrap_or_default();
        pairs.push((re, rep_str));
    } else {
        set_preg_error(PREG_INTERNAL_ERROR, "Invalid pattern parameter");
        return Ok(Value::null());
    }

    let mut total_count = 0;
    clear_preg_error();

    let result_val = if let Some(subj_arr_ptr) = subj_val.as_array_ptr() {
        let subj_arr = unsafe { &*(subj_arr_ptr as *const PhpArray) };
        let mut new_arr = PhpArray::new();
        for (k, v) in &subj_arr.elements {
            if let Some(s) = value_to_string(v) {
                match apply_replacements_to_string(&pairs, &s, limit, &mut total_count) {
                    Ok(new_s) => {
                        let boxed = crate::into_raw(Box::new(new_s));
                        let val = Value::new_string_ptr(boxed as *mut ());
                        match k {
                            ArrayKey::Int(i) => new_arr.insert_int(*i, val),
                            ArrayKey::StringId(s) => new_arr.insert_string_id(*s, val),
                        }
                    }
                    Err((code, msg)) => {
                        set_preg_error(code, &msg);
                        return Ok(Value::null());
                    }
                }
            } else {
                match k {
                    ArrayKey::Int(i) => new_arr.insert_int(*i, *v),
                    ArrayKey::StringId(s) => new_arr.insert_string_id(*s, *v),
                }
            }
        }
        Value::new_array_ptr(ctx.get_arena().alloc(new_arr) as *mut ())
    } else if let Some(s) = value_to_string(&subj_val) {
        match apply_replacements_to_string(&pairs, &s, limit, &mut total_count) {
            Ok(new_s) => {
                let boxed = crate::into_raw(Box::new(new_s));
                Value::new_string_ptr(boxed as *mut ())
            }
            Err((code, msg)) => {
                set_preg_error(code, &msg);
                return Ok(Value::null());
            }
        }
    } else {
        set_preg_error(PREG_INTERNAL_ERROR, "Invalid subject parameter");
        return Ok(Value::null());
    };

    if args.len() > 4 {
        write_out_var(&args[4], Value::new_int(total_count as i32), ctx);
    }

    Ok(result_val)
}

fn replace_callback_in_string(
    re: &Regex,
    callback: Value,
    subject: &str,
    limit: i64,
    flags: i64,
    count: &mut i64,
    ctx: &mut dyn NativeContext,
) -> Result<String, (i32, String)> {
    let mut result = String::with_capacity(subject.len());
    let mut last_end = 0;
    let mut current_pos = 0;
    let mut replaces_done = 0;

    loop {
        if limit >= 0 && replaces_done >= limit {
            break;
        }

        match re.captures_from_pos(subject, current_pos) {
            Ok(Some(caps)) => {
                let m0 = caps.get(0).unwrap();
                let m_start = m0.start();
                let m_end = m0.end();

                result.push_str(&subject[last_end..m_start]);
                let matches_val = build_single_matches_array(&caps, re, flags, 0, ctx);

                let cb_ret = ctx.call_callable_synchronously(callback, vec![matches_val])
                    .map_err(|e| (PREG_INTERNAL_ERROR, e))?;

                let rep_str = value_to_string(&cb_ret).unwrap_or_default();
                result.push_str(&rep_str);

                last_end = m_end;
                replaces_done += 1;
                *count += 1;

                if m_end > m_start {
                    current_pos = m_end;
                } else {
                    current_pos = next_char_boundary(subject, m_start + 1);
                }
                if current_pos > subject.len() {
                    break;
                }
            }
            Ok(None) => break,
            Err(e) => {
                let (code, msg) = error_to_code(&e);
                return Err((code, msg.to_string()));
            }
        }
    }

    result.push_str(&subject[last_end..]);
    Ok(result)
}

pub fn native_preg_replace_callback(args: &[Value], ctx: &mut dyn NativeContext) -> Result<Value, String> {
    if args.len() < 3 {
        return Err("preg_replace_callback() expects at least 3 parameters".to_string());
    }

    let pat_val = args[0].deref();
    let cb_val = args[1].deref();
    let subj_val = args[2].deref();

    let limit = if args.len() > 3 {
        args[3].deref().as_int().map(|i| i as i64).unwrap_or(-1)
    } else {
        -1
    };

    let flags = if args.len() > 5 {
        args[5].deref().as_int().map(|i| i as i64).unwrap_or(0)
    } else {
        0
    };

    let mut patterns = Vec::new();
    if let Some(pat_arr_ptr) = pat_val.as_array_ptr() {
        let pat_arr = unsafe { &*(pat_arr_ptr as *const PhpArray) };
        for (_k, p_val) in &pat_arr.elements {
            if let Some(p_str) = value_to_string(p_val) {
                let re = match get_or_compile_regex(&p_str) {
                    Ok(re) => re,
                    Err((code, msg)) => {
                        set_preg_error(code, &msg);
                        return Ok(Value::null());
                    }
                };
                patterns.push(re);
            }
        }
    } else if let Some(p_str) = value_to_string(&pat_val) {
        let re = match get_or_compile_regex(&p_str) {
            Ok(re) => re,
            Err((code, msg)) => {
                set_preg_error(code, &msg);
                return Ok(Value::null());
            }
        };
        patterns.push(re);
    } else {
        set_preg_error(PREG_INTERNAL_ERROR, "Invalid pattern parameter");
        return Ok(Value::null());
    }

    let mut total_count = 0;
    clear_preg_error();

    let result_val = if let Some(subj_arr_ptr) = subj_val.as_array_ptr() {
        let subj_arr = unsafe { &*(subj_arr_ptr as *const PhpArray) };
        let mut new_arr = PhpArray::new();
        for (k, v) in &subj_arr.elements {
            if let Some(mut s) = value_to_string(v) {
                let mut err = None;
                for re in &patterns {
                    match replace_callback_in_string(re, cb_val, &s, limit, flags, &mut total_count, ctx) {
                        Ok(new_s) => s = new_s,
                        Err(e) => { err = Some(e); break; }
                    }
                }
                if let Some((code, msg)) = err {
                    set_preg_error(code, &msg);
                    return Ok(Value::null());
                }
                let boxed = crate::into_raw(Box::new(s));
                let val = Value::new_string_ptr(boxed as *mut ());
                match k {
                    ArrayKey::Int(i) => new_arr.insert_int(*i, val),
                    ArrayKey::StringId(s) => new_arr.insert_string_id(*s, val),
                }
            } else {
                match k {
                    ArrayKey::Int(i) => new_arr.insert_int(*i, *v),
                    ArrayKey::StringId(s) => new_arr.insert_string_id(*s, *v),
                }
            }
        }
        Value::new_array_ptr(ctx.get_arena().alloc(new_arr) as *mut ())
    } else if let Some(mut s) = value_to_string(&subj_val) {
        let mut err = None;
        for re in &patterns {
            match replace_callback_in_string(re, cb_val, &s, limit, flags, &mut total_count, ctx) {
                Ok(new_s) => s = new_s,
                Err(e) => { err = Some(e); break; }
            }
        }
        if let Some((code, msg)) = err {
            set_preg_error(code, &msg);
            return Ok(Value::null());
        }
        let boxed = crate::into_raw(Box::new(s));
        Value::new_string_ptr(boxed as *mut ())
    } else {
        set_preg_error(PREG_INTERNAL_ERROR, "Invalid subject parameter");
        return Ok(Value::null());
    };

    if args.len() > 4 {
        write_out_var(&args[4], Value::new_int(total_count as i32), ctx);
    }

    Ok(result_val)
}

pub fn native_preg_replace_callback_array(args: &[Value], ctx: &mut dyn NativeContext) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("preg_replace_callback_array() expects at least 2 parameters".to_string());
    }

    let map_val = args[0].deref();
    let subj_val = args[1].deref();

    let limit = if args.len() > 2 {
        args[2].deref().as_int().map(|i| i as i64).unwrap_or(-1)
    } else {
        -1
    };

    let flags = if args.len() > 4 {
        args[4].deref().as_int().map(|i| i as i64).unwrap_or(0)
    } else {
        0
    };

    let Some(map_arr_ptr) = map_val.as_array_ptr() else {
        set_preg_error(PREG_INTERNAL_ERROR, "preg_replace_callback_array() expects parameter 1 to be an array");
        return Ok(Value::null());
    };

    let map_arr = unsafe { &*(map_arr_ptr as *const PhpArray) };
    let mut pairs = Vec::new();

    for (k, cb_val) in &map_arr.elements {
        let pat_str = match k {
            ArrayKey::StringId(id) => ctx.lookup_string(*id).unwrap_or_default(),
            ArrayKey::Int(i) => i.to_string(),
        };
        let re = match get_or_compile_regex(&pat_str) {
            Ok(re) => re,
            Err((code, msg)) => {
                set_preg_error(code, &msg);
                return Ok(Value::null());
            }
        };
        pairs.push((re, *cb_val));
    }

    let mut total_count = 0;
    clear_preg_error();

    let result_val = if let Some(subj_arr_ptr) = subj_val.as_array_ptr() {
        let subj_arr = unsafe { &*(subj_arr_ptr as *const PhpArray) };
        let mut new_arr = PhpArray::new();
        for (k, v) in &subj_arr.elements {
            if let Some(mut s) = value_to_string(v) {
                let mut err = None;
                for (re, cb) in &pairs {
                    match replace_callback_in_string(re, *cb, &s, limit, flags, &mut total_count, ctx) {
                        Ok(new_s) => s = new_s,
                        Err(e) => { err = Some(e); break; }
                    }
                }
                if let Some((code, msg)) = err {
                    set_preg_error(code, &msg);
                    return Ok(Value::null());
                }
                let boxed = crate::into_raw(Box::new(s));
                let val = Value::new_string_ptr(boxed as *mut ());
                match k {
                    ArrayKey::Int(i) => new_arr.insert_int(*i, val),
                    ArrayKey::StringId(s) => new_arr.insert_string_id(*s, val),
                }
            } else {
                match k {
                    ArrayKey::Int(i) => new_arr.insert_int(*i, *v),
                    ArrayKey::StringId(s) => new_arr.insert_string_id(*s, *v),
                }
            }
        }
        Value::new_array_ptr(ctx.get_arena().alloc(new_arr) as *mut ())
    } else if let Some(mut s) = value_to_string(&subj_val) {
        let mut err = None;
        for (re, cb) in &pairs {
            match replace_callback_in_string(re, *cb, &s, limit, flags, &mut total_count, ctx) {
                Ok(new_s) => s = new_s,
                Err(e) => { err = Some(e); break; }
            }
        }
        if let Some((code, msg)) = err {
            set_preg_error(code, &msg);
            return Ok(Value::null());
        }
        let boxed = crate::into_raw(Box::new(s));
        Value::new_string_ptr(boxed as *mut ())
    } else {
        set_preg_error(PREG_INTERNAL_ERROR, "Invalid subject parameter");
        return Ok(Value::null());
    };

    if args.len() > 3 {
        write_out_var(&args[3], Value::new_int(total_count as i32), ctx);
    }

    Ok(result_val)
}

pub fn native_preg_filter(args: &[Value], ctx: &mut dyn NativeContext) -> Result<Value, String> {
    if args.len() < 3 {
        return Err("preg_filter() expects at least 3 parameters".to_string());
    }

    let pat_val = args[0].deref();
    let rep_val = args[1].deref();
    let subj_val = args[2].deref();

    let limit = if args.len() > 3 {
        args[3].deref().as_int().map(|i| i as i64).unwrap_or(-1)
    } else {
        -1
    };

    let mut pairs = Vec::new();
    if let Some(pat_arr_ptr) = pat_val.as_array_ptr() {
        let pat_arr = unsafe { &*(pat_arr_ptr as *const PhpArray) };
        let rep_arr_opt = rep_val.as_array_ptr().map(|p| unsafe { &*(p as *const PhpArray) });

        for (idx, (_k, p_val)) in pat_arr.elements.iter().enumerate() {
            let Some(p_str) = value_to_string(p_val) else { continue };
            let re = match get_or_compile_regex(&p_str) {
                Ok(re) => re,
                Err((code, msg)) => {
                    set_preg_error(code, &msg);
                    return Ok(Value::null());
                }
            };
            let rep_str = if let Some(rep_arr) = rep_arr_opt {
                rep_arr.get_int(idx as i64).and_then(value_to_string).unwrap_or_default()
            } else {
                value_to_string(&rep_val).unwrap_or_default()
            };
            pairs.push((re, rep_str));
        }
    } else if let Some(p_str) = value_to_string(&pat_val) {
        let re = match get_or_compile_regex(&p_str) {
            Ok(re) => re,
            Err((code, msg)) => {
                set_preg_error(code, &msg);
                return Ok(Value::null());
            }
        };
        let rep_str = value_to_string(&rep_val).unwrap_or_default();
        pairs.push((re, rep_str));
    } else {
        set_preg_error(PREG_INTERNAL_ERROR, "Invalid pattern parameter");
        return Ok(Value::null());
    }

    let mut total_count = 0;
    clear_preg_error();

    let result_val = if let Some(subj_arr_ptr) = subj_val.as_array_ptr() {
        let subj_arr = unsafe { &*(subj_arr_ptr as *const PhpArray) };
        let mut new_arr = PhpArray::new();
        for (k, v) in &subj_arr.elements {
            if let Some(s) = value_to_string(v) {
                let mut elem_count = 0;
                match apply_replacements_to_string(&pairs, &s, limit, &mut elem_count) {
                    Ok(new_s) => {
                        if elem_count > 0 {
                            total_count += elem_count;
                            let boxed = crate::into_raw(Box::new(new_s));
                            let val = Value::new_string_ptr(boxed as *mut ());
                            match k {
                                ArrayKey::Int(i) => new_arr.insert_int(*i, val),
                                ArrayKey::StringId(s) => new_arr.insert_string_id(*s, val),
                            }
                        }
                    }
                    Err((code, msg)) => {
                        set_preg_error(code, &msg);
                        return Ok(Value::null());
                    }
                }
            }
        }
        Value::new_array_ptr(ctx.get_arena().alloc(new_arr) as *mut ())
    } else if let Some(s) = value_to_string(&subj_val) {
        match apply_replacements_to_string(&pairs, &s, limit, &mut total_count) {
            Ok(new_s) => {
                if total_count > 0 {
                    let boxed = crate::into_raw(Box::new(new_s));
                    Value::new_string_ptr(boxed as *mut ())
                } else {
                    Value::null()
                }
            }
            Err((code, msg)) => {
                set_preg_error(code, &msg);
                return Ok(Value::null());
            }
        }
    } else {
        set_preg_error(PREG_INTERNAL_ERROR, "Invalid subject parameter");
        return Ok(Value::null());
    };

    if args.len() > 4 {
        write_out_var(&args[4], Value::new_int(total_count as i32), ctx);
    }

    Ok(result_val)
}

pub fn native_preg_split(args: &[Value], ctx: &mut dyn NativeContext) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("preg_split() expects at least 2 parameters".to_string());
    }

    let Some(pat_str) = value_to_string(&args[0]) else {
        set_preg_error(PREG_INTERNAL_ERROR, "Invalid pattern type");
        return Ok(Value::new_bool(false));
    };

    let Some(subj_str) = value_to_string(&args[1]) else {
        set_preg_error(PREG_INTERNAL_ERROR, "Invalid subject type");
        return Ok(Value::new_bool(false));
    };

    let limit = if args.len() > 2 {
        args[2].deref().as_int().map(|i| i as i64).unwrap_or(-1)
    } else {
        -1
    };

    let flags = if args.len() > 3 {
        args[3].deref().as_int().map(|i| i as i64).unwrap_or(0)
    } else {
        0
    };

    let no_empty = (flags & PREG_SPLIT_NO_EMPTY) != 0;
    let delim_capture = (flags & PREG_SPLIT_DELIM_CAPTURE) != 0;
    let offset_capture = (flags & PREG_SPLIT_OFFSET_CAPTURE) != 0;

    let re = match get_or_compile_regex(&pat_str) {
        Ok(re) => re,
        Err((code, msg)) => {
            set_preg_error(code, &msg);
            return Ok(Value::new_bool(false));
        }
    };

    let mut result_arr = PhpArray::new();
    let mut last_end = 0;
    let mut current_pos = 0;
    let mut pieces_count: usize = 0;
    let max_pieces = if limit > 0 { limit as usize } else { usize::MAX };

    let add_piece = |arr: &mut PhpArray, text: &str, offset: usize, ctx: &mut dyn NativeContext| {
        let val = if offset_capture {
            let mut pair = PhpArray::new();
            let boxed = crate::into_raw(Box::new(text.to_string()));
            pair.insert_int(0, Value::new_string_ptr(boxed as *mut ()));
            pair.insert_int(1, Value::new_int(offset as i32));
            Value::new_array_ptr(ctx.get_arena().alloc(pair) as *mut ())
        } else {
            let boxed = crate::into_raw(Box::new(text.to_string()));
            Value::new_string_ptr(boxed as *mut ())
        };
        arr.insert_int(arr.len() as i64, val);
    };

    while pieces_count + 1 < max_pieces {
        match re.captures_from_pos(subj_str.as_str(), current_pos) {
            Ok(Some(caps)) => {
                if let Some(skip_m) = caps.name("__skip__") {
                    current_pos = if skip_m.end() > current_pos { skip_m.end() } else { next_char_boundary(subj_str.as_str(), current_pos + 1) };
                    if current_pos > subj_str.len() {
                        break;
                    }
                    continue;
                }
                let m0 = caps.name("__match__").or_else(|| caps.get(0)).unwrap();
                let m_start = m0.start();
                let m_end = m0.end();

                let piece = &subj_str[last_end..m_start];
                if !no_empty || !piece.is_empty() {
                    add_piece(&mut result_arr, piece, last_end, ctx);
                    pieces_count += 1;
                }

                if delim_capture {
                    for g in 1..caps.len() {
                        if let Some(gm) = caps.get(g) {
                            if !no_empty || !gm.as_str().is_empty() {
                                add_piece(&mut result_arr, gm.as_str(), gm.start(), ctx);
                            }
                        }
                    }
                }

                last_end = m_end;
                if m_end > m_start {
                    current_pos = m_end;
                } else {
                    current_pos = next_char_boundary(subj_str.as_str(), m_start + 1);
                }
                if current_pos > subj_str.len() {
                    break;
                }
            }
            Ok(None) => break,
            Err(e) => {
                let (code, msg) = error_to_code(&e);
                set_preg_error(code, msg);
                return Ok(Value::new_bool(false));
            }
        }
    }

    let remainder = &subj_str[last_end..];
    if !no_empty || !remainder.is_empty() {
        add_piece(&mut result_arr, remainder, last_end, ctx);
    }

    clear_preg_error();
    Ok(Value::new_array_ptr(ctx.get_arena().alloc(result_arr) as *mut ()))
}

pub fn native_preg_grep(args: &[Value], ctx: &mut dyn NativeContext) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("preg_grep() expects at least 2 parameters".to_string());
    }

    let Some(pat_str) = value_to_string(&args[0]) else {
        set_preg_error(PREG_INTERNAL_ERROR, "Invalid pattern type");
        return Ok(Value::new_bool(false));
    };

    let Some(arr_ptr) = args[1].deref().as_array_ptr() else {
        set_preg_error(PREG_INTERNAL_ERROR, "preg_grep() expects parameter 2 to be array");
        return Ok(Value::new_bool(false));
    };

    let flags = if args.len() > 2 {
        args[2].deref().as_int().map(|i| i as i64).unwrap_or(0)
    } else {
        0
    };

    let invert = (flags & PREG_GREP_INVERT) != 0;

    let re = match get_or_compile_regex(&pat_str) {
        Ok(re) => re,
        Err((code, msg)) => {
            set_preg_error(code, &msg);
            return Ok(Value::new_bool(false));
        }
    };

    let arr = unsafe { &*(arr_ptr as *const PhpArray) };
    let mut result_arr = PhpArray::new();

    for (k, v) in &arr.elements {
        if let Some(s) = value_to_string(v) {
            match re.is_match(&s) {
                Ok(is_match) => {
                    if is_match ^ invert {
                        match k {
                            ArrayKey::Int(i) => result_arr.insert_int(*i, *v),
                            ArrayKey::StringId(s_id) => result_arr.insert_string_id(*s_id, *v),
                        }
                    }
                }
                Err(e) => {
                    let (code, msg) = error_to_code(&e);
                    set_preg_error(code, msg);
                    return Ok(Value::new_bool(false));
                }
            }
        }
    }

    clear_preg_error();
    Ok(Value::new_array_ptr(ctx.get_arena().alloc(result_arr) as *mut ()))
}

pub fn native_preg_quote(args: &[Value], _ctx: &mut dyn NativeContext) -> Result<Value, String> {
    if args.is_empty() {
        return Err("preg_quote() expects at least 1 parameter".to_string());
    }

    let Some(s) = value_to_string(&args[0]) else {
        return Err("preg_quote() expects parameter 1 to be string".to_string());
    };

    let delimiter = if args.len() > 1 {
        value_to_string(&args[1]).and_then(|d| d.chars().next())
    } else {
        None
    };

    let special = r".\+*?[^]$(){}=!<>|:-#";
    let mut result = String::with_capacity(s.len() * 2);

    for c in s.chars() {
        if special.contains(c) || delimiter == Some(c) {
            result.push('\\');
        }
        result.push(c);
    }

    let boxed = crate::into_raw(Box::new(result));
    Ok(Value::new_string_ptr(boxed as *mut ()))
}

pub fn native_preg_last_error(_args: &[Value], _ctx: &mut dyn NativeContext) -> Result<Value, String> {
    let code = LAST_PREG_ERROR.with(|e| e.get());
    Ok(Value::new_int(code))
}

pub fn native_preg_last_error_msg(_args: &[Value], _ctx: &mut dyn NativeContext) -> Result<Value, String> {
    let msg = LAST_PREG_ERROR_MSG.with(|m| m.borrow().clone());
    let boxed = crate::into_raw(Box::new(msg));
    Ok(Value::new_string_ptr(boxed as *mut ()))
}
