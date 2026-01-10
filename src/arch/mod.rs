// Architecture-specific code for RISC-V 64-bit
// INTEGER-ONLY VERSION (No Floating Point)

use crate::kernel::task::TaskControlBlock;
use core::arch::asm;

/// Size of saved context on stack (in bytes)
/// RISC-V has 32 registers, but x0 (zero) is hardwired to 0
/// So we save 31 registers × 8 bytes = 248 bytes
pub const CONTEXT_SIZE: usize = 31 * 8;

/// Stack must be aligned to 16 bytes (RISC-V ABI requirement)
pub const STACK_ALIGNMENT: usize = 16;

/// Initialize a task's stack for first-time execution
///
/// This creates a fake context on the stack so that when we "restore"
/// it for the first time, the task starts running from its entry point.
///
/// # Arguments
/// * `entry` - Task entry point function
/// * `stack` - Task's stack buffer (must be aligned)
///
/// # Returns
/// Pointer to top of initialized stack (where SP should point)
pub fn initialize_task_stack(entry: extern "C" fn() -> !, stack: &mut [usize]) -> *mut usize {
    // Get the top of the stack (stacks grow downward)
    let stack_top = unsafe { stack.as_mut_ptr().add(stack.len()) };
    
    // Align stack to 16 bytes
    let aligned_top = (stack_top as usize) & !(STACK_ALIGNMENT - 1);
    let mut sp = aligned_top as *mut usize;
    
    // Reserve space for context (31 registers)
    sp = unsafe { sp.sub(31) };
    
    // Initialize all registers to 0
    for i in 0..31 {
        unsafe {
            *sp.add(i) = 0;
        }
    }
    
    // Set ra (x1) to task entry point
    // When we "return" from the first context restore, we'll jump here
    // Register order: x1 is at offset 0
    unsafe {
        *sp = entry as usize;  // x1 (ra) = entry point
    }
    
    // Return the stack pointer
    // This will be saved in TCB->stack_top
    sp
}

/// Perform a context switch from one task to another
///
/// This is a wrapper around the assembly function.
/// It saves the current task's context and restores the next task's context.
///
/// # Safety
/// - Both task pointers must be valid
/// - Tasks must have properly initialized stacks
/// - This function never returns to the same context (it "returns" to the new task)
///
/// # Arguments
/// * `from_tcb` - Pointer to current task's TCB (can be null for first task)
/// * `to_tcb` - Pointer to next task's TCB
#[inline(never)]
pub unsafe fn switch_context(from_tcb: *mut TaskControlBlock, to_tcb: *mut TaskControlBlock) {
    // Update the scheduler's current task pointer
    crate::kernel::set_current_task(to_tcb);

    // Call the assembly function
    // It will save current context (if from_tcb != null) and load new context
    perform_context_switch(from_tcb, to_tcb);
}

/// Start the first task (never returns)
///
/// This is called once by the scheduler to start multitasking.
/// It loads the first task's context and begins execution.
///
/// # Safety
/// - Task must have a properly initialized stack
/// - Never call this function twice
///
/// # Arguments
/// * `tcb` - Pointer to first task's TCB
pub unsafe fn start_first_task(tcb: *mut TaskControlBlock) -> ! {
    // For the first task, we don't need to save any previous context
    // We just load the new task's context
    
    // Get the stack pointer from TCB
    let sp = (*tcb).stack_top;
    
    // Restore all registers from the stack
    // The assembly code will do this and "return" to the task entry point
    restore_context(sp);
    
    // Never reaches here
    unreachable!()
}

// ============================================================================
// ASSEMBLY FUNCTIONS
// ============================================================================
// These are implemented in switch.S

extern "C" {
    /// Perform context switch (implemented in assembly)
    ///
    /// Saves all integer registers from current task (if from != null)
    /// Loads all integer registers from next task
    ///
    /// # Arguments (in registers)
    /// * a0 (x10) = from_tcb pointer (can be null)
    /// * a1 (x11) = to_tcb pointer
    fn perform_context_switch(from_tcb: *mut TaskControlBlock, to_tcb: *mut TaskControlBlock);
    
    /// Restore context and start executing (implemented in assembly)
    ///
    /// Loads all integer registers from stack and jumps to task
    /// Used for starting the first task
    ///
    /// # Arguments (in registers)
    /// * a0 (x10) = stack pointer
    fn restore_context(sp: *mut usize) -> !;
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Disable interrupts (enter critical section)
///
/// Returns the previous interrupt state
#[inline]
pub fn disable_interrupts() -> bool {
    let status: usize;
    unsafe {
        // Read mstatus
        asm!("csrr {}, mstatus", out(reg) status);
        // Clear MIE bit (bit 3)
        asm!("csrci mstatus, 0x8");
    }
    // Return true if interrupts were enabled
    (status & 0x8) != 0
}

/// Enable interrupts (exit critical section)
#[inline]
pub fn enable_interrupts() {
    unsafe {
        // Set MIE bit (bit 3)
        asm!("csrsi mstatus, 0x8");
    }
}

/// Restore interrupt state
#[inline]
pub fn restore_interrupts(enabled: bool) {
    if enabled {
        enable_interrupts();
    }
}

/// Wait for interrupt (low power mode)
#[inline]
pub fn wait_for_interrupt() {
    unsafe {
        asm!("wfi");
    }
}

// ============================================================================
// CRITICAL SECTION GUARD
// ============================================================================

/// RAII guard for critical sections
///
/// Automatically restores interrupt state when dropped
pub struct CriticalSection {
    was_enabled: bool,
}

impl CriticalSection {
    /// Enter critical section (disable interrupts)
    pub fn enter() -> Self {
        CriticalSection {
            was_enabled: disable_interrupts(),
        }
    }
}

impl Drop for CriticalSection {
    fn drop(&mut self) {
        restore_interrupts(self.was_enabled);
    }
}

/// Macro for executing code in a critical section
#[macro_export]
macro_rules! critical_section {
    ($($body:tt)*) => {{
        let _guard = $crate::arch::CriticalSection::enter();
        $($body)*
    }};
}