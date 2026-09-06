use crate::types::class::PhpClass;
use hyperion_core::memory::nan_box::Value;
use hyperion_parser::parser::ast::Visibility;

pub fn register_core_classes(classes: &dashmap::DashMap<usize, PhpClass>, current_id: &mut usize) {
    // --- Interfaces ---
    let interfaces = vec![
        "Throwable",
        "ArrayAccess",
        "Countable",
        "Traversable",
        "Iterator",
        "IteratorAggregate",
        "SeekableIterator",
        "RecursiveIterator",
        "OuterIterator",
        "Serializable",
        "Stringable",
        "JsonSerializable",
        "DateTimeInterface",
    ];

    for name in interfaces {
        let mut iface = PhpClass::new(*current_id, name.to_string());
        // `Iterator` and `IteratorAggregate` both extend `Traversable`, and code
        // in the wild tests for `Traversable` alone before iterating.
        if matches!(name, "Iterator" | "IteratorAggregate") {
            iface.implements("Traversable");
        }
        if matches!(name, "SeekableIterator" | "RecursiveIterator" | "OuterIterator") {
            iface.implements("Iterator");
            iface.implements("Traversable");
        }
        classes.insert(*current_id, iface);
        *current_id += 1;
    }

    if let Some(mut gen) = classes.iter_mut().find(|c| c.value().name == "Generator") {
        gen.implements("Iterator");
        gen.implements("Traversable");
        gen.add_native_method("rewind".to_string(), 0, "native_generator_rewind".to_string());
        gen.add_native_method("valid".to_string(), 0, "native_generator_valid".to_string());
        gen.add_native_method("current".to_string(), 0, "native_generator_current".to_string());
        gen.add_native_method("key".to_string(), 0, "native_generator_key".to_string());
        gen.add_native_method("next".to_string(), 0, "native_generator_next".to_string());
        gen.add_native_method("send".to_string(), 1, "native_generator_send".to_string());
        gen.add_native_method("throw".to_string(), 1, "native_generator_throw".to_string());
        gen.add_native_method("getReturn".to_string(), 0, "native_generator_get_return".to_string());
    }

    // --- Exceptions ---
    let mut exception = PhpClass::new(*current_id, "Exception".to_string());
    exception.implements("Throwable");
    let empty_msg = Box::into_raw(Box::new("".to_string()));
    exception.add_property(
        "message",
        Value::new_string_ptr(empty_msg as *mut ()),
        Visibility::Protected,
    );
    exception.add_property("code", Value::new_int(0), Visibility::Protected);
    let empty_file = Box::into_raw(Box::new("".to_string()));
    exception.add_property(
        "file",
        Value::new_string_ptr(empty_file as *mut ()),
        Visibility::Protected,
    );
    exception.add_property("line", Value::new_int(0), Visibility::Protected);
    exception.add_property("previous", Value::null(), Visibility::Private);
    // Add methods for Exception (construct, getMessage, getCode, etc.)
    // For now we just add empty methods so they exist in the VM. The constructor could take args.
    exception.add_native_method(
        "__construct".to_string(),
        3,
        "native_exception_construct".to_string(),
    );
    exception.add_native_method(
        "getMessage".to_string(),
        0,
        "native_exception_get_message".to_string(),
    );
    exception.add_native_method(
        "getCode".to_string(),
        0,
        "native_exception_get_code".to_string(),
    );
    exception.add_native_method(
        "getFile".to_string(),
        0,
        "native_exception_get_file".to_string(),
    );
    exception.add_native_method(
        "getLine".to_string(),
        0,
        "native_exception_get_line".to_string(),
    );
    exception.add_native_method(
        "getTrace".to_string(),
        0,
        "native_exception_get_trace".to_string(),
    );
    exception.add_native_method(
        "getTraceAsString".to_string(),
        0,
        "native_exception_get_trace_as_string".to_string(),
    );
    exception.add_native_method(
        "getPrevious".to_string(),
        0,
        "native_exception_get_previous".to_string(),
    );
    exception.add_native_method(
        "__toString".to_string(),
        0,
        "native_exception_to_string".to_string(),
    );
    classes.insert(*current_id, exception);
    *current_id += 1;

    let mut error = PhpClass::new(*current_id, "Error".to_string());
    error.extends("Exception");
    classes.insert(*current_id, error);
    *current_id += 1;

    let simple_exceptions = vec![
        ("RuntimeException", "Exception"),
        ("InvalidArgumentException", "Exception"),
        ("LogicException", "Exception"),
        ("BadMethodCallException", "LogicException"),
        ("BadFunctionCallException", "LogicException"),
        ("DomainException", "LogicException"),
        ("LengthException", "LogicException"),
        ("OutOfRangeException", "LogicException"),
        ("UnexpectedValueException", "RuntimeException"),
        ("OutOfBoundsException", "RuntimeException"),
        ("OverflowException", "RuntimeException"),
        ("UnderflowException", "RuntimeException"),
        ("RangeException", "RuntimeException"),
        ("TypeError", "Error"),
        ("ValueError", "Error"),
        ("ParseError", "Error"),
        ("CompileError", "Error"),
        ("DivisionByZeroError", "Error"),
        ("UnhandledMatchError", "Error"),
        ("ErrorException", "Exception"),
        ("AssertionError", "Error"),
        ("PDOException", "RuntimeException"),
        ("ReflectionException", "Exception"),
    ];

    for (name, parent) in simple_exceptions {
        let mut ex = PhpClass::new(*current_id, name.to_string());
        ex.extends(parent);
        classes.insert(*current_id, ex);
        *current_id += 1;
    }

    // --- DateTime / DateInterval ---
    let mut tz = PhpClass::new(*current_id, "DateTimeZone".to_string());
    let empty_str = Box::into_raw(Box::new("".to_string()));
    tz.add_property(
        "name",
        Value::new_string_ptr(empty_str as *mut ()),
        Visibility::Protected,
    );
    tz.add_native_method(
        "__construct".to_string(),
        1,
        "native_datetimezone_construct".to_string(),
    );
    tz.add_native_method(
        "getName".to_string(),
        0,
        "native_datetimezone_get_name".to_string(),
    );
    classes.insert(*current_id, tz);
    *current_id += 1;

    let mut datetime = PhpClass::new(*current_id, "DateTime".to_string());
    datetime.implements("DateTimeInterface");
    datetime.add_property("timestamp", Value::null(), Visibility::Protected);
    let utc_str1 = Box::into_raw(Box::new("UTC".to_string()));
    datetime.add_property(
        "timezone_name",
        Value::new_string_ptr(utc_str1 as *mut ()),
        Visibility::Protected,
    );
    datetime.add_native_method(
        "__construct".to_string(),
        2,
        "native_datetime_construct".to_string(),
    );
    datetime.add_native_method(
        "format".to_string(),
        1,
        "native_datetime_format".to_string(),
    );
    datetime.add_native_method(
        "modify".to_string(),
        1,
        "native_datetime_modify".to_string(),
    );
    datetime.add_native_method(
        "getTimestamp".to_string(),
        0,
        "native_datetime_get_timestamp".to_string(),
    );
    datetime.add_native_method(
        "setTimestamp".to_string(),
        1,
        "native_datetime_set_timestamp".to_string(),
    );
    datetime.add_native_method(
        "getLastErrors".to_string(),
        0,
        "native_datetime_get_last_errors".to_string(),
    );
    datetime.add_native_method(
        "getTimezone".to_string(),
        0,
        "native_datetime_get_timezone".to_string(),
    );
    datetime.add_native_method(
        "setTimezone".to_string(),
        1,
        "native_datetime_set_timezone".to_string(),
    );
    datetime.add_native_method("diff".to_string(), 2, "native_datetime_diff".to_string());
    datetime.add_native_method(
        "getOffset".to_string(),
        0,
        "native_datetime_get_offset".to_string(),
    );
    datetime.add_native_method(
        "__wakeup".to_string(),
        0,
        "native_datetime_wakeup".to_string(),
    );
    datetime.add_native_method(
        "add".to_string(),
        1,
        "native_datetime_add".to_string(),
    );
    datetime.add_native_method(
        "sub".to_string(),
        1,
        "native_datetime_sub".to_string(),
    );
    datetime.add_native_method(
        "setTime".to_string(),
        4,
        "native_datetime_set_time".to_string(),
    );
    datetime.add_native_method(
        "setDate".to_string(),
        3,
        "native_datetime_set_date".to_string(),
    );
    datetime.add_native_static_method(
        "setTime".to_string(),
        4,
        "native_datetime_set_time".to_string(),
    );
    datetime.add_native_static_method(
        "setDate".to_string(),
        3,
        "native_datetime_set_date".to_string(),
    );
    datetime.add_native_static_method(
        "createFromFormat".to_string(),
        3,
        "native_datetime_create_from_format".to_string(),
    );
    datetime.add_native_static_method(
        "createFromInterface".to_string(),
        1,
        "native_datetime_create_from_interface".to_string(),
    );
    datetime.add_native_static_method(
        "createFromImmutable".to_string(),
        1,
        "native_datetime_create_from_interface".to_string(),
    );
    datetime.add_native_static_method(
        "getLastErrors".to_string(),
        0,
        "native_datetime_get_last_errors".to_string(),
    );
    datetime.add_native_static_method(
        "setLastErrors".to_string(),
        1,
        "native_datetime_set_last_errors".to_string(),
    );
    classes.insert(*current_id, datetime);
    *current_id += 1;

    let mut dt_imm = PhpClass::new(*current_id, "DateTimeImmutable".to_string());
    dt_imm.implements("DateTimeInterface");
    dt_imm.add_property("timestamp", Value::null(), Visibility::Protected);
    let utc_str2 = Box::into_raw(Box::new("UTC".to_string()));
    dt_imm.add_property(
        "timezone_name",
        Value::new_string_ptr(utc_str2 as *mut ()),
        Visibility::Protected,
    );
    dt_imm.add_native_method(
        "__construct".to_string(),
        2,
        "native_datetime_construct".to_string(),
    );
    dt_imm.add_native_static_method(
        "createFromFormat".to_string(),
        3,
        "native_datetime_immutable_create_from_format".to_string(),
    );
    dt_imm.add_native_static_method(
        "createFromInterface".to_string(),
        1,
        "native_datetime_immutable_create_from_interface".to_string(),
    );
    dt_imm.add_native_static_method(
        "createFromMutable".to_string(),
        1,
        "native_datetime_immutable_create_from_interface".to_string(),
    );
    dt_imm.add_native_method(
        "setTime".to_string(),
        4,
        "native_datetimeimmutable_set_time".to_string(),
    );
    dt_imm.add_native_method(
        "setDate".to_string(),
        3,
        "native_datetimeimmutable_set_date".to_string(),
    );
    dt_imm.add_native_static_method(
        "setTime".to_string(),
        4,
        "native_datetimeimmutable_set_time".to_string(),
    );
    dt_imm.add_native_static_method(
        "setDate".to_string(),
        3,
        "native_datetimeimmutable_set_date".to_string(),
    );
    dt_imm.add_native_static_method(
        "getLastErrors".to_string(),
        0,
        "native_datetime_get_last_errors".to_string(),
    );
    dt_imm.add_native_static_method(
        "setLastErrors".to_string(),
        1,
        "native_datetime_set_last_errors".to_string(),
    );
    dt_imm.add_native_method(
        "getLastErrors".to_string(),
        0,
        "native_datetime_get_last_errors".to_string(),
    );
    dt_imm.add_native_method(
        "format".to_string(),
        1,
        "native_datetime_format".to_string(),
    );
    dt_imm.add_native_method(
        "modify".to_string(),
        1,
        "native_datetimeimmutable_modify".to_string(),
    );
    dt_imm.add_native_method(
        "add".to_string(),
        1,
        "native_datetimeimmutable_add".to_string(),
    );
    dt_imm.add_native_method(
        "sub".to_string(),
        1,
        "native_datetimeimmutable_sub".to_string(),
    );
    dt_imm.add_native_method(
        "getTimestamp".to_string(),
        0,
        "native_datetime_get_timestamp".to_string(),
    );
    dt_imm.add_native_method(
        "setTimestamp".to_string(),
        1,
        "native_datetimeimmutable_set_timestamp".to_string(),
    );
    dt_imm.add_native_method(
        "getTimezone".to_string(),
        0,
        "native_datetime_get_timezone".to_string(),
    );
    dt_imm.add_native_method(
        "setTimezone".to_string(),
        1,
        "native_datetimeimmutable_set_timezone".to_string(),
    );
    dt_imm.add_native_method("diff".to_string(), 2, "native_datetime_diff".to_string());
    dt_imm.add_native_method(
        "getOffset".to_string(),
        0,
        "native_datetime_get_offset".to_string(),
    );
    dt_imm.add_native_method(
        "__wakeup".to_string(),
        0,
        "native_datetime_wakeup".to_string(),
    );
    classes.insert(*current_id, dt_imm);
    *current_id += 1;

    let mut di = PhpClass::new(*current_id, "DateInterval".to_string());
    di.add_property("y", Value::new_int(0), Visibility::Public);
    di.add_property("m", Value::new_int(0), Visibility::Public);
    di.add_property("d", Value::new_int(0), Visibility::Public);
    di.add_property("h", Value::new_int(0), Visibility::Public);
    di.add_property("i", Value::new_int(0), Visibility::Public);
    di.add_property("s", Value::new_int(0), Visibility::Public);
    di.add_property("f", Value::new_float(0.0), Visibility::Public);
    di.add_property("invert", Value::new_int(0), Visibility::Public);
    di.add_property("days", Value::new_bool(false), Visibility::Public);
    di.add_property("from_string", Value::new_bool(false), Visibility::Public);
    di.add_native_method(
        "__construct".to_string(),
        1,
        "native_dateinterval_construct".to_string(),
    );
    di.add_native_static_method(
        "createFromDateString".to_string(),
        1,
        "native_dateinterval_create_from_date_string".to_string(),
    );
    classes.insert(*current_id, di);
    *current_id += 1;

    // --- Fiber ---
    let mut fiber_class = PhpClass::new(*current_id, "Fiber".to_string());
    fiber_class.add_native_method("__construct".to_string(), 1, "native_fiber_construct".to_string());
    fiber_class.add_native_method("start".to_string(), 0, "native_fiber_start".to_string());
    fiber_class.add_native_method("resume".to_string(), 1, "native_fiber_resume".to_string());
    fiber_class.add_native_method("throw".to_string(), 1, "native_fiber_throw".to_string());
    fiber_class.add_native_method("getReturn".to_string(), 0, "native_fiber_get_return".to_string());
    fiber_class.add_native_method("isStarted".to_string(), 0, "native_fiber_is_started".to_string());
    fiber_class.add_native_method("isSuspended".to_string(), 0, "native_fiber_is_suspended".to_string());
    fiber_class.add_native_method("isRunning".to_string(), 0, "native_fiber_is_running".to_string());
    fiber_class.add_native_method("isTerminated".to_string(), 0, "native_fiber_is_terminated".to_string());
    fiber_class.add_native_method("getCurrent".to_string(), 0, "native_fiber_get_current".to_string());
    fiber_class.add_native_method("suspend".to_string(), 1, "native_fiber_suspend".to_string());
    classes.insert(*current_id, fiber_class);
    *current_id += 1;

    let mut fiber_error = PhpClass::new(*current_id, "FiberError".to_string());
    fiber_error.implements("Throwable");
    fiber_error.extends = Some("Error".to_string());
    classes.insert(*current_id, fiber_error);
    *current_id += 1;

    // --- ZipArchive ---
    let mut zip_class = PhpClass::new(*current_id, "ZipArchive".to_string());
    zip_class.implements("Countable");
    zip_class.add_property("status", Value::new_int(0), Visibility::Public);
    zip_class.add_property("numFiles", Value::new_int(0), Visibility::Public);
    zip_class.add_property("filename", Value::new_string_ptr(Box::into_raw(Box::new("".to_string())) as *mut ()), Visibility::Public);
    zip_class.add_native_method("open".to_string(), 2, "native_ziparchive_open".to_string());
    zip_class.add_native_method("extractTo".to_string(), 2, "native_ziparchive_extract_to".to_string());
    zip_class.add_native_method("addFile".to_string(), 2, "native_ziparchive_add_file".to_string());
    zip_class.add_native_method("addFromString".to_string(), 2, "native_ziparchive_add_from_string".to_string());
    zip_class.add_native_method("close".to_string(), 0, "native_ziparchive_close".to_string());
    zip_class.add_native_method("getNameIndex".to_string(), 1, "native_ziparchive_get_name_index".to_string());
    classes.insert(*current_id, zip_class);
    *current_id += 1;

    // --- WeakReference ---
    let weakref_class = PhpClass::new(*current_id, "WeakReference".to_string());
    classes.insert(*current_id, weakref_class);
    *current_id += 1;

    // --- Redis ---
    let mut redis_class = PhpClass::new(*current_id, "Redis".to_string());
    redis_class.add_native_method("connect".to_string(), 4, "native_redis_connect".to_string());
    redis_class.add_native_method("pconnect".to_string(), 4, "native_redis_connect".to_string());
    redis_class.add_native_method("set".to_string(), 3, "native_redis_set".to_string());
    redis_class.add_native_method("get".to_string(), 1, "native_redis_get".to_string());
    redis_class.add_native_method("del".to_string(), 1, "native_redis_del".to_string());
    redis_class.add_native_method("delete".to_string(), 1, "native_redis_del".to_string());
    redis_class.add_native_method("exists".to_string(), 1, "native_redis_exists".to_string());
    redis_class.add_native_method("ping".to_string(), 0, "native_redis_ping".to_string());
    redis_class.add_native_method("flushDB".to_string(), 0, "native_redis_flushdb".to_string());
    redis_class.add_native_method("flushAll".to_string(), 0, "native_redis_flushdb".to_string());
    redis_class.add_native_method("close".to_string(), 0, "native_redis_close".to_string());
    classes.insert(*current_id, redis_class);
    *current_id += 1;

    let mut redis_exc = PhpClass::new(*current_id, "RedisException".to_string());
    redis_exc.implements("Throwable");
    redis_exc.extends = Some("Exception".to_string());
    classes.insert(*current_id, redis_exc);
    *current_id += 1;

    // (DOM classes registered via prelude.php)


    let mut simplexml = PhpClass::new(*current_id, "SimpleXMLElement".to_string());
    simplexml.implements("Countable");
    simplexml.implements("Iterator");
    simplexml.implements("Traversable");
    simplexml.implements("ArrayAccess");
    simplexml.implements("Stringable");
    simplexml.add_native_method("__construct".to_string(), 5, "native_simplexmlelement_construct".to_string());
    simplexml.add_native_method("asXML".to_string(), 1, "native_simplexmlelement_as_xml".to_string());
    simplexml.add_native_method("xpath".to_string(), 1, "native_simplexmlelement_xpath".to_string());
    simplexml.add_native_method("children".to_string(), 2, "native_simplexmlelement_children".to_string());
    simplexml.add_native_method("attributes".to_string(), 2, "native_simplexmlelement_attributes".to_string());
    simplexml.add_native_method("getName".to_string(), 0, "native_simplexmlelement_get_name".to_string());
    simplexml.add_native_method("count".to_string(), 0, "native_simplexmlelement_count".to_string());
    classes.insert(*current_id, simplexml);
    *current_id += 1;

    let mut xmlwriter_cls = PhpClass::new(*current_id, "XMLWriter".to_string());
    xmlwriter_cls.add_native_method("openMemory".to_string(), 0, "native_xmlwriter_open_memory".to_string());
    xmlwriter_cls.add_native_method("openUri".to_string(), 1, "native_xmlwriter_open_uri".to_string());
    xmlwriter_cls.add_native_method("setIndent".to_string(), 1, "native_xmlwriter_set_indent".to_string());
    xmlwriter_cls.add_native_method("setIndentString".to_string(), 1, "native_xmlwriter_set_indent_string".to_string());
    xmlwriter_cls.add_native_method("startDocument".to_string(), 3, "native_xmlwriter_start_document".to_string());
    xmlwriter_cls.add_native_method("endDocument".to_string(), 0, "native_xmlwriter_end_document".to_string());
    xmlwriter_cls.add_native_method("startElement".to_string(), 1, "native_xmlwriter_start_element".to_string());
    xmlwriter_cls.add_native_method("endElement".to_string(), 0, "native_xmlwriter_end_element".to_string());
    xmlwriter_cls.add_native_method("writeElement".to_string(), 2, "native_xmlwriter_write_element".to_string());
    xmlwriter_cls.add_native_method("writeAttribute".to_string(), 2, "native_xmlwriter_write_attribute".to_string());
    xmlwriter_cls.add_native_method("text".to_string(), 1, "native_xmlwriter_text".to_string());
    xmlwriter_cls.add_native_method("writeCdata".to_string(), 1, "native_xmlwriter_write_cdata".to_string());
    xmlwriter_cls.add_native_method("outputMemory".to_string(), 1, "native_xmlwriter_output_memory".to_string());
    xmlwriter_cls.add_native_method("flush".to_string(), 1, "native_xmlwriter_flush".to_string());
    classes.insert(*current_id, xmlwriter_cls);
    *current_id += 1;

    let mut libxml_err = PhpClass::new(*current_id, "LibXMLError".to_string());
    libxml_err.add_property("level", Value::new_int(0), Visibility::Public);
    libxml_err.add_property("code", Value::new_int(0), Visibility::Public);
    libxml_err.add_property("column", Value::new_int(0), Visibility::Public);
    libxml_err.add_property("message", Value::new_string_ptr(Box::into_raw(Box::new("".to_string())) as *mut ()), Visibility::Public);
    libxml_err.add_property("file", Value::new_string_ptr(Box::into_raw(Box::new("".to_string())) as *mut ()), Visibility::Public);
    libxml_err.add_property("line", Value::new_int(0), Visibility::Public);
    classes.insert(*current_id, libxml_err);
    *current_id += 1;
}
