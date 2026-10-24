// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! Generic 32-bit Win32 Handle Table.
//!
//! Provides thread-safe or atomic generation of non-zero 32-bit handles (`HWND`, `HDC`,
//! `HGDIOBJ`, `HMENU`, `HCURSOR`, `HICON`) safe for 32-bit x86 GPRs and stack slots.

use std::collections::HashMap;

/// Generic handle table storing values of type `T` keyed by 32-bit integer handles.
#[derive(Debug, Clone)]
pub struct HandleTable<T> {
    base_id: u32,
    next_id: u32,
    entries: HashMap<u32, T>,
}

impl<T> HandleTable<T> {
    /// Creates a new handle table starting allocations at `base_id` (must be non-zero).
    pub fn new(base_id: u32) -> Self {
        assert!(base_id > 0, "base_id must be non-zero");
        Self {
            base_id,
            next_id: base_id,
            entries: HashMap::new(),
        }
    }

    /// Allocates a new handle and associates it with `value`.
    pub fn insert(&mut self, value: T) -> u32 {
        let handle = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        if self.next_id == 0 {
            self.next_id = self.base_id;
        }
        self.entries.insert(handle, value);
        handle
    }

    /// Retrieves an immutable reference to the value associated with `handle`.
    pub fn get(&self, handle: u32) -> Option<&T> {
        self.entries.get(&handle)
    }

    /// Retrieves a mutable reference to the value associated with `handle`.
    pub fn get_mut(&mut self, handle: u32) -> Option<&mut T> {
        self.entries.get_mut(&handle)
    }

    /// Removes and returns the value associated with `handle`.
    pub fn remove(&mut self, handle: u32) -> Option<T> {
        self.entries.remove(&handle)
    }

    /// Checks if `handle` exists in the table.
    pub fn contains(&self, handle: u32) -> bool {
        self.entries.contains_key(&handle)
    }

    /// Returns the number of active entries in the handle table.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if the handle table is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterates over all active handles and values.
    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, u32, T> {
        self.entries.iter()
    }

    /// Clears all entries from the handle table.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.next_id = self.base_id;
    }
}

impl<T> Default for HandleTable<T> {
    fn default() -> Self {
        Self::new(0x0001_0000)
    }
}
