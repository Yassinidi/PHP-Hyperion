use indexmap::IndexMap;
use rustc_hash::FxBuildHasher;
use crate::memory::nan_box::Value;
use serde::ser::{Serialize, Serializer, SerializeMap};

/// Represents a PHP Object in the Virtual Machine
pub struct PhpObject {
    /// Reference to the internal class table ID
    pub class_id: usize,
    /// Explicit class name (used by enum cases and dynamically compiled instances)
    pub class_name: Option<String>,
    /// The properties of the object
    pub properties: IndexMap<String, Value, FxBuildHasher>,
    /// Whether __destruct has been invoked
    pub destructed: bool,
}

impl PhpObject {
    /// Create a new empty PhpObject
    pub fn new(class_id: usize) -> Self {
        PhpObject {
            class_id,
            class_name: None,
            properties: IndexMap::with_hasher(FxBuildHasher),
            destructed: false,
        }
    }

    /// Create a new PhpObject with an explicit class name
    pub fn new_with_name(class_id: usize, class_name: String) -> Self {
        PhpObject {
            class_id,
            class_name: Some(class_name),
            properties: IndexMap::with_hasher(FxBuildHasher),
            destructed: false,
        }
    }
}


impl Serialize for PhpObject {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.properties.len()))?;
        for (k, v) in &self.properties {
            map.serialize_entry(k, v)?;
        }
        map.end()
    }
}

impl Default for PhpObject {
    fn default() -> Self {
        Self::new(0)
    }
}
