use dashmap::DashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

pub struct GlobalStringTable {
    map: DashMap<String, usize>,
    reverse: DashMap<usize, String>,
    next_id: AtomicUsize,
}

impl Default for GlobalStringTable {
    fn default() -> Self {
        Self::new()
    }
}

impl GlobalStringTable {
    pub fn new() -> Self {
        Self {
            map: DashMap::new(),
            reverse: DashMap::new(),
            next_id: AtomicUsize::new(1),
        }
    }

    pub fn intern(&self, s: &str) -> usize {
        if let Some(id) = self.map.get(s) {
            return *id;
        }
        match self.map.entry(s.to_string()) {
            dashmap::mapref::entry::Entry::Occupied(occ) => *occ.get(),
            dashmap::mapref::entry::Entry::Vacant(vac) => {
                let id = self.next_id.fetch_add(1, Ordering::SeqCst);
                let s_owned = vac.key().clone();
                vac.insert(id);
                self.reverse.insert(id, s_owned);
                id
            }
        }
    }

    pub fn get_string(&self, id: usize) -> Option<String> {
        self.reverse.get(&id).map(|s| s.clone())
    }
}

static STRING_TABLE: OnceLock<GlobalStringTable> = OnceLock::new();

pub fn get_string_table() -> &'static GlobalStringTable {
    STRING_TABLE.get_or_init(GlobalStringTable::new)
}

thread_local! {
    static FAST_INTERN_CACHE: std::cell::RefCell<rustc_hash::FxHashMap<String, usize>> = std::cell::RefCell::new(
        rustc_hash::FxHashMap::with_capacity_and_hasher(2048, rustc_hash::FxBuildHasher)
    );
}

pub fn intern_string(s: &str) -> usize {
    FAST_INTERN_CACHE.with(|cache| {
        let mut c = cache.borrow_mut();
        if let Some(&id) = c.get(s) {
            return id;
        }
        let id = get_string_table().intern(s);
        if c.len() < 16384 {
            c.insert(s.to_string(), id);
        }
        id
    })
}

pub fn lookup_string(id: usize) -> Option<String> {
    get_string_table().get_string(id)
}

pub fn preintern_http_symbols() {
    let table = get_string_table();
    let symbols = [
        "REQUEST_METHOD", "REQUEST_URI", "SERVER_PROTOCOL", "PATH_INFO",
        "QUERY_STRING", "HTTP_HOST", "HTTP_USER_AGENT", "HTTP_ACCEPT",
        "HTTP_COOKIE", "HTTP_AUTHORIZATION", "CONTENT_TYPE", "CONTENT_LENGTH",
        "REMOTE_ADDR", "REMOTE_PORT", "SERVER_NAME", "SERVER_PORT", "SCRIPT_NAME",
        "SCRIPT_FILENAME", "DOCUMENT_ROOT", "GET", "POST", "PUT", "DELETE", "PATCH",
        "HEAD", "OPTIONS", "id", "name", "email", "status", "data", "message",
        "success", "error", "headers", "cookies", "session", "user", "app"
    ];
    for s in symbols {
        table.intern(s);
    }
}

