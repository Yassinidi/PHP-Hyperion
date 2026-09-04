//! Which parameters of a native function are declared by reference.
//!
//! Native functions have no PHP declaration to scan, so the positions have to
//! be listed. Two places need the same answer and must not drift apart:
//!
//!   * the compiler, deciding whether to emit a reference for an argument, and
//!   * the VM, when it synthesises a thunk for a native reached through
//!     `Opcode::Call` — that thunk's parameter list is what the argument-binding
//!     funnel consults, and an empty list would make it deep-copy array
//!     arguments and quietly hand `sort()` a copy to sort.
//!
//! Hence one table, in the crate both of them already depend on.

/// Positions of `callee`'s by-reference parameters, or `&[]` when it has none.
/// The name may be namespace-qualified — PHP falls back to the global function
/// when a namespaced one does not exist, so only the last segment is matched.
pub fn by_ref_param_positions(callee: &str) -> &'static [usize] {
    let short = callee.rsplit('\\').next().unwrap_or(callee).to_ascii_lowercase();
    match short.as_str() {
        // Out-parameter holding the captured groups.
        "preg_match" | "preg_match_all" => &[2],
        // Mutate the array they are handed or inspect internal pointer without copying.
        "array_push" | "array_pop" | "array_shift" | "array_unshift" | "array_splice"
        | "shuffle" | "sort" | "rsort" | "usort" | "uasort" | "uksort" | "ksort" | "krsort"
        | "asort" | "arsort" | "array_walk" | "array_multisort" | "natsort" | "natcasesort"
        | "current" | "key" | "pos" | "end" | "reset" | "next" | "prev" | "each" | "settype" => &[0],
        // Out-parameter counting replacements.
        "str_replace" | "str_ireplace" => &[3],
        // Same, one position further along: the signature is
        // (pattern, replacement, subject, limit, &count).
        "preg_replace" | "preg_replace_callback" | "preg_filter" => &[4],
        // (pattern_callback_array, subject, limit, &count).
        "preg_replace_callback_array" => &[3],
        // Out-parameter holding the similarity percentage.
        "similar_text" => &[2],
        // Output array for parse_str($str, &$result)
        "parse_str" => &[1],
        // exec($cmd, &$output, &$result_code)
        "exec" => &[1, 2],
        // system($cmd, &$result_code), passthru($cmd, &$result_code)
        "system" | "passthru" => &[1],
        // fsockopen($hostname, $port, &$errno, &$errstr)
        "fsockopen" => &[2, 3],
        // openssl out params
        "openssl_sign" | "openssl_pkey_export" => &[1],
        // proc_open($cmd, $descriptors, &$pipes)
        "proc_open" => &[2],
        // stream_select(&$read, &$write, &$except, $sec, $usec)
        "stream_select" => &[0, 1, 2],
        // BoundMethod / ImplicitlyBoundMethod dependency resolution
        "adddependencyforcallparameter" => &[2, 3],
        "substitutenamebindingforcallparameter" => &[1, 2],
        "substituteimplicitbindingforcallparameter" => &[2],
        _ => &[],
    }
}

use std::sync::OnceLock;
use dashmap::DashMap;

static USERLAND_BY_REF_REGISTRY: OnceLock<DashMap<String, Vec<usize>>> = OnceLock::new();

fn userland_registry() -> &'static DashMap<String, Vec<usize>> {
    USERLAND_BY_REF_REGISTRY.get_or_init(DashMap::new)
}

/// Dynamically register by-reference parameter positions for userland functions/methods.
pub fn register_by_ref_param(callee: &str, positions: Vec<usize>) {
    let short = callee.rsplit('\\').next().unwrap_or(callee).to_ascii_lowercase();
    userland_registry()
        .entry(short)
        .and_modify(|existing| {
            for pos in &positions {
                if !existing.contains(pos) {
                    existing.push(*pos);
                }
            }
        })
        .or_insert(positions);
}

/// Whether argument `index` of `callee` is passed by reference.
pub fn is_by_ref_param(callee: &str, index: usize) -> bool {
    if by_ref_param_positions(callee).contains(&index) {
        return true;
    }
    let short = callee.rsplit('\\').next().unwrap_or(callee).to_ascii_lowercase();
    if let Some(positions) = userland_registry().get(&short) {
        return positions.contains(&index);
    }
    false
}

/// Whether the reference *cell* for this by-ref position must reach the native
/// intact, instead of being collapsed to the value inside it.
///
/// `Opcode::CallNative` normally resolves ref cells before calling, because the
/// hundred-odd `.as_array_ptr()` sites in `ext/` know nothing about references —
/// and for an array out-parameter that is enough, since arrays alias by handle.
/// A *scalar* out-parameter cannot work that way: there is no handle to write
/// through, so `preg_replace()`'s `&$count` would have nowhere to put an integer
/// and the collapse would hand it a freshly allocated empty array instead.
/// These positions keep their cell and the native assigns through it.
pub fn keeps_ref_cell(callee: &str, index: usize) -> bool {
    let short = callee.rsplit('\\').next().unwrap_or(callee).to_ascii_lowercase();
    let scalar_out: &[usize] = match short.as_str() {
        "preg_replace" | "preg_replace_callback" | "preg_filter" => &[4],
        "preg_replace_callback_array" => &[3],
        "str_replace" | "str_ireplace" => &[3],
        "similar_text" => &[2],
        "proc_open" => &[2],
        "exec" => &[1, 2],
        "system" | "passthru" => &[1],
        "parse_str" => &[1],
        "preg_match" | "preg_match_all" => &[2],
        "stream_select" => &[0, 1, 2],
        _ => &[],
    };
    scalar_out.contains(&index)
}
