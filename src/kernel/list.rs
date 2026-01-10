// Intrusive doubly-linked list implementation
use core::ptr;

/// List node that lives inside the data structure
///
/// This is an intrusive list - the ListNode is embedded directly in
/// the data structure (like TCB), not allocated separately.
#[derive(Debug)]
pub struct ListNode {
    next: *mut ListNode,
    /// Pointer to next node in list
    prev: *mut ListNode,
    /// Pointer to previous node in list
    container: *mut List,
    /// Pointer to the containing list
    /// Sort value (used for priority ordering)
    /// Lower values appear earlier in the list
    value: u64,
    /// Pointer back to owner structure (TCB, etc.)        
    owner: *mut u8,
}

impl ListNode {
    pub const fn new() -> Self {
        ListNode {
            next: ptr::null_mut(),
            prev: ptr::null_mut(),
            container: ptr::null_mut(),
            value: 0,
            owner: ptr::null_mut(),
        }
    }

    pub fn set_value(&mut self, value: u64) {
        self.value = value;
    }

    pub fn get_value(&self) -> u64 {
        self.value
    }

    pub fn set_owner(&mut self, owner: *mut u8) {
        self.owner = owner;
    }

    pub fn is_in_list(&self) -> bool {
        !self.container.is_null()
    }

    pub fn get_container(&self) -> *mut List {
        self.container
    }

    pub fn get_next(&self) -> *mut ListNode {
        self.next
    }

    pub fn get_prev(&self) -> *mut ListNode {
        self.prev
    }

    pub fn get_owner<T>(&self) -> *mut T {
        self.owner as *mut T
    }
}

#[derive(Debug)]
pub struct List {
    length: usize,
    index: *mut ListNode,
    end_marker: ListNode,
}

impl List {
    pub const fn new() -> Self {
        List {
            length: 0,
            index: ptr::null_mut(),
            end_marker: ListNode::new(),
        }
    }

    pub fn init(&mut self) {
        self.end_marker.next = &mut self.end_marker as *mut ListNode;
        self.end_marker.prev = &mut self.end_marker as *mut ListNode;
        self.end_marker.value = u64::MAX;
        self.length = 0;
        self.index = &mut self.end_marker as *mut ListNode;
    }

    pub fn insert_sorted(&mut self, item: &mut ListNode) {
        unsafe {
            let item_value = item.value;
            let mut iterator = &mut self.end_marker as *mut ListNode;

            loop {
                iterator = (*iterator).next;
                if (*iterator).value >= item_value {
                    break;
                }
            }

            item.next = iterator;
            item.prev = (*iterator).prev;

            (*(*iterator).prev).next = item as *mut ListNode;
            (*iterator).prev = item as *mut ListNode;
            item.container = self as *mut List;

            self.length += 1;
        }
    }

    /// Insert item at end of list (O(1))
    pub fn insert_end(&mut self, item: &mut ListNode) {
        unsafe {
            let end_marker = &mut self.end_marker as *mut ListNode;

            item.next = end_marker;
            item.prev = (*end_marker).prev;
            (*(*end_marker).prev).next = item as *mut ListNode;
            (*end_marker).prev = item as *mut ListNode;
            item.container = self as *mut List;

            self.length += 1;
        }
    }

    /// Remove item from list
    ///
    /// Uses prev/next pointer validation instead of container pointer check
    /// This is more reliable as container pointers can become stale
    pub fn remove(&mut self, item: &mut ListNode) -> bool {
        // Check if item has valid prev/next pointers (is linked)
        if item.next.is_null() || item.prev.is_null() {
            return false; // Not in a list
        }

        unsafe {
            // Unlink the item from the list
            (*item.next).prev = item.prev;
            (*item.prev).next = item.next;

            // Reset index if we're removing current item
            if self.index == item as *mut ListNode {
                self.index = item.prev;
            }

            // Clear all pointers to prevent double-removal
            item.container = ptr::null_mut();
            item.next = ptr::null_mut();
            item.prev = ptr::null_mut();

            self.length -= 1;
        }

        true
    }

    /// Get head item (first item, not end marker)
    pub fn get_head(&self) -> Option<&ListNode> {
        if self.is_empty() {
            None
        } else {
            unsafe { Some(&*self.end_marker.next) }
        }
    }

    /// Get head item mutable
    pub fn get_head_mut(&mut self) -> Option<&mut ListNode> {
        if self.is_empty() {
            None
        } else {
            unsafe { Some(&mut *self.end_marker.next) }
        }
    }

    /// Check if list is empty
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// Get number of items in list
    pub fn len(&self) -> usize {
        self.length
    }
}
