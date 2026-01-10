use crate::kernel::list::List;
use crate::kernel::task::TaskControlBlock;
use crate::kernel::types::*;
use core::ptr;

// Debug output helpers
#[allow(dead_code)]
fn debug_print_ready_lists(scheduler: &Scheduler, label: &str) {
    // This function would need uart access, which we don't have here
    // We'll use the counters instead
}

pub struct Scheduler {
    /// Ready lists - one per priority level
    /// Index 0 = priority 0 (idle task)
    /// Index 31 = priority 31 (highest priority)
    ///
    /// Each list contains tasks at that priority that are ready to run
    ready_lists: [List; config::MAX_PRIORITIES],

    /// Currently running task (single-core for now)
    /// Points to the TCB of the task that's executing
    current_task: *mut TaskControlBlock,

    /// Highest priority level that has ready tasks
    /// Optimization: Don't scan all 32 lists, start from here
    top_ready_priority: Priority,

    /// Total number of tasks in the system
    task_count: usize,

    /// Current system tick count (incremented by timer interrupt)
    tick_count: TickType,

    /// Is the scheduler running?
    scheduler_running: bool,

    /// Scheduler suspension depth (for critical sections)
    /// 0 = not suspended, >0 = suspended
    /// Suspensions nest - must call resume same number of times
    suspend_depth: usize,
}

impl Scheduler {
    pub const fn new() -> Self {
        const EMPTY_LIST: List = List::new();
        Scheduler {
            // Array of 32 empty lists
            ready_lists: [EMPTY_LIST; config::MAX_PRIORITIES],

            // No current task yet
            current_task: ptr::null_mut(),

            // Start at idle priority
            top_ready_priority: config::IDLE_PRIORITY,

            // No tasks yet
            task_count: 0,

            // Time starts at 0
            tick_count: TickType::zero(),

            // Not running yet
            scheduler_running: false,

            // Not suspended
            suspend_depth: 0,
        }
    }

    pub fn init(&mut self) {
        for list in &mut self.ready_lists {
            list.init();
        }

        self.current_task = ptr::null_mut();
        self.top_ready_priority = config::IDLE_PRIORITY;
        self.task_count = 0;
        self.tick_count = TickType::zero();
        self.scheduler_running = false;
        self.suspend_depth = 0;
    }

    pub fn add_task_to_ready_list(&mut self, tcb: &mut TaskControlBlock) {
        tcb.state = TaskState::Ready;
        let priority = tcb.priority;

        self.ready_lists[priority].insert_end(&mut tcb.state_list_item);
        if priority > self.top_ready_priority {
            self.top_ready_priority = priority;
        }
    }

    pub fn remove_task_from_ready_list(&mut self, tcb: &mut TaskControlBlock) -> bool {
        let priority = tcb.priority;

        // Try to remove from the list
        let removed = self.ready_lists[priority].remove(&mut tcb.state_list_item);

        if removed {
            // If we just emptied the top priority list, find new top
            if self.ready_lists[priority].is_empty() && priority == self.top_ready_priority {
                self.update_top_ready_priority();
            }
        }

        removed
    }

    pub fn update_top_ready_priority(&mut self) {
        let mut priority = self.top_ready_priority;

        while priority > config::IDLE_PRIORITY {
            if !self.ready_lists[priority].is_empty() {
                self.top_ready_priority = priority;
                return;
            }
            priority -= 1;
        }
        self.top_ready_priority = config::IDLE_PRIORITY;
    }

