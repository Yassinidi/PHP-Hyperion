use hyperion_core::php_function;
use hyperion_core::memory::nan_box::Value;
use hyperion_core::types::array::{PhpArray, ArrayKey};
use hyperion_core::types::object::PhpObject;
use hyperion_core::types::string_table::intern_string;
use lazy_static::lazy_static;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use rusqlite::types::ValueRef;
use rusqlite::ToSql;

#[derive(Clone, Debug)]
pub enum SqlValue {
    Null,
    Int(i64),
    Float(f64),
    Bool(bool),
    Text(String),
    Blob(Vec<u8>),
}

impl SqlValue {
    pub fn from_php(val: Value) -> Self {
        let val = val.deref();
        if let Some(i) = val.as_int() {
            SqlValue::Int(i as i64)
        } else if let Some(f) = val.as_float() {
            SqlValue::Float(f)
        } else if let Some(b) = val.as_bool() {
            SqlValue::Bool(b)
        } else if let Some(s_ptr) = val.as_string_ptr() {
            let s = unsafe { (*(s_ptr as *const String)).clone() };
            SqlValue::Text(s)
        } else {
            SqlValue::Null
        }
    }
}

impl rusqlite::ToSql for SqlValue {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        match self {
            SqlValue::Int(i) => Ok(rusqlite::types::ToSqlOutput::from(*i)),
            SqlValue::Float(f) => Ok(rusqlite::types::ToSqlOutput::from(*f)),
            SqlValue::Bool(b) => Ok(rusqlite::types::ToSqlOutput::from(*b)),
            SqlValue::Text(s) => Ok(rusqlite::types::ToSqlOutput::from(s.as_str())),
            SqlValue::Blob(b) => Ok(rusqlite::types::ToSqlOutput::from(b.as_slice())),
            SqlValue::Null => Ok(rusqlite::types::ToSqlOutput::from(rusqlite::types::Null)),
        }
    }
}

#[derive(Clone, Debug)]
pub struct CachedResultSet {
    pub col_names: Arc<Vec<String>>,
    pub col_key_ids: Arc<Vec<usize>>,
    pub rows: Vec<Vec<SqlValue>>,
}

fn sqlite_value_to_sql_value(v: ValueRef<'_>) -> SqlValue {
    match v {
        ValueRef::Null => SqlValue::Null,
        ValueRef::Integer(i) => SqlValue::Int(i),
        ValueRef::Real(f) => SqlValue::Float(f),
        ValueRef::Text(t) => SqlValue::Text(String::from_utf8_lossy(t).into_owned()),
        ValueRef::Blob(b) => SqlValue::Blob(b.to_vec()),
    }
}

fn sql_value_into_php_value(v: SqlValue, ctx: &mut dyn hyperion_core::types::function::NativeContext) -> Value {
    match v {
        SqlValue::Null => Value::null(),
        SqlValue::Int(i) => Value::new_int(i as i32),
        SqlValue::Float(f) => Value::new_float(f),
        SqlValue::Bool(b) => Value::new_bool(b),
        SqlValue::Text(s) => {
            let ptr = ctx.get_arena().alloc_and_track(s);
            Value::new_string_ptr(ptr as *mut ())
        }
        SqlValue::Blob(b) => {
            let s = String::from_utf8_lossy(&b).into_owned();
            let ptr = ctx.get_arena().alloc_and_track(s);
            Value::new_string_ptr(ptr as *mut ())
        }
    }
}

pub enum DbConnection {
    Sqlite(rusqlite::Connection),
    Mysql(mysql::Pool),
}

pub struct PreparedStatement {
    pub conn_id: u64,
    pub query: String,
    pub bound_params: HashMap<String, SqlValue>,
    pub bound_positional: HashMap<usize, SqlValue>,
    pub cached_rows: Option<CachedResultSet>,
    pub affected_rows: usize,
    pub current_cursor: usize,
    pub fetch_mode: i32,
}

lazy_static! {
    static ref DB_POOL: threadpool::ThreadPool = threadpool::ThreadPool::new(4);
    static ref CONNECTIONS: dashmap::DashMap<u64, Arc<Mutex<DbConnection>>> = dashmap::DashMap::new();
    static ref DSN_POOLS: dashmap::DashMap<String, Arc<Mutex<DbConnection>>> = dashmap::DashMap::new();
    static ref STATEMENTS: dashmap::DashMap<u64, PreparedStatement> = dashmap::DashMap::new();
    static ref ID_COUNTER: AtomicU64 = AtomicU64::new(1);
}

fn next_id() -> u64 {
    ID_COUNTER.fetch_add(1, Ordering::SeqCst)
}

