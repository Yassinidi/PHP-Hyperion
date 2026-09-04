use indexmap::IndexMap;
use rustc_hash::FxBuildHasher;
use crate::memory::nan_box::Value;

// In PHP, keys can be Strings or Integers. 
// For now, we will use an Enum to support both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArrayKey {
    Int(i64),
    StringId(usize),
}


#[repr(C)]
#[derive(Debug, Clone)]
pub struct PhpArray {
    pub packed: Vec<Value>,
    pub is_packed: bool,
    pub next_int_key: i64,
    pub cursor: usize,
    pub elements: IndexMap<ArrayKey, Value, FxBuildHasher>,
}

impl Default for PhpArray {
    fn default() -> Self {
        Self::new()
    }
}

impl PhpArray {
    pub fn new() -> Self {
        Self {
            packed: Vec::new(),
            is_packed: true,
            next_int_key: 0,
            cursor: 0,
            elements: IndexMap::with_hasher(FxBuildHasher),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            packed: Vec::with_capacity(capacity),
            is_packed: true,
            next_int_key: 0,
            cursor: 0,
            elements: IndexMap::with_capacity_and_hasher(capacity, FxBuildHasher),
        }
    }

    pub fn from_elements(elements: IndexMap<ArrayKey, Value, FxBuildHasher>) -> Self {
        let mut next_int_key = 0;
        let mut is_packed = true;
        let mut packed = Vec::with_capacity(elements.len());

        for (expected_idx, k) in elements.keys().enumerate() {
            if let ArrayKey::Int(i) = k {
                if *i == expected_idx as i64 {
                    packed.push(elements[k]);
                } else {
                    is_packed = false;
                    break;
                }
                if *i >= next_int_key {
                    next_int_key = *i + 1;
                }
            } else {
                is_packed = false;
                break;
            }
        }

        if !is_packed {
            packed.clear();
            for k in elements.keys() {
                if let ArrayKey::Int(i) = k {
                    if *i >= next_int_key {
                        next_int_key = *i + 1;
                    }
                }
            }
        }

        Self {
            packed,
            is_packed,
            next_int_key,
            cursor: 0,
            elements,
        }
    }

    pub fn insert_int(&mut self, key: i64, value: Value) {
        if key >= self.next_int_key {
            self.next_int_key = key + 1;
        }

        if self.is_packed {
            if key >= 0 && (key as usize) < self.packed.len() {
                self.packed[key as usize] = value;
                self.elements.insert(ArrayKey::Int(key), value);
                return;
            } else if key >= 0 && (key as usize) == self.packed.len() {
                self.packed.push(value);
                self.elements.insert(ArrayKey::Int(key), value);
                return;
            } else {
                self.is_packed = false;
            }
        }

        self.elements.insert(ArrayKey::Int(key), value);
    }

    #[inline(always)]
    pub fn push(&mut self, value: Value) {
        let next_k = self.next_int_key;
        self.next_int_key += 1;
        if self.is_packed {
            self.packed.push(value);
        }
        self.elements.insert(ArrayKey::Int(next_k), value);
    }

    pub fn pop(&mut self) -> Option<Value> {
        if self.elements.is_empty() {
            return None;
        }
        if let Some((k, v)) = self.elements.pop() {
            if self.is_packed {
                self.packed.pop();
            }
            if let ArrayKey::Int(i) = k {
                if i + 1 == self.next_int_key {
                    let mut max_k = 0;
                    for k in self.elements.keys() {
                        if let ArrayKey::Int(ki) = k {
                            if *ki >= max_k {
                                max_k = *ki + 1;
                            }
                        }
                    }
                    self.next_int_key = max_k;
                }
            }
            Some(v)
        } else {
            None
        }
    }

    pub fn rebuild_packed(&mut self) {
        self.next_int_key = 0;
        self.is_packed = true;
        self.packed.clear();
        for (expected_idx, k) in self.elements.keys().enumerate() {
            if let ArrayKey::Int(i) = k {
                if *i == expected_idx as i64 {
                    self.packed.push(self.elements[k]);
                } else {
                    self.is_packed = false;
                    break;
                }
                if *i >= self.next_int_key {
                    self.next_int_key = *i + 1;
                }
            } else {
                self.is_packed = false;
                break;
            }
        }
        if !self.is_packed {
            self.packed.clear();
            for k in self.elements.keys() {
                if let ArrayKey::Int(i) = k {
                    if *i >= self.next_int_key {
                        self.next_int_key = *i + 1;
                    }
                }
            }
        }
    }

    pub fn insert_string_id(&mut self, key: usize, value: Value) {
        if self.is_packed {
            self.is_packed = false;
        }
        self.elements.insert(ArrayKey::StringId(key), value);
    }

    pub fn insert_key(&mut self, key: ArrayKey, value: Value) {
        match key {
            ArrayKey::Int(i) => self.insert_int(i, value),
            ArrayKey::StringId(s) => self.insert_string_id(s, value),
        }
    }

    #[inline(always)]
    pub fn get_int(&self, key: i64) -> Option<&Value> {
        if self.is_packed && key >= 0 && (key as usize) < self.packed.len() {
            self.packed.get(key as usize)
        } else {
            self.elements.get(&ArrayKey::Int(key))
        }
    }

    #[inline(always)]
    pub fn get_string_id(&self, key: usize) -> Option<&Value> {
        self.elements.get(&ArrayKey::StringId(key))
    }

    #[inline(always)]
    pub fn get_by_str(&self, key: &str) -> Option<&Value> {
        let sid = crate::types::string_table::intern_string(key);
        self.get_string_id(sid)
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    #[inline(always)]
    pub fn next_free_int_key(&self) -> i64 {
        self.next_int_key
    }

    pub fn remove_int(&mut self, key: i64) {
        self.is_packed = false;
        self.elements.shift_remove(&ArrayKey::Int(key));
    }

    pub fn remove_string_id(&mut self, key: usize) {
        self.is_packed = false;
        self.elements.shift_remove(&ArrayKey::StringId(key));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_php_array_offsets() {
        assert_eq!(std::mem::offset_of!(PhpArray, packed), 0);
        assert_eq!(std::mem::offset_of!(PhpArray, is_packed), 24);
    }
}