    pub fn select_highest_priority_task(&mut self) -> *mut TaskControlBlock {
        // Set previous running task back to Ready state
        if !self.current_task.is_null() {
            unsafe {
                (*self.current_task).state = TaskState::Ready;
            }
        }

        // Start from the highest priority with ready tasks
        let mut priority = self.top_ready_priority;

        loop {
            // Check if this priority level has any ready tasks
            if !self.ready_lists[priority].is_empty() {
                // Get the head of this priority's list
                if let Some(node) = self.ready_lists[priority].get_head() {
                    // Get the TCB that owns this list node
                    let tcb_ptr = node.get_owner::<TaskControlBlock>();

                    if !tcb_ptr.is_null() {
                        unsafe {
                            // Mark this task as Running (keep it in ready list for round-robin)
                            (*tcb_ptr).state = TaskState::Running;
                        }
                        return tcb_ptr;
                    }
                }
            }

            // Move to next lower priority
            if priority == config::IDLE_PRIORITY {
                // We've checked all priorities, no task found
                // This should never happen if idle task exists!
                break;
            }
            priority -= 1;
        }

        // Should never reach here if idle task exists
        ptr::null_mut()
    }

    /// Select the next task to run, ensuring it's DIFFERENT from current task
    ///
    /// This function implements true round-robin behavior by temporarily
    /// removing the current task from the ready list, selecting the next
    /// highest priority task, then re-adding the current task.
    ///
    /// This ensures that yielding actually gives other tasks a chance to run,
    /// even if the current task is the highest priority.
    ///
    /// # Returns
    /// Pointer to the next task's TCB, or current task if no others available
    pub fn select_next_different_task(&mut self) -> *mut TaskControlBlock {
        let current = self.current_task;

        if current.is_null() {
            return self.select_highest_priority_task();
        }

        unsafe {
            let current_ref = &mut *current;
            let priority = current_ref.priority;

            // Debug: Check if task is actually in the list before removing
            let _in_list_before = current_ref.state_list_item.is_in_list();

            // Temporarily remove current task from ready list
            let removed = self.remove_task_from_ready_list(current_ref);

            // CRITICAL DEBUG: The task should be in the list and removable
            assert!(removed, "BUG: Failed to remove current task from ready list!");

            // If remove succeeded, the list at priority 2 should now be empty
            // (since Task1 is the only task at that priority)
            if priority == 2 {
                assert!(self.ready_lists[priority].is_empty(),
                    "BUG: Task removed but list not empty!");
            }

            // Debug: Check list state after attempted removal
            let _list_empty_after_remove = self.ready_lists[priority].is_empty();
            let _in_list_after = current_ref.state_list_item.is_in_list();

            // Now select from remaining tasks (current excluded)
            let next = self.select_highest_priority_task();

            // Add current task back to ready list
            if removed {
                current_ref.state = TaskState::Ready;
                self.add_task_to_ready_list(current_ref);
            }

            // If we found a different task, return it
            if !next.is_null() && next != current {
                return next;
            }

            // No other task found, or somehow got same task
            // Return current task and mark it as running
            current_ref.state = TaskState::Running;
            current
        }
    }

    pub fn yield_task(&mut self) {
        if self.current_task.is_null() {
            return;
        }
        unsafe {
            let current = &mut *self.current_task;

            self.remove_task_from_ready_list(current);

            self.add_task_to_ready_list(current);
        }
    }

    /// Suspend the scheduler (enter critical section)
    ///
    /// Suspensions nest - you must call resume() the same number of times
    /// Used to protect scheduler data structures from interrupts
    pub fn suspend(&mut self) {
        // Disable interrupts on first suspension
        if self.suspend_depth == 0 {
            unsafe {
                // Clear MIE bit in mstatus to disable machine interrupts
                riscv::interrupt::disable();
            }
        }
        self.suspend_depth += 1;
    }

    /// Resume the scheduler (exit critical section)
    ///
    /// Decrements suspension depth and re-enables interrupts when depth reaches 0
    pub fn resume(&mut self) {
        if self.suspend_depth > 0 {
            self.suspend_depth -= 1;
            // Re-enable interrupts when we've fully exited critical section
            if self.suspend_depth == 0 {
                unsafe {
                    // Set MIE bit in mstatus to enable machine interrupts
                    riscv::interrupt::enable();
                }
            }
        }
    }

    /// Check if scheduler is suspended
    pub fn is_suspended(&self) -> bool {
        self.suspend_depth > 0
    }

    /// Get current task pointer
    pub fn get_current_task(&self) -> *mut TaskControlBlock {
        self.current_task
    }

