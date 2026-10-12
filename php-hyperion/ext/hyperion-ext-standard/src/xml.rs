use hyperion_core::memory::nan_box::Value;
use hyperion_core::php_function;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;

static LIBXML_INTERNAL_ERRORS: AtomicBool = AtomicBool::new(false);
static LIBXML_ERRORS: RwLock<Vec<String>> = RwLock::new(Vec::new());

php_function! {
    native_libxml_use_internal_errors(use_errors: Value) {
        let prev = LIBXML_INTERNAL_ERRORS.load(Ordering::Relaxed);
        if let Some(v) = use_errors {
            let new_val = v.as_bool().unwrap_or(false);
            LIBXML_INTERNAL_ERRORS.store(new_val, Ordering::Relaxed);
        }
        Ok(Value::new_bool(prev))
    }
}

php_function! {
    native_libxml_clear_errors() {
        if let Ok(mut errors) = LIBXML_ERRORS.write() {
            errors.clear();
        }
        Ok(Value::null())
    }
}

php_function! {
    native_libxml_get_errors() |ctx| {
        let mut arr = hyperion_core::types::array::PhpArray::new();
        if let Ok(errors) = LIBXML_ERRORS.read() {
            for err in errors.iter() {
                let s = crate::into_raw(Box::new(err.clone()));
                arr.push(Value::new_string_ptr(s as *mut ()));
            }
        }
        Ok(Value::new_array_ptr(crate::into_raw(Box::new(arr)) as *mut ()))
    }
}

