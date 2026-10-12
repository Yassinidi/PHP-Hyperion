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
    /// Fast direct slot storage for declared class properties (O(1) CPU cycle access)
    pub slots: Vec<Value>,
    /// The dynamic / ad-hoc properties of the object (Safe dynamic fallback)
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
            slots: Vec::with_capacity(8),
            properties: IndexMap::with_hasher(FxBuildHasher),
            destructed: false,
        }
    }

    /// Create a new PhpObject with an explicit class name
    pub fn new_with_name(class_id: usize, class_name: String) -> Self {
        PhpObject {
            class_id,
            class_name: Some(class_name),
            slots: Vec::with_capacity(8),
            properties: IndexMap::with_hasher(FxBuildHasher),
            destructed: false,
        }
    }

    /// Create a new PhpObject with pre-allocated slot capacity
    pub fn with_slots(class_id: usize, slot_count: usize) -> Self {
        let mut slots = Vec::with_capacity(slot_count);
        slots.resize(slot_count, Value::null());
        PhpObject {
            class_id,
            class_name: None,
            slots,
            properties: IndexMap::with_hasher(FxBuildHasher),
            destructed: false,
        }
    }

    #[inline(always)]
    pub fn is_dropped(&self) -> bool {
        self.class_id == usize::MAX
    }

    #[inline(always)]
    pub fn get_slot(&self, slot: usize) -> Option<Value> {
        self.slots.get(slot).copied()
    }

    #[inline(always)]
    pub fn set_slot(&mut self, slot: usize, val: Value) {
        if slot >= self.slots.len() {
            self.slots.resize(slot + 1, Value::null());
        }
        self.slots[slot] = val;
        if let Some((_k, prop_val)) = self.properties.get_index_mut(slot) {
            *prop_val = val;
        }
    }

    #[inline(always)]
    pub fn set_property(&mut self, name: &str, val: Value) {
        if let Some(idx) = self.properties.get_index_of(name) {
            if idx < self.slots.len() {
                self.slots[idx] = val;
            }
            self.properties[idx] = val;
        } else {
            self.properties.insert(name.to_string(), val);
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

impl Drop for PhpObject {
    fn drop(&mut self) {
        self.class_id = usize::MAX;
    }
}
