use bumpalo::Bump;
use crate::types::array::PhpArray;
use crate::types::object::PhpObject;
use std::cell::{Cell, UnsafeCell};

thread_local! {
    static CURRENT_ARENA: Cell<*const GcArena> = const { Cell::new(std::ptr::null()) };
}

pub struct ArenaScope {
    prev: *const GcArena,
}

impl Drop for ArenaScope {
    fn drop(&mut self) {
        CURRENT_ARENA.with(|cell| cell.set(self.prev));
    }
}

/// Helper function that converts a Box<T> into a raw pointer *mut T.
/// If an active GcArena is set on the current thread, the pointer is automatically
/// registered in its drop_list to be deallocated on arena reset / checkpoint restore.
/// If no arena is active, the raw pointer is returned without registration.
#[inline(always)]
pub fn into_raw<T: 'static>(b: Box<T>) -> *mut T {
    let ptr = Box::into_raw(b);
    CURRENT_ARENA.with(|cell| {
        let arena_ptr = cell.get();
        if !arena_ptr.is_null() {
            unsafe {
                (*arena_ptr).track_boxed(ptr);
            }
        }
    });
    ptr
}

#[derive(Debug, Clone, Copy)]
pub struct ArenaCheckpoint {
    pub drop_list_len: usize,
}

/// A per-request Memory Arena (Gen0 Scavenger).
/// Matches PHP's "Shared-Nothing" architecture.
/// Fibres are executed exclusively on a single worker thread at any given time,
/// making UnsafeCell pointer bumps completely lock-free.
pub struct GcArena {
    boot_bump: UnsafeCell<Option<Bump>>,
    request_bump: UnsafeCell<Bump>,
    drop_list: UnsafeCell<Vec<( *mut (), unsafe fn(*mut ()) )>>,
}

unsafe impl Send for GcArena {}
unsafe impl Sync for GcArena {}

impl Default for GcArena {
    fn default() -> Self {
        Self::new()
    }
}

impl GcArena {
    pub fn new() -> Self {
        Self {
            boot_bump: UnsafeCell::new(None),
            request_bump: UnsafeCell::new(Bump::new()),
            drop_list: UnsafeCell::new(Vec::new()),
        }
    }

    /// Sets this arena as the active arena on the current thread.
    /// Returns an `ArenaScope` RAII guard that restores the previous arena when dropped.
    #[inline(always)]
    pub fn enter(&self) -> ArenaScope {
        let prev = CURRENT_ARENA.with(|cell| {
            let prev = cell.get();
            cell.set(self as *const GcArena);
            prev
        });
        ArenaScope { prev }
    }

    /// Allocates any type `T` into the Arena and returns a raw pointer without drop registration.
    #[inline(always)]
    pub fn alloc_raw<T>(&self, val: T) -> *mut T {
        let bump = unsafe { &mut *self.request_bump.get() };
        let reference = bump.alloc(val);
        reference as *mut T
    }

