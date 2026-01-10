// Kernel module - Core RTOS functionality
pub mod list;
pub mod scheduler;
pub mod task;
pub mod types;

// Re-export commonly used items
pub use list::{List, ListNode};
pub use task::TaskControlBlock;
pub use types::{config, Priority, Result, RtosError, TaskState, TickType};

pub use scheduler::{
    add_task_to_scheduler,
    debug_count_non_empty_ready_lists,
    debug_get_ready_list_address,
    debug_is_ready_list_empty,
    get_current_task,
    get_task_count,
    get_tick_count,
    get_top_ready_priority,
    increment_tick,
    init_scheduler,
    is_scheduler_running,
    is_scheduler_suspended,
    remove_task_from_scheduler,
    resume_scheduler,
    select_next_task,
    select_next_different_task,
    set_current_task,
    suspend_scheduler,
    yield_current_task,
};
