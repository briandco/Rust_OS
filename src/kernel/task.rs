use crate::kernel::list::ListNode;
use crate::kernel::types::*;

pub const MAX_TASK_NAME_LEN: usize = 16;

#[repr(C)]
pub struct TaskControlBlock {
    /// Current stack pointer - MUST BE FIRST!
    /// Assembly code depends on this being at offset 0
    pub stack_top: *mut usize,
    // Note: Full context is saved on the stack during context switch
    // We only store the stack pointer here
    /// List node for state list (ready/blocked/suspended)
    pub state_list_item: ListNode,
    /// List node for event list (when blocked on object)
    pub event_list_item: ListNode,
    /// Current priority
    pub priority: Priority,
    /// Base priority (for priority inheritance - Phase 2)
    pub base_priority: Priority,
    /// Task name (for debugging)
    pub name: [u8; MAX_TASK_NAME_LEN],
    /// Stack base pointer (bottom of stack)
    pub stack_base: *mut usize,
    /// Stack size in words
    pub stack_size: StackSize,
    /// Current task state
    pub state: TaskState,
    /// Delay until this tick (for task delays )
    pub delay_until: TickType,
    /// Number of mutexes held (for priority inheritance - Phase 2)
    pub mutexes_held: usize,
}

impl TaskControlBlock {
    /// Create a new TCB
    ///
    /// # Arguments
    /// * `name` - Task name (max 15 chars, null-terminated)
    /// * `priority` - Task priority (0 = lowest/idle, higher = more important)
    /// * `stack` - Pointer to top of initialized stack
    /// * `stack_size` - Size of stack in words
    pub fn new(name: &str, priority: Priority, stack: *mut usize, stack_size: StackSize) -> Self {
        // Validate priority to prevent array out-of-bounds
        assert!(priority < config::MAX_PRIORITIES,
            "Priority {} exceeds maximum allowed priority {}",
            priority,
            config::MAX_PRIORITIES - 1);

        // Validate stack size
        assert!(stack_size >= config::MIN_STACK_SIZE,
            "Stack size {} is below minimum required size {}",
            stack_size,
            config::MIN_STACK_SIZE);

        // Initialize List Node
        let mut state_item = ListNode::new();
        let mut event_item = ListNode::new();

        // Set list item values for priority sorting
        // State list: sorted by priority (lower value = higher priority for ready list)
        // We use MAX_PRIORITIES - priority so higher priority tasks are at the front
        state_item.set_value((config::MAX_PRIORITIES - priority) as u64);

        // Event list: sorted by priority (same scheme)
        event_item.set_value((config::MAX_PRIORITIES - priority) as u64);

        // Copy name with null termination
        let mut name_buf = [0u8; MAX_TASK_NAME_LEN];
        let name_bytes = name.as_bytes();
        let copy_len = core::cmp::min(name_bytes.len(), MAX_TASK_NAME_LEN - 1);
        name_buf[..copy_len].copy_from_slice(&name_bytes[..copy_len]);
        // name_buf is already zero-initialized, so it's null-terminated

        TaskControlBlock {
            stack_top: stack,
            state_list_item: state_item,
            event_list_item: event_item,
            priority,
            base_priority: priority,
            name: name_buf,
            stack_base: stack,
            stack_size,
            state: TaskState::Ready,
            delay_until: TickType::zero(),
            mutexes_held: 0,
        }
    }

    /// Get task name as string
    pub fn name_str(&self) -> &str {
        // Find null terminator
        let len = self
            .name
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(MAX_TASK_NAME_LEN);

        core::str::from_utf8(&self.name[..len]).unwrap_or("<invalid>")
    }

    /// Check if task is ready to run
    pub fn is_ready(&self) -> bool {
        self.state == TaskState::Ready || self.state == TaskState::Running
    }

    /// Check if task is running
    pub fn is_running(&self) -> bool {
        self.state == TaskState::Running
    }

    /// Check if task is blocked
    pub fn is_blocked(&self) -> bool {
        self.state == TaskState::Blocked
    }

    /// Check if task is suspended
    pub fn is_suspended(&self) -> bool {
        self.state == TaskState::Suspended
    }

    /// Update list item owner pointers
    ///
    /// CRITICAL: Must be called IMMEDIATELY after TCB is placed in its final location
    /// (e.g., static mut variable) and before adding to any scheduler lists.
    /// The TCB must NOT move in memory after calling this function.
    ///
    /// # Safety
    /// - TCB must be at its final memory location (won't be moved)
    /// - Must be called before adding task to scheduler
    /// - Calling this function makes the TCB effectively pinned
    ///
    /// # Panics
    /// Panics if called on a TCB that has already been initialized
    pub unsafe fn update_list_item_owners(&mut self) {
        // Safety check: ensure list items don't already have owners set
        assert!(self.state_list_item.get_owner::<TaskControlBlock>().is_null(),
            "TCB list items already initialized! Don't call update_list_item_owners() twice.");

        let tcb_ptr = self as *mut TaskControlBlock;
        self.state_list_item.set_owner(tcb_ptr as *mut u8);
        self.event_list_item.set_owner(tcb_ptr as *mut u8);
    }

    /// Initialize a static TCB and return a mutable reference
    ///
    /// This is a safer wrapper that ensures update_list_item_owners() is called
    /// automatically for static TCBs.
    ///
    /// # Safety
    /// - The provided TCB must be static and never move
    /// - Only call once per static TCB
    pub unsafe fn init_static(tcb: &mut TaskControlBlock) -> &mut TaskControlBlock {
        tcb.update_list_item_owners();
        tcb
    }
}

// Safety: TCB is only accessed from one core in single-core RTOS
// Multi-core support will add proper synchronization
unsafe impl Send for TaskControlBlock {}
