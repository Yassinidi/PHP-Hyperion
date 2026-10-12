//! The `iconv` family.
//!
//! Symfony's mbstring polyfill is written on top of these: with `mbstring`
//! absent it defines `mb_convert_encoding()` and friends as thin wrappers that
//! call `iconv()`. phpdotenv routes every `.env` byte through
//! `mb_convert_encoding()`, so with no `iconv` here Laravel could not read its
//! environment at all.
//!
//! A note on what conversion can mean in this engine. A PHP string is a byte
//! string; here it is a Rust `String`, which is always valid UTF-8. A byte
//! sequence in a single-byte charset is therefore already decoded to code
//! points by the time it reaches a native — there are no raw `0xE9` bytes left
//! to reinterpret. What remains for `iconv()` to decide is whether the *target*
//! charset can represent each character, and that is what these functions
//! implement: conversions *to* UTF-8 are the identity, and conversions to a
//! narrower charset drop, transliterate, or fail per the `//IGNORE` and
//! `//TRANSLIT` flags, exactly as PHP does.

use hyperion_core::memory::nan_box::Value;
use hyperion_core::php_function;

/// A charset this engine recognises.
///
/// Only the width matters — see the module note — so charsets that agree on
/// which code points they can hold share a variant.
#[derive(Clone, Copy, PartialEq)]
enum Charset {
    /// Everything is representable.
    Utf8,
    /// 7-bit: code points up to 0x7F.
    Ascii,
    /// One byte per character: code points up to 0xFF. Windows-1252 is treated
    /// as ISO-8859-1 here, so the handful of characters it maps into 0x80-0x9F
    /// (`€`, `„`, `—`) are reported unrepresentable rather than encoded.
    SingleByte,
}

/// The `//IGNORE` and `//TRANSLIT` suffixes of an iconv charset spec.
struct Flags {
    ignore: bool,
    translit: bool,
}

/// Split an iconv charset spec into a charset and its flags.
///
/// `None` for a charset this engine does not know, which is how callers get
/// PHP's `false` return — Symfony probes support with
/// `false !== @iconv($enc, $enc, ' ')`, so guessing here would make it claim
/// charsets that then convert wrongly.
fn parse_spec(spec: &str) -> Option<(Charset, Flags)> {
    let mut flags = Flags {
        ignore: false,
        translit: false,
    };
    let mut name = spec.trim().to_ascii_uppercase();
    // Both suffixes may be present, in either order: `UTF-8//TRANSLIT//IGNORE`.
    loop {
        if let Some(base) = name.strip_suffix("//IGNORE") {
            flags.ignore = true;
            name = base.to_string();
        } else if let Some(base) = name.strip_suffix("//TRANSLIT") {
            flags.translit = true;
            name = base.to_string();
        } else {
            break;
        }
    }
    let charset = match name.as_str() {
        "UTF-8" | "UTF8" => Charset::Utf8,
        "ASCII" | "US-ASCII" | "ISO646-US" | "ANSI_X3.4-1968" => Charset::Ascii,
        "ISO-8859-1" | "ISO8859-1" | "ISO_8859-1" | "LATIN1" | "L1" | "CP819"
        | "WINDOWS-1252" | "CP1252" => Charset::SingleByte,
        _ => return None,
    };
    Some((charset, flags))
}

/// The ASCII stand-in for a character under `//TRANSLIT`, if there is a sensible
/// one.
///
/// glibc's transliteration is locale-dependent; this covers the Latin-1 letters,
/// which is what slug builders actually pass through `iconv('UTF-8',
/// 'ASCII//TRANSLIT', $s)`. Anything else falls back to `?`, as PHP does.
fn transliterate(c: char) -> &'static str {
    match c {
        'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' => "A",
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => "a",
        'Æ' => "AE",
        'æ' => "ae",
        'Ç' => "C",
        'ç' => "c",
        'È' | 'É' | 'Ê' | 'Ë' => "E",
        'è' | 'é' | 'ê' | 'ë' => "e",
        'Ì' | 'Í' | 'Î' | 'Ï' => "I",
        'ì' | 'í' | 'î' | 'ï' => "i",
        'Ñ' => "N",
        'ñ' => "n",
        'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' | 'Ø' => "O",
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' => "o",
        'Ù' | 'Ú' | 'Û' | 'Ü' => "U",
        'ù' | 'ú' | 'û' | 'ü' => "u",
        'Ý' => "Y",
        'ý' | 'ÿ' => "y",
        'Ð' => "D",
        'ð' => "d",
        'Þ' => "TH",
        'þ' => "th",
        'ß' => "ss",
        '×' => "x",
        '÷' => "/",
        _ => "?",
    }
}