fn get_or_create_connection(dsn_str: &str, username: Option<&Value>, password: Option<&Value>) -> Result<Arc<Mutex<DbConnection>>, String> {
    let is_memory_sqlite = (dsn_str.starts_with("sqlite:") && {
        let path = &dsn_str[7..];
        path == ":memory:" || path.is_empty()
    }) || (!dsn_str.starts_with("sqlite:") && !dsn_str.starts_with("mysql:"));

    if !is_memory_sqlite {
        if let Some(conn) = DSN_POOLS.get(dsn_str) {
            return Ok(conn.value().clone());
        }
    }

    let conn = if dsn_str.starts_with("sqlite:") {
        let path = &dsn_str[7..];
        let c = if path == ":memory:" || path.is_empty() {
            rusqlite::Connection::open_in_memory()
                .map_err(|e| format!("PDOException: SQLite connection error: {}", e))?
        } else if path.starts_with("file:") {
            let flags = rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
                | rusqlite::OpenFlags::SQLITE_OPEN_CREATE
                | rusqlite::OpenFlags::SQLITE_OPEN_URI;
            let c = rusqlite::Connection::open_with_flags(path, flags)
                .map_err(|e| format!("PDOException: SQLite connection error: {}", e))?;
            c.busy_timeout(std::time::Duration::from_millis(5000)).ok();
            let _ = c.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL; PRAGMA cache_size = -2000; PRAGMA temp_store = MEMORY;");
            c
        } else {
            let c = rusqlite::Connection::open(path)
                .map_err(|e| format!("PDOException: SQLite connection error: {}", e))?;
            c.busy_timeout(std::time::Duration::from_millis(5000)).ok();
            let _ = c.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL; PRAGMA cache_size = -2000; PRAGMA temp_store = MEMORY;");
            c
        };
        Arc::new(Mutex::new(DbConnection::Sqlite(c)))
    } else if dsn_str.starts_with("mysql:") {
        let user_str = username.and_then(|v| {
            if let Some(sp) = v.deref().as_string_ptr() {
                Some(unsafe { (*(sp as *const String)).clone() })
            } else {
                None
            }
        }).unwrap_or_default();
        let pass_str = password.and_then(|v| {
            if let Some(sp) = v.deref().as_string_ptr() {
                Some(unsafe { (*(sp as *const String)).clone() })
            } else {
                None
            }
        }).unwrap_or_default();
        let url = format!("mysql://{}:{}@localhost:3306/test", user_str, pass_str);
        let opts = mysql::Opts::from_url(&url)
            .map_err(|e| format!("PDOException: MySQL connection error: {}", e))?;
        let pool = mysql::Pool::new(opts)
            .map_err(|e| format!("PDOException: MySQL pool error: {}", e))?;
        Arc::new(Mutex::new(DbConnection::Mysql(pool)))
    } else {
        let conn = rusqlite::Connection::open_in_memory()
            .map_err(|e| format!("PDOException: SQLite connection error: {}", e))?;
        Arc::new(Mutex::new(DbConnection::Sqlite(conn)))
    };

    if !is_memory_sqlite {
        DSN_POOLS.insert(dsn_str.to_string(), conn.clone());
    }
    Ok(conn)
}

fn get_connection(conn_id: u64) -> Option<Arc<Mutex<DbConnection>>> {
    CONNECTIONS.get(&conn_id).map(|r| r.value().clone())
}

fn insert_statement(stmt_id: u64, stmt: PreparedStatement) {
    if STATEMENTS.len() >= 256 {
        STATEMENTS.clear();
    }
    STATEMENTS.insert(stmt_id, stmt);
}

fn get_conn_id(this: Option<&Value>) -> u64 {
    if let Some(this_val) = this {
        if let Some(obj_ptr) = this_val.deref().as_object_ptr() {
            let obj = unsafe { &*(obj_ptr as *const PhpObject) };
            if let Some(id_val) = obj.properties.get("conn_id") {
                return id_val.deref().as_int().unwrap_or(0) as u64;
            }
        }
    }
    0
}

fn get_stmt_id(this: Option<&Value>) -> u64 {
    if let Some(this_val) = this {
        if let Some(obj_ptr) = this_val.deref().as_object_ptr() {
            let obj = unsafe { &*(obj_ptr as *const PhpObject) };
            if let Some(id_val) = obj.properties.get("stmt_id") {
                return id_val.deref().as_int().unwrap_or(0) as u64;
            }
        }
    }
    0
}

// ============================================================
// PDO Methods
// ============================================================

php_function! {
    native_pdo_construct(this: Value, dsn: String, username: Value, password: Value, options: Value) {
        let dsn_str = dsn.cloned().unwrap_or_default();
        let conn_id = next_id();

        let conn_arc = get_or_create_connection(&dsn_str, username, password)?;
        if CONNECTIONS.len() >= 1024 {
            CONNECTIONS.clear();
        }
        CONNECTIONS.insert(conn_id, conn_arc);

        if let Some(obj_ptr) = this.and_then(|v| v.as_object_ptr()) {
            unsafe {
                let obj = &mut *(obj_ptr as *mut PhpObject);
                obj.properties.insert("conn_id".to_string(), Value::new_int(conn_id as i32));
                obj.properties.insert("in_transaction".to_string(), Value::new_bool(false));
                // Default ATTR_ERRMODE (3) => ERRMODE_SILENT (0)
                obj.properties.insert("attr_errmode".to_string(), Value::new_int(0));
                if dsn_str.starts_with("sqlite:") {
                    let dsn_boxed = crate::into_raw(Box::new(dsn_str)) as *mut ();
                    obj.properties.insert("dsn".to_string(), Value::new_string_ptr(dsn_boxed));
                }
            }
        }

        Ok(Value::null())
    }
}

php_function! {
    native_pdo_connect(this: Value, dsn: String, username: Value, password: Value, options: Value) |ctx| {
        let dsn_str = dsn.cloned().unwrap_or_default();
        let conn_id = next_id();

        let conn_arc = get_or_create_connection(&dsn_str, username, password)?;
        if CONNECTIONS.len() >= 1024 {
            CONNECTIONS.clear();
        }
        CONNECTIONS.insert(conn_id, conn_arc);

        let pdo_class_id = ctx.get_class_id("PDO").unwrap_or(0);
        let mut pdo_obj = PhpObject::new(pdo_class_id);
        pdo_obj.properties.insert("conn_id".to_string(), Value::new_int(conn_id as i32));
        pdo_obj.properties.insert("in_transaction".to_string(), Value::new_bool(false));
        pdo_obj.properties.insert("attr_errmode".to_string(), Value::new_int(0));
        let boxed_obj = ctx.get_arena().alloc_and_track(pdo_obj);
        Ok(Value::new_object_ptr(boxed_obj as *mut ()))
    }
}

