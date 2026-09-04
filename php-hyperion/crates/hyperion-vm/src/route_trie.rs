use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone)]
pub struct RouteMatch {
    pub route_id: u32,
    pub metadata: String,
    pub params: HashMap<String, String>,
}

#[derive(Debug, Clone)]
struct TrieNode {
    is_leaf: bool,
    route_id: u32,
    metadata: String,
    param_name: Option<String>,
    static_children: HashMap<String, TrieNode>,
    param_child: Option<Box<TrieNode>>,
    wildcard_child: Option<Box<TrieNode>>,
}

impl TrieNode {
    fn new() -> Self {
        Self {
            is_leaf: false,
            route_id: 0,
            metadata: String::new(),
            param_name: None,
            static_children: HashMap::new(),
            param_child: None,
            wildcard_child: None,
        }
    }
}

pub struct RadixRouteTree {
    roots: RwLock<HashMap<String, TrieNode>>,
}

impl Default for RadixRouteTree {
    fn default() -> Self {
        Self::new()
    }
}

impl RadixRouteTree {
    pub fn new() -> Self {
        Self {
            roots: RwLock::new(HashMap::new()),
        }
    }

    pub fn insert(&self, method: &str, pattern: &str, route_id: u32, metadata: &str) {
        let method_upper = method.to_uppercase();
        let mut roots = self.roots.write().unwrap();
        let root = roots.entry(method_upper).or_insert_with(TrieNode::new);

        let segments: Vec<&str> = pattern.trim_matches('/').split('/').filter(|s| !s.is_empty()).collect();
        let mut current = root;

        for segment in segments {
            if segment.starts_with('{') && segment.ends_with('}') {
                let param_name = segment[1..segment.len() - 1].to_string();
                if current.param_child.is_none() {
                    let mut node = TrieNode::new();
                    node.param_name = Some(param_name);
                    current.param_child = Some(Box::new(node));
                }
                current = current.param_child.as_mut().unwrap();
            } else if segment == "*" || segment == "**" {
                if current.wildcard_child.is_none() {
                    current.wildcard_child = Some(Box::new(TrieNode::new()));
                }
                current = current.wildcard_child.as_mut().unwrap();
            } else {
                current = current.static_children.entry(segment.to_string()).or_insert_with(TrieNode::new);
            }
        }

        current.is_leaf = true;
        current.route_id = route_id;
        current.metadata = metadata.to_string();
    }

    pub fn match_path(&self, method: &str, path: &str) -> Option<RouteMatch> {
        let method_upper = method.to_uppercase();
        let roots = self.roots.read().unwrap();
        let root = roots.get(&method_upper)?;

        let raw_path = path.split('?').next().unwrap_or(path);
        let segments: Vec<&str> = raw_path.trim_matches('/').split('/').filter(|s| !s.is_empty()).collect();

        let mut params = HashMap::new();
        if self.match_recursive(root, &segments, 0, &mut params) {
            let mut current = root;
            for segment in &segments {
                if let Some(child) = current.static_children.get(*segment) {
                    current = child;
                } else if let Some(ref child) = current.param_child {
                    current = child;
                } else if let Some(ref child) = current.wildcard_child {
                    current = child;
                }
            }
            if current.is_leaf {
                return Some(RouteMatch {
                    route_id: current.route_id,
                    metadata: current.metadata.clone(),
                    params,
                });
            }
        }

        None
    }

    fn match_recursive<'a>(
        &self,
        node: &'a TrieNode,
        segments: &[&str],
        index: usize,
        params: &mut HashMap<String, String>,
    ) -> bool {
        if index == segments.len() {
            return node.is_leaf;
        }

        let segment = segments[index];

        // 1. Try exact static segment
        if let Some(child) = node.static_children.get(segment) {
            if self.match_recursive(child, segments, index + 1, params) {
                return true;
            }
        }

        // 2. Try parameterized segment
        if let Some(ref child) = node.param_child {
            if let Some(ref param_name) = child.param_name {
                params.insert(param_name.clone(), segment.to_string());
                if self.match_recursive(child, segments, index + 1, params) {
                    return true;
                }
                params.remove(param_name);
            }
        }

        // 3. Try wildcard
        if let Some(ref child) = node.wildcard_child {
            if self.match_recursive(child, segments, index + 1, params) {
                return true;
            }
        }

        false
    }
}

pub static GLOBAL_ROUTE_TREE: std::sync::OnceLock<Arc<RadixRouteTree>> = std::sync::OnceLock::new();

pub fn get_global_route_tree() -> &'static Arc<RadixRouteTree> {
    GLOBAL_ROUTE_TREE.get_or_init(|| Arc::new(RadixRouteTree::new()))
}