    /// Set current task pointer
    ///
    /// Called by context switcher
    pub fn set_current_task(&mut self, tcb: *mut TaskControlBlock) {
        self.current_task = tcb;
    }

    /// Get current tick count
    pub fn get_tick_count(&self) -> TickType {
        self.tick_count
    }

    /// Increment tick count
    ///
    /// Called by timer interrupt handler (future implementation)
    pub fn increment_tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(TickType::new(1));
    }

    /// Check if scheduler is running
    pub fn is_running(&self) -> bool {
        self.scheduler_running
    }

    /// Mark scheduler as running
    ///
    /// Called when scheduler starts (future implementation)
    pub fn set_running(&mut self, running: bool) {
        self.scheduler_running = running;
    }

    /// Get total task count
    pub fn get_task_count(&self) -> usize {
        self.task_count
    }

    /// Increment task count
    ///
    /// Called when a task is added to the system
    pub fn increment_task_count(&mut self) {
        self.task_count += 1;
    }

    /// Decrement task count
    ///
    /// Called when a task is deleted (future implementation)
    pub fn decrement_task_count(&mut self) {
        if self.task_count > 0 {
            self.task_count -= 1;
        }
    }

    /// Get the top ready priority level
    ///
    /// Useful for debugging
    pub fn get_top_ready_priority(&self) -> Priority {
        self.top_ready_priority
    }

    /// Debug: Check if a specific ready list is empty
    pub fn is_ready_list_empty(&self, priority: Priority) -> bool {
        if priority < config::MAX_PRIORITIES {
            self.ready_lists[priority].is_empty()
        } else {
            true
        }
    }

    /// Debug: Get the number of non-empty ready lists
    pub fn count_non_empty_ready_lists(&self) -> usize {
        (0..config::MAX_PRIORITIES)
            .filter(|&p| !self.ready_lists[p].is_empty())
            .count()
    }

    /// Debug: Get the address of a specific ready list
    pub fn get_ready_list_address(&self, priority: Priority) -> usize {
        if priority < config::MAX_PRIORITIES {
            &self.ready_lists[priority] as *const List as usize
        } else {
            0
        }
    }
}

// ============================================================================
// GLOBAL SCHEDULER INSTANCE
// ============================================================================

/// Global scheduler instance
///
/// In a single-core system, we only need one scheduler
/// Access is controlled by disabling interrupts (critical sections)
static mut GLOBAL_SCHEDULER: Scheduler = Scheduler::new();

// ============================================================================
// GLOBAL FUNCTIONS - Convenient API for using the scheduler
// ============================================================================

/// Initialize the global scheduler
///
/// MUST be called before any other scheduler functions
///
/// # Example
/// ```
/// init_scheduler();
/// // Now scheduler is ready to use
/// ```
pub fn init_scheduler() {
    unsafe {
        GLOBAL_SCHEDULER.init();
    }
}

/// Add a task to the scheduler
///
/// The task will be added to the ready list for its priority
/// Task count is incremented
///
/// # Arguments
/// * `tcb` - Task Control Block to add
///
/// # Example
/// ```
/// let mut tcb = TaskControlBlock::new(...);
/// tcb.update_list_item_owners();
/// add_task_to_scheduler(&mut tcb);
/// ```
pub fn add_task_to_scheduler(tcb: &mut TaskControlBlock) {
    unsafe {
        GLOBAL_SCHEDULER.add_task_to_ready_list(tcb);
        GLOBAL_SCHEDULER.increment_task_count();
    }
}

/// Remove a task from the scheduler
///
/// Returns true if the task was successfully removed
///
/// # Arguments
/// * `tcb` - Task Control Block to remove
pub fn remove_task_from_scheduler(tcb: &mut TaskControlBlock) -> bool {
    unsafe {
        let removed = GLOBAL_SCHEDULER.remove_task_from_ready_list(tcb);
        if removed {
            GLOBAL_SCHEDULER.decrement_task_count();
        }
        removed
    }
}