php_function! {
    native_pdo_exec(this: Value, statement: String) |ctx| {
        let conn_id = get_conn_id(this);
        let sql = statement.cloned().unwrap_or_default();
        let task_id = next_id();

        let conn_arc = get_connection(conn_id);

        let callback = ctx.get_db_completion_callback();

        if let Some(conn_arc) = conn_arc {
            let conn_guard = conn_arc.lock().unwrap();
            match &*conn_guard {
                DbConnection::Sqlite(conn) => {
                    match conn.execute_batch(&sql) {
                        Ok(_) => {
                            hyperion_core::DB_MUTATION_VERSION.fetch_add(1, std::sync::atomic::Ordering::Release);
                            let changes = conn.changes();
                            return Ok(Value::new_int(changes as i32));
                        }
                        Err(e) => return Err(format!("PDOException: SQL error: {}", e)),
                    }
                }
                DbConnection::Mysql(pool) => {
                    let pool = pool.clone();
                    DB_POOL.execute(move || {
                        use mysql::prelude::Queryable;
                        let res = match pool.get_conn() {
                            Ok(mut conn) => match conn.query_drop(&sql) {
                                Ok(_) => Ok(Value::new_int(conn.affected_rows() as i32)),
                                Err(e) => Err(format!("PDOException: MySQL query error: {}", e)),
                            },
                            Err(e) => Err(format!("PDOException: MySQL conn error: {}", e)),
                        };
                        callback(task_id, res);
                    });
                    return Ok(Value::new_yield(task_id));
                }
            }
        } else {
            Err("PDOException: Database handle is not connected".to_string())
        }
    }
}

php_function! {
    native_pdo_query(this: Value, statement: String) |ctx| {
        let conn_id = get_conn_id(this);
        let sql = statement.cloned().unwrap_or_default();
        let stmt_id = next_id();
        let task_id = next_id();

        let conn_arc = get_connection(conn_id);

        let default_fetch_mode = if let Some(obj_ptr) = this.and_then(|v| v.as_object_ptr()) {
            unsafe {
                let obj = &*(obj_ptr as *const PhpObject);
                obj.properties.get("attr_19").and_then(|v| v.as_int()).unwrap_or(4)
            }
        } else {
            4
        };

        let callback = ctx.get_db_completion_callback();
        let stmt_class_id = ctx.get_class_id("PDOStatement").unwrap_or(0);
        let mut stmt_obj = PhpObject::new(stmt_class_id);
        stmt_obj.properties.insert("stmt_id".to_string(), Value::new_int(stmt_id as i32));
        stmt_obj.properties.insert("conn_id".to_string(), Value::new_int(conn_id as i32));
        let boxed_obj = ctx.get_arena().alloc_and_track(stmt_obj);
        let stmt_val = Value::new_object_ptr(boxed_obj as *mut ());

        if let Some(conn_arc) = conn_arc {
            let conn_guard = conn_arc.lock().unwrap();
            match &*conn_guard {
                DbConnection::Sqlite(conn) => {
                    let mut sqlite_stmt = conn.prepare_cached(&sql).map_err(|e| format!("PDOException: Query prepare error: {}", e))?;
                    let col_count = sqlite_stmt.column_count();
                    let col_names: Vec<String> = sqlite_stmt.column_names().into_iter().map(|s| s.to_string()).collect();
                    let col_key_ids: Vec<usize> = col_names.iter().map(|s| intern_string(s)).collect();

                    let mut rows_data = Vec::new();
                    let mut rows = sqlite_stmt.query([]).map_err(|e| format!("PDOException: Query execution error: {}", e))?;
                    while let Ok(Some(row)) = rows.next() {
                        let mut row_items = Vec::with_capacity(col_count);
                        for i in 0..col_count {
                            let col_val_ref = row.get_ref_unwrap(i);
                            let sql_val = sqlite_value_to_sql_value(col_val_ref);
                            row_items.push(sql_val);
                        }
                        rows_data.push(row_items);
                    }

                    let affected = rows_data.len();
                    insert_statement(stmt_id, PreparedStatement {
                        conn_id,
                        query: sql,
                        bound_params: HashMap::new(),
                        bound_positional: HashMap::new(),
                        cached_rows: Some(CachedResultSet {
                            col_names: Arc::new(col_names),
                            col_key_ids: Arc::new(col_key_ids),
                            rows: rows_data,
                        }),
                        affected_rows: affected,
                        current_cursor: 0,
                        fetch_mode: default_fetch_mode,
                    });

                    return Ok(stmt_val);
                }
                DbConnection::Mysql(pool) => {
                    let pool = pool.clone();
                    let sql_clone = sql.clone();
                    DB_POOL.execute(move || {
                        use mysql::prelude::Queryable;
                        let res = match pool.get_conn() {
                            Ok(mut conn) => {
                                match conn.query_iter(&sql_clone) {
                                    Ok(result) => {
                                        let mut col_names = Vec::new();
                                        let mut rows_data = Vec::new();
                                        for (row_idx, row) in result.flatten().enumerate() {
                                            if row_idx == 0 {
                                                col_names = row.columns_ref().iter().map(|c| c.name_str().into_owned()).collect();
                                            }
                                            let mut row_items = Vec::with_capacity(col_names.len());
                                            for i in 0..col_names.len() {
                                                let val_opt: Option<String> = row.get(i);
                                                let sql_val = match val_opt {
                                                    Some(s) => SqlValue::Text(s),
                                                    None => SqlValue::Null,
                                                };
                                                row_items.push(sql_val);
                                            }
                                            rows_data.push(row_items);
                                        }
                                        let col_key_ids = col_names.iter().map(|s| intern_string(s)).collect();
                                        let affected = rows_data.len();
                                        insert_statement(stmt_id, PreparedStatement {
                                            conn_id,
                                            query: sql_clone,
                                            bound_params: HashMap::new(),
                                            bound_positional: HashMap::new(),
                                            cached_rows: Some(CachedResultSet {
                                                col_names: Arc::new(col_names),
                                                col_key_ids: Arc::new(col_key_ids),
                                                rows: rows_data,
                                            }),
                                            affected_rows: affected,
                                            current_cursor: 0,
                                            fetch_mode: default_fetch_mode,
                                        });

                                        Ok(stmt_val)
                                    }
                                    Err(e) => Err(format!("PDOException: {}", e)),
                                }
                            }
                            Err(e) => Err(format!("PDOException: {}", e)),
                        };
                        callback(task_id, res);
                    });

                    return Ok(Value::new_yield(task_id));
                }
            }
        } else {
            Err("PDOException: Database handle is not connected".to_string())
        }
    }
}