    /// Allocates any type `T` into the Arena and returns a raw pointer.
    /// If `T` needs to be dropped (like `PhpArray`, `PhpObject`, etc.), it is automatically
    /// registered to be dropped when the arena resets.
    #[inline(always)]
    pub fn alloc<T: 'static>(&self, val: T) -> *mut T {
        if std::mem::needs_drop::<T>() {
            self.alloc_and_track(val)
        } else {
            self.alloc_raw(val)
        }
    }

    /// Allocates an item and registers it to be dropped when the arena resets.
    #[inline(always)]
    pub fn alloc_and_track<T: 'static>(&self, val: T) -> *mut T {
        let ptr = self.alloc_raw(val);
        unsafe fn drop_item<T>(ptr: *mut ()) {
            std::ptr::drop_in_place(ptr as *mut T);
        }
        let drop_list = unsafe { &mut *self.drop_list.get() };
        drop_list.push((ptr as *mut (), drop_item::<T>));
        ptr
    }

    /// Registers an externally-allocated raw pointer (e.g. from Box::into_raw)
    /// for cleanup when the arena resets. Calls drop_in_place + dealloc on reset.
    #[inline(always)]
    pub fn track_boxed<T>(&self, ptr: *mut T) {
        unsafe fn drop_boxed<T>(ptr: *mut ()) {
            // Reconstruct the Box so it gets properly deallocated
            let _ = Box::from_raw(ptr as *mut T);
        }
        let drop_list = unsafe { &mut *self.drop_list.get() };
        drop_list.push((ptr as *mut (), drop_boxed::<T>));
    }

    /// Allocates an array on the Fibre's local memory arena
    #[inline(always)]
    pub fn alloc_array(&self) -> *mut PhpArray {
        self.alloc_and_track(PhpArray::new())
    }
    
    /// Allocates an object on the Fibre's local memory arena
    #[inline(always)]
    pub fn alloc_object(&self, class_id: usize) -> *mut PhpObject {
        self.alloc_and_track(PhpObject::new(class_id))
    }

    /// Returns the total bytes currently allocated in this arena.
    #[inline(always)]
    pub fn allocated_bytes(&self) -> usize {
        let req = unsafe { (*self.request_bump.get()).allocated_bytes() };
        let boot = unsafe { (*self.boot_bump.get()).as_ref().map(|b| b.allocated_bytes()).unwrap_or(0) };
        req + boot
    }

    #[inline(always)]
    pub fn create_checkpoint(&self) -> ArenaCheckpoint {
        let drop_list_len = unsafe { (*self.drop_list.get()).len() };
        unsafe {
            let current = std::mem::replace(&mut *self.request_bump.get(), Bump::new());
            *self.boot_bump.get() = Some(current);
        }
        ArenaCheckpoint { drop_list_len }
    }

    #[inline(always)]
    pub fn drop_list_len(&self) -> usize {
        unsafe { (*self.drop_list.get()).len() }
    }

    #[inline(always)]
    pub fn reset_to_checkpoint(&self, cp: ArenaCheckpoint) {
        let drop_list = unsafe { &mut *self.drop_list.get() };
        if cp.drop_list_len < drop_list.len() {
            for (ptr, drop_fn) in drop_list.drain(cp.drop_list_len..) {
                unsafe {
                    drop_fn(ptr);
                }
            }
        }

        let bump = unsafe { &mut *self.request_bump.get() };
        bump.reset();

        #[cfg(target_os = "macos")]
        unsafe {
            extern "C" {
                fn malloc_zone_pressure_relief(zone: *mut std::ffi::c_void, goal: usize) -> usize;
            }
            malloc_zone_pressure_relief(std::ptr::null_mut(), 0);
        }
    }

    #[inline(always)]
    pub fn sweep(&self, from_idx: usize, marked: &rustc_hash::FxHashSet<*mut ()>) -> usize {
        let drop_list = unsafe { &mut *self.drop_list.get() };
        if from_idx >= drop_list.len() {
            return 0;
        }
        let mut freed = 0;
        let mut i = from_idx;
        while i < drop_list.len() {
            let (ptr, drop_fn) = drop_list[i];
            if !marked.contains(&ptr) {
                unsafe {
                    drop_fn(ptr);
                }
                drop_list.swap_remove(i);
                freed += 1;
            } else {
                i += 1;
            }
        }
        #[cfg(target_os = "macos")]
        if freed > 0 {
            unsafe {
                extern "C" {
                    fn malloc_zone_pressure_relief(zone: *mut std::ffi::c_void, goal: usize) -> usize;
                }
                malloc_zone_pressure_relief(std::ptr::null_mut(), 0);
            }
        }
        freed
    }

    /// Instantly clears the arena, deallocating everything in O(1) time.
    /// This solves Cyclic Reference leaks and fragmentation.
    #[inline(always)]
    pub fn reset(&self) {
        let drop_list = unsafe { &mut *self.drop_list.get() };
        for (ptr, drop_fn) in drop_list.drain(..) {
            unsafe {
                drop_fn(ptr);
            }
        }
        *drop_list = Vec::with_capacity(1024);
        unsafe {
            *self.boot_bump.get() = None;
            let bump = &mut *self.request_bump.get();
            *bump = Bump::new();
        }

        #[cfg(target_os = "macos")]
        unsafe {
            extern "C" {
                fn malloc_zone_pressure_relief(zone: *mut std::ffi::c_void, goal: usize) -> usize;
            }
            malloc_zone_pressure_relief(std::ptr::null_mut(), 0);
        }
    }
}

impl Drop for GcArena {
    fn drop(&mut self) {
        self.reset();
    }
}



/// A global, thread-safe memory arena for storing long-lived engine state.
/// Useful for compile-time artifacts (classes, functions, default properties)
/// that must outlive individual requests.
pub struct GlobalGcArena {
    bump: std::sync::Mutex<Bump>,
    drop_list: std::sync::Mutex<Vec<( *mut (), unsafe fn(*mut ()) )>>,
}

