#![no_std]              // No standard library (embedded)
#![no_main]             // Custom entry point

use core::panic::PanicInfo;
use riscv_rt::entry;     // Provides #[entry] macro

mod kernel;              // Your kernel modules
mod arch;                // Your architecture code

// Import what we need from kernel
use kernel::{
    TaskControlBlock,         // TCB struct
    init_scheduler,           // Initialize scheduler
    add_task_to_scheduler,    // Add task to ready list
    select_next_task,         // Pick next task to run
    select_next_different_task,
    get_task_count,
    get_top_ready_priority
};

// Import what we need from arch
use arch::{
    initialize_task_stack,    // Setup task's initial stack
    switch_context,           // Perform context switch
    start_first_task,         // Start the first task
};

const UART_BASE: usize = 0x10000000;

fn uart_putc(c: u8) {
    unsafe {
        core::ptr::write_volatile(UART_BASE as *mut u8, c);
    }
}

fn uart_puts(s: &str) {
    for b in s.bytes() {
        uart_putc(b);
    }
}

fn uart_puthex(value: usize) {
    uart_puts("0x");
    for i in (0..16).rev() {
        let nibble = ((value >> (i * 4)) & 0xF) as u8;
        let c = if nibble < 10 {
            b'0' + nibble
        } else {
            b'A' + (nibble - 10)
        };
        uart_putc(c);
    }
}

fn uart_putdec(value: usize) {
    if value == 0 {
        uart_putc(b'0');
        return;
    }

    let mut divisor = 1_000_000_000_000_000_000;
    let mut started = false;

    while divisor > 0 {
        let digit = (value / divisor) % 10;
        if digit != 0 || started {
            uart_putc(b'0' + digit as u8);
            started = true;
        }
        divisor /= 10;
    }
}

// ============================================================================
// TASK FUNCTIONS
// ============================================================================

/// Idle task - runs when no other tasks are ready
extern "C" fn idle_task() -> !{
    uart_puts("[Idle] Starting\r\n");

    let mut count:usize = 0;

    loop{
        for _ in 0..100000{
            unsafe {
                core::arch::asm!("nop");
            }
        }
        uart_puts("[Idle] Count: ");
        uart_puthex(count);
        uart_puts("\r\n");
        count += 1;

        unsafe {
            let current = kernel::get_current_task();
            let next = kernel::select_next_different_task();

            if next != current && !next.is_null(){
                uart_puts("[Idle] Yielding\r\n");
                kernel::yield_current_task();
                switch_context(current, next);
            }
        }
    }
}


/// Task 1 - High priority task WITH DEBUG OUTPUT
extern "C" fn task1() -> ! {
    uart_puts("[Task 1] Starting (Priority 2)\r\n");
    
    let mut count: usize = 0;
    loop {
        // Busy wait a bit
        for _ in 0..100000 {
            unsafe { core::arch::asm!("nop"); }
        }
        
        uart_puts("[Task 1] Count: ");
        uart_puthex(count);
        uart_puts("\r\n");
        count += 1;
        
        // Yield to other tasks
        unsafe {
            let current = kernel::get_current_task();

            uart_puts("[Task 1] DEBUG: Before select - non-empty lists: ");
            uart_putdec(kernel::debug_count_non_empty_ready_lists());
            uart_puts(", ready[0]: ");
            if kernel::debug_is_ready_list_empty(0) { uart_puts("empty"); } else { uart_puts("HAS_TASKS"); }
            uart_puts(", ready[1]: ");
            if kernel::debug_is_ready_list_empty(1) { uart_puts("empty"); } else { uart_puts("HAS_TASKS"); }
            uart_puts(", ready[2]: ");
            if kernel::debug_is_ready_list_empty(2) { uart_puts("empty"); } else { uart_puts("HAS_TASKS"); }
            uart_puts("\r\n");

            // DEBUG: Check if current task's list pointers are valid
            if !current.is_null() {
                uart_puts("[Task 1] DEBUG: current TCB address: 0x");
                uart_puthex(current as usize);
                uart_puts("\r\n");
                uart_puts("[Task 1] DEBUG: container: 0x");
                uart_puthex((*current).state_list_item.get_container() as usize);
                uart_puts(", prev: 0x");
                uart_puthex((*current).state_list_item.get_prev() as usize);
                uart_puts(", next: 0x");
                uart_puthex((*current).state_list_item.get_next() as usize);
                uart_puts("\r\n");
            }

            uart_puts("[Task 1] DEBUG: Calling select_next_different_task...\r\n");
            let next = select_next_different_task();

            uart_puts("[Task 1] DEBUG: After select - non-empty lists: ");
            uart_putdec(kernel::debug_count_non_empty_ready_lists());
            uart_puts("\r\n");
            
            uart_puts("[Task 1] DEBUG: current=");
            uart_puthex(current as usize);
            uart_puts(", next=");
            uart_puthex(next as usize);
            uart_puts("\r\n");
            
            if !current.is_null() {
                uart_puts("[Task 1] DEBUG: Current task name: ");
                uart_puts((*current).name_str());
                uart_puts("\r\n");
            }
            
            if !next.is_null() {
                uart_puts("[Task 1] DEBUG: Next task name: ");
                uart_puts((*next).name_str());
                uart_puts("\r\n");
            } else {
                uart_puts("[Task 1] DEBUG: Next task is NULL!\r\n");
            }
            
            if next != current && !next.is_null() {
                uart_puts("[Task 1] Yielding to ");
                uart_puts((*next).name_str());
                uart_puts("\r\n");
                kernel::yield_current_task();
                switch_context(current, next);
            } else {
                if next == current {
                    uart_puts("[Task 1] DEBUG: PROBLEM - next == current (same task!)\r\n");
                }
                if next.is_null() {
                    uart_puts("[Task 1] DEBUG: PROBLEM - next is null!\r\n");
                }
            }
        }
    }
}