/// Can `to` hold this character?
fn representable(c: char, to: Charset) -> bool {
    match to {
        Charset::Utf8 => true,
        Charset::Ascii => (c as u32) <= 0x7F,
        Charset::SingleByte => (c as u32) <= 0xFF,
    }
}

/// The body of `iconv()`. `None` means PHP's `false`.
fn convert(from_spec: &str, to_spec: &str, s: &str) -> Option<String> {
    // The from-charset is validated but otherwise unused: the input is already
    // decoded (see the module note). Rejecting an unknown one still matters —
    // that is how `iconv($enc, $enc, ' ')` reports whether $enc is supported.
    parse_spec(from_spec)?;
    let (to, flags) = parse_spec(to_spec)?;

    if to == Charset::Utf8 {
        return Some(s.to_string());
    }

    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if representable(c, to) {
            out.push(c);
        } else if flags.translit {
            out.push_str(transliterate(c));
        } else if flags.ignore {
            // Dropped.
        } else {
            // PHP warns "Detected an illegal character in input string" and
            // returns false. Truncating at the offending character, which is
            // what iconv() did before PHP 8, would silently corrupt the value.
            return None;
        }
    }
    Some(out)
}

/// Wrap an owned Rust string as a PHP string value.
fn php_str(s: String) -> Value {
    Value::new_string_ptr(crate::into_raw(Box::new(s)) as *mut ())
}