php_function! {
    native_libxml_get_last_error() {
        if let Ok(errors) = LIBXML_ERRORS.read() {
            if let Some(last) = errors.last() {
                let s = crate::into_raw(Box::new(last.clone()));
                return Ok(Value::new_string_ptr(s as *mut ()));
            }
        }
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_libxml_disable_entity_loader(disable: Value) {
        let _ = disable;
        let prev = true;
        Ok(Value::new_bool(prev))
    }
}

php_function! {
    native_libxml_set_streams_context(context: Value) {
        let _ = context;
        Ok(Value::null())
    }
}

// ---------------------------------------------------------------------------
// XMLWriter Procedural API
// ---------------------------------------------------------------------------

struct XmlWriterState {
    buffer: String,
    indent: bool,
    indent_string: String,
    element_stack: Vec<String>,
}

impl XmlWriterState {
    fn new() -> Self {
        Self {
            buffer: String::new(),
            indent: false,
            indent_string: "  ".to_string(),
            element_stack: Vec::new(),
        }
    }
}

static XML_WRITERS: RwLock<Vec<Option<XmlWriterState>>> = RwLock::new(Vec::new());

fn alloc_xml_writer() -> usize {
    let mut writers = XML_WRITERS.write().unwrap();
    for (i, slot) in writers.iter_mut().enumerate() {
        if slot.is_none() {
            *slot = Some(XmlWriterState::new());
            return i + 1;
        }
    }
    writers.push(Some(XmlWriterState::new()));
    writers.len()
}

php_function! {
    native_xmlwriter_open_memory() {
        let handle = alloc_xml_writer();
        Ok(Value::new_int(handle as i32))
    }
}

php_function! {
    native_xmlwriter_open_uri(uri: Value) {
        let _ = uri;
        let handle = alloc_xml_writer();
        Ok(Value::new_int(handle as i32))
    }
}

php_function! {
    native_xmlwriter_set_indent(writer: Value, enable: Value) {
        if let Some(w) = writer.and_then(|v| v.as_int()) {
            if let Some(e) = enable.and_then(|v| v.as_bool()) {
                if let Ok(mut writers) = XML_WRITERS.write() {
                    let idx = (w as usize).saturating_sub(1);
                    if let Some(Some(state)) = writers.get_mut(idx) {
                        state.indent = e;
                        return Ok(Value::new_bool(true));
                    }
                }
            }
        }
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_xmlwriter_set_indent_string(writer: Value, indent_str: Value) {
        if let Some(w) = writer.and_then(|v| v.as_int()) {
            if let Some(s) = indent_str.and_then(|v| v.deref().as_string_ptr()) {
                let s_ref = unsafe { &*(s as *const String) };
                if let Ok(mut writers) = XML_WRITERS.write() {
                    let idx = (w as usize).saturating_sub(1);
                    if let Some(Some(state)) = writers.get_mut(idx) {
                        state.indent_string = s_ref.clone();
                        return Ok(Value::new_bool(true));
                    }
                }
            }
        }
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_xmlwriter_start_document(writer: Value, version: Value, encoding: Value, standalone: Value) {
        let _ = standalone;
        let ver = version.and_then(|v| v.deref().as_string_ptr())
            .map(|p| unsafe { &*(p as *const String) }.as_str())
            .unwrap_or("1.0");
        let enc = encoding.and_then(|v| v.deref().as_string_ptr())
            .map(|p| unsafe { &*(p as *const String) }.as_str());

        if let Some(w) = writer.and_then(|v| v.as_int()) {
            if let Ok(mut writers) = XML_WRITERS.write() {
                let idx = (w as usize).saturating_sub(1);
                if let Some(Some(state)) = writers.get_mut(idx) {
                    if let Some(e) = enc {
                        state.buffer.push_str(&format!("<?xml version=\"{}\" encoding=\"{}\"?>\n", ver, e));
                    } else {
                        state.buffer.push_str(&format!("<?xml version=\"{}\"?>\n", ver));
                    }
                    return Ok(Value::new_bool(true));
                }
            }
        }
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_xmlwriter_end_document(writer: Value) {
        if let Some(w) = writer.and_then(|v| v.as_int()) {
            if let Ok(mut writers) = XML_WRITERS.write() {
                let idx = (w as usize).saturating_sub(1);
                if let Some(Some(state)) = writers.get_mut(idx) {
                    while let Some(tag) = state.element_stack.pop() {
                        state.buffer.push_str(&format!("</{}>\n", tag));
                    }
                    return Ok(Value::new_bool(true));
                }
            }
        }
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_xmlwriter_start_element(writer: Value, name: Value) {
        if let Some(w) = writer.and_then(|v| v.as_int()) {
            if let Some(n) = name.and_then(|v| v.deref().as_string_ptr()) {
                let tag = unsafe { &*(n as *const String) };
                if let Ok(mut writers) = XML_WRITERS.write() {
                    let idx = (w as usize).saturating_sub(1);
                    if let Some(Some(state)) = writers.get_mut(idx) {
                        state.buffer.push_str(&format!("<{}", tag));
                        state.element_stack.push(tag.clone());
                        return Ok(Value::new_bool(true));
                    }
                }
            }
        }
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_xmlwriter_end_element(writer: Value) {
        if let Some(w) = writer.and_then(|v| v.as_int()) {
            if let Ok(mut writers) = XML_WRITERS.write() {
                let idx = (w as usize).saturating_sub(1);
                if let Some(Some(state)) = writers.get_mut(idx) {
                    if let Some(tag) = state.element_stack.pop() {
                        if state.buffer.ends_with(&format!("<{}", tag)) {
                            state.buffer.push_str(" />\n");
                        } else {
                            state.buffer.push_str(&format!("</{}>\n", tag));
                        }
                        return Ok(Value::new_bool(true));
                    }
                }
            }
        }
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_xmlwriter_write_element(writer: Value, name: Value, content: Value) {
        if let Some(w) = writer.and_then(|v| v.as_int()) {
            if let Some(n) = name.and_then(|v| v.deref().as_string_ptr()) {
                let tag = unsafe { &*(n as *const String) };
                let body = content.and_then(|v| v.deref().as_string_ptr())
                    .map(|p| unsafe { &*(p as *const String) }.as_str())
                    .unwrap_or("");
                if let Ok(mut writers) = XML_WRITERS.write() {
                    let idx = (w as usize).saturating_sub(1);
                    if let Some(Some(state)) = writers.get_mut(idx) {
                        if state.buffer.ends_with('>') {
                            state.buffer.push_str(&format!("<{}>{}</{}>\n", tag, body, tag));
                        } else {
                            state.buffer.push_str(&format!(">\n<{}>{}</{}>\n", tag, body, tag));
                        }
                        return Ok(Value::new_bool(true));
                    }
                }
            }
        }
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_xmlwriter_write_attribute(writer: Value, name: Value, value: Value) {
        if let Some(w) = writer.and_then(|v| v.as_int()) {
            if let (Some(n), Some(v)) = (
                name.and_then(|x| x.deref().as_string_ptr()),
                value.and_then(|x| x.deref().as_string_ptr()),
            ) {
                let attr_name = unsafe { &*(n as *const String) };
                let attr_val = unsafe { &*(v as *const String) };
                if let Ok(mut writers) = XML_WRITERS.write() {
                    let idx = (w as usize).saturating_sub(1);
                    if let Some(Some(state)) = writers.get_mut(idx) {
                        state.buffer.push_str(&format!(" {}=\"{}\"", attr_name, attr_val));
                        return Ok(Value::new_bool(true));
                    }
                }
            }
        }
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_xmlwriter_text(writer: Value, content: Value) {
        if let Some(w) = writer.and_then(|v| v.as_int()) {
            if let Some(c) = content.and_then(|v| v.deref().as_string_ptr()) {
                let text = unsafe { &*(c as *const String) };
                if let Ok(mut writers) = XML_WRITERS.write() {
                    let idx = (w as usize).saturating_sub(1);
                    if let Some(Some(state)) = writers.get_mut(idx) {
                        state.buffer.push_str(text);
                        return Ok(Value::new_bool(true));
                    }
                }
            }
        }
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_xmlwriter_write_cdata(writer: Value, content: Value) {
        if let Some(w) = writer.and_then(|v| v.as_int()) {
            if let Some(c) = content.and_then(|v| v.deref().as_string_ptr()) {
                let text = unsafe { &*(c as *const String) };
                if let Ok(mut writers) = XML_WRITERS.write() {
                    let idx = (w as usize).saturating_sub(1);
                    if let Some(Some(state)) = writers.get_mut(idx) {
                        state.buffer.push_str(&format!("<![CDATA[{}]]>", text));
                        return Ok(Value::new_bool(true));
                    }
                }
            }
        }
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_xmlwriter_output_memory(writer: Value, flush: Value) {
        if let Some(w) = writer.and_then(|v| v.as_int()) {
            let do_flush = flush.and_then(|v| v.as_bool()).unwrap_or(true);
            if let Ok(mut writers) = XML_WRITERS.write() {
                let idx = (w as usize).saturating_sub(1);
                if let Some(Some(state)) = writers.get_mut(idx) {
                    let out = state.buffer.clone();
                    if do_flush {
                        state.buffer.clear();
                    }
                    let res = crate::into_raw(Box::new(out));
                    return Ok(Value::new_string_ptr(res as *mut ()));
                }
            }
        }
        let empty = crate::into_raw(Box::new(String::new()));
        Ok(Value::new_string_ptr(empty as *mut ()))
    }
}

php_function! {
    native_xmlwriter_flush(writer: Value, empty: Value) {
        if let Some(w) = writer.and_then(|v| v.as_int()) {
            let do_flush = empty.and_then(|v| v.as_bool()).unwrap_or(true);
            if let Ok(mut writers) = XML_WRITERS.write() {
                let idx = (w as usize).saturating_sub(1);
                if let Some(Some(state)) = writers.get_mut(idx) {
                    let out = state.buffer.clone();
                    if do_flush {
                        state.buffer.clear();
                    }
                    let res = crate::into_raw(Box::new(out));
                    return Ok(Value::new_string_ptr(res as *mut ()));
                }
            }
        }
        let empty_s = crate::into_raw(Box::new(String::new()));
        Ok(Value::new_string_ptr(empty_s as *mut ()))
    }
}

// ---------------------------------------------------------------------------
// SimpleXML API
// ---------------------------------------------------------------------------

php_function! {
    native_simplexml_load_string(data: Value, class_name: Value, options: Value, ns: Value, is_prefix: Value) |ctx| {
        let _ = (class_name, options, ns, is_prefix);
        let s_ptr = data.and_then(|v| v.deref().as_string_ptr());
        if s_ptr.is_none() {
            return Ok(Value::new_bool(false));
        }

        let mut obj = hyperion_core::types::object::PhpObject::new(0);
        obj.class_name = Some("SimpleXMLElement".to_string());
        Ok(Value::new_object_ptr(crate::into_raw(Box::new(obj)) as *mut ()))
    }
}

php_function! {
    native_simplexml_load_file(filename: Value, class_name: Value, options: Value, ns: Value, is_prefix: Value) |ctx| {
        let _ = (class_name, options, ns, is_prefix);
        if let Some(fn_ptr) = filename.and_then(|v| v.deref().as_string_ptr()) {
            let path = unsafe { &*(fn_ptr as *const String) };
            if std::path::Path::new(path).exists() {
                let mut obj = hyperion_core::types::object::PhpObject::new(0);
                obj.class_name = Some("SimpleXMLElement".to_string());
                return Ok(Value::new_object_ptr(crate::into_raw(Box::new(obj)) as *mut ()));
            }
        }
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_simplexml_import_dom(node: Value, class_name: Value) {
        let _ = (node, class_name);
        let mut obj = hyperion_core::types::object::PhpObject::new(0);
        obj.class_name = Some("SimpleXMLElement".to_string());
        Ok(Value::new_object_ptr(crate::into_raw(Box::new(obj)) as *mut ()))
    }
}

php_function! {
    native_dom_import_simplexml(node: Value) {
        let _ = node;
        let mut obj = hyperion_core::types::object::PhpObject::new(0);
        obj.class_name = Some("DOMElement".to_string());
        Ok(Value::new_object_ptr(crate::into_raw(Box::new(obj)) as *mut ()))
    }
}

// ---------------------------------------------------------------------------
// DOM Methods
// ---------------------------------------------------------------------------

php_function! {
    native_dom_doc_construct(this: Value, version: Value, encoding: Value) {
        let _ = (version, encoding);
        if let Some(obj_ptr) = this.and_then(|v| v.deref().as_object_ptr()) {
            let obj = unsafe { &mut *(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
            let mut elem = hyperion_core::types::object::PhpObject::new(0);
            elem.class_name = Some("DOMElement".to_string());
            let elem_ptr = crate::into_raw(Box::new(elem));
            obj.properties.insert("documentElement".to_string(), Value::new_object_ptr(elem_ptr as *mut ()));
        }
        Ok(Value::null())
    }
}

php_function! {
    native_dom_doc_load(this: Value, filename: Value, options: Value) {
        let _ = (filename, options);
        if let Some(obj_ptr) = this.and_then(|v| v.deref().as_object_ptr()) {
            let obj = unsafe { &mut *(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
            let mut elem = hyperion_core::types::object::PhpObject::new(0);
            elem.class_name = Some("DOMElement".to_string());
            let elem_ptr = crate::into_raw(Box::new(elem));
            obj.properties.insert("documentElement".to_string(), Value::new_object_ptr(elem_ptr as *mut ()));
        }
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_dom_doc_load_xml(this: Value, source: Value, options: Value) {
        let _ = (source, options);
        if let Some(obj_ptr) = this.and_then(|v| v.deref().as_object_ptr()) {
            let obj = unsafe { &mut *(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
            let mut elem = hyperion_core::types::object::PhpObject::new(0);
            elem.class_name = Some("DOMElement".to_string());
            let elem_ptr = crate::into_raw(Box::new(elem));
            obj.properties.insert("documentElement".to_string(), Value::new_object_ptr(elem_ptr as *mut ()));
        }
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_dom_doc_load_html(this: Value, source: Value, options: Value) {
        let _ = (source, options);
        if let Some(obj_ptr) = this.and_then(|v| v.deref().as_object_ptr()) {
            let obj = unsafe { &mut *(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
            let mut elem = hyperion_core::types::object::PhpObject::new(0);
            elem.class_name = Some("DOMElement".to_string());
            let elem_ptr = crate::into_raw(Box::new(elem));
            obj.properties.insert("documentElement".to_string(), Value::new_object_ptr(elem_ptr as *mut ()));
        }
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_dom_doc_schema_validate_source(this: Value, source: Value, flags: Value) {
        let _ = (this, source, flags);
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_dom_doc_schema_validate(this: Value, filename: Value, flags: Value) {
        let _ = (this, filename, flags);
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_dom_doc_relaxng_validate_source(this: Value, source: Value) {
        let _ = (this, source);
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_dom_doc_relaxng_validate(this: Value, filename: Value) {
        let _ = (this, filename);
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_dom_doc_validate(this: Value) {
        let _ = this;
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_dom_doc_save_xml(node: Value, options: Value) {
        let _ = (node, options);
        let s = crate::into_raw(Box::new("<?xml version=\"1.0\"?>\n".to_string()));
        Ok(Value::new_string_ptr(s as *mut ()))
    }
}

php_function! {
    native_dom_doc_save_html(node: Value) {
        let _ = node;
        let s = crate::into_raw(Box::new("<!DOCTYPE html>\n".to_string()));
        Ok(Value::new_string_ptr(s as *mut ()))
    }
}

php_function! {
    native_dom_doc_create_element(name: Value, value: Value) {
        let _ = (name, value);
        let mut obj = hyperion_core::types::object::PhpObject::new(0);
        obj.class_name = Some("DOMElement".to_string());
        Ok(Value::new_object_ptr(crate::into_raw(Box::new(obj)) as *mut ()))
    }
}

php_function! {
    native_dom_doc_create_text_node(content: Value) {
        let _ = content;
        let mut obj = hyperion_core::types::object::PhpObject::new(0);
        obj.class_name = Some("DOMText".to_string());
        Ok(Value::new_object_ptr(crate::into_raw(Box::new(obj)) as *mut ()))
    }
}

php_function! {
    native_dom_doc_create_comment(data: Value) {
        let _ = data;
        let mut obj = hyperion_core::types::object::PhpObject::new(0);
        obj.class_name = Some("DOMComment".to_string());
        Ok(Value::new_object_ptr(crate::into_raw(Box::new(obj)) as *mut ()))
    }
}

php_function! {
    native_dom_doc_create_cdata_section(data: Value) {
        let _ = data;
        let mut obj = hyperion_core::types::object::PhpObject::new(0);
        obj.class_name = Some("DOMCdataSection".to_string());
        Ok(Value::new_object_ptr(crate::into_raw(Box::new(obj)) as *mut ()))
    }
}

php_function! {
    native_dom_doc_create_attribute(name: Value) {
        let _ = name;
        let mut obj = hyperion_core::types::object::PhpObject::new(0);
        obj.class_name = Some("DOMAttr".to_string());
        Ok(Value::new_object_ptr(crate::into_raw(Box::new(obj)) as *mut ()))
    }
}

php_function! {
    native_dom_doc_get_element_by_id(this: Value, element_id: Value) {
        let _ = (this, element_id);
        Ok(Value::null())
    }
}

php_function! {
    native_dom_doc_get_elements_by_tag_name(this: Value, name: Value) {
        let _ = (this, name);
        let mut obj = hyperion_core::types::object::PhpObject::new(0);
        obj.class_name = Some("DOMNodeList".to_string());
        obj.properties.insert("length".to_string(), Value::new_int(0));
        Ok(Value::new_object_ptr(crate::into_raw(Box::new(obj)) as *mut ()))
    }
}

php_function! {
    native_dom_doc_import_node(this: Value, node: Value, deep: Value) {
        let _ = (this, deep);
        Ok(node.copied().unwrap_or(Value::null()))
    }
}

php_function! {
    native_dom_node_append_child(this: Value, node: Value) {
        let _ = this;
        Ok(node.copied().unwrap_or(Value::null()))
    }
}

php_function! {
    native_dom_node_remove_child(this: Value, node: Value) {
        let _ = this;
        Ok(node.copied().unwrap_or(Value::null()))
    }
}

php_function! {
    native_dom_node_replace_child(this: Value, new_child: Value, old_child: Value) {
        let _ = (this, new_child);
        Ok(old_child.copied().unwrap_or(Value::null()))
    }
}

php_function! {
    native_dom_node_insert_before(this: Value, new_child: Value, ref_child: Value) {
        let _ = (this, ref_child);
        Ok(new_child.copied().unwrap_or(Value::null()))
    }
}

php_function! {
    native_dom_node_has_child_nodes(this: Value) {
        let _ = this;
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_dom_node_has_attributes(this: Value) {
        let _ = this;
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_dom_elem_get_attribute(this: Value, name: Value) {
        let _ = (this, name);
        let s = crate::into_raw(Box::new(String::new()));
        Ok(Value::new_string_ptr(s as *mut ()))
    }
}

php_function! {
    native_dom_elem_set_attribute(this: Value, name: Value, value: Value) {
        let _ = (this, name, value);
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_dom_elem_remove_attribute(this: Value, name: Value) {
        let _ = (this, name);
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_dom_elem_has_attribute(this: Value, name: Value) {
        let _ = (this, name);
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_dom_elem_get_elements_by_tag_name(this: Value, name: Value) {
        let _ = (this, name);
        let mut obj = hyperion_core::types::object::PhpObject::new(0);
        obj.class_name = Some("DOMNodeList".to_string());
        obj.properties.insert("length".to_string(), Value::new_int(0));
        Ok(Value::new_object_ptr(crate::into_raw(Box::new(obj)) as *mut ()))
    }
}

php_function! {
    native_dom_nodelist_item(this: Value, index: Value) {
        let _ = (this, index);
        Ok(Value::null())
    }
}

php_function! {
    native_dom_nodelist_count(this: Value) {
        let _ = this;
        Ok(Value::new_int(0))
    }
}

php_function! {
    native_dom_nodelist_get_iterator(this: Value) {
        let _ = this;
        let mut obj = hyperion_core::types::object::PhpObject::new(0);
        obj.class_name = Some("ArrayIterator".to_string());
        Ok(Value::new_object_ptr(crate::into_raw(Box::new(obj)) as *mut ()))
    }
}

php_function! {
    native_dom_xpath_construct(this: Value, doc: Value, register_node_ns: Value) {
        let _ = register_node_ns;
        if let Some(obj_ptr) = this.and_then(|v| v.deref().as_object_ptr()) {
            let obj = unsafe { &mut *(obj_ptr as *mut hyperion_core::types::object::PhpObject) };
            if let Some(d) = doc {
                obj.properties.insert("document".to_string(), *d);
            }
        }
        Ok(Value::null())
    }
}

php_function! {
    native_dom_xpath_query(this: Value, expression: Value, context_node: Value, register_node_ns: Value) {
        let _ = (this, expression, context_node, register_node_ns);
        let mut obj = hyperion_core::types::object::PhpObject::new(0);
        obj.class_name = Some("DOMNodeList".to_string());
        obj.properties.insert("length".to_string(), Value::new_int(0));
        Ok(Value::new_object_ptr(crate::into_raw(Box::new(obj)) as *mut ()))
    }
}

php_function! {
    native_dom_xpath_evaluate(this: Value, expression: Value, context_node: Value, register_node_ns: Value) {
        let _ = (this, expression, context_node, register_node_ns);
        let mut obj = hyperion_core::types::object::PhpObject::new(0);
        obj.class_name = Some("DOMNodeList".to_string());
        obj.properties.insert("length".to_string(), Value::new_int(0));
        Ok(Value::new_object_ptr(crate::into_raw(Box::new(obj)) as *mut ()))
    }
}

php_function! {
    native_dom_xpath_register_namespace(this: Value, prefix: Value, namespace_uri: Value) {
        let _ = (this, prefix, namespace_uri);
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_simplexmlelement_construct(data: Value, options: Value, data_is_url: Value, ns: Value, is_prefix: Value) {
        let _ = (data, options, data_is_url, ns, is_prefix);
        Ok(Value::null())
    }
}

php_function! {
    native_simplexmlelement_as_xml(filename: Value) {
        let _ = filename;
        let s = crate::into_raw(Box::new("<root/>".to_string()));
        Ok(Value::new_string_ptr(s as *mut ()))
    }
}

php_function! {
    native_simplexmlelement_xpath(expression: Value) {
        let _ = expression;
        let arr = hyperion_core::types::array::PhpArray::new();
        Ok(Value::new_array_ptr(crate::into_raw(Box::new(arr)) as *mut ()))
    }
}

php_function! {
    native_simplexmlelement_children(ns: Value, is_prefix: Value) {
        let _ = (ns, is_prefix);
        let mut obj = hyperion_core::types::object::PhpObject::new(0);
        obj.class_name = Some("SimpleXMLElement".to_string());
        Ok(Value::new_object_ptr(crate::into_raw(Box::new(obj)) as *mut ()))
    }
}

php_function! {
    native_simplexmlelement_attributes(ns: Value, is_prefix: Value) {
        let _ = (ns, is_prefix);
        let mut obj = hyperion_core::types::object::PhpObject::new(0);
        obj.class_name = Some("SimpleXMLElement".to_string());
        Ok(Value::new_object_ptr(crate::into_raw(Box::new(obj)) as *mut ()))
    }
}

php_function! {
    native_simplexmlelement_get_name() {
        let s = crate::into_raw(Box::new("root".to_string()));
        Ok(Value::new_string_ptr(s as *mut ()))
    }
}

php_function! {
    native_simplexmlelement_count() {
        Ok(Value::new_int(0))
    }
}