/// Yield the current task
///
/// Moves current task to end of its ready list
/// Allows other tasks at same priority to run
///
/// # Example
/// ```
/// // In a task:
/// loop {
///     do_work();
///     yield_current_task(); // Give other tasks a chance
/// }
/// ```
pub fn yield_current_task() {
    unsafe {
        GLOBAL_SCHEDULER.yield_task();
    }
}

/// Get the current task pointer
///
/// Returns the TCB of the currently running task
pub fn get_current_task() -> *mut TaskControlBlock {
    unsafe { GLOBAL_SCHEDULER.get_current_task() }
}

/// Set the current task pointer
///
/// Called by context switcher
///
/// # Safety
/// Caller must ensure tcb is valid
pub unsafe fn set_current_task(tcb: *mut TaskControlBlock) {
    GLOBAL_SCHEDULER.set_current_task(tcb);
}

/// Select the highest priority ready task
///
/// This is called by the context switcher to decide which task runs next
///
/// Returns pointer to TCB that should run
pub fn select_next_task() -> *mut TaskControlBlock {
    unsafe { GLOBAL_SCHEDULER.select_highest_priority_task() }
}

/// Select the next task to run, ensuring it's different from the current task
///
/// This is a wrapper around the scheduler's select_next_different_task() method.
/// Use this when you want to implement cooperative round-robin scheduling.
///
/// # Safety
/// Must be called from a valid task context
///
/// # Returns
/// Pointer to the next task's TCB
pub fn select_next_different_task() -> *mut TaskControlBlock {
    unsafe {
        GLOBAL_SCHEDULER.select_next_different_task()
    }
}

/// Get current system tick count
///
/// Returns the number of timer ticks since scheduler started
pub fn get_tick_count() -> TickType {
    unsafe { GLOBAL_SCHEDULER.get_tick_count() }
}

/// Increment system tick count
///
/// Called by timer interrupt handler (future implementation)
pub fn increment_tick() {
    unsafe {
        GLOBAL_SCHEDULER.increment_tick();
    }
}

/// Get total number of tasks in system
pub fn get_task_count() -> usize {
    unsafe { GLOBAL_SCHEDULER.get_task_count() }
}

/// Get top ready priority
///
/// Returns the highest priority level that has ready tasks
/// Useful for debugging
pub fn get_top_ready_priority() -> Priority {
    unsafe { GLOBAL_SCHEDULER.get_top_ready_priority() }
}

/// Check if scheduler is running
pub fn is_scheduler_running() -> bool {
    unsafe { GLOBAL_SCHEDULER.is_running() }
}

/// Suspend scheduler (enter critical section)
///
/// Use this to protect scheduler operations from interrupts
/// Must call resume() same number of times
///
/// # Example
/// ```
/// suspend_scheduler();
/// // Critical section - modify scheduler state
/// add_task_to_scheduler(&mut tcb);
/// resume_scheduler();
/// ```
pub fn suspend_scheduler() {
    unsafe {
        GLOBAL_SCHEDULER.suspend();
    }
}

/// Resume scheduler (exit critical section)
pub fn resume_scheduler() {
    unsafe {
        GLOBAL_SCHEDULER.resume();
    }
}

/// Check if scheduler is suspended
pub fn is_scheduler_suspended() -> bool {
    unsafe { GLOBAL_SCHEDULER.is_suspended() }
}

/// Debug: Get the number of non-empty ready lists
pub fn debug_count_non_empty_ready_lists() -> usize {
    unsafe { GLOBAL_SCHEDULER.count_non_empty_ready_lists() }
}

/// Debug: Check if a specific ready list is empty
pub fn debug_is_ready_list_empty(priority: Priority) -> bool {
    unsafe { GLOBAL_SCHEDULER.is_ready_list_empty(priority) }
}

/// Debug: Get the address of a specific ready list
pub fn debug_get_ready_list_address(priority: Priority) -> usize {
    unsafe { GLOBAL_SCHEDULER.get_ready_list_address(priority) }
}