php_function! {
    native_pdo_prepare(this: Value, statement: String, options: Value) |ctx| {
        let conn_id = get_conn_id(this);
        let sql = statement.cloned().unwrap_or_default();
        let stmt_id = next_id();

        let default_fetch_mode = if let Some(obj_ptr) = this.and_then(|v| v.as_object_ptr()) {
            unsafe {
                let obj = &*(obj_ptr as *const PhpObject);
                obj.properties.get("attr_19").and_then(|v| v.as_int()).unwrap_or(4)
            }
        } else {
            4
        };

        insert_statement(stmt_id, PreparedStatement {
            conn_id,
            query: sql,
            bound_params: HashMap::new(),
            bound_positional: HashMap::new(),
            cached_rows: None,
            affected_rows: 0,
            current_cursor: 0,
            fetch_mode: default_fetch_mode,
        });

        let stmt_class_id = ctx.get_class_id("PDOStatement").unwrap_or(0);
        let mut stmt_obj = PhpObject::new(stmt_class_id);
        stmt_obj.properties.insert("stmt_id".to_string(), Value::new_int(stmt_id as i32));
        stmt_obj.properties.insert("conn_id".to_string(), Value::new_int(conn_id as i32));
        let boxed_obj = ctx.get_arena().alloc_and_track(stmt_obj);
        Ok(Value::new_object_ptr(boxed_obj as *mut ()))
    }
}

php_function! {
    native_pdo_last_insert_id(this: Value, name: Value) |ctx| {
        let conn_id = get_conn_id(this);
        let conn_arc = get_connection(conn_id);

        if let Some(conn_arc) = conn_arc {
            let conn_guard = conn_arc.lock().unwrap();
            match &*conn_guard {
                DbConnection::Sqlite(conn) => {
                    let id = conn.last_insert_rowid();
                    let ptr = ctx.get_arena().alloc_and_track(id.to_string());
                    Ok(Value::new_string_ptr(ptr as *mut ()))
                }
                DbConnection::Mysql(pool) => {
                    if let Ok(conn) = pool.get_conn() {
                        let id = conn.last_insert_id();
                        let ptr = ctx.get_arena().alloc_and_track(id.to_string());
                        Ok(Value::new_string_ptr(ptr as *mut ()))
                    } else {
                        let ptr = ctx.get_arena().alloc_and_track("0".to_string());
                        Ok(Value::new_string_ptr(ptr as *mut ()))
                    }
                }
            }
        } else {
            let ptr = ctx.get_arena().alloc_and_track("0".to_string());
            Ok(Value::new_string_ptr(ptr as *mut ()))
        }
    }
}