/// Read a `Value` argument as a string, following references.
///
/// The `String` form of `extract_arg!` raises a fatal error when the argument is
/// not a string, but these functions take charset arguments that callers leave
/// null (`iconv_strlen($s, null)` means "the internal charset"), so the softer
/// read is needed.
fn as_str(v: Option<&Value>) -> Option<String> {
    let v = v?.deref();
    if let Some(ptr) = v.as_string_ptr() {
        Some(unsafe { &*(ptr as *const String) }.clone())
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

/// The charset to use when an argument is null or absent.
///
/// `iconv.internal_encoding` is UTF-8 in every modern PHP build, and it is the
/// only charset this engine stores strings in.
const DEFAULT_CHARSET: &str = "UTF-8";

php_function! {
    native_iconv(from: Value, to: Value, string: Value) {
        if from.is_none() || to.is_none() || string.is_none() {
            return Err("iconv() expects at least 3 parameters".to_string());
        }
        let from = as_str(from).unwrap_or_default();
        let to = as_str(to).unwrap_or_default();
        let s = as_str(string).unwrap_or_default();
        match convert(&from, &to, &s) {
            Some(out) => Ok(php_str(out)),
            None => Ok(Value::new_bool(false)),
        }
    }
}

php_function! {
    native_iconv_strlen(string: Value, charset: Value) {
        if string.is_none() {
            return Err("iconv_strlen() expects at least 1 parameter".to_string());
        }
        let s = as_str(string).unwrap_or_default();
        let charset = as_str(charset).filter(|c| !c.is_empty()).unwrap_or_else(|| DEFAULT_CHARSET.to_string());
        if parse_spec(&charset).is_none() {
            return Ok(Value::new_bool(false));
        }
        // Characters, not bytes — that is the whole point of the function.
        Ok(Value::new_int(s.chars().count() as i32))
    }
}

php_function! {
    native_iconv_substr(string: Value, offset: Value, length: Value, charset: Value) {
        if string.is_none() || offset.is_none() {
            return Err("iconv_substr() expects at least 2 parameters".to_string());
        }
        let s = as_str(string).unwrap_or_default();
        let charset = as_str(charset).filter(|c| !c.is_empty()).unwrap_or_else(|| DEFAULT_CHARSET.to_string());
        if parse_spec(&charset).is_none() {
            return Ok(Value::new_bool(false));
        }

        let chars: Vec<char> = s.chars().collect();
        let total = chars.len() as i64;
        let offset = offset.map(|v| v.deref()).and_then(|v| {
            v.as_int().or_else(|| v.as_float().map(|f| f as i32)).or_else(|| {
                if let Some(sp) = v.as_string_ptr() {
                    let s = unsafe { &*(sp as *const String) };
                    s.parse::<i32>().ok()
                } else {
                    None
                }
            })
        }).unwrap_or(0) as i64;
        // A negative offset counts back from the end.
        let start = if offset < 0 {
            (total + offset).max(0)
        } else {
            offset.min(total)
        };

        // A null length means "to the end"; a negative one leaves that many
        // characters off the end.
        let len_val = length.map(|v| v.deref());
        let len = match len_val.filter(|v| !v.is_null()).and_then(|v| {
            v.as_int().or_else(|| v.as_float().map(|f| f as i32)).or_else(|| {
                if let Some(sp) = v.as_string_ptr() {
                    let s = unsafe { &*(sp as *const String) };
                    s.parse::<i32>().ok()
                } else {
                    None
                }
            })
        }) {
            Some(l) if (l as i64) < 0 => (total - start + l as i64).max(0),
            Some(l) => (l as i64).min(total - start),
            None => total - start,
        };

        let out: String = chars[start as usize..(start + len) as usize].iter().collect();
        Ok(php_str(out))
    }
}

php_function! {
    native_iconv_strpos(haystack: Value, needle: Value, offset: Value, charset: Value) {
        if haystack.is_none() || needle.is_none() {
            return Err("iconv_strpos() expects at least 2 parameters".to_string());
        }
        let h = as_str(haystack).unwrap_or_default();
        let n = as_str(needle).unwrap_or_default();
        let charset = as_str(charset).filter(|c| !c.is_empty()).unwrap_or_else(|| DEFAULT_CHARSET.to_string());
        if parse_spec(&charset).is_none() {
            return Ok(Value::new_bool(false));
        }

        let h_chars: Vec<char> = h.chars().collect();
        let n_chars: Vec<char> = n.chars().collect();
        let total = h_chars.len() as i64;
        let offset = offset.map(|v| v.deref()).and_then(|v| {
            v.as_int().or_else(|| v.as_float().map(|f| f as i32)).or_else(|| {
                if let Some(sp) = v.as_string_ptr() {
                    let s = unsafe { &*(sp as *const String) };
                    s.parse::<i32>().ok()
                } else {
                    None
                }
            })
        }).unwrap_or(0) as i64;
        let start = if offset < 0 { (total + offset).max(0) } else { offset };
        if start > total {
            return Ok(Value::new_bool(false));
        }
        // The empty needle matches at the search offset.
        if n_chars.is_empty() {
            return Ok(Value::new_int(start as i32));
        }
        if n_chars.len() as i64 > total - start {
            return Ok(Value::new_bool(false));
        }
        for i in (start as usize)..=(h_chars.len() - n_chars.len()) {
            if h_chars[i..i + n_chars.len()] == n_chars[..] {
                return Ok(Value::new_int(i as i32));
            }
        }
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_iconv_strrpos(haystack: Value, needle: Value, charset: Value) {
        if haystack.is_none() || needle.is_none() {
            return Err("iconv_strrpos() expects at least 2 parameters".to_string());
        }
        let h = as_str(haystack).unwrap_or_default();
        let n = as_str(needle).unwrap_or_default();
        let charset = as_str(charset).filter(|c| !c.is_empty()).unwrap_or_else(|| DEFAULT_CHARSET.to_string());
        if parse_spec(&charset).is_none() {
            return Ok(Value::new_bool(false));
        }

        let h_chars: Vec<char> = h.chars().collect();
        let n_chars: Vec<char> = n.chars().collect();
        // Unlike iconv_strpos(), PHP returns false for an empty needle here.
        if n_chars.is_empty() || n_chars.len() > h_chars.len() {
            return Ok(Value::new_bool(false));
        }
        for i in (0..=(h_chars.len() - n_chars.len())).rev() {
            if h_chars[i..i + n_chars.len()] == n_chars[..] {
                return Ok(Value::new_int(i as i32));
            }
        }
        Ok(Value::new_bool(false))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_roundtrip_is_identity() {
        assert_eq!(convert("UTF-8", "UTF-8", "héllo").unwrap(), "héllo");
        // The support probe Symfony runs must return a string, not false.
        assert_eq!(convert("UTF-8", "UTF-8//IGNORE", "").unwrap(), "");
    }

    #[test]
    fn unknown_charset_is_false() {
        assert!(convert("UTF-8", "EBCDIC-US", "x").is_none());
        assert!(convert("KOI8-R", "UTF-8", "x").is_none());
    }

    #[test]
    fn unrepresentable_character_fails_without_flags() {
        assert!(convert("UTF-8", "ASCII", "é").is_none());
        assert_eq!(convert("UTF-8", "ASCII//IGNORE", "aéb").unwrap(), "ab");
        assert_eq!(convert("UTF-8", "ASCII//TRANSLIT", "aéb").unwrap(), "aeb");
        // No transliteration for a character outside Latin-1.
        assert_eq!(convert("UTF-8", "ASCII//TRANSLIT", "日").unwrap(), "?");
    }

    #[test]
    fn latin1_holds_its_own_range() {
        assert_eq!(convert("UTF-8", "ISO-8859-1", "é").unwrap(), "é");
        assert!(convert("UTF-8", "ISO-8859-1", "日").is_none());
    }

    #[test]
    fn flags_parse_in_either_order() {
        for spec in ["ASCII//IGNORE//TRANSLIT", "ASCII//TRANSLIT//IGNORE"] {
            let (charset, flags) = parse_spec(spec).unwrap();
            assert!(charset == Charset::Ascii);
            assert!(flags.ignore && flags.translit);
        }
    }
}