unsafe impl Send for GlobalGcArena {}
unsafe impl Sync for GlobalGcArena {}

impl Default for GlobalGcArena {
    fn default() -> Self {
        Self::new()
    }
}

impl GlobalGcArena {
    pub fn new() -> Self {
        Self {
            bump: std::sync::Mutex::new(Bump::new()),
            drop_list: std::sync::Mutex::new(Vec::new()),
        }
    }

    pub fn alloc_raw<T>(&self, val: T) -> *mut T {
        let bump = self.bump.lock().unwrap();
        let reference = bump.alloc(val);
        reference as *mut T
    }

    pub fn alloc<T: 'static>(&self, val: T) -> *mut T {
        if std::mem::needs_drop::<T>() {
            self.alloc_and_track(val)
        } else {
            self.alloc_raw(val)
        }
    }

    pub fn alloc_and_track<T: 'static>(&self, val: T) -> *mut T {
        let ptr = self.alloc_raw(val);
        unsafe fn drop_item<T>(ptr: *mut ()) {
            std::ptr::drop_in_place(ptr as *mut T);
        }
        self.drop_list.lock().unwrap().push((ptr as *mut (), drop_item::<T>));
        ptr
    }

    pub fn track_boxed<T>(&self, ptr: *mut T) {
        unsafe fn drop_boxed<T>(ptr: *mut ()) {
            let _ = Box::from_raw(ptr as *mut T);
        }
        self.drop_list.lock().unwrap().push((ptr as *mut (), drop_boxed::<T>));
    }

    pub fn alloc_array(&self) -> *mut PhpArray {
        self.alloc_and_track(PhpArray::new())
    }
    
    pub fn alloc_object(&self, class_id: usize) -> *mut PhpObject {
        self.alloc_and_track(PhpObject::new(class_id))
    }

    pub fn allocated_bytes(&self) -> usize {
        self.bump.lock().unwrap().allocated_bytes()
    }

    pub fn drop_list_len(&self) -> usize {
        self.drop_list.lock().unwrap().len()
    }

    pub fn reset(&self) {
        let mut drop_list = self.drop_list.lock().unwrap();
        for (ptr, drop_fn) in drop_list.drain(..) {
            unsafe {
                drop_fn(ptr);
            }
        }
        self.bump.lock().unwrap().reset();
    }
}

impl Drop for GlobalGcArena {
    fn drop(&mut self) {
        self.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::array::PhpArray;
    use crate::memory::nan_box::Value;

    #[test]
    fn test_gc_arena_allocation() {
        let arena = GcArena::new();
        
        let mut arr = PhpArray::new();
        arr.insert_int(0, Value::new_int(42));
        
        let ptr = arena.alloc(arr);
        assert!(!ptr.is_null());
        assert!(arena.allocated_bytes() > 0);
        
        unsafe {
            assert_eq!((*ptr).get_int(0).unwrap().as_int(), Some(42));
        }
    }
    
    #[test]
    fn test_gc_arena_reset() {
        let arena = GcArena::new();
        let _ptr = arena.alloc(100u64);
        assert!(arena.allocated_bytes() > 0);
        
        arena.reset();
        // Bumpalo retains allocated chunks for future allocations to avoid OS overhead,
        // so we just verify we can allocate again successfully after reset.
        let _ptr2 = arena.alloc(200u64);
        assert!(!_ptr2.is_null());
    }

    #[test]
    fn test_current_arena_and_into_raw() {
        use std::sync::atomic::{AtomicBool, Ordering};

        struct DropDetector(std::sync::Arc<AtomicBool>);
        impl Drop for DropDetector {
            fn drop(&mut self) {
                self.0.store(true, Ordering::SeqCst);
            }
        }

        let dropped = std::sync::Arc::new(AtomicBool::new(false));
        let arena = GcArena::new();
        assert_eq!(arena.drop_list_len(), 0);

        {
            let _scope = arena.enter();
            let ptr = into_raw(Box::new(DropDetector(dropped.clone())));
            assert_eq!(arena.drop_list_len(), 1);
            assert_eq!(dropped.load(Ordering::SeqCst), false);
            // Verify ptr is not null
            assert!(!ptr.is_null());
        }

        // Out of scope, but not dropped until arena reset or reset_to_checkpoint
        assert_eq!(dropped.load(Ordering::SeqCst), false);

        arena.reset();
        assert_eq!(arena.drop_list_len(), 0);
        assert_eq!(dropped.load(Ordering::SeqCst), true);
    }
}