php_function! {
    native_pdo_begin_transaction(this: Value) {
        let conn_id = get_conn_id(this);
        let conn_arc = get_connection(conn_id);

        if let Some(conn_arc) = conn_arc {
            let conn_guard = conn_arc.lock().unwrap();
            match &*conn_guard {
                DbConnection::Sqlite(conn) => {
                    conn.execute("BEGIN", []).map_err(|e| format!("PDOException: Failed to begin transaction: {}", e))?;
                    if let Some(obj_ptr) = this.and_then(|v| v.deref().as_object_ptr()) {
                        unsafe {
                            let obj = &mut *(obj_ptr as *mut PhpObject);
                            obj.properties.insert("in_transaction".to_string(), Value::new_bool(true));
                        }
                    }
                    Ok(Value::new_bool(true))
                }
                DbConnection::Mysql(pool) => {
                    use mysql::prelude::Queryable;
                    if let Ok(mut conn) = pool.get_conn() {
                        let _ = conn.query_drop("START TRANSACTION");
                        Ok(Value::new_bool(true))
                    } else {
                        Ok(Value::new_bool(false))
                    }
                }
            }
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_pdo_commit(this: Value) {
        let conn_id = get_conn_id(this);
        let conn_arc = get_connection(conn_id);

        if let Some(conn_arc) = conn_arc {
            let conn_guard = conn_arc.lock().unwrap();
            match &*conn_guard {
                DbConnection::Sqlite(conn) => {
                    conn.execute("COMMIT", []).map_err(|e| format!("PDOException: Failed to commit transaction: {}", e))?;
                    if let Some(obj_ptr) = this.and_then(|v| v.deref().as_object_ptr()) {
                        unsafe {
                            let obj = &mut *(obj_ptr as *mut PhpObject);
                            obj.properties.insert("in_transaction".to_string(), Value::new_bool(false));
                        }
                    }
                    Ok(Value::new_bool(true))
                }
                DbConnection::Mysql(pool) => {
                    use mysql::prelude::Queryable;
                    if let Ok(mut conn) = pool.get_conn() {
                        let _ = conn.query_drop("COMMIT");
                        Ok(Value::new_bool(true))
                    } else {
                        Ok(Value::new_bool(false))
                    }
                }
            }
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_pdo_roll_back(this: Value) {
        let conn_id = get_conn_id(this);
        let conn_arc = get_connection(conn_id);

        if let Some(conn_arc) = conn_arc {
            let conn_guard = conn_arc.lock().unwrap();
            match &*conn_guard {
                DbConnection::Sqlite(conn) => {
                    conn.execute("ROLLBACK", []).map_err(|e| format!("PDOException: Failed to rollback transaction: {}", e))?;
                    if let Some(obj_ptr) = this.and_then(|v| v.deref().as_object_ptr()) {
                        unsafe {
                            let obj = &mut *(obj_ptr as *mut PhpObject);
                            obj.properties.insert("in_transaction".to_string(), Value::new_bool(false));
                        }
                    }
                    Ok(Value::new_bool(true))
                }
                DbConnection::Mysql(pool) => {
                    use mysql::prelude::Queryable;
                    if let Ok(mut conn) = pool.get_conn() {
                        let _ = conn.query_drop("ROLLBACK");
                        Ok(Value::new_bool(true))
                    } else {
                        Ok(Value::new_bool(false))
                    }
                }
            }
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_pdo_in_transaction(this: Value) {
        if let Some(obj_ptr) = this.and_then(|v| v.deref().as_object_ptr()) {
            let obj = unsafe { &*(obj_ptr as *const PhpObject) };
            if let Some(in_tx) = obj.properties.get("in_transaction") {
                return Ok(in_tx.deref());
            }
        }
        Ok(Value::new_bool(false))
    }
}

php_function! {
    native_pdo_set_attribute(this: Value, attribute: Value, value: Value) {
        if let (Some(attr_id), Some(val)) = (attribute.and_then(|v| v.deref().as_int()), value) {
            if let Some(obj_ptr) = this.and_then(|v| v.deref().as_object_ptr()) {
                unsafe {
                    let obj = &mut *(obj_ptr as *mut PhpObject);
                    let key = format!("attr_{}", attr_id);
                    obj.properties.insert(key, val.deref());
                }
            }
        }
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_pdo_get_attribute(this: Value, attribute: Value) |ctx| {
        let conn_id = get_conn_id(this);
        let conn_arc = get_connection(conn_id);

        if let Some(attr_id) = attribute.and_then(|v| v.deref().as_int()) {
            if let Some(obj_ptr) = this.and_then(|v| v.deref().as_object_ptr()) {
                let obj = unsafe { &*(obj_ptr as *const PhpObject) };
                let key = format!("attr_{}", attr_id);
                if let Some(val) = obj.properties.get(&key) {
                    return Ok(val.deref());
                }
            }

            match attr_id {
                4 | 5 => {
                    let ver = if let Some(conn_arc) = conn_arc {
                        let conn_guard = conn_arc.lock().unwrap();
                        match &*conn_guard {
                            DbConnection::Sqlite(_) => rusqlite::version().to_string(),
                            DbConnection::Mysql(_) => "8.0.32".to_string(),
                        }
                    } else {
                        rusqlite::version().to_string()
                    };
                    let ptr = ctx.get_arena().alloc_and_track(ver);
                    return Ok(Value::new_string_ptr(ptr as *mut ()));
                }
                16 => {
                    let driver = if let Some(conn_arc) = conn_arc {
                        let conn_guard = conn_arc.lock().unwrap();
                        match &*conn_guard {
                            DbConnection::Sqlite(_) => "sqlite".to_string(),
                            DbConnection::Mysql(_) => "mysql".to_string(),
                        }
                    } else {
                        "sqlite".to_string()
                    };
                    let ptr = ctx.get_arena().alloc_and_track(driver);
                    return Ok(Value::new_string_ptr(ptr as *mut ()));
                }
                3 => {
                    return Ok(Value::new_int(2)); // PDO::ERRMODE_EXCEPTION
                }
                _ => {}
            }
        }
        Ok(Value::null())
    }
}

// ============================================================
// PDOStatement Methods
// ============================================================

php_function! {
    native_pdostatement_execute(this: Value, params: Value) |ctx| {
        let stmt_id = get_stmt_id(this);
        let task_id = next_id();
        
        let (conn_id, query, mut bound_named, mut bound_pos) = if let Some(stmt) = STATEMENTS.get(&stmt_id) {
            (stmt.conn_id, stmt.query.clone(), stmt.bound_params.clone(), stmt.bound_positional.clone())
        } else {
            return Err("PDOException: Invalid prepared statement".to_string());
        };

        // Merge runtime execution parameters if provided
        if let Some(params_val) = params {
            let params_val = params_val.deref();
            if let Some(arr_ptr) = params_val.as_array_ptr() {
                let arr = unsafe { &*(arr_ptr as *const PhpArray) };
                if arr.is_packed {
                    for (idx, v) in arr.packed.iter().enumerate() {
                        bound_pos.insert(idx, SqlValue::from_php(*v));
                    }
                } else {
                    for (k, v) in &arr.elements {
                        let v_sql = SqlValue::from_php(*v);
                        match k {
                            ArrayKey::StringId(id) => {
                                if let Some(name) = ctx.lookup_string(*id) {
                                    let key_formatted = if name.starts_with(':') { name } else { format!(":{}", name) };
                                    bound_named.insert(key_formatted, v_sql);
                                }
                            }
                            ArrayKey::Int(idx) => {
                                bound_pos.insert(*idx as usize, v_sql);
                            }
                        }
                    }
                }
            }
        }

        let conn_arc = get_connection(conn_id);

        if let Some(conn_arc) = conn_arc {
            let mut conn_guard = conn_arc.lock().unwrap();
            match &mut *conn_guard {
                DbConnection::Sqlite(conn) => {
                    let prepare_res = conn.prepare_cached(&query);
                    match prepare_res {
                        Ok(mut sqlite_stmt) => {
                            let param_count = sqlite_stmt.parameter_count();
                            let is_one_based = bound_pos.contains_key(&1) && !bound_pos.contains_key(&0);
                            let null_val = SqlValue::Null;
                            let mut pos_params: Vec<&dyn ToSql> = Vec::with_capacity(param_count);
                            if bound_named.is_empty() {
                                for i in 0..param_count {
                                    let key = if is_one_based { i + 1 } else { i };
                                    if let Some(val) = bound_pos.get(&key) {
                                        pos_params.push(val as &dyn ToSql);
                                    } else {
                                        pos_params.push(&null_val as &dyn ToSql);
                                    }
                                }
                            }

                            let named_slice: Vec<(&str, &dyn ToSql)> = if !bound_named.is_empty() {
                                bound_named.iter().map(|(n, v)| (n.as_str(), v as &dyn ToSql)).collect()
                            } else {
                                Vec::new()
                            };

                            let is_select = {
                                let trimmed = query.trim_start().to_uppercase();
                                trimmed.starts_with("SELECT") || trimmed.starts_with("PRAGMA") || trimmed.starts_with("EXPLAIN")
                            };

                            if is_select {
                                let col_count = sqlite_stmt.column_count();
                                let col_names: Vec<String> = sqlite_stmt.column_names().into_iter().map(|s| s.to_string()).collect();
                                let col_key_ids: Vec<usize> = col_names.iter().map(|s| intern_string(s)).collect();
                                let mut rows_data = Vec::new();

                                let query_res = if !named_slice.is_empty() {
                                    sqlite_stmt.query(named_slice.as_slice())
                                } else if !pos_params.is_empty() {
                                    sqlite_stmt.query(pos_params.as_slice())
                                } else {
                                    sqlite_stmt.query([])
                                };

                                match query_res {
                                    Ok(mut rows) => {
                                        while let Ok(Some(row)) = rows.next() {
                                            let mut row_items = Vec::with_capacity(col_count);
                                            for i in 0..col_count {
                                                let col_val_ref = row.get_ref_unwrap(i);
                                                let sql_val = sqlite_value_to_sql_value(col_val_ref);
                                                row_items.push(sql_val);
                                            }
                                            rows_data.push(row_items);
                                        }

                                        let total = rows_data.len();
                                        if let Some(mut stmt) = STATEMENTS.get_mut(&stmt_id) {
                                            stmt.cached_rows = Some(CachedResultSet {
                                                col_names: Arc::new(col_names),
                                                col_key_ids: Arc::new(col_key_ids),
                                                rows: rows_data,
                                            });
                                            stmt.affected_rows = total;
                                            stmt.current_cursor = 0;
                                        }
                                        Ok(Value::new_bool(true))
                                    }
                                    Err(e) => Err(format!("PDOException: {}", e)),
                                }
                            } else {
                                let exec_res = if !named_slice.is_empty() {
                                    sqlite_stmt.execute(named_slice.as_slice())
                                } else if !pos_params.is_empty() {
                                    sqlite_stmt.execute(pos_params.as_slice())
                                } else {
                                    sqlite_stmt.execute([])
                                };

                                match exec_res {
                                    Ok(affected) => {
                                        hyperion_core::DB_MUTATION_VERSION.fetch_add(1, std::sync::atomic::Ordering::Release);
                                        if let Some(mut stmt) = STATEMENTS.get_mut(&stmt_id) {
                                            stmt.cached_rows = None;
                                            stmt.affected_rows = affected;
                                            stmt.current_cursor = 0;
                                        }
                                        Ok(Value::new_bool(true))
                                    }
                                    Err(e) => Err(format!("PDOException: {}", e)),
                                }
                            }
                        }
                        Err(e) => Err(format!("PDOException: Statement prepare error: {}", e)),
                    }
                }
                DbConnection::Mysql(pool) => {
                    let pool = pool.clone();
                    let callback = ctx.get_db_completion_callback();
                    drop(conn_guard);
                    DB_POOL.execute(move || {
                        use mysql::prelude::Queryable;
                        let res = match pool.get_conn() {
                            Ok(mut conn) => {
                                match conn.query_drop(&query) {
                                    Ok(_) => Ok(Value::new_bool(true)),
                                    Err(e) => Err(format!("PDOException: {}", e)),
                                }
                            }
                            Err(e) => Err(format!("PDOException: {}", e)),
                        };
                        callback(task_id, res);
                    });
                    Ok(Value::new_yield(task_id))
                }
            }
        } else {
            Err("PDOException: Database handle is not connected".to_string())
        }
    }
}

php_function! {
    native_pdostatement_fetchall(this: Value, fetch_style: Value) |ctx| {
        let stmt_id = get_stmt_id(this);
        let default_fetch_mode = STATEMENTS.get(&stmt_id).map(|s| s.fetch_mode).unwrap_or(2);
        let style = fetch_style.and_then(|v| v.as_int()).unwrap_or(default_fetch_mode);

        let rows_opt = STATEMENTS.get_mut(&stmt_id).and_then(|mut s| s.cached_rows.take());

        let mut result_arr = PhpArray::new();

        if let Some(CachedResultSet { col_names, col_key_ids, rows }) = rows_opt {
            for (row_idx, row) in rows.into_iter().enumerate() {
                match style {
                    5 => { // FETCH_OBJ
                        let stdclass_id = ctx.get_class_id("stdClass").unwrap_or(0);
                        let mut obj = PhpObject::new(stdclass_id);
                        for (col_idx, val) in row.into_iter().enumerate() {
                            let col_name = &col_names[col_idx];
                            let php_val = sql_value_into_php_value(val, ctx);
                            obj.properties.insert(col_name.clone(), php_val);
                        }
                        let boxed_obj = ctx.get_arena().alloc_and_track(obj);
                        result_arr.insert_int(row_idx as i64, Value::new_object_ptr(boxed_obj as *mut ()));
                    }
                    2 => { // FETCH_ASSOC
                        let mut row_arr = PhpArray::new();
                        for (col_idx, val) in row.into_iter().enumerate() {
                            let key_id = col_key_ids[col_idx];
                            let php_val = sql_value_into_php_value(val, ctx);
                            row_arr.insert_string_id(key_id, php_val);
                        }
                        let boxed_row = ctx.get_arena().alloc_and_track(row_arr);
                        result_arr.insert_int(row_idx as i64, Value::new_array_ptr(boxed_row as *mut ()));
                    }
                    3 => { // FETCH_NUM
                        let mut row_arr = PhpArray::new();
                        for (col_idx, val) in row.into_iter().enumerate() {
                            let php_val = sql_value_into_php_value(val, ctx);
                            row_arr.insert_int(col_idx as i64, php_val);
                        }
                        let boxed_row = ctx.get_arena().alloc_and_track(row_arr);
                        result_arr.insert_int(row_idx as i64, Value::new_array_ptr(boxed_row as *mut ()));
                    }
                    _ => { // FETCH_BOTH (default)
                        let mut row_arr = PhpArray::new();
                        for (col_idx, val) in row.into_iter().enumerate() {
                            let key_id = col_key_ids[col_idx];
                            let php_val = sql_value_into_php_value(val, ctx);
                            row_arr.insert_string_id(key_id, php_val);
                            row_arr.insert_int(col_idx as i64, php_val);
                        }
                        let boxed_row = ctx.get_arena().alloc_and_track(row_arr);
                        result_arr.insert_int(row_idx as i64, Value::new_array_ptr(boxed_row as *mut ()));
                    }
                }
            }
        }

        let boxed_res = ctx.get_arena().alloc_and_track(result_arr);
        Ok(Value::new_array_ptr(boxed_res as *mut ()))
    }
}

php_function! {
    native_pdostatement_fetch(this: Value, fetch_style: Value) |ctx| {
        let stmt_id = get_stmt_id(this);
        let default_fetch_mode = STATEMENTS.get(&stmt_id).map(|s| s.fetch_mode).unwrap_or(2);
        let style = fetch_style.and_then(|v| v.as_int()).unwrap_or(default_fetch_mode);

        let row_opt = STATEMENTS.get_mut(&stmt_id).and_then(|mut s| {
            if let Some(ref set) = s.cached_rows {
                if s.current_cursor < set.rows.len() {
                    let row = set.rows[s.current_cursor].clone();
                    let col_names = set.col_names.clone();
                    let col_key_ids = set.col_key_ids.clone();
                    s.current_cursor += 1;
                    Some((col_names, col_key_ids, row))
                } else {
                    None
                }
            } else {
                None
            }
        });

        if let Some((col_names, col_key_ids, row)) = row_opt {
            match style {
                5 => { // FETCH_OBJ
                    let stdclass_id = ctx.get_class_id("stdClass").unwrap_or(0);
                    let mut obj = PhpObject::new(stdclass_id);
                    for (col_idx, val) in row.into_iter().enumerate() {
                        let col_name = &col_names[col_idx];
                        let php_val = sql_value_into_php_value(val, ctx);
                        obj.properties.insert(col_name.clone(), php_val);
                    }
                    let boxed_obj = ctx.get_arena().alloc_and_track(obj);
                    Ok(Value::new_object_ptr(boxed_obj as *mut ()))
                }
                2 => { // FETCH_ASSOC
                    let mut row_arr = PhpArray::new();
                    for (col_idx, val) in row.into_iter().enumerate() {
                        let key_id = col_key_ids[col_idx];
                        let php_val = sql_value_into_php_value(val, ctx);
                        row_arr.insert_string_id(key_id, php_val);
                    }
                    let boxed_row = ctx.get_arena().alloc_and_track(row_arr);
                    Ok(Value::new_array_ptr(boxed_row as *mut ()))
                }
                3 => { // FETCH_NUM
                    let mut row_arr = PhpArray::new();
                    for (col_idx, val) in row.into_iter().enumerate() {
                        let php_val = sql_value_into_php_value(val, ctx);
                        row_arr.insert_int(col_idx as i64, php_val);
                    }
                    let boxed_row = ctx.get_arena().alloc_and_track(row_arr);
                    Ok(Value::new_array_ptr(boxed_row as *mut ()))
                }
                _ => { // FETCH_BOTH
                    let mut row_arr = PhpArray::new();
                    for (col_idx, val) in row.into_iter().enumerate() {
                        let key_id = col_key_ids[col_idx];
                        let php_val = sql_value_into_php_value(val, ctx);
                        row_arr.insert_string_id(key_id, php_val);
                        row_arr.insert_int(col_idx as i64, php_val);
                    }
                    let boxed_row = ctx.get_arena().alloc_and_track(row_arr);
                    Ok(Value::new_array_ptr(boxed_row as *mut ()))
                }
            }
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_pdostatement_fetch_column(this: Value, column_number: Value) |ctx| {
        let stmt_id = get_stmt_id(this);
        let col_idx = column_number.and_then(|v| v.as_int()).unwrap_or(0) as usize;

        let val_opt = STATEMENTS.get_mut(&stmt_id).and_then(|mut s| {
            if let Some(ref set) = s.cached_rows {
                if s.current_cursor < set.rows.len() {
                    let val = set.rows[s.current_cursor].get(col_idx).cloned();
                    s.current_cursor += 1;
                    val
                } else {
                    None
                }
            } else {
                None
            }
        });

        if let Some(sql_val) = val_opt {
            Ok(sql_value_into_php_value(sql_val, ctx))
        } else {
            Ok(Value::new_bool(false))
        }
    }
}

php_function! {
    native_pdostatement_row_count(this: Value) {
        let stmt_id = get_stmt_id(this);
        let count = STATEMENTS.get(&stmt_id).map(|s| s.affected_rows).unwrap_or(0);
        Ok(Value::new_int(count as i32))
    }
}

php_function! {
    native_pdostatement_bind_value(this: Value, parameter: Value, value: Value, data_type: Value) {
        let stmt_id = get_stmt_id(this);
        if let (Some(param_val), Some(val)) = (parameter, value) {
            let param_val = param_val.deref();
            let sql_val = SqlValue::from_php(*val);
            if let Some(mut stmt) = STATEMENTS.get_mut(&stmt_id) {
                if let Some(s_ptr) = param_val.as_string_ptr() {
                    let name = unsafe { (*(s_ptr as *const String)).clone() };
                    let key = if name.starts_with(':') { name } else { format!(":{}", name) };
                    stmt.bound_params.insert(key, sql_val);
                } else if let Some(i) = param_val.as_int() {
                    stmt.bound_positional.insert(i as usize, sql_val);
                }
            }
        }
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_pdostatement_bind_param(this: Value, parameter: Value, variable: Value, data_type: Value) {
        let stmt_id = get_stmt_id(this);
        if let (Some(param_val), Some(val)) = (parameter, variable) {
            let param_val = param_val.deref();
            let sql_val = SqlValue::from_php(*val);
            if let Some(mut stmt) = STATEMENTS.get_mut(&stmt_id) {
                if let Some(s_ptr) = param_val.as_string_ptr() {
                    let name = unsafe { (*(s_ptr as *const String)).clone() };
                    let key = if name.starts_with(':') { name } else { format!(":{}", name) };
                    stmt.bound_params.insert(key, sql_val);
                } else if let Some(i) = param_val.as_int() {
                    stmt.bound_positional.insert(i as usize, sql_val);
                }
            }
        }
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_pdostatement_set_fetch_mode(this: Value, mode: Value) {
        let stmt_id = get_stmt_id(this);
        let m_opt = mode.and_then(|v| v.as_int().or_else(|| v.as_float().map(|f| f as i32)).or_else(|| v.as_string_ptr().and_then(|p| unsafe { (*(p as *const String)).parse::<i32>().ok() })));
        if let Some(m) = m_opt {
            if let Some(mut stmt) = STATEMENTS.get_mut(&stmt_id) {
                stmt.fetch_mode = m;
            }
        }
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_pdo_get_available_drivers() |ctx| {
        let mut arr = PhpArray::new();
        let drivers = ["sqlite", "mysql", "pgsql"];
        for (i, &d) in drivers.iter().enumerate() {
            let ptr = ctx.get_arena().alloc_and_track(d.to_string());
            arr.insert_int(i as i64, Value::new_string_ptr(ptr as *mut ()));
        }
        let ptr = ctx.get_arena().alloc_and_track(arr);
        Ok(Value::new_array_ptr(ptr as *mut ()))
    }
}

php_function! {
    native_pdostatement_close_cursor(this: Value) {
        let stmt_id = get_stmt_id(this);
        if let Some(mut stmt) = STATEMENTS.get_mut(&stmt_id) {
            stmt.cached_rows = None;
            stmt.current_cursor = 0;
        }
        Ok(Value::new_bool(true))
    }
}

php_function! {
    native_pdostatement_destruct(this: Value) {
        let stmt_id = get_stmt_id(this);
        STATEMENTS.remove(&stmt_id);
        Ok(Value::null())
    }
}

php_function! {
    native_pdo_destruct(this: Value) {
        let conn_id = get_conn_id(this);
        CONNECTIONS.remove(&conn_id);
        Ok(Value::null())
    }
}