/// Task 2 - Medium priority task
extern "C" fn task2() -> ! {
    uart_puts("[Task 2] Starting (Priority 1)\r\n");
    
    let mut count: usize = 0;
    loop {
        // Busy wait a bit
        for _ in 0..100000 {
            unsafe { core::arch::asm!("nop"); }
        }
        
        uart_puts("[Task 2] Count: ");
        uart_puthex(count);
        uart_puts("\r\n");
        count += 1;
        
        // Yield to other tasks
        unsafe {
            let current = kernel::get_current_task();
            let next = select_next_different_task();
            if next != current && !next.is_null() {
                uart_puts("[Task 2] Yielding\r\n");
                kernel::yield_current_task();
                switch_context(current, next);
            }
        }
    }
}

// ============================================================================
// MAIN
// ============================================================================

#[entry]
fn main() -> ! {
    uart_puts("\r\n");
    uart_puts("========================================\r\n");
    uart_puts("  RTOS Step 5: Context Switching Demo\r\n");
    uart_puts("  Tasks Will Actually RUN!\r\n");
    uart_puts("========================================\r\n");
    uart_puts("\r\n");
    
    // Initialize scheduler
    uart_puts("[Init] Initializing scheduler...\r\n");
    init_scheduler();

    unsafe {

// Task stacks
        static mut IDLE_STACK: [usize; 512] = [0; 512];
        static mut TASK1_STACK: [usize; 1024] = [0; 1024];
        static mut TASK2_STACK: [usize; 1024] = [0; 1024];
        
        // Task TCBs
        static mut IDLE_TCB: Option<TaskControlBlock> = None;
        static mut TASK1_TCB: Option<TaskControlBlock> = None;
        static mut TASK2_TCB: Option<TaskControlBlock> = None;
        
        // Create idle task (priority 0)
        uart_puts("[Init] Creating idle task...\r\n");
        let idle_sp = initialize_task_stack(idle_task, &mut IDLE_STACK);
        let idle_tcb = TaskControlBlock::new(
            "idle",
            0,  // Priority 0 (lowest)
            idle_sp,
            IDLE_STACK.len(),
        );
        IDLE_TCB = Some(idle_tcb);

        // CRITICAL: Update list item owners AFTER TCB is in final location
        if let Some(ref mut tcb) = IDLE_TCB {
            tcb.update_list_item_owners();
            add_task_to_scheduler(tcb);
            uart_puts("[Init] Idle task added\r\n");
        }
        
        // Create task1 (priority 2)
        uart_puts("[Init] Creating task 1...\r\n");
        let task1_sp = initialize_task_stack(task1, &mut TASK1_STACK);
        let task1_tcb = TaskControlBlock::new(
            "task1",
            2,  // Priority 2
            task1_sp,
            TASK1_STACK.len(),
        );
        TASK1_TCB = Some(task1_tcb);

        // CRITICAL: Update list item owners AFTER TCB is in final location
        if let Some(ref mut tcb) = TASK1_TCB {
            tcb.update_list_item_owners();
            add_task_to_scheduler(tcb);
            uart_puts("[Init] Task 1 added\r\n");
        }
        
        // Create task2 (priority 1)
        uart_puts("[Init] Creating task 2...\r\n");
        let task2_sp = initialize_task_stack(task2, &mut TASK2_STACK);
        let task2_tcb = TaskControlBlock::new(
            "task2",
            1,  // Priority 1
            task2_sp,
            TASK2_STACK.len(),
        );
        TASK2_TCB = Some(task2_tcb);

        // CRITICAL: Update list item owners AFTER TCB is in final location
        if let Some(ref mut tcb) = TASK2_TCB {
            tcb.update_list_item_owners();
            add_task_to_scheduler(tcb);
            uart_puts("[Init] Task 2 added\r\n");
        }

        uart_puts("\r\n");
        uart_puts("[DEBUG] ========== SCHEDULER STATE ==========\r\n");
        uart_puts("[DEBUG] Task count: ");
        uart_putdec(kernel::get_task_count());
        uart_puts("\r\n");
        uart_puts("[DEBUG] Top ready priority: ");
        uart_putdec(kernel::get_top_ready_priority());
        uart_puts("\r\n");

        // Check container pointers for each task and compare with actual ready_list addresses
        uart_puts("[DEBUG] Ready list addresses:\r\n");
        uart_puts("[DEBUG] ready_lists[0]: 0x");
        uart_puthex(kernel::debug_get_ready_list_address(0));
        uart_puts("\r\n");
        uart_puts("[DEBUG] ready_lists[1]: 0x");
        uart_puthex(kernel::debug_get_ready_list_address(1));
        uart_puts("\r\n");
        uart_puts("[DEBUG] ready_lists[2]: 0x");
        uart_puthex(kernel::debug_get_ready_list_address(2));
        uart_puts("\r\n");

        uart_puts("\r\n[DEBUG] Task container pointers:\r\n");
        if let Some(ref tcb) = IDLE_TCB {
            uart_puts("[DEBUG] Idle (pri 0) - container: 0x");
            uart_puthex(tcb.state_list_item.get_container() as usize);
            let expected = kernel::debug_get_ready_list_address(0);
            uart_puts(", expected: 0x");
            uart_puthex(expected);
            if tcb.state_list_item.get_container() as usize == expected {
                uart_puts(" ✓ MATCH");
            } else {
                uart_puts(" ✗ MISMATCH!");
            }
            uart_puts("\r\n");
        }
        if let Some(ref tcb) = TASK1_TCB {
            uart_puts("[DEBUG] Task1 (pri 2) - container: 0x");
            uart_puthex(tcb.state_list_item.get_container() as usize);
            let expected = kernel::debug_get_ready_list_address(2);
            uart_puts(", expected: 0x");
            uart_puthex(expected);
            if tcb.state_list_item.get_container() as usize == expected {
                uart_puts(" ✓ MATCH");
            } else {
                uart_puts(" ✗ MISMATCH!");
            }
            uart_puts("\r\n");
        }
        if let Some(ref tcb) = TASK2_TCB {
            uart_puts("[DEBUG] Task2 (pri 1) - container: 0x");
            uart_puthex(tcb.state_list_item.get_container() as usize);
            let expected = kernel::debug_get_ready_list_address(1);
            uart_puts(", expected: 0x");
            uart_puthex(expected);
            if tcb.state_list_item.get_container() as usize == expected {
                uart_puts(" ✓ MATCH");
            } else {
                uart_puts(" ✗ MISMATCH!");
            }
            uart_puts("\r\n");
        }
        uart_puts("[DEBUG] ========================================\r\n\r\n");

        uart_puts("[Init] Starting scheduler...\r\n");
        uart_puts("========================================\r\n");
        uart_puts("\r\n");
        
        // Select first task to run (should be task1 - highest priority)
        let first_task = select_next_task();
        
        if !first_task.is_null() {
            uart_puts("[Init] First task selected: ");
            uart_puts((*first_task).name_str());
            uart_puts("\r\n\r\n");
            
            // Set as current task
            kernel::set_current_task(first_task);
            
            // Start the first task!
            // This will never return - we'll be in task-land forever
            uart_puts("[Init] Jumping to first task...\r\n\r\n");
            start_first_task(first_task);
        } else {
            uart_puts("[Init] ERROR: No tasks to run!\r\n");
        }
    }
    
    // Should never reach here
    panic!("Scheduler failed to start!");
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    uart_puts("\r\n\r\n");
    uart_puts("========================================\r\n");
    uart_puts("           *** PANIC! ***\r\n");
    uart_puts("========================================\r\n");
    
    if let Some(location) = info.location() {
        uart_puts("Location: ");
        uart_puts(location.file());
        uart_puts(":");
        uart_putdec(location.line() as usize);
        uart_puts(":");
        uart_putdec(location.column() as usize);
        uart_puts("\r\n");
    }

    // Print panic message
    uart_puts("Message: ");
    // Use a simple writer to format the message
    struct UartWriter;
    impl core::fmt::Write for UartWriter {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            uart_puts(s);
            Ok(())
        }
    }
    use core::fmt::Write;
    let _ = write!(UartWriter, "{}", info.message());
    uart_puts("\r\n");
    
    uart_puts("========================================\r\n");
    uart_puts("System halted.\r\n");
    
    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}
